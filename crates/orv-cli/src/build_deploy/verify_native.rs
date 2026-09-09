use super::*;

pub(crate) fn verify_native_server_plan_target(dir: &Path, target: &Path) -> anyhow::Result<()> {
    let plan = read_json_value(target)?;
    let artifact = read_server_artifact(&dir.join(SERVER_ARTIFACT_PATH))?;
    verify_native_server_plan_value(
        dir,
        &plan,
        SERVER_ARTIFACT_PATH,
        SERVER_LAUNCH_PATH,
        &artifact,
    )
}

pub(crate) fn verify_native_runtime_image_plan_target(
    dir: &Path,
    target: &Path,
) -> anyhow::Result<()> {
    let plan = read_json_value(target)?;
    let artifact = read_server_artifact(&dir.join(SERVER_ARTIFACT_PATH))?;
    verify_native_runtime_image_plan_value(
        &plan,
        SERVER_ARTIFACT_PATH,
        NATIVE_SERVER_PLAN_PATH,
        &artifact,
    )
}

pub(crate) fn verify_native_server_plan_artifact(
    dir: &Path,
    path: &str,
    artifact_path: &str,
    launcher_path: &str,
    artifact: &orv_compiler::ServerRuntimeArtifact,
) -> anyhow::Result<()> {
    let native_plan_path = dir.join(path);
    if !native_plan_path.is_file() {
        anyhow::bail!(
            "missing native server plan artifact: {}",
            native_plan_path.display()
        );
    }
    let plan = read_json_value(&native_plan_path)?;
    verify_native_server_plan_value(dir, &plan, artifact_path, launcher_path, artifact)
}

pub(crate) fn verify_native_runtime_image_plan_artifact(
    dir: &Path,
    path: &str,
    artifact_path: &str,
    native_plan_path: &str,
    artifact: &orv_compiler::ServerRuntimeArtifact,
) -> anyhow::Result<()> {
    let image_plan_path = dir.join(path);
    if !image_plan_path.is_file() {
        anyhow::bail!(
            "missing native runtime image plan artifact: {}",
            image_plan_path.display()
        );
    }
    let plan = read_json_value(&image_plan_path)?;
    verify_native_runtime_image_plan_value(&plan, artifact_path, native_plan_path, artifact)
}

