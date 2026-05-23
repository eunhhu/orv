use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_dir(name: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    std::env::temp_dir().join(format!("orv-{name}-{}-{nonce}", std::process::id()))
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates dir")
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

const fn orv_bin() -> &'static str {
    env!("CARGO_BIN_EXE_orv")
}

fn run_orv_json(args: &[&str]) -> serde_json::Value {
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
    serde_json::from_slice(&output.stdout).expect("json stdout")
}

fn run_orv(args: &[&str]) -> std::process::Output {
    Command::new(orv_bin())
        .args(args)
        .output()
        .expect("run orv")
}

fn read_json(path: &Path) -> serde_json::Value {
    serde_json::from_str(&std::fs::read_to_string(path).expect("read json")).expect("json")
}

fn assert_keys(value: &serde_json::Value, expected: &[&str], context: &str) {
    let object = value
        .as_object()
        .unwrap_or_else(|| panic!("{context} must be an object"));
    let actual = object.keys().map(String::as_str).collect::<BTreeSet<_>>();
    let expected = expected.iter().copied().collect::<BTreeSet<_>>();
    assert_eq!(actual, expected, "{context} keys drifted");
}

#[test]
fn project_graph_v1_freezes_cli_json_and_view_artifact_shape() {
    let entry = workspace_root()
        .join("fixtures")
        .join("e2e")
        .join("hello.orv");
    let entry_arg = entry.display().to_string();
    let cli_graph = run_orv_json(&["graph", &entry_arg]);

    assert_project_graph_contract(&cli_graph);
    assert!(cli_graph["nodes"].as_array().is_some_and(|nodes| {
        nodes
            .iter()
            .any(|node| node["kind"] == "domain" && node["name"] == "route")
    }));
    assert!(cli_graph["semantic"]["origin_map"]["entries"]
        .as_array()
        .is_some_and(|entries| {
            entries
                .iter()
                .any(|entry| entry["kind"] == "route" && entry["name"] == "GET /ping")
        }));

    let out = temp_dir("project-graph-contract");
    let out_arg = out.display().to_string();
    let view = run_orv(&["graph", &entry_arg, "--view", "--out", &out_arg]);
    assert!(
        view.status.success(),
        "orv graph --view failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&view.stdout),
        String::from_utf8_lossy(&view.stderr)
    );
    assert!(String::from_utf8_lossy(&view.stdout).starts_with("graph view: "));
    let view_graph = read_json(&out.join("graph.json"));
    assert_project_graph_contract(&view_graph);
    assert_eq!(view_graph, cli_graph);
    let html = std::fs::read_to_string(out.join("index.html")).expect("graph html");
    assert!(html.contains("ORV Project Graph"));
    assert!(html.contains("graph.json"));
    assert!(html.contains("filterProjectGraphRows"));

    let _ = std::fs::remove_dir_all(out);
}

fn assert_project_graph_contract(graph: &serde_json::Value) {
    assert_keys(
        graph,
        &["schema_version", "stats", "nodes", "edges", "semantic"],
        "project graph",
    );
    assert_eq!(graph["schema_version"], serde_json::json!(1));

    assert_keys(
        &graph["stats"],
        &[
            "node_count",
            "edge_count",
            "file_count",
            "import_count",
            "declaration_count",
            "domain_count",
            "max_source_contains_depth",
            "semantic_origin_count",
            "semantic_edge_count",
            "semantic_call_edge_count",
            "max_semantic_contains_depth",
        ],
        "project graph stats",
    );
    for key in graph["stats"].as_object().expect("stats object").keys() {
        assert!(
            graph["stats"][key].is_u64(),
            "project graph stats.{key} must be an unsigned integer"
        );
    }

    let node = graph["nodes"]
        .as_array()
        .expect("nodes array")
        .first()
        .expect("node");
    assert_keys(node, &["id", "kind", "name", "file", "span"], "node");
    assert!(node["id"].is_u64());
    assert!(node["kind"].is_string());
    assert!(node["name"].is_string());
    assert!(node["file"].is_u64());
    assert_span_contract(&node["span"], "node span");

    let edge = graph["edges"]
        .as_array()
        .expect("edges array")
        .first()
        .expect("edge");
    assert_keys(edge, &["from", "to", "kind"], "edge");
    assert!(edge["from"].is_u64());
    assert!(edge["to"].is_u64());
    assert!(edge["kind"].is_string());

    assert_keys(
        &graph["semantic"],
        &["origin_map", "origin_edges", "origin_links"],
        "semantic",
    );
    assert_origin_map_contract(&graph["semantic"]["origin_map"]);

    let semantic_edge = graph["semantic"]["origin_edges"]
        .as_array()
        .expect("origin edges array")
        .first()
        .expect("semantic origin edge");
    assert_keys(
        semantic_edge,
        &["kind", "from", "to"],
        "semantic origin edge",
    );
    assert!(semantic_edge["kind"].is_string());
    assert!(semantic_edge["from"].is_string());
    assert!(semantic_edge["to"].is_string());

    let origin_link = graph["semantic"]["origin_links"]
        .as_array()
        .expect("origin links array")
        .first()
        .expect("semantic origin link");
    assert_keys(
        origin_link,
        &["kind", "origin_id", "node_id"],
        "semantic origin link",
    );
    assert_eq!(origin_link["kind"], serde_json::json!("source_node"));
    assert!(origin_link["origin_id"].is_string());
    assert!(origin_link["node_id"].is_u64());
}

fn assert_origin_map_contract(origin_map: &serde_json::Value) {
    assert_keys(origin_map, &["version", "entries", "edges"], "origin map");
    assert_eq!(origin_map["version"], serde_json::json!(2));

    let entry = origin_map["entries"]
        .as_array()
        .expect("origin entries array")
        .first()
        .expect("origin entry");
    assert_keys(
        entry,
        &["id", "kind", "name", "span", "fingerprint"],
        "origin entry",
    );
    assert!(entry["id"].is_string());
    assert!(entry["kind"].is_string());
    assert!(entry["name"].is_string());
    assert!(entry["fingerprint"].is_string());
    assert_span_contract(&entry["span"], "origin entry span");

    let edge = origin_map["edges"]
        .as_array()
        .expect("origin edges array")
        .first()
        .expect("origin map edge");
    assert_keys(edge, &["from", "to", "kind"], "origin map edge");
    assert!(edge["from"].is_string());
    assert!(edge["to"].is_string());
    assert!(edge["kind"].is_string());
}

fn assert_span_contract(span: &serde_json::Value, context: &str) {
    assert_keys(span, &["file", "start", "end"], context);
    assert!(span["file"].is_u64());
    assert!(span["start"].is_u64());
    assert!(span["end"].is_u64());
}
