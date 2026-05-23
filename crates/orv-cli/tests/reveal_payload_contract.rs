use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;

const CLI_ROOT_KEYS: &[&str] = &[
    "origin",
    "production",
    "project_graph",
    "schema_version",
    "source",
];
const EDITOR_ROOT_KEYS: &[&str] = &[
    "focus",
    "origin",
    "production",
    "project_graph",
    "schema_version",
    "source",
];
const LSP_ROOT_KEYS: &[&str] = &[
    "location",
    "origin",
    "production",
    "project_graph",
    "schema_version",
];
const CLI_SOURCE_KEYS: &[&str] = &["content", "end", "file", "path", "snippet", "start"];
const EDITOR_SOURCE_KEYS: &[&str] = &["file", "location", "path", "snippet"];
const FOCUS_KEYS: &[&str] = &["node_id", "origin_id", "panel"];
const LOCATION_KEYS: &[&str] = &["range", "uri"];
const RANGE_KEYS: &[&str] = &["end", "start"];
const POSITION_KEYS: &[&str] = &["character", "line"];
const PRODUCTION_KEYS: &[&str] = &[
    "client",
    "commerce_adapters",
    "db_adapters",
    "graph_contract",
    "native_server",
    "preflight",
    "routes",
    "static",
    "summary",
];
const PRODUCTION_SUMMARY_KEYS: &[&str] = &[
    "adapter_count",
    "build_dir",
    "client_capability_surface_count",
    "client_manifest_count",
    "client_target_count",
    "commerce_adapter_count",
    "commerce_target_count",
    "db_adapter_count",
    "db_target_count",
    "graph_contract_count",
    "missing_artifact_count",
    "native_server_blocker_count",
    "native_server_route_count",
    "native_server_target_count",
    "origin_entry_count",
    "preflight_command_count",
    "preflight_optional_env_count",
    "preflight_required_env_count",
    "preflight_route_count",
    "preflight_smoke_summary_missing_count",
    "preflight_smoke_summary_missing_marker_count",
    "preflight_smoke_summary_present_count",
    "preflight_target_count",
    "project_graph_node_count",
    "route_policy_count",
    "route_policy_kind_counts",
    "route_target_count",
    "schema_version",
    "source_bundle_file_count",
    "static_target_count",
    "static_verified_count",
];
const ROUTE_TARGET_KEYS: &[&str] = &[
    "artifact",
    "match",
    "matched_origin_id",
    "method",
    "origin_id",
    "path",
    "policies",
];
const ADAPTER_TARGET_KEYS: &[&str] = &[
    "adapters",
    "artifact",
    "exists",
    "kind",
    "matched",
    "matched_adapter_count",
    "matched_adapters",
    "path",
    "selected_origin_id",
    "source_reveal_commands",
];
const ADAPTER_REVEAL_COMMAND_KEYS: &[&str] = &[
    "adapter_index",
    "command",
    "endpoint",
    "env",
    "kind",
    "provider",
    "record_path",
    "source_origin_id",
];

struct RevealPayloadFixture {
    root: PathBuf,
    out_arg: String,
    route_id: String,
    db_id: String,
    payment_id: String,
}

#[test]
fn reveal_payload_v1_freezes_cli_editor_lsp_public_keys() {
    let fixture = build_reveal_payload_fixture();

    let cli_reveal = run_orv_json(&["reveal", &fixture.out_arg, &fixture.route_id]);
    assert_cli_reveal_contract(&cli_reveal, &fixture);

    let editor_reveal = run_orv_json(&["editor", "reveal", &fixture.out_arg, &fixture.payment_id]);
    assert_editor_reveal_contract(&editor_reveal, &fixture);

    let lsp_reveal = run_orv_json(&["lsp", "reveal", &fixture.out_arg, &fixture.db_id]);
    assert_lsp_reveal_contract(&lsp_reveal, &fixture);

    let _ = std::fs::remove_dir_all(fixture.root);
}

fn build_reveal_payload_fixture() -> RevealPayloadFixture {
    let root = temp_dir("reveal-payload-contract");
    std::fs::create_dir_all(&root).expect("create temp dir");
    let source = root.join("app.orv");
    let out = root.join("dist");
    std::fs::write(
        &source,
        r#"@server {
  @listen 8080
  let shopdb = @db.connect(@env.SHOP_DATABASE_URL ?? "postgres://db.internal/shop")
  let payments = @payment.connect(@env.PAYMENT_ADAPTER_URL ?? "http://payments.internal/capture")
  @route POST /checkout {
    let order = await shopdb.create("Order", { id: "o_1", total: 42 })
    let captured = payments.capture({ orderId: order.id, amount: 42, method: "card" })
    @respond 200 { payment: captured.status }
  }
}
"#,
    )
    .expect("write source");

    let source_arg = source.display().to_string();
    let out_arg = out.display().to_string();
    run_orv(&["build", &source_arg, "--prod", "--out", &out_arg]);

    let origin_map = read_json(&out.join("origin-map.json"));
    RevealPayloadFixture {
        root,
        out_arg,
        route_id: origin_id(&origin_map, "route", "POST /checkout"),
        db_id: origin_id(&origin_map, "call", "@db.connect"),
        payment_id: origin_id(&origin_map, "call", "@payment.connect"),
    }
}

