use super::*;

pub(crate) fn verify_deploy_manifest_if_present(
    dir: &Path,
    origin_map: &orv_compiler::OriginMap,
    source_bundle: &orv_compiler::SourceBundleArtifact,
) -> anyhow::Result<()> {
    let deploy_manifest = dir.join("deploy").join("manifest.json");
    if !deploy_manifest.is_file() {
        return Ok(());
    }
    let deploy = read_json_value(&deploy_manifest)?;
    let version = deploy
        .get("schema_version")
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| anyhow::anyhow!("deploy manifest schema_version must be an integer"))?;
    if version != 1 {
        anyhow::bail!("unsupported deploy manifest schema_version {version}");
    }
    verify_json_object_keys_exact(
        &deploy,
        &[
            "schema_version",
            "profile",
            "entry",
            "runtime",
            "runtime_features",
            "source_bundle",
            "server",
            "static",
            "client",
        ],
        "deploy manifest",
    )?;
    if deploy.get("profile").and_then(serde_json::Value::as_str) != Some("prod") {
        anyhow::bail!("deploy manifest profile must be prod");
    }
    if json_str(&deploy, "entry", "deploy manifest")? != source_bundle.entry.as_str() {
        anyhow::bail!("deploy manifest entry does not match source-bundle entry");
    }
    if json_str(&deploy, "runtime", "deploy manifest")? != "reference-interpreter" {
        anyhow::bail!("deploy manifest runtime must be reference-interpreter");
    }
    if !deploy
        .get("runtime_features")
        .is_some_and(serde_json::Value::is_array)
    {
        anyhow::bail!("deploy manifest runtime_features must be an array");
    }
    if json_str(&deploy, "source_bundle", "deploy manifest")? != SOURCE_BUNDLE_PATH {
        anyhow::bail!("deploy manifest source_bundle must be {SOURCE_BUNDLE_PATH}");
    }
    verify_deploy_source_bundle(dir, deploy.get("source_bundle"), source_bundle)?;
    verify_deploy_server_target(
        dir,
        deploy.get("server"),
        deploy.get("client"),
        origin_map,
        source_bundle,
    )?;
    verify_deploy_static_target(dir, deploy.get("static"))?;
    verify_deploy_client_target(dir, deploy.get("client"))
}

pub(crate) fn verify_deploy_source_bundle(
    dir: &Path,
    source_bundle: Option<&serde_json::Value>,
    expected: &orv_compiler::SourceBundleArtifact,
) -> anyhow::Result<()> {
    let Some(path) = source_bundle.and_then(serde_json::Value::as_str) else {
        anyhow::bail!("deploy manifest source_bundle must be a string");
    };
    let target = dir.join(path);
    if !target.is_file() {
        anyhow::bail!("missing deploy source bundle: {}", target.display());
    }
    let artifact = read_source_bundle_artifact(&target)?;
    if &artifact != expected {
        anyhow::bail!("deploy manifest source_bundle does not match build source-bundle artifact");
    }
    Ok(())
}

