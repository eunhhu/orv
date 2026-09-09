use super::*;

pub(crate) fn verify_build_dir(dir: &Path) -> anyhow::Result<()> {
    let manifest = read_json_value(&dir.join("build-manifest.json"))?;
    let plan = read_json_value(&dir.join("bundle-plan.json"))?;
    let origin_map = read_origin_map(dir)?;
    verify_origin_map_contract(&origin_map)?;
    let source_bundle = read_source_bundle_artifact(&dir.join("source-bundle.json"))?;
    verify_origin_map_source_spans(&origin_map, &source_bundle)?;
    verify_project_graph_contract(dir, &origin_map, &source_bundle)?;
    verify_bundle_targets(dir, &plan, &origin_map, &source_bundle)?;
    verify_manifest_artifacts(dir, &manifest, &plan, &source_bundle, &origin_map)?;
    verify_deploy_manifest_if_present(dir, &origin_map, &source_bundle)?;
    verify_dev_hmr_session_if_present(dir, &plan)?;
    verify_dev_hmr_transport_if_present(dir)?;
    verify_dev_hmr_server_if_present(dir)?;
    verify_dev_watch_session_if_present(dir, &plan)?;
    verify_dev_watch_events_if_present(dir)
}

pub(crate) fn verify_bundle_targets(
    dir: &Path,
    plan: &serde_json::Value,
    origin_map: &orv_compiler::OriginMap,
    source_bundle: &orv_compiler::SourceBundleArtifact,
) -> anyhow::Result<()> {
    verify_json_object_keys_exact(plan, &["schema_version", "bundles"], "bundle plan")?;
    if plan
        .get("schema_version")
        .and_then(serde_json::Value::as_u64)
        != Some(1)
    {
        anyhow::bail!("bundle plan schema_version must be 1");
    }
    let bundles = plan
        .get("bundles")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| anyhow::anyhow!("bundle plan bundles must be an array"))?;
    let mut seen_targets = HashSet::new();
    let mut server_artifacts = HashMap::new();
    for bundle in bundles {
        verify_json_object_keys_exact(
            bundle,
            &["kind", "path", "runtime_features"],
            "bundle target",
        )?;
        let kind = json_str(bundle, "kind", "bundle target")?;
        let path = json_str(bundle, "path", "bundle target")?;
        if !seen_targets.insert(kind.to_string()) {
            anyhow::bail!("bundle plan contains duplicate target kind {kind}");
        }
        if !bundle
            .get("runtime_features")
            .is_some_and(serde_json::Value::is_array)
        {
            anyhow::bail!("bundle target runtime_features must be an array");
        }
        verify_bundle_target_runtime_features(dir, bundle, kind, &mut server_artifacts)?;
        let target = dir.join(path);
        if !target.is_file() {
            anyhow::bail!("missing bundle target {kind}: {}", target.display());
        }
        match kind {
            "server_runtime" => {
                let artifact = cached_server_artifact(&mut server_artifacts, &target)?;
                orv_compiler::verify_server_runtime_artifact(artifact)
                    .map_err(|errors| anyhow::anyhow!("{}", errors.join("; ")))?;
                verify_server_runtime_origin_contract(artifact, origin_map)?;
                verify_server_runtime_source_bundle_contract(artifact, source_bundle)?;
            }
            "server_launcher" => verify_server_launcher_target(dir, &target)?,
            "native_server_plan" => verify_native_server_plan_target(dir, &target)?,
            "native_runtime_image_plan" => verify_native_runtime_image_plan_target(dir, &target)?,
            "native_runtime_image_dockerfile" => verify_native_runtime_image_dockerfile(&target)?,
            "native_server_launcher_source" => {
                let artifact =
                    cached_server_artifact(&mut server_artifacts, &dir.join(SERVER_ARTIFACT_PATH))?;
                verify_native_server_launcher_source(
                    &target,
                    SERVER_ARTIFACT_PATH,
                    NATIVE_SERVER_PLAN_PATH,
                    artifact,
                )?;
            }
            "native_server_routes_source" => {
                let artifact =
                    cached_server_artifact(&mut server_artifacts, &dir.join(SERVER_ARTIFACT_PATH))?;
                verify_native_server_routes_source(&target, artifact)?;
            }
            "native_server_router_source" => {
                verify_native_server_router_source(&target)?;
            }
            "native_server_handlers_source" => {
                let artifact =
                    cached_server_artifact(&mut server_artifacts, &dir.join(SERVER_ARTIFACT_PATH))?;
                verify_native_server_handlers_source(&target, artifact)?;
            }
            "native_server_launcher_package" => verify_native_server_launcher_package(&target)?,
            "static_page" => verify_static_page_target(bundle, &target)?,
            "client_manifest" => verify_client_manifest_target(dir, bundle, &target)?,
            "client_reactive_plan" => verify_client_reactive_plan_target(dir, bundle, &target)?,
            "client_page" => verify_client_page_target(bundle, &target)?,
            "client_js" => verify_client_js_target(dir, &target)?,
            "client_wasm" => verify_client_wasm_target(dir, &target)?,
            _ => anyhow::bail!("bundle target kind {kind} is not supported"),
        }
    }
    let expected_manifest = orv_compiler::build_manifest(&source_bundle.entry, origin_map);
    let expected_plan = serde_json::to_value(orv_compiler::bundle_plan(&expected_manifest))?;
    if plan != &expected_plan {
        anyhow::bail!("bundle plan does not match origin-map contract");
    }
    Ok(())
}

