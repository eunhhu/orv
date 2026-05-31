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
const GRAPH_SOURCE_BUNDLE_KEYS: &[&str] = &[
    "artifact_hash",
    "entry",
    "exists",
    "file_count",
    "files",
    "kind",
    "path",
    "schema_version",
];
const GRAPH_SOURCE_BUNDLE_FILE_KEYS: &[&str] = &["content_hash", "path"];
const GRAPH_PROJECT_GRAPH_KEYS: &[&str] = &[
    "artifact_hash",
    "edge_count",
    "exists",
    "kind",
    "node_count",
    "path",
    "schema_version",
    "semantic_edge_count",
    "semantic_origin_count",
    "semantic_origin_link_count",
    "stats",
];
const GRAPH_ORIGIN_MAP_KEYS: &[&str] = &[
    "artifact_hash",
    "call_edge_count",
    "edge_count",
    "entry_count",
    "exists",
    "kind",
    "path",
    "version",
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
const REVEAL_PRODUCTION_SUMMARY_GOLDEN: &str =
    include_str!("../../../docs/samples/reveal-production-summary-v1.golden.json");
const BUILD_DIR_PLACEHOLDER: &str = "<build-dir>";
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

#[test]
fn reveal_payload_v1_preserves_missing_adapter_target_shape() {
    let fixture = build_reveal_payload_fixture();
    std::fs::remove_file(fixture.root.join("dist/deploy/db-adapters.json"))
        .expect("remove db adapters artifact");
    std::fs::remove_file(fixture.root.join("dist/deploy/commerce-adapters.json"))
        .expect("remove commerce adapters artifact");

    let reveal = run_orv_json(&["reveal", &fixture.out_arg, &fixture.route_id]);
    let db_target = reveal["production"]["db_adapters"]
        .as_array()
        .expect("db adapter targets")
        .first()
        .expect("db adapter target");
    let commerce_target = reveal["production"]["commerce_adapters"]
        .as_array()
        .expect("commerce adapter targets")
        .first()
        .expect("commerce adapter target");
    assert_missing_adapter_target_contract(db_target, "db_adapters", &fixture.route_id);
    assert_missing_adapter_target_contract(commerce_target, "commerce_adapters", &fixture.route_id);
    assert_eq!(reveal["production"]["summary"]["db_target_count"], 1);
    assert_eq!(reveal["production"]["summary"]["commerce_target_count"], 1);
    assert_eq!(reveal["production"]["summary"]["adapter_count"], 0);
    assert_eq!(reveal["production"]["summary"]["missing_artifact_count"], 2);

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
    assert_production_summary_golden(&reveal["production"]["summary"]);
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
    let policy = route["policies"]
        .as_array()
        .expect("route policies")
        .iter()
        .find(|policy| policy["kind"] == "rate_limit")
        .expect("checkout rate-limit policy");
    assert_eq!(policy["surface"], "shop_template");
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
    assert_eq!(
        target["matched_adapters"][0]["surface"],
        "library_provider_package"
    );
    assert_eq!(target["matched_adapters"][0]["package"], "orv-commerce");

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
    assert_graph_contract_targets(&production["graph_contract"]);
    assert_object_keys(&production["summary"], PRODUCTION_SUMMARY_KEYS);
    assert_eq!(production["summary"]["schema_version"], 1);
    assert_eq!(production["summary"]["graph_contract_count"], 3);
}

fn assert_production_summary_golden(summary: &Value) {
    assert_object_keys(summary, PRODUCTION_SUMMARY_KEYS);
    let summary = normalize_summary_build_dir(summary.clone());
    let expected: Value =
        serde_json::from_str(REVEAL_PRODUCTION_SUMMARY_GOLDEN).expect("reveal summary golden");
    assert_eq!(summary, expected, "reveal production summary golden drift");
}

fn normalize_summary_build_dir(mut summary: Value) -> Value {
    assert!(
        summary["build_dir"].is_string(),
        "production summary build_dir must be a string"
    );
    summary["build_dir"] = serde_json::json!(BUILD_DIR_PLACEHOLDER);
    summary
}

fn assert_graph_contract_targets(targets: &Value) {
    let targets = targets.as_array().expect("graph contract targets");
    assert_eq!(targets.len(), 3);
    for target in targets {
        match target["kind"].as_str().expect("graph contract kind") {
            "source_bundle" => assert_source_bundle_graph_target(target),
            "project_graph" => assert_project_graph_target(target),
            "origin_map" => assert_origin_map_target(target),
            kind => panic!("unexpected graph contract kind {kind}"),
        }
    }
}

fn assert_source_bundle_graph_target(target: &Value) {
    assert_object_keys(target, GRAPH_SOURCE_BUNDLE_KEYS);
    assert_eq!(target["schema_version"], 1);
    assert_eq!(target["path"], "source-bundle.json");
    assert_eq!(target["exists"], true);
    assert_eq!(target["file_count"], 1);
    assert!(target["artifact_hash"]
        .as_str()
        .is_some_and(|hash| hash.len() == 16));
    let file = target["files"]
        .as_array()
        .expect("source bundle graph files")
        .first()
        .expect("source bundle graph file");
    assert_object_keys(file, GRAPH_SOURCE_BUNDLE_FILE_KEYS);
    assert!(file["content_hash"]
        .as_str()
        .is_some_and(|hash| hash.starts_with("fnv1a64:")));
}

fn assert_project_graph_target(target: &Value) {
    assert_object_keys(target, GRAPH_PROJECT_GRAPH_KEYS);
    assert_eq!(target["schema_version"], 1);
    assert_eq!(target["path"], "project-graph.json");
    assert_eq!(target["exists"], true);
    assert!(target["artifact_hash"]
        .as_str()
        .is_some_and(|hash| hash.len() == 16));
    assert!(target["node_count"].as_u64().is_some());
    assert!(target["edge_count"].as_u64().is_some());
    assert!(target["stats"].as_object().is_some());
}

fn assert_origin_map_target(target: &Value) {
    assert_object_keys(target, GRAPH_ORIGIN_MAP_KEYS);
    assert_eq!(target["version"], 2);
    assert_eq!(target["path"], "origin-map.json");
    assert_eq!(target["exists"], true);
    assert!(target["artifact_hash"]
        .as_str()
        .is_some_and(|hash| hash.len() == 16));
    assert!(target["entry_count"].as_u64().is_some());
    assert!(target["edge_count"].as_u64().is_some());
}

fn assert_location_contract(location: &Value) {
    assert_object_keys(location, LOCATION_KEYS);
    assert!(location["uri"]
        .as_str()
        .is_some_and(|uri| uri.starts_with("file://")));
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

fn assert_missing_adapter_target_contract(target: &Value, kind: &str, origin_id: &str) {
    assert_object_keys(target, ADAPTER_TARGET_KEYS);
    assert_eq!(target["kind"], kind);
    assert_eq!(target["exists"], false);
    assert_eq!(target["selected_origin_id"], origin_id);
    assert_eq!(target["matched"], false);
    assert_eq!(target["matched_adapter_count"], 0);
    assert!(target["artifact"].is_null());
    assert_eq!(target["adapters"], serde_json::json!([]));
    assert_eq!(target["source_reveal_commands"], serde_json::json!([]));
    assert_eq!(target["matched_adapters"], serde_json::json!([]));
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