#[allow(clippy::too_many_lines)]
pub(crate) fn verify_native_server_plan_value(
    dir: &Path,
    plan: &serde_json::Value,
    artifact_path: &str,
    launcher_path: &str,
    artifact: &orv_compiler::ServerRuntimeArtifact,
) -> anyhow::Result<()> {
    verify_native_server_plan_contract_keys(plan)?;
    if plan
        .get("schema_version")
        .and_then(serde_json::Value::as_u64)
        != Some(u64::from(orv_compiler::NATIVE_SERVER_PLAN_ARTIFACT_VERSION))
    {
        anyhow::bail!(
            "native server plan schema_version must be {}",
            orv_compiler::NATIVE_SERVER_PLAN_ARTIFACT_VERSION
        );
    }
    if json_str(plan, "kind", "native server plan")? != "native_server_plan" {
        anyhow::bail!("native server plan kind must be native_server_plan");
    }
    let direct_http = orv_compiler::native_server_direct_http_capable(artifact);
    let expected_status = native_server_plan_status(direct_http);
    if json_str(plan, "status", "native server plan")? != expected_status {
        anyhow::bail!("native server plan status must be {expected_status}");
    }
    if json_str(plan, "artifact", "native server plan")? != artifact_path {
        anyhow::bail!("native server plan artifact must be {artifact_path}");
    }
    if json_str(plan, "launcher", "native server plan")? != launcher_path {
        anyhow::bail!("native server plan launcher must be {launcher_path}");
    }
    let source_path = json_str(plan, "source", "native server plan")?;
    verify_native_server_launcher_source(
        &dir.join(source_path),
        artifact_path,
        NATIVE_SERVER_PLAN_PATH,
        artifact,
    )?;
    verify_native_server_plan_routes_source(dir, plan, artifact)?;
    verify_native_server_plan_router_source(dir, plan)?;
    verify_native_server_plan_handlers_source(dir, plan, artifact)?;
    let package_path = json_str(plan, "package", "native server plan")?;
    verify_native_server_launcher_package(&dir.join(package_path))?;
    verify_native_server_plan_runtime_image(plan)?;
    let expected_build = serde_json::json!([
        "cargo",
        "build",
        "--manifest-path",
        package_path,
        "--release"
    ]);
    if plan.pointer("/commands/build") != Some(&expected_build) {
        anyhow::bail!("native server plan build command must match generated launcher package");
    }
    let expected_run_env = serde_json::json!({ "ORV_BUILD_DIR": "." });
    if plan.pointer("/commands/run/env") != Some(&expected_run_env) {
        anyhow::bail!("native server plan run env must set ORV_BUILD_DIR to build directory");
    }
    let expected_run_command = serde_json::json!([NATIVE_SERVER_LAUNCHER_BINARY_PATH]);
    if plan.pointer("/commands/run/command") != Some(&expected_run_command) {
        anyhow::bail!("native server plan run command must match generated launcher binary");
    }
    let launch = read_server_launch_artifact(&dir.join(launcher_path))?;
    if launch.artifact != artifact_path {
        anyhow::bail!("native server plan launcher artifact does not match server artifact");
    }
    if json_str(plan, "runtime", "native server plan")? != artifact.runtime {
        anyhow::bail!("native server plan runtime does not match runtime artifact");
    }
    if plan.get("runtime_features") != Some(&serde_json::to_value(&artifact.runtime_features)?) {
        anyhow::bail!("native server plan runtime_features do not match runtime artifact");
    }
    let target = plan
        .get("target")
        .ok_or_else(|| anyhow::anyhow!("native server plan target must be an object"))?;
    if json_str(target, "kind", "native server plan target")? != "server_binary" {
        anyhow::bail!("native server plan target kind must be server_binary");
    }
    if json_str(target, "path", "native server plan target")? != NATIVE_SERVER_BINARY_PATH {
        anyhow::bail!("native server plan target path must be {NATIVE_SERVER_BINARY_PATH}");
    }
    if json_str(target, "protocol", "native server plan target")? != "http1" {
        anyhow::bail!("native server plan target protocol must be http1");
    }
    let blocked_by = plan
        .get("blocked_by")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| anyhow::anyhow!("native server plan blocked_by must be an array"))?;
    if direct_http
        && blocked_by
            .iter()
            .any(|item| item.as_str() == Some("native-codegen"))
    {
        anyhow::bail!("native server plan direct_http must not be blocked by native-codegen");
    }
    if !direct_http
        && !blocked_by
            .iter()
            .any(|item| item.as_str() == Some("native-codegen"))
    {
        anyhow::bail!("native server plan blocked_by must include native-codegen");
    }
    if direct_http
        && blocked_by
            .iter()
            .any(|item| item.as_str() == Some("native-runtime-image"))
    {
        anyhow::bail!("native server plan direct_http must not be blocked by native-runtime-image");
    }
    if !direct_http
        && !blocked_by
            .iter()
            .any(|item| item.as_str() == Some("native-runtime-image"))
    {
        anyhow::bail!("native server plan blocked_by must include native-runtime-image");
    }
    verify_deploy_listen_value(
        plan.get("listen"),
        artifact.listen.as_ref(),
        "native server plan",
    )?;
    let artifact_routes = serde_json::to_value(&artifact.routes)?;
    if plan.get("routes") != Some(&artifact_routes) {
        anyhow::bail!("native server plan routes do not match runtime artifact");
    }
    Ok(())
}

