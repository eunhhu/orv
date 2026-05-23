use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

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
    assert_success(&output, &format!("orv {args:?}"));
}

fn assert_success(output: &Output, context: &str) {
    assert!(
        output.status.success(),
        "{context} failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn read_text(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|err| panic!("read {}: {err}", path.display()))
}

fn read_json(path: &Path) -> Value {
    serde_json::from_str(&read_text(path))
        .unwrap_or_else(|err| panic!("parse {}: {err}", path.display()))
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

struct ProviderFixture {
    root: PathBuf,
    out_arg: String,
    deploy: Value,
    container: Value,
    adapters: Value,
    preflight: Value,
    compose: String,
    env_example: String,
    runbook: String,
}

#[test]
fn commerce_provider_hardening_v1_freezes_deploy_and_env_gate() {
    let fixture = build_provider_fixture();

    assert_provider_adapter_artifact(&fixture.adapters);
    assert_provider_deploy_handoff(&fixture);
    assert_provider_env_gate(&fixture);

    let _ = std::fs::remove_dir_all(fixture.root);
}

fn build_provider_fixture() -> ProviderFixture {
    let root = temp_dir("commerce-provider-hardening");
    let out = root.join("dist");
    let source = root.join("app.orv");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("create temp root");
    write_provider_server_app(&source);
    let source_arg = source.display().to_string();
    let out_arg = out.display().to_string();

    run_orv(&["build", &source_arg, "--prod", "--out", &out_arg]);
    run_orv(&["verify-build", &out_arg]);

    ProviderFixture {
        root,
        out_arg,
        deploy: read_json(&out.join("deploy").join("manifest.json")),
        container: read_json(&out.join("deploy").join("container.json")),
        adapters: read_json(&out.join("deploy").join("commerce-adapters.json")),
        preflight: read_json(&out.join("deploy").join("preflight.json")),
        compose: read_text(&out.join("deploy").join("compose.yaml")),
        env_example: read_text(&out.join("deploy").join("env.example")),
        runbook: read_text(&out.join("deploy").join("README.md")),
    }
}

fn write_provider_server_app(path: &Path) {
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
    .expect("write provider server source");
}

fn assert_provider_adapter_artifact(adapters: &Value) {
    assert_eq!(adapters["schema_version"], json!(1));
    assert_eq!(adapters["artifact"], json!("server/app.orv-runtime.json"));
    assert_eq!(
        adapters_without_source_origin_ids(&adapters["adapters"]),
        expected_provider_adapters()
    );
    assert!(adapters["adapters"]
        .as_array()
        .expect("adapters")
        .iter()
        .all(|adapter| adapter["source_origin_id"]
            .as_str()
            .is_some_and(|origin_id| origin_id.starts_with("ori_"))));
}

fn expected_provider_adapters() -> Value {
    json!([
        {
            "kind": "payment",
            "mode": "provider",
            "provider": "stripe",
            "env": "PAYMENT_ADAPTER_URL",
            "default": "stripe://local",
            "endpoint": null,
            "record_path": null,
            "provider_env": [
                { "env": "STRIPE_API_ENDPOINT", "required": false, "purpose": "api_endpoint" },
                { "env": "STRIPE_SECRET_KEY", "required": true, "purpose": "api_secret" },
                { "env": "STRIPE_WEBHOOK_SECRET", "required": false, "purpose": "webhook_signature" },
                { "env": "STRIPE_WEBHOOK_SECRET_PREVIOUS", "required": false, "purpose": "webhook_signature_previous" }
            ],
            "request": {
                "method": "POST",
                "content_type": "application/json",
                "kind": "payment.capture",
                "body": { "kind": "payment.capture", "payload": "payment capture payload" }
            }
        },
        {
            "kind": "shipping",
            "mode": "provider",
            "provider": "carrier",
            "env": "SHIPPING_ADAPTER_URL",
            "default": "carrier://local",
            "endpoint": null,
            "record_path": null,
            "provider_env": [
                { "env": "CARRIER_API_ENDPOINT", "required": false, "purpose": "api_endpoint" },
                { "env": "CARRIER_API_KEY", "required": true, "purpose": "api_key" },
                { "env": "CARRIER_WEBHOOK_SECRET", "required": false, "purpose": "webhook_signature" }
            ],
            "request": {
                "method": "POST",
                "content_type": "application/json",
                "kind": "shipping.booking",
                "body": { "kind": "shipping.booking", "payload": "shipping booking payload" }
            }
        }
    ])
}

fn assert_provider_deploy_handoff(fixture: &ProviderFixture) {
    assert_eq!(
        fixture.deploy["server"]["commerce_adapters"],
        json!("deploy/commerce-adapters.json")
    );
    assert_eq!(
        fixture.deploy["server"]["persistence"]["commerce_endpoints"],
        json!([])
    );
    assert_eq!(
        fixture.deploy["server"]["persistence"]["commerce_env"],
        json!([
            { "env": "PAYMENT_ADAPTER_URL", "default": "stripe://local" },
            { "env": "SHIPPING_ADAPTER_URL", "default": "carrier://local" }
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
    assert_provider_compose_and_env_example(fixture);
    assert_provider_runbook(fixture);
}

fn assert_provider_compose_and_env_example(fixture: &ProviderFixture) {
    for expected in [
        r#"PAYMENT_ADAPTER_URL: "${PAYMENT_ADAPTER_URL:-stripe://local}""#,
        r#"SHIPPING_ADAPTER_URL: "${SHIPPING_ADAPTER_URL:-carrier://local}""#,
        r#"STRIPE_API_ENDPOINT: "${STRIPE_API_ENDPOINT}""#,
        r#"STRIPE_SECRET_KEY: "${STRIPE_SECRET_KEY}""#,
        r#"STRIPE_WEBHOOK_SECRET: "${STRIPE_WEBHOOK_SECRET}""#,
        r#"STRIPE_WEBHOOK_SECRET_PREVIOUS: "${STRIPE_WEBHOOK_SECRET_PREVIOUS}""#,
        r#"CARRIER_API_ENDPOINT: "${CARRIER_API_ENDPOINT}""#,
        r#"CARRIER_API_KEY: "${CARRIER_API_KEY}""#,
        r#"CARRIER_WEBHOOK_SECRET: "${CARRIER_WEBHOOK_SECRET}""#,
    ] {
        assert!(
            fixture.compose.contains(expected),
            "missing compose {expected}"
        );
    }
    for expected in [
        "STRIPE_API_ENDPOINT=",
        "STRIPE_SECRET_KEY=",
        "STRIPE_WEBHOOK_SECRET=",
        "STRIPE_WEBHOOK_SECRET_PREVIOUS=",
        "CARRIER_API_ENDPOINT=",
        "CARRIER_API_KEY=",
        "CARRIER_WEBHOOK_SECRET=",
    ] {
        assert!(
            fixture.env_example.contains(expected),
            "missing env.example {expected}"
        );
    }
}

fn assert_provider_runbook(fixture: &ProviderFixture) {
    for expected in [
        "- Commerce adapter env: PAYMENT_ADAPTER_URL default stripe://local",
        "- Commerce adapter env: SHIPPING_ADAPTER_URL default carrier://local",
        "- Commerce provider env: payment stripe STRIPE_API_ENDPOINT optional api_endpoint",
        "- Commerce provider env: payment stripe STRIPE_SECRET_KEY required api_secret",
        "- Commerce provider env: payment stripe STRIPE_WEBHOOK_SECRET optional webhook_signature",
        "- Commerce provider env: payment stripe STRIPE_WEBHOOK_SECRET_PREVIOUS optional webhook_signature_previous",
        "- Commerce provider env: shipping carrier CARRIER_API_ENDPOINT optional api_endpoint",
        "- Commerce provider env: shipping carrier CARRIER_API_KEY required api_key",
        "- Commerce provider env: shipping carrier CARRIER_WEBHOOK_SECRET optional webhook_signature",
    ] {
        assert!(fixture.runbook.contains(expected), "missing runbook {expected}");
    }
}

fn assert_provider_env_gate(fixture: &ProviderFixture) {
    assert_preflight_env(
        &fixture.preflight["required_env"],
        "STRIPE_SECRET_KEY",
        "api_secret",
        true,
    );
    assert_preflight_env(
        &fixture.preflight["required_env"],
        "CARRIER_API_KEY",
        "api_key",
        true,
    );
    assert_preflight_env(
        &fixture.preflight["optional_env"],
        "STRIPE_WEBHOOK_SECRET_PREVIOUS",
        "webhook_signature_previous",
        false,
    );

    let missing = Command::new(orv_bin())
        .arg("deploy-env-check")
        .arg(&fixture.out_arg)
        .output()
        .expect("run missing provider env check");
    assert!(!missing.status.success());
    let missing_output = command_output_text(&missing);
    assert!(missing_output.contains("STRIPE_SECRET_KEY"));
    assert!(missing_output.contains("CARRIER_API_KEY"));

    let satisfied = Command::new(orv_bin())
        .arg("deploy-env-check")
        .arg(&fixture.out_arg)
        .env("STRIPE_SECRET_KEY", "sk_contract_secret")
        .env("CARRIER_API_KEY", "carrier_contract_secret")
        .output()
        .expect("run satisfied provider env check");
    assert_success(&satisfied, "orv deploy-env-check provider env");
    let satisfied_output = command_output_text(&satisfied);
    assert!(!satisfied_output.contains("sk_contract_secret"));
    assert!(!satisfied_output.contains("carrier_contract_secret"));
}

fn assert_preflight_env(envs: &Value, name: &str, purpose: &str, required: bool) {
    assert!(envs
        .as_array()
        .expect("preflight env array")
        .iter()
        .any(|env| env["env"] == name && env["purpose"] == purpose && env["required"] == required));
}

#[test]
fn commerce_provider_hardening_v1_retries_with_stable_idempotency_keys() {
    let root = temp_dir("commerce-provider-runtime");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("create temp root");
    let source = root.join("provider.orv");
    write_provider_runtime_app(&source);

    let (address, requests, server) = start_provider_test_server();
    let output = Command::new(orv_bin())
        .arg("run")
        .arg(&source)
        .env(
            "STRIPE_API_ENDPOINT",
            format!("http://{address}/stripe/payment_intents"),
        )
        .env("STRIPE_SECRET_KEY", "sk_contract_secret")
        .env(
            "CARRIER_API_ENDPOINT",
            format!("http://{address}/carrier/shipments"),
        )
        .env("CARRIER_API_KEY", "carrier_contract_secret")
        .output()
        .expect("run provider source");
    assert_success(&output, "orv run provider source");
    server.join().expect("provider server finished");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(stdout, "pi_contract\nship_contract\n");
    assert!(!stdout.contains("sk_contract_secret"));
    assert!(!stderr.contains("carrier_contract_secret"));
    assert_provider_requests(&requests.lock().expect("provider requests"));

    let _ = std::fs::remove_dir_all(root);
}

fn write_provider_runtime_app(path: &Path) {
    std::fs::write(
        path,
        r#"let payments = @payment.connect("stripe://local")
let captured = payments.capture({ orderId: "o_contract", amount: 4200, method: "card" })
let shipping = @shipping.connect("carrier://local")
let booked = shipping.book({ orderId: "o_contract", carrier: "post", address: "Seoul" })
@out captured.id
@out booked.id
"#,
    )
    .expect("write provider runtime source");
}

fn start_provider_test_server() -> (
    std::net::SocketAddr,
    Arc<Mutex<Vec<String>>>,
    thread::JoinHandle<()>,
) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind provider test server");
    listener
        .set_nonblocking(true)
        .expect("set provider listener nonblocking");
    let address = listener.local_addr().expect("provider test server address");
    let requests = Arc::new(Mutex::new(Vec::new()));
    let server_requests = requests.clone();
    let server = thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(10);
        while server_requests.lock().expect("requests").len() < 3 && Instant::now() < deadline {
            match listener.accept() {
                Ok((mut stream, _)) => {
                    let request = read_http_request(&mut stream);
                    let index = {
                        let mut requests = server_requests.lock().expect("requests");
                        let index = requests.len();
                        requests.push(request);
                        index
                    };
                    write_provider_response(&mut stream, index);
                }
                Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(10));
                }
                Err(err) => panic!("accept provider request: {err}"),
            }
        }
    });
    (address, requests, server)
}

