use crate::support::{assert_keys, assert_success, orv_output as run_orv, read_json, temp_dir};
use std::path::{Path, PathBuf};

const BUILD_MANIFEST_GOLDEN: &str =
    include_str!("../../../docs/samples/build-manifest-v1.golden.json");
const BUNDLE_PLAN_GOLDEN: &str = include_str!("../../../docs/samples/bundle-plan-v1.golden.json");
const SOURCE_BUNDLE_GOLDEN: &str =
    include_str!("../../../docs/samples/source-bundle-v1.golden.json");
const FIXTURE_PATH_PLACEHOLDER: &str = "<fixture>/app.orv";

fn write_server_fixture(root: &Path) -> PathBuf {
    let entry = root.join("app.orv");
    std::fs::write(
        &entry,
        r#"@server {
  @listen 8080

  @route GET /ping {
    @respond 200 { ok: true, msg: "pong" }
  }
}
"#,
    )
    .expect("write fixture");
    entry
}

fn normalize_entry_path(mut value: serde_json::Value) -> serde_json::Value {
    let entry = value["entry"].as_str().expect("entry path");
    assert!(
        entry.ends_with("/app.orv"),
        "unexpected entry path: {entry}"
    );
    value["entry"] = serde_json::json!(FIXTURE_PATH_PLACEHOLDER);
    value
}

fn normalize_source_bundle_paths(mut value: serde_json::Value) -> serde_json::Value {
    value = normalize_entry_path(value);
    let files = value["files"].as_array_mut().expect("source files");
    for file in files {
        let path = file["path"].as_str().expect("source file path");
        assert!(path.ends_with("/app.orv"), "unexpected source path: {path}");
        file["path"] = serde_json::json!(FIXTURE_PATH_PLACEHOLDER);
    }
    value
}

#[test]
fn build_artifacts_v1_freezes_common_build_artifact_shapes() {
    let out = temp_dir("build-artifacts-contract");
    let _ = std::fs::remove_dir_all(&out);
    std::fs::create_dir_all(&out).expect("temp output dir");
    let entry = write_server_fixture(&out);
    let entry_arg = entry.display().to_string();
    let build_out = out.join("dist");
    let out_arg = build_out.display().to_string();

    let build = run_orv(&["build", &entry_arg, "--out", &out_arg]);
    assert_success(&build, "orv build");

    let manifest = read_json(&build_out.join("build-manifest.json"));
    let manifest_golden: serde_json::Value =
        serde_json::from_str(BUILD_MANIFEST_GOLDEN).expect("build manifest golden");
    assert_eq!(
        normalize_entry_path(manifest),
        manifest_golden,
        "build manifest golden drift"
    );

    let bundle_plan = read_json(&build_out.join("bundle-plan.json"));
    let bundle_plan_golden: serde_json::Value =
        serde_json::from_str(BUNDLE_PLAN_GOLDEN).expect("bundle plan golden");
    assert_eq!(bundle_plan, bundle_plan_golden, "bundle plan golden drift");

    let source_bundle = read_json(&build_out.join("source-bundle.json"));
    let source_bundle_golden: serde_json::Value =
        serde_json::from_str(SOURCE_BUNDLE_GOLDEN).expect("source bundle golden");
    assert_eq!(
        normalize_source_bundle_paths(source_bundle),
        source_bundle_golden,
        "source bundle golden drift"
    );
    let origin_map = read_json(&build_out.join("origin-map.json"));
    assert_origin_map_root_contract(&origin_map);
    assert_project_graph_root_contract(
        &read_json(&build_out.join("project-graph.json")),
        &origin_map,
    );

    let verify = run_orv(&["verify-build", &out_arg]);
    assert_success(&verify, "orv verify-build");

    let _ = std::fs::remove_dir_all(out);
}

fn assert_origin_map_root_contract(origin_map: &serde_json::Value) {
    assert_keys(origin_map, &["version", "entries", "edges"], "origin map");
    assert_eq!(origin_map["version"], serde_json::json!(2));
    assert!(origin_map["entries"].is_array());
    assert!(origin_map["edges"].is_array());
}

fn assert_project_graph_root_contract(graph: &serde_json::Value, origin_map: &serde_json::Value) {
    assert_keys(
        graph,
        &["schema_version", "stats", "nodes", "edges", "semantic"],
        "project graph",
    );
    assert_eq!(graph["schema_version"], serde_json::json!(1));
    assert!(graph["stats"].is_object());
    assert!(graph["nodes"].is_array());
    assert!(graph["edges"].is_array());
    assert_eq!(graph["semantic"]["origin_map"], *origin_map);
}
