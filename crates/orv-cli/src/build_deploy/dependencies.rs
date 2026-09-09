use super::*;

pub(crate) fn cmd_fetch(path: &Path, out: &Path) -> anyhow::Result<()> {
    let manifest = project_manifest_path(path)?;
    let root = manifest.parent().unwrap_or_else(|| Path::new("."));
    let lock_path = root.join("orv.lock");
    let lock = read_json_value(&lock_path)?;
    let expected = project_lock_json(&manifest)?;
    if lock != expected {
        anyhow::bail!("orv.lock is out of date; run `orv lock` before `orv fetch`");
    }

    fetch_lock_dependencies(root, out, &lock, "orv.lock")?;
    println!("fetch: wrote {}", out.display());
    Ok(())
}

pub(crate) fn fetch_lock_dependencies(
    root: &Path,
    out: &Path,
    lock: &serde_json::Value,
    lockfile: &str,
) -> anyhow::Result<serde_json::Value> {
    let mut packages = Vec::new();
    for key in ["dependencies", "dev_dependencies"] {
        let entries = lock
            .get(key)
            .and_then(serde_json::Value::as_array)
            .ok_or_else(|| anyhow::anyhow!("orv.lock field `{key}` must be an array"))?;
        for entry in entries {
            packages.push(fetch_dependency_package(root, out, entry)?);
        }
    }

    let manifest = serde_json::json!({
        "schema_version": 1,
        "kind": "orv.dependencies",
        "root": root.display().to_string(),
        "lockfile": lockfile,
        "stats": {
            "package_count": packages.len(),
        },
        "packages": packages,
    });
    write_json(&out.join("deps-manifest.json"), &manifest)?;
    Ok(manifest)
}

pub(crate) fn fetch_dependency_package(
    root: &Path,
    out: &Path,
    dependency: &serde_json::Value,
) -> anyhow::Result<serde_json::Value> {
    let name = json_str(dependency, "name", "lock dependency")?;
    let section = json_str(dependency, "section", "lock dependency")?;
    let source = json_str(dependency, "source", "lock dependency")?;
    let version = json_str(dependency, "version", "lock dependency")?;
    let checksum = json_str(dependency, "checksum", "lock dependency")?;
    let fetched = match source {
        "path" => FetchedDependency::ProjectRoot(path_dependency_project_root(root, dependency)?),
        "registry" => registry_dependency_source(root, dependency)?,
        other => anyhow::bail!("unsupported dependency source `{other}`"),
    };
    let resolved_url;
    let resolved_path;
    let source_bundle = match fetched {
        FetchedDependency::ProjectRoot(package_root) => {
            let entry = project_entry_path(&package_root)?;
            let loaded = orv_project::load_project(&entry).map_err(|e| anyhow::anyhow!("{e}"))?;
            report_diagnostics(&loaded.diagnostics, &loaded.files)?;
            resolved_path = Some(package_root.display().to_string());
            resolved_url = None;
            orv_compiler::source_bundle_artifact(
                entry.display().to_string(),
                loaded
                    .files
                    .iter()
                    .map(|file| (file.path.display().to_string(), file.source.clone())),
            )
        }
        FetchedDependency::SourceBundle { url, artifact } => {
            resolved_path = None;
            resolved_url = Some(url);
            artifact
        }
    };
    orv_compiler::verify_source_bundle_artifact(&source_bundle)
        .map_err(|errors| anyhow::anyhow!("{}", errors.join("; ")))?;
    let package_dir = format!(
        "packages/{}/{}",
        dependency_cache_component(section),
        dependency_cache_component(name)
    );
    let source_bundle_path = format!("{package_dir}/source-bundle.json");
    write_json(
        &out.join(&source_bundle_path),
        &serde_json::to_value(&source_bundle)?,
    )?;
    let source_entry = source_bundle.entry.clone();
    let source_file_count = source_bundle.files.len();

    let mut package = serde_json::json!({
        "name": name,
        "section": section,
        "source": source,
        "version": version,
        "checksum": checksum,
        "entry": source_entry,
        "source_bundle": source_bundle_path,
        "source_file_count": source_file_count,
        "verified": true,
    });
    if let Some(path) = resolved_path {
        package["resolved_path"] = serde_json::json!(path);
    }
    if let Some(url) = resolved_url {
        package["resolved_url"] = serde_json::json!(url);
    }
    if source == "path" {
        package["path"] = serde_json::json!(json_str(dependency, "path", "path dependency")?);
    } else {
        package["registry"] =
            serde_json::json!(json_str(dependency, "registry", "registry dependency")?);
        if let Some(auth_token_env) =
            json_optional_str(dependency, "auth_token_env", "registry dependency")?
        {
            package["auth_token_env"] = serde_json::json!(auth_token_env);
        }
    }
    Ok(package)
}