pub(crate) fn verify_deploy_server_target(
    dir: &Path,
    server: Option<&serde_json::Value>,
    client: Option<&serde_json::Value>,
    origin_map: &orv_compiler::OriginMap,
    source_bundle: &orv_compiler::SourceBundleArtifact,
) -> anyhow::Result<()> {
    let Some(server) = server.filter(|value| !value.is_null()) else {
        return Ok(());
    };
    verify_json_object_keys_exact(
        server,
        &[
            "runtime",
            "runtime_features",
            "artifact",
            "entrypoint",
            "routes_artifact",
            "native_plan",
            "native_runtime_image_plan",
            "native_routes_source",
            "native_router_source",
            "native_handlers_source",
            "container",
            "dockerfile",
            "compose",
            "env_example",
            "db_adapters",
            "commerce_adapters",
            "smoke_test",
            "smoke_output",
            "preflight",
            "benchmark_evidence",
            "participant_notes_template",
            "runbook",
            "runtime_image",
            "protocol",
            "listen",
            "routes",
            "persistence",
        ],
        "deploy server",
    )?;
    let artifact_path = json_str(server, "artifact", "deploy server")?;
    let entrypoint = json_str(server, "entrypoint", "deploy server")?;
    let routes_artifact = json_str(server, "routes_artifact", "deploy server")?;
    let native_plan = json_str(server, "native_plan", "deploy server")?;
    let native_runtime_image_plan = json_str(server, "native_runtime_image_plan", "deploy server")?;
    let native_route_table_source = json_str(server, "native_routes_source", "deploy server")?;
    let native_dispatch_source = json_str(server, "native_router_source", "deploy server")?;
    let native_handlers_source = json_str(server, "native_handlers_source", "deploy server")?;
    let container = json_str(server, "container", "deploy server")?;
    let dockerfile = json_str(server, "dockerfile", "deploy server")?;
    let compose = json_str(server, "compose", "deploy server")?;
    let env_example = json_str(server, "env_example", "deploy server")?;
    let db_adapters = json_str(server, "db_adapters", "deploy server")?;
    let commerce_adapters = json_str(server, "commerce_adapters", "deploy server")?;
    let smoke_test = json_str(server, "smoke_test", "deploy server")?;
    if smoke_test != DEPLOY_SMOKE_TEST_PATH {
        anyhow::bail!("deploy server smoke_test must be {DEPLOY_SMOKE_TEST_PATH}");
    }
    let smoke_output = json_str(server, "smoke_output", "deploy server")?;
    if smoke_output != DEPLOY_SMOKE_OUTPUT_PATH {
        anyhow::bail!("deploy server smoke_output must be {DEPLOY_SMOKE_OUTPUT_PATH}");
    }
    let preflight = json_str(server, "preflight", "deploy server")?;
    if preflight != DEPLOY_PREFLIGHT_PATH {
        anyhow::bail!("deploy server preflight must be {DEPLOY_PREFLIGHT_PATH}");
    }
    let benchmark_evidence = json_str(server, "benchmark_evidence", "deploy server")?;
    if benchmark_evidence != DEPLOY_BENCHMARK_EVIDENCE_PATH {
        anyhow::bail!("deploy server benchmark_evidence must be {DEPLOY_BENCHMARK_EVIDENCE_PATH}");
    }
    let participant_notes_template =
        json_str(server, "participant_notes_template", "deploy server")?;
    if participant_notes_template != DEPLOY_PARTICIPANT_NOTES_TEMPLATE_PATH {
        anyhow::bail!(
            "deploy server participant_notes_template must be {DEPLOY_PARTICIPANT_NOTES_TEMPLATE_PATH}"
        );
    }
    let runbook = json_str(server, "runbook", "deploy server")?;
    let runtime_image = json_str(server, "runtime_image", "deploy server")?;
    if runtime_image != ORV_REFERENCE_RUNTIME_IMAGE {
        anyhow::bail!("deploy server runtime_image must be {ORV_REFERENCE_RUNTIME_IMAGE}");
    }
    if json_str(server, "protocol", "deploy server")? != "http1" {
        anyhow::bail!("deploy server protocol must be http1");
    }
    verify_deploy_server_entrypoint(dir, entrypoint, artifact_path)?;
    let artifact = read_server_artifact(&dir.join(artifact_path))?;
    orv_compiler::verify_server_runtime_artifact(&artifact)
        .map_err(|errors| anyhow::anyhow!("{}", errors.join("; ")))?;
    verify_server_runtime_origin_contract(&artifact, origin_map)?;
    verify_server_runtime_source_bundle_contract(&artifact, source_bundle)?;
    validate_prod_server_listen(Some(&artifact))?;
    let persistence = server_artifact_deploy_persistence(&artifact)?;
    verify_deploy_routes_artifact(
        dir,
        routes_artifact,
        artifact_path,
        artifact.runtime.as_str(),
        &artifact,
    )?;
    verify_native_server_plan_artifact(
        dir,
        native_plan,
        artifact_path,
        SERVER_LAUNCH_PATH,
        &artifact,
    )?;
    verify_native_runtime_image_plan_artifact(
        dir,
        native_runtime_image_plan,
        artifact_path,
        native_plan,
        &artifact,
    )?;
    if native_route_table_source != NATIVE_SERVER_ROUTES_SOURCE_PATH {
        anyhow::bail!(
            "deploy server native_routes_source must be {NATIVE_SERVER_ROUTES_SOURCE_PATH}"
        );
    }
    verify_native_server_routes_source(&dir.join(native_route_table_source), &artifact)?;
    if native_dispatch_source != NATIVE_SERVER_ROUTER_SOURCE_PATH {
        anyhow::bail!(
            "deploy server native_router_source must be {NATIVE_SERVER_ROUTER_SOURCE_PATH}"
        );
    }
    verify_native_server_router_source(&dir.join(native_dispatch_source))?;
    if native_handlers_source != NATIVE_SERVER_HANDLERS_SOURCE_PATH {
        anyhow::bail!(
            "deploy server native_handlers_source must be {NATIVE_SERVER_HANDLERS_SOURCE_PATH}"
        );
    }
    verify_native_server_handlers_source(&dir.join(native_handlers_source), &artifact)?;
    verify_deploy_container_artifact(
        dir,
        container,
        dockerfile,
        &DeployServerContract {
            artifact_path,
            entrypoint,
            routes_artifact,
            runtime: artifact.runtime.as_str(),
            runtime_image,
            listen: artifact.listen.as_ref(),
        },
        &persistence,
    )?;
    verify_deploy_compose_artifact(
        dir,
        compose,
        dockerfile,
        runtime_image,
        artifact.listen.as_ref(),
        &persistence,
    )?;
    verify_deploy_env_example_artifact(dir, env_example, artifact.listen.as_ref(), &persistence)?;
    verify_deploy_db_adapters_artifact(dir, db_adapters, artifact_path, &persistence, origin_map)?;
    verify_deploy_commerce_adapters_artifact(
        dir,
        commerce_adapters,
        artifact_path,
        &persistence,
        origin_map,
    )?;
    verify_deploy_smoke_test_artifact(
        dir,
        smoke_test,
        artifact.listen.as_ref(),
        &artifact,
        origin_map,
        &persistence,
        client,
    )?;
    let deploy_artifacts = DeployRunbookArtifacts {
        server_artifact: artifact_path,
        compose,
        env_example,
        db_adapters,
        commerce_adapters,
        smoke_test,
        smoke_output,
        preflight,
        benchmark_evidence,
        participant_notes_template,
        runbook,
        routes: routes_artifact,
    };
    verify_deploy_preflight_artifact(
        dir,
        preflight,
        &deploy_artifacts,
        &artifact,
        &persistence,
        client,
    )?;
    verify_deploy_benchmark_evidence_artifact(
        dir,
        benchmark_evidence,
        &deploy_artifacts,
        &artifact,
        &persistence,
        client,
    )?;
    verify_deploy_participant_notes_template_artifact(dir, DEPLOY_PARTICIPANT_NOTES_TEMPLATE_PATH)?;
    verify_deploy_runbook_artifact(
        dir,
        runbook,
        &deploy_artifacts,
        &artifact,
        &persistence,
        client,
    )?;
    if server.get("runtime").and_then(serde_json::Value::as_str) != Some(artifact.runtime.as_str())
    {
        anyhow::bail!("deploy server runtime does not match runtime artifact");
    }
    if server.get("runtime_features") != Some(&serde_json::to_value(&artifact.runtime_features)?) {
        anyhow::bail!("deploy server runtime_features do not match runtime artifact");
    }
    verify_deploy_listen_value(
        server.get("listen"),
        artifact.listen.as_ref(),
        "deploy server",
    )?;
    if let Some(routes) = server.get("routes") {
        let artifact_routes = serde_json::to_value(&artifact.routes)?;
        if routes != &artifact_routes {
            anyhow::bail!("deploy server routes do not match runtime artifact");
        }
    }
    if server.get("persistence") != Some(&deploy_persistence_value(&persistence)) {
        anyhow::bail!("deploy server persistence does not match runtime artifact");
    }
    Ok(())
}

