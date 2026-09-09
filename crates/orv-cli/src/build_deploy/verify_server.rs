use super::*;

pub(crate) fn verify_manifest_artifacts(
    dir: &Path,
    manifest: &serde_json::Value,
    plan: &serde_json::Value,
    source_bundle: &orv_compiler::SourceBundleArtifact,
    origin_map: &orv_compiler::OriginMap,
) -> anyhow::Result<()> {
    verify_json_object_keys_exact(
        manifest,
        &[
            "schema_version",
            "entry",
            "runtime",
            "artifacts",
            "capabilities",
        ],
        "build manifest",
    )?;
    if manifest
        .get("schema_version")
        .and_then(serde_json::Value::as_u64)
        != Some(1)
    {
        anyhow::bail!("build manifest schema_version must be 1");
    }
    if json_str(manifest, "entry", "build manifest")? != source_bundle.entry.as_str() {
        anyhow::bail!("build manifest entry does not match source-bundle entry");
    }
    if json_str(manifest, "runtime", "build manifest")? != "reference-interpreter" {
        anyhow::bail!("build manifest runtime must be reference-interpreter");
    }
    let capabilities = manifest
        .get("capabilities")
        .ok_or_else(|| anyhow::anyhow!("build manifest capabilities must be an object"))?;
    verify_json_object_keys_exact(
        capabilities,
        &[
            "has_server",
            "server_routes",
            "client_wasm",
            "runtime_features",
        ],
        "build manifest capabilities",
    )?;
    if !capabilities
        .get("has_server")
        .is_some_and(serde_json::Value::is_boolean)
    {
        anyhow::bail!("build manifest capabilities.has_server must be a boolean");
    }
    if !capabilities
        .get("server_routes")
        .is_some_and(json_nonnegative_integer)
    {
        anyhow::bail!("build manifest capabilities.server_routes must be an integer");
    }
    if !capabilities
        .get("client_wasm")
        .is_some_and(serde_json::Value::is_boolean)
    {
        anyhow::bail!("build manifest capabilities.client_wasm must be a boolean");
    }
    if !capabilities
        .get("runtime_features")
        .is_some_and(serde_json::Value::is_array)
    {
        anyhow::bail!("build manifest capabilities.runtime_features must be an array");
    }
    let expected_capabilities = serde_json::to_value(
        orv_compiler::build_manifest(&source_bundle.entry, origin_map).capabilities,
    )?;
    if capabilities != &expected_capabilities {
        anyhow::bail!("build manifest capabilities do not match origin-map contract");
    }
    let artifacts = manifest
        .get("artifacts")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| anyhow::anyhow!("build manifest artifacts must be an array"))?;
    let mut actual = std::collections::BTreeMap::new();
    for artifact in artifacts {
        verify_json_object_keys_exact(artifact, &["kind", "path"], "build manifest artifact")?;
        let kind = json_str(artifact, "kind", "build manifest artifact")?;
        let path = json_str(artifact, "path", "build manifest artifact")?;
        if actual.insert(kind.to_string(), path.to_string()).is_some() {
            anyhow::bail!("build manifest artifacts contains duplicate kind {kind}");
        }
        let artifact_path = dir.join(path);
        if !artifact_path.is_file() {
            anyhow::bail!(
                "missing manifest artifact {kind}: {}",
                artifact_path.display()
            );
        }
        if kind == "source_bundle" {
            let source_bundle = read_source_bundle_artifact(&artifact_path)?;
            orv_compiler::verify_source_bundle_artifact(&source_bundle)
                .map_err(|errors| anyhow::anyhow!("{}", errors.join("; ")))?;
        }
    }
    let expected = expected_build_manifest_artifacts(plan)?;
    if actual != expected {
        anyhow::bail!("build manifest artifacts must match bundle plan contract");
    }
    Ok(())
}

pub(crate) fn verify_server_runtime_source_bundle_contract(
    artifact: &orv_compiler::ServerRuntimeArtifact,
    source_bundle: &orv_compiler::SourceBundleArtifact,
) -> anyhow::Result<()> {
    if artifact.entry != source_bundle.entry {
        anyhow::bail!("server runtime entry does not match source-bundle artifact");
    }
    if artifact.source_bundle.files.len() != source_bundle.files.len() {
        anyhow::bail!("server runtime source bundle does not match build source-bundle artifact");
    }
    for expected in &source_bundle.files {
        let Some(actual) = artifact
            .source_bundle
            .files
            .iter()
            .find(|file| file.path == expected.path)
        else {
            let path = &expected.path;
            anyhow::bail!("server runtime source bundle is missing source file {path}");
        };
        if actual.content_hash != expected.content_hash || actual.source != expected.source {
            let path = &expected.path;
            anyhow::bail!(
                "server runtime source file {path} does not match build source-bundle artifact"
            );
        }
    }
    Ok(())
}

