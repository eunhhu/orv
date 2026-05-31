use super::*;

#[test]
fn benchmark_report_rejects_duplicate_raw_notes_identity_fields() {
    // Given: two retained participant raw-notes artifacts and recorded benchmark evidence rows.
    let out = temp_output_dir("benchmark-report-duplicate-identity");
    let evidence_dir = out.join("evidence");
    std::fs::create_dir_all(&evidence_dir).expect("create participant evidence dir");
    let participant_1_notes = "# Shop Benchmark Participant Notes\n\n## Participant\n\n- participant_id: participant-1\n- run_id: run-1\n- participant_id: participant-2\n- run_id: run-2\n- started_at: 2026-05-18T09:00:00Z\n- completed_at: 2026-05-18T10:30:00Z\n\n## Task Notes\n\nCompleted the shop flow and retained real observations.\n";
    let participant_2_notes = "# Shop Benchmark Participant Notes\n\n## Participant\n\n- participant_id: participant-2\n- run_id: run-2\n- started_at: 2026-05-18T11:00:00Z\n- completed_at: 2026-05-18T12:20:00Z\n\n## Task Notes\n\nCompleted the shop flow and retained real observations.\n";
    std::fs::write(evidence_dir.join("participant-1.md"), participant_1_notes)
        .expect("write duplicate-identity participant notes");
    std::fs::write(evidence_dir.join("participant-2.md"), participant_2_notes)
        .expect("write matched participant notes");
    let mut evidence = serde_json::json!({
        "data": deploy_benchmark::evidence_data_value(),
    });
    fill_benchmark_report_observation_data(&mut evidence);
    fill_benchmark_participant_runs(&mut evidence);
    evidence["data"]["participant_runs"][0]["raw_notes_sha256"] = serde_json::json!(format!(
        "sha256:{}",
        sha256_hex(participant_1_notes.as_bytes())
    ));
    evidence["data"]["participant_runs"][1]["raw_notes_sha256"] = serde_json::json!(format!(
        "sha256:{}",
        sha256_hex(participant_2_notes.as_bytes())
    ));

    // When: creating benchmark data/status reports from that evidence set.
    let data_report =
        benchmark_report_data(&evidence, Some(&out), None).expect("benchmark data report");
    let status = benchmark_report_status_summary(
        &serde_json::json!({
            "failed_tasks": [],
            "missing_tasks": [],
            "total_elapsed_minutes": 100.0,
        }),
        &data_report,
        300.0,
    );

    // Then: duplicate identity fields keep report status incomplete and flag artifact mismatch.
    assert_eq!(status.status, "incomplete");
    assert!(data_report["missing_data"]
        .as_array()
        .expect("missing data")
        .iter()
        .any(|item| item == "participant_runs[0].raw_notes_artifact.identity_match"));
    assert_eq!(
        data_report["participant_raw_notes_artifacts"][0]["identity_match"],
        false
    );
    assert_eq!(
        data_report["participant_raw_notes_artifacts"][1]["identity_match"],
        true
    );

    let _ = std::fs::remove_dir_all(out);
}

#[test]
fn benchmark_report_rejects_review_timestamp_before_participant_completion() {
    // Given: otherwise valid review evidence recorded before a participant completed.
    let mut evidence = serde_json::json!({
        "recording_status": "recorded",
        "data": deploy_benchmark::evidence_data_value(),
    });
    fill_benchmark_report_observation_data(&mut evidence);
    fill_benchmark_human_evidence_review(&mut evidence);
    fill_benchmark_participant_runs(&mut evidence);
    evidence["data"]["human_evidence_review"]["reviewed_at"] =
        serde_json::json!("2026-05-18T10:00:00Z");

    // When: creating the benchmark data report.
    let data_report = benchmark_report_data(&evidence, None, None).expect("benchmark data report");

    // Then: the review timestamp is rejected because participant 1 and 2 finish later.
    assert!(data_report["failed_data"]
        .as_array()
        .expect("failed data")
        .iter()
        .any(|item| item == "human_evidence_review.reviewed_at.after_participants"));
}

#[test]
fn benchmark_report_fails_smoke_output_artifact_mismatch() {
    // Given: evidence copied from one smoke run and a different retained smoke-output artifact.
    let out = temp_output_dir("benchmark-report-smoke-output-mismatch");
    let deploy_dir = out.join("deploy");
    std::fs::create_dir_all(&deploy_dir).expect("create deploy dir");
    std::fs::write(
        deploy_dir.join("smoke-output.txt"),
        "orv deploy smoke test passed\nbuild_dir=/tmp/other-build\nbase_url=http://127.0.0.1:8080\ngraph_contract=verified\ndap_summary=verified\ndap_source_bundle=verified\nserver_routes=1\ntrace_stream_requested=1\n",
    )
    .expect("write smoke-output artifact");

    let mut evidence = serde_json::json!({
        "recording_status": "recorded",
        "data": deploy_benchmark::evidence_data_value(),
    });
    fill_benchmark_report_observation_data(&mut evidence);
    fill_benchmark_human_evidence_review(&mut evidence);
    fill_benchmark_participant_runs(&mut evidence);

    // When: creating benchmark data/status reports with both copied output and artifact present.
    let data_report = benchmark_report_data(&evidence, Some(&out), Some("deploy/smoke-output.txt"))
        .expect("benchmark data report");
    let status = benchmark_report_status_summary(
        &serde_json::json!({
            "failed_tasks": [],
            "missing_tasks": [],
            "total_elapsed_minutes": 100.0,
        }),
        &data_report,
        300.0,
    );

    // Then: contradictory smoke evidence is a failed gate, not an incomplete field.
    assert_eq!(status.status, "failed");
    assert_eq!(data_report["smoke_test_output_artifact_match"], false);
    assert!(data_report["failed_data"]
        .as_array()
        .expect("failed data")
        .iter()
        .any(|item| item == "smoke_test_output.artifact_match"));
    assert!(!data_report["missing_data"]
        .as_array()
        .expect("missing data")
        .iter()
        .any(|item| item == "smoke_test_output.artifact_match"));

    let _ = std::fs::remove_dir_all(out);
}

#[test]
fn verify_deploy_benchmark_evidence_data_rejects_review_timestamp_before_participant_completion() {
    let mut evidence = serde_json::json!({
        "data": deploy_benchmark::evidence_data_value(),
    });
    fill_benchmark_report_observation_data(&mut evidence);
    fill_benchmark_human_evidence_review(&mut evidence);
    fill_benchmark_participant_runs(&mut evidence);
    evidence["data"]["human_evidence_review"]["reviewed_at"] =
        serde_json::json!("2026-05-18T10:00:00Z");

    let err = verify_deploy_benchmark_evidence_data(&evidence)
        .expect_err("review timestamp earlier than completion must fail");
    assert!(
        err.to_string().contains(
            "deploy benchmark evidence data human_evidence_review reviewed_at must be >= participant_runs[].completed_at"
        ),
        "unexpected error: {err:#}"
    );
}

fn temp_output_dir(name: &str) -> PathBuf {
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock after unix epoch")
        .as_nanos();
    let mut path = std::env::temp_dir();
    path.push(format!("orv-cli-{name}-{}-{unique}", std::process::id()));
    let _ = std::fs::remove_dir_all(&path);
    path
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