pub(crate) fn verify_deploy_server_entrypoint(
    dir: &Path,
    entrypoint: &str,
    artifact_path: &str,
) -> anyhow::Result<()> {
    let entrypoint_path = dir.join(entrypoint);
    if !entrypoint_path.is_file() {
        anyhow::bail!(
            "missing deploy server entrypoint: {}",
            entrypoint_path.display()
        );
    }
    let script = std::fs::read_to_string(&entrypoint_path)
        .map_err(|e| anyhow::anyhow!("failed to read {}: {e}", entrypoint_path.display()))?;
    if !script.contains("orv run-artifact") {
        anyhow::bail!("deploy server entrypoint must run `orv run-artifact`");
    }
    let expected = deploy_server_entrypoint_content(artifact_path);
    if script != expected {
        anyhow::bail!("deploy server entrypoint must match generated artifact");
    }
    Ok(())
}

pub(crate) fn verify_deploy_container_artifact(
    dir: &Path,
    path: &str,
    dockerfile_path: &str,
    contract: &DeployServerContract<'_>,
    persistence: &DeployPersistence,
) -> anyhow::Result<()> {
    let container_path = dir.join(path);
    if !container_path.is_file() {
        anyhow::bail!(
            "missing deploy container artifact: {}",
            container_path.display()
        );
    }
    let container = read_json_value(&container_path)?;
    verify_json_object_keys_exact(
        &container,
        &[
            "schema_version",
            "kind",
            "dockerfile",
            "artifact",
            "entrypoint",
            "routes_artifact",
            "runtime",
            "runtime_image",
            "protocol",
            "listen",
            "ports",
            "command",
            "persistence",
        ],
        "deploy container",
    )?;
    if container
        .get("schema_version")
        .and_then(serde_json::Value::as_u64)
        != Some(1)
    {
        anyhow::bail!("deploy container schema_version must be 1");
    }
    if json_str(&container, "kind", "deploy container")? != "reference-server-container" {
        anyhow::bail!("deploy container kind must be reference-server-container");
    }
    if json_str(&container, "artifact", "deploy container")? != contract.artifact_path {
        let artifact_path = contract.artifact_path;
        anyhow::bail!("deploy container artifact must be {artifact_path}");
    }
    if json_str(&container, "entrypoint", "deploy container")? != contract.entrypoint {
        let entrypoint = contract.entrypoint;
        anyhow::bail!("deploy container entrypoint must be {entrypoint}");
    }
    if json_str(&container, "routes_artifact", "deploy container")? != contract.routes_artifact {
        let routes_artifact = contract.routes_artifact;
        anyhow::bail!("deploy container routes_artifact must be {routes_artifact}");
    }
    if json_str(&container, "dockerfile", "deploy container")? != dockerfile_path {
        anyhow::bail!("deploy container dockerfile must be {dockerfile_path}");
    }
    if json_str(&container, "runtime", "deploy container")? != contract.runtime {
        anyhow::bail!("deploy container runtime does not match runtime artifact");
    }
    if json_str(&container, "runtime_image", "deploy container")? != contract.runtime_image {
        let runtime_image = contract.runtime_image;
        anyhow::bail!("deploy container runtime_image must be {runtime_image}");
    }
    if json_str(&container, "protocol", "deploy container")? != "http1" {
        anyhow::bail!("deploy container protocol must be http1");
    }
    verify_deploy_listen_value(container.get("listen"), contract.listen, "deploy container")?;
    if container.get("ports") != Some(&deploy_ports_value(contract.listen)) {
        anyhow::bail!("deploy container ports do not match runtime artifact");
    }
    if container.get("command") != Some(&serde_json::json!(["./deploy/server.sh"])) {
        anyhow::bail!("deploy container command must be [\"./deploy/server.sh\"]");
    }
    if container.get("persistence") != Some(&deploy_persistence_value(persistence)) {
        anyhow::bail!("deploy container persistence does not match runtime artifact");
    }
    verify_deploy_dockerfile(
        dir,
        dockerfile_path,
        contract.runtime_image,
        contract.listen,
    )
}

pub(crate) fn verify_deploy_compose_artifact(
    dir: &Path,
    path: &str,
    dockerfile_path: &str,
    runtime_image: &str,
    listen: Option<&orv_compiler::ServerListenArtifact>,
    persistence: &DeployPersistence,
) -> anyhow::Result<()> {
    let compose_path = dir.join(path);
    if !compose_path.is_file() {
        anyhow::bail!("missing deploy compose file: {}", compose_path.display());
    }
    let compose = std::fs::read_to_string(&compose_path)
        .map_err(|e| anyhow::anyhow!("failed to read {}: {e}", compose_path.display()))?;
    let dockerfile_line = format!("dockerfile: {dockerfile_path}");
    if !compose.contains(&dockerfile_line) {
        anyhow::bail!("deploy compose must use {dockerfile_path}");
    }
    let runtime_image_line = format!("ORV_RUNTIME_IMAGE: {runtime_image}");
    if !compose.contains(&runtime_image_line) {
        anyhow::bail!("deploy compose must set ORV_RUNTIME_IMAGE");
    }
    if let Some(port) = deploy_compose_port(listen) {
        if !compose.contains(&port.binding) {
            let display = port.display;
            anyhow::bail!("deploy compose must publish {display}");
        }
    }
    for environment in deploy_compose_environment_lines(listen, persistence) {
        if !compose.contains(&environment) {
            anyhow::bail!("deploy compose must configure {environment}");
        }
    }
    for volume in &persistence.volumes {
        if !compose.contains(&volume.compose_mount) {
            let mount = &volume.compose_mount;
            anyhow::bail!("deploy compose must mount persistent volume {mount}");
        }
    }
    let expected = deploy_compose_content(dockerfile_path, listen, persistence);
    if compose != expected {
        anyhow::bail!("deploy compose must match generated artifact");
    }
    Ok(())
}