pub(crate) fn path_dependency_project_root(
    root: &Path,
    dependency: &serde_json::Value,
) -> anyhow::Result<PathBuf> {
    let path = PathBuf::from(json_str(dependency, "path", "path dependency")?);
    if path.is_absolute() {
        Ok(path)
    } else {
        Ok(root.join(path))
    }
}

pub(crate) enum FetchedDependency {
    ProjectRoot(PathBuf),
    SourceBundle {
        url: String,
        artifact: orv_compiler::SourceBundleArtifact,
    },
}

pub(crate) fn registry_dependency_source(
    root: &Path,
    dependency: &serde_json::Value,
) -> anyhow::Result<FetchedDependency> {
    let registry = json_str(dependency, "registry", "registry dependency")?;
    if registry.starts_with("http://") || registry.starts_with("https://") {
        let url = registry_source_bundle_url(
            registry,
            json_str(dependency, "name", "registry dependency")?,
            json_str(dependency, "version", "registry dependency")?,
        );
        let artifact = download_registry_source_bundle(
            &url,
            json_optional_str(dependency, "auth_token_env", "registry dependency")?,
        )?;
        return Ok(FetchedDependency::SourceBundle { url, artifact });
    }
    if registry == "registry.orv.dev" {
        anyhow::bail!(
            "remote registry download requires an explicit http://, https://, or file:// registry"
        );
    }
    let registry_root = registry.strip_prefix("file://").map_or_else(
        || {
            let path = PathBuf::from(registry);
            if path.is_absolute() {
                path
            } else {
                root.join(path)
            }
        },
        PathBuf::from,
    );
    Ok(FetchedDependency::ProjectRoot(
        registry_root
            .join(json_str(dependency, "name", "registry dependency")?)
            .join(json_str(dependency, "version", "registry dependency")?),
    ))
}

pub(crate) fn registry_source_bundle_url(registry: &str, name: &str, version: &str) -> String {
    format!(
        "{}/{}/{}/source-bundle.json",
        registry.trim_end_matches('/'),
        name,
        version
    )
}

pub(crate) fn download_registry_source_bundle(
    url: &str,
    auth_token_env: Option<&str>,
) -> anyhow::Result<orv_compiler::SourceBundleArtifact> {
    let body = registry_get_string_with_auth(url, auth_token_env)?;
    let artifact: orv_compiler::SourceBundleArtifact = serde_json::from_str(&body)
        .map_err(|e| anyhow::anyhow!("failed to parse registry source bundle {url}: {e}"))?;
    orv_compiler::verify_source_bundle_artifact(&artifact)
        .map_err(|errors| anyhow::anyhow!("{}", errors.join("; ")))?;
    Ok(artifact)
}

pub(crate) fn registry_get_string_with_auth(
    url: &str,
    auth_token_env: Option<&str>,
) -> anyhow::Result<String> {
    if url.starts_with("https://") {
        return https_get_string_with_auth(url, auth_token_env);
    }
    http_get_string_with_auth(url, auth_token_env)
}

pub(crate) fn https_get_string_with_auth(
    url: &str,
    auth_token_env: Option<&str>,
) -> anyhow::Result<String> {
    let mut request = ureq::get(url);
    if let Some(authorization) = registry_authorization_header(auth_token_env)? {
        request = request.header("Authorization", &authorization);
    }
    let mut response = request
        .call()
        .map_err(|e| anyhow::anyhow!("registry request {url} failed: {e}"))?;
    response
        .body_mut()
        .read_to_string()
        .map_err(|e| anyhow::anyhow!("failed to read registry response {url}: {e}"))
}

