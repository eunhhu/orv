use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;

fn temp_dir(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time")
        .as_nanos();
    std::env::temp_dir().join(format!("orv-cli-{name}-{}-{nanos}", std::process::id()))
}

const fn orv_bin() -> &'static str {
    env!("CARGO_BIN_EXE_orv")
}

fn run_orv(args: &[&str]) {
    let output = Command::new(orv_bin())
        .args(args)
        .output()
        .expect("run orv");
    assert!(
        output.status.success(),
        "orv {args:?} failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn run_orv_json(args: &[&str]) -> Value {
    let output = Command::new(orv_bin())
        .args(args)
        .output()
        .expect("run orv");
    assert!(
        output.status.success(),
        "orv {args:?} failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("orv json")
}

fn read_json(path: &Path) -> Value {
    serde_json::from_str(&std::fs::read_to_string(path).expect("read json")).expect("json")
}

fn origin_id(origin_map: &Value, kind: &str, name: &str) -> String {
    origin_map["entries"]
        .as_array()
        .expect("origin entries")
        .iter()
        .find(|entry| entry["kind"] == kind && entry["name"] == name)
        .unwrap_or_else(|| panic!("missing origin {kind}:{name}"))
        .get("id")
        .and_then(Value::as_str)
        .expect("origin id")
        .to_string()
}

fn route<'a>(routes: &'a Value, method: &str, path: &str) -> &'a Value {
    routes
        .as_array()
        .expect("routes array")
        .iter()
        .find(|route| route["method"] == method && route["path"] == path)
        .unwrap_or_else(|| panic!("missing route {method} {path}"))
}

fn policy<'a>(route: &'a Value, kind: &str) -> &'a Value {
    route["policies"]
        .as_array()
        .expect("route policies")
        .iter()
        .find(|policy| policy["kind"] == kind)
        .unwrap_or_else(|| panic!("missing policy {kind} on route {route:?}"))
}

fn assert_policy_surface(route: &Value, kind: &str, surface: &str) {
    assert_eq!(policy(route, kind)["surface"], surface, "{kind} surface");
}

fn assert_not_promoted_policy(route: &Value, kind: &str) {
    let surface = &policy(route, kind)["surface"];
    assert_ne!(surface, "first_party_compiler_plugin", "{kind} promoted");
    assert_ne!(surface, "core_intrinsic", "{kind} promoted");
    assert_ne!(surface, "compiler_core", "{kind} promoted");
}

fn matched_commerce_adapter(reveal: &Value) -> &Value {
    &reveal["production"]["commerce_adapters"]
        .as_array()
        .expect("commerce targets")
        .iter()
        .find(|target| target["matched"] == true)
        .expect("matched commerce target")["matched_adapters"][0]
}

#[test]
fn consumer_artifacts_keep_plugin_template_and_provider_surfaces_separate() {
    let root = temp_dir("consumer-artifact-boundary");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("create temp dir");
    let source = root.join("app.orv");
    let out = root.join("dist");
    std::fs::write(
        &source,
        r#"@server {
  @listen 8080
  let payments = @payment.connect(@env.PAYMENT_ADAPTER_URL ?? "http://payments.internal/capture")
  @route GET /admin {
    @session required
    @Auth required role="admin"
    @respond 200 { status: "admin" }
  }
  @route POST /checkout {
    @session required
    @csrf
    let captured = payments.capture({ orderId: "o_1", amount: 42, method: "card" })
    @respond 200 { payment: captured.status }
  }
  @route POST /webhooks/stripe {
    @respond 200 { status: "ok" }
  }
}
"#,
    )
    .expect("write source");

    let source_arg = source.display().to_string();
    let out_arg = out.display().to_string();
    run_orv(&["build", &source_arg, "--prod", "--out", &out_arg]);

    let origin_map = read_json(&out.join("origin-map.json"));
    let checkout_id = origin_id(&origin_map, "route", "POST /checkout");
    let webhook_id = origin_id(&origin_map, "route", "POST /webhooks/stripe");
    let payment_id = origin_id(&origin_map, "call", "@payment.connect");
    let runtime = read_json(&out.join("server/app.orv-runtime.json"));
    let preflight = read_json(&out.join("deploy/preflight.json"));
    let native_routes =
        std::fs::read_to_string(out.join("server/native/routes.rs")).expect("native routes");
    let checkout_reveal = run_orv_json(&["reveal", &out_arg, &checkout_id]);
    let webhook_reveal = run_orv_json(&["reveal", &out_arg, &webhook_id]);
    let payment_reveal = run_orv_json(&["editor", "reveal", &out_arg, &payment_id]);

    let admin_runtime = route(&runtime["routes"], "GET", "/admin");
    assert_policy_surface(admin_runtime, "session", "first_party_compiler_plugin");
    assert_policy_surface(admin_runtime, "auth", "first_party_compiler_plugin");

    for routes in [
        &runtime["routes"],
        &preflight["routes"],
        &checkout_reveal["production"]["routes"],
    ] {
        let checkout = route(routes, "POST", "/checkout");
        assert_policy_surface(checkout, "session", "first_party_compiler_plugin");
        assert_policy_surface(checkout, "csrf", "first_party_compiler_plugin");
        assert_policy_surface(checkout, "rate_limit", "shop_template");
        assert_not_promoted_policy(checkout, "rate_limit");
    }

    for routes in [
        &runtime["routes"],
        &preflight["routes"],
        &webhook_reveal["production"]["routes"],
    ] {
        let webhook = route(routes, "POST", "/webhooks/stripe");
        assert_policy_surface(webhook, "rate_limit", "provider_package_template");
        assert_not_promoted_policy(webhook, "rate_limit");
    }

    let commerce_adapter = matched_commerce_adapter(&payment_reveal);
    assert_eq!(commerce_adapter["surface"], "library_provider_package");
    assert_eq!(commerce_adapter["package"], "orv-commerce");
    assert_ne!(commerce_adapter["surface"], "first_party_compiler_plugin");
    assert_ne!(commerce_adapter["surface"], "core_intrinsic");

    for marker in [
        "surface: Some(\"first_party_compiler_plugin\")",
        "surface: Some(\"shop_template\")",
        "surface: Some(\"provider_package_template\")",
    ] {
        assert!(
            native_routes.contains(marker),
            "missing native marker {marker}"
        );
    }
    assert!(!native_routes.contains("surface: Some(\"core_intrinsic\")"));
    assert!(!native_routes.contains("surface: Some(\"compiler_core\")"));

    let _ = std::fs::remove_dir_all(root);
}
