use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_dir(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time")
        .as_nanos();
    std::env::temp_dir().join(format!("orv-cli-{name}-{}-{nanos}", std::process::id()))
}

fn orv_bin() -> &'static str {
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

fn index_after(source: &str, start: usize, needle: &str) -> usize {
    source[start..]
        .find(needle)
        .map(|offset| start + offset)
        .unwrap_or_else(|| panic!("missing {needle:?}"))
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

    let source = std::fs::read_to_string(shop.join("src").join("main.orv")).expect("shop source");
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

    let _ = std::fs::remove_dir_all(root);
}
