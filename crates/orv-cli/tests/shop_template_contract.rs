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

fn assert_contains_all(text: &str, markers: &[&str], context: &str) {
    for marker in markers {
        assert!(text.contains(marker), "{context} missing marker {marker:?}");
    }
}

#[test]
fn shop_template_v1_freezes_scaffold_contract() {
    let root = temp_output_dir("shop-template-contract");
    let shop = root.join("starter-shop");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("temp root");
    let shop_arg = shop.display().to_string();

    run_orv(
        &[
            "init",
            &shop_arg,
            "--name",
            "starter-shop",
            "--template",
            "shop",
        ],
        None,
    );

    assert_manifest_contract(&shop);
    assert_source_contract(&shop);
    assert_readme_contract(&shop);
    run_orv(&["check", "."], Some(&shop));

    let _ = std::fs::remove_dir_all(&root);
}

fn assert_manifest_contract(shop: &Path) {
    let manifest = read_text(&shop.join("orv.toml"));
    assert_contains_all(
        &manifest,
        &[
            "[project]",
            "name = \"starter-shop\"",
            "version = \"0.1.0\"",
            "entry = \"src/main.orv\"",
        ],
        "shop manifest",
    );
}

fn assert_source_contract(shop: &Path) {
    let source = read_text(&shop.join("src").join("main.orv"));
    assert_contains_all(
        &source,
        &[
            "@listen 8080",
            r#"let shopdb = @db.connect(@env.SHOP_DATABASE_URL ?? "sqlite://data/shop.sqlite")"#,
            "@design",
            "@colors",
            "@spacing",
            "@typography",
            "@design.colors.surface",
            "@design.spacing.lg",
            "@design.typography.fontFamily",
            "struct ProductInput",
            "badge: string(trim, min=1)",
            "@input type=text name=badge value=\"New arrival\" required",
            "badge: @body.badge",
            "{product.badge}",
            "@route GET /catalog",
            "@route GET /cart",
            "@route GET /account/sessions",
            "@session required",
            "@route GET /admin",
            "@Auth required role=\"admin\"",
            "@route GET /admin/summary",
            "@route GET /admin/catalog",
            "@route GET /admin/orders",
            "@route GET /admin/payments",
            "@route GET /admin/shipments",
            "@route GET /admin/webhooks",
            "@route GET /admin/audit",
            "@csrf",
            "@input type=hidden name=_csrf value=\"orv-reference-csrf\"",
            "@body: ProductInput",
            "@body: MemberSignupInput",
            "@body: MemberLoginInput",
            "@body: CartItemInput",
            "@body: CheckoutInput",
            "hash.password(@body.password)",
            "hash.verify(@body.password, member.passwordHash)",
            "shopdb.transaction(",
            r#"@payment.connect(@env.PAYMENT_ADAPTER_URL ?? "file://data/payments.jsonl")"#,
            r#"@shipping.connect(@env.SHIPPING_ADAPTER_URL ?? "file://data/shipments.jsonl")"#,
            "payment_captured_pending_shipment",
            "checkout.compensation_required",
            "@route POST /webhooks/stripe",
            r#"@header["stripe-signature"]"#,
            "payments.verifyWebhook",
            "duplicate: true",
            "shopdb.create(\"AuditEvent\"",
        ],
        "shop source",
    );
    assert!(
        source.matches("@Auth required role=\"admin\"").count() >= 8,
        "shop source must keep admin read-model routes protected"
    );
    assert!(
        source.matches("@csrf").count() >= 8,
        "shop source must keep browser mutations csrf-protected"
    );
}

fn assert_readme_contract(shop: &Path) {
    let readme = read_text(&shop.join("README.md"));
    assert_contains_all(
        &readme,
        &[
            "orv check .",
            "orv build . --prod --out dist",
            "orv verify-build dist",
            "orv deploy-env-check dist",
            "orv run-build dist",
            "sh dist/deploy/smoke-test.sh",
            "orv benchmark-report dist",
            "orv benchmark-report dist --require-pass",
            "ProductInput.badge",
            "@Auth required role=\"admin\"",
            "hash.password",
            "hash.verify",
            "@session required",
            "@csrf",
            "deploy/smoke-output.txt",
            "server/native-server.json",
            "server/native/Cargo.toml",
            "GET /admin/audit",
            "POST /webhooks/stripe",
        ],
        "shop README",
    );
}