// Share runtime artifact reads between feature and runtime/native-source checks.
// Each bundle verification owns its cache, so the next call sees edits on disk.
fn cached_server_artifact<'a>(
    cache: &'a mut HashMap<PathBuf, orv_compiler::ServerRuntimeArtifact>,
    path: &Path,
) -> anyhow::Result<&'a orv_compiler::ServerRuntimeArtifact> {
    use std::collections::hash_map::Entry;
    Ok(match cache.entry(path.to_path_buf()) {
        Entry::Occupied(entry) => entry.into_mut(),
        Entry::Vacant(entry) => entry.insert(read_server_artifact(path)?),
    })
}

fn verify_bundle_target_runtime_features(
    dir: &Path,
    bundle: &serde_json::Value,
    kind: &str,
    server_artifacts: &mut HashMap<PathBuf, orv_compiler::ServerRuntimeArtifact>,
) -> anyhow::Result<()> {
    let actual = json_string_array_field(bundle, "runtime_features", "bundle target")?;
    let expected = match kind {
        "static_page" => Vec::new(),
        "client_manifest"
        | "client_reactive_plan"
        | "client_page"
        | "client_js"
        | "client_wasm" => vec!["client_wasm".to_string()],
        "server_runtime"
        | "server_launcher"
        | "native_server_plan"
        | "native_runtime_image_plan"
        | "native_runtime_image_dockerfile"
        | "native_server_launcher_source"
        | "native_server_routes_source"
        | "native_server_router_source"
        | "native_server_handlers_source"
        | "native_server_launcher_package" => {
            let artifact =
                cached_server_artifact(server_artifacts, &dir.join(SERVER_ARTIFACT_PATH))?;
            artifact.runtime_features.clone()
        }
        _ => return Ok(()),
    };
    if actual != expected {
        anyhow::bail!("bundle target {kind} runtime_features do not match target contract");
    }
    Ok(())
}

pub(crate) fn verify_static_page_target(
    bundle: &serde_json::Value,
    target: &Path,
) -> anyhow::Result<()> {
    let runtime_features = bundle
        .get("runtime_features")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| anyhow::anyhow!("static_page runtime_features must be an array"))?;
    if !runtime_features.is_empty() {
        anyhow::bail!("static_page bundle must be zero-runtime");
    }
    let html = std::fs::read_to_string(target)
        .map_err(|e| anyhow::anyhow!("failed to read {}: {e}", target.display()))?;
    let trimmed = html.trim_start();
    if trimmed.is_empty() {
        anyhow::bail!("static_page bundle is empty: {}", target.display());
    }
    if !(trimmed.starts_with("<html") || trimmed.starts_with("<!doctype")) {
        anyhow::bail!("static_page bundle is not html: {}", target.display());
    }
    Ok(())
}