pub(crate) fn verify_deploy_commerce_adapters_artifact(
    dir: &Path,
    path: &str,
    artifact_path: &str,
    persistence: &DeployPersistence,
    origin_map: &orv_compiler::OriginMap,
) -> anyhow::Result<()> {
    let adapters_path = dir.join(path);
    if !adapters_path.is_file() {
        anyhow::bail!(
            "missing deploy commerce adapters artifact: {}",
            adapters_path.display()
        );
    }
    let adapters = read_json_value(&adapters_path)?;
    verify_deploy_commerce_adapter_contract_keys(&adapters)?;
    if adapters
        .get("schema_version")
        .and_then(serde_json::Value::as_u64)
        != Some(1)
    {
        anyhow::bail!("deploy commerce adapters schema_version must be 1");
    }
    if json_str(&adapters, "kind", "deploy commerce adapters")? != "orv.deploy.commerce_adapters" {
        anyhow::bail!("deploy commerce adapters kind must be orv.deploy.commerce_adapters");
    }
    if json_str(&adapters, "artifact", "deploy commerce adapters")? != artifact_path {
        anyhow::bail!("deploy commerce adapters artifact must be {artifact_path}");
    }
    if adapters.get("adapters")
        != Some(&serde_json::Value::Array(deploy_commerce_adapter_value(
            &persistence.commerce_adapters,
        )))
    {
        anyhow::bail!("deploy commerce adapters do not match runtime artifact persistence");
    }
    verify_deploy_commerce_adapter_source_origins(origin_map, &persistence.commerce_adapters)?;
    Ok(())
}

pub(crate) fn verify_deploy_db_adapters_artifact(
    dir: &Path,
    path: &str,
    artifact_path: &str,
    persistence: &DeployPersistence,
    origin_map: &orv_compiler::OriginMap,
) -> anyhow::Result<()> {
    let adapters_path = dir.join(path);
    if !adapters_path.is_file() {
        anyhow::bail!(
            "missing deploy DB adapters artifact: {}",
            adapters_path.display()
        );
    }
    let adapters = read_json_value(&adapters_path)?;
    verify_deploy_db_adapter_contract_keys(&adapters)?;
    if adapters
        .get("schema_version")
        .and_then(serde_json::Value::as_u64)
        != Some(1)
    {
        anyhow::bail!("deploy DB adapters schema_version must be 1");
    }
    if json_str(&adapters, "kind", "deploy DB adapters")? != "orv.deploy.db_adapters" {
        anyhow::bail!("deploy DB adapters kind must be orv.deploy.db_adapters");
    }
    if json_str(&adapters, "artifact", "deploy DB adapters")? != artifact_path {
        anyhow::bail!("deploy DB adapters artifact must be {artifact_path}");
    }
    if adapters.get("adapters")
        != Some(&serde_json::Value::Array(deploy_db_adapter_value(
            &persistence.db_adapters,
        )))
    {
        anyhow::bail!("deploy DB adapters do not match runtime artifact persistence");
    }
    verify_deploy_db_adapter_source_origins(origin_map, &persistence.db_adapters)?;
    Ok(())
}

pub(crate) fn verify_deploy_db_adapter_contract_keys(
    adapters: &serde_json::Value,
) -> anyhow::Result<()> {
    verify_json_object_keys_exact(
        adapters,
        &["schema_version", "kind", "artifact", "adapters"],
        "deploy DB adapters",
    )?;
    let entries = adapters
        .get("adapters")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| anyhow::anyhow!("deploy DB adapters must include adapters array"))?;
    for (index, adapter) in entries.iter().enumerate() {
        verify_json_object_keys_exact(
            adapter,
            &[
                "kind",
                "mode",
                "provider",
                "env",
                "default",
                "endpoint",
                "adapter_status",
                "source_origin_id",
                "source_origin_ids",
                "runtime",
                "bridge",
            ],
            &format!("deploy DB adapter adapters[{index}]"),
        )?;
        let runtime = adapter.get("runtime").ok_or_else(|| {
            anyhow::anyhow!("deploy DB adapter adapters[{index}].runtime must be an object")
        })?;
        verify_json_object_keys_exact(
            runtime,
            &["status", "query_methods"],
            &format!("deploy DB adapter adapters[{index}].runtime"),
        )?;
        let bridge = adapter.get("bridge").ok_or_else(|| {
            anyhow::anyhow!("deploy DB adapter adapters[{index}].bridge must be an object")
        })?;
        verify_json_object_keys_exact(
            bridge,
            &[
                "contract",
                "method",
                "content_type",
                "query_methods",
                "body",
                "retry",
                "env",
            ],
            &format!("deploy DB adapter adapters[{index}].bridge"),
        )?;
        let body = bridge.get("body").ok_or_else(|| {
            anyhow::anyhow!("deploy DB adapter adapters[{index}].bridge.body must be an object")
        })?;
        verify_json_object_keys_exact(
            body,
            &["kind", "contract", "provider", "url", "method", "args"],
            &format!("deploy DB adapter adapters[{index}].bridge.body"),
        )?;
        let retry = bridge.get("retry").ok_or_else(|| {
            anyhow::anyhow!("deploy DB adapter adapters[{index}].bridge.retry must be an object")
        })?;
        verify_json_object_keys_exact(
            retry,
            &["attempts", "on"],
            &format!("deploy DB adapter adapters[{index}].bridge.retry"),
        )?;
        verify_deploy_provider_env_contract_keys(
            bridge.get("env"),
            &format!("deploy DB adapter adapters[{index}].bridge.env"),
        )?;
    }
    Ok(())
}