pub(crate) fn verify_native_server_plan_contract_keys(
    plan: &serde_json::Value,
) -> anyhow::Result<()> {
    verify_json_object_keys_exact(
        plan,
        &[
            "schema_version",
            "kind",
            "status",
            "runtime",
            "runtime_features",
            "artifact",
            "launcher",
            "source",
            "routes_source",
            "router_source",
            "handlers_source",
            "package",
            "runtime_image_plan",
            "target",
            "commands",
            "blocked_by",
            "listen",
            "routes",
        ],
        "native server plan",
    )?;
    verify_json_object_keys_exact(
        plan.get("target")
            .ok_or_else(|| anyhow::anyhow!("native server plan target must be an object"))?,
        &["kind", "path", "protocol"],
        "native server plan target",
    )?;
    let commands = plan
        .get("commands")
        .ok_or_else(|| anyhow::anyhow!("native server plan commands must be an object"))?;
    verify_json_object_keys_exact(commands, &["build", "run"], "native server plan commands")?;
    let run = commands
        .get("run")
        .ok_or_else(|| anyhow::anyhow!("native server plan commands.run must be an object"))?;
    verify_json_object_keys_exact(run, &["env", "command"], "native server plan run command")?;
    verify_json_object_keys_exact(
        run.get("env")
            .ok_or_else(|| anyhow::anyhow!("native server plan run env must be an object"))?,
        &["ORV_BUILD_DIR"],
        "native server plan run env",
    )?;
    verify_native_routes_contract_keys(plan, "native server plan")
}

pub(crate) fn verify_native_server_plan_routes_source(
    dir: &Path,
    plan: &serde_json::Value,
    artifact: &orv_compiler::ServerRuntimeArtifact,
) -> anyhow::Result<()> {
    let routes_source_path = json_str(plan, "routes_source", "native server plan")?;
    if routes_source_path != NATIVE_SERVER_ROUTES_SOURCE_PATH {
        anyhow::bail!(
            "native server plan routes_source must be {NATIVE_SERVER_ROUTES_SOURCE_PATH}"
        );
    }
    verify_native_server_routes_source(&dir.join(routes_source_path), artifact)
}

pub(crate) fn verify_native_server_plan_router_source(
    dir: &Path,
    plan: &serde_json::Value,
) -> anyhow::Result<()> {
    let router_source_path = json_str(plan, "router_source", "native server plan")?;
    if router_source_path != NATIVE_SERVER_ROUTER_SOURCE_PATH {
        anyhow::bail!(
            "native server plan router_source must be {NATIVE_SERVER_ROUTER_SOURCE_PATH}"
        );
    }
    verify_native_server_router_source(&dir.join(router_source_path))
}

pub(crate) fn verify_native_server_plan_handlers_source(
    dir: &Path,
    plan: &serde_json::Value,
    artifact: &orv_compiler::ServerRuntimeArtifact,
) -> anyhow::Result<()> {
    let handlers_source_path = json_str(plan, "handlers_source", "native server plan")?;
    if handlers_source_path != NATIVE_SERVER_HANDLERS_SOURCE_PATH {
        anyhow::bail!(
            "native server plan handlers_source must be {NATIVE_SERVER_HANDLERS_SOURCE_PATH}"
        );
    }
    verify_native_server_handlers_source(&dir.join(handlers_source_path), artifact)
}

pub(crate) fn verify_native_server_plan_runtime_image(
    plan: &serde_json::Value,
) -> anyhow::Result<()> {
    if json_str(plan, "runtime_image_plan", "native server plan")? != NATIVE_RUNTIME_IMAGE_PLAN_PATH
    {
        anyhow::bail!(
            "native server plan runtime_image_plan must be {NATIVE_RUNTIME_IMAGE_PLAN_PATH}"
        );
    }
    Ok(())
}