pub(crate) fn verify_dev_hmr_session_if_present(
    dir: &Path,
    plan: &serde_json::Value,
) -> anyhow::Result<()> {
    let session_path = dir.join("dev").join("session.json");
    if !session_path.is_file() {
        return Ok(());
    }
    let session = read_json_value(&session_path)?;
    if session
        .get("schema_version")
        .and_then(serde_json::Value::as_u64)
        != Some(1)
    {
        anyhow::bail!("dev session schema_version must be 1");
    }
    if json_str(&session, "mode", "dev session")? != "hmr" {
        anyhow::bail!("dev session mode must be hmr");
    }
    if json_str(&session, "source_bundle", "dev session")? != "source-bundle.json" {
        anyhow::bail!("dev session source_bundle must be source-bundle.json");
    }
    let watch = session
        .get("watch")
        .ok_or_else(|| anyhow::anyhow!("dev session watch must be an object"))?;
    let session_sources = watch
        .get("sources")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| anyhow::anyhow!("dev session watch.sources must be an array"))?;
    let session_targets = watch
        .get("targets")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| anyhow::anyhow!("dev session watch.targets must be an array"))?;
    let source_bundle = read_json_value(&dir.join("source-bundle.json"))?;
    let expected_sources = source_bundle
        .get("files")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| anyhow::anyhow!("source-bundle.json files must be an array"))?;
    for source in expected_sources {
        let path = json_str(source, "path", "source bundle file")?;
        let content_hash = json_str(source, "content_hash", "source bundle file")?;
        if !session_sources.iter().any(|session_source| {
            session_source
                .get("path")
                .and_then(serde_json::Value::as_str)
                == Some(path)
                && session_source
                    .get("content_hash")
                    .and_then(serde_json::Value::as_str)
                    == Some(content_hash)
        }) {
            anyhow::bail!("dev session missing source {path}");
        }
    }
    let bundles = plan
        .get("bundles")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| anyhow::anyhow!("bundle plan bundles must be an array"))?;
    for bundle in bundles {
        let kind = json_str(bundle, "kind", "bundle target")?;
        let path = json_str(bundle, "path", "bundle target")?;
        if !session_targets.iter().any(|session_target| {
            session_target
                .get("kind")
                .and_then(serde_json::Value::as_str)
                == Some(kind)
                && session_target
                    .get("path")
                    .and_then(serde_json::Value::as_str)
                    == Some(path)
        }) {
            anyhow::bail!("dev session missing bundle target {kind}:{path}");
        }
    }
    let reload = session
        .get("reload")
        .ok_or_else(|| anyhow::anyhow!("dev session reload must be an object"))?;
    let has_client_target = bundles.iter().any(|target| {
        target
            .get("kind")
            .and_then(serde_json::Value::as_str)
            .is_some_and(is_client_bundle_kind)
    });
    let expected_strategy = if has_client_target {
        "hot-reload"
    } else {
        "full-reload"
    };
    if json_str(reload, "strategy", "dev session reload")? != expected_strategy {
        anyhow::bail!("dev session reload strategy must be {expected_strategy}");
    }
    if json_str(reload, "fallback", "dev session reload")? != "full-reload" {
        anyhow::bail!("dev session reload fallback must be full-reload");
    }
    Ok(())
}