pub(crate) fn verify_deploy_commerce_adapter_contract_keys(
    adapters: &serde_json::Value,
) -> anyhow::Result<()> {
    verify_json_object_keys_exact(
        adapters,
        &["schema_version", "kind", "artifact", "adapters"],
        "deploy commerce adapters",
    )?;
    let entries = adapters
        .get("adapters")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| anyhow::anyhow!("deploy commerce adapters must include adapters array"))?;
    for (index, adapter) in entries.iter().enumerate() {
        let mut expected = vec![
            "kind",
            "surface",
            "package",
            "provider_package",
            "mode",
            "env",
            "default",
            "endpoint",
            "record_path",
            "source_origin_id",
            "source_origin_ids",
            "request",
        ];
        if adapter.get("provider").is_some() {
            expected.push("provider");
        }
        if adapter.get("provider_env").is_some() {
            expected.push("provider_env");
        }
        verify_json_object_keys_exact(
            adapter,
            &expected,
            &format!("deploy commerce adapter adapters[{index}]"),
        )?;
        verify_deploy_commerce_adapter_surface(adapter, index)?;
        let request = adapter.get("request").ok_or_else(|| {
            anyhow::anyhow!("deploy commerce adapter adapters[{index}].request must be an object")
        })?;
        verify_json_object_keys_exact(
            request,
            &["method", "content_type", "kind", "body"],
            &format!("deploy commerce adapter adapters[{index}].request"),
        )?;
        let body = request.get("body").ok_or_else(|| {
            anyhow::anyhow!(
                "deploy commerce adapter adapters[{index}].request.body must be an object"
            )
        })?;
        verify_json_object_keys_exact(
            body,
            &["kind", "payload"],
            &format!("deploy commerce adapter adapters[{index}].request.body"),
        )?;
        if adapter.get("provider_env").is_some() {
            verify_deploy_provider_env_contract_keys(
                adapter.get("provider_env"),
                &format!("deploy commerce adapter adapters[{index}].provider_env"),
            )?;
        }
    }
    Ok(())
}

pub(crate) fn verify_deploy_commerce_adapter_surface(
    adapter: &serde_json::Value,
    index: usize,
) -> anyhow::Result<()> {
    let context = format!("deploy commerce adapter adapters[{index}]");
    let kind = json_str(adapter, "kind", &context)?;
    let surface = json_str(adapter, "surface", &context)?;
    let expected = deploy_commerce_adapter_surface(kind);
    if surface != expected {
        anyhow::bail!("{context} surface must be {expected} for {kind}");
    }
    if orv_hir::domain_surface(kind) != orv_hir::DomainSurface::LibraryProviderPackage {
        anyhow::bail!("{context} kind {kind} must be library/provider package surface");
    }
    let package = json_str(adapter, "package", &context)?;
    let expected_package = deploy_commerce_adapter_package(kind);
    if package != expected_package {
        anyhow::bail!("{context} package must be {expected_package} for {kind}");
    }
    let provider_package = adapter.get("provider_package").ok_or_else(|| {
        anyhow::anyhow!("{context}.provider_package must be present as string or null")
    })?;
    if let Some(provider) = adapter.get("provider").and_then(serde_json::Value::as_str) {
        let expected_provider_package =
            deploy_commerce_provider_package(provider).ok_or_else(|| {
                anyhow::anyhow!("{context} provider {provider} has no known provider package")
            })?;
        if provider_package.as_str() != Some(expected_provider_package) {
            anyhow::bail!(
                "{context} provider_package must be {expected_provider_package} for {provider}"
            );
        }
    } else if !provider_package.is_null() {
        anyhow::bail!("{context} provider_package must be null without provider");
    }
    Ok(())
}

pub(crate) fn verify_deploy_provider_env_contract_keys(
    envs: Option<&serde_json::Value>,
    context: &str,
) -> anyhow::Result<()> {
    let envs = envs
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| anyhow::anyhow!("{context} must be an array"))?;
    for (index, env) in envs.iter().enumerate() {
        verify_json_object_keys_exact(
            env,
            &["env", "required", "purpose"],
            &format!("{context}[{index}]"),
        )?;
    }
    Ok(())
}

pub(crate) fn verify_deploy_db_adapter_source_origins(
    origin_map: &orv_compiler::OriginMap,
    adapters: &[DeployDbAdapter],
) -> anyhow::Result<()> {
    let entries_by_id = origin_entries_by_id(origin_map);
    for adapter in adapters {
        if adapter.source_origin_ids.is_empty() {
            let provider = &adapter.provider;
            anyhow::bail!("deploy DB adapter {provider} is missing source_origin_ids");
        }
        for origin_id in &adapter.source_origin_ids {
            verify_deploy_adapter_source_origin(
                &entries_by_id,
                origin_id,
                "deploy DB adapter",
                "@db.connect",
            )?;
        }
    }
    Ok(())
}