pub(crate) fn verify_native_runtime_image_plan_value(
    plan: &serde_json::Value,
    artifact_path: &str,
    native_plan_path: &str,
    artifact: &orv_compiler::ServerRuntimeArtifact,
) -> anyhow::Result<()> {
    verify_native_runtime_image_plan_contract_keys(plan)?;
    if plan
        .get("schema_version")
        .and_then(serde_json::Value::as_u64)
        != Some(u64::from(
            orv_compiler::NATIVE_RUNTIME_IMAGE_PLAN_ARTIFACT_VERSION,
        ))
    {
        anyhow::bail!(
            "native runtime image plan schema_version must be {}",
            orv_compiler::NATIVE_RUNTIME_IMAGE_PLAN_ARTIFACT_VERSION
        );
    }
    if json_str(plan, "kind", "native runtime image plan")? != "native_runtime_image_plan" {
        anyhow::bail!("native runtime image plan kind must be native_runtime_image_plan");
    }
    let direct_http = orv_compiler::native_server_direct_http_capable(artifact);
    let expected_status = native_runtime_image_plan_status(direct_http);
    if json_str(plan, "status", "native runtime image plan")? != expected_status {
        anyhow::bail!("native runtime image plan status must be {expected_status}");
    }
    if json_str(plan, "artifact", "native runtime image plan")? != artifact_path {
        anyhow::bail!("native runtime image plan artifact must be {artifact_path}");
    }
    if json_str(plan, "native_plan", "native runtime image plan")? != native_plan_path {
        anyhow::bail!("native runtime image plan native_plan must be {native_plan_path}");
    }
    if json_str(plan, "runtime", "native runtime image plan")? != artifact.runtime {
        anyhow::bail!("native runtime image plan runtime does not match runtime artifact");
    }
    if plan.get("runtime_features") != Some(&serde_json::to_value(&artifact.runtime_features)?) {
        anyhow::bail!("native runtime image plan runtime_features do not match runtime artifact");
    }
    if json_str(plan, "reference_image", "native runtime image plan")?
        != ORV_REFERENCE_RUNTIME_IMAGE
    {
        anyhow::bail!(
            "native runtime image plan reference_image must be {ORV_REFERENCE_RUNTIME_IMAGE}"
        );
    }
    let target = plan
        .get("target")
        .ok_or_else(|| anyhow::anyhow!("native runtime image plan target must be an object"))?;
    if json_str(target, "kind", "native runtime image plan target")? != "oci_image" {
        anyhow::bail!("native runtime image plan target kind must be oci_image");
    }
    if json_str(target, "image", "native runtime image plan target")? != NATIVE_RUNTIME_IMAGE_NAME {
        anyhow::bail!("native runtime image plan target image must be {NATIVE_RUNTIME_IMAGE_NAME}");
    }
    if json_str(target, "binary", "native runtime image plan target")? != NATIVE_SERVER_BINARY_PATH
    {
        anyhow::bail!(
            "native runtime image plan target binary must be {NATIVE_SERVER_BINARY_PATH}"
        );
    }
    if json_str(target, "protocol", "native runtime image plan target")? != "http1" {
        anyhow::bail!("native runtime image plan target protocol must be http1");
    }
    let blocked_by = plan
        .get("blocked_by")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| anyhow::anyhow!("native runtime image plan blocked_by must be an array"))?;
    if direct_http
        && blocked_by
            .iter()
            .any(|item| item.as_str() == Some("native-codegen"))
    {
        anyhow::bail!(
            "native runtime image plan direct_http must not be blocked by native-codegen"
        );
    }
    if !direct_http
        && !blocked_by
            .iter()
            .any(|item| item.as_str() == Some("native-codegen"))
    {
        anyhow::bail!("native runtime image plan blocked_by must include native-codegen");
    }
    if direct_http
        && blocked_by
            .iter()
            .any(|item| item.as_str() == Some("native-runtime-image"))
    {
        anyhow::bail!(
            "native runtime image plan direct_http must not be blocked by native-runtime-image"
        );
    }
    if !direct_http
        && !blocked_by
            .iter()
            .any(|item| item.as_str() == Some("native-runtime-image"))
    {
        anyhow::bail!("native runtime image plan blocked_by must include native-runtime-image");
    }
    if json_str(plan, "dockerfile", "native runtime image plan")?
        != NATIVE_RUNTIME_IMAGE_DOCKERFILE_PATH
    {
        anyhow::bail!(
            "native runtime image plan dockerfile must be {NATIVE_RUNTIME_IMAGE_DOCKERFILE_PATH}"
        );
    }
    if plan.pointer("/commands/build")
        != Some(&serde_json::json!([
            "docker",
            "build",
            "-f",
            NATIVE_RUNTIME_IMAGE_DOCKERFILE_PATH,
            "-t",
            NATIVE_RUNTIME_IMAGE_NAME,
            "."
        ]))
    {
        anyhow::bail!("native runtime image plan build command must match generated Dockerfile");
    }
    verify_deploy_listen_value(
        plan.get("listen"),
        artifact.listen.as_ref(),
        "native runtime image plan",
    )?;
    let artifact_routes = serde_json::to_value(&artifact.routes)?;
    if plan.get("routes") != Some(&artifact_routes) {
        anyhow::bail!("native runtime image plan routes do not match runtime artifact");
    }
    Ok(())
}

