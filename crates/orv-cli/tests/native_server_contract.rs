use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

fn temp_output_dir(name: &str) -> PathBuf {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    std::env::temp_dir().join(format!("orv-{name}-{}-{nonce}", std::process::id()))
}

const fn orv_bin() -> &'static str {
    env!("CARGO_BIN_EXE_orv")
}

fn run_orv(args: &[&str]) {
    let status = Command::new(orv_bin())
        .args(args)
        .status()
        .expect("run orv");
    assert!(status.success(), "orv {args:?} failed with {status}");
}

fn read_json(path: &Path) -> serde_json::Value {
    serde_json::from_str(&std::fs::read_to_string(path).expect("read json")).expect("json")
}

fn assert_keys(value: &serde_json::Value, expected: &[&str], context: &str) {
    let object = value
        .as_object()
        .unwrap_or_else(|| panic!("{context} must be an object"));
    let actual = object.keys().map(String::as_str).collect::<BTreeSet<_>>();
    let expected = expected.iter().copied().collect::<BTreeSet<_>>();
    assert_eq!(actual, expected, "{context} keys drifted");
}

fn assert_string_array(value: &serde_json::Value, expected: &[&str], context: &str) {
    let actual = value
        .as_array()
        .unwrap_or_else(|| panic!("{context} must be an array"))
        .iter()
        .map(|item| {
            item.as_str()
                .unwrap_or_else(|| panic!("{context} item must be a string"))
        })
        .collect::<Vec<_>>();
    assert_eq!(actual, expected, "{context} drifted");
}

fn write_server_fixture(out: &Path) -> PathBuf {
    let fixture = out.join("app.orv");
    std::fs::write(
        &fixture,
        r#"@server {
  @listen 8080

  @route GET /ping {
    @respond 200 { ok: true, msg: "pong" }
  }
}
"#,
    )
    .expect("write fixture");
    fixture
}

#[test]
fn native_server_plan_and_runtime_image_contract_freezes_public_shape() {
    let (root, build_out) = build_contract_fixture();

    let native_plan = assert_native_plan_contract(&build_out);
    assert_runtime_image_contract(&build_out, &native_plan);
    assert_generated_source_contract(&build_out);

    let _ = std::fs::remove_dir_all(&root);
}

fn build_contract_fixture() -> (PathBuf, PathBuf) {
    let root = temp_output_dir("native-server-contract");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("temp root");
    let fixture = write_server_fixture(&root);
    let build_out = root.join("dist");
    let fixture_arg = fixture.display().to_string();
    let build_out_arg = build_out.display().to_string();

    run_orv(&["build", &fixture_arg, "--out", &build_out_arg, "--prod"]);
    run_orv(&["verify-build", &build_out_arg]);

    (root, build_out)
}

fn assert_native_plan_contract(build_out: &Path) -> serde_json::Value {
    let native_plan = read_json(&build_out.join("server").join("native-server.json"));
    assert_native_plan_root(&native_plan);
    assert_native_target(&native_plan);
    assert_native_commands(&native_plan);
    assert_native_route(&native_plan);
    native_plan
}

fn assert_native_plan_root(native_plan: &serde_json::Value) {
    assert_keys(
        native_plan,
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
    );
    assert_eq!(native_plan["schema_version"], serde_json::json!(1));
    assert_eq!(native_plan["kind"], serde_json::json!("native_server_plan"));
    assert_eq!(native_plan["status"], serde_json::json!("direct_http"));
    assert_eq!(
        native_plan["artifact"],
        serde_json::json!("server/app.orv-runtime.json")
    );
    assert_eq!(
        native_plan["launcher"],
        serde_json::json!("server/launch.json")
    );
    assert_eq!(
        native_plan["source"],
        serde_json::json!("server/native/main.rs")
    );
    assert_eq!(
        native_plan["routes_source"],
        serde_json::json!("server/native/routes.rs")
    );
    assert_eq!(
        native_plan["router_source"],
        serde_json::json!("server/native/router.rs")
    );
    assert_eq!(
        native_plan["handlers_source"],
        serde_json::json!("server/native/handlers.rs")
    );
    assert_eq!(
        native_plan["package"],
        serde_json::json!("server/native/Cargo.toml")
    );
    assert_eq!(
        native_plan["runtime_image_plan"],
        serde_json::json!("server/runtime-image.json")
    );
    assert!(native_plan["runtime_features"].is_array());
    assert!(native_plan["blocked_by"]
        .as_array()
        .expect("blocked_by")
        .is_empty());
}