pub(crate) fn verify_deploy_commerce_adapter_source_origins(
    origin_map: &orv_compiler::OriginMap,
    adapters: &[DeployCommerceAdapter],
) -> anyhow::Result<()> {
    let entries_by_id = origin_entries_by_id(origin_map);
    for adapter in adapters {
        if adapter.source_origin_ids.is_empty() {
            let kind = &adapter.kind;
            anyhow::bail!("deploy commerce adapter {kind} is missing source_origin_ids");
        }
        let expected_call = format!("@{}.connect", adapter.kind);
        if deploy_commerce_adapter_kind_for_call(&expected_call) != Some(adapter.kind.as_str()) {
            let kind = &adapter.kind;
            anyhow::bail!("deploy commerce adapter {kind} has unknown source kind");
        }
        let context = format!("deploy commerce adapter {}", adapter.kind);
        for origin_id in &adapter.source_origin_ids {
            verify_deploy_adapter_source_origin(
                &entries_by_id,
                origin_id,
                &context,
                &expected_call,
            )?;
        }
    }
    Ok(())
}

pub(crate) fn verify_deploy_adapter_source_origin(
    entries_by_id: &HashMap<&str, &orv_compiler::OriginEntry>,
    origin_id: &str,
    context: &str,
    expected_call: &str,
) -> anyhow::Result<()> {
    let Some(entry) = entries_by_id.get(origin_id).copied() else {
        anyhow::bail!("{context} source_origin_id `{origin_id}` not found in origin-map.json");
    };
    if entry.kind != "call" || entry.name != expected_call {
        anyhow::bail!(
            "{context} source_origin_id `{origin_id}` must reference origin-map call {expected_call}"
        );
    }
    Ok(())
}