pub(crate) fn verify_dev_hmr_transport_if_present(dir: &Path) -> anyhow::Result<()> {
    let transport_path = dir.join("dev").join("transport.json");
    if !transport_path.is_file() {
        return Ok(());
    }
    if !dir.join("dev").join("session.json").is_file() {
        anyhow::bail!("dev hmr transport requires dev/session.json");
    }
    let transport = read_json_value(&transport_path)?;
    if transport
        .get("schema_version")
        .and_then(serde_json::Value::as_u64)
        != Some(1)
    {
        anyhow::bail!("dev hmr transport schema_version must be 1");
    }
    if json_str(&transport, "mode", "dev hmr transport")? != "hmr-transport" {
        anyhow::bail!("dev hmr transport mode must be hmr-transport");
    }
    if json_str(&transport, "source_bundle", "dev hmr transport")? != "source-bundle.json" {
        anyhow::bail!("dev hmr transport source_bundle must be source-bundle.json");
    }
    if json_str(&transport, "session", "dev hmr transport")? != "dev/session.json" {
        anyhow::bail!("dev hmr transport session must be dev/session.json");
    }
    let browser = transport
        .get("browser")
        .ok_or_else(|| anyhow::anyhow!("dev hmr transport browser must be an object"))?;
    if json_str(browser, "kind", "dev hmr transport browser")? != "event-source" {
        anyhow::bail!("dev hmr transport browser kind must be event-source");
    }
    if json_str(browser, "client", "dev hmr transport browser")? != "dev/hmr-client.js" {
        anyhow::bail!("dev hmr transport browser client must be dev/hmr-client.js");
    }
    if json_str(browser, "event_source", "dev hmr transport browser")? != "/__orv/hmr/events" {
        anyhow::bail!("dev hmr transport browser event_source must be /__orv/hmr/events");
    }
    if json_str(browser, "session", "dev hmr transport browser")? != "/__orv/hmr/session" {
        anyhow::bail!("dev hmr transport browser session must be /__orv/hmr/session");
    }
    let server = transport
        .get("server")
        .ok_or_else(|| anyhow::anyhow!("dev hmr transport server must be an object"))?;
    if json_str(server, "kind", "dev hmr transport server")? != "reference-dev" {
        anyhow::bail!("dev hmr transport server kind must be reference-dev");
    }
    if json_str(server, "events", "dev hmr transport server")? != "dev/events.json" {
        anyhow::bail!("dev hmr transport server events must be dev/events.json");
    }
    let client_path = dir.join("dev").join("hmr-client.js");
    let client = std::fs::read_to_string(&client_path)
        .map_err(|e| anyhow::anyhow!("failed to read {}: {e}", client_path.display()))?;
    if !client.contains("EventSource('/__orv/hmr/events')") {
        anyhow::bail!("dev hmr client must connect to /__orv/hmr/events");
    }
    if !client.contains("window.location.reload()") {
        anyhow::bail!("dev hmr client must support full reload fallback");
    }
    Ok(())
}

