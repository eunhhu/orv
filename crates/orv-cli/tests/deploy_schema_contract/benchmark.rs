use crate::common::assert_keys;

pub(crate) fn assert_benchmark_evidence_contract(
    evidence: &serde_json::Value,
    preflight: &serde_json::Value,
) {
    assert_keys(
        evidence,
        &[
            "schema_version",
            "kind",
            "preflight",
            "preflight_hash",
            "benchmark",
            "commands",
            "artifacts",
            "smoke_output_contract",
            "recording_status",
            "task_entries",
            "data",
        ],
        "benchmark evidence",
    );
    assert_eq!(evidence["schema_version"], serde_json::json!(1));
    assert_eq!(
        evidence["kind"],
        serde_json::json!("orv.benchmark.shop_5h.evidence")
    );
    assert!(evidence["preflight_hash"].is_string());
    assert_eq!(evidence["commands"], preflight["commands"]);
    assert_eq!(evidence["artifacts"], preflight["artifacts"]);
    assert_eq!(
        evidence["smoke_output_contract"],
        preflight["smoke_output_contract"]
    );
    assert_eq!(
        evidence["data"]["smoke_test_required_markers"],
        preflight["smoke_output_contract"]["required_markers"],
        "benchmark evidence smoke_test_required_markers must mirror preflight required markers"
    );
    assert_eq!(
        evidence["recording_status"],
        serde_json::json!("not_recorded")
    );
    assert_eq!(
        evidence["data"]["recommended_participant_count"]["minimum"],
        serde_json::json!(2)
    );
    assert_eq!(
        evidence["data"]["recommended_participant_count"]["target"],
        serde_json::json!(3)
    );
    let participant = evidence["data"]["participant_runs"]
        .as_array()
        .expect("benchmark evidence participant runs")
        .first()
        .expect("benchmark evidence seed participant run");
    assert_eq!(participant["status"], serde_json::json!("not_recorded"));
    assert_eq!(
        participant["participant_profile"],
        serde_json::json!("non_developer")
    );
    assert!(evidence["task_entries"].is_array());
    assert!(evidence["data"].is_object());
}