pub(crate) fn verify_deploy_runbook_artifact(
    dir: &Path,
    path: &str,
    artifacts: &DeployRunbookArtifacts<'_>,
    artifact: &orv_compiler::ServerRuntimeArtifact,
    persistence: &DeployPersistence,
    client: Option<&serde_json::Value>,
) -> anyhow::Result<()> {
    let runbook_path = dir.join(path);
    if !runbook_path.is_file() {
        anyhow::bail!("missing deploy runbook: {}", runbook_path.display());
    }
    let runbook = std::fs::read_to_string(&runbook_path)
        .map_err(|e| anyhow::anyhow!("failed to read {}: {e}", runbook_path.display()))?;
    let compose_command = format!("docker compose -f {} up --build -d", artifacts.compose);
    if !runbook.contains(&compose_command) {
        anyhow::bail!("deploy runbook must include compose launch command");
    }
    if !runbook.contains(artifacts.routes) {
        let routes_artifact = artifacts.routes;
        anyhow::bail!("deploy runbook must reference {routes_artifact}");
    }
    if !runbook.contains(artifacts.env_example) {
        let env_example_path = artifacts.env_example;
        anyhow::bail!("deploy runbook must reference {env_example_path}");
    }
    if !runbook.contains(artifacts.db_adapters) {
        let db_adapters_path = artifacts.db_adapters;
        anyhow::bail!("deploy runbook must reference {db_adapters_path}");
    }
    if !runbook.contains(artifacts.commerce_adapters) {
        let commerce_adapters_path = artifacts.commerce_adapters;
        anyhow::bail!("deploy runbook must reference {commerce_adapters_path}");
    }
    if !runbook.contains(artifacts.smoke_test) {
        let smoke_test_path = artifacts.smoke_test;
        anyhow::bail!("deploy runbook must reference {smoke_test_path}");
    }
    if !runbook.contains(artifacts.smoke_output) {
        let smoke_output_path = artifacts.smoke_output;
        anyhow::bail!("deploy runbook must reference {smoke_output_path}");
    }
    if !runbook.contains(artifacts.preflight) {
        let preflight_path = artifacts.preflight;
        anyhow::bail!("deploy runbook must reference {preflight_path}");
    }
    if !runbook.contains(artifacts.benchmark_evidence) {
        let benchmark_evidence_path = artifacts.benchmark_evidence;
        anyhow::bail!("deploy runbook must reference {benchmark_evidence_path}");
    }
    if !runbook.contains(artifacts.participant_notes_template) {
        let participant_notes_template_path = artifacts.participant_notes_template;
        anyhow::bail!(
            "deploy runbook must reference participant notes template {participant_notes_template_path}"
        );
    }
    let smoke_command = format!("./{}", artifacts.smoke_test);
    if !runbook.contains(&smoke_command) {
        anyhow::bail!("deploy runbook must document deploy smoke test command");
    }
    if !runbook.contains("## Benchmark Evidence") {
        anyhow::bail!("deploy runbook must document benchmark evidence capture");
    }
    if !runbook.contains("## Participant Notes Template") {
        anyhow::bail!("deploy runbook must document participant notes template");
    }
    if !runbook.contains("## Smoke Output Markers") {
        anyhow::bail!("deploy runbook must document smoke output markers");
    }
    for marker in deploy_benchmark::SMOKE_REQUIRED_MARKERS {
        let marker_line = format!("- `{marker}`");
        if !runbook.contains(&marker_line) {
            anyhow::bail!("deploy runbook must document smoke output marker {marker}");
        }
    }
    if !runbook.contains("orv benchmark-report .") {
        anyhow::bail!("deploy runbook must document benchmark report command");
    }
    if !runbook.contains("orv benchmark-prepare . --participants 2") {
        anyhow::bail!("deploy runbook must document benchmark prepare command");
    }
    if !runbook.contains("orv editor run-debug . --control next") {
        anyhow::bail!("deploy runbook must document DAP production summary command");
    }
    if !runbook.contains("orv benchmark-report . --require-pass") {
        anyhow::bail!("deploy runbook must document benchmark report require-pass command");
    }
    if !runbook.contains("./deploy/server.sh --trace deploy/request-trace.json") {
        anyhow::bail!("deploy runbook must document request trace capture command");
    }
    if !runbook.contains("orv editor trace . --trace deploy/request-trace.json") {
        anyhow::bail!("deploy runbook must document editor trace navigation command");
    }
    if !runbook.contains("ORV_SMOKE_TRACE_STREAM=1 ./deploy/smoke-test.sh") {
        anyhow::bail!("deploy runbook must document trace stream smoke command");
    }
    if !runbook.contains("orv deploy-env-check .") {
        anyhow::bail!("deploy runbook must document deploy env preflight command");
    }
    if !runbook.contains("orv verify-build .") {
        anyhow::bail!("deploy runbook must document build verification preflight command");
    }
    if !runbook.contains("cargo build --manifest-path server/native/Cargo.toml --release") {
        anyhow::bail!("deploy runbook must document native launcher build command");
    }
    if !runbook.contains("ORV_BUILD_DIR=. ./server/native/target/release/orv-native-server") {
        anyhow::bail!("deploy runbook must document native launcher run command");
    }
    if !runbook.contains("docker build -f server/native/Dockerfile -t orv-native-server:latest .") {
        anyhow::bail!("deploy runbook must document native runtime image build command");
    }
    if !runbook.contains("ORV_BUILD_DIR is an explicit override") {
        anyhow::bail!("deploy runbook must document native launcher build-dir inference");
    }
    if !runbook.contains("/__orv/trace/events") {
        anyhow::bail!("deploy runbook must document live trace event stream endpoint");
    }
    verify_deploy_runbook_client_section(&runbook, client)?;
    for path in &persistence.wal_paths {
        if !runbook.contains(path) {
            anyhow::bail!("deploy runbook must document persistent WAL path {path}");
        }
    }
    for path in &persistence.db_paths {
        if !runbook.contains(path) {
            anyhow::bail!("deploy runbook must document persistent DB path {path}");
        }
    }
    for endpoint in &persistence.db_endpoints {
        if !runbook.contains(endpoint) {
            anyhow::bail!("deploy runbook must document DB endpoint {endpoint}");
        }
    }
    for env in &persistence.db_env {
        if !runbook.contains(&env.env) {
            let variable = &env.env;
            anyhow::bail!("deploy runbook must document DB adapter env {variable}");
        }
        if let Some(default) = &env.default {
            if !runbook.contains(default) {
                anyhow::bail!("deploy runbook must document DB adapter env default {default}");
            }
        }
    }
    for path in &persistence.record_paths {
        if !runbook.contains(path) {
            anyhow::bail!("deploy runbook must document commerce record path {path}");
        }
    }
    for endpoint in &persistence.commerce_endpoints {
        if !runbook.contains(endpoint) {
            anyhow::bail!("deploy runbook must document commerce endpoint {endpoint}");
        }
    }
    for env in &persistence.commerce_env {
        if !runbook.contains(&env.env) {
            let variable = &env.env;
            anyhow::bail!("deploy runbook must document commerce endpoint env {variable}");
        }
        if let Some(default) = &env.default {
            if !runbook.contains(default) {
                anyhow::bail!(
                    "deploy runbook must document commerce endpoint env default {default}"
                );
            }
        }
    }
    for adapter in &persistence.commerce_adapters {
        let Some(provider) = &adapter.provider else {
            continue;
        };
        for env in &adapter.provider_env {
            let required = if env.required { "required" } else { "optional" };
            let line = format!(
                "- Commerce provider env: {} {provider} {} {required} {}",
                adapter.kind, env.env, env.purpose
            );
            if !runbook.contains(&line) {
                anyhow::bail!("deploy runbook must document {line}");
            }
        }
    }
    let has_provider_env = persistence
        .commerce_adapters
        .iter()
        .any(|adapter| !adapter.provider_env.is_empty());
    if has_provider_env {
        for line in [
            "- Secret store: supply commerce provider credentials through deployment secret manager or vault values, not deploy/env.example.",
            "- Stripe webhook rotation: set STRIPE_WEBHOOK_SECRET to the new value and STRIPE_WEBHOOK_SECRET_PREVIOUS to the previous value during overlap.",
            "- Stripe replay window: STRIPE_WEBHOOK_TOLERANCE_SECONDS defaults to 300 seconds; override only with provider runbook approval.",
            "- Provider replay: payment and shipping calls use stable idempotency keys; inspect provider records before retrying checkout compensation.",
        ] {
            if !runbook.contains(line) {
                anyhow::bail!("deploy runbook must document provider operation {line}");
            }
        }
    }
    let has_db_bridge_env = persistence
        .db_adapters
        .iter()
        .any(|adapter| !adapter.bridge_env.is_empty());
    if has_db_bridge_env {
        for line in [
            "- DB bridge secret store: supply ORV_DB_ADAPTER_*_AUTH_TOKEN values through deployment secret manager or vault values, not deploy/env.example.",
            "- DB bridge rotation: prefer provider-specific auth token envs before ORV_DB_ADAPTER_AUTH_TOKEN fallback during rotation.",
            "- DB bridge replay: bridge calls use bounded transient retry; confirm provider-side idempotency before replaying writes.",
        ] {
            if !runbook.contains(line) {
                anyhow::bail!("deploy runbook must document DB bridge operation {line}");
            }
        }
    }
    for volume in &persistence.volumes {
        if !runbook.contains(&volume.compose_mount) {
            let mount = &volume.compose_mount;
            anyhow::bail!("deploy runbook must document persistent volume {mount}");
        }
    }
    if let Some(port) = deploy_runbook_port_assignment(artifact.listen.as_ref()) {
        if !runbook.contains(&port) {
            anyhow::bail!("deploy runbook must document {port}");
        }
    }
    for route in &artifact.routes {
        let route_line = format!("- {} {}", route.method, route.path);
        if !runbook.contains(&route_line) {
            let method = &route.method;
            let path = &route.path;
            anyhow::bail!("deploy runbook must list route {method} {path}");
        }
    }
    let expected = deploy_runbook_content(artifacts, artifact, persistence, client);
    if runbook != expected {
        anyhow::bail!("deploy runbook must match generated artifact");
    }
    Ok(())
}