pub(crate) fn verify_dev_watch_session_if_present(
    dir: &Path,
    plan: &serde_json::Value,
) -> anyhow::Result<()> {
    let session_path = dir.join("dev").join("watch.json");
    if !session_path.is_file() {
        return Ok(());
    }
    let session = read_json_value(&session_path)?;
    if session
        .get("schema_version")
        .and_then(serde_json::Value::as_u64)
        != Some(1)
    {
        anyhow::bail!("dev watch session schema_version must be 1");
    }
    if json_str(&session, "mode", "dev watch session")? != "watch" {
        anyhow::bail!("dev watch session mode must be watch");
    }
    if json_str(&session, "source_bundle", "dev watch session")? != "source-bundle.json" {
        anyhow::bail!("dev watch session source_bundle must be source-bundle.json");
    }
    verify_dev_watch_set(dir, plan, &session, "dev watch session")?;
    let loop_config = session
        .get("loop")
        .ok_or_else(|| anyhow::anyhow!("dev watch session loop must be an object"))?;
    if json_str(loop_config, "strategy", "dev watch session loop")? != "poll" {
        anyhow::bail!("dev watch session loop strategy must be poll");
    }
    if json_str(loop_config, "run", "dev watch session loop")? != "build-verify-run" {
        anyhow::bail!("dev watch session loop run must be build-verify-run");
    }
    let hmr = loop_config
        .get("hmr")
        .and_then(serde_json::Value::as_bool)
        .ok_or_else(|| anyhow::anyhow!("dev watch session loop hmr must be a boolean"))?;
    let interval_ms = loop_config
        .get("interval_ms")
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| anyhow::anyhow!("dev watch session loop interval_ms must be a number"))?;
    if interval_ms == 0 {
        anyhow::bail!("dev watch session loop interval_ms must be positive");
    }
    let reload = session
        .get("reload")
        .ok_or_else(|| anyhow::anyhow!("dev watch session reload must be an object"))?;
    let expected_strategy = if hmr && bundle_plan_has_client_target(plan)? {
        "hot-reload"
    } else {
        "full-reload"
    };
    if json_str(reload, "strategy", "dev watch session reload")? != expected_strategy {
        anyhow::bail!("dev watch session reload strategy must be {expected_strategy}");
    }
    if json_str(reload, "fallback", "dev watch session reload")? != "full-reload" {
        anyhow::bail!("dev watch session reload fallback must be full-reload");
    }
    let transport = session
        .get("transport")
        .ok_or_else(|| anyhow::anyhow!("dev watch session transport must be an object"))?;
    if json_str(transport, "kind", "dev watch session transport")? != "manifest" {
        anyhow::bail!("dev watch session transport kind must be manifest");
    }
    if json_str(transport, "path", "dev watch session transport")? != "dev/watch.json" {
        anyhow::bail!("dev watch session transport path must be dev/watch.json");
    }
    Ok(())
}

pub(crate) fn verify_dev_watch_events_if_present(dir: &Path) -> anyhow::Result<()> {
    let events_path = dir.join("dev").join("events.json");
    if !events_path.is_file() {
        return Ok(());
    }
    let events = read_json_value(&events_path)?;
    if events
        .get("schema_version")
        .and_then(serde_json::Value::as_u64)
        != Some(1)
    {
        anyhow::bail!("dev watch events schema_version must be 1");
    }
    if json_str(&events, "mode", "dev watch events")? != "watch-loop" {
        anyhow::bail!("dev watch events mode must be watch-loop");
    }
    if json_str(&events, "source_bundle", "dev watch events")? != "source-bundle.json" {
        anyhow::bail!("dev watch events source_bundle must be source-bundle.json");
    }
    let transport = events
        .get("transport")
        .ok_or_else(|| anyhow::anyhow!("dev watch events transport must be an object"))?;
    if json_str(transport, "kind", "dev watch events transport")? != "manifest" {
        anyhow::bail!("dev watch events transport kind must be manifest");
    }
    if json_str(transport, "path", "dev watch events transport")? != "dev/events.json" {
        anyhow::bail!("dev watch events transport path must be dev/events.json");
    }
    let loop_config = events
        .get("loop")
        .ok_or_else(|| anyhow::anyhow!("dev watch events loop must be an object"))?;
    if json_str(loop_config, "strategy", "dev watch events loop")? != "poll" {
        anyhow::bail!("dev watch events loop strategy must be poll");
    }
    if json_str(loop_config, "run", "dev watch events loop")? != "build-verify-run" {
        anyhow::bail!("dev watch events loop run must be build-verify-run");
    }
    let interval_ms = loop_config
        .get("interval_ms")
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| anyhow::anyhow!("dev watch events loop interval_ms must be a number"))?;
    if interval_ms == 0 {
        anyhow::bail!("dev watch events loop interval_ms must be positive");
    }
    let event_items = events
        .get("events")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| anyhow::anyhow!("dev watch events events must be an array"))?;
    if event_items.is_empty() {
        anyhow::bail!("dev watch events must contain at least one event");
    }
    for event in event_items {
        if event
            .get("iteration")
            .and_then(serde_json::Value::as_u64)
            .is_none()
        {
            anyhow::bail!("dev watch event iteration must be a number");
        }
        let action = json_str(event, "action", "dev watch event")?;
        if !matches!(action, "build-verify-run" | "skip") {
            anyhow::bail!("dev watch event action must be build-verify-run or skip");
        }
        if json_str(event, "status", "dev watch event")? != "ok" {
            anyhow::bail!("dev watch event status must be ok");
        }
        if json_str(event, "watch", "dev watch event")? != "dev/watch.json" {
            anyhow::bail!("dev watch event watch must be dev/watch.json");
        }
    }
    Ok(())
}

