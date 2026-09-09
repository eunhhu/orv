use crate::support::{read_json, run_orv, run_orv_json, temp_dir};

use serde_json::Value;

#[test]
fn reveal_payload_v1_freezes_preflight_benchmark_evidence_proof_gate_fields() {
    // Given: a production build with generated pre-human benchmark evidence.
    let root = temp_dir("reveal-benchmark-evidence-contract");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("create temp dir");

    let source = root.join("app.orv");
    let out = root.join("dist");
    std::fs::write(
        &source,
        r"@server {
  @listen 8080

  @route GET /ping {
    @respond 200 { ok: true }
  }
}
",
    )
    .expect("write fixture");

    let source_arg = source.display().to_string();
    let out_arg = out.display().to_string();
    run_orv(&["build", &source_arg, "--prod", "--out", &out_arg]);

    // When: revealing the route origin from the build artifacts.
    let origin_map = read_json(&out.join("origin-map.json"));
    let route_id = origin_id(&origin_map, "route", "GET /ping");
    let reveal = run_orv_json(&["reveal", &out_arg, &route_id]);

    // Then: the nested benchmark evidence summary preserves proof-gate fields.
    let benchmark = &reveal["production"]["preflight"][0]["benchmark_evidence"];
    assert_eq!(benchmark["exists"], true);
    assert_eq!(benchmark["path"], "deploy/benchmark-evidence.json");
    assert_eq!(benchmark["report_status"], "incomplete");
    assert_eq!(benchmark["recording_status"], "not_recorded");
    assert_eq!(benchmark["failed_data_count"], 0);
    assert_eq!(benchmark["failed_data"], serde_json::json!([]));

    let missing_data = benchmark["missing_data"]
        .as_array()
        .expect("missing_data array");
    let missing_data_count = benchmark["missing_data_count"]
        .as_u64()
        .expect("missing_data_count");
    assert_eq!(
        missing_data_count,
        u64::try_from(missing_data.len()).expect("missing data length fits u64")
    );
    assert!(missing_data.iter().any(|item| item == "smoke_test_output"));
    assert!(missing_data
        .iter()
        .any(|item| item == "participant_runs.minimum"));
    assert!(missing_data
        .iter()
        .any(|item| item == "human_evidence_review.raw_notes_reviewed"));

    let raw_notes_artifacts = benchmark["participant_raw_notes_artifacts"]
        .as_array()
        .expect("participant_raw_notes_artifacts array");
    let first_artifact = raw_notes_artifacts.first().expect("raw-notes artifact");
    assert_eq!(first_artifact["checked"], false);
    assert!(first_artifact["retained"].is_null());

    let required_markers = benchmark["smoke_test_required_markers"]
        .as_array()
        .expect("smoke_test_required_markers array");
    assert!(!required_markers.is_empty());
    assert!(required_markers
        .iter()
        .any(|marker| marker == "trace_stream_requested"));
    assert!(required_markers.iter().any(|marker| marker == "base_url"));

    assert!(benchmark["smoke_test_output_source"].is_null());
    assert!(benchmark["smoke_test_output_artifact_path"].is_null());
    assert!(benchmark["smoke_test_output_artifact_match"].is_null());

    let _ = std::fs::remove_dir_all(root);
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
