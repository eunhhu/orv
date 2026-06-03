use super::*;

#[test]
fn benchmark_report_rejects_review_timestamp_before_participant_completion() {
    // Given: otherwise valid review evidence recorded before a participant completed.
    let mut evidence = complete_recorded_evidence();
    evidence["data"]["human_evidence_review"]["reviewed_at"] =
        serde_json::json!("2026-05-18T10:00:00Z");

    // When: creating the benchmark data report.
    let data_report = benchmark_report_data(&evidence, None, None).expect("benchmark data report");

    // Then: the review timestamp is rejected because participant 1 and 2 finish later.
    assert_failed_data(
        &data_report,
        "human_evidence_review.reviewed_at.after_participants",
    );
}

#[test]
fn benchmark_report_rejects_false_human_review_raw_notes() {
    assert_benchmark_report_rejects_false_human_review_bool("raw_notes_reviewed");
}

#[test]
fn benchmark_report_rejects_false_human_review_smoke_output() {
    assert_benchmark_report_rejects_false_human_review_bool("smoke_output_reviewed");
}

#[test]
fn benchmark_report_rejects_false_human_review_participant_identity() {
    assert_benchmark_report_rejects_false_human_review_bool("participant_identity_reviewed");
}

#[test]
fn benchmark_report_rejects_false_human_review_no_ai_assistance() {
    assert_benchmark_report_rejects_false_human_review_bool("no_ai_assistance_confirmed");
}

#[test]
fn verify_deploy_benchmark_evidence_data_rejects_review_timestamp_before_participant_completion() {
    // Given: otherwise structured recorded evidence reviewed before participant completion.
    let mut evidence = complete_recorded_evidence();
    evidence["data"]["human_evidence_review"]["reviewed_at"] =
        serde_json::json!("2026-05-18T10:00:00Z");

    // When: verifying deploy benchmark evidence data.
    let err = verify_deploy_benchmark_evidence_data(&evidence)
        .expect_err("review timestamp earlier than completion must fail");

    // Then: deploy evidence rejects the timestamp order drift.
    assert!(
        err.to_string().contains(
            "deploy benchmark evidence data human_evidence_review reviewed_at must be >= participant_runs[].completed_at"
        ),
        "unexpected error: {err:#}"
    );
}

#[test]
fn benchmark_report_rejects_non_utc_human_review_timestamp() {
    // Given: recorded evidence whose human review timestamp has an offset instead of UTC Z.
    let mut evidence = complete_recorded_evidence();
    evidence["data"]["human_evidence_review"]["reviewed_at"] =
        serde_json::json!("2026-05-18T17:00:00+09:00");

    // When: creating the benchmark data report and verifying deploy evidence.
    let data_report = benchmark_report_data(&evidence, None, None).expect("benchmark data report");
    let err = verify_deploy_benchmark_evidence_data(&evidence)
        .expect_err("non-UTC review timestamp must fail");

    // Then: both paths reject the timestamp format before it can look reviewed.
    assert_failed_data(&data_report, "human_evidence_review.reviewed_at.utc");
    assert!(
        err.to_string().contains(
            "deploy benchmark evidence data human_evidence_review reviewed_at must be null or an RFC3339 UTC timestamp"
        ),
        "unexpected error: {err:#}"
    );
}

#[test]
fn benchmark_report_rejects_non_string_human_review_timestamp() {
    // Given: recorded evidence whose human review timestamp is not a string.
    let mut evidence = complete_recorded_evidence();
    evidence["data"]["human_evidence_review"]["reviewed_at"] = serde_json::json!(1700000000);

    // When: creating the benchmark data report and verifying deploy evidence.
    let data_report = benchmark_report_data(&evidence, None, None).expect("benchmark data report");
    let err = verify_deploy_benchmark_evidence_data(&evidence)
        .expect_err("non-string review timestamp must fail");

    // Then: report and verifier both keep the field out of the pass path.
    assert_failed_data(&data_report, "human_evidence_review.reviewed_at.string");
    assert!(
        err.to_string().contains(
            "deploy benchmark evidence data human_evidence_review reviewed_at must be null or an RFC3339 UTC timestamp"
        ),
        "unexpected error: {err:#}"
    );
}