pub(crate) fn verify_server_launcher_target(dir: &Path, target: &Path) -> anyhow::Result<()> {
    let launch = read_server_launch_artifact(target)?;
    if launch.protocol != "http1" {
        anyhow::bail!("server launcher protocol must be http1");
    }
    let expected = vec![
        "orv".to_string(),
        "run-artifact".to_string(),
        launch.artifact.clone(),
    ];
    if launch.command != expected {
        anyhow::bail!("server launcher command must be `orv run-artifact <artifact>`");
    }
    let artifact = read_server_artifact(&dir.join(&launch.artifact))?;
    orv_compiler::verify_server_runtime_artifact(&artifact)
        .map_err(|errors| anyhow::anyhow!("{}", errors.join("; ")))?;
    if launch.runtime != artifact.runtime {
        anyhow::bail!("server launcher runtime does not match runtime artifact");
    }
    if launch.routes != artifact.routes {
        anyhow::bail!("server launcher routes do not match runtime artifact");
    }
    if launch.listen != artifact.listen {
        anyhow::bail!("server launcher listen does not match runtime artifact");
    }
    Ok(())
}

pub(crate) fn verify_dev_hmr_server_if_present(dir: &Path) -> anyhow::Result<()> {
    let server_path = dir.join("dev").join("server.json");
    if !server_path.is_file() {
        return Ok(());
    }
    if !dir.join("dev").join("session.json").is_file() {
        anyhow::bail!("dev hmr server requires dev/session.json");
    }
    if !dir.join("dev").join("events.json").is_file() {
        anyhow::bail!("dev hmr server requires dev/events.json");
    }
    let server = read_json_value(&server_path)?;
    if server
        .get("schema_version")
        .and_then(serde_json::Value::as_u64)
        != Some(1)
    {
        anyhow::bail!("dev hmr server schema_version must be 1");
    }
    if json_str(&server, "mode", "dev hmr server")? != "hmr-server" {
        anyhow::bail!("dev hmr server mode must be hmr-server");
    }
    if json_str(&server, "protocol", "dev hmr server")? != "http1" {
        anyhow::bail!("dev hmr server protocol must be http1");
    }
    if json_str(&server, "session", "dev hmr server")? != "dev/session.json" {
        anyhow::bail!("dev hmr server session must be dev/session.json");
    }
    if json_str(&server, "events", "dev hmr server")? != "dev/events.json" {
        anyhow::bail!("dev hmr server events must be dev/events.json");
    }
    let address = json_str(&server, "address", "dev hmr server")?;
    address
        .parse::<SocketAddr>()
        .map_err(|e| anyhow::anyhow!("dev hmr server address must be a socket address: {e}"))?;
    let endpoints = server
        .get("endpoints")
        .ok_or_else(|| anyhow::anyhow!("dev hmr server endpoints must be an object"))?;
    if json_str(endpoints, "session", "dev hmr server endpoints")? != "/__orv/hmr/session" {
        anyhow::bail!("dev hmr server session endpoint must be /__orv/hmr/session");
    }
    if json_str(endpoints, "events", "dev hmr server endpoints")? != "/__orv/hmr/events" {
        anyhow::bail!("dev hmr server events endpoint must be /__orv/hmr/events");
    }
    Ok(())
}

pub(crate) fn verify_source_bundle_artifact_keys(value: &serde_json::Value) -> anyhow::Result<()> {
    verify_json_object_keys_exact(
        value,
        &["schema_version", "entry", "files"],
        "source-bundle.json",
    )?;
    let files = value
        .get("files")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| anyhow::anyhow!("source-bundle.json files must be an array"))?;
    for (index, file) in files.iter().enumerate() {
        verify_json_object_keys_exact(
            file,
            &["path", "content_hash", "source"],
            &format!("source-bundle.json files[{index}]"),
        )?;
    }
    Ok(())
}
