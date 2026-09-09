use super::*;

pub(crate) fn verify_deploy_env_example_artifact(
    dir: &Path,
    path: &str,
    listen: Option<&orv_compiler::ServerListenArtifact>,
    persistence: &DeployPersistence,
) -> anyhow::Result<()> {
    let env_path = dir.join(path);
    if !env_path.is_file() {
        anyhow::bail!("missing deploy env example: {}", env_path.display());
    }
    let env_example = std::fs::read_to_string(&env_path)
        .map_err(|e| anyhow::anyhow!("failed to read {}: {e}", env_path.display()))?;
    for assignment in deploy_env_example_assignments(listen, persistence) {
        if !env_example.contains(&assignment) {
            anyhow::bail!("deploy env example must include {assignment}");
        }
    }
    let expected = deploy_env_example_content(listen, persistence);
    if env_example != expected {
        anyhow::bail!("deploy env example must match generated artifact");
    }
    Ok(())
}

pub(crate) fn verify_deploy_preflight_artifact(
    dir: &Path,
    path: &str,
    artifacts: &DeployRunbookArtifacts<'_>,
    artifact: &orv_compiler::ServerRuntimeArtifact,
    persistence: &DeployPersistence,
    client: Option<&serde_json::Value>,
) -> anyhow::Result<()> {
    let preflight_path = dir.join(path);
    if !preflight_path.is_file() {
        anyhow::bail!(
            "missing deploy preflight artifact: {}",
            preflight_path.display()
        );
    }
    let preflight = read_json_value(&preflight_path)?;
    if preflight
        .get("schema_version")
        .and_then(serde_json::Value::as_u64)
        != Some(1)
    {
        anyhow::bail!("deploy preflight schema_version must be 1");
    }
    if json_str(&preflight, "kind", "deploy preflight")? != "orv.deploy.preflight" {
        anyhow::bail!("deploy preflight kind must be orv.deploy.preflight");
    }
    verify_json_object_keys_exact(
        &preflight,
        &[
            "schema_version",
            "kind",
            "artifact",
            "runtime",
            "runtime_features",
            "security_features",
            "listen",
            "routes",
            "persistence",
            "required_env",
            "optional_env",
            "commands",
            "artifacts",
            "smoke_output_contract",
            "benchmark",
            "client",
        ],
        "deploy preflight",
    )?;
    verify_json_pointer_str(
        &preflight,
        "/artifact",
        artifacts.server_artifact,
        "deploy preflight artifact",
    )?;
    let commands = preflight
        .get("commands")
        .ok_or_else(|| anyhow::anyhow!("deploy preflight commands must be an object"))?;
    verify_json_object_keys_exact(
        commands,
        &[
            "verify_build",
            "env_check",
            "run_build",
            "smoke_test",
            "editor_run_debug",
            "benchmark_prepare",
            "benchmark_report",
            "benchmark_report_require_pass",
            "compose_up",
            "trace",
            "trace_run_build",
            "editor_trace",
            "trace_stream_smoke",
        ],
        "deploy preflight commands",
    )?;
    verify_json_pointer_str(
        &preflight,
        "/commands/verify_build",
        "orv verify-build .",
        "deploy preflight verify_build command",
    )?;
    verify_json_pointer_str(
        &preflight,
        "/commands/env_check",
        "orv deploy-env-check .",
        "deploy preflight env_check command",
    )?;
    verify_json_pointer_str(
        &preflight,
        "/commands/run_build",
        "orv run-build .",
        "deploy preflight run_build command",
    )?;
    verify_json_pointer_str(
        &preflight,
        "/commands/smoke_test",
        &format!("./{}", artifacts.smoke_test),
        "deploy preflight smoke_test command",
    )?;
    verify_json_pointer_str(
        &preflight,
        "/commands/editor_run_debug",
        "orv editor run-debug . --control next",
        "deploy preflight editor_run_debug command",
    )?;
    verify_json_pointer_str(
        &preflight,
        "/commands/benchmark_prepare",
        "orv benchmark-prepare . --participants 2",
        "deploy preflight benchmark_prepare command",
    )?;
    verify_json_pointer_str(
        &preflight,
        "/commands/benchmark_report",
        "orv benchmark-report .",
        "deploy preflight benchmark_report command",
    )?;
    verify_json_pointer_str(
        &preflight,
        "/commands/benchmark_report_require_pass",
        "orv benchmark-report . --require-pass",
        "deploy preflight benchmark_report_require_pass command",
    )?;
    verify_json_pointer_str(
        &preflight,
        "/commands/compose_up",
        &format!("docker compose -f {} up --build -d", artifacts.compose),
        "deploy preflight compose_up command",
    )?;
    verify_json_pointer_str(
        &preflight,
        "/commands/trace",
        "./deploy/server.sh --trace deploy/request-trace.json",
        "deploy preflight trace command",
    )?;
    verify_json_pointer_str(
        &preflight,
        "/commands/trace_run_build",
        "orv run-build . --trace deploy/request-trace.json",
        "deploy preflight trace_run_build command",
    )?;
    verify_json_pointer_str(
        &preflight,
        "/commands/editor_trace",
        "orv editor trace . --trace deploy/request-trace.json",
        "deploy preflight editor_trace command",
    )?;
    verify_json_pointer_str(
        &preflight,
        "/commands/trace_stream_smoke",
        "ORV_SMOKE_TRACE_STREAM=1 ./deploy/smoke-test.sh",
        "deploy preflight trace_stream_smoke command",
    )?;
    let artifact_links = preflight
        .get("artifacts")
        .ok_or_else(|| anyhow::anyhow!("deploy preflight artifacts must be an object"))?;
    verify_json_object_keys_exact(
        artifact_links,
        &[
            "server",
            "routes",
            "source_bundle",
            "project_graph",
            "origin_map",
            "build_manifest",
            "bundle_plan",
            "env_example",
            "db_adapters",
            "commerce_adapters",
            "smoke_test",
            "smoke_output",
            "preflight",
            "benchmark_evidence",
            "participant_notes_template",
            "runbook",
        ],
        "deploy preflight artifacts",
    )?;
    for (key, expected) in [
        ("server", artifacts.server_artifact),
        ("routes", artifacts.routes),
        ("source_bundle", SOURCE_BUNDLE_PATH),
        ("project_graph", "project-graph.json"),
        ("origin_map", "origin-map.json"),
        ("build_manifest", "build-manifest.json"),
        ("bundle_plan", "bundle-plan.json"),
        ("env_example", artifacts.env_example),
        ("db_adapters", artifacts.db_adapters),
        ("commerce_adapters", artifacts.commerce_adapters),
        ("smoke_test", artifacts.smoke_test),
        ("smoke_output", artifacts.smoke_output),
        ("preflight", artifacts.preflight),
        ("benchmark_evidence", artifacts.benchmark_evidence),
        (
            "participant_notes_template",
            artifacts.participant_notes_template,
        ),
        ("runbook", artifacts.runbook),
    ] {
        let pointer = format!("/artifacts/{key}");
        verify_json_pointer_str(
            &preflight,
            &pointer,
            expected,
            &format!("deploy preflight artifact {key}"),
        )?;
    }
    verify_deploy_smoke_output_contract_keys(
        preflight.get("smoke_output_contract").ok_or_else(|| {
            anyhow::anyhow!("deploy preflight smoke_output_contract must be an object")
        })?,
        "deploy preflight smoke_output_contract",
    )?;
    if preflight.get("smoke_output_contract")
        != Some(&deploy_smoke_output_contract_value(artifacts))
    {
        anyhow::bail!("deploy preflight smoke_output_contract must match smoke output contract");
    }
    if preflight.get("runtime").and_then(serde_json::Value::as_str)
        != Some(artifact.runtime.as_str())
    {
        anyhow::bail!("deploy preflight runtime does not match runtime artifact");
    }
    if preflight.get("runtime_features") != Some(&serde_json::to_value(&artifact.runtime_features)?)
    {
        anyhow::bail!("deploy preflight runtime_features do not match runtime artifact");
    }
    if preflight.get("security_features")
        != Some(&serde_json::to_value(deploy_security_runtime_features(
            &artifact.runtime_features,
        ))?)
    {
        anyhow::bail!("deploy preflight security_features do not match runtime artifact");
    }
    if preflight.get("listen") != Some(&serde_json::to_value(&artifact.listen)?) {
        anyhow::bail!("deploy preflight listen does not match runtime artifact");
    }
    if preflight.get("routes") != Some(&serde_json::to_value(&artifact.routes)?) {
        anyhow::bail!("deploy preflight routes do not match runtime artifact");
    }
    if preflight.get("persistence") != Some(&deploy_persistence_value(persistence)) {
        anyhow::bail!("deploy preflight persistence does not match runtime artifact");
    }
    let expected_required_env =
        deploy_preflight_env_values(artifact.listen.as_ref(), persistence, true);
    if preflight.get("required_env") != Some(&expected_required_env) {
        anyhow::bail!("deploy preflight required_env does not match runtime artifact");
    }
    let expected_optional_env =
        deploy_preflight_env_values(artifact.listen.as_ref(), persistence, false);
    if preflight.get("optional_env") != Some(&expected_optional_env) {
        anyhow::bail!("deploy preflight optional_env does not match runtime artifact");
    }
    if preflight.get("client") != Some(&deploy_preflight_client_value(client)) {
        anyhow::bail!("deploy preflight client does not match deploy manifest");
    }
    if preflight.get("benchmark") != Some(&deploy_preflight_benchmark_value()) {
        anyhow::bail!("deploy preflight benchmark does not match 5-hour shop contract");
    }
    Ok(())
}
