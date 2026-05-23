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

fn run_orv(args: &[&str], cwd: Option<&Path>) {
    let mut command = Command::new(orv_bin());
    command.args(args);
    if let Some(cwd) = cwd {
        command.current_dir(cwd);
    }
    let output = command.output().expect("run orv");
    assert!(
        output.status.success(),
        "orv {args:?} failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn read_text(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|err| panic!("read {}: {err}", path.display()))
}

fn read_json(path: &Path) -> Value {
    let text = read_text(path);
    serde_json::from_str(&text).unwrap_or_else(|err| panic!("parse {}: {err}", path.display()))
}

fn json_route<'a>(routes: &'a Value, method: &str, path: &str) -> &'a Value {
    routes
        .as_array()
        .expect("routes array")
        .iter()
        .find(|route| route["method"] == method && route["path"] == path)
        .unwrap_or_else(|| panic!("missing route {method} {path}"))
}

fn policies(route: &Value) -> &[Value] {
    route["policies"].as_array().expect("route policies")
}

fn has_runtime_feature(value: &Value, feature: &str) -> bool {
    value
        .as_array()
        .expect("runtime_features array")
        .iter()
        .any(|item| item == feature)
}

fn index_after(source: &str, start: usize, needle: &str) -> usize {
    let Some(offset) = source[start..].find(needle) else {
        panic!("missing {needle:?}");
    };
    start + offset
}

fn assert_source_security_markers(source: &str) {
    assert!(source.contains("@session required"));
    assert!(
        source.matches("@Auth required role=\"admin\"").count() >= 8,
        "shop source must keep admin read-model routes protected"
    );
    assert!(source.contains("@route POST /members/login"));
    assert!(source.contains("@route POST /checkout"));
    assert!(source.contains("@route POST /webhooks/stripe"));
    assert!(source.contains("@input type=hidden name=_csrf value=\"orv-reference-csrf\""));
    assert!(
        source.matches("@csrf").count() >= 8,
        "shop source must keep browser mutations csrf-protected"
    );
}

fn assert_prod_security_artifacts(shop: &Path) {
    run_orv(&["build", ".", "--prod", "--out", "dist"], Some(shop));

    let out = shop.join("dist");
    let manifest = read_json(&out.join("build-manifest.json"));
    let runtime = read_json(&out.join("server").join("app.orv-runtime.json"));
    let deploy = read_json(&out.join("deploy").join("manifest.json"));
    let preflight = read_json(&out.join("deploy").join("preflight.json"));
    let smoke_test = read_text(&out.join("deploy").join("smoke-test.sh"));
    let native_routes = read_text(&out.join("server").join("native").join("routes.rs"));

    for feature in [
        "auth_roles",
        "session_cookies",
        "csrf_protection",
        "rate_limit",
    ] {
        assert!(has_runtime_feature(
            &manifest["capabilities"]["runtime_features"],
            feature
        ));
        assert!(has_runtime_feature(&runtime["runtime_features"], feature));
        assert!(has_runtime_feature(
            &deploy["server"]["runtime_features"],
            feature
        ));
        assert!(has_runtime_feature(&preflight["runtime_features"], feature));
    }

    let admin = json_route(&runtime["routes"], "GET", "/admin");
    assert!(policies(admin).iter().any(|policy| {
        policy["kind"] == "auth"
            && policy["role"] == "admin"
            && policy["required"] == true
            && policy["origin_id"]
                .as_str()
                .is_some_and(|origin_id| origin_id.starts_with("ori_"))
    }));

    let account_sessions = json_route(&runtime["routes"], "GET", "/account/sessions");
    assert!(policies(account_sessions).iter().any(|policy| {
        policy["kind"] == "session"
            && policy["required"] == true
            && policy["origin_id"]
                .as_str()
                .is_some_and(|origin_id| origin_id.starts_with("ori_"))
    }));

    let checkout = json_route(&preflight["routes"], "POST", "/checkout");
    assert!(policies(checkout).iter().any(|policy| {
        policy["kind"] == "csrf"
            && policy["required"] == true
            && policy["origin_id"]
                .as_str()
                .is_some_and(|origin_id| origin_id.starts_with("ori_"))
    }));
    assert!(policies(checkout).iter().any(|policy| {
        policy["kind"] == "rate_limit" && policy["limit"] == 10 && policy["window_seconds"] == 60
    }));

    let webhook = json_route(&preflight["routes"], "POST", "/webhooks/stripe");
    assert!(policies(webhook).iter().any(|policy| {
        policy["kind"] == "rate_limit" && policy["limit"] == 60 && policy["window_seconds"] == 60
    }));

    for marker in [
        "kind: \"auth\"",
        "role: Some(\"admin\")",
        "kind: \"session\"",
        "kind: \"csrf\"",
        "kind: \"rate_limit\"",
        "limit: Some(10)",
        "limit: Some(60)",
        "window_seconds: Some(60)",
    ] {
        assert!(
            native_routes.contains(marker),
            "native route table missing marker {marker:?}"
        );
    }

    for marker in [
        "CSRF_COOKIE",
        "x-csrf-token",
        "ADMIN_SESSION_COOKIE",
        "ADMIN_ROLE_COOKIE",
        "POST /checkout",
        "GET /admin/webhooks content",
        "checkout captured payment",
        "admin audit checkout",
    ] {
        assert!(
            smoke_test.contains(marker),
            "generated smoke script missing marker {marker:?}"
        );
    }
}

