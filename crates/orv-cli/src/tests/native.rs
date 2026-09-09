use super::*;

#[test]
fn build_writes_native_runtime_image_plan_contract() {
    let path = workspace_path(&["fixtures", "e2e", "hello.orv"]);
    let out = temp_output_dir("native-runtime-image-plan");

    cmd_build(&path, &out).expect("build artifacts");

    let image_plan_path = out.join("server").join("runtime-image.json");
    assert!(
        image_plan_path.is_file(),
        "missing {}",
        image_plan_path.display()
    );
    let image_plan = read_json_value(&image_plan_path).expect("runtime image plan");
    let server_artifact =
        read_json_value(&out.join(SERVER_ARTIFACT_PATH)).expect("server artifact");
    let native_plan =
        read_json_value(&out.join(NATIVE_SERVER_PLAN_PATH)).expect("native server plan");
    assert_manifest_artifact(
        &out.join("build-manifest.json"),
        "native_runtime_image_plan",
        "server/runtime-image.json",
    );
    assert_bundle_target(
        &out.join("bundle-plan.json"),
        "native_runtime_image_plan",
        "server/runtime-image.json",
    );
    assert_manifest_artifact(
        &out.join("build-manifest.json"),
        "native_runtime_image_dockerfile",
        NATIVE_RUNTIME_IMAGE_DOCKERFILE_PATH,
    );
    assert_bundle_target(
        &out.join("bundle-plan.json"),
        "native_runtime_image_dockerfile",
        NATIVE_RUNTIME_IMAGE_DOCKERFILE_PATH,
    );
    assert_eq!(
        native_plan["runtime_image_plan"],
        "server/runtime-image.json"
    );
    assert_eq!(image_plan["kind"], "native_runtime_image_plan");
    assert_eq!(image_plan["status"], "image_planned");
    assert_eq!(image_plan["artifact"], SERVER_ARTIFACT_PATH);
    assert_eq!(image_plan["native_plan"], NATIVE_SERVER_PLAN_PATH);
    assert_eq!(image_plan["runtime"], server_artifact["runtime"]);
    assert_eq!(
        image_plan["reference_image"],
        "ghcr.io/orv-lang/orv-reference:latest"
    );
    assert_eq!(image_plan["target"]["kind"], "oci_image");
    assert_eq!(image_plan["target"]["binary"], NATIVE_SERVER_BINARY_PATH);
    assert_eq!(
        image_plan["dockerfile"],
        NATIVE_RUNTIME_IMAGE_DOCKERFILE_PATH
    );
    assert_eq!(
        image_plan["commands"]["build"],
        serde_json::json!([
            "docker",
            "build",
            "-f",
            NATIVE_RUNTIME_IMAGE_DOCKERFILE_PATH,
            "-t",
            NATIVE_RUNTIME_IMAGE_NAME,
            "."
        ])
    );
    assert_eq!(image_plan["routes"], server_artifact["routes"]);
    assert!(!image_plan["blocked_by"]
        .as_array()
        .expect("blocked_by")
        .iter()
        .any(|item| item == "native-codegen"));
    assert!(!image_plan["blocked_by"]
        .as_array()
        .expect("blocked_by")
        .iter()
        .any(|item| item == "native-runtime-image"));
    let dockerfile = std::fs::read_to_string(out.join(NATIVE_RUNTIME_IMAGE_DOCKERFILE_PATH))
        .expect("native runtime image Dockerfile");
    assert!(dockerfile.contains("FROM rust:"));
    assert!(
        dockerfile.contains("cargo build --manifest-path /work/server/native/Cargo.toml --release")
    );
    assert!(dockerfile.contains("COPY . /app"));
    assert!(dockerfile.contains(
        "COPY --from=build /work/server/native/target/release/orv-native-server /app/server/app"
    ));
    assert!(dockerfile.contains("ENV ORV_BUILD_DIR=/app"));
    assert!(dockerfile.contains("ENTRYPOINT [\"/app/server/app\"]"));

    cmd_verify_build(&out).expect("verify build artifacts");
    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn build_writes_native_server_routes_source_contract() {
    let path = workspace_path(&["fixtures", "e2e", "hello.orv"]);
    let out = temp_output_dir("native-server-routes-source");

    cmd_build(&path, &out).expect("build artifacts");

    let routes_source_path = out.join("server").join("native").join("routes.rs");
    assert!(
        routes_source_path.is_file(),
        "missing {}",
        routes_source_path.display()
    );
    assert_manifest_artifact(
        &out.join("build-manifest.json"),
        "native_server_routes_source",
        "server/native/routes.rs",
    );
    assert_bundle_target(
        &out.join("bundle-plan.json"),
        "native_server_routes_source",
        "server/native/routes.rs",
    );
    let native_plan =
        read_json_value(&out.join(NATIVE_SERVER_PLAN_PATH)).expect("native server plan");
    let server_artifact =
        read_json_value(&out.join(SERVER_ARTIFACT_PATH)).expect("server artifact");
    let source = std::fs::read_to_string(&routes_source_path).expect("routes source");
    let route_origin = server_artifact["routes"][0]["origin_id"]
        .as_str()
        .expect("route origin id");
    let response_origin = server_artifact["routes"][0]["response_origin_ids"][0]
        .as_str()
        .expect("response origin id");

    assert_eq!(native_plan["routes_source"], "server/native/routes.rs");
    assert!(source.contains("pub struct OrvNativeRoute"));
    assert!(source.contains("pub response_origin_ids: &'static [&'static str]"));
    assert!(source.contains("pub policies: &'static [OrvNativeRoutePolicy]"));
    assert!(source.contains("pub struct OrvNativeRoutePolicy"));
    assert!(source.contains("pub const ORV_NATIVE_ROUTES"));
    assert!(source.contains("OrvNativeRoute {"));
    assert!(source.contains("method: \"GET\""));
    assert!(source.contains("path: \"/ping\""));
    assert!(source.contains("pub fn orv_native_match_route("));
    assert!(source.contains("orv_native_route_path_params(route.path, path)"));
    assert!(source.contains(&format!("origin_id: \"{route_origin}\"")));
    assert!(source.contains(&format!("response_origin_ids: &[\"{response_origin}\"]")));
    assert!(source.contains("policies: &[]"));
    assert!(source.contains("pub const ORV_NATIVE_ROUTE_COUNT: usize = ORV_NATIVE_ROUTES.len();"));

    cmd_verify_build(&out).expect("verify build artifacts");
    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn build_writes_native_server_router_source_contract() {
    let path = workspace_path(&["fixtures", "e2e", "hello.orv"]);
    let out = temp_output_dir("native-server-router-source");

    cmd_build(&path, &out).expect("build artifacts");

    let router_source_path = out.join("server").join("native").join("router.rs");
    assert!(
        router_source_path.is_file(),
        "missing {}",
        router_source_path.display()
    );
    assert_manifest_artifact(
        &out.join("build-manifest.json"),
        "native_server_router_source",
        "server/native/router.rs",
    );
    assert_bundle_target(
        &out.join("bundle-plan.json"),
        "native_server_router_source",
        "server/native/router.rs",
    );
    let native_plan =
        read_json_value(&out.join(NATIVE_SERVER_PLAN_PATH)).expect("native server plan");
    let source = std::fs::read_to_string(&router_source_path).expect("router source");

    assert_eq!(native_plan["router_source"], "server/native/router.rs");
    assert!(source.contains("use crate::{handlers, routes};"));
    assert!(source.contains("pub struct OrvNativeDispatch"));
    assert!(source.contains("pub const ORV_NATIVE_HANDLER_COUNT"));
    assert!(source.contains("pub fn orv_native_dispatch("));
    assert!(source.contains("routes::orv_native_match_route(method, path)"));
    assert!(source.contains("handlers::orv_native_handle_route(&route_match)"));
    assert!(source.contains("origin_id: response.origin_id"));
    assert!(source.contains("response_origin_id: response.response_origin_id"));
    assert!(source.contains("params: response.params"));
    assert!(source.contains("status: 404"));

    cmd_verify_build(&out).expect("verify build artifacts");
    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn build_writes_native_server_handler_source_contract() {
    let path = workspace_path(&["fixtures", "e2e", "hello.orv"]);
    let out = temp_output_dir("native-server-handler-source");

    cmd_build(&path, &out).expect("build artifacts");

    let handlers_source_path = out.join("server").join("native").join("handlers.rs");
    assert!(
        handlers_source_path.is_file(),
        "missing {}",
        handlers_source_path.display()
    );
    assert_manifest_artifact(
        &out.join("build-manifest.json"),
        "native_server_handlers_source",
        "server/native/handlers.rs",
    );
    assert_bundle_target(
        &out.join("bundle-plan.json"),
        "native_server_handlers_source",
        "server/native/handlers.rs",
    );
    let native_plan =
        read_json_value(&out.join(NATIVE_SERVER_PLAN_PATH)).expect("native server plan");
    let server_artifact =
        read_json_value(&out.join(SERVER_ARTIFACT_PATH)).expect("server artifact");
    let response_origin = server_artifact["routes"][0]["response_origin_ids"][0]
        .as_str()
        .expect("response origin id");
    let source = std::fs::read_to_string(&handlers_source_path).expect("handlers source");

    assert_eq!(native_plan["handlers_source"], "server/native/handlers.rs");
    assert!(source.contains("use crate::routes;"));
    assert!(source.contains("pub struct OrvNativeHandlerResponse"));
    assert!(source
        .contains("pub const ORV_NATIVE_HANDLER_COUNT: usize = routes::ORV_NATIVE_ROUTE_COUNT;"));
    assert!(source.contains("pub fn orv_native_handle_route("));
    assert!(source.contains("response_origin_id: Some("));
    assert!(source.contains(response_origin));
    assert!(source.contains("status: 200"));
    assert!(source.contains(r#"body: "{\"ok\":true,\"msg\":\"pong\"}""#));
    assert!(!source.contains("native route body lowering pending"));

    cmd_verify_build(&out).expect("verify build artifacts");
    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn build_lowers_static_response_body_into_native_handler_source() {
    let path = workspace_path(&["fixtures", "e2e", "hello.orv"]);
    let out = temp_output_dir("native-static-response-handler");

    cmd_build(&path, &out).expect("build artifacts");

    let server_artifact =
        read_json_value(&out.join(SERVER_ARTIFACT_PATH)).expect("server artifact");
    let response = &server_artifact["routes"][0]["responses"][0];
    let handlers_source_path = out.join("server").join("native").join("handlers.rs");
    let source = std::fs::read_to_string(&handlers_source_path).expect("handlers source");

    assert_eq!(response["status"], 200);
    assert_eq!(response["body_kind"], "static_json");
    assert_eq!(response["body_json"], r#"{"ok":true,"msg":"pong"}"#);
    assert!(source.contains("status: 200"));
    assert!(source.contains(r#"body: "{\"ok\":true,\"msg\":\"pong\"}""#));
    assert!(!source.contains("native route body lowering pending"));

    cmd_verify_build(&out).expect("verify build artifacts");
    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn build_lowers_route_param_response_into_native_handler_source() {
    let dir = temp_output_dir("native-route-param-response-source");
    std::fs::create_dir_all(&dir).expect("create source dir");
    let path = dir.join("app.orv");
    std::fs::write(
        &path,
        r"@server {
  @listen 8080
  @route GET /users/:id {
    @respond 200 { id: @param.id }
  }
}
",
    )
    .expect("write source");
    let out = temp_output_dir("native-route-param-response-build");

    cmd_build(&path, &out).expect("build artifacts");

    let server_artifact =
        read_json_value(&out.join(SERVER_ARTIFACT_PATH)).expect("server artifact");
    let response = &server_artifact["routes"][0]["responses"][0];
    let handlers_source_path = out.join("server").join("native").join("handlers.rs");
    let handlers = std::fs::read_to_string(&handlers_source_path).expect("handlers source");
    let launcher = std::fs::read_to_string(out.join("server").join("native").join("main.rs"))
        .expect("native launcher");

    assert_eq!(response["status"], 200);
    assert_eq!(response["body_kind"], "route_param_json");
    assert_eq!(response["body_route_params"][0]["field"], "id");
    assert_eq!(response["body_route_params"][0]["param"], "id");
    assert!(handlers.contains("routes::orv_native_param_value(route_match, \"id\")"));
    assert!(handlers.contains("orv_native_push_json_string("));
    assert!(!handlers.contains("native route body lowering pending"));
    assert!(launcher.contains("fn orv_native_serve() -> std::io::Result<()>"));
    assert!(!launcher.contains(r#"std::process::Command::new("orv")"#));
    cmd_verify_build(&out).expect("verify route param native build");
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let output = std::process::Command::new(cargo)
        .arg("check")
        .arg("--manifest-path")
        .arg(out.join("server").join("native").join("Cargo.toml"))
        .arg("--color")
        .arg("never")
        .output()
        .expect("cargo check route param native launcher");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "route param native launcher cargo check failed:\n{stderr}"
    );
    assert!(
        !stderr.contains("warning:"),
        "route param native launcher cargo check should be warning-free:\n{stderr}"
    );

    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn build_lowers_query_param_response_into_native_handler_source() {
    let dir = temp_output_dir("native-query-param-response-source");
    std::fs::create_dir_all(&dir).expect("create source dir");
    let path = dir.join("app.orv");
    std::fs::write(
        &path,
        r"@server {
  @listen 8080
  @route GET /search {
    @respond 200 { q: @query.q }
  }
}
",
    )
    .expect("write source");
    let out = temp_output_dir("native-query-param-response-build");

    cmd_build(&path, &out).expect("build artifacts");

    let server_artifact =
        read_json_value(&out.join(SERVER_ARTIFACT_PATH)).expect("server artifact");
    let response = &server_artifact["routes"][0]["responses"][0];
    let routes = std::fs::read_to_string(out.join("server").join("native").join("routes.rs"))
        .expect("routes source");
    let handlers = std::fs::read_to_string(out.join("server").join("native").join("handlers.rs"))
        .expect("handlers source");
    let launcher = std::fs::read_to_string(out.join("server").join("native").join("main.rs"))
        .expect("native launcher");

    assert_eq!(response["status"], 200);
    assert_eq!(response["body_kind"], "query_param_json");
    assert_eq!(response["body_query_params"][0]["field"], "q");
    assert_eq!(response["body_query_params"][0]["param"], "q");
    assert!(routes.contains("pub query: Vec<OrvNativeParam>"));
    assert!(routes.contains("pub fn orv_native_query_value<'a>("));
    assert!(handlers.contains("routes::orv_native_query_value(route_match, \"q\")"));
    assert!(handlers.contains("orv_native_push_json_string("));
    assert!(!handlers.contains("native route body lowering pending"));
    assert!(launcher.contains("orv_native_parse_query(query)"));
    assert!(launcher.contains("router::orv_native_dispatch_with_request("));
    assert!(launcher.contains("fn orv_native_serve() -> std::io::Result<()>"));
    assert!(!launcher.contains(r#"std::process::Command::new("orv")"#));
    cmd_verify_build(&out).expect("verify query param native build");
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let output = std::process::Command::new(cargo)
        .arg("check")
        .arg("--manifest-path")
        .arg(out.join("server").join("native").join("Cargo.toml"))
        .arg("--color")
        .arg("never")
        .output()
        .expect("cargo check query param native launcher");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "query param native launcher cargo check failed:\n{stderr}"
    );
    assert!(
        !stderr.contains("warning:"),
        "query param native launcher cargo check should be warning-free:\n{stderr}"
    );

    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn build_lowers_request_body_response_into_native_handler_source() {
    let dir = temp_output_dir("native-request-body-response-source");
    std::fs::create_dir_all(&dir).expect("create source dir");
    let path = dir.join("app.orv");
    std::fs::write(
        &path,
        r"@server {
  @listen 8080
  @route POST /echo {
    @respond 201 { received: @body }
  }
}
",
    )
    .expect("write source");
    let out = temp_output_dir("native-request-body-response-build");

    cmd_build(&path, &out).expect("build artifacts");

    let server_artifact =
        read_json_value(&out.join(SERVER_ARTIFACT_PATH)).expect("server artifact");
    let response = &server_artifact["routes"][0]["responses"][0];
    let routes = std::fs::read_to_string(out.join("server").join("native").join("routes.rs"))
        .expect("routes source");
    let handlers = std::fs::read_to_string(out.join("server").join("native").join("handlers.rs"))
        .expect("handlers source");
    let launcher = std::fs::read_to_string(out.join("server").join("native").join("main.rs"))
        .expect("native launcher");

    assert_eq!(response["status"], 201);
    assert_eq!(response["body_kind"], "request_body_json");
    assert_eq!(response["body_request_json"][0]["field"], "received");
    assert!(routes.contains("pub body: String"));
    assert!(routes.contains("pub fn orv_native_body_json("));
    assert!(handlers.contains("routes::orv_native_body_json(route_match).unwrap_or(\"null\")"));
    assert!(handlers.contains("body.push_str(\"\\\"received\\\":\");"));
    assert!(!handlers.contains("native route body lowering pending"));
    assert!(launcher.contains("body: String"));
    assert!(launcher.contains("orv_native_content_length("));
    assert!(launcher.contains("router::orv_native_dispatch_with_request("));
    assert!(launcher.contains("fn orv_native_serve() -> std::io::Result<()>"));
    assert!(!launcher.contains(r#"std::process::Command::new("orv")"#));
    cmd_verify_build(&out).expect("verify request body native build");
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let output = std::process::Command::new(cargo)
        .arg("check")
        .arg("--manifest-path")
        .arg(out.join("server").join("native").join("Cargo.toml"))
        .arg("--color")
        .arg("never")
        .output()
        .expect("cargo check request body native launcher");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "request body native launcher cargo check failed:\n{stderr}"
    );
    assert!(
        !stderr.contains("warning:"),
        "request body native launcher cargo check should be warning-free:\n{stderr}"
    );

    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn build_lowers_request_body_field_response_into_native_handler_source() {
    let dir = temp_output_dir("native-request-body-field-response-source");
    std::fs::create_dir_all(&dir).expect("create source dir");
    let path = dir.join("app.orv");
    std::fs::write(
        &path,
        r"@server {
  @listen 8080
  @route POST /members {
    @respond 201 { handle: @body.handle, email: @body.email }
  }
}
",
    )
    .expect("write source");
    let out = temp_output_dir("native-request-body-field-response-build");

    cmd_build(&path, &out).expect("build artifacts");

    let server_artifact =
        read_json_value(&out.join(SERVER_ARTIFACT_PATH)).expect("server artifact");
    let response = &server_artifact["routes"][0]["responses"][0];
    let routes = std::fs::read_to_string(out.join("server").join("native").join("routes.rs"))
        .expect("routes source");
    let handlers = std::fs::read_to_string(out.join("server").join("native").join("handlers.rs"))
        .expect("handlers source");
    let launcher = std::fs::read_to_string(out.join("server").join("native").join("main.rs"))
        .expect("native launcher");

    assert_eq!(response["status"], 201);
    assert_eq!(response["body_kind"], "request_body_field_json");
    assert_eq!(response["body_request_fields"][0]["field"], "handle");
    assert_eq!(response["body_request_fields"][0]["name"], "handle");
    assert_eq!(response["body_request_fields"][1]["field"], "email");
    assert_eq!(response["body_request_fields"][1]["name"], "email");
    assert!(routes.contains("pub body_fields: Vec<OrvNativeParam>"));
    assert!(routes.contains("pub fn orv_native_body_field_value<'a>("));
    assert!(handlers.contains("routes::orv_native_body_field_value(route_match, \"handle\")"));
    assert!(handlers.contains("routes::orv_native_body_field_value(route_match, \"email\")"));
    assert!(handlers.contains("orv_native_push_json_string("));
    assert!(!handlers.contains("native route body lowering pending"));
    assert!(launcher.contains("orv_native_parse_body_fields("));
    assert!(launcher.contains("orv_native_parse_json_object_fields("));
    assert!(launcher.contains("orv_native_parse_query(&body)"));
    assert!(launcher.contains("fn orv_native_serve() -> std::io::Result<()>"));
    assert!(!launcher.contains(r#"std::process::Command::new("orv")"#));
    cmd_verify_build(&out).expect("verify request body field native build");
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let output = std::process::Command::new(cargo)
        .arg("check")
        .arg("--manifest-path")
        .arg(out.join("server").join("native").join("Cargo.toml"))
        .arg("--color")
        .arg("never")
        .output()
        .expect("cargo check request body field native launcher");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "request body field native launcher cargo check failed:\n{stderr}"
    );
    assert!(
        !stderr.contains("warning:"),
        "request body field native launcher cargo check should be warning-free:\n{stderr}"
    );

    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn build_lowers_mixed_static_and_request_body_field_response_into_native_handler_source() {
    let dir = temp_output_dir("native-mixed-body-field-response-source");
    std::fs::create_dir_all(&dir).expect("create source dir");
    let path = dir.join("app.orv");
    std::fs::write(
        &path,
        r#"@server {
  @listen 8080
  @route POST /orders {
    @respond 404 { err: "product_not_found", sku: @body.sku }
  }
}
"#,
    )
    .expect("write source");
    let out = temp_output_dir("native-mixed-body-field-response-build");

    cmd_build(&path, &out).expect("build artifacts");

    let server_artifact =
        read_json_value(&out.join(SERVER_ARTIFACT_PATH)).expect("server artifact");
    let response = &server_artifact["routes"][0]["responses"][0];
    let handlers = std::fs::read_to_string(out.join("server").join("native").join("handlers.rs"))
        .expect("handlers source");
    let launcher = std::fs::read_to_string(out.join("server").join("native").join("main.rs"))
        .expect("native launcher");

    assert_eq!(response["status"], 404);
    assert_eq!(response["body_kind"], "mixed_json");
    assert_eq!(response["body_object_fields"][0]["field"], "err");
    assert_eq!(
        response["body_object_fields"][0]["value_kind"],
        "static_json"
    );
    assert_eq!(
        response["body_object_fields"][0]["value_json"],
        r#""product_not_found""#
    );
    assert_eq!(response["body_object_fields"][1]["field"], "sku");
    assert_eq!(
        response["body_object_fields"][1]["value_kind"],
        "request_body_field"
    );
    assert_eq!(response["body_object_fields"][1]["name"], "sku");
    assert!(handlers.contains("body.push_str(\"\\\"err\\\":\");"));
    assert!(handlers.contains("body.push_str(\"\\\"product_not_found\\\"\");"));
    assert!(handlers.contains("routes::orv_native_body_field_value(route_match, \"sku\")"));
    assert!(handlers.contains("orv_native_push_json_string("));
    assert!(!handlers.contains("native route body lowering pending"));
    assert!(launcher.contains("fn orv_native_serve() -> std::io::Result<()>"));
    assert!(!launcher.contains(r#"std::process::Command::new("orv")"#));
    cmd_verify_build(&out).expect("verify mixed native build");
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let output = std::process::Command::new(cargo)
        .arg("check")
        .arg("--manifest-path")
        .arg(out.join("server").join("native").join("Cargo.toml"))
        .arg("--color")
        .arg("never")
        .output()
        .expect("cargo check mixed native launcher");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "mixed native launcher cargo check failed:\n{stderr}"
    );
    assert!(
        !stderr.contains("warning:"),
        "mixed native launcher cargo check should be warning-free:\n{stderr}"
    );

    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn build_lowers_mixed_dynamic_response_into_native_handler_source() {
    let dir = temp_output_dir("native-mixed-dynamic-response-source");
    std::fs::create_dir_all(&dir).expect("create source dir");
    let path = dir.join("app.orv");
    std::fs::write(&path, MIXED_DYNAMIC_RESPONSE_SOURCE).expect("write source");
    let out = temp_output_dir("native-mixed-dynamic-response-build");

    cmd_build(&path, &out).expect("build artifacts");

    let server_artifact =
        read_json_value(&out.join(SERVER_ARTIFACT_PATH)).expect("server artifact");
    let response = &server_artifact["routes"][0]["responses"][0];
    let sku_label_response = &server_artifact["routes"][3]["responses"][0];
    let joined_label_response = &server_artifact["routes"][4]["responses"][0];
    let handlers = std::fs::read_to_string(out.join("server").join("native").join("handlers.rs"))
        .expect("handlers source");
    let launcher = std::fs::read_to_string(out.join("server").join("native").join("main.rs"))
        .expect("native launcher");

    assert_eq!(response["status"], 201);
    assert_eq!(response["body_kind"], "mixed_json");
    assert_eq!(response["body_object_fields"][0]["field"], "sku");
    assert_eq!(
        response["body_object_fields"][0]["value_kind"],
        "request_body_field"
    );
    assert_eq!(response["body_object_fields"][0]["name"], "sku");
    assert_eq!(response["body_object_fields"][1]["field"], "coupon");
    assert_eq!(
        response["body_object_fields"][1]["value_kind"],
        "query_param"
    );
    assert_eq!(response["body_object_fields"][1]["name"], "coupon");
    assert_eq!(sku_label_response["body_kind"], "request_body_field_json");
    assert_eq!(
        sku_label_response["body_request_fields"][0]["op"],
        "concat_affix"
    );
    assert_eq!(
        sku_label_response["body_request_fields"][0]["operand_json"],
        "4:sku--v1"
    );
    assert_eq!(
        joined_label_response["body_kind"],
        "request_body_field_json"
    );
    assert_eq!(
        joined_label_response["body_request_fields"][0]["op"],
        "concat_join"
    );
    assert_eq!(
        joined_label_response["body_request_fields"][0]["operand_json"],
        "-"
    );
    assert_eq!(
        joined_label_response["body_request_fields"][0]["operand_kind"],
        "query_param"
    );
    assert!(handlers.contains("routes::orv_native_body_field_value(route_match, \"sku\")"));
    assert!(handlers.contains("routes::orv_native_query_value(route_match, \"coupon\")"));
    assert!(handlers.contains("value.push_str(operand)"));
    assert!(handlers.contains("let mut value = String::from(\"sku-\")"));
    assert!(handlers.contains("value.push_str(\"-v1\")"));
    assert!(handlers.contains("value.push_str(\"-\")"));
    assert!(handlers.contains("match value.checked_add(1)"));
    assert!(handlers.contains("orv_native_push_json_string("));
    assert!(!handlers.contains("native route body lowering pending"));
    assert!(launcher.contains("fn orv_native_serve() -> std::io::Result<()>"));
    assert!(!launcher.contains(r#"std::process::Command::new("orv")"#));
    cmd_verify_build(&out).expect("verify mixed dynamic native build");
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let output = std::process::Command::new(cargo)
        .arg("check")
        .arg("--manifest-path")
        .arg(out.join("server").join("native").join("Cargo.toml"))
        .arg("--color")
        .arg("never")
        .output()
        .expect("cargo check mixed dynamic native launcher");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "mixed dynamic native launcher cargo check failed:\n{stderr}"
    );
    assert!(
        !stderr.contains("warning:"),
        "mixed dynamic native launcher cargo check should be warning-free:\n{stderr}"
    );

    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn build_lowers_static_left_ordered_arithmetic_response_into_native_handler_source() {
    let dir = temp_output_dir("native-static-left-ordered-response-source");
    std::fs::create_dir_all(&dir).expect("create source dir");
    let path = dir.join("app.orv");
    std::fs::write(
        &path,
        r"@server {
  @listen 8080
  @route POST /int/unit {
    @respond 201 { unit: 100 / (@body.parts as int) }
  }
  @route POST /int/remainder {
    @respond 201 { remainder: 10 % (@body.parts as int) }
  }
  @route POST /float/ratio {
    @respond 201 { ratio: 100.0 / (@body.amount as float) }
  }
  @route POST /float/remainder {
    @respond 201 { remainder: 10.5 % (@body.amount as float) }
  }
  @route POST /int/power {
    @respond 201 { total: 2 ** (@body.exp as int) }
  }
  @route POST /float/power {
    @respond 201 { total: 2.0 ** (@body.exp as float) }
  }
}
",
    )
    .expect("write source");
    let out = temp_output_dir("native-static-left-ordered-response-build");

    cmd_build(&path, &out).expect("build artifacts");

    let server_artifact =
        read_json_value(&out.join(SERVER_ARTIFACT_PATH)).expect("server artifact");
    let handlers = std::fs::read_to_string(out.join("server").join("native").join("handlers.rs"))
        .expect("handlers source");
    let int_unit = &server_artifact["routes"][0]["responses"][0]["body_request_fields"][0];
    let int_remainder = &server_artifact["routes"][1]["responses"][0]["body_request_fields"][0];
    let float_ratio = &server_artifact["routes"][2]["responses"][0]["body_request_fields"][0];
    let float_remainder = &server_artifact["routes"][3]["responses"][0]["body_request_fields"][0];
    let int_power = &server_artifact["routes"][4]["responses"][0]["body_request_fields"][0];
    let float_power = &server_artifact["routes"][5]["responses"][0]["body_request_fields"][0];

    assert_eq!(int_unit["op"], "rdiv");
    assert_eq!(int_remainder["op"], "rrem");
    assert_eq!(float_ratio["op"], "rdiv");
    assert_eq!(float_remainder["op"], "rrem");
    assert_eq!(int_power["op"], "rpow");
    assert_eq!(float_power["op"], "rpow");
    assert!(handlers.contains("100_i64.checked_div(value)"));
    assert!(handlers.contains("10_i64.checked_rem(value)"));
    assert!(handlers.contains("let value = 100.0 / value;"));
    assert!(handlers.contains("let value = 10.5 % value;"));
    assert!(handlers.contains("2_i64.checked_pow(u32::try_from(value).unwrap_or(0))"));
    assert!(handlers.contains("let value = (2.0_f64).powf(value);"));
    assert!(!handlers.contains("native route body lowering pending"));
    cmd_verify_build(&out).expect("verify static-left ordered native build");
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let output = std::process::Command::new(cargo)
        .arg("check")
        .arg("--manifest-path")
        .arg(out.join("server").join("native").join("Cargo.toml"))
        .arg("--color")
        .arg("never")
        .output()
        .expect("cargo check static-left ordered native launcher");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "static-left ordered native launcher cargo check failed:\n{stderr}"
    );

    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn generated_native_server_serves_mixed_static_and_request_body_field_response() {
    let dir = temp_output_dir("native-mixed-body-field-server-source");
    std::fs::create_dir_all(&dir).expect("create source dir");
    let path = dir.join("app.orv");
    std::fs::write(
        &path,
        r#"@server {
  @listen 8080
  @route POST /orders {
    @respond 404 { err: "product_not_found", sku: @body.sku }
  }
}
"#,
    )
    .expect("write source");
    let out = temp_output_dir("native-mixed-body-field-server-build");

    cmd_build(&path, &out).expect("build artifacts");
    cmd_verify_build(&out).expect("verify mixed native server build");
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let output = std::process::Command::new(cargo)
        .arg("build")
        .arg("--manifest-path")
        .arg(out.join("server").join("native").join("Cargo.toml"))
        .arg("--release")
        .arg("--color")
        .arg("never")
        .output()
        .expect("cargo build mixed native server");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "mixed native server cargo build failed:\n{stderr}"
    );

    let binary = out
        .join("server")
        .join("native")
        .join("target")
        .join("release")
        .join("orv-native-server");
    let mut child = std::process::Command::new(&binary)
        .env("ORV_BUILD_DIR", &out)
        .env("ORV_HOST", "127.0.0.1")
        .env("ORV_PORT", "0")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("spawn generated native server");
    let stderr = child.stderr.take().expect("native server stderr");
    let child = ChildGuard(child);
    let mut stderr = std::io::BufReader::new(stderr);
    let mut line = String::new();
    std::io::BufRead::read_line(&mut stderr, &mut line).expect("native server listen line");
    let address = line
        .trim()
        .strip_prefix("orv native server listening on ")
        .expect("native listen address");

    let response = send_raw_http_json_post(address, "/orders", r#"{"sku":"sku-1"}"#);

    assert!(response.starts_with("HTTP/1.1 404"));
    assert!(response.contains("content-type: application/json"));
    assert!(response.contains(r#"{"err":"product_not_found","sku":"sku-1"}"#));

    drop(child);
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn generated_native_server_serves_mixed_dynamic_response() {
    let dir = temp_output_dir("native-mixed-dynamic-server-source");
    std::fs::create_dir_all(&dir).expect("create source dir");
    let path = dir.join("app.orv");
    std::fs::write(&path, MIXED_DYNAMIC_SERVER_SOURCE).expect("write source");
    let out = temp_output_dir("native-mixed-dynamic-server-build");

    cmd_build(&path, &out).expect("build artifacts");
    cmd_verify_build(&out).expect("verify mixed dynamic native server build");
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let output = std::process::Command::new(cargo)
        .arg("build")
        .arg("--manifest-path")
        .arg(out.join("server").join("native").join("Cargo.toml"))
        .arg("--release")
        .arg("--color")
        .arg("never")
        .output()
        .expect("cargo build mixed dynamic native server");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "mixed dynamic native server cargo build failed:\n{stderr}"
    );

    let binary = out
        .join("server")
        .join("native")
        .join("target")
        .join("release")
        .join("orv-native-server");
    let mut child = std::process::Command::new(&binary)
        .env("ORV_BUILD_DIR", &out)
        .env("ORV_HOST", "127.0.0.1")
        .env("ORV_PORT", "0")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("spawn generated native server");
    let stderr = child.stderr.take().expect("native server stderr");
    let child = ChildGuard(child);
    let mut stderr = std::io::BufReader::new(stderr);
    let mut line = String::new();
    std::io::BufRead::read_line(&mut stderr, &mut line).expect("native server listen line");
    let address = line
        .trim()
        .strip_prefix("orv native server listening on ")
        .expect("native listen address");

    let response = send_raw_http_json_post(address, "/orders?coupon=SAVE10", r#"{"sku":"sku-1"}"#);
    let session_response =
        send_raw_http_json_post(address, "/sessions?token=abc", r#"{"token":"abc"}"#);
    let label_response =
        send_raw_http_json_post(address, "/labels?suffix=-pro", r#"{"first":"orv"}"#);
    let sku_label_response = send_raw_http_json_post(address, "/sku-labels", r#"{"sku":"A1"}"#);
    let joined_label_response =
        send_raw_http_json_post(address, "/joined-labels?suffix=pro", r#"{"first":"orv"}"#);
    let quantity_response = send_raw_http_json_post(address, "/quantities", r#"{"quantity":"7"}"#);
    let doubled_response =
        send_raw_http_json_post(address, "/quantity-doubles", r#"{"quantity":"7"}"#);
    let limit_response =
        send_raw_http_json_post(address, "/quantity-limits", r#"{"quantity":"7"}"#);

    assert!(response.starts_with("HTTP/1.1 201"));
    assert!(response.contains("content-type: application/json"));
    assert!(response.contains(r#"{"sku":"sku-1","coupon":"SAVE10"}"#));
    assert!(session_response.starts_with("HTTP/1.1 201"));
    assert!(session_response.contains(r#"{"matches":true}"#));
    assert!(label_response.contains(r#"{"label":"orv-pro"}"#));
    assert!(sku_label_response.contains(r#"{"label":"sku-A1-v1"}"#));
    assert!(joined_label_response.contains(r#"{"label":"orv-pro"}"#));
    assert!(quantity_response.contains(r#"{"next":8}"#));
    assert!(doubled_response.contains(r#"{"doubled":14}"#));
    assert!(limit_response.contains(r#"{"below_limit":true}"#));

    drop(child);
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn generated_native_server_serves_guarded_multi_response_route() {
    let dir = temp_output_dir("native-guarded-multi-response-server-source");
    std::fs::create_dir_all(&dir).expect("create source dir");
    let path = dir.join("app.orv");
    std::fs::write(
            &path,
            r#"@server {
  @listen 8080
  @route POST /orders {
    if @body.sku == "" {
      @respond 400 { err: "missing_sku" }
    }
    @respond 201 { sku: @body.sku }
  }
  @route POST /orders-bonus {
    if @body.sku == "" {
      @respond 400 { err: "missing_sku" }
    }
    @respond 201 { quantity: (@body.quantity as int) + ((@body.bonus as int) * 2) }
  }
  @route POST /orders-bonus-left {
    @respond 201 { quantity: ((@body.bonus as int) * 2) + (@body.quantity as int) }
  }
  @route POST /orders-bonus-delta {
    @respond 201 { quantity: ((@body.bonus as int) * 2) - (@body.quantity as int) }
  }
  @route POST /members {
    if @body.password != @body.confirm {
      @respond 400 { err: "password_mismatch" }
    }
    @respond 201 { email: @body.email }
  }
  @route POST /sessions {
    if @body.token == @query.token {
      @respond 201 { ok: true }
    }
    @respond 401 { err: "token_mismatch" }
  }
  @route POST /quantity {
    if (@body.quantity as int) > 0 {
      @respond 201 { accepted: true, quantity: @body.quantity as int }
    }
    @respond 400 { err: "bad_quantity" }
  }
  @route POST /inventory {
    if (@body.quantity as int) <= (@body.stock as int) {
      @respond 201 { accepted: true, quantity: @body.quantity as int }
    }
    @respond 409 { err: "out_of_stock" }
  }
  @route POST /inventory-bulk {
    if (@body.quantity as int) <= ((@body.stock as int) * 10) {
      @respond 201 { accepted: true, quantity: @body.quantity as int }
    }
    @respond 409 { err: "out_of_stock" }
  }
  @route POST /inventory-value {
    if (@body.total as int) <= ((@body.quantity as int) * (@body.unit_price as int)) {
      @respond 201 { accepted: true, total: @body.total as int }
    }
    @respond 409 { err: "over_total" }
  }
  @route POST /inventory-value-scaled {
    if (@body.total as int) <= (((@body.quantity as int) * (@body.unit_price as int)) * 100) {
      @respond 201 { accepted: true, total: @body.total as int }
    }
    @respond 409 { err: "over_total" }
  }
  @route POST /inventory-value-static {
    if ((@body.quantity as int) * (@body.unit_price as int)) <= 1000 {
      @respond 201 { accepted: true, quantity: @body.quantity as int }
    }
    @respond 409 { err: "over_total" }
  }
  @route POST /inventory-value-product {
    if ((@body.quantity as int) * (@body.unit_price as int)) <= ((@body.stock as int) * (@body.reserve_price as int)) {
      @respond 201 { accepted: true, quantity: @body.quantity as int }
    }
    @respond 409 { err: "over_total" }
  }
  @route POST /ifelse-inventory {
    if (@body.quantity as int) <= (@body.stock as int) {
      @respond 201 { accepted: true, quantity: @body.quantity as int }
    } else {
      @respond 409 { err: "out_of_stock" }
    }
  }
  @route POST /tiered-inventory {
    if (@body.quantity as int) <= 0 {
      @respond 400 { err: "bad_quantity" }
    } else if (@body.quantity as int) <= (@body.stock as int) {
      @respond 201 { accepted: true, quantity: @body.quantity as int }
    } else {
      @respond 409 { err: "out_of_stock" }
    }
  }
  @route POST /tiered-block-inventory {
    if (@body.quantity as int) <= 0 {
      @respond 400 { err: "bad_quantity" }
    } else {
      if (@body.quantity as int) <= (@body.stock as int) {
        @respond 201 { accepted: true, quantity: @body.quantity as int }
      } else {
        @respond 409 { err: "out_of_stock" }
      }
    }
  }
  @route POST /tiered-fallback-inventory {
    if (@body.quantity as int) <= 0 {
      @respond 400 { err: "bad_quantity" }
    } else if (@body.quantity as int) <= (@body.stock as int) {
      @respond 201 { accepted: true, quantity: @body.quantity as int }
    }
    @respond 409 { err: "out_of_stock" }
  }
  @route POST /amount {
    if (@body.amount as float) > 0.0 {
      @respond 201 { accepted: true, amount: @body.amount as float }
    }
    @respond 400 { err: "bad_amount" }
  }
  @route POST /limit {
    if (@body.amount as float) <= (@query.limit as float) {
      @respond 201 { accepted: true, amount: @body.amount as float }
    }
    @respond 409 { err: "amount_over_limit" }
  }
  @route POST /limit-product {
    if ((@body.price as float) * (@body.quantity as float)) <= ((@body.limit_price as float) * (@body.limit_units as float)) {
      @respond 201 { accepted: true, amount: @body.price as float }
    }
    @respond 409 { err: "amount_over_limit" }
  }
  @route GET /catalog/:kind {
    if @param.kind == "sale" {
      @respond 200 { kind: @param.kind }
    }
    @respond 200 { kind: "regular" }
  }
  @route GET /search {
    if @query.mode != "compact" {
      @respond 200 { mode: @query.mode }
    }
    @respond 200 { mode: "compact" }
  }
}
"#,
        )
        .expect("write source");
    let out = temp_output_dir("native-guarded-multi-response-server-build");

    cmd_build(&path, &out).expect("build artifacts");
    cmd_verify_build(&out).expect("verify guarded native server build");
    let launcher = std::fs::read_to_string(out.join("server").join("native").join("main.rs"))
        .expect("native launcher");
    assert!(launcher.contains("fn orv_native_serve() -> std::io::Result<()>"));
    assert!(!launcher.contains(r#"std::process::Command::new("orv")"#));
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let output = std::process::Command::new(cargo)
        .arg("build")
        .arg("--manifest-path")
        .arg(out.join("server").join("native").join("Cargo.toml"))
        .arg("--release")
        .arg("--color")
        .arg("never")
        .output()
        .expect("cargo build guarded native server");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "guarded native server cargo build failed:\n{stderr}"
    );

    let binary = out
        .join("server")
        .join("native")
        .join("target")
        .join("release")
        .join("orv-native-server");
    let mut child = std::process::Command::new(&binary)
        .env("ORV_BUILD_DIR", &out)
        .env("ORV_HOST", "127.0.0.1")
        .env("ORV_PORT", "0")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("spawn generated guarded native server");
    let stderr = child.stderr.take().expect("native server stderr");
    let child = ChildGuard(child);
    let mut stderr = std::io::BufReader::new(stderr);
    let mut line = String::new();
    std::io::BufRead::read_line(&mut stderr, &mut line).expect("native server listen line");
    let address = line
        .trim()
        .strip_prefix("orv native server listening on ")
        .expect("native listen address");

    let missing = send_raw_http_json_post(address, "/orders", r#"{"sku":""}"#);
    let created = send_raw_http_json_post(address, "/orders", r#"{"sku":"sku-7"}"#);
    let missing_bonus = send_raw_http_json_post(address, "/orders-bonus", r#"{"sku":""}"#);
    let created_bonus = send_raw_http_json_post(
        address,
        "/orders-bonus",
        r#"{"sku":"sku-7","quantity":"7","bonus":"2"}"#,
    );
    let created_bonus_left = send_raw_http_json_post(
        address,
        "/orders-bonus-left",
        r#"{"quantity":"7","bonus":"2"}"#,
    );
    let created_bonus_delta = send_raw_http_json_post(
        address,
        "/orders-bonus-delta",
        r#"{"quantity":"5","bonus":"8"}"#,
    );
    let mismatch = send_raw_http_json_post(
        address,
        "/members",
        r#"{"email":"a@orv.dev","password":"one","confirm":"two"}"#,
    );
    let member = send_raw_http_json_post(
        address,
        "/members",
        r#"{"email":"a@orv.dev","password":"same","confirm":"same"}"#,
    );
    let session = send_raw_http_json_post(address, "/sessions?token=abc", r#"{"token":"abc"}"#);
    let rejected_session =
        send_raw_http_json_post(address, "/sessions?token=abc", r#"{"token":"xyz"}"#);
    let accepted_quantity = send_raw_http_json_post(address, "/quantity", r#"{"quantity":"3"}"#);
    let rejected_quantity = send_raw_http_json_post(address, "/quantity", r#"{"quantity":"0"}"#);
    let accepted_inventory =
        send_raw_http_json_post(address, "/inventory", r#"{"quantity":"3","stock":"5"}"#);
    let rejected_inventory =
        send_raw_http_json_post(address, "/inventory", r#"{"quantity":"7","stock":"5"}"#);
    let accepted_bulk_inventory = send_raw_http_json_post(
        address,
        "/inventory-bulk",
        r#"{"quantity":"30","stock":"5"}"#,
    );
    let rejected_bulk_inventory = send_raw_http_json_post(
        address,
        "/inventory-bulk",
        r#"{"quantity":"51","stock":"5"}"#,
    );
    let accepted_value_inventory = send_raw_http_json_post(
        address,
        "/inventory-value",
        r#"{"total":"875","quantity":"7","unit_price":"125"}"#,
    );
    let rejected_value_inventory = send_raw_http_json_post(
        address,
        "/inventory-value",
        r#"{"total":"901","quantity":"7","unit_price":"125"}"#,
    );
    let accepted_static_value_inventory = send_raw_http_json_post(
        address,
        "/inventory-value-static",
        r#"{"quantity":"7","unit_price":"125"}"#,
    );
    let rejected_static_value_inventory = send_raw_http_json_post(
        address,
        "/inventory-value-static",
        r#"{"quantity":"9","unit_price":"125"}"#,
    );
    let accepted_product_value_inventory = send_raw_http_json_post(
        address,
        "/inventory-value-product",
        r#"{"quantity":"7","unit_price":"125","stock":"8","reserve_price":"125"}"#,
    );
    let rejected_product_value_inventory = send_raw_http_json_post(
        address,
        "/inventory-value-product",
        r#"{"quantity":"9","unit_price":"125","stock":"8","reserve_price":"125"}"#,
    );
    let accepted_scaled_product_value_inventory = send_raw_http_json_post(
        address,
        "/inventory-value-scaled",
        r#"{"total":"87500","quantity":"7","unit_price":"125"}"#,
    );
    let rejected_scaled_product_value_inventory = send_raw_http_json_post(
        address,
        "/inventory-value-scaled",
        r#"{"total":"87501","quantity":"7","unit_price":"125"}"#,
    );
    let accepted_ifelse_inventory = send_raw_http_json_post(
        address,
        "/ifelse-inventory",
        r#"{"quantity":"3","stock":"5"}"#,
    );
    let rejected_ifelse_inventory = send_raw_http_json_post(
        address,
        "/ifelse-inventory",
        r#"{"quantity":"7","stock":"5"}"#,
    );
    let invalid_tiered_inventory = send_raw_http_json_post(
        address,
        "/tiered-inventory",
        r#"{"quantity":"0","stock":"5"}"#,
    );
    let accepted_tiered_inventory = send_raw_http_json_post(
        address,
        "/tiered-inventory",
        r#"{"quantity":"3","stock":"5"}"#,
    );
    let rejected_tiered_inventory = send_raw_http_json_post(
        address,
        "/tiered-inventory",
        r#"{"quantity":"7","stock":"5"}"#,
    );
    let accepted_tiered_block_inventory = send_raw_http_json_post(
        address,
        "/tiered-block-inventory",
        r#"{"quantity":"3","stock":"5"}"#,
    );
    let rejected_tiered_block_inventory = send_raw_http_json_post(
        address,
        "/tiered-block-inventory",
        r#"{"quantity":"7","stock":"5"}"#,
    );
    let invalid_tiered_fallback_inventory = send_raw_http_json_post(
        address,
        "/tiered-fallback-inventory",
        r#"{"quantity":"0","stock":"5"}"#,
    );
    let accepted_tiered_fallback_inventory = send_raw_http_json_post(
        address,
        "/tiered-fallback-inventory",
        r#"{"quantity":"3","stock":"5"}"#,
    );
    let rejected_tiered_fallback_inventory = send_raw_http_json_post(
        address,
        "/tiered-fallback-inventory",
        r#"{"quantity":"7","stock":"5"}"#,
    );
    let accepted_amount = send_raw_http_json_post(address, "/amount", r#"{"amount":"12.5"}"#);
    let rejected_amount = send_raw_http_json_post(address, "/amount", r#"{"amount":"0.0"}"#);
    let accepted_limit =
        send_raw_http_json_post(address, "/limit?limit=20.0", r#"{"amount":"12.5"}"#);
    let rejected_limit =
        send_raw_http_json_post(address, "/limit?limit=10.0", r#"{"amount":"12.5"}"#);
    let accepted_product_limit = send_raw_http_json_post(
        address,
        "/limit-product",
        r#"{"price":"12.5","quantity":"3","limit_price":"20.0","limit_units":"2"}"#,
    );
    let rejected_product_limit = send_raw_http_json_post(
        address,
        "/limit-product",
        r#"{"price":"12.5","quantity":"4","limit_price":"12.5","limit_units":"3"}"#,
    );
    let sale = send_raw_http(address, "/catalog/sale");
    let regular = send_raw_http(address, "/catalog/full");
    let expanded = send_raw_http(address, "/search?mode=expanded");
    let compact = send_raw_http(address, "/search?mode=compact");

    assert!(missing.starts_with("HTTP/1.1 400"));
    assert!(missing.contains(r#"{"err":"missing_sku"}"#));
    assert!(created.starts_with("HTTP/1.1 201"));
    assert!(created.contains(r#"{"sku":"sku-7"}"#));
    assert!(missing_bonus.starts_with("HTTP/1.1 400"));
    assert!(missing_bonus.contains(r#"{"err":"missing_sku"}"#));
    assert!(created_bonus.starts_with("HTTP/1.1 201"));
    assert!(created_bonus.contains(r#"{"quantity":11}"#));
    assert!(created_bonus_left.starts_with("HTTP/1.1 201"));
    assert!(created_bonus_left.contains(r#"{"quantity":11}"#));
    assert!(created_bonus_delta.starts_with("HTTP/1.1 201"));
    assert!(created_bonus_delta.contains(r#"{"quantity":11}"#));
    assert!(mismatch.starts_with("HTTP/1.1 400"));
    assert!(mismatch.contains(r#"{"err":"password_mismatch"}"#));
    assert!(member.starts_with("HTTP/1.1 201"));
    assert!(member.contains(r#"{"email":"a@orv.dev"}"#));
    assert!(session.starts_with("HTTP/1.1 201"));
    assert!(session.contains(r#"{"ok":true}"#));
    assert!(rejected_session.starts_with("HTTP/1.1 401"));
    assert!(rejected_session.contains(r#"{"err":"token_mismatch"}"#));
    assert!(accepted_quantity.starts_with("HTTP/1.1 201"));
    assert!(accepted_quantity.contains(r#"{"accepted":true,"quantity":3}"#));
    assert!(rejected_quantity.starts_with("HTTP/1.1 400"));
    assert!(rejected_quantity.contains(r#"{"err":"bad_quantity"}"#));
    assert!(accepted_inventory.starts_with("HTTP/1.1 201"));
    assert!(accepted_inventory.contains(r#"{"accepted":true,"quantity":3}"#));
    assert!(rejected_inventory.starts_with("HTTP/1.1 409"));
    assert!(rejected_inventory.contains(r#"{"err":"out_of_stock"}"#));
    assert!(accepted_bulk_inventory.starts_with("HTTP/1.1 201"));
    assert!(accepted_bulk_inventory.contains(r#"{"accepted":true,"quantity":30}"#));
    assert!(rejected_bulk_inventory.starts_with("HTTP/1.1 409"));
    assert!(rejected_bulk_inventory.contains(r#"{"err":"out_of_stock"}"#));
    assert!(accepted_value_inventory.starts_with("HTTP/1.1 201"));
    assert!(accepted_value_inventory.contains(r#"{"accepted":true,"total":875}"#));
    assert!(rejected_value_inventory.starts_with("HTTP/1.1 409"));
    assert!(rejected_value_inventory.contains(r#"{"err":"over_total"}"#));
    assert!(accepted_static_value_inventory.starts_with("HTTP/1.1 201"));
    assert!(accepted_static_value_inventory.contains(r#"{"accepted":true,"quantity":7}"#));
    assert!(rejected_static_value_inventory.starts_with("HTTP/1.1 409"));
    assert!(rejected_static_value_inventory.contains(r#"{"err":"over_total"}"#));
    assert!(accepted_product_value_inventory.starts_with("HTTP/1.1 201"));
    assert!(accepted_product_value_inventory.contains(r#"{"accepted":true,"quantity":7}"#));
    assert!(rejected_product_value_inventory.starts_with("HTTP/1.1 409"));
    assert!(rejected_product_value_inventory.contains(r#"{"err":"over_total"}"#));
    assert!(accepted_scaled_product_value_inventory.starts_with("HTTP/1.1 201"));
    assert!(accepted_scaled_product_value_inventory.contains(r#"{"accepted":true,"total":87500}"#));
    assert!(rejected_scaled_product_value_inventory.starts_with("HTTP/1.1 409"));
    assert!(rejected_scaled_product_value_inventory.contains(r#"{"err":"over_total"}"#));
    assert!(accepted_ifelse_inventory.starts_with("HTTP/1.1 201"));
    assert!(accepted_ifelse_inventory.contains(r#"{"accepted":true,"quantity":3}"#));
    assert!(rejected_ifelse_inventory.starts_with("HTTP/1.1 409"));
    assert!(rejected_ifelse_inventory.contains(r#"{"err":"out_of_stock"}"#));
    assert!(invalid_tiered_inventory.starts_with("HTTP/1.1 400"));
    assert!(invalid_tiered_inventory.contains(r#"{"err":"bad_quantity"}"#));
    assert!(accepted_tiered_inventory.starts_with("HTTP/1.1 201"));
    assert!(accepted_tiered_inventory.contains(r#"{"accepted":true,"quantity":3}"#));
    assert!(rejected_tiered_inventory.starts_with("HTTP/1.1 409"));
    assert!(rejected_tiered_inventory.contains(r#"{"err":"out_of_stock"}"#));
    assert!(accepted_tiered_block_inventory.starts_with("HTTP/1.1 201"));
    assert!(accepted_tiered_block_inventory.contains(r#"{"accepted":true,"quantity":3}"#));
    assert!(rejected_tiered_block_inventory.starts_with("HTTP/1.1 409"));
    assert!(rejected_tiered_block_inventory.contains(r#"{"err":"out_of_stock"}"#));
    assert!(invalid_tiered_fallback_inventory.starts_with("HTTP/1.1 400"));
    assert!(invalid_tiered_fallback_inventory.contains(r#"{"err":"bad_quantity"}"#));
    assert!(accepted_tiered_fallback_inventory.starts_with("HTTP/1.1 201"));
    assert!(accepted_tiered_fallback_inventory.contains(r#"{"accepted":true,"quantity":3}"#));
    assert!(rejected_tiered_fallback_inventory.starts_with("HTTP/1.1 409"));
    assert!(rejected_tiered_fallback_inventory.contains(r#"{"err":"out_of_stock"}"#));
    assert!(accepted_amount.starts_with("HTTP/1.1 201"));
    assert!(accepted_amount.contains(r#"{"accepted":true,"amount":12.5}"#));
    assert!(rejected_amount.starts_with("HTTP/1.1 400"));
    assert!(rejected_amount.contains(r#"{"err":"bad_amount"}"#));
    assert!(accepted_limit.starts_with("HTTP/1.1 201"));
    assert!(accepted_limit.contains(r#"{"accepted":true,"amount":12.5}"#));
    assert!(rejected_limit.starts_with("HTTP/1.1 409"));
    assert!(rejected_limit.contains(r#"{"err":"amount_over_limit"}"#));
    assert!(accepted_product_limit.starts_with("HTTP/1.1 201"));
    assert!(accepted_product_limit.contains(r#"{"accepted":true,"amount":12.5}"#));
    assert!(rejected_product_limit.starts_with("HTTP/1.1 409"));
    assert!(rejected_product_limit.contains(r#"{"err":"amount_over_limit"}"#));
    assert!(sale.starts_with("HTTP/1.1 200"));
    assert!(sale.contains(r#"{"kind":"sale"}"#));
    assert!(regular.starts_with("HTTP/1.1 200"));
    assert!(regular.contains(r#"{"kind":"regular"}"#));
    assert!(expanded.starts_with("HTTP/1.1 200"));
    assert!(expanded.contains(r#"{"mode":"expanded"}"#));
    assert!(compact.starts_with("HTTP/1.1 200"));
    assert!(compact.contains(r#"{"mode":"compact"}"#));

    drop(child);
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn generated_native_server_serves_request_body_int_cast_response() {
    let dir = temp_output_dir("native-request-body-int-cast-server-source");
    std::fs::create_dir_all(&dir).expect("create source dir");
    let path = dir.join("app.orv");
    std::fs::write(
            &path,
            r#"@server {
  @listen 8080
  @route POST /orders {
    @respond 201 { quantity: @body.quantity as int }
  }
  @route POST /orders/next {
    @respond 201 { quantity: (@body.quantity as int) + 1 }
  }
  @route POST /orders/remaining {
    @respond 201 { remaining: 10 - (@body.quantity as int) }
  }
  @route POST /orders/neg {
    @respond 201 { quantity: -(@body.quantity as int) }
  }
  @route POST /orders/cents {
    @respond 201 { cents: (@body.quantity as int) * 100 }
  }
  @route POST /orders/cents-total {
    @respond 201 { cents: (@body.quantity as int) * ((@body.unit_price as int) * 100) }
  }
  @route POST /orders/total {
    @respond 201 { total: (@body.quantity as int) * (@body.unit_price as int) }
  }
  @route POST /orders/total-with-fee {
    @respond 201 { total: (@body.fee as int) + ((@body.quantity as int) * (@body.unit_price as int)) }
  }
  @route POST /orders/scaled-product-fee {
    @respond 201 { total: (@body.base as int) + (((@body.quantity as int) * (@body.unit_price as int)) * 100) }
  }
  @route POST /orders/product-plus-static-fee {
    @respond 201 { total: ((@body.quantity as int) * (@body.unit_price as int)) + 25 }
  }
  @route POST /orders/product-plus-product-fee {
    @respond 201 { total: ((@body.quantity as int) * (@body.unit_price as int)) + ((@body.fee_units as int) * (@body.fee_value as int)) }
  }
  @route POST /orders/triple-product-fee {
    @respond 201 { total: (@body.base as int) + (((@body.quantity as int) * (@body.unit_price as int)) * (@body.bundle_count as int)) }
  }
  @route POST /orders/static-minus-product {
    @respond 201 { remaining: 1000 - ((@body.quantity as int) * (@body.unit_price as int)) }
  }
  @route POST /orders/bundles {
    @respond 201 { bundles: (@body.total as int) / ((@body.quantity as int) * (@body.unit_price as int)) }
  }
  @route POST /orders/remainder-product-left {
    @respond 201 { remainder: ((@body.quantity as int) * (@body.unit_price as int)) % (@body.total as int) }
  }
  @route POST /orders/power {
    @respond 201 { total: (@body.quantity as int) ** (@body.bonus as int) }
  }
  @route POST /orders/power-invalid {
    @respond 201 { total: (@body.quantity as int) ** -1 }
  }
  @route POST /orders/due {
    @respond 201 { due: (@body.total as int) - (@body.discount as int) }
  }
  @route POST /orders/share {
    @respond 201 { share: (@body.total as int) / (@body.parts as int) }
  }
  @route POST /orders/unit-bundle {
    @respond 201 { unit: (@body.total as int) / ((@body.parts as int) * 100) }
  }
  @route POST /orders/unit-bundle-left {
    @respond 201 { unit: ((@body.total as int) * 100) / (@body.parts as int) }
  }
  @route POST /orders/remainder {
    @respond 201 { remainder: (@body.total as int) % (@body.parts as int) }
  }
  @route POST /orders/remainder-scaled {
    @respond 201 { remainder: (@body.total as int) % ((@body.parts as int) * 10) }
  }
  @route POST /orders/remainder-scaled-left {
    @respond 201 { remainder: ((@body.total as int) * 10) % (@body.parts as int) }
  }
  @route POST /orders/available {
    @respond 201 { available: (@body.quantity as int) <= (@body.stock as int) }
  }
  @route POST /orders/available-bulk {
    @respond 201 { available: (@body.quantity as int) <= ((@body.stock as int) * 10) }
  }
  @route POST /orders/covered-min {
    @respond 201 { covered: ((@body.minimum as int) * 100) <= (@body.total as int) }
  }
  @route POST /orders/covered-total {
    @respond 201 { covered: (@body.total as int) <= ((@body.quantity as int) * (@body.unit_price as int)) }
  }
  @route POST /orders/product-covered-static {
    @respond 201 { covered: ((@body.quantity as int) * (@body.unit_price as int)) <= 1000 }
  }
  @route POST /orders/product-covered-product {
    @respond 201 { covered: ((@body.quantity as int) * (@body.unit_price as int)) <= ((@body.stock as int) * (@body.reserve_price as int)) }
  }
}
"#,
        )
        .expect("write source");
    let out = temp_output_dir("native-request-body-int-cast-server-build");

    cmd_build(&path, &out).expect("build artifacts");
    cmd_verify_build(&out).expect("verify int cast native server build");
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let output = std::process::Command::new(cargo)
        .arg("build")
        .arg("--manifest-path")
        .arg(out.join("server").join("native").join("Cargo.toml"))
        .arg("--release")
        .arg("--color")
        .arg("never")
        .output()
        .expect("cargo build int cast native server");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "int cast native server cargo build failed:\n{stderr}"
    );

    let binary = out
        .join("server")
        .join("native")
        .join("target")
        .join("release")
        .join("orv-native-server");
    let mut child = std::process::Command::new(&binary)
        .env("ORV_BUILD_DIR", &out)
        .env("ORV_HOST", "127.0.0.1")
        .env("ORV_PORT", "0")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("spawn generated int cast native server");
    let stderr = child.stderr.take().expect("native server stderr");
    let child = ChildGuard(child);
    let mut stderr = std::io::BufReader::new(stderr);
    let mut line = String::new();
    std::io::BufRead::read_line(&mut stderr, &mut line).expect("native server listen line");
    let address = line
        .trim()
        .strip_prefix("orv native server listening on ")
        .expect("native listen address");

    let response = send_raw_http_json_post(address, "/orders", r#"{"quantity":"7"}"#);
    let next_response = send_raw_http_json_post(address, "/orders/next", r#"{"quantity":"7"}"#);
    let remaining_response =
        send_raw_http_json_post(address, "/orders/remaining", r#"{"quantity":"7"}"#);
    let neg_response = send_raw_http_json_post(address, "/orders/neg", r#"{"quantity":"7"}"#);
    let cents_response = send_raw_http_json_post(address, "/orders/cents", r#"{"quantity":"7"}"#);
    let cents_total_response = send_raw_http_json_post(
        address,
        "/orders/cents-total",
        r#"{"quantity":"2","unit_price":"125"}"#,
    );
    let total_response = send_raw_http_json_post(
        address,
        "/orders/total",
        r#"{"quantity":"7","unit_price":"125"}"#,
    );
    let total_with_fee_response = send_raw_http_json_post(
        address,
        "/orders/total-with-fee",
        r#"{"fee":"25","quantity":"7","unit_price":"125"}"#,
    );
    let scaled_product_fee_response = send_raw_http_json_post(
        address,
        "/orders/scaled-product-fee",
        r#"{"base":"25","quantity":"7","unit_price":"125"}"#,
    );
    let product_plus_static_fee_response = send_raw_http_json_post(
        address,
        "/orders/product-plus-static-fee",
        r#"{"quantity":"7","unit_price":"125"}"#,
    );
    let product_plus_product_fee_response = send_raw_http_json_post(
        address,
        "/orders/product-plus-product-fee",
        r#"{"quantity":"7","unit_price":"125","fee_units":"2","fee_value":"50"}"#,
    );
    let triple_product_fee_response = send_raw_http_json_post(
        address,
        "/orders/triple-product-fee",
        r#"{"base":"25","quantity":"7","unit_price":"125","bundle_count":"2"}"#,
    );
    let static_minus_product_response = send_raw_http_json_post(
        address,
        "/orders/static-minus-product",
        r#"{"quantity":"7","unit_price":"125"}"#,
    );
    let bundles_response = send_raw_http_json_post(
        address,
        "/orders/bundles",
        r#"{"total":"1750","quantity":"7","unit_price":"125"}"#,
    );
    let remainder_product_left_response = send_raw_http_json_post(
        address,
        "/orders/remainder-product-left",
        r#"{"quantity":"7","unit_price":"125","total":"400"}"#,
    );
    let power_response =
        send_raw_http_json_post(address, "/orders/power", r#"{"quantity":"2","bonus":"6"}"#);
    let invalid_power_response =
        send_raw_http_json_post(address, "/orders/power-invalid", r#"{"quantity":"2"}"#);
    let due_response = send_raw_http_json_post(
        address,
        "/orders/due",
        r#"{"total":"875","discount":"125"}"#,
    );
    let share_response =
        send_raw_http_json_post(address, "/orders/share", r#"{"total":"875","parts":"7"}"#);
    let unit_bundle_response = send_raw_http_json_post(
        address,
        "/orders/unit-bundle",
        r#"{"total":"1000","parts":"2"}"#,
    );
    let unit_bundle_left_response = send_raw_http_json_post(
        address,
        "/orders/unit-bundle-left",
        r#"{"total":"5","parts":"2"}"#,
    );
    let remainder_response = send_raw_http_json_post(
        address,
        "/orders/remainder",
        r#"{"total":"875","parts":"6"}"#,
    );
    let remainder_scaled_response = send_raw_http_json_post(
        address,
        "/orders/remainder-scaled",
        r#"{"total":"101","parts":"3"}"#,
    );
    let remainder_scaled_left_response = send_raw_http_json_post(
        address,
        "/orders/remainder-scaled-left",
        r#"{"total":"3","parts":"7"}"#,
    );
    let available_response = send_raw_http_json_post(
        address,
        "/orders/available",
        r#"{"quantity":"7","stock":"10"}"#,
    );
    let available_bulk_response = send_raw_http_json_post(
        address,
        "/orders/available-bulk",
        r#"{"quantity":"70","stock":"7"}"#,
    );
    let covered_min_response = send_raw_http_json_post(
        address,
        "/orders/covered-min",
        r#"{"minimum":"10","total":"1000"}"#,
    );
    let covered_total_response = send_raw_http_json_post(
        address,
        "/orders/covered-total",
        r#"{"total":"875","quantity":"7","unit_price":"125"}"#,
    );
    let product_covered_static_response = send_raw_http_json_post(
        address,
        "/orders/product-covered-static",
        r#"{"quantity":"7","unit_price":"125"}"#,
    );
    let product_covered_product_response = send_raw_http_json_post(
        address,
        "/orders/product-covered-product",
        r#"{"quantity":"7","unit_price":"125","stock":"8","reserve_price":"125"}"#,
    );

    assert!(response.starts_with("HTTP/1.1 201"));
    assert!(response.contains(r#"{"quantity":7}"#));
    assert!(next_response.starts_with("HTTP/1.1 201"));
    assert!(next_response.contains(r#"{"quantity":8}"#));
    assert!(remaining_response.starts_with("HTTP/1.1 201"));
    assert!(remaining_response.contains(r#"{"remaining":3}"#));
    assert!(neg_response.starts_with("HTTP/1.1 201"));
    assert!(neg_response.contains(r#"{"quantity":-7}"#));
    assert!(cents_response.starts_with("HTTP/1.1 201"));
    assert!(cents_response.contains(r#"{"cents":700}"#));
    assert!(cents_total_response.starts_with("HTTP/1.1 201"));
    assert!(cents_total_response.contains(r#"{"cents":25000}"#));
    assert!(total_response.starts_with("HTTP/1.1 201"));
    assert!(total_response.contains(r#"{"total":875}"#));
    assert!(total_with_fee_response.starts_with("HTTP/1.1 201"));
    assert!(total_with_fee_response.contains(r#"{"total":900}"#));
    assert!(scaled_product_fee_response.starts_with("HTTP/1.1 201"));
    assert!(scaled_product_fee_response.contains(r#"{"total":87525}"#));
    assert!(product_plus_static_fee_response.starts_with("HTTP/1.1 201"));
    assert!(product_plus_static_fee_response.contains(r#"{"total":900}"#));
    assert!(product_plus_product_fee_response.starts_with("HTTP/1.1 201"));
    assert!(product_plus_product_fee_response.contains(r#"{"total":975}"#));
    assert!(triple_product_fee_response.starts_with("HTTP/1.1 201"));
    assert!(triple_product_fee_response.contains(r#"{"total":1775}"#));
    assert!(static_minus_product_response.starts_with("HTTP/1.1 201"));
    assert!(static_minus_product_response.contains(r#"{"remaining":125}"#));
    assert!(bundles_response.starts_with("HTTP/1.1 201"));
    assert!(bundles_response.contains(r#"{"bundles":2}"#));
    assert!(remainder_product_left_response.starts_with("HTTP/1.1 201"));
    assert!(remainder_product_left_response.contains(r#"{"remainder":75}"#));
    assert!(power_response.starts_with("HTTP/1.1 201"));
    assert!(power_response.contains(r#"{"total":64}"#));
    assert!(invalid_power_response.starts_with("HTTP/1.1 500"));
    assert!(
        invalid_power_response.contains(r#"{"error":"native request body int arithmetic failed"}"#)
    );
    assert!(due_response.starts_with("HTTP/1.1 201"));
    assert!(due_response.contains(r#"{"due":750}"#));
    assert!(share_response.starts_with("HTTP/1.1 201"));
    assert!(share_response.contains(r#"{"share":125}"#));
    assert!(unit_bundle_response.starts_with("HTTP/1.1 201"));
    assert!(unit_bundle_response.contains(r#"{"unit":5}"#));
    assert!(unit_bundle_left_response.starts_with("HTTP/1.1 201"));
    assert!(unit_bundle_left_response.contains(r#"{"unit":250}"#));
    assert!(remainder_response.starts_with("HTTP/1.1 201"));
    assert!(remainder_response.contains(r#"{"remainder":5}"#));
    assert!(remainder_scaled_response.starts_with("HTTP/1.1 201"));
    assert!(remainder_scaled_response.contains(r#"{"remainder":11}"#));
    assert!(remainder_scaled_left_response.starts_with("HTTP/1.1 201"));
    assert!(remainder_scaled_left_response.contains(r#"{"remainder":2}"#));
    assert!(available_response.starts_with("HTTP/1.1 201"));
    assert!(available_response.contains(r#"{"available":true}"#));
    assert!(available_bulk_response.starts_with("HTTP/1.1 201"));
    assert!(available_bulk_response.contains(r#"{"available":true}"#));
    assert!(covered_min_response.starts_with("HTTP/1.1 201"));
    assert!(covered_min_response.contains(r#"{"covered":true}"#));
    assert!(covered_total_response.starts_with("HTTP/1.1 201"));
    assert!(covered_total_response.contains(r#"{"covered":true}"#));
    assert!(product_covered_static_response.starts_with("HTTP/1.1 201"));
    assert!(product_covered_static_response.contains(r#"{"covered":true}"#));
    assert!(product_covered_product_response.starts_with("HTTP/1.1 201"));
    assert!(product_covered_product_response.contains(r#"{"covered":true}"#));

    drop(child);
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn generated_native_server_serves_request_body_float_cast_response() {
    let dir = temp_output_dir("native-request-body-float-cast-server-source");
    std::fs::create_dir_all(&dir).expect("create source dir");
    let path = dir.join("app.orv");
    std::fs::write(
            &path,
            r#"@server {
  @listen 8080
  @route POST /payments {
    @respond 201 { amount: @body.amount as float }
  }
  @route POST /payments/refund {
    @respond 201 { amount: -(@body.amount as float) }
  }
  @route POST /payments/remaining {
    @respond 201 { remaining: 100.5 - (@body.amount as float) }
  }
  @route POST /payments/total {
    @respond 201 { total: (@body.price as float) * (@body.quantity as float) }
  }
  @route POST /payments/total-plus-fee {
    @respond 201 { total: ((@body.price as float) * (@body.quantity as float)) + 1.25 }
  }
  @route POST /payments/scaled-product-fee {
    @respond 201 { total: (@body.base as float) + (((@body.price as float) * (@body.quantity as float)) * 0.5) }
  }
  @route POST /payments/power {
    @respond 201 { total: (@body.base as float) ** (@body.exp as float) }
  }
  @route POST /payments/under-limit {
    @respond 201 { under_limit: (@body.amount as float) <= (@query.limit as float) }
  }
  @route POST /payments/product-under-static-limit {
    @respond 201 { under_limit: ((@body.price as float) * (@body.quantity as float)) <= 40.0 }
  }
  @route POST /payments/product-plus-product-fee {
    @respond 201 { total: ((@body.price as float) * (@body.quantity as float)) + ((@body.fee as float) * (@body.fee_units as float)) }
  }
  @route POST /payments/triple-product-fee {
    @respond 201 { total: (@body.base as float) + (((@body.price as float) * (@body.quantity as float)) * (@body.multiplier as float)) }
  }
  @route POST /payments/product-under-product-limit {
    @respond 201 { under_limit: ((@body.price as float) * (@body.quantity as float)) <= ((@body.limit_price as float) * (@body.limit_units as float)) }
  }
}
"#,
        )
        .expect("write source");
    let out = temp_output_dir("native-request-body-float-cast-server-build");

    cmd_build(&path, &out).expect("build artifacts");
    cmd_verify_build(&out).expect("verify float cast native server build");
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let output = std::process::Command::new(cargo)
        .arg("build")
        .arg("--manifest-path")
        .arg(out.join("server").join("native").join("Cargo.toml"))
        .arg("--release")
        .arg("--color")
        .arg("never")
        .output()
        .expect("cargo build float cast native server");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "float cast native server cargo build failed:\n{stderr}"
    );

    let binary = out
        .join("server")
        .join("native")
        .join("target")
        .join("release")
        .join("orv-native-server");
    let mut child = std::process::Command::new(&binary)
        .env("ORV_BUILD_DIR", &out)
        .env("ORV_HOST", "127.0.0.1")
        .env("ORV_PORT", "0")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("spawn generated float cast native server");
    let stderr = child.stderr.take().expect("native server stderr");
    let child = ChildGuard(child);
    let mut stderr = std::io::BufReader::new(stderr);
    let mut line = String::new();
    std::io::BufRead::read_line(&mut stderr, &mut line).expect("native server listen line");
    let address = line
        .trim()
        .strip_prefix("orv native server listening on ")
        .expect("native listen address");

    let response = send_raw_http_json_post(address, "/payments", r#"{"amount":"12.5"}"#);
    let refund_response =
        send_raw_http_json_post(address, "/payments/refund", r#"{"amount":"12.5"}"#);
    let remaining_response =
        send_raw_http_json_post(address, "/payments/remaining", r#"{"amount":"12.5"}"#);
    let total_response = send_raw_http_json_post(
        address,
        "/payments/total",
        r#"{"price":"12.5","quantity":"3"}"#,
    );
    let total_plus_fee_response = send_raw_http_json_post(
        address,
        "/payments/total-plus-fee",
        r#"{"price":"12.5","quantity":"3"}"#,
    );
    let scaled_product_fee_response = send_raw_http_json_post(
        address,
        "/payments/scaled-product-fee",
        r#"{"base":"1.25","price":"12.5","quantity":"3"}"#,
    );
    let power_response =
        send_raw_http_json_post(address, "/payments/power", r#"{"base":"2.5","exp":"2.0"}"#);
    let under_limit_response = send_raw_http_json_post(
        address,
        "/payments/under-limit?limit=20.0",
        r#"{"amount":"12.5"}"#,
    );
    let product_under_static_limit_response = send_raw_http_json_post(
        address,
        "/payments/product-under-static-limit",
        r#"{"price":"12.5","quantity":"3"}"#,
    );
    let product_plus_product_fee_response = send_raw_http_json_post(
        address,
        "/payments/product-plus-product-fee",
        r#"{"price":"12.5","quantity":"3","fee":"1.25","fee_units":"2"}"#,
    );
    let triple_product_fee_response = send_raw_http_json_post(
        address,
        "/payments/triple-product-fee",
        r#"{"base":"1.25","price":"12.5","quantity":"3","multiplier":"2.0"}"#,
    );
    let product_under_product_limit_response = send_raw_http_json_post(
        address,
        "/payments/product-under-product-limit",
        r#"{"price":"12.5","quantity":"3","limit_price":"20.0","limit_units":"2"}"#,
    );

    let assert_created = |response: &str, body: &str| {
        assert!(response.starts_with("HTTP/1.1 201"));
        assert!(response.contains(body));
    };
    assert_created(&response, r#"{"amount":12.5}"#);
    assert_created(&refund_response, r#"{"amount":-12.5}"#);
    assert_created(&remaining_response, r#"{"remaining":88}"#);
    assert_created(&total_response, r#"{"total":37.5}"#);
    assert_created(&total_plus_fee_response, r#"{"total":38.75}"#);
    assert_created(&scaled_product_fee_response, r#"{"total":20}"#);
    assert_created(&power_response, r#"{"total":6.25}"#);
    assert_created(&under_limit_response, r#"{"under_limit":true}"#);
    assert_created(
        &product_under_static_limit_response,
        r#"{"under_limit":true}"#,
    );
    assert_created(&product_plus_product_fee_response, r#"{"total":40}"#);
    assert_created(&triple_product_fee_response, r#"{"total":76.25}"#);
    assert_created(
        &product_under_product_limit_response,
        r#"{"under_limit":true}"#,
    );

    drop(child);
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn generated_native_server_serves_route_and_query_numeric_cast_responses() {
    let dir = temp_output_dir("native-param-query-cast-server-source");
    std::fs::create_dir_all(&dir).expect("create source dir");
    let path = dir.join("app.orv");
    std::fs::write(
        &path,
        r#"@server {
  @listen 8080
  @route GET /products/:id.json {
    @respond 200 { id: @param.id as int }
  }
  @route GET /products/:id {
    @respond 200 { id: @param.id as int }
  }
  @route GET /products/:id/math {
    @respond 200 {
      prev: (@param.id as int) - 1,
      doubled: (@param.id as int) * 2,
      half: (@param.id as int) / 2,
      parity: (@param.id as int) % 2
    }
  }
  @route GET /products/:id/shift/:offset {
    @respond 200 { shifted: (@param.id as int) + (@param.offset as int) }
  }
  @route GET /products/:price/float-math/:tax {
    @respond 200 {
      discounted: (@param.price as float) * 0.5,
      taxed: (@param.price as float) + (@param.tax as float)
    }
  }
  @route GET /products/:id/mixed {
    @respond 200 {
      kind: "calc",
      next_id: (@param.id as int) + 1,
      prev_page: (@query.page as int) - 1
    }
  }
  @route GET /search {
    @respond 200 { page: @query.page as float }
  }
  @route GET /search/next {
    @respond 200 { next: (@query.page as int) + 1 }
  }
  @route GET /search/step {
    @respond 200 { next: (@query.page as int) + (@query.step as int) }
  }
  @route GET /search/math {
    @respond 200 {
      prev: (@query.page as int) - 1,
      doubled: (@query.page as int) * 2,
      half: (@query.page as int) / 2,
      parity: (@query.page as int) % 2
    }
  }
  @route GET /search/float-total {
    @respond 200 { total: (@query.amount as float) * (@query.quantity as float) }
  }
  @route GET /search/float-ratio {
    @respond 200 { ratio: 100.0 / (@query.parts as float) }
  }
}
"#,
    )
    .expect("write source");
    let out = temp_output_dir("native-param-query-cast-server-build");

    cmd_build(&path, &out).expect("build artifacts");
    cmd_verify_build(&out).expect("verify param/query cast native server build");
    let launcher = std::fs::read_to_string(out.join("server").join("native").join("main.rs"))
        .expect("native launcher");
    assert!(launcher.contains("fn orv_native_serve() -> std::io::Result<()>"));
    assert!(!launcher.contains(r#"std::process::Command::new("orv")"#));
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let output = std::process::Command::new(cargo)
        .arg("build")
        .arg("--manifest-path")
        .arg(out.join("server").join("native").join("Cargo.toml"))
        .arg("--release")
        .arg("--color")
        .arg("never")
        .output()
        .expect("cargo build param/query cast native server");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "param/query cast native server cargo build failed:\n{stderr}"
    );

    let binary = out
        .join("server")
        .join("native")
        .join("target")
        .join("release")
        .join("orv-native-server");
    let mut child = std::process::Command::new(&binary)
        .env("ORV_BUILD_DIR", &out)
        .env("ORV_HOST", "127.0.0.1")
        .env("ORV_PORT", "0")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("spawn generated param/query cast native server");
    let stderr = child.stderr.take().expect("native server stderr");
    let child = ChildGuard(child);
    let mut stderr = std::io::BufReader::new(stderr);
    let mut line = String::new();
    std::io::BufRead::read_line(&mut stderr, &mut line).expect("native server listen line");
    let address = line
        .trim()
        .strip_prefix("orv native server listening on ")
        .expect("native listen address");

    let route_response = send_raw_http(address, "/products/42");
    let route_suffix_response = send_raw_http(address, "/products/42.json");
    let route_math_response = send_raw_http(address, "/products/13/math");
    let route_shift_response = send_raw_http(address, "/products/13/shift/4");
    let route_float_response = send_raw_http(address, "/products/12.5/float-math/1.25");
    let route_mixed_response = send_raw_http(address, "/products/41/mixed?page=13");
    let query_response = send_raw_http(address, "/search?page=12.5");
    let next_response = send_raw_http(address, "/search/next?page=12");
    let step_response = send_raw_http(address, "/search/step?page=12&step=3");
    let math_response = send_raw_http(address, "/search/math?page=13");
    let float_total_response = send_raw_http(address, "/search/float-total?amount=12.5&quantity=3");
    let float_ratio_response = send_raw_http(address, "/search/float-ratio?parts=4");

    assert!(route_response.starts_with("HTTP/1.1 200"));
    assert!(route_response.contains(r#"{"id":42}"#));
    assert!(route_suffix_response.starts_with("HTTP/1.1 200"));
    assert!(route_suffix_response.contains(r#"{"id":42}"#));
    assert!(route_math_response.starts_with("HTTP/1.1 200"));
    assert!(route_math_response.contains(r#"{"prev":12,"doubled":26,"half":6,"parity":1}"#));
    assert!(route_shift_response.starts_with("HTTP/1.1 200"));
    assert!(route_shift_response.contains(r#"{"shifted":17}"#));
    assert!(route_float_response.starts_with("HTTP/1.1 200"));
    assert!(route_float_response.contains(r#"{"discounted":6.25,"taxed":13.75}"#));
    assert!(route_mixed_response.starts_with("HTTP/1.1 200"));
    assert!(route_mixed_response.contains(r#"{"kind":"calc","next_id":42,"prev_page":12}"#));
    assert!(query_response.starts_with("HTTP/1.1 200"));
    assert!(query_response.contains(r#"{"page":12.5}"#));
    assert!(next_response.starts_with("HTTP/1.1 200"));
    assert!(next_response.contains(r#"{"next":13}"#));
    assert!(step_response.starts_with("HTTP/1.1 200"));
    assert!(step_response.contains(r#"{"next":15}"#));
    assert!(math_response.starts_with("HTTP/1.1 200"));
    assert!(math_response.contains(r#"{"prev":12,"doubled":26,"half":6,"parity":1}"#));
    assert!(float_total_response.starts_with("HTTP/1.1 200"));
    assert!(float_total_response.contains(r#"{"total":37.5}"#));
    assert!(float_ratio_response.starts_with("HTTP/1.1 200"));
    assert!(float_ratio_response.contains(r#"{"ratio":25}"#));

    drop(child);
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn build_writes_cargo_checkable_native_launcher_package() {
    let path = workspace_path(&["fixtures", "e2e", "hello.orv"]);
    let out = temp_output_dir("native-server-cargo-check");

    cmd_build(&path, &out).expect("build artifacts");

    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let output = std::process::Command::new(cargo)
        .arg("check")
        .arg("--manifest-path")
        .arg(out.join("server").join("native").join("Cargo.toml"))
        .arg("--color")
        .arg("never")
        .output()
        .expect("cargo check native launcher");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "native launcher cargo check failed:\n{stderr}"
    );
    assert!(
        !stderr.contains("warning:"),
        "native launcher cargo check should be warning-free:\n{stderr}"
    );

    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn build_uses_reference_native_launcher_for_dynamic_handlers() {
    let dir = temp_output_dir("native-server-dynamic-fallback-source");
    std::fs::create_dir_all(&dir).expect("create source dir");
    let path = dir.join("app.orv");
    std::fs::write(
            &path,
            r"@server {
  @listen 8080
  @route POST /echo {
    @respond 201 { received: (@body.id as int) + ((((@body.bonus as int) * (@body.scale as int)) * (@body.extra as int)) * (@body.more as int)) }
  }
}
",
        )
        .expect("write source");
    let out = temp_output_dir("native-server-dynamic-fallback");

    cmd_build(&path, &out).expect("build artifacts");

    let source = std::fs::read_to_string(out.join("server").join("native").join("main.rs"))
        .expect("native source");
    let native_plan = read_json_value(&out.join(NATIVE_SERVER_PLAN_PATH)).expect("native plan");
    let image_plan =
        read_json_value(&out.join(NATIVE_RUNTIME_IMAGE_PLAN_PATH)).expect("image plan");
    assert_eq!(native_plan["status"], "planned");
    assert!(native_plan["blocked_by"]
        .as_array()
        .expect("blocked_by")
        .iter()
        .any(|item| item == "native-codegen"));
    assert_eq!(image_plan["status"], "planned");
    assert!(image_plan["blocked_by"]
        .as_array()
        .expect("blocked_by")
        .iter()
        .any(|item| item == "native-codegen"));
    assert!(source.contains("fn orv_native_reference_bridge("));
    assert!(source.contains(r#"std::process::Command::new("orv")"#));
    assert!(source.contains(r#".arg("run-artifact")"#));
    assert!(!source.contains("fn orv_native_serve() -> std::io::Result<()>"));
    cmd_verify_build(&out).expect("verify dynamic fallback build");
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let output = std::process::Command::new(cargo)
        .arg("check")
        .arg("--manifest-path")
        .arg(out.join("server").join("native").join("Cargo.toml"))
        .arg("--color")
        .arg("never")
        .output()
        .expect("cargo check dynamic fallback native launcher");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "dynamic fallback native launcher cargo check failed:\n{stderr}"
    );

    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn native_host_desktop_contract_freezes_public_object_keys_and_types() {
    fn assert_keys(value: &serde_json::Value, expected: &[&str], context: &str) {
        let object = value
            .as_object()
            .unwrap_or_else(|| panic!("{context} must be an object"));
        let actual = object
            .keys()
            .map(String::as_str)
            .collect::<std::collections::BTreeSet<_>>();
        let expected = expected
            .iter()
            .copied()
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(actual, expected, "{context} keys drifted");
    }

    let dir = temp_output_dir("native-host-desktop-contract");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let path = dir.join("app.orv");
    std::fs::write(&path, "@out \"desktop-contract\"\n").expect("write source");
    let out = dir.join("editor");

    cmd_editor_export(&path, &out).expect("editor export");

    let package = read_json_value(&out.join(EDITOR_NATIVE_HOST_DESKTOP_PACKAGE_PATH))
        .expect("desktop package");
    let shell =
        editor_native_host_desktop_shell_json(&out, "127.0.0.1:38123").expect("desktop shell");
    let native_host =
        read_json_value(&out.join(EDITOR_NATIVE_HOST_MANIFEST_PATH)).expect("native host");

    assert_keys(
        &package,
        &[
            "schema_version",
            "kind",
            "runtime",
            "entry",
            "export_root",
            "artifacts",
            "platform_matrix",
            "desktop_app",
            "packaging",
            "lifecycle",
            "process_policy",
            "refresh",
            "source_permissions",
        ],
        "desktop package",
    );
    assert_eq!(package["schema_version"], 1);
    assert_eq!(package["kind"], "orv.editor.native_host.desktop_package");
    assert_eq!(package["runtime"], "local-http-bridge");
    assert!(package["entry"].as_str().is_some());

    assert_keys(
        &package["platform_matrix"],
        &[
            "schema_version",
            "kind",
            "default_platform",
            "implemented_count",
            "planned_count",
            "targets",
        ],
        "desktop platform matrix",
    );
    assert_eq!(
        package["platform_matrix"]["kind"],
        "orv.editor.native_host.desktop_platform_matrix"
    );
    let targets = package["platform_matrix"]["targets"]
        .as_array()
        .expect("platform matrix targets");
    assert_eq!(targets.len(), 3);
    assert_keys(
        &targets[0],
        &[
            "platform",
            "status",
            "container",
            "package",
            "main",
            "session_artifact",
            "packaging",
            "capabilities",
            "verification",
        ],
        "macos platform target",
    );
    assert_keys(
        &targets[1],
        &[
            "platform",
            "status",
            "container",
            "blocked_by",
            "shared_contracts",
        ],
        "windows platform target",
    );
    assert_keys(
        &targets[2],
        &[
            "platform",
            "status",
            "container",
            "blocked_by",
            "shared_contracts",
        ],
        "linux platform target",
    );
    for (index, platform) in [(1, "windows"), (2, "linux")] {
        assert_eq!(targets[index]["platform"], platform);
        assert_eq!(targets[index]["status"], "planned");
        assert!(targets[index]["blocked_by"]
            .as_array()
            .is_some_and(|blockers| !blockers.is_empty()));
        let shared_contracts = targets[index]["shared_contracts"]
            .as_array()
            .expect("planned desktop shared contracts");
        assert!(shared_contracts
            .iter()
            .any(|contract| contract == EDITOR_NATIVE_HOST_DESKTOP_PACKAGE_PATH));
        assert!(shared_contracts
            .iter()
            .any(|contract| contract == EDITOR_NATIVE_HOST_DESKTOP_SESSION_PATH));
        assert!(shared_contracts
            .iter()
            .any(|contract| contract == EDITOR_NATIVE_HOST_BRIDGE_JS_PATH));
    }

    assert_keys(
        &package["source_permissions"],
        &[
            "mode",
            "default",
            "denied_mode",
            "reveal_requires_origin_id",
            "webview_injection",
            "decision_event",
            "blocked_event",
            "root_count",
            "source_count",
            "allowed_roots",
            "source_hashes",
            "prompt",
        ],
        "desktop source permissions",
    );
    assert_keys(
        &package["source_permissions"]["prompt"],
        &["title", "allow_label", "read_only_label", "quit_label"],
        "desktop source permission prompt",
    );
    assert_eq!(
        package["source_permissions"]["denied_mode"],
        "open-read-only"
    );
    assert_eq!(
        package["source_permissions"]["webview_injection"],
        "orvNativeHostSourcePermissions"
    );
    assert!(package["source_permissions"]["allowed_roots"]
        .as_array()
        .is_some_and(|roots| !roots.is_empty()));
    assert!(package["source_permissions"]["source_hashes"]
        .as_array()
        .is_some_and(|sources| !sources.is_empty()));

    assert_keys(
        &shell,
        &[
            "schema_version",
            "kind",
            "status",
            "root",
            "package",
            "lifecycle",
            "process_supervision",
            "webview",
            "refresh",
            "platform_matrix",
            "source_permission_prompt",
            "artifact_checks",
            "session_artifact",
        ],
        "desktop shell",
    );
    assert_eq!(shell["schema_version"], 1);
    assert_eq!(shell["kind"], "orv.editor.native_host.desktop_shell");
    assert_eq!(shell["platform_matrix"], package["platform_matrix"]);
    assert_eq!(
        shell["source_permission_prompt"]["webview_injection"],
        "orvNativeHostSourcePermissions"
    );

    assert_eq!(
        native_host["host"]["desktop_platform_matrix"],
        package["platform_matrix"]
    );
    assert_eq!(
        native_host["capabilities"]["native_host_desktop_platform_matrix"],
        true
    );
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn native_host_desktop_shell_rejects_extra_package_root_key() {
    let dir = temp_output_dir("native-host-desktop-extra-package-root");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let path = dir.join("app.orv");
    std::fs::write(&path, "@out \"desktop-contract\"\n").expect("write source");
    let out = dir.join("editor");

    cmd_editor_export(&path, &out).expect("editor export");
    let package_path = out.join(EDITOR_NATIVE_HOST_DESKTOP_PACKAGE_PATH);
    let mut package = read_json_value(&package_path).expect("desktop package");
    package["unexpected"] = serde_json::json!(true);
    write_json(&package_path, &package).expect("write corrupt desktop package");

    let err = editor_native_host_desktop_shell_json(&out, "127.0.0.1:38123")
        .expect_err("extra desktop package root key must fail");

    assert!(err
        .to_string()
        .contains("desktop package keys must match contract"));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn native_host_desktop_shell_rejects_extra_platform_target_key() {
    let dir = temp_output_dir("native-host-desktop-extra-platform-target");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let path = dir.join("app.orv");
    std::fs::write(&path, "@out \"desktop-contract\"\n").expect("write source");
    let out = dir.join("editor");

    cmd_editor_export(&path, &out).expect("editor export");
    let package_path = out.join(EDITOR_NATIVE_HOST_DESKTOP_PACKAGE_PATH);
    let mut package = read_json_value(&package_path).expect("desktop package");
    package["platform_matrix"]["targets"][0]["unexpected"] = serde_json::json!("drift");
    write_json(&package_path, &package).expect("write corrupt desktop package");

    let err = editor_native_host_desktop_shell_json(&out, "127.0.0.1:38123")
        .expect_err("extra desktop platform target key must fail");

    assert!(err
        .to_string()
        .contains("desktop platform_matrix targets[0] keys must match contract"));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn native_host_desktop_run_rejects_extra_session_root_key() {
    let dir = temp_output_dir("native-host-desktop-extra-session-root");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let path = dir.join("app.orv");
    std::fs::write(&path, "@out \"desktop-contract\"\n").expect("write source");
    let out = dir.join("editor");

    cmd_editor_export(&path, &out).expect("editor export");
    let mut session =
        editor_native_host_desktop_shell_json(&out, "127.0.0.1:38123").expect("desktop shell");
    session["unexpected"] = serde_json::json!(true);
    let session_path = out.join(EDITOR_NATIVE_HOST_DESKTOP_SESSION_PATH);
    write_json(&session_path, &session).expect("write corrupt desktop session");

    let err = editor_native_host_desktop_run_session_json(&session_path, "127.0.0.1:38124")
        .expect_err("extra desktop session root key must fail");

    assert!(err
        .to_string()
        .contains("desktop shell keys must match contract"));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn native_host_desktop_shell_rejects_empty_planned_platform_blockers() {
    let dir = temp_output_dir("native-host-desktop-empty-platform-blockers");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let path = dir.join("app.orv");
    std::fs::write(&path, "@out \"desktop-contract\"\n").expect("write source");
    let out = dir.join("editor");

    cmd_editor_export(&path, &out).expect("editor export");
    let package_path = out.join(EDITOR_NATIVE_HOST_DESKTOP_PACKAGE_PATH);
    let mut package = read_json_value(&package_path).expect("desktop package");
    package["platform_matrix"]["targets"][1]["blocked_by"] = serde_json::json!([]);
    write_json(&package_path, &package).expect("write corrupt desktop package");

    let err = editor_native_host_desktop_shell_json(&out, "127.0.0.1:38123")
        .expect_err("empty planned desktop blockers must fail");

    assert!(err
        .to_string()
        .contains("desktop platform_matrix targets[1].blocked_by must be non-empty"));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn native_host_desktop_shell_rejects_empty_planned_platform_shared_contracts() {
    let dir = temp_output_dir("native-host-desktop-empty-shared-contracts");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let path = dir.join("app.orv");
    std::fs::write(&path, "@out \"desktop-contract\"\n").expect("write source");
    let out = dir.join("editor");

    cmd_editor_export(&path, &out).expect("editor export");
    let package_path = out.join(EDITOR_NATIVE_HOST_DESKTOP_PACKAGE_PATH);
    let mut package = read_json_value(&package_path).expect("desktop package");
    package["platform_matrix"]["targets"][1]["shared_contracts"] = serde_json::json!([]);
    write_json(&package_path, &package).expect("write corrupt desktop package");

    let err = editor_native_host_desktop_shell_json(&out, "127.0.0.1:38123")
        .expect_err("empty planned desktop shared contracts must fail");

    assert!(err
        .to_string()
        .contains("desktop platform_matrix targets[1].shared_contracts must be non-empty"));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn native_host_desktop_shell_rejects_extra_source_permission_key() {
    let dir = temp_output_dir("native-host-desktop-extra-source-permission");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let path = dir.join("app.orv");
    std::fs::write(&path, "@out \"desktop-contract\"\n").expect("write source");
    let out = dir.join("editor");

    cmd_editor_export(&path, &out).expect("editor export");
    let package_path = out.join(EDITOR_NATIVE_HOST_DESKTOP_PACKAGE_PATH);
    let mut package = read_json_value(&package_path).expect("desktop package");
    package["source_permissions"]["unexpected"] = serde_json::json!("drift");
    write_json(&package_path, &package).expect("write corrupt desktop package");

    let err = editor_native_host_desktop_shell_json(&out, "127.0.0.1:38123")
        .expect_err("extra desktop source permission key must fail");

    assert!(err
        .to_string()
        .contains("desktop source permissions keys must match contract"));
    let _ = std::fs::remove_dir_all(dir);
}