fn read_http_request(stream: &mut std::net::TcpStream) -> String {
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .expect("set stream read timeout");
    let mut bytes = Vec::new();
    let mut buf = [0_u8; 512];
    let header_end = loop {
        let read = stream.read(&mut buf).expect("read provider request");
        assert!(read > 0, "provider request closed before headers");
        bytes.extend_from_slice(&buf[..read]);
        if let Some(index) = bytes.windows(4).position(|window| window == b"\r\n\r\n") {
            break index + 4;
        }
    };
    let headers = String::from_utf8_lossy(&bytes[..header_end]);
    let content_length = headers
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case("content-length")
                .then(|| value.trim().parse::<usize>().ok())
                .flatten()
        })
        .unwrap_or(0);
    while bytes.len() < header_end + content_length {
        let read = stream.read(&mut buf).expect("read provider request body");
        assert!(read > 0, "provider request closed before body");
        bytes.extend_from_slice(&buf[..read]);
    }
    String::from_utf8_lossy(&bytes).into_owned()
}

fn write_provider_response(stream: &mut std::net::TcpStream, index: usize) {
    let response = match index {
        0 => (
            "500 Internal Server Error",
            "text/plain",
            "transient provider failure",
        ),
        1 => (
            "200 OK",
            "application/json",
            r#"{"id":"pi_contract","status":"succeeded"}"#,
        ),
        _ => (
            "200 OK",
            "application/json",
            r#"{"id":"ship_contract","status":"booked_provider"}"#,
        ),
    };
    let (status, content_type, body) = response;
    let response = format!(
        "HTTP/1.1 {status}\r\ncontent-type: {content_type}\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
        body.len()
    );
    stream
        .write_all(response.as_bytes())
        .expect("write provider response");
}

fn assert_provider_requests(requests: &[String]) {
    assert_eq!(requests.len(), 3);
    assert!(requests[0].contains(r#""kind":"stripe.payment_intent.create""#));
    assert!(requests[0].contains(r#""orderId":"o_contract""#));
    assert!(requests[0].contains("authorization: Bearer sk_contract_secret"));
    assert!(requests[0].contains("idempotency-key: stripe.payment_intent.create:o_contract"));
    assert!(requests[1].contains("idempotency-key: stripe.payment_intent.create:o_contract"));
    assert!(requests[2].contains(r#""kind":"carrier.shipment.create""#));
    assert!(requests[2].contains("authorization: Bearer carrier_contract_secret"));
    assert!(requests[2].contains("idempotency-key: carrier.shipment.create:o_contract"));
}

fn command_output_text(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}
