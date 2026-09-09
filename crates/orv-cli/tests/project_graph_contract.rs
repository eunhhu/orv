use crate::support::{orv_output as run_orv, read_json, run_orv_json, temp_dir};
use std::path::PathBuf;

const PROJECT_GRAPH_GOLDEN: &str =
    include_str!("../../../docs/samples/project-graph-v1.golden.json");
const WORKSPACE_HELLO_PLACEHOLDER: &str = "<workspace>/fixtures/e2e/hello.orv";

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates dir")
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

fn normalize_project_graph_paths(mut graph: serde_json::Value) -> serde_json::Value {
    let nodes = graph["nodes"].as_array_mut().expect("project graph nodes");
    for node in nodes {
        if node["kind"] == "file" {
            let name = node["name"].as_str().expect("file node name");
            assert!(
                name.ends_with("/fixtures/e2e/hello.orv"),
                "unexpected project graph file node name: {name}"
            );
            node["name"] = serde_json::json!(WORKSPACE_HELLO_PLACEHOLDER);
        }
    }
    graph
}

#[test]
fn project_graph_v1_freezes_cli_json_and_view_artifact_shape() {
    let entry = workspace_root()
        .join("fixtures")
        .join("e2e")
        .join("hello.orv");
    let entry_arg = entry.display().to_string();
    let cli_graph = run_orv_json(&["graph", &entry_arg]);
    let expected_golden: serde_json::Value =
        serde_json::from_str(PROJECT_GRAPH_GOLDEN).expect("project graph golden");
    assert_eq!(
        normalize_project_graph_paths(cli_graph.clone()),
        expected_golden,
        "project graph golden drift"
    );

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
    assert_eq!(view_graph, cli_graph);
    let html = std::fs::read_to_string(out.join("index.html")).expect("graph html");
    assert!(html.contains("ORV Project Graph"));
    assert!(html.contains("graph.json"));
    assert!(html.contains("filterProjectGraphRows"));

    let _ = std::fs::remove_dir_all(out);
}
