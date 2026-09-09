use super::*;

#[test]
fn build_subcommand_is_accepted() {
    let parsed = Cli::try_parse_from([
        "orv",
        "build",
        "fixtures/e2e/hello.orv",
        "--out",
        "target/orv-build-test",
    ]);
    if let Err(err) = parsed {
        panic!("{}", err.render());
    }
}

#[test]
fn build_prod_subcommand_flag_is_accepted() {
    let parsed = Cli::try_parse_from([
        "orv",
        "build",
        "fixtures/e2e/hello.orv",
        "--out",
        "target/orv-prod-build-test",
        "--prod",
    ]);
    if let Err(err) = parsed {
        panic!("{}", err.render());
    }
}

#[test]
fn build_writes_manifest_origin_map_and_project_graph() {
    let path = workspace_path(&["fixtures", "e2e", "hello.orv"]);
    let out = temp_output_dir("build-artifacts");

    cmd_build(&path, &out).expect("build artifacts");

    let manifest_path = out.join("build-manifest.json");
    let origin_map_path = out.join("origin-map.json");
    let bundle_plan_path = out.join("bundle-plan.json");
    let server_artifact_path = out.join("server").join("app.orv-runtime.json");
    let server_launch_path = out.join("server").join("launch.json");
    let native_server_plan_path = out.join("server").join("native-server.json");
    let native_server_package_path = out.join("server").join("native").join("Cargo.toml");
    let native_server_source_path = out.join("server").join("native").join("main.rs");
    let native_server_routes_path = out.join("server").join("native").join("routes.rs");
    let native_server_router_path = out.join("server").join("native").join("router.rs");
    let native_server_handlers_path = out.join("server").join("native").join("handlers.rs");
    let graph_path = out.join("project-graph.json");
    let source_bundle_path = out.join("source-bundle.json");
    assert!(
        manifest_path.is_file(),
        "missing {}",
        manifest_path.display()
    );
    assert!(
        origin_map_path.is_file(),
        "missing {}",
        origin_map_path.display()
    );
    assert!(
        bundle_plan_path.is_file(),
        "missing {}",
        bundle_plan_path.display()
    );
    assert!(
        server_artifact_path.is_file(),
        "missing {}",
        server_artifact_path.display()
    );
    assert!(
        server_launch_path.is_file(),
        "missing {}",
        server_launch_path.display()
    );
    assert!(
        native_server_plan_path.is_file(),
        "missing {}",
        native_server_plan_path.display()
    );
    assert!(
        native_server_source_path.is_file(),
        "missing {}",
        native_server_source_path.display()
    );
    assert!(
        native_server_routes_path.is_file(),
        "missing {}",
        native_server_routes_path.display()
    );
    assert!(
        native_server_router_path.is_file(),
        "missing {}",
        native_server_router_path.display()
    );
    assert!(
        native_server_handlers_path.is_file(),
        "missing {}",
        native_server_handlers_path.display()
    );
    assert!(
        native_server_package_path.is_file(),
        "missing {}",
        native_server_package_path.display()
    );
    assert!(graph_path.is_file(), "missing {}", graph_path.display());
    assert!(
        source_bundle_path.is_file(),
        "missing {}",
        source_bundle_path.display()
    );

    let manifest: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&manifest_path).expect("manifest"))
            .expect("manifest json");
    assert_eq!(manifest["schema_version"], 1);
    assert_eq!(manifest["entry"], path.display().to_string());
    assert_eq!(manifest["runtime"], "reference-interpreter");
    let runtime_features = manifest["capabilities"]["runtime_features"]
        .as_array()
        .expect("runtime features array");
    assert!(runtime_features
        .iter()
        .any(|feature| feature == "http_server"));
    assert!(runtime_features.iter().any(|feature| feature == "router"));
    assert!(manifest["artifacts"]
        .as_array()
        .expect("artifacts array")
        .iter()
        .any(|artifact| artifact["kind"] == "origin_map" && artifact["path"] == "origin-map.json"));
    assert!(manifest["artifacts"]
        .as_array()
        .expect("artifacts array")
        .iter()
        .any(
            |artifact| artifact["kind"] == "bundle_plan" && artifact["path"] == "bundle-plan.json"
        ));
    assert!(manifest["artifacts"]
        .as_array()
        .expect("artifacts array")
        .iter()
        .any(|artifact| artifact["kind"] == "project_graph"
            && artifact["path"] == "project-graph.json"));
    assert!(manifest["artifacts"]
        .as_array()
        .expect("artifacts array")
        .iter()
        .any(|artifact| artifact["kind"] == "source_bundle"
            && artifact["path"] == "source-bundle.json"));
    assert!(manifest["artifacts"]
        .as_array()
        .expect("artifacts array")
        .iter()
        .any(|artifact| artifact["kind"] == "server_runtime"
            && artifact["path"] == "server/app.orv-runtime.json"));
    assert!(manifest["artifacts"]
        .as_array()
        .expect("artifacts array")
        .iter()
        .any(|artifact| artifact["kind"] == "server_launcher"
            && artifact["path"] == "server/launch.json"));
    assert!(manifest["artifacts"]
        .as_array()
        .expect("artifacts array")
        .iter()
        .any(|artifact| artifact["kind"] == "native_server_plan"
            && artifact["path"] == "server/native-server.json"));
    assert!(manifest["artifacts"]
        .as_array()
        .expect("artifacts array")
        .iter()
        .any(
            |artifact| artifact["kind"] == "native_server_launcher_source"
                && artifact["path"] == "server/native/main.rs"
        ));
    assert!(manifest["artifacts"]
        .as_array()
        .expect("artifacts array")
        .iter()
        .any(|artifact| artifact["kind"] == "native_server_routes_source"
            && artifact["path"] == "server/native/routes.rs"));
    assert!(manifest["artifacts"]
        .as_array()
        .expect("artifacts array")
        .iter()
        .any(|artifact| artifact["kind"] == "native_server_router_source"
            && artifact["path"] == "server/native/router.rs"));
    assert!(manifest["artifacts"]
        .as_array()
        .expect("artifacts array")
        .iter()
        .any(
            |artifact| artifact["kind"] == "native_server_handlers_source"
                && artifact["path"] == "server/native/handlers.rs"
        ));
    assert!(manifest["artifacts"]
        .as_array()
        .expect("artifacts array")
        .iter()
        .any(
            |artifact| artifact["kind"] == "native_server_launcher_package"
                && artifact["path"] == "server/native/Cargo.toml"
        ));
    let source_bundle: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&source_bundle_path).expect("source bundle"))
            .expect("source bundle json");
    assert_eq!(source_bundle["schema_version"], 1);
    assert!(source_bundle["files"]
        .as_array()
        .expect("source files")
        .iter()
        .any(|file| file["source"]
            .as_str()
            .is_some_and(|source| source.contains("@route GET /ping"))));
    let plan: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&bundle_plan_path).expect("plan"))
            .expect("bundle plan json");
    assert_eq!(plan["schema_version"], 1);
    assert!(plan["bundles"]
        .as_array()
        .expect("bundles array")
        .iter()
        .any(|bundle| bundle["kind"] == "server_runtime"
            && bundle["path"] == "server/app.orv-runtime.json"));
    assert!(plan["bundles"]
        .as_array()
        .expect("bundles array")
        .iter()
        .any(
            |bundle| bundle["kind"] == "server_launcher" && bundle["path"] == "server/launch.json"
        ));
    assert!(plan["bundles"]
        .as_array()
        .expect("bundles array")
        .iter()
        .any(|bundle| bundle["kind"] == "native_server_plan"
            && bundle["path"] == "server/native-server.json"));
    assert!(plan["bundles"]
        .as_array()
        .expect("bundles array")
        .iter()
        .any(|bundle| bundle["kind"] == "native_server_launcher_source"
            && bundle["path"] == "server/native/main.rs"));
    assert!(plan["bundles"]
        .as_array()
        .expect("bundles array")
        .iter()
        .any(|bundle| bundle["kind"] == "native_server_routes_source"
            && bundle["path"] == "server/native/routes.rs"));
    assert!(plan["bundles"]
        .as_array()
        .expect("bundles array")
        .iter()
        .any(|bundle| bundle["kind"] == "native_server_router_source"
            && bundle["path"] == "server/native/router.rs"));
    assert!(plan["bundles"]
        .as_array()
        .expect("bundles array")
        .iter()
        .any(|bundle| bundle["kind"] == "native_server_handlers_source"
            && bundle["path"] == "server/native/handlers.rs"));
    assert!(plan["bundles"]
        .as_array()
        .expect("bundles array")
        .iter()
        .any(|bundle| bundle["kind"] == "native_server_launcher_package"
            && bundle["path"] == "server/native/Cargo.toml"));
    let server_artifact: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(&server_artifact_path).expect("server artifact"),
    )
    .expect("server artifact json");
    assert_eq!(server_artifact["schema_version"], 1);
    assert_eq!(server_artifact["runtime"], "reference-interpreter");
    assert_eq!(server_artifact["listen"]["port"], 0);
    assert!(server_artifact["listen"]["origin_id"]
        .as_str()
        .is_some_and(|origin| origin.starts_with("ori_")));
    assert!(server_artifact["routes"]
        .as_array()
        .expect("routes array")
        .iter()
        .any(|route| route["method"] == "GET" && route["path"] == "/ping"));
    assert!(server_artifact["routes"][0]["response_origin_ids"]
        .as_array()
        .expect("route response origins")
        .iter()
        .any(|origin| origin
            .as_str()
            .is_some_and(|origin| origin.starts_with("ori_"))));
    assert!(server_artifact["source_bundle"]["files"]
        .as_array()
        .expect("source bundle files")
        .iter()
        .any(|file| file["source"]
            .as_str()
            .is_some_and(|source| source.contains("@route GET /ping"))
            && file["content_hash"]
                .as_str()
                .is_some_and(|hash| hash.starts_with("fnv1a64:"))));
    let launch: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(&server_launch_path).expect("server launch artifact"),
    )
    .expect("server launch json");
    assert_eq!(launch["schema_version"], 1);
    assert_eq!(launch["runtime"], "reference-interpreter");
    assert_eq!(launch["artifact"], "server/app.orv-runtime.json");
    assert_eq!(launch["protocol"], "http1");
    assert_eq!(launch["listen"], server_artifact["listen"]);
    assert_eq!(launch["command"][0], "orv");
    assert_eq!(launch["command"][1], "run-artifact");
    assert_eq!(launch["command"][2], "server/app.orv-runtime.json");
    assert!(launch["routes"]
        .as_array()
        .expect("launch routes")
        .iter()
        .any(|route| route["method"] == "GET" && route["path"] == "/ping"));
    let native_plan: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(&native_server_plan_path).expect("native server plan"),
    )
    .expect("native server plan json");
    assert_eq!(native_plan["schema_version"], 1);
    assert_eq!(native_plan["kind"], "native_server_plan");
    assert_eq!(native_plan["status"], "direct_http");
    assert_eq!(native_plan["artifact"], "server/app.orv-runtime.json");
    assert_eq!(native_plan["launcher"], "server/launch.json");
    assert_eq!(native_plan["source"], "server/native/main.rs");
    assert_eq!(native_plan["routes_source"], "server/native/routes.rs");
    assert_eq!(native_plan["router_source"], "server/native/router.rs");
    assert_eq!(native_plan["handlers_source"], "server/native/handlers.rs");
    assert_eq!(native_plan["package"], "server/native/Cargo.toml");
    assert_eq!(native_plan["runtime"], "reference-interpreter");
    assert_eq!(native_plan["target"]["kind"], "server_binary");
    assert_eq!(native_plan["target"]["path"], "server/app");
    assert_eq!(native_plan["target"]["protocol"], "http1");
    assert_eq!(
        native_plan["commands"]["build"],
        serde_json::json!([
            "cargo",
            "build",
            "--manifest-path",
            "server/native/Cargo.toml",
            "--release"
        ])
    );
    assert_eq!(native_plan["commands"]["run"]["env"]["ORV_BUILD_DIR"], ".");
    assert_eq!(
        native_plan["commands"]["run"]["command"],
        serde_json::json!(["./server/native/target/release/orv-native-server"])
    );
    assert_eq!(native_plan["listen"], server_artifact["listen"]);
    assert!(json_routes_include(&native_plan["routes"], "GET", "/ping"));
    assert!(!native_plan["blocked_by"]
        .as_array()
        .expect("blocked_by")
        .iter()
        .any(|item| item == "native-codegen"));
    assert!(!native_plan["blocked_by"]
        .as_array()
        .expect("blocked_by")
        .iter()
        .any(|item| item == "native-runtime-image"));
    let native_source = std::fs::read_to_string(&native_server_source_path).expect("native source");
    assert!(native_source.contains("const ORV_SERVER_ARTIFACT"));
    assert!(native_source.contains("server/app.orv-runtime.json"));
    assert!(native_source.contains("build_dir.join(ORV_NATIVE_SERVER_PLAN)"));
    assert!(native_source.contains("fn orv_build_dir() -> std::path::PathBuf"));
    assert!(native_source.contains("std::env::current_exe()"));
    assert!(native_source.contains("native_plan.is_file()"));
    assert!(native_source.contains("build_dir.join(ORV_SERVER_ARTIFACT)"));
    assert!(native_source.contains("artifact.is_file()"));
    assert!(native_source.contains("fn orv_native_serve() -> std::io::Result<()>"));
    assert!(native_source.contains("std::net::TcpListener::bind(orv_native_listen_address())"));
    assert!(native_source.contains("router::orv_native_dispatch_with_request("));
    assert!(native_source.contains("request.body"));
    assert!(native_source.contains("fn orv_native_http_response("));
    assert!(!native_source.contains("Command::new(\"orv\")"));
    assert!(!native_source.contains(".arg(\"run-artifact\")"));
    assert!(native_source.contains("mod routes;"));
    assert!(native_source.contains("mod router;"));
    assert!(native_source.contains("mod handlers;"));
    assert!(native_source.contains("routes::ORV_NATIVE_ROUTE_COUNT"));
    assert!(native_source
        .contains(r#"routes::orv_native_match_route("__orv_probe__", "__orv_probe__")"#));
    assert!(native_source.contains("router::ORV_NATIVE_HANDLER_COUNT"));
    assert!(
        native_source.contains(r#"router::orv_native_dispatch("__orv_probe__", "__orv_probe__")"#)
    );
    assert!(native_source.contains("handlers::ORV_NATIVE_HANDLER_COUNT"));
    let native_route_table_source =
        std::fs::read_to_string(&native_server_routes_path).expect("native routes source");
    let route_origin = server_artifact["routes"][0]["origin_id"]
        .as_str()
        .expect("route origin id");
    let response_origin = server_artifact["routes"][0]["response_origin_ids"][0]
        .as_str()
        .expect("response origin id");
    assert!(native_route_table_source.contains("pub struct OrvNativeRoute"));
    assert!(native_route_table_source.contains("pub response_origin_ids: &'static [&'static str]"));
    assert!(native_route_table_source.contains("pub const ORV_NATIVE_ROUTES"));
    assert!(native_route_table_source.contains("method: \"GET\""));
    assert!(native_route_table_source.contains("path: \"/ping\""));
    assert!(native_route_table_source.contains("pub fn orv_native_match_route("));
    assert!(native_route_table_source.contains("orv_native_route_path_params(route.path, path)"));
    assert!(native_route_table_source.contains(&format!("origin_id: \"{route_origin}\"")));
    assert!(native_route_table_source
        .contains(&format!("response_origin_ids: &[\"{response_origin}\"]")));
    assert!(native_route_table_source
        .contains("pub const ORV_NATIVE_ROUTE_COUNT: usize = ORV_NATIVE_ROUTES.len();"));
    let native_router_source_text =
        std::fs::read_to_string(&native_server_router_path).expect("native router source");
    assert!(native_router_source_text.contains("use crate::{handlers, routes};"));
    assert!(native_router_source_text.contains("pub struct OrvNativeDispatch"));
    assert!(native_router_source_text.contains("pub const ORV_NATIVE_HANDLER_COUNT"));
    assert!(native_router_source_text.contains("pub fn orv_native_dispatch("));
    assert!(native_router_source_text.contains("routes::orv_native_match_route(method, path)"));
    assert!(native_router_source_text.contains("handlers::orv_native_handle_route(&route_match)"));
    assert!(native_router_source_text.contains("pub response_origin_id: Option<&'static str>"));
    assert!(native_router_source_text.contains("response_origin_id: response.response_origin_id"));
    assert!(native_router_source_text.contains("status: 404"));
    let native_handlers_source_text =
        std::fs::read_to_string(&native_server_handlers_path).expect("native handlers source");
    assert!(native_handlers_source_text.contains("use crate::routes;"));
    assert!(native_handlers_source_text.contains("pub struct OrvNativeHandlerDescriptor"));
    assert!(native_handlers_source_text.contains("pub struct OrvNativeHandlerResponse"));
    assert!(native_handlers_source_text.contains("pub const ORV_NATIVE_HANDLERS"));
    assert!(native_handlers_source_text.contains("pub const ORV_NATIVE_HANDLER_COUNT"));
    assert!(native_handlers_source_text.contains("pub fn orv_native_handle_route("));
    assert!(native_handlers_source_text.contains(&format!("route_origin_id: \"{route_origin}\"")));
    assert!(native_handlers_source_text
        .contains(&format!("response_origin_ids: &[\"{response_origin}\"]")));
    assert!(native_handlers_source_text.contains("response_origin_id: Some("));
    assert!(native_handlers_source_text.contains("status: 200"));
    assert!(native_handlers_source_text.contains(r#"body: "{\"ok\":true,\"msg\":\"pong\"}""#));
    assert!(!native_handlers_source_text.contains("native route body lowering pending"));
    let native_package =
        std::fs::read_to_string(&native_server_package_path).expect("native package");
    assert!(native_package.contains("name = \"orv-native-server\""));
    assert!(native_package.contains("path = \"main.rs\""));

    cmd_verify_build(&out).expect("verify build artifacts");

    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn build_prod_records_env_configured_http_commerce_endpoints() {
    let dir = temp_output_dir("build-prod-env-http-commerce-source");
    std::fs::create_dir_all(&dir).expect("create env http commerce source dir");
    let path = dir.join("app.orv");
    std::fs::write(
        &path,
        r#"@server {
  @listen 8080
  let payments = @payment.connect(@env.PAYMENT_ADAPTER_URL ?? "http://payments.internal/capture")
  let shipping = @shipping.connect(@env.SHIPPING_ADAPTER_URL ?? "http://shipping.internal/book")
  @route POST /checkout {
    let captured = payments.capture({ orderId: "o_1", amount: 42, method: "card" })
    let booked = shipping.book({ orderId: "o_1", carrier: "post", address: "Seoul" })
    @respond 200 { payment: captured.status, shipment: booked.status }
  }
}
"#,
    )
    .expect("write env http commerce source");
    let out = temp_output_dir("build-prod-env-http-commerce");

    cmd_build_with_profile(&path, &out, BuildProfile::Production).expect("prod build");

    let deploy = read_json_value(&out.join("deploy").join("manifest.json")).expect("deploy");
    let container = read_json_value(&out.join("deploy").join("container.json")).expect("container");
    let compose =
        std::fs::read_to_string(out.join("deploy").join("compose.yaml")).expect("compose");
    let commerce_adapters_path = out.join("deploy").join("commerce-adapters.json");
    let commerce_adapters = read_json_value(&commerce_adapters_path).expect("commerce adapters");
    let runbook = std::fs::read_to_string(out.join("deploy").join("README.md")).expect("runbook");

    assert_eq!(
        deploy["server"]["commerce_adapters"],
        "deploy/commerce-adapters.json"
    );
    assert_eq!(
        deploy["server"]["persistence"]["commerce_endpoints"],
        serde_json::json!([
            "http://payments.internal/capture",
            "http://shipping.internal/book"
        ])
    );
    assert_eq!(
        deploy["server"]["persistence"]["commerce_env"],
        serde_json::json!([
            {
                "env": "PAYMENT_ADAPTER_URL",
                "default": "http://payments.internal/capture"
            },
            {
                "env": "SHIPPING_ADAPTER_URL",
                "default": "http://shipping.internal/book"
            }
        ])
    );
    assert_eq!(
        container["persistence"]["commerce_env"],
        deploy["server"]["persistence"]["commerce_env"]
    );
    assert!(compose.contains(
        r#"PAYMENT_ADAPTER_URL: "${PAYMENT_ADAPTER_URL:-http://payments.internal/capture}""#
    ));
    assert!(compose.contains(
        r#"SHIPPING_ADAPTER_URL: "${SHIPPING_ADAPTER_URL:-http://shipping.internal/book}""#
    ));
    assert_eq!(commerce_adapters["schema_version"], 1);
    assert_eq!(commerce_adapters["artifact"], "server/app.orv-runtime.json");
    assert_eq!(
        adapter_values_without_source_origin_ids(&commerce_adapters["adapters"]),
        serde_json::json!([
            {
                "kind": "payment",
                "surface": "library_provider_package",
                "package": "orv-commerce",
                "provider_package": null,
                "mode": "http",
                "env": "PAYMENT_ADAPTER_URL",
                "default": "http://payments.internal/capture",
                "endpoint": "http://payments.internal/capture",
                "record_path": null,
                "request": {
                    "method": "POST",
                    "content_type": "application/json",
                    "kind": "payment.capture",
                    "body": {
                        "kind": "payment.capture",
                        "payload": "payment capture payload"
                    }
                }
            },
            {
                "kind": "shipping",
                "surface": "library_provider_package",
                "package": "orv-commerce",
                "provider_package": null,
                "mode": "http",
                "env": "SHIPPING_ADAPTER_URL",
                "default": "http://shipping.internal/book",
                "endpoint": "http://shipping.internal/book",
                "record_path": null,
                "request": {
                    "method": "POST",
                    "content_type": "application/json",
                    "kind": "shipping.booking",
                    "body": {
                        "kind": "shipping.booking",
                        "payload": "shipping booking payload"
                    }
                }
            }
        ])
    );
    assert!(commerce_adapters["adapters"]
        .as_array()
        .expect("commerce adapters")
        .iter()
        .all(|adapter| adapter["source_origin_id"]
            .as_str()
            .is_some_and(|origin_id| origin_id.starts_with("ori_"))));
    assert!(runbook.contains(
        "- Commerce adapter env: PAYMENT_ADAPTER_URL default http://payments.internal/capture"
    ));
    assert!(runbook.contains(
        "- Commerce adapter env: SHIPPING_ADAPTER_URL default http://shipping.internal/book"
    ));
    assert!(runbook.contains("deploy/commerce-adapters.json"));
    cmd_verify_build(&out).expect("verify prod build");
    let _ = std::fs::remove_dir_all(dir);
    let _ = std::fs::remove_dir_all(out);
}

#[test]
fn build_prod_writes_env_listen_container_contract() {
    let (src_dir, path) = env_prod_server_source("build-prod-env-listen-source");
    let out = temp_output_dir("build-prod-env-listen");

    cmd_build_with_profile(&path, &out, BuildProfile::Production).expect("prod build");

    let deploy_manifest_path = out.join("deploy").join("manifest.json");
    let deploy_container_path = out.join("deploy").join("container.json");
    let deploy_dockerfile_path = out.join("deploy").join("Dockerfile");
    let deploy_compose_path = out.join("deploy").join("compose.yaml");
    let deploy_env_example_path = out.join("deploy").join("env.example");
    let deploy = read_json_value(&deploy_manifest_path).expect("deploy manifest");
    let container = read_json_value(&deploy_container_path).expect("deploy container");

    assert_eq!(deploy["server"]["listen"]["port"], serde_json::Value::Null);
    assert_eq!(deploy["server"]["listen"]["env"]["variable"], "PORT");
    assert_eq!(deploy["server"]["listen"]["env"]["default_port"], 8080);
    assert_eq!(container["listen"], deploy["server"]["listen"]);
    assert_eq!(container["ports"][0]["env"], "PORT");
    assert_eq!(container["ports"][0]["default"], 8080);
    assert_eq!(container["ports"][0]["protocol"], "tcp");
    let dockerfile = std::fs::read_to_string(&deploy_dockerfile_path).expect("Dockerfile");
    assert!(dockerfile.contains("EXPOSE 8080"));
    let compose = std::fs::read_to_string(&deploy_compose_path).expect("compose");
    assert!(compose.contains(r#""${PORT:-8080}:8080""#));
    assert!(compose.contains(r#"PORT: "${PORT:-8080}""#));
    let env_example = std::fs::read_to_string(&deploy_env_example_path).expect("env example");
    assert!(env_example.contains("PORT=8080"));

    cmd_verify_build(&out).expect("verify prod build");
    let _ = std::fs::remove_dir_all(src_dir);
    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn build_prod_rejects_test_only_ephemeral_listen_port() {
    let path = workspace_path(&["fixtures", "e2e", "hello.orv"]);
    let out = temp_output_dir("build-prod-ephemeral-listen");

    let err = cmd_build_with_profile(&path, &out, BuildProfile::Production)
        .expect_err("ephemeral prod listen");

    assert!(err
        .to_string()
        .contains("prod server listen port must be 1..=65535"));
    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn build_accepts_orv_toml_project_entry() {
    let dir = temp_output_dir("project-manifest-build");
    let src = dir.join("src");
    std::fs::create_dir_all(&src).expect("create src dir");
    let entry = src.join("main.orv");
    std::fs::write(&entry, "@html { \"Manifest page\" }\n").expect("write entry");
    let manifest = dir.join("orv.toml");
    std::fs::write(
        &manifest,
        r#"[project]
name = "manifest-build"
entry = "src/main.orv"
"#,
    )
    .expect("write manifest");
    let out = dir.join("dist");

    cmd_build(&manifest, &out).expect("manifest build");

    let build_manifest = read_json_value(&out.join("build-manifest.json")).expect("manifest");
    assert_eq!(build_manifest["entry"], entry.display().to_string());
    assert!(
        out.join("pages").join("index.html").is_file(),
        "missing static page"
    );
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn build_writes_static_html_page_for_html_only_entry() {
    let out = temp_output_dir("build-static-page");
    std::fs::create_dir_all(&out).expect("create temp root");
    let entry = out.join("page.orv");
    std::fs::write(
        &entry,
        r#"@out @html { @body { @h1 "Home" @p "zero runtime" } }"#,
    )
    .expect("write entry");
    let build_out = out.join("dist");

    cmd_build(&entry, &build_out).expect("build artifacts");

    let page = build_out.join("pages").join("index.html");
    let html = std::fs::read_to_string(&page).expect("static page");
    assert_eq!(
        html,
        "<html><body><h1>Home</h1><p>zero runtime</p></body></html>"
    );
    let plan: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(build_out.join("bundle-plan.json")).expect("plan"),
    )
    .expect("bundle plan json");
    let static_bundle = plan["bundles"]
        .as_array()
        .expect("bundles array")
        .iter()
        .find(|bundle| bundle["kind"] == "static_page")
        .expect("static page bundle");
    assert_eq!(static_bundle["path"], "pages/index.html");
    assert_eq!(
        static_bundle["runtime_features"]
            .as_array()
            .expect("runtime features")
            .len(),
        0
    );
    assert!(!plan["bundles"]
        .as_array()
        .expect("bundles array")
        .iter()
        .any(|bundle| bundle["kind"] == "server_runtime"));

    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn build_prod_records_static_page_target() {
    let out = temp_output_dir("build-prod-static-page");
    std::fs::create_dir_all(&out).expect("create temp root");
    let entry = out.join("page.orv");
    std::fs::write(&entry, r#"@out @html { @body { @h1 "Home" } }"#).expect("write entry");
    let build_out = out.join("dist");

    cmd_build_with_profile(&entry, &build_out, BuildProfile::Production).expect("build prod");

    let deploy = read_json_value(&build_out.join("deploy").join("manifest.json")).expect("deploy");
    assert_eq!(deploy["static"]["path"], "pages/index.html");
    assert!(deploy["static"]["runtime_features"]
        .as_array()
        .expect("runtime features")
        .is_empty());
    assert_eq!(deploy["client"], serde_json::Value::Null);
    assert_eq!(deploy["server"], serde_json::Value::Null);
    cmd_verify_build(&build_out).expect("verify prod static build");
    let _ = std::fs::remove_dir_all(&out);
}