pub(crate) fn verify_dev_watch_set(
    dir: &Path,
    plan: &serde_json::Value,
    session: &serde_json::Value,
    context: &str,
) -> anyhow::Result<()> {
    let watch = session
        .get("watch")
        .ok_or_else(|| anyhow::anyhow!("{context} watch must be an object"))?;
    let session_sources = watch
        .get("sources")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| anyhow::anyhow!("{context} watch.sources must be an array"))?;
    let session_targets = watch
        .get("targets")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| anyhow::anyhow!("{context} watch.targets must be an array"))?;
    let source_bundle = read_json_value(&dir.join("source-bundle.json"))?;
    let expected_sources = source_bundle
        .get("files")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| anyhow::anyhow!("source-bundle.json files must be an array"))?;
    for source in expected_sources {
        let path = json_str(source, "path", "source bundle file")?;
        let content_hash = json_str(source, "content_hash", "source bundle file")?;
        if !session_sources.iter().any(|session_source| {
            session_source
                .get("path")
                .and_then(serde_json::Value::as_str)
                == Some(path)
                && session_source
                    .get("content_hash")
                    .and_then(serde_json::Value::as_str)
                    == Some(content_hash)
        }) {
            anyhow::bail!("{context} missing source {path}");
        }
    }
    let bundles = plan
        .get("bundles")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| anyhow::anyhow!("bundle plan bundles must be an array"))?;
    for bundle in bundles {
        let kind = json_str(bundle, "kind", "bundle target")?;
        let path = json_str(bundle, "path", "bundle target")?;
        if !session_targets.iter().any(|session_target| {
            session_target
                .get("kind")
                .and_then(serde_json::Value::as_str)
                == Some(kind)
                && session_target
                    .get("path")
                    .and_then(serde_json::Value::as_str)
                    == Some(path)
        }) {
            anyhow::bail!("{context} missing bundle target {kind}:{path}");
        }
    }
    Ok(())
}

pub(crate) fn verify_json_pointer_str(
    root: &serde_json::Value,
    pointer: &str,
    expected: &str,
    context: &str,
) -> anyhow::Result<()> {
    if root.pointer(pointer).and_then(serde_json::Value::as_str) != Some(expected) {
        anyhow::bail!("{context} must be {expected}");
    }
    Ok(())
}

#[cfg(unix)]
pub(crate) fn verify_executable_if_supported(path: &Path, label: &str) -> anyhow::Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let permissions = std::fs::metadata(path)
        .map_err(|e| anyhow::anyhow!("failed to stat {}: {e}", path.display()))?
        .permissions();
    if permissions.mode() & 0o111 == 0 {
        anyhow::bail!("{label} must be executable: {}", path.display());
    }
    Ok(())
}

#[cfg(not(unix))]
pub(crate) fn verify_executable_if_supported(_path: &Path, _label: &str) -> anyhow::Result<()> {
    Ok(())
}

#[cfg(unix)]
pub(crate) fn verify_shell_syntax_if_supported(path: &Path, label: &str) -> anyhow::Result<()> {
    let output = ProcessCommand::new("sh")
        .arg("-n")
        .arg(path)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| anyhow::anyhow!("failed to run shell syntax check for {label}: {e}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("{label} shell syntax invalid: {}", stderr.trim());
    }
    Ok(())
}

#[cfg(not(unix))]
pub(crate) fn verify_shell_syntax_if_supported(_path: &Path, _label: &str) -> anyhow::Result<()> {
    Ok(())
}
