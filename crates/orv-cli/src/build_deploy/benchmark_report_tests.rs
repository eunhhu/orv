use super::*;

#[test]
fn verify_deploy_benchmark_evidence_data_with_artifacts_rejects_raw_notes_hash_drift() {
    let out = temp_output_dir("verify-benchmark-raw-notes-hash-drift");
    let participant_1_notes = recorded_participant_notes(
        "participant-1",
        "run-1",
        "Completed the shop flow and retained participant one observations.",
    );
    let participant_2_notes = recorded_participant_notes(
        "participant-2",
        "run-2",
        "Completed the shop flow and retained participant two observations.",
    );
    write_participant_note_artifacts(&out, &participant_1_notes, &participant_2_notes);
    let mut evidence = recorded_evidence_with_raw_notes();
    evidence["data"]["participant_runs"][0]["raw_notes_sha256"] = serde_json::json!(
        "sha256:0000000000000000000000000000000000000000000000000000000000000000"
    );
    evidence["data"]["participant_runs"][1]["raw_notes_sha256"] = serde_json::json!(format!(
        "sha256:{}",
        sha256_hex(participant_2_notes.as_bytes())
    ));

    let err = verify_deploy_benchmark_evidence_data_with_artifacts(&evidence, Some(&out))
        .expect_err("raw-notes hash drift must fail deploy evidence verification");

    assert!(
        err.to_string().contains(
            "deploy benchmark evidence data participant_runs[0] raw_notes_sha256 must match retained raw notes"
        ),
        "unexpected error: {err:#}"
    );
    let _ = std::fs::remove_dir_all(out);
}

#[test]
fn verify_deploy_benchmark_evidence_data_with_artifacts_rejects_raw_notes_identity_drift() {
    let out = temp_output_dir("verify-benchmark-raw-notes-identity-drift");
    let participant_1_notes = recorded_participant_notes(
        "participant-2",
        "run-2",
        "Completed the shop flow with mismatched retained identity.",
    );
    let participant_2_notes = recorded_participant_notes(
        "participant-2",
        "run-2",
        "Completed the shop flow and retained participant two observations.",
    );
    write_participant_note_artifacts(&out, &participant_1_notes, &participant_2_notes);
    let mut evidence = recorded_evidence_with_raw_notes();
    evidence["data"]["participant_runs"][0]["raw_notes_sha256"] = serde_json::json!(format!(
        "sha256:{}",
        sha256_hex(participant_1_notes.as_bytes())
    ));
    evidence["data"]["participant_runs"][1]["raw_notes_sha256"] = serde_json::json!(format!(
        "sha256:{}",
        sha256_hex(participant_2_notes.as_bytes())
    ));

    let err = verify_deploy_benchmark_evidence_data_with_artifacts(&evidence, Some(&out))
        .expect_err("raw-notes identity drift must fail deploy evidence verification");

    assert!(
        err.to_string().contains(
            "deploy benchmark evidence data participant_runs[0] raw_notes_artifact participant_id/run_id must match exactly once"
        ),
        "unexpected error: {err:#}"
    );
    let _ = std::fs::remove_dir_all(out);
}

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
        "recording_status": "recorded",
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
fn benchmark_report_rejects_smoke_required_marker_drift() {
    let mut evidence = serde_json::json!({
        "recording_status": "recorded",
        "data": deploy_benchmark::evidence_data_value(),
    });
    fill_benchmark_report_observation_data(&mut evidence);
    fill_benchmark_human_evidence_review(&mut evidence);
    fill_benchmark_participant_runs(&mut evidence);
    evidence["data"]["smoke_test_required_markers"] =
        serde_json::json!(["pass_marker", "build_dir"]);

    let data_report = benchmark_report_data(&evidence, None, None).expect("benchmark data report");
    let status = benchmark_report_status_summary(
        &serde_json::json!({
            "failed_tasks": [],
            "missing_tasks": [],
            "total_elapsed_minutes": 100.0,
        }),
        &data_report,
        300.0,
    );

    assert_eq!(status.status, "failed");
    assert!(data_report["failed_data"]
        .as_array()
        .expect("failed data")
        .iter()
        .any(|item| item == "smoke_test_required_markers.contract"));
}

#[test]
fn benchmark_report_rejects_malformed_raw_notes_sha256_format() {
    let out = temp_output_dir("benchmark-report-malformed-raw-notes-sha");
    let participant_1_notes = recorded_participant_notes(
        "participant-1",
        "run-1",
        "Completed the shop flow with retained participant one observations.",
    );
    let participant_2_notes = recorded_participant_notes(
        "participant-2",
        "run-2",
        "Completed the shop flow with retained participant two observations.",
    );
    write_participant_note_artifacts(&out, &participant_1_notes, &participant_2_notes);
    let mut evidence = recorded_evidence_with_raw_notes();
    evidence["data"]["participant_runs"][0]["raw_notes_sha256"] =
        serde_json::json!("sha256:not-a-hex-digest");
    evidence["data"]["participant_runs"][1]["raw_notes_sha256"] = serde_json::json!(format!(
        "sha256:{}",
        sha256_hex(participant_2_notes.as_bytes())
    ));

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
    let verifier_error =
        verify_deploy_benchmark_evidence_data_with_artifacts(&evidence, Some(&out))
            .expect_err("malformed raw-notes sha256 must fail deploy evidence verification");

    assert_eq!(status.status, "incomplete");
    assert!(data_report["missing_data"]
        .as_array()
        .expect("missing data")
        .iter()
        .any(|item| item == "participant_runs[0].raw_notes_sha256.format"));
    assert!(
        verifier_error.to_string().contains(
            "deploy benchmark evidence data participant_runs[0] raw_notes_sha256 must be null or sha256:<64 lowercase hex>"
        ),
        "unexpected error: {verifier_error:#}"
    );
    let _ = std::fs::remove_dir_all(out);
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

fn recorded_evidence_with_raw_notes() -> serde_json::Value {
    let mut evidence = serde_json::json!({
        "recording_status": "recorded",
        "data": deploy_benchmark::evidence_data_value(),
    });
    fill_benchmark_report_observation_data(&mut evidence);
    fill_benchmark_human_evidence_review(&mut evidence);
    fill_benchmark_participant_runs(&mut evidence);
    evidence
}

fn write_participant_note_artifacts(
    out: &Path,
    participant_1_notes: &str,
    participant_2_notes: &str,
) {
    let evidence_dir = out.join("evidence");
    std::fs::create_dir_all(&evidence_dir).expect("create participant evidence dir");
    std::fs::write(evidence_dir.join("participant-1.md"), participant_1_notes)
        .expect("write participant 1 notes");
    std::fs::write(evidence_dir.join("participant-2.md"), participant_2_notes)
        .expect("write participant 2 notes");
}

fn recorded_participant_notes(participant_id: &str, run_id: &str, task_notes: &str) -> String {
    format!(
        "# Shop Benchmark Participant Notes\n\n## Participant\n\n- participant_id: {participant_id}\n- run_id: {run_id}\n- started_at: 2026-05-18T09:00:00Z\n- completed_at: 2026-05-18T10:30:00Z\n\n## Task Notes\n\n{task_notes}\n"
    )
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
