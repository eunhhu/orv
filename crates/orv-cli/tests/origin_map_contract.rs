use std::collections::BTreeSet;
use std::path::PathBuf;
use std::process::Command;

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

fn assert_keys(value: &serde_json::Value, expected: &[&str], context: &str) {
    let object = value
        .as_object()
        .unwrap_or_else(|| panic!("{context} must be an object"));
    let actual = object.keys().map(String::as_str).collect::<BTreeSet<_>>();
    let expected = expected.iter().copied().collect::<BTreeSet<_>>();
    assert_eq!(actual, expected, "{context} keys drifted");
}

#[test]
fn origin_map_v2_freezes_cli_json_and_graph_embedding() {
    let entry = workspace_root()
        .join("fixtures")
        .join("e2e")
        .join("hello.orv");
    let entry_arg = entry.display().to_string();

    let origins = run_orv_json(&["origins", &entry_arg]);
    assert_origin_map_contract(&origins);
    assert!(origin_entry(&origins, "domain", "server").is_some());
    let route = origin_entry(&origins, "route", "GET /ping").expect("route origin");
    let respond = origin_entry(&origins, "domain", "respond").expect("respond origin");
    assert!(origin_edge(
        &origins,
        "contains",
        route["id"].as_str(),
        respond["id"].as_str()
    ));

    let graph = run_orv_json(&["graph", &entry_arg]);
    assert_eq!(graph["semantic"]["origin_map"], origins);
}

fn assert_origin_map_contract(origin_map: &serde_json::Value) {
    assert_keys(origin_map, &["version", "entries", "edges"], "origin map");
    assert_eq!(origin_map["version"], serde_json::json!(2));

    let entries = origin_map["entries"].as_array().expect("entries array");
    assert!(!entries.is_empty(), "origin entries must not be empty");
    for entry in entries {
        assert_keys(
            entry,
            &["id", "kind", "name", "span", "fingerprint"],
            "origin entry",
        );
        assert!(entry["id"]
            .as_str()
            .is_some_and(|id| id.starts_with("ori_")));
        assert!(entry["kind"].is_string());
        assert!(entry["name"].is_string());
        assert!(entry["fingerprint"]
            .as_str()
            .is_some_and(|fingerprint| !fingerprint.is_empty()));
        assert_span_contract(&entry["span"], "origin entry span");
    }

    let ids = entries
        .iter()
        .filter_map(|entry| entry["id"].as_str())
        .collect::<BTreeSet<_>>();
    let edges = origin_map["edges"].as_array().expect("edges array");
    assert!(!edges.is_empty(), "origin edges must not be empty");
    for edge in edges {
        assert_keys(edge, &["from", "to", "kind"], "origin edge");
        let from = edge["from"].as_str().expect("edge from");
        let to = edge["to"].as_str().expect("edge to");
        assert!(ids.contains(from), "edge.from must reference an entry");
        assert!(ids.contains(to), "edge.to must reference an entry");
        assert!(matches!(edge["kind"].as_str(), Some("contains" | "calls")));
    }
}

fn origin_entry<'a>(
    origin_map: &'a serde_json::Value,
    kind: &str,
    name: &str,
) -> Option<&'a serde_json::Value> {
    origin_map["entries"]
        .as_array()?
        .iter()
        .find(|entry| entry["kind"] == kind && entry["name"] == name)
}

fn origin_edge(
    origin_map: &serde_json::Value,
    kind: &str,
    from: Option<&str>,
    to: Option<&str>,
) -> bool {
    origin_map["edges"]
        .as_array()
        .into_iter()
        .flatten()
        .any(|edge| {
            edge["kind"] == kind && edge["from"].as_str() == from && edge["to"].as_str() == to
        })
}

fn assert_span_contract(span: &serde_json::Value, context: &str) {
    assert_keys(span, &["file", "start", "end"], context);
    assert!(span["file"].is_u64());
    assert!(span["start"].is_u64());
    assert!(span["end"].is_u64());
}