#[test]
fn benchmark_report_requires_failure_classification_for_failed_participant_run() {
    // Given: recorded human evidence with a failed participant run but no failure class.
    let mut evidence = complete_recorded_evidence();
    evidence["data"]["participant_runs"][0]["status"] = serde_json::json!("failed");
    evidence["data"]["failure_classification"]["primary"] = serde_json::Value::Null;

    // When: creating the benchmark data report.
    let (data_report, status) = benchmark_report_status_for(&evidence);

    // Then: the report names the missing failure classification instead of passing.
    assert_eq!(status, "failed");
    assert_missing_data(&data_report, "failure_classification.primary");
    assert_failed_data(&data_report, "participant_runs.failed");
}

#[test]
fn benchmark_report_rejects_blank_human_review_text_fields() {
    for (field, value) in [("reviewer", " "), ("notes", " ")] {
        // Given: recorded human evidence with a blank human review text field.
        let mut evidence = complete_recorded_evidence();
        evidence["data"]["human_evidence_review"][field] = serde_json::json!(value);

        // When: creating the benchmark data report.
        let (data_report, status) = benchmark_report_status_for(&evidence);

        // Then: the report marks the exact field as missing and keeps the status incomplete.
        assert_eq!(status, "incomplete");
        assert_missing_data(&data_report, &format!("human_evidence_review.{field}"));
    }
}

#[test]
fn verify_deploy_benchmark_evidence_data_rejects_failed_participant_without_classification() {
    // Given: recorded deploy evidence with a failed participant run and no failure class.
    let mut evidence = complete_recorded_evidence();
    evidence["data"]["participant_runs"][0]["status"] = serde_json::json!("failed");
    evidence["data"]["failure_classification"]["primary"] = serde_json::Value::Null;

    // When: verifying deploy benchmark evidence data.
    let err = verify_deploy_benchmark_evidence_data(&evidence)
        .expect_err("failed participant run without failure classification must fail");

    // Then: deploy evidence verifier enforces the same failure-classification gate.
    assert!(
        err.to_string().contains(
            "deploy benchmark evidence data failure_classification primary is required when participant_runs contain failed runs"
        ),
        "unexpected error: {err:#}"
    );
}

#[test]
fn verify_deploy_benchmark_evidence_data_rejects_blank_human_review_reviewer() {
    // Given: otherwise structured recorded evidence with a blank reviewer.
    let mut evidence = complete_recorded_evidence();
    evidence["data"]["human_evidence_review"]["reviewer"] = serde_json::json!(" ");

    // When: verifying deploy benchmark evidence data.
    let err = verify_deploy_benchmark_evidence_data(&evidence)
        .expect_err("blank human review reviewer must fail");

    // Then: the verifier rejects the blank reviewer before it can look recorded.
    assert!(
        err.to_string().contains(
            "deploy benchmark evidence data human_evidence_review reviewer must be a non-empty string"
        ),
        "unexpected error: {err:#}"
    );
}

#[test]
fn verify_deploy_benchmark_evidence_data_rejects_blank_human_review_notes() {
    // Given: otherwise structured recorded evidence with blank notes.
    let mut evidence = complete_recorded_evidence();
    evidence["data"]["human_evidence_review"]["notes"] = serde_json::json!(" ");

    // When: verifying deploy benchmark evidence data.
    let err = verify_deploy_benchmark_evidence_data(&evidence)
        .expect_err("blank human review notes must fail");

    // Then: the verifier rejects the blank notes before they can look recorded.
    assert!(
        err.to_string().contains(
            "deploy benchmark evidence data human_evidence_review notes must be a non-empty string"
        ),
        "unexpected error: {err:#}"
    );
}

#[test]
fn verify_deploy_benchmark_evidence_data_rejects_false_human_review_raw_notes() {
    assert_deploy_verifier_rejects_false_human_review_bool("raw_notes_reviewed");
}

#[test]
fn verify_deploy_benchmark_evidence_data_rejects_false_human_review_smoke_output() {
    assert_deploy_verifier_rejects_false_human_review_bool("smoke_output_reviewed");
}

#[test]
fn verify_deploy_benchmark_evidence_data_rejects_false_human_review_participant_identity() {
    assert_deploy_verifier_rejects_false_human_review_bool("participant_identity_reviewed");
}

#[test]
fn verify_deploy_benchmark_evidence_data_rejects_false_human_review_no_ai_assistance() {
    assert_deploy_verifier_rejects_false_human_review_bool("no_ai_assistance_confirmed");
}

fn assert_benchmark_report_rejects_false_human_review_bool(key: &str) {
    let mut evidence = complete_recorded_evidence();
    evidence["data"]["human_evidence_review"][key] = serde_json::json!(false);

    let (data_report, status) = benchmark_report_status_for(&evidence);

    assert_eq!(status, "failed");
    assert_failed_data(&data_report, &format!("human_evidence_review.{key}"));
}