fn assert_native_target(native_plan: &serde_json::Value) {
    assert_keys(
        &native_plan["target"],
        &["kind", "path", "protocol"],
        "native server target",
    );
    assert_eq!(
        native_plan["target"]["kind"],
        serde_json::json!("server_binary")
    );
    assert_eq!(
        native_plan["target"]["path"],
        serde_json::json!("server/app")
    );
    assert_eq!(
        native_plan["target"]["protocol"],
        serde_json::json!("http1")
    );
}

fn assert_native_commands(native_plan: &serde_json::Value) {
    assert_keys(
        &native_plan["commands"],
        &["build", "run"],
        "native server commands",
    );
    assert_string_array(
        &native_plan["commands"]["build"],
        &[
            "cargo",
            "build",
            "--manifest-path",
            "server/native/Cargo.toml",
            "--release",
        ],
        "native server build command",
    );
    assert_keys(
        &native_plan["commands"]["run"],
        &["env", "command"],
        "native server run command",
    );
    assert_keys(
        &native_plan["commands"]["run"]["env"],
        &["ORV_BUILD_DIR"],
        "native server run env",
    );
    assert_eq!(
        native_plan["commands"]["run"]["env"]["ORV_BUILD_DIR"],
        serde_json::json!(".")
    );
    assert_string_array(
        &native_plan["commands"]["run"]["command"],
        &["./server/native/target/release/orv-native-server"],
        "native server run argv",
    );
}

fn assert_native_route(native_plan: &serde_json::Value) {
    assert_keys(
        &native_plan["listen"],
        &["origin_id", "name", "port"],
        "native server listen",
    );
    assert_eq!(native_plan["listen"]["port"], serde_json::json!(8080));
    assert!(native_plan["listen"]["origin_id"]
        .as_str()
        .expect("listen origin id")
        .starts_with("ori_"));

    let route = native_plan["routes"]
        .as_array()
        .expect("native routes")
        .first()
        .expect("native route");
    assert_keys(
        route,
        &[
            "method",
            "path",
            "origin_id",
            "response_origin_ids",
            "responses",
        ],
        "native route",
    );
    assert_eq!(route["method"], serde_json::json!("GET"));
    assert_eq!(route["path"], serde_json::json!("/ping"));
    assert!(route["origin_id"]
        .as_str()
        .expect("route origin id")
        .starts_with("ori_"));
    assert!(route["response_origin_ids"]
        .as_array()
        .expect("response origin ids")
        .first()
        .expect("response origin id")
        .as_str()
        .expect("response origin string")
        .starts_with("ori_"));
    let response = route["responses"]
        .as_array()
        .expect("route responses")
        .first()
        .expect("route response");
    assert_keys(
        response,
        &["origin_id", "status", "body_kind", "body_json"],
        "native route response",
    );
    assert_eq!(response["status"], serde_json::json!(200));
    assert_eq!(response["body_kind"], serde_json::json!("static_json"));
}

fn assert_runtime_image_contract(build_out: &Path, native_plan: &serde_json::Value) {
    let image_plan = read_json(&build_out.join("server").join("runtime-image.json"));
    assert_runtime_image_root(&image_plan, native_plan);
    assert_runtime_image_target(&image_plan);
    assert_runtime_image_commands(&image_plan);
}

fn assert_runtime_image_root(image_plan: &serde_json::Value, native_plan: &serde_json::Value) {
    assert_keys(
        image_plan,
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
    );
    assert_eq!(image_plan["schema_version"], serde_json::json!(1));
    assert_eq!(
        image_plan["kind"],
        serde_json::json!("native_runtime_image_plan")
    );
    assert_eq!(image_plan["status"], serde_json::json!("image_planned"));
    assert_eq!(
        image_plan["artifact"],
        serde_json::json!("server/app.orv-runtime.json")
    );
    assert_eq!(
        image_plan["native_plan"],
        serde_json::json!("server/native-server.json")
    );
    assert_eq!(
        image_plan["reference_image"],
        serde_json::json!("ghcr.io/orv-lang/orv-reference:latest")
    );
    assert_eq!(
        image_plan["dockerfile"],
        serde_json::json!("server/native/Dockerfile")
    );
    assert_eq!(image_plan["listen"], native_plan["listen"]);
    assert_eq!(image_plan["routes"], native_plan["routes"]);
    assert!(image_plan["blocked_by"]
        .as_array()
        .expect("image blocked_by")
        .is_empty());
}