pub(crate) fn verify_deploy_listen_value(
    actual: Option<&serde_json::Value>,
    expected: Option<&orv_compiler::ServerListenArtifact>,
    label: &str,
) -> anyhow::Result<()> {
    let expected = serde_json::to_value(expected)?;
    if actual != Some(&expected) {
        anyhow::bail!("{label} listen does not match runtime artifact");
    }
    Ok(())
}

pub(crate) fn verify_deploy_dockerfile(
    dir: &Path,
    path: &str,
    runtime_image: &str,
    listen: Option<&orv_compiler::ServerListenArtifact>,
) -> anyhow::Result<()> {
    let dockerfile_path = dir.join(path);
    if !dockerfile_path.is_file() {
        anyhow::bail!("missing deploy Dockerfile: {}", dockerfile_path.display());
    }
    let dockerfile = std::fs::read_to_string(&dockerfile_path)
        .map_err(|e| anyhow::anyhow!("failed to read {}: {e}", dockerfile_path.display()))?;
    let expected_runtime_image = format!("ARG ORV_RUNTIME_IMAGE={runtime_image}");
    if !dockerfile.contains(&expected_runtime_image) {
        anyhow::bail!("deploy Dockerfile must declare {expected_runtime_image}");
    }
    if !dockerfile.contains("FROM ${ORV_RUNTIME_IMAGE}") {
        anyhow::bail!("deploy Dockerfile must use ORV_RUNTIME_IMAGE");
    }
    if !dockerfile.contains("COPY . /app") {
        anyhow::bail!("deploy Dockerfile must copy build output into /app");
    }
    if let Some(port) = deploy_exposed_port(listen) {
        let expected = format!("EXPOSE {port}");
        if !dockerfile.contains(&expected) {
            anyhow::bail!("deploy Dockerfile must expose {port}");
        }
    }
    if !dockerfile.contains(r#"ENTRYPOINT ["./deploy/server.sh"]"#) {
        anyhow::bail!("deploy Dockerfile must run ./deploy/server.sh");
    }
    let expected = deploy_dockerfile_content(runtime_image, listen);
    if dockerfile != expected {
        anyhow::bail!("deploy Dockerfile must match generated artifact");
    }
    Ok(())
}

pub(crate) fn verify_deploy_routes_artifact(
    dir: &Path,
    path: &str,
    artifact_path: &str,
    runtime: &str,
    artifact: &orv_compiler::ServerRuntimeArtifact,
) -> anyhow::Result<()> {
    let routes_path = dir.join(path);
    if !routes_path.is_file() {
        anyhow::bail!("missing deploy routes artifact: {}", routes_path.display());
    }
    let routes = read_json_value(&routes_path)?;
    verify_json_object_keys_exact(
        &routes,
        &[
            "schema_version",
            "artifact",
            "runtime",
            "protocol",
            "routes",
        ],
        "deploy routes",
    )?;
    if routes
        .get("schema_version")
        .and_then(serde_json::Value::as_u64)
        != Some(1)
    {
        anyhow::bail!("deploy routes schema_version must be 1");
    }
    if json_str(&routes, "artifact", "deploy routes")? != artifact_path {
        anyhow::bail!("deploy routes artifact must be {artifact_path}");
    }
    if json_str(&routes, "runtime", "deploy routes")? != runtime {
        anyhow::bail!("deploy routes runtime does not match runtime artifact");
    }
    if json_str(&routes, "protocol", "deploy routes")? != "http1" {
        anyhow::bail!("deploy routes protocol must be http1");
    }
    let expected_routes = serde_json::to_value(&artifact.routes)?;
    if routes.get("routes") != Some(&expected_routes) {
        anyhow::bail!("deploy routes do not match runtime artifact");
    }
    Ok(())
}

pub(crate) fn verify_deploy_static_target(
    dir: &Path,
    static_target: Option<&serde_json::Value>,
) -> anyhow::Result<()> {
    let plan = read_json_value(&dir.join("bundle-plan.json"))?;
    let bundles = plan
        .get("bundles")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| anyhow::anyhow!("bundle plan bundles must be an array"))?;
    let static_bundle = bundles.iter().find(|bundle| {
        bundle.get("kind").and_then(serde_json::Value::as_str) == Some("static_page")
    });
    let Some(static_target) = static_target.filter(|value| !value.is_null()) else {
        if static_bundle.is_some() {
            anyhow::bail!("deploy static target missing for bundle static_page");
        }
        return Ok(());
    };
    verify_json_object_keys_exact(
        static_target,
        &["path", "runtime_features"],
        "deploy static",
    )?;
    let path = json_str(static_target, "path", "deploy static")?;
    let Some(static_bundle) = static_bundle else {
        anyhow::bail!("deploy static target exists without bundle static_page target");
    };
    if json_str(static_bundle, "path", "bundle target")? != path {
        anyhow::bail!("deploy static path does not match bundle static_page target");
    }
    let target = dir.join(path);
    if !target.is_file() {
        anyhow::bail!("missing deploy static target: {}", target.display());
    }
    let runtime_features = static_target
        .get("runtime_features")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| anyhow::anyhow!("deploy static runtime_features must be an array"))?;
    if !runtime_features.is_empty() {
        anyhow::bail!("deploy static target must be zero-runtime");
    }
    verify_static_page_target(static_bundle, &target)
}