#[test]
fn shop_template_keeps_checkout_and_webhook_side_effect_boundaries_ordered() {
    let root = temp_dir("shop-security-boundary");
    let shop = root.join("shop");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("create temp root");
    let shop_arg = shop.display().to_string();

    run_orv(&["init", &shop_arg, "--template", "shop"], None);
    run_orv(&["check", "."], Some(&shop));

    let source = read_text(&shop.join("src").join("main.orv"));
    assert_source_security_markers(&source);

    let checkout_route = index_after(&source, 0, "@route POST /checkout");
    let checkout_transaction = index_after(&source, checkout_route, "shopdb.transaction(");
    let checkout_stock_guard = index_after(&source, checkout_route, "if product.stock < quantity");
    let checkout_stock_update = index_after(&source, checkout_route, r#"shopdb.update("Product""#);
    let checkout_order_create = index_after(&source, checkout_route, r#"shopdb.create("Order""#);
    let checkout_payment_connect = index_after(&source, checkout_route, "@payment.connect");
    let checkout_shipping_connect = index_after(&source, checkout_route, "@shipping.connect");
    let checkout_shipping_catch = index_after(&source, checkout_route, "catch shipmentErr");
    let checkout_pending_status =
        index_after(&source, checkout_route, "payment_captured_pending_shipment");
    let checkout_compensation_audit = index_after(
        &source,
        checkout_route,
        r#"kind: "checkout.compensation_required""#,
    );
    let checkout_complete_audit =
        index_after(&source, checkout_route, r#"kind: "checkout.complete""#);

    assert!(checkout_stock_guard < checkout_stock_update);
    assert!(checkout_stock_guard < checkout_transaction);
    assert!(checkout_transaction < checkout_stock_update);
    assert!(checkout_stock_update < checkout_order_create);
    assert!(checkout_order_create < checkout_payment_connect);
    assert!(checkout_payment_connect < checkout_shipping_connect);
    assert!(checkout_shipping_connect < checkout_shipping_catch);
    assert!(checkout_shipping_catch < checkout_pending_status);
    assert!(checkout_pending_status < checkout_compensation_audit);
    assert!(checkout_shipping_connect < checkout_complete_audit);

    let webhook_route = index_after(&source, 0, "@route POST /webhooks/stripe");
    let webhook_verify = index_after(&source, webhook_route, "payments.verifyWebhook");
    let webhook_existing_lookup =
        index_after(&source, webhook_route, r#"shopdb.find("WebhookEvent""#);
    let webhook_duplicate_audit =
        index_after(&source, webhook_route, r#"kind: "webhook.duplicate""#);
    let webhook_create = index_after(
        &source,
        webhook_route,
        r#"let event = await shopdb.create("WebhookEvent""#,
    );
    let webhook_received_audit = index_after(&source, webhook_route, r#"kind: "webhook.received""#);

    assert!(webhook_verify < webhook_existing_lookup);
    assert!(webhook_existing_lookup < webhook_duplicate_audit);
    assert!(webhook_duplicate_audit < webhook_create);
    assert!(webhook_create < webhook_received_audit);

    assert_prod_security_artifacts(&shop);

    let _ = std::fs::remove_dir_all(root);
}