fn assert_runtime_image_target(image_plan: &serde_json::Value) {
    assert_keys(
        &image_plan["target"],
        &["kind", "image", "binary", "protocol"],
        "native runtime image target",
    );
    assert_eq!(image_plan["target"]["kind"], serde_json::json!("oci_image"));
    assert_eq!(
        image_plan["target"]["image"],
        serde_json::json!("orv-native-server:latest")
    );
    assert_eq!(
        image_plan["target"]["binary"],
        serde_json::json!("server/app")
    );
    assert_eq!(image_plan["target"]["protocol"], serde_json::json!("http1"));
}

fn assert_runtime_image_commands(image_plan: &serde_json::Value) {
    assert_keys(
        &image_plan["commands"],
        &["build"],
        "native runtime image commands",
    );
    assert_string_array(
        &image_plan["commands"]["build"],
        &[
            "docker",
            "build",
            "-f",
            "server/native/Dockerfile",
            "-t",
            "orv-native-server:latest",
            ".",
        ],
        "native runtime image build command",
    );
}

fn assert_generated_source_contract(build_out: &Path) {
    let launcher =
        std::fs::read_to_string(build_out.join("server/native/main.rs")).expect("launcher source");
    for marker in [
        "const ORV_SERVER_ARTIFACT",
        "const ORV_NATIVE_SERVER_PLAN",
        "fn orv_build_dir() -> std::path::PathBuf",
        "mod routes;",
        "mod router;",
        "mod handlers;",
        "router::orv_native_dispatch_with_request(",
    ] {
        assert!(
            launcher.contains(marker),
            "launcher source missing {marker}"
        );
    }

    let route_table_source =
        std::fs::read_to_string(build_out.join("server/native/routes.rs")).expect("routes source");
    for marker in [
        "pub struct OrvNativeRoute",
        "pub struct OrvNativeRoutePolicy",
        "pub const ORV_NATIVE_ROUTES",
        "pub fn orv_native_match_route(",
        "pub const ORV_NATIVE_ROUTE_COUNT",
        "method: \"GET\"",
        "path: \"/ping\"",
    ] {
        assert!(
            route_table_source.contains(marker),
            "routes source missing {marker}"
        );
    }

    let dispatch_source =
        std::fs::read_to_string(build_out.join("server/native/router.rs")).expect("router source");
    for marker in [
        "use crate::{handlers, routes};",
        "pub struct OrvNativeDispatch",
        "pub const ORV_NATIVE_HANDLER_COUNT",
        "routes::orv_native_match_route(method, path)",
        "handlers::orv_native_handle_route(&route_match)",
        "status: 404",
    ] {
        assert!(
            dispatch_source.contains(marker),
            "router source missing {marker}"
        );
    }

    let handlers_source = std::fs::read_to_string(build_out.join("server/native/handlers.rs"))
        .expect("handlers source");
    for marker in [
        "use crate::routes;",
        "pub struct OrvNativeHandlerDescriptor",
        "pub struct OrvNativeHandlerResponse",
        "pub const ORV_NATIVE_HANDLERS",
        "pub const ORV_NATIVE_HANDLER_COUNT",
        "pub fn orv_native_handle_route(",
        r#"body: "{\"ok\":true,\"msg\":\"pong\"}""#,
    ] {
        assert!(
            handlers_source.contains(marker),
            "handlers source missing {marker}"
        );
    }

    let package =
        std::fs::read_to_string(build_out.join("server/native/Cargo.toml")).expect("package");
    assert!(package.contains("name = \"orv-native-server\""));
    assert!(package.contains("path = \"main.rs\""));
    let dockerfile =
        std::fs::read_to_string(build_out.join("server/native/Dockerfile")).expect("Dockerfile");
    assert!(dockerfile.contains("FROM rust:"));
    assert!(
        dockerfile.contains("cargo build --manifest-path /work/server/native/Cargo.toml --release")
    );
    assert!(dockerfile.contains("ENTRYPOINT [\"/app/server/app\"]"));
}
