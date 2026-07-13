use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;

const SHOP_SECURITY_BOUNDARIES_GOLDEN: &str =
    include_str!("../../../docs/samples/shop-security-boundaries-v1.golden.json");

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

fn shop_security_boundaries_golden() -> Value {
    serde_json::from_str(SHOP_SECURITY_BOUNDARIES_GOLDEN).expect("shop security boundaries golden")
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

fn assert_route_policy_surfaces(runtime: &serde_json::Value, preflight: &serde_json::Value) {
    let admin = json_route(&runtime["routes"], "GET", "/admin");
    assert!(policies(admin).iter().any(|policy| {
        policy["kind"] == "auth"
            && policy["surface"] == "first_party_compiler_plugin"
            && policy["role"] == "admin"
            && policy["required"] == true
            && policy["origin_id"]
                .as_str()
                .is_some_and(|origin_id| origin_id.starts_with("ori_"))
    }));

    let account_sessions = json_route(&runtime["routes"], "GET", "/account/sessions");
    assert!(policies(account_sessions).iter().any(|policy| {
        policy["kind"] == "session"
            && policy["surface"] == "first_party_compiler_plugin"
            && policy["required"] == true
            && policy["origin_id"]
                .as_str()
                .is_some_and(|origin_id| origin_id.starts_with("ori_"))
    }));

    let checkout = json_route(&preflight["routes"], "POST", "/checkout");
    assert!(policies(checkout).iter().any(|policy| {
        policy["kind"] == "csrf"
            && policy["surface"] == "first_party_compiler_plugin"
            && policy["required"] == true
            && policy["origin_id"]
                .as_str()
                .is_some_and(|origin_id| origin_id.starts_with("ori_"))
    }));
    assert!(policies(checkout).iter().any(|policy| {
        policy["kind"] == "rate_limit"
            && policy["surface"] == "shop_template"
            && policy["limit"] == 10
            && policy["window_seconds"] == 60
    }));
    let checkout_rate_limit = policies(checkout)
        .iter()
        .find(|policy| policy["kind"] == "rate_limit")
        .expect("checkout rate_limit policy");
    assert_ne!(
        checkout_rate_limit["surface"], "first_party_compiler_plugin",
        "checkout rate limit must stay a shop template policy"
    );
    assert_ne!(checkout_rate_limit["surface"], "core_intrinsic");

    let webhook = json_route(&preflight["routes"], "POST", "/webhooks/stripe");
    assert!(policies(webhook).iter().any(|policy| {
        policy["kind"] == "rate_limit"
            && policy["surface"] == "provider_package_template"
            && policy["limit"] == 60
            && policy["window_seconds"] == 60
    }));
    let webhook_rate_limit = policies(webhook)
        .iter()
        .find(|policy| policy["kind"] == "rate_limit")
        .expect("webhook rate_limit policy");
    assert_ne!(
        webhook_rate_limit["surface"], "first_party_compiler_plugin",
        "webhook rate limit must stay a provider package template policy"
    );
    assert_ne!(webhook_rate_limit["surface"], "core_intrinsic");
}

fn assert_prod_security_artifacts(shop: &Path, source: &str) {
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

    assert_route_policy_surfaces(&runtime, &preflight);

    for marker in [
        "kind: \"auth\"",
        "role: Some(\"admin\")",
        "kind: \"session\"",
        "kind: \"csrf\"",
        "kind: \"rate_limit\"",
        "surface: Some(\"first_party_compiler_plugin\")",
        "surface: Some(\"shop_template\")",
        "surface: Some(\"provider_package_template\")",
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

    assert_eq!(
        shop_security_inventory(
            source,
            &manifest,
            &runtime,
            &deploy,
            &preflight,
            &native_routes,
            &smoke_test,
        ),
        shop_security_boundaries_golden(),
        "Shop Security Boundaries v1 golden drift"
    );
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

    assert_prod_security_artifacts(&shop, &source);

    let _ = std::fs::remove_dir_all(root);
}

fn shop_security_inventory(
    source: &str,
    manifest: &Value,
    runtime: &Value,
    deploy: &Value,
    preflight: &Value,
    native_routes: &str,
    smoke_test: &str,
) -> Value {
    let checkout_route = index_after(source, 0, "@route POST /checkout");
    let checkout_transaction = index_after(source, checkout_route, "shopdb.transaction(");
    let checkout_stock_guard = index_after(source, checkout_route, "if product.stock < quantity");
    let checkout_stock_update = index_after(source, checkout_route, r#"shopdb.update("Product""#);
    let checkout_order_create = index_after(source, checkout_route, r#"shopdb.create("Order""#);
    let checkout_payment_connect = index_after(source, checkout_route, "@payment.connect");
    let checkout_shipping_connect = index_after(source, checkout_route, "@shipping.connect");
    let checkout_shipping_catch = index_after(source, checkout_route, "catch shipmentErr");
    let checkout_pending_status =
        index_after(source, checkout_route, "payment_captured_pending_shipment");
    let checkout_compensation_audit = index_after(
        source,
        checkout_route,
        r#"kind: "checkout.compensation_required""#,
    );
    let checkout_complete_audit =
        index_after(source, checkout_route, r#"kind: "checkout.complete""#);
    let webhook_route = index_after(source, 0, "@route POST /webhooks/stripe");
    let webhook_verify = index_after(source, webhook_route, "payments.verifyWebhook");
    let webhook_existing_lookup =
        index_after(source, webhook_route, r#"shopdb.find("WebhookEvent""#);
    let webhook_duplicate_audit =
        index_after(source, webhook_route, r#"kind: "webhook.duplicate""#);
    let webhook_create = index_after(
        source,
        webhook_route,
        r#"let event = await shopdb.create("WebhookEvent""#,
    );
    let webhook_received_audit = index_after(source, webhook_route, r#"kind: "webhook.received""#);
    let admin = json_route(&runtime["routes"], "GET", "/admin");
    let account_sessions = json_route(&runtime["routes"], "GET", "/account/sessions");
    let checkout = json_route(&preflight["routes"], "POST", "/checkout");
    let webhook = json_route(&preflight["routes"], "POST", "/webhooks/stripe");
    serde_json::json!({
        "schema_version": 1,
        "kind": "orv.shop_security_boundaries.inventory",
        "source": {
            "session_required": source.contains("@session required"),
            "admin_auth_count": source.matches("@Auth required role=\"admin\"").count(),
            "csrf_count": source.matches("@csrf").count(),
            "login_route": source.contains("@route POST /members/login"),
            "checkout_route": source.contains("@route POST /checkout"),
            "webhook_route": source.contains("@route POST /webhooks/stripe"),
            "csrf_hidden_input": source.contains("@input type=hidden name=_csrf value=\"orv-reference-csrf\""),
            "checkout_ordering": {
                "stock_guard_before_stock_update": checkout_stock_guard < checkout_stock_update,
                "stock_guard_before_transaction": checkout_stock_guard < checkout_transaction,
                "transaction_before_stock_update": checkout_transaction < checkout_stock_update,
                "stock_update_before_order_create": checkout_stock_update < checkout_order_create,
                "order_create_before_payment": checkout_order_create < checkout_payment_connect,
                "payment_before_shipping": checkout_payment_connect < checkout_shipping_connect,
                "shipping_before_catch": checkout_shipping_connect < checkout_shipping_catch,
                "catch_before_pending_status": checkout_shipping_catch < checkout_pending_status,
                "pending_before_compensation_audit": checkout_pending_status < checkout_compensation_audit,
                "shipping_before_complete_audit": checkout_shipping_connect < checkout_complete_audit,
            },
            "webhook_ordering": {
                "verify_before_lookup": webhook_verify < webhook_existing_lookup,
                "lookup_before_duplicate_audit": webhook_existing_lookup < webhook_duplicate_audit,
                "duplicate_audit_before_create": webhook_duplicate_audit < webhook_create,
                "create_before_received_audit": webhook_create < webhook_received_audit,
            },
        },
        "runtime_features": {
            "build_manifest": runtime_feature_inventory(&manifest["capabilities"]["runtime_features"]),
            "runtime": runtime_feature_inventory(&runtime["runtime_features"]),
            "deploy": runtime_feature_inventory(&deploy["server"]["runtime_features"]),
            "preflight": runtime_feature_inventory(&preflight["runtime_features"]),
        },
        "policies": {
            "admin_auth": policy_inventory(admin, "auth"),
            "account_session": policy_inventory(account_sessions, "session"),
            "checkout_csrf": policy_inventory(checkout, "csrf"),
            "checkout_rate_limit": rate_limit_inventory(checkout),
            "webhook_rate_limit": rate_limit_inventory(webhook),
        },
        "native_routes": marker_inventory(native_routes, &[
            "kind: \"auth\"",
            "role: Some(\"admin\")",
            "kind: \"session\"",
        "kind: \"csrf\"",
        "kind: \"rate_limit\"",
        "surface: Some(\"first_party_compiler_plugin\")",
        "surface: Some(\"shop_template\")",
        "surface: Some(\"provider_package_template\")",
        "limit: Some(10)",
            "limit: Some(60)",
            "window_seconds: Some(60)",
        ]),
        "smoke": marker_inventory(smoke_test, &[
            "CSRF_COOKIE",
            "x-csrf-token",
            "ADMIN_SESSION_COOKIE",
            "ADMIN_ROLE_COOKIE",
            "POST /checkout",
            "GET /admin/webhooks content",
            "checkout captured payment",
            "admin audit checkout",
        ]),
    })
}

fn runtime_feature_inventory(features: &Value) -> Vec<Value> {
    [
        "auth_roles",
        "session_cookies",
        "csrf_protection",
        "rate_limit",
    ]
    .iter()
    .map(|feature| {
        serde_json::json!({
            "feature": feature,
            "present": has_runtime_feature(features, feature),
        })
    })
    .collect()
}

fn policy_inventory(route: &Value, kind: &str) -> Value {
    let policy = policies(route)
        .iter()
        .find(|policy| policy["kind"] == kind)
        .unwrap_or_else(|| panic!("missing {kind} policy"));
    serde_json::json!({
        "present": true,
        "surface": policy["surface"].clone(),
        "required": policy["required"].clone(),
        "role": policy.get("role").cloned().unwrap_or(Value::Null),
        "origin_present": policy["origin_id"]
            .as_str()
            .is_some_and(|origin_id| origin_id.starts_with("ori_")),
    })
}

fn rate_limit_inventory(route: &Value) -> Value {
    let policy = policies(route)
        .iter()
        .find(|policy| policy["kind"] == "rate_limit")
        .expect("missing rate_limit policy");
    serde_json::json!({
        "present": true,
        "surface": policy["surface"].clone(),
        "limit": policy["limit"].clone(),
        "window_seconds": policy["window_seconds"].clone(),
    })
}

fn marker_inventory(text: &str, markers: &[&str]) -> Vec<Value> {
    markers
        .iter()
        .map(|marker| {
            serde_json::json!({
                "marker": marker,
                "present": text.contains(marker),
            })
        })
        .collect()
}
