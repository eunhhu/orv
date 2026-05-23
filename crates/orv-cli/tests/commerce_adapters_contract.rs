use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{json, Value};

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

fn read_text(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|err| panic!("read {}: {err}", path.display()))
}

fn read_json(path: &Path) -> Value {
    serde_json::from_str(&read_text(path))
        .unwrap_or_else(|err| panic!("parse {}: {err}", path.display()))
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

fn adapters_without_source_origin_ids(adapters: &Value) -> Value {
    Value::Array(
        adapters
            .as_array()
            .expect("adapters")
            .iter()
            .map(|adapter| {
                let mut adapter = adapter.clone();
                adapter
                    .as_object_mut()
                    .expect("adapter object")
                    .remove("source_origin_id");
                adapter
                    .as_object_mut()
                    .expect("adapter object")
                    .remove("source_origin_ids");
                adapter
            })
            .collect(),
    )
}

struct CommerceFixture {
    root: PathBuf,
    out_arg: String,
    deploy: Value,
    container: Value,
    adapters: Value,
    compose: String,
    runbook: String,
    payment_origin_id: String,
    shipping_origin_id: String,
}

#[test]
fn commerce_adapters_v1_freezes_http_adapter_artifacts() {
    let fixture = build_http_commerce_fixture();

    assert_source_origin_contract(&fixture);
    assert_adapter_artifact_contract(&fixture.adapters);
    assert_deploy_handoff_contract(&fixture);
    assert_reveal_contract(&fixture);

    let _ = std::fs::remove_dir_all(fixture.root);
}

fn build_http_commerce_fixture() -> CommerceFixture {
    let root = temp_dir("commerce-adapters-contract");
    let out = root.join("dist");
    let source = root.join("app.orv");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("create temp root");
    std::fs::write(
        &source,
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
    .expect("write source");
    let source_arg = source.display().to_string();
    let out_arg = out.display().to_string();

    run_orv(&["build", &source_arg, "--prod", "--out", &out_arg]);
    run_orv(&["verify-build", &out_arg]);

    let deploy = read_json(&out.join("deploy").join("manifest.json"));
    let container = read_json(&out.join("deploy").join("container.json"));
    let adapters = read_json(&out.join("deploy").join("commerce-adapters.json"));
    let origin_map = read_json(&out.join("origin-map.json"));
    let compose = read_text(&out.join("deploy").join("compose.yaml"));
    let runbook = read_text(&out.join("deploy").join("README.md"));
    let payment_origin_id = origin_id(&origin_map, "call", "@payment.connect");
    let shipping_origin_id = origin_id(&origin_map, "call", "@shipping.connect");

    CommerceFixture {
        root,
        out_arg,
        deploy,
        container,
        adapters,
        compose,
        runbook,
        payment_origin_id,
        shipping_origin_id,
    }
}

fn assert_source_origin_contract(fixture: &CommerceFixture) {
    let adapter_list = fixture.adapters["adapters"]
        .as_array()
        .expect("adapter list");
    assert_eq!(
        adapter_list[0]["source_origin_id"],
        json!(fixture.payment_origin_id)
    );
    assert_eq!(
        adapter_list[0]["source_origin_ids"],
        json!([fixture.payment_origin_id])
    );
    assert_eq!(
        adapter_list[1]["source_origin_id"],
        json!(fixture.shipping_origin_id)
    );
    assert_eq!(
        adapter_list[1]["source_origin_ids"],
        json!([fixture.shipping_origin_id])
    );
}

fn assert_adapter_artifact_contract(adapters: &Value) {
    assert_eq!(adapters["schema_version"], json!(1));
    assert_eq!(adapters["artifact"], json!("server/app.orv-runtime.json"));
    assert_eq!(
        adapters_without_source_origin_ids(&adapters["adapters"]),
        expected_http_adapters()
    );
}

fn expected_http_adapters() -> Value {
    json!([
        {
            "kind": "payment",
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
}

fn assert_deploy_handoff_contract(fixture: &CommerceFixture) {
    assert_eq!(
        fixture.deploy["server"]["commerce_adapters"],
        json!("deploy/commerce-adapters.json")
    );
    assert_eq!(
        fixture.deploy["server"]["persistence"]["commerce_endpoints"],
        json!([
            "http://payments.internal/capture",
            "http://shipping.internal/book"
        ])
    );
    assert_eq!(
        fixture.deploy["server"]["persistence"]["commerce_env"],
        json!([
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
        fixture.container["persistence"]["commerce_env"],
        fixture.deploy["server"]["persistence"]["commerce_env"]
    );
    assert!(fixture.container["persistence"]["volumes"]
        .as_array()
        .expect("volumes")
        .is_empty());
    assert!(fixture.compose.contains(
        r#"PAYMENT_ADAPTER_URL: "${PAYMENT_ADAPTER_URL:-http://payments.internal/capture}""#
    ));
    assert!(fixture.compose.contains(
        r#"SHIPPING_ADAPTER_URL: "${SHIPPING_ADAPTER_URL:-http://shipping.internal/book}""#
    ));
    assert!(fixture.runbook.contains(
        "- Commerce adapter env: PAYMENT_ADAPTER_URL default http://payments.internal/capture"
    ));
    assert!(fixture.runbook.contains(
        "- Commerce adapter env: SHIPPING_ADAPTER_URL default http://shipping.internal/book"
    ));
    assert!(fixture.runbook.contains("deploy/commerce-adapters.json"));
}

fn assert_reveal_contract(fixture: &CommerceFixture) {
    let reveal = run_orv_json(&["reveal", &fixture.out_arg, &fixture.payment_origin_id]);
    let matched = reveal["production"]["commerce_adapters"][0]["matched_adapters"]
        .as_array()
        .expect("matched adapters");
    assert_eq!(
        matched[0]["source_origin_id"],
        json!(fixture.payment_origin_id)
    );
    assert_eq!(
        matched[0]["endpoint"],
        json!("http://payments.internal/capture")
    );
    assert_eq!(matched[0]["request"]["kind"], json!("payment.capture"));
    assert_eq!(matched[0]["request"]["method"], json!("POST"));
}