fn assert_deploy_verifier_rejects_false_human_review_bool(key: &str) {
    let mut evidence = complete_recorded_evidence();
    evidence["data"]["human_evidence_review"][key] = serde_json::json!(false);

    let err = verify_deploy_benchmark_evidence_data(&evidence)
        .expect_err("false human review bool must fail");
    let expected =
        format!("deploy benchmark evidence data human_evidence_review {key} must be true for recorded evidence");
    assert!(
        err.to_string().contains(&expected),
        "unexpected error: {err:#}"
    );
}

fn benchmark_report_status_for(evidence: &serde_json::Value) -> (serde_json::Value, &'static str) {
    let data_report = benchmark_report_data(evidence, None, None).expect("benchmark data report");
    let status = benchmark_report_status_summary(
        &serde_json::json!({
            "failed_tasks": [],
            "missing_tasks": [],
            "total_elapsed_minutes": 100.0,
        }),
        &data_report,
        300.0,
    );
    (data_report, status.status)
}

fn assert_failed_data(data_report: &serde_json::Value, field: &str) {
    assert!(data_report["failed_data"]
        .as_array()
        .expect("failed data")
        .iter()
        .any(|item| item.as_str() == Some(field)));
}

fn assert_missing_data(data_report: &serde_json::Value, field: &str) {
    assert!(data_report["missing_data"]
        .as_array()
        .expect("missing data")
        .iter()
        .any(|item| item.as_str() == Some(field)));
}

fn complete_recorded_evidence() -> serde_json::Value {
    let mut evidence = serde_json::json!({
        "recording_status": "recorded",
        "data": deploy_benchmark::evidence_data_value(),
    });
    fill_benchmark_report_observation_data(&mut evidence);
    fill_benchmark_human_evidence_review(&mut evidence);
    fill_benchmark_participant_runs(&mut evidence);
    evidence
}

fn fill_benchmark_report_observation_data(evidence: &mut serde_json::Value) {
    evidence["data"]["docs_help_lookups"] = serde_json::json!(1);
    evidence["data"]["compiler_runtime_errors"] = serde_json::json!(0);
    evidence["data"]["ai_assistance_used"] = serde_json::json!(false);
    evidence["data"]["generated_artifact_edits"] = serde_json::json!(false);
    evidence["data"]["manual_undocumented_security_steps"] = serde_json::json!(false);
    evidence["data"]["manual_config_edits"] = serde_json::json!([]);
    evidence["data"]["participant_notes"] = serde_json::json!("required observation data");
    evidence["data"]["smoke_test_output"] = serde_json::json!(
        "orv deploy smoke test passed\nbuild_dir=/tmp/orv-build\nbase_url=http://127.0.0.1:8080\ngraph_contract=verified\ndap_summary=verified\ndap_source_bundle=verified\nserver_routes=1\ntrace_stream_requested=1\n"
    );
}

fn fill_benchmark_human_evidence_review(evidence: &mut serde_json::Value) {
    evidence["data"]["human_evidence_review"] = serde_json::json!({
        "reviewer": "benchmark-reviewer",
        "reviewed_at": "2026-05-18T17:00:00Z",
        "raw_notes_reviewed": true,
        "smoke_output_reviewed": true,
        "participant_identity_reviewed": true,
        "no_ai_assistance_confirmed": true,
        "notes": "reviewed retained participant notes, smoke output, participant identities, and no-AI evidence",
    });
}

fn fill_benchmark_participant_runs(evidence: &mut serde_json::Value) {
    evidence["data"]["participant_runs"] = serde_json::json!([
        {
            "run_id": "run-1",
            "participant_id": "participant-1",
            "participant_profile": deploy_benchmark::PARTICIPANT_PROFILE_NON_DEVELOPER,
            "status": "passed",
            "started_at": "2026-05-18T09:00:00Z",
            "completed_at": "2026-05-18T10:30:00Z",
            "raw_notes_artifact": "evidence/participant-1.md",
            "raw_notes_sha256": null,
        },
        {
            "run_id": "run-2",
            "participant_id": "participant-2",
            "participant_profile": deploy_benchmark::PARTICIPANT_PROFILE_NON_DEVELOPER,
            "status": "passed",
            "started_at": "2026-05-18T11:00:00Z",
            "completed_at": "2026-05-18T12:20:00Z",
            "raw_notes_artifact": "evidence/participant-2.md",
            "raw_notes_sha256": null,
        },
    ]);
}