pub(crate) fn verify_native_runtime_image_plan_contract_keys(
    plan: &serde_json::Value,
) -> anyhow::Result<()> {
    verify_json_object_keys_exact(
        plan,
        &[
            "schema_version",
            "kind",
            "status",
            "runtime",
            "runtime_features",
            "artifact",
            "native_plan",
            "reference_image",
            "target",
            "dockerfile",
            "commands",
            "blocked_by",
            "listen",
            "routes",
        ],
        "native runtime image plan",
    )?;
    verify_json_object_keys_exact(
        plan.get("target")
            .ok_or_else(|| anyhow::anyhow!("native runtime image plan target must be an object"))?,
        &["kind", "image", "binary", "protocol"],
        "native runtime image plan target",
    )?;
    verify_json_object_keys_exact(
        plan.get("commands").ok_or_else(|| {
            anyhow::anyhow!("native runtime image plan commands must be an object")
        })?,
        &["build"],
        "native runtime image plan commands",
    )?;
    verify_native_routes_contract_keys(plan, "native runtime image plan")
}

pub(crate) fn verify_native_routes_contract_keys(
    value: &serde_json::Value,
    label: &str,
) -> anyhow::Result<()> {
    let routes = value
        .get("routes")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| anyhow::anyhow!("{label} routes must be an array"))?;
    for (index, route) in routes.iter().enumerate() {
        let context = format!("{label} routes[{index}]");
        verify_json_object_keys_allowing_optional(
            route,
            &["method", "path", "origin_id"],
            &["response_origin_ids", "responses", "policies"],
            &context,
        )?;
        if let Some(responses) = route.get("responses") {
            let responses = responses
                .as_array()
                .ok_or_else(|| anyhow::anyhow!("{context}.responses must be an array"))?;
            for (response_index, response) in responses.iter().enumerate() {
                verify_native_response_contract_keys(
                    response,
                    &format!("{context}.responses[{response_index}]"),
                )?;
            }
        }
    }
    Ok(())
}

pub(crate) fn verify_native_response_contract_keys(
    response: &serde_json::Value,
    context: &str,
) -> anyhow::Result<()> {
    verify_json_object_keys_allowing_optional(
        response,
        &["origin_id", "body_kind"],
        &[
            "status",
            "condition",
            "body_json",
            "body_object_fields",
            "body_route_params",
            "body_query_params",
            "body_request_json",
            "body_request_fields",
        ],
        context,
    )
}

pub(crate) fn verify_native_runtime_image_dockerfile(target: &Path) -> anyhow::Result<()> {
    if !target.is_file() {
        anyhow::bail!(
            "missing native runtime image Dockerfile: {}",
            target.display()
        );
    }
    let source = std::fs::read_to_string(target)
        .map_err(|e| anyhow::anyhow!("failed to read {}: {e}", target.display()))?;
    if source != NATIVE_RUNTIME_IMAGE_DOCKERFILE_SOURCE {
        anyhow::bail!("native runtime image Dockerfile must match generated Dockerfile");
    }
    Ok(())
}

