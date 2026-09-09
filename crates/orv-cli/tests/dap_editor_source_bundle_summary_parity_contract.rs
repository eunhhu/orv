use crate::source_bundle_support as support;

use serde_json::Value;
use support::{assert_source_bundle_files, build_fixture, read_json, run_orv_json};

const SUMMARY_KEYS: [&str; 31] = [
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
const BUILD_DIR_PLACEHOLDER: &str = "<build-dir>";

#[test]
fn production_summary_parity_matches_run_debug_and_reveal_for_same_fixture() {
    let root = support::temp_dir("source-bundle-summary-parity");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("temp root");

    // Given
    let (app, imported, out) = build_fixture(&root);
    let bundle_path = out.join("source-bundle.json");
    assert_source_bundle_files(&read_json(&bundle_path));
    std::fs::remove_file(&app).expect("remove app source");
    std::fs::remove_file(&imported).expect("remove imported source");
    let origin_map = read_json(&out.join("origin-map.json"));
    let reveal_origin_id = origin_map["entries"]
        .as_array()
        .expect("origin entries")
        .first()
        .and_then(|entry| entry["id"].as_str())
        .expect("origin id")
        .to_string();

    // When
    let out_arg = out.display().to_string();
    let run_args = [
        "editor",
        "run-debug",
        out_arg.as_str(),
        "--control",
        "next",
        "--watch-expression",
        "total",
    ];
    let reveal_args = ["reveal", out_arg.as_str(), reveal_origin_id.as_str()];
    let run = run_orv_json(&run_args);
    let reveal = run_orv_json(&reveal_args);

    // Then
    let run_summary = &run["production_context"]["summary"];
    let panel_summary = &run["panels"]["debug"]["production_summary"];
    let reveal_summary = &reveal["production"]["summary"];
    assert_shared_summary_fields(run_summary);
    assert_shared_summary_fields(panel_summary);
    assert_shared_summary_fields(reveal_summary);
    assert_same_summary_subset(run_summary, panel_summary);
    assert_same_summary_subset(run_summary, reveal_summary);

    let _ = std::fs::remove_dir_all(root);
}

fn assert_shared_summary_fields(summary: &Value) {
    for key in SUMMARY_KEYS {
        assert!(summary.get(key).is_some(), "missing summary key {key}");
    }
}

fn assert_same_summary_subset(left: &Value, right: &Value) {
    let left = normalize_build_dir(left.clone());
    let right = normalize_build_dir(right.clone());
    for key in SUMMARY_KEYS {
        assert_eq!(left[key], right[key], "summary drift at {key}");
    }
}

fn normalize_build_dir(mut summary: Value) -> Value {
    summary["build_dir"] = serde_json::json!(BUILD_DIR_PLACEHOLDER);
    summary
}