pub(crate) fn http_get_string_with_auth(
    url: &str,
    auth_token_env: Option<&str>,
) -> anyhow::Result<String> {
    let (host, port, path) = parse_http_url(url)?;
    let mut stream = std::net::TcpStream::connect((host.as_str(), port))
        .map_err(|e| anyhow::anyhow!("failed to connect to registry {host}:{port}: {e}"))?;
    stream
        .set_read_timeout(Some(Duration::from_secs(10)))
        .map_err(|e| anyhow::anyhow!("failed to configure registry read timeout: {e}"))?;
    let authorization = registry_authorization_header(auth_token_env)?;
    let mut request = format!("GET {path} HTTP/1.1\r\nHost: {host}\r\n");
    if let Some(authorization) = authorization {
        request.push_str("Authorization: ");
        request.push_str(&authorization);
        request.push_str("\r\n");
    }
    request.push_str("Connection: close\r\n\r\n");
    std::io::Write::write_all(&mut stream, request.as_bytes())
        .map_err(|e| anyhow::anyhow!("failed to send registry request {url}: {e}"))?;
    let mut response = Vec::new();
    std::io::Read::read_to_end(&mut stream, &mut response)
        .map_err(|e| anyhow::anyhow!("failed to read registry response {url}: {e}"))?;
    let header_end = response
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .ok_or_else(|| anyhow::anyhow!("registry response missing HTTP header terminator"))?;
    let headers = std::str::from_utf8(&response[..header_end])
        .map_err(|e| anyhow::anyhow!("registry response headers are not UTF-8: {e}"))?;
    let status = headers.lines().next().unwrap_or_default();
    if !status.starts_with("HTTP/1.1 200") && !status.starts_with("HTTP/1.0 200") {
        anyhow::bail!("registry request {url} failed with {status}");
    }
    String::from_utf8(response[header_end + 4..].to_vec())
        .map_err(|e| anyhow::anyhow!("registry response body is not UTF-8: {e}"))
}

pub(crate) fn registry_authorization_header(
    auth_token_env: Option<&str>,
) -> anyhow::Result<Option<String>> {
    let Some(auth_token_env) = auth_token_env else {
        return Ok(None);
    };
    let token = std::env::var(auth_token_env)
        .map_err(|_| anyhow::anyhow!("registry auth token env `{auth_token_env}` is not set"))?;
    if token.trim().is_empty() {
        anyhow::bail!("registry auth token env `{auth_token_env}` must not be empty");
    }
    if token.contains('\r') || token.contains('\n') {
        anyhow::bail!("registry auth token env `{auth_token_env}` must not contain newlines");
    }
    Ok(Some(format!("Bearer {token}")))
}

pub(crate) fn parse_http_url(url: &str) -> anyhow::Result<(String, u16, String)> {
    let Some(rest) = url.strip_prefix("http://") else {
        anyhow::bail!("registry URL must start with http://");
    };
    let (authority, path) = rest
        .split_once('/')
        .map_or((rest, "/"), |(authority, path)| {
            (authority, path.strip_prefix('/').unwrap_or(path))
        });
    if authority.is_empty() {
        anyhow::bail!("registry URL host must not be empty");
    }
    let (host, port) = if let Some((host, port)) = authority.rsplit_once(':') {
        let port = port
            .parse::<u16>()
            .map_err(|e| anyhow::anyhow!("registry URL port must be a u16: {e}"))?;
        (host.to_string(), port)
    } else {
        (authority.to_string(), 80)
    };
    if host.is_empty() {
        anyhow::bail!("registry URL host must not be empty");
    }
    Ok((host, port, format!("/{}", path.trim_start_matches('/'))))
}

pub(crate) fn dependency_cache_component(value: &str) -> String {
    let component = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_') {
                ch
            } else {
                '_'
            }
        })
        .collect::<String>();
    if component.is_empty() {
        "package".to_string()
    } else {
        component
    }
}

pub(crate) fn cmd_add_dependency(
    path: &Path,
    name: &str,
    version: Option<&str>,
    dev: bool,
    dependency_path: Option<&Path>,
    registry: Option<&str>,
) -> anyhow::Result<()> {
    let manifest_path = project_manifest_path(path)?;
    let mut manifest = read_toml_manifest(&manifest_path)?;
    add_dependency_to_manifest(&mut manifest, name, version, dev, dependency_path, registry)?;
    write_toml_manifest_atomic(&manifest_path, &manifest)?;
    cmd_lock(&manifest_path, false)?;
    println!("dependency: added {} to {}", name, dependency_section(dev));
    Ok(())
}

pub(crate) fn cmd_remove_dependency(path: &Path, name: &str, dev: bool) -> anyhow::Result<()> {
    let manifest_path = project_manifest_path(path)?;
    let mut manifest = read_toml_manifest(&manifest_path)?;
    remove_dependency_from_manifest(&mut manifest, name, dev)?;
    write_toml_manifest_atomic(&manifest_path, &manifest)?;
    cmd_lock(&manifest_path, false)?;
    println!(
        "dependency: removed {} from {}",
        name,
        dependency_section(dev)
    );
    Ok(())
}