pub(crate) fn verify_native_server_launcher_package(target: &Path) -> anyhow::Result<()> {
    if !target.is_file() {
        anyhow::bail!(
            "missing native server launcher package: {}",
            target.display()
        );
    }
    let source = std::fs::read_to_string(target)
        .map_err(|e| anyhow::anyhow!("failed to read {}: {e}", target.display()))?;
    let manifest = toml::from_str::<toml::Value>(&source)
        .map_err(|e| anyhow::anyhow!("failed to parse {}: {e}", target.display()))?;
    let package = manifest
        .get("package")
        .and_then(toml::Value::as_table)
        .ok_or_else(|| anyhow::anyhow!("native server launcher package must have [package]"))?;
    if package.get("name").and_then(toml::Value::as_str) != Some("orv-native-server") {
        anyhow::bail!("native server launcher package name must be orv-native-server");
    }
    if package.get("edition").and_then(toml::Value::as_str) != Some("2021") {
        anyhow::bail!("native server launcher package edition must be 2021");
    }
    if package.get("publish").and_then(toml::Value::as_bool) != Some(false) {
        anyhow::bail!("native server launcher package publish must be false");
    }
    if manifest
        .get("dependencies")
        .and_then(toml::Value::as_table)
        .is_some_and(|dependencies| !dependencies.is_empty())
    {
        anyhow::bail!("native server launcher package must not declare dependencies");
    }
    let bins = manifest
        .get("bin")
        .and_then(toml::Value::as_array)
        .ok_or_else(|| anyhow::anyhow!("native server launcher package must declare [[bin]]"))?;
    let bin = bins
        .iter()
        .find_map(toml::Value::as_table)
        .ok_or_else(|| anyhow::anyhow!("native server launcher package bin must be a table"))?;
    if bin.get("name").and_then(toml::Value::as_str) != Some("orv-native-server") {
        anyhow::bail!("native server launcher package bin name must be orv-native-server");
    }
    if bin.get("path").and_then(toml::Value::as_str) != Some("main.rs") {
        anyhow::bail!("native server launcher package bin path must be main.rs");
    }
    Ok(())
}