fn assert_cli_reveal_contract(reveal: &Value, fixture: &RevealPayloadFixture) {
    assert_object_keys(reveal, CLI_ROOT_KEYS);
    assert_eq!(reveal["schema_version"], 1);
    assert_eq!(reveal["origin"]["id"], fixture.route_id);
    assert_object_keys(&reveal["source"], CLI_SOURCE_KEYS);
    assert!(reveal["source"]["snippet"]
        .as_str()
        .is_some_and(|snippet| snippet.contains("@route POST /checkout")));

    assert_production_contract(&reveal["production"]);
    let route = reveal["production"]["routes"]
        .as_array()
        .expect("routes")
        .iter()
        .find(|route| route["path"] == "/checkout")
        .expect("checkout route target");
    assert_object_keys(route, ROUTE_TARGET_KEYS);
    assert_eq!(route["method"], "POST");
    assert_eq!(route["match"], "direct");
    assert_eq!(route["matched_origin_id"], fixture.route_id);
}

fn assert_editor_reveal_contract(reveal: &Value, fixture: &RevealPayloadFixture) {
    assert_object_keys(reveal, EDITOR_ROOT_KEYS);
    assert_eq!(reveal["schema_version"], 1);
    assert_eq!(reveal["origin"]["id"], fixture.payment_id);
    assert_object_keys(&reveal["focus"], FOCUS_KEYS);
    assert_eq!(reveal["focus"]["origin_id"], fixture.payment_id);
    assert_eq!(reveal["focus"]["panel"], "source");
    assert_object_keys(&reveal["source"], EDITOR_SOURCE_KEYS);
    assert_location_contract(&reveal["source"]["location"]);

    assert_production_contract(&reveal["production"]);
    let target = matched_adapter_target(&reveal["production"]["commerce_adapters"]);
    assert_object_keys(target, ADAPTER_TARGET_KEYS);
    assert_eq!(target["kind"], "commerce_adapters");
    assert_eq!(target["selected_origin_id"], fixture.payment_id);
    assert_eq!(target["matched"], true);
    assert_eq!(target["matched_adapter_count"], 1);

    let command = target["source_reveal_commands"]
        .as_array()
        .expect("source reveal commands")
        .first()
        .expect("source reveal command");
    assert_object_keys(command, ADAPTER_REVEAL_COMMAND_KEYS);
    assert_eq!(command["source_origin_id"], fixture.payment_id);
    assert_eq!(
        command["command"],
        serde_json::json!([
            "orv",
            "editor",
            "reveal",
            fixture.out_arg,
            fixture.payment_id
        ])
    );
}

fn assert_lsp_reveal_contract(reveal: &Value, fixture: &RevealPayloadFixture) {
    assert_object_keys(reveal, LSP_ROOT_KEYS);
    assert_eq!(reveal["schema_version"], 1);
    assert_eq!(reveal["origin"]["id"], fixture.db_id);
    assert_location_contract(&reveal["location"]);

    assert_production_contract(&reveal["production"]);
    let target = matched_adapter_target(&reveal["production"]["db_adapters"]);
    assert_object_keys(target, ADAPTER_TARGET_KEYS);
    assert_eq!(target["kind"], "db_adapters");
    assert_eq!(target["selected_origin_id"], fixture.db_id);
    assert_eq!(target["matched"], true);
    assert_eq!(target["matched_adapter_count"], 1);
    assert_eq!(
        target["matched_adapters"][0]["source_origin_id"],
        fixture.db_id
    );
    assert_eq!(
        target["matched_adapters"][0]["matched_origin_id"],
        fixture.db_id
    );
    assert_eq!(target["matched_adapters"][0]["match"], "direct");
    assert_eq!(
        target["matched_adapters"][0]["bridge"]["contract"],
        "http-json-v1"
    );
}

fn assert_production_contract(production: &Value) {
    assert_object_keys(production, PRODUCTION_KEYS);
    assert_object_keys(&production["summary"], PRODUCTION_SUMMARY_KEYS);
    assert_eq!(production["summary"]["schema_version"], 1);
    assert_eq!(production["summary"]["graph_contract_count"], 3);
}

fn assert_location_contract(location: &Value) {
    assert_object_keys(location, LOCATION_KEYS);
    assert_object_keys(&location["range"], RANGE_KEYS);
    assert_object_keys(&location["range"]["start"], POSITION_KEYS);
    assert_object_keys(&location["range"]["end"], POSITION_KEYS);
}

fn matched_adapter_target(targets: &Value) -> &Value {
    targets
        .as_array()
        .expect("adapter targets")
        .iter()
        .find(|target| target["matched"] == true)
        .expect("matched adapter target")
}

fn assert_object_keys(value: &Value, expected: &[&str]) {
    let object = value.as_object().expect("object");
    let actual = object.keys().map(String::as_str).collect::<BTreeSet<_>>();
    let expected = expected.iter().copied().collect::<BTreeSet<_>>();
    assert_eq!(actual, expected);
}

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
