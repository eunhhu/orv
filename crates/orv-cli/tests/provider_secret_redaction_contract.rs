use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

const SECRET_VALUES: [&str; 7] = [
    "sk_live_orv_secret_should_not_leak",
    "whsec_orv_secret_should_not_leak",
    "whsec_prev_orv_secret_should_not_leak",
    "carrier_key_orv_secret_should_not_leak",
    "carrier_webhook_orv_secret_should_not_leak",
    "postgres_bridge_auth_should_not_leak",
    "generic_db_bridge_auth_should_not_leak",
];

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

fn assert_success(output: &Output, context: &str) {
    assert!(
        output.status.success(),
        "{context} failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn assert_no_secret_values(label: &str, text: &str) {
    for secret in SECRET_VALUES {
        assert!(
            !text.contains(secret),
            "{label} leaked provider secret value {secret}"
        );
    }
}

fn write_provider_app(path: &Path) {
    std::fs::write(
        path,
        r#"@server {
  @listen 8080
  let payments = @payment.connect(@env.PAYMENT_ADAPTER_URL ?? "stripe://local")
  let shipping = @shipping.connect(@env.SHIPPING_ADAPTER_URL ?? "carrier://local")
  @route POST /checkout {
    let captured = payments.capture({ orderId: "o_1", amount: 42, method: "card" })
    let booked = shipping.book({ orderId: "o_1", carrier: "post", address: "Seoul" })
    @respond 200 { payment: captured.status, shipment: booked.status }
  }
}
"#,
    )
    .expect("write provider app");
}

fn write_db_bridge_app(path: &Path) {
    std::fs::write(
        path,
        r#"@server {
  @listen 8080
  let analytics = @db.connect "postgres://db.internal/shop"
  @route GET /ping { @respond 200 { ok: true } }
}
"#,
    )
    .expect("write DB bridge app");
}

#[test]
fn provider_secret_values_do_not_leak_to_deploy_artifacts_or_env_check_output() {
    let root = temp_dir("provider-secret-redaction");
    std::fs::create_dir_all(&root).expect("create temp dir");
    let source = root.join("app.orv");
    let out = root.join("dist");
    write_provider_app(&source);

    let build = Command::new(orv_bin())
        .args(["build"])
        .arg(&source)
        .args(["--prod", "--out"])
        .arg(&out)
        .output()
        .expect("run prod build");
    assert_success(&build, "orv build");

    for relative in [
        "deploy/manifest.json",
        "deploy/container.json",
        "deploy/preflight.json",
        "deploy/commerce-adapters.json",
        "deploy/env.example",
        "deploy/compose.yaml",
        "deploy/README.md",
        "deploy/smoke-test.sh",
    ] {
        let path = out.join(relative);
        let text = std::fs::read_to_string(&path).expect("read deploy artifact");
        assert_no_secret_values(relative, &text);
    }

    let check = Command::new(orv_bin())
        .arg("deploy-env-check")
        .arg(&out)
        .env("STRIPE_SECRET_KEY", SECRET_VALUES[0])
        .env("STRIPE_WEBHOOK_SECRET", SECRET_VALUES[1])
        .env("STRIPE_WEBHOOK_SECRET_PREVIOUS", SECRET_VALUES[2])
        .env("CARRIER_API_KEY", SECRET_VALUES[3])
        .env("CARRIER_WEBHOOK_SECRET", SECRET_VALUES[4])
        .output()
        .expect("run deploy-env-check");
    assert_success(&check, "orv deploy-env-check");
    let output_text = format!(
        "{}{}",
        String::from_utf8_lossy(&check.stdout),
        String::from_utf8_lossy(&check.stderr)
    );
    assert_no_secret_values("deploy-env-check output", &output_text);

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn db_bridge_auth_tokens_do_not_leak_to_deploy_artifacts_or_env_check_output() {
    let root = temp_dir("db-bridge-secret-redaction");
    std::fs::create_dir_all(&root).expect("create temp dir");
    let source = root.join("app.orv");
    let out = root.join("dist");
    write_db_bridge_app(&source);

    let build = Command::new(orv_bin())
        .args(["build"])
        .arg(&source)
        .args(["--prod", "--out"])
        .arg(&out)
        .output()
        .expect("run prod build");
    assert_success(&build, "orv build");

    for relative in [
        "deploy/manifest.json",
        "deploy/container.json",
        "deploy/preflight.json",
        "deploy/db-adapters.json",
        "deploy/env.example",
        "deploy/compose.yaml",
        "deploy/README.md",
        "deploy/smoke-test.sh",
    ] {
        let path = out.join(relative);
        let text = std::fs::read_to_string(&path).expect("read deploy artifact");
        assert_no_secret_values(relative, &text);
    }

    let check = Command::new(orv_bin())
        .arg("deploy-env-check")
        .arg(&out)
        .env(
            "ORV_DB_ADAPTER_POSTGRES_ENDPOINT",
            "http://127.0.0.1:65535/db-adapter",
        )
        .env("ORV_DB_ADAPTER_POSTGRES_AUTH_TOKEN", SECRET_VALUES[5])
        .env("ORV_DB_ADAPTER_AUTH_TOKEN", SECRET_VALUES[6])
        .output()
        .expect("run deploy-env-check");
    assert_success(&check, "orv deploy-env-check");
    let output_text = format!(
        "{}{}",
        String::from_utf8_lossy(&check.stdout),
        String::from_utf8_lossy(&check.stderr)
    );
    assert_no_secret_values("deploy-env-check output", &output_text);

    let _ = std::fs::remove_dir_all(root);
}