pub(crate) fn verify_native_server_launcher_source(
    target: &Path,
    artifact_path: &str,
    native_plan_path: &str,
    artifact: &orv_compiler::ServerRuntimeArtifact,
) -> anyhow::Result<()> {
    if !target.is_file() {
        anyhow::bail!(
            "missing native server launcher source: {}",
            target.display()
        );
    }
    let source = std::fs::read_to_string(target)
        .map_err(|e| anyhow::anyhow!("failed to read {}: {e}", target.display()))?;
    let artifact_line = format!(r#"const ORV_SERVER_ARTIFACT: &str = "{artifact_path}";"#);
    if !source.contains(&artifact_line) {
        anyhow::bail!("native server launcher source must reference {artifact_path}");
    }
    let plan_line = format!(r#"const ORV_NATIVE_SERVER_PLAN: &str = "{native_plan_path}";"#);
    if !source.contains(&plan_line) {
        anyhow::bail!("native server launcher source must reference {native_plan_path}");
    }
    if !source.contains("build_dir.join(ORV_NATIVE_SERVER_PLAN)")
        || !source.contains("native_plan.is_file()")
    {
        anyhow::bail!("native server launcher source must validate native server plan");
    }
    if !source.contains("fn orv_build_dir() -> std::path::PathBuf")
        || !source.contains(r#"std::env::var_os("ORV_BUILD_DIR")"#)
        || !source.contains("std::env::current_exe()")
        || !source.contains("path.parent()?.parent()?.parent()?.parent()?.parent()")
    {
        anyhow::bail!("native server launcher source must infer build dir from executable path");
    }
    if !source.contains("build_dir.join(ORV_SERVER_ARTIFACT)")
        || !source.contains("artifact.is_file()")
    {
        anyhow::bail!("native server launcher source must validate server artifact");
    }
    let expected =
        orv_compiler::native_server_launcher_source(artifact_path, native_plan_path, artifact);
    if expected.contains("fn orv_native_serve() -> std::io::Result<()>") {
        if !source.contains("fn orv_native_serve() -> std::io::Result<()>")
            || !source.contains("std::net::TcpListener::bind(orv_native_listen_address())")
        {
            anyhow::bail!("native server launcher source must serve HTTP directly");
        }
        if !source.contains("router::orv_native_dispatch_with_request(")
            || !source.contains("request.body")
        {
            anyhow::bail!("native server launcher source must dispatch through generated router");
        }
        if !source.contains("fn orv_native_http_response(") {
            anyhow::bail!("native server launcher source must serialize native HTTP responses");
        }
        if source.contains(r#"Command::new("orv")"#) || source.contains(r#".arg("run-artifact")"#) {
            anyhow::bail!(
                "native server launcher source must not shell through `orv run-artifact`"
            );
        }
    } else {
        if !source.contains("fn orv_native_reference_bridge(")
            || !source.contains(r#"Command::new("orv")"#)
            || !source.contains(r#".arg("run-artifact")"#)
        {
            anyhow::bail!("native server launcher source must fall back to `orv run-artifact`");
        }
        if !source.contains("std::env::args_os().skip(1)") {
            anyhow::bail!("native server launcher source must forward process arguments");
        }
    }
    if !source.contains("mod routes;") || !source.contains("routes::ORV_NATIVE_ROUTE_COUNT") {
        anyhow::bail!("native server launcher source must link generated routes source");
    }
    if !source.contains(r#"routes::orv_native_match_route("__orv_probe__", "__orv_probe__")"#) {
        anyhow::bail!("native server launcher source must link generated route matcher");
    }
    if !source.contains("mod router;") || !source.contains("router::ORV_NATIVE_HANDLER_COUNT") {
        anyhow::bail!("native server launcher source must link generated router source");
    }
    if !source.contains(r#"router::orv_native_dispatch("__orv_probe__", "__orv_probe__")"#) {
        anyhow::bail!("native server launcher source must link generated router dispatch");
    }
    if !source.contains("mod handlers;") || !source.contains("handlers::ORV_NATIVE_HANDLER_COUNT") {
        anyhow::bail!("native server launcher source must link generated handlers source");
    }
    if source != expected {
        anyhow::bail!("native server launcher source must match generated source");
    }
    Ok(())
}

pub(crate) fn verify_native_server_routes_source(
    target: &Path,
    artifact: &orv_compiler::ServerRuntimeArtifact,
) -> anyhow::Result<()> {
    if !target.is_file() {
        anyhow::bail!("missing native server routes source: {}", target.display());
    }
    let source = std::fs::read_to_string(target)
        .map_err(|e| anyhow::anyhow!("failed to read {}: {e}", target.display()))?;
    let expected = orv_compiler::native_server_routes_source(artifact);
    if source != expected {
        anyhow::bail!("native server routes source must match server runtime artifact");
    }
    Ok(())
}

pub(crate) fn verify_native_server_router_source(target: &Path) -> anyhow::Result<()> {
    if !target.is_file() {
        anyhow::bail!("missing native server router source: {}", target.display());
    }
    let source = std::fs::read_to_string(target)
        .map_err(|e| anyhow::anyhow!("failed to read {}: {e}", target.display()))?;
    let expected = orv_compiler::native_server_router_source();
    if source != expected {
        anyhow::bail!("native server router source must match generated source");
    }
    Ok(())
}

pub(crate) fn verify_native_server_handlers_source(
    target: &Path,
    artifact: &orv_compiler::ServerRuntimeArtifact,
) -> anyhow::Result<()> {
    if !target.is_file() {
        anyhow::bail!(
            "missing native server handlers source: {}",
            target.display()
        );
    }
    let source = std::fs::read_to_string(target)
        .map_err(|e| anyhow::anyhow!("failed to read {}: {e}", target.display()))?;
    let expected = orv_compiler::native_server_handlers_source(artifact);
    if source != expected {
        anyhow::bail!("native server handlers source must match generated source");
    }
    Ok(())
}
