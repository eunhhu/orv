use super::*;

#[test]
fn benchmark_prepare_subcommand_is_accepted() {
    let parsed = Cli::try_parse_from([
        "orv",
        "benchmark-prepare",
        "target/orv-build-test",
        "--participants",
        "3",
    ]);
    if let Err(err) = parsed {
        panic!("{}", err.render());
    }
}

#[test]
fn benchmark_report_marks_unrecorded_evidence_incomplete() {
    let (src_dir, path) = prod_server_source("benchmark-report-incomplete-source");
    let out = temp_output_dir("benchmark-report-incomplete");

    cmd_build_with_profile(&path, &out, BuildProfile::Production).expect("prod build");

    let report = benchmark_report_value(&out).expect("benchmark report");

    assert_eq!(report["kind"], "orv.benchmark.shop_5h.report");
    assert_eq!(report["status"], "incomplete");
    assert_eq!(report["contract_verified"], true);
    assert_eq!(report["evidence"], "deploy/benchmark-evidence.json");
    assert_eq!(report["preflight"], "deploy/preflight.json");
    assert_eq!(report["max_elapsed_minutes"], 300.0);
    assert_eq!(report["tasks"]["task_count"], 10);
    assert_eq!(report["tasks"]["recorded_task_count"], 0);
    assert_eq!(report["tasks"]["missing_task_count"], 10);
    assert!(report["data"]["missing_data"]
        .as_array()
        .expect("missing data")
        .iter()
        .any(|item| item == "docs_help_lookups"));
    assert!(report["data"]["missing_data"]
        .as_array()
        .expect("missing data")
        .iter()
        .any(|item| item == "smoke_test_output"));
    assert!(cmd_benchmark_report(&out, true)
        .expect_err("require pass rejects incomplete")
        .to_string()
        .contains("benchmark report status must be passed"));
    let _ = std::fs::remove_dir_all(src_dir);
    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn benchmark_prepare_seeds_participant_note_artifacts() {
    let (src_dir, path) = prod_server_source("benchmark-prepare-source");
    let out = temp_output_dir("benchmark-prepare");

    cmd_build_with_profile(&path, &out, BuildProfile::Production).expect("prod build");

    let prepared =
        benchmark_prepare_participants_value(&out, 3).expect("prepare participant evidence");

    assert_eq!(prepared["kind"], "orv.benchmark.shop_5h.prepare");
    assert_eq!(prepared["participants_requested"], 3);
    assert_eq!(prepared["participants_total"], 3);
    assert_eq!(
        prepared["participant_notes_template"],
        "deploy/participant-notes-template.md"
    );
    assert_eq!(
        prepared["recording_handoff"]["evidence"],
        "deploy/benchmark-evidence.json"
    );
    assert_eq!(
        prepared["recording_handoff"]["recording_status"],
        "not_recorded"
    );
    assert_eq!(
        prepared["recording_handoff"]["set_recording_status_after_human_run"],
        "recorded"
    );
    assert_eq!(
        prepared["recording_handoff"]["require_pass_command"],
        "orv benchmark-report . --require-pass"
    );
    assert!(prepared["recording_handoff"]["fields_to_record"]
        .as_array()
        .expect("fields to record")
        .iter()
        .any(|item| item == "participant run metadata"));
    assert!(prepared["recording_handoff"]["participant_run_fields"]
        .as_array()
        .expect("participant run fields")
        .iter()
        .any(|item| item == "raw_notes_artifact"));
    assert!(prepared["recording_handoff"]["participant_run_fields"]
        .as_array()
        .expect("participant run fields")
        .iter()
        .any(|item| item == "raw_notes_sha256"));
    assert_eq!(
        prepared["raw_notes_artifacts"][0]["path"],
        "deploy/evidence/participant-1.md"
    );
    assert_eq!(prepared["raw_notes_artifacts"][0]["created"], true);
    let note = std::fs::read_to_string(out.join("deploy/evidence/participant-1.md"))
        .expect("participant note");
    assert!(note.contains("- participant_id: participant-1"));
    assert!(note.contains("- run_id: run-1"));

    let evidence =
        read_json_value(&out.join("deploy/benchmark-evidence.json")).expect("benchmark evidence");
    assert_eq!(
        evidence["data"]["participant_runs"]
            .as_array()
            .unwrap()
            .len(),
        3
    );
    assert_eq!(
        evidence["data"]["participant_runs"][0]["status"],
        serde_json::json!("todo")
    );
    assert_eq!(
        evidence["data"]["participant_runs"][2]["raw_notes_artifact"],
        serde_json::json!("deploy/evidence/participant-3.md")
    );
    assert!(evidence["data"]["participant_runs"][2]["raw_notes_sha256"].is_null());
    cmd_verify_build(&out).expect("prepared benchmark evidence remains build-valid");

    let prepared_again =
        benchmark_prepare_participants_value(&out, 3).expect("prepare participant evidence again");
    assert_eq!(
        prepared_again["raw_notes_artifacts"][0]["created"],
        serde_json::json!(false)
    );

    let _ = std::fs::remove_dir_all(src_dir);
    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn benchmark_report_marks_missing_participant_runs_incomplete() {
    let mut evidence = serde_json::json!({
        "data": deploy_benchmark::evidence_data_value(),
    });
    evidence["data"]["docs_help_lookups"] = serde_json::json!(1);
    evidence["data"]["compiler_runtime_errors"] = serde_json::json!(0);
    evidence["data"]["manual_config_edits"] = serde_json::json!([]);
    evidence["data"]["participant_notes"] = serde_json::json!("human run notes retained");
    evidence["data"]["smoke_test_output"] = serde_json::json!(
        "orv deploy smoke test passed\nbuild_dir=/tmp/orv-build\nbase_url=http://127.0.0.1:8080\ngraph_contract=verified\ndap_summary=verified\ndap_source_bundle=verified\nserver_routes=1\ntrace_stream_requested=1\n"
    );

    let data_report = benchmark_report_data(&evidence, None, None).expect("benchmark data report");

    assert!(data_report["missing_data"]
        .as_array()
        .expect("missing data")
        .iter()
        .any(|item| item == "participant_runs.minimum"));
    assert_eq!(
        data_report["participant_summary"]["recommended_minimum"],
        serde_json::json!(2)
    );
    assert_eq!(
        data_report["participant_summary"]["recorded_run_count"],
        serde_json::json!(0)
    );
}

#[test]
fn benchmark_report_marks_failed_participant_run_failed() {
    let mut evidence = serde_json::json!({
        "data": deploy_benchmark::evidence_data_value(),
    });
    evidence["data"]["docs_help_lookups"] = serde_json::json!(1);
    evidence["data"]["compiler_runtime_errors"] = serde_json::json!(0);
    evidence["data"]["manual_config_edits"] = serde_json::json!([]);
    evidence["data"]["participant_notes"] = serde_json::json!("one participant blocked");
    evidence["data"]["smoke_test_output"] = serde_json::json!(
        "orv deploy smoke test passed\nbuild_dir=/tmp/orv-build\nbase_url=http://127.0.0.1:8080\ngraph_contract=verified\ndap_summary=verified\ndap_source_bundle=verified\nserver_routes=1\ntrace_stream_requested=1\n"
    );
    fill_benchmark_participant_runs(&mut evidence);
    evidence["data"]["participant_runs"][1]["status"] = serde_json::json!("failed");
    evidence["data"]["failure_classification"]["primary"] = serde_json::json!("documentation");

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
        .any(|item| item == "participant_runs.failed"));
}

#[test]
fn benchmark_report_requires_failure_classification_for_failed_tasks() {
    let mut evidence = serde_json::json!({
        "task_entries": deploy_benchmark::evidence_task_entries_value(),
        "data": deploy_benchmark::evidence_data_value(),
    });
    fill_benchmark_task_entries(&mut evidence);
    evidence["task_entries"][0]["status"] = serde_json::json!("failed");
    evidence["data"]["docs_help_lookups"] = serde_json::json!(1);
    evidence["data"]["compiler_runtime_errors"] = serde_json::json!(0);
    evidence["data"]["manual_config_edits"] = serde_json::json!([]);
    evidence["data"]["participant_notes"] = serde_json::json!("task failed");
    evidence["data"]["smoke_test_output"] = serde_json::json!(
        "orv deploy smoke test passed\nbuild_dir=/tmp/orv-build\nbase_url=http://127.0.0.1:8080\ngraph_contract=verified\ndap_summary=verified\ndap_source_bundle=verified\nserver_routes=1\ntrace_stream_requested=1\n"
    );
    fill_benchmark_participant_runs(&mut evidence);

    let task_report = benchmark_report_tasks(&evidence, 300.0).expect("benchmark task report");
    let mut data_report =
        benchmark_report_data(&evidence, None, None).expect("benchmark data report");
    benchmark_report_apply_failure_classification_requirement(&task_report, &mut data_report);

    assert!(data_report["missing_data"]
        .as_array()
        .expect("missing data")
        .iter()
        .any(|item| item == "failure_classification.primary"));

    evidence["data"]["failure_classification"]["primary"] = serde_json::json!("syntax");
    let mut data_report =
        benchmark_report_data(&evidence, None, None).expect("benchmark data report");
    benchmark_report_apply_failure_classification_requirement(&task_report, &mut data_report);

    assert!(!data_report["missing_data"]
        .as_array()
        .expect("missing data")
        .iter()
        .any(|item| item == "failure_classification.primary"));
}

#[test]
fn benchmark_report_rejects_failure_classification_category_drift() {
    let mut evidence = serde_json::json!({
        "data": deploy_benchmark::evidence_data_value(),
    });
    evidence["data"]["failure_classification"]["primary"] = serde_json::json!("custom");
    evidence["data"]["failure_classification"]["allowed_categories"] =
        serde_json::json!(["custom"]);

    let err = benchmark_report_data(&evidence, None, None)
        .expect_err("failure classification category drift must fail");

    assert!(
        err.to_string().contains(
            "benchmark evidence data failure_classification allowed_categories must match benchmark contract"
        ),
        "{err:?}"
    );

    evidence["data"]["failure_classification"]["allowed_categories"] =
        serde_json::json!(deploy_benchmark::FAILURE_CLASSIFICATION_CATEGORIES);

    let err = benchmark_report_data(&evidence, None, None)
        .expect_err("unknown failure classification primary must fail");

    assert!(
        err.to_string().contains(
            "benchmark evidence data failure_classification primary must be an allowed category"
        ),
        "{err:?}"
    );
}

#[test]
fn benchmark_report_requires_notes_for_other_failure_classification() {
    let mut evidence = serde_json::json!({
        "data": deploy_benchmark::evidence_data_value(),
    });
    fill_benchmark_report_observation_data(&mut evidence);
    evidence["data"]["failure_classification"]["primary"] = serde_json::json!("other");
    evidence["data"]["failure_classification"]["notes"] = serde_json::json!("");

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

    assert_eq!(status.status, "incomplete");
    assert!(data_report["missing_data"]
        .as_array()
        .expect("missing data")
        .iter()
        .any(|item| item == "failure_classification.notes"));

    evidence["data"]["failure_classification"]["notes"] =
        serde_json::json!("failure did not fit the fixed categories");
    let data_report = benchmark_report_data(&evidence, None, None).expect("benchmark data report");
    assert!(!data_report["missing_data"]
        .as_array()
        .expect("missing data")
        .iter()
        .any(|item| item == "failure_classification.notes"));
}

#[test]
fn benchmark_report_requires_recording_status_recorded_before_pass() {
    let mut evidence = serde_json::json!({
        "recording_status": "sample",
        "task_entries": deploy_benchmark::evidence_task_entries_value(),
        "data": deploy_benchmark::evidence_data_value(),
    });
    fill_benchmark_task_entries(&mut evidence);
    evidence["data"]["docs_help_lookups"] = serde_json::json!(1);
    evidence["data"]["compiler_runtime_errors"] = serde_json::json!(0);
    evidence["data"]["manual_config_edits"] = serde_json::json!([]);
    evidence["data"]["participant_notes"] = serde_json::json!("sample evidence only");
    evidence["data"]["smoke_test_output"] = serde_json::json!(
        "orv deploy smoke test passed\nbuild_dir=/tmp/orv-build\nbase_url=http://127.0.0.1:8080\ngraph_contract=verified\ndap_summary=verified\ndap_source_bundle=verified\nserver_routes=1\ntrace_stream_requested=1\n"
    );
    fill_benchmark_participant_runs(&mut evidence);

    let task_report = benchmark_report_tasks(&evidence, 300.0).expect("benchmark task report");
    let mut data_report =
        benchmark_report_data(&evidence, None, None).expect("benchmark data report");
    benchmark_report_apply_recording_status_requirement(&evidence, &mut data_report);
    let status = benchmark_report_status_summary(&task_report, &data_report, 300.0);

    assert_eq!(status.status, "incomplete");
    assert!(data_report["missing_data"]
        .as_array()
        .expect("missing data")
        .iter()
        .any(|item| item == "recording_status.recorded"));

    evidence["recording_status"] = serde_json::json!("recorded");
    let mut data_report =
        benchmark_report_data(&evidence, None, None).expect("benchmark data report");
    benchmark_report_apply_recording_status_requirement(&evidence, &mut data_report);

    assert!(!data_report["missing_data"]
        .as_array()
        .expect("missing data")
        .iter()
        .any(|item| item == "recording_status.recorded"));
}

#[test]
fn benchmark_report_requires_human_evidence_review_before_pass() {
    let mut evidence = serde_json::json!({
        "recording_status": "recorded",
        "task_entries": deploy_benchmark::evidence_task_entries_value(),
        "data": deploy_benchmark::evidence_data_value(),
    });
    fill_benchmark_task_entries(&mut evidence);
    fill_benchmark_report_observation_data(&mut evidence);
    evidence["data"]["human_evidence_review"] = serde_json::json!({
        "reviewer": "",
        "reviewed_at": null,
        "raw_notes_reviewed": null,
        "smoke_output_reviewed": false,
        "participant_identity_reviewed": true,
        "no_ai_assistance_confirmed": true,
        "notes": "",
    });

    let task_report = benchmark_report_tasks(&evidence, 300.0).expect("benchmark task report");
    let data_report = benchmark_report_data(&evidence, None, None).expect("benchmark data report");
    let status = benchmark_report_status_summary(&task_report, &data_report, 300.0);

    assert_eq!(status.status, "failed");
    assert!(data_report["missing_data"]
        .as_array()
        .expect("missing data")
        .iter()
        .any(|item| item == "human_evidence_review.reviewer"));
    assert!(data_report["missing_data"]
        .as_array()
        .expect("missing data")
        .iter()
        .any(|item| item == "human_evidence_review.reviewed_at"));
    assert!(data_report["missing_data"]
        .as_array()
        .expect("missing data")
        .iter()
        .any(|item| item == "human_evidence_review.raw_notes_reviewed"));
    assert!(data_report["failed_data"]
        .as_array()
        .expect("failed data")
        .iter()
        .any(|item| item == "human_evidence_review.smoke_output_reviewed"));

    fill_benchmark_human_evidence_review(&mut evidence);
    let data_report = benchmark_report_data(&evidence, None, None).expect("benchmark data report");
    assert!(!data_report["missing_data"]
        .as_array()
        .expect("missing data")
        .iter()
        .any(|item| item
            .as_str()
            .is_some_and(|item| { item.starts_with("human_evidence_review") })));
    assert!(!data_report["failed_data"]
        .as_array()
        .expect("failed data")
        .iter()
        .any(|item| item
            .as_str()
            .is_some_and(|item| { item.starts_with("human_evidence_review") })));
}

#[test]
fn shop_benchmark_sample_evidence_matches_current_contract() {
    let sample_path = workspace_path(&["docs", "samples", "shop-benchmark-evidence.sample.json"]);
    let sample = read_json_value(&sample_path).expect("shop benchmark sample evidence");

    verify_deploy_benchmark_evidence_task_entries(&sample).expect("sample task entries contract");
    verify_deploy_benchmark_evidence_data(&sample).expect("sample data contract");
    assert_eq!(sample["recording_status"], serde_json::json!("sample"));
}

#[test]
fn benchmark_report_rejects_participant_count_drift() {
    let mut evidence = serde_json::json!({
        "data": deploy_benchmark::evidence_data_value(),
    });
    evidence["data"]["recommended_participant_count"]["minimum"] = serde_json::json!(1);

    let err = benchmark_report_data(&evidence, None, None)
        .expect_err("participant count drift must fail");

    assert!(
        err.to_string().contains(
            "benchmark evidence data recommended_participant_count must match benchmark contract"
        ),
        "{err:?}"
    );
}

#[test]
fn benchmark_report_fails_negative_observation_counts() {
    let mut evidence = serde_json::json!({
        "data": deploy_benchmark::evidence_data_value(),
    });
    fill_benchmark_report_observation_data(&mut evidence);
    evidence["data"]["docs_help_lookups"] = serde_json::json!(-1);
    evidence["data"]["compiler_runtime_errors"] = serde_json::json!("one");

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
        .any(|item| item == "docs_help_lookups.non_negative_integer"));
    assert!(data_report["failed_data"]
        .as_array()
        .expect("failed data")
        .iter()
        .any(|item| item == "compiler_runtime_errors.non_negative_integer"));
}

#[test]
fn benchmark_report_requires_manual_config_edit_entries() {
    let mut evidence = serde_json::json!({
        "data": deploy_benchmark::evidence_data_value(),
    });
    fill_benchmark_report_observation_data(&mut evidence);
    evidence["data"]["manual_config_edits"] = serde_json::json!([""]);

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

    assert_eq!(status.status, "incomplete");
    assert!(data_report["missing_data"]
        .as_array()
        .expect("missing data")
        .iter()
        .any(|item| item == "manual_config_edits[0].non_empty"));

    evidence["data"]["manual_config_edits"] = serde_json::json!([42]);
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
        .any(|item| item == "manual_config_edits[0].string"));
}

#[test]
fn benchmark_report_fails_negative_time_values() {
    let mut evidence = serde_json::json!({
        "task_entries": deploy_benchmark::evidence_task_entries_value(),
        "data": deploy_benchmark::evidence_data_value(),
    });
    fill_benchmark_task_entries(&mut evidence);
    evidence["task_entries"][0]["elapsed_minutes"] = serde_json::json!(-1.0);
    fill_benchmark_report_observation_data(&mut evidence);
    evidence["data"]["compiler_runtime_errors"] = serde_json::json!(1);
    evidence["data"]["first_error_to_fix_minutes"] = serde_json::json!(-0.5);

    let task_report = benchmark_report_tasks(&evidence, 300.0).expect("benchmark task report");
    let data_report = benchmark_report_data(&evidence, None, None).expect("benchmark data report");
    let status = benchmark_report_status_summary(&task_report, &data_report, 300.0);

    assert_eq!(status.status, "failed");
    assert_eq!(task_report["recorded_task_count"], serde_json::json!(9));
    assert_eq!(
        task_report["missing_tasks"][0]["invalid_elapsed_minutes"],
        serde_json::json!(true)
    );
    assert!(task_report["failed_tasks"]
        .as_array()
        .expect("failed tasks")
        .iter()
        .any(|item| item["invalid_elapsed_minutes"] == true));
    assert!(data_report["failed_data"]
        .as_array()
        .expect("failed data")
        .iter()
        .any(|item| item == "first_error_to_fix_minutes.non_negative_number"));
}

#[test]
fn benchmark_report_marks_invalid_participant_timestamp_incomplete() {
    let mut evidence = serde_json::json!({
        "data": deploy_benchmark::evidence_data_value(),
    });
    fill_benchmark_report_observation_data(&mut evidence);
    evidence["data"]["participant_runs"][0]["started_at"] =
        serde_json::json!("2026-05-18T10:00:00Z");
    evidence["data"]["participant_runs"][0]["completed_at"] =
        serde_json::json!("2026-05-18T09:00:00Z");

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

    assert_eq!(status.status, "incomplete");
    assert!(data_report["missing_data"]
        .as_array()
        .expect("missing data")
        .iter()
        .any(|item| item == "participant_runs[0].completed_at.order"));
    assert_eq!(
        data_report["participant_summary"]["recorded_run_count"],
        serde_json::json!(1)
    );
}

#[test]
fn benchmark_report_marks_duplicate_participant_identity_incomplete() {
    let mut evidence = serde_json::json!({
        "data": deploy_benchmark::evidence_data_value(),
    });
    fill_benchmark_report_observation_data(&mut evidence);
    evidence["data"]["participant_runs"][1]["participant_id"] = serde_json::json!("participant-1");

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

    assert_eq!(status.status, "incomplete");
    assert!(data_report["missing_data"]
        .as_array()
        .expect("missing data")
        .iter()
        .any(|item| item == "participant_runs[1].participant_id.unique"));
    assert_eq!(
        data_report["participant_summary"]["recorded_run_count"],
        serde_json::json!(1)
    );
}

#[test]
fn benchmark_report_requires_ai_assistance_evidence_before_pass() {
    let mut evidence = serde_json::json!({
        "data": deploy_benchmark::evidence_data_value(),
    });
    fill_benchmark_report_observation_data(&mut evidence);
    evidence["data"]["ai_assistance_used"] = serde_json::Value::Null;
    evidence["data"]["participant_notes"] = serde_json::json!("ai usage must be recorded");

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

    assert_eq!(status.status, "incomplete");
    assert!(data_report["missing_data"]
        .as_array()
        .expect("missing data")
        .iter()
        .any(|item| item == "ai_assistance_used"));
}

#[test]
fn benchmark_report_requires_manual_failure_gate_evidence_before_pass() {
    let mut evidence = serde_json::json!({
        "data": deploy_benchmark::evidence_data_value(),
    });
    fill_benchmark_report_observation_data(&mut evidence);
    evidence["data"]["generated_artifact_edits"] = serde_json::Value::Null;
    evidence["data"]["manual_undocumented_security_steps"] = serde_json::Value::Null;
    evidence["data"]["participant_notes"] =
        serde_json::json!("manual failure gates must be recorded");

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

    assert_eq!(status.status, "incomplete");
    assert!(data_report["missing_data"]
        .as_array()
        .expect("missing data")
        .iter()
        .any(|item| item == "generated_artifact_edits"));
    assert!(data_report["missing_data"]
        .as_array()
        .expect("missing data")
        .iter()
        .any(|item| item == "manual_undocumented_security_steps"));
}

#[test]
fn benchmark_report_fails_when_ai_assistance_was_used() {
    let mut evidence = serde_json::json!({
        "data": deploy_benchmark::evidence_data_value(),
    });
    fill_benchmark_report_observation_data(&mut evidence);
    evidence["data"]["ai_assistance_used"] = serde_json::json!(true);
    evidence["data"]["participant_notes"] = serde_json::json!("ai assistance was used");

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
        .any(|item| item == "ai_assistance_used"));
}

#[test]
fn benchmark_report_fails_when_manual_failure_gate_is_triggered() {
    for key in [
        "generated_artifact_edits",
        "manual_undocumented_security_steps",
    ] {
        let mut evidence = serde_json::json!({
            "data": deploy_benchmark::evidence_data_value(),
        });
        fill_benchmark_report_observation_data(&mut evidence);
        evidence["data"][key] = serde_json::json!(true);

        let data_report =
            benchmark_report_data(&evidence, None, None).expect("benchmark data report");
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
            .any(|item| item == key));
    }
}

#[test]
fn benchmark_report_marks_wrong_participant_profile_incomplete() {
    let mut evidence = serde_json::json!({
        "data": deploy_benchmark::evidence_data_value(),
    });
    evidence["data"]["docs_help_lookups"] = serde_json::json!(1);
    evidence["data"]["compiler_runtime_errors"] = serde_json::json!(0);
    evidence["data"]["manual_config_edits"] = serde_json::json!([]);
    evidence["data"]["participant_notes"] =
        serde_json::json!("developer participant is not target evidence");
    evidence["data"]["smoke_test_output"] = serde_json::json!(
        "orv deploy smoke test passed\nbuild_dir=/tmp/orv-build\nbase_url=http://127.0.0.1:8080\ngraph_contract=verified\ndap_summary=verified\ndap_source_bundle=verified\nserver_routes=1\ntrace_stream_requested=1\n"
    );
    fill_benchmark_participant_runs(&mut evidence);
    evidence["data"]["participant_runs"][0]["participant_profile"] = serde_json::json!("developer");

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

    assert_eq!(status.status, "incomplete");
    assert!(data_report["missing_data"]
        .as_array()
        .expect("missing data")
        .iter()
        .any(|item| item == "participant_runs[0].participant_profile.allowed"));
    assert_eq!(
        data_report["participant_summary"]["recorded_run_count"],
        serde_json::json!(1)
    );
}

#[test]
fn benchmark_report_marks_unknown_participant_status_incomplete() {
    let mut evidence = serde_json::json!({
        "data": deploy_benchmark::evidence_data_value(),
    });
    evidence["data"]["docs_help_lookups"] = serde_json::json!(1);
    evidence["data"]["compiler_runtime_errors"] = serde_json::json!(0);
    evidence["data"]["manual_config_edits"] = serde_json::json!([]);
    evidence["data"]["participant_notes"] = serde_json::json!("unknown status is not evidence");
    evidence["data"]["smoke_test_output"] = serde_json::json!(
        "orv deploy smoke test passed\nbuild_dir=/tmp/orv-build\nbase_url=http://127.0.0.1:8080\ngraph_contract=verified\ndap_summary=verified\ndap_source_bundle=verified\nserver_routes=1\ntrace_stream_requested=1\n"
    );
    fill_benchmark_participant_runs(&mut evidence);
    evidence["data"]["participant_runs"][0]["status"] = serde_json::json!("maybe");

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

    assert_eq!(status.status, "incomplete");
    assert!(data_report["missing_data"]
        .as_array()
        .expect("missing data")
        .iter()
        .any(|item| item == "participant_runs[0].status.allowed"));
    assert_eq!(
        data_report["participant_summary"]["recorded_run_count"],
        serde_json::json!(1)
    );
}

#[test]
fn benchmark_report_marks_unknown_task_status_incomplete() {
    let mut evidence = serde_json::json!({
        "task_entries": deploy_benchmark::evidence_task_entries_value(),
    });
    fill_benchmark_task_entries(&mut evidence);
    evidence["task_entries"][0]["status"] = serde_json::json!("maybe");

    let task_report = benchmark_report_tasks(&evidence, 300.0).expect("benchmark task report");
    let status = benchmark_report_status_summary(
        &task_report,
        &serde_json::json!({
            "failed_data": [],
            "missing_data": [],
        }),
        300.0,
    );

    assert_eq!(status.status, "incomplete");
    assert_eq!(task_report["recorded_task_count"], serde_json::json!(9));
    assert_eq!(
        task_report["missing_tasks"][0]["invalid_status"],
        serde_json::json!(true)
    );
}

#[test]
fn benchmark_report_requires_notes_for_recorded_tasks() {
    let mut evidence = serde_json::json!({
        "task_entries": deploy_benchmark::evidence_task_entries_value(),
    });
    fill_benchmark_task_entries(&mut evidence);
    evidence["task_entries"][0]["notes"] = serde_json::json!("   ");

    let task_report = benchmark_report_tasks(&evidence, 300.0).expect("benchmark task report");
    let status = benchmark_report_status_summary(
        &task_report,
        &serde_json::json!({
            "failed_data": [],
            "missing_data": [],
        }),
        300.0,
    );

    assert_eq!(status.status, "incomplete");
    assert_eq!(task_report["recorded_task_count"], serde_json::json!(9));
    assert_eq!(
        task_report["missing_tasks"][0]["missing_notes"],
        serde_json::json!(true)
    );
    assert_eq!(
        task_report["entries"][0]["missing_notes"],
        serde_json::json!(true)
    );
}

#[test]
fn benchmark_report_requires_retained_participant_note_artifacts() {
    let (src_dir, path) = prod_server_source("benchmark-report-missing-notes-source");
    let out = temp_output_dir("benchmark-report-missing-notes");

    cmd_build_with_profile(&path, &out, BuildProfile::Production).expect("prod build");
    let evidence_path = out.join("deploy").join("benchmark-evidence.json");
    let mut evidence = read_json_value(&evidence_path).expect("benchmark evidence");
    evidence["recording_status"] = serde_json::json!("recorded");
    fill_benchmark_task_entries(&mut evidence);
    evidence["data"]["docs_help_lookups"] = serde_json::json!(2);
    evidence["data"]["compiler_runtime_errors"] = serde_json::json!(0);
    evidence["data"]["ai_assistance_used"] = serde_json::json!(false);
    evidence["data"]["generated_artifact_edits"] = serde_json::json!(false);
    evidence["data"]["manual_undocumented_security_steps"] = serde_json::json!(false);
    evidence["data"]["manual_config_edits"] = serde_json::json!([]);
    evidence["data"]["smoke_test_output"] = serde_json::json!(benchmark_smoke_output_for(&out, 1));
    evidence["data"]["participant_notes"] = serde_json::json!("notes are summarized only");
    fill_benchmark_human_evidence_review(&mut evidence);
    fill_benchmark_participant_runs(&mut evidence);
    write_json(&evidence_path, &evidence).expect("write recorded benchmark evidence");

    let report = benchmark_report_value(&out).expect("benchmark report");

    assert_eq!(report["status"], "incomplete");
    assert!(report["data"]["missing_data"]
        .as_array()
        .expect("missing data")
        .iter()
        .any(|item| item == "participant_runs[0].raw_notes_artifact.retained"));
    assert!(report["data"]["missing_data"]
        .as_array()
        .expect("missing data")
        .iter()
        .any(|item| item == "participant_runs[1].raw_notes_artifact.retained"));
    assert_eq!(
        report["data"]["participant_raw_notes_artifacts"][0]["path"],
        "evidence/participant-1.md"
    );
    assert_eq!(
        report["data"]["participant_raw_notes_artifacts"][0]["path_safe"],
        true
    );
    assert_eq!(
        report["data"]["participant_raw_notes_artifacts"][0]["checked"],
        true
    );
    assert_eq!(
        report["data"]["participant_raw_notes_artifacts"][0]["retained"],
        false
    );
    assert!(report["data"]["participant_raw_notes_artifacts"][0]["non_empty"].is_null());
    assert!(report["data"]["participant_raw_notes_artifacts"][0]["template_filled"].is_null());
    assert!(report["data"]["participant_raw_notes_artifacts"][0]["size_bytes"].is_null());
    assert!(cmd_benchmark_report(&out, true)
        .expect_err("require pass rejects missing participant note artifacts")
        .to_string()
        .contains("benchmark report status must be passed"));
    let _ = std::fs::remove_dir_all(src_dir);
    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn benchmark_report_requires_non_empty_participant_note_artifacts() {
    let out = temp_output_dir("benchmark-report-empty-notes");
    let evidence_dir = out.join("evidence");
    std::fs::create_dir_all(&evidence_dir).expect("create participant evidence dir");
    std::fs::write(evidence_dir.join("participant-1.md"), "")
        .expect("write empty participant notes");
    std::fs::write(
        evidence_dir.join("participant-2.md"),
        "# Shop Benchmark Participant Notes\n\n## Participant\n\n- participant_id: participant-2\n- run_id: run-2\n- started_at: 2026-05-18T11:00:00Z\n- completed_at: 2026-05-18T12:20:00Z\n\n## Task Notes\n\nParticipant 2 completed the shop flow and retained real observations.\n",
    )
    .expect("write participant 2 notes");
    let mut evidence = serde_json::json!({
        "data": deploy_benchmark::evidence_data_value(),
    });
    evidence["data"]["docs_help_lookups"] = serde_json::json!(1);
    evidence["data"]["compiler_runtime_errors"] = serde_json::json!(0);
    evidence["data"]["ai_assistance_used"] = serde_json::json!(false);
    evidence["data"]["generated_artifact_edits"] = serde_json::json!(false);
    evidence["data"]["manual_undocumented_security_steps"] = serde_json::json!(false);
    evidence["data"]["manual_config_edits"] = serde_json::json!([]);
    evidence["data"]["participant_notes"] = serde_json::json!("one raw note artifact is empty");
    evidence["data"]["smoke_test_output"] = serde_json::json!(
        "orv deploy smoke test passed\nbuild_dir=/tmp/orv-build\nbase_url=http://127.0.0.1:8080\ngraph_contract=verified\ndap_summary=verified\ndap_source_bundle=verified\nserver_routes=1\ntrace_stream_requested=1\n"
    );
    fill_benchmark_participant_runs(&mut evidence);

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

    assert_eq!(status.status, "incomplete");
    assert!(data_report["missing_data"]
        .as_array()
        .expect("missing data")
        .iter()
        .any(|item| item == "participant_runs[0].raw_notes_artifact.non_empty"));
    assert_eq!(
        data_report["participant_raw_notes_artifacts"][0]["retained"],
        true
    );
    assert_eq!(
        data_report["participant_raw_notes_artifacts"][0]["non_empty"],
        false
    );
    assert_eq!(
        data_report["participant_raw_notes_artifacts"][0]["template_filled"],
        false
    );
    assert_eq!(
        data_report["participant_raw_notes_artifacts"][0]["size_bytes"],
        0
    );
    assert_eq!(
        data_report["participant_raw_notes_artifacts"][1]["non_empty"],
        true
    );
    assert_eq!(
        data_report["participant_raw_notes_artifacts"][1]["template_filled"],
        true
    );
    let _ = std::fs::remove_dir_all(out);
}

#[test]
fn benchmark_report_rejects_unfilled_participant_note_templates() {
    let out = temp_output_dir("benchmark-report-template-notes");
    let evidence_dir = out.join("evidence");
    std::fs::create_dir_all(&evidence_dir).expect("create participant evidence dir");
    std::fs::write(
        evidence_dir.join("participant-1.md"),
        participant_notes_template_content()
            .replace("- participant_id:", "- participant_id: participant-1")
            .replace("- run_id:", "- run_id: run-1"),
    )
    .expect("write unfilled participant 1 notes");
    std::fs::write(
        evidence_dir.join("participant-2.md"),
        "# Shop Benchmark Participant Notes\n\n## Participant\n\n- participant_id: participant-2\n- run_id: run-2\n- started_at: 2026-05-18T11:00:00Z\n- completed_at: 2026-05-18T12:20:00Z\n\n## Task Notes\n\nTask details filled from the human run.\n\n## Evidence Review\n\n- failure_classification.primary: documentation\n- failure_classification.notes: docs path was confusing\n",
    )
    .expect("write filled participant 2 notes");
    let mut evidence = serde_json::json!({
        "data": deploy_benchmark::evidence_data_value(),
    });
    fill_benchmark_report_observation_data(&mut evidence);

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

    assert_eq!(status.status, "incomplete");
    assert!(data_report["missing_data"]
        .as_array()
        .expect("missing data")
        .iter()
        .any(|item| item == "participant_runs[0].raw_notes_artifact.template_filled"));
    assert_eq!(
        data_report["participant_raw_notes_artifacts"][0]["retained"],
        true
    );
    assert_eq!(
        data_report["participant_raw_notes_artifacts"][0]["non_empty"],
        true
    );
    assert_eq!(
        data_report["participant_raw_notes_artifacts"][0]["template_filled"],
        false
    );
    assert_eq!(
        data_report["participant_raw_notes_artifacts"][0]["identity_match"],
        true
    );
    assert_eq!(
        data_report["participant_raw_notes_artifacts"][1]["template_filled"],
        true
    );
    let _ = std::fs::remove_dir_all(out);
}

#[test]
fn benchmark_report_rejects_raw_notes_identity_mismatch() {
    let out = temp_output_dir("benchmark-report-note-identity");
    let evidence_dir = out.join("evidence");
    std::fs::create_dir_all(&evidence_dir).expect("create participant evidence dir");
    std::fs::write(
        evidence_dir.join("participant-1.md"),
        "# Shop Benchmark Participant Notes\n\n- participant_id: participant-2\n- run_id: run-1\n- started_at: 2026-05-18T09:00:00Z\n- completed_at: 2026-05-18T10:00:00Z\n- failure_classification.primary: documentation\n- failure_classification.notes: docs path was confusing\n",
    )
    .expect("write mismatched participant notes");
    std::fs::write(
        evidence_dir.join("participant-2.md"),
        "# Shop Benchmark Participant Notes\n\n- participant_id: participant-2\n- run_id: run-2\n- started_at: 2026-05-18T11:00:00Z\n- completed_at: 2026-05-18T12:20:00Z\n- failure_classification.primary: documentation\n- failure_classification.notes: docs path was confusing\n",
    )
    .expect("write matched participant notes");
    let mut evidence = serde_json::json!({
        "data": deploy_benchmark::evidence_data_value(),
    });
    fill_benchmark_report_observation_data(&mut evidence);

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
fn benchmark_report_rejects_raw_notes_hash_mismatch() {
    let out = temp_output_dir("benchmark-report-note-hash");
    let mut evidence = serde_json::json!({
        "data": deploy_benchmark::evidence_data_value(),
    });
    fill_benchmark_report_observation_data(&mut evidence);
    write_benchmark_participant_note_artifacts(&out, &mut evidence);
    evidence["data"]["participant_runs"][0]["raw_notes_sha256"] = serde_json::json!(
        "sha256:0000000000000000000000000000000000000000000000000000000000000000"
    );

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

    assert_eq!(status.status, "failed");
    assert!(data_report["failed_data"]
        .as_array()
        .expect("failed data")
        .iter()
        .any(|item| item == "participant_runs[0].raw_notes_artifact.sha256_match"));
    assert_eq!(
        data_report["participant_raw_notes_artifacts"][0]["sha256_match"],
        false
    );
    assert_eq!(
        data_report["participant_raw_notes_artifacts"][1]["sha256_match"],
        true
    );
    assert_eq!(
        data_report["participant_raw_notes_artifacts"][0]["actual_sha256"],
        serde_json::json!(
            "sha256:7beae552ebe29639b2d61bf50985696b8c5ed9732c2d4f09e486806ca5033fdb"
        )
    );
    let _ = std::fs::remove_dir_all(out);
}

#[test]
#[cfg(unix)]
fn benchmark_report_rejects_symlinked_participant_note_artifacts_outside_build_dir() {
    let out = temp_output_dir("benchmark-report-symlink-notes");
    let outside = temp_output_dir("benchmark-report-outside-notes");
    let evidence_dir = out.join("evidence");
    std::fs::create_dir_all(&evidence_dir).expect("create participant evidence dir");
    std::fs::create_dir_all(&outside).expect("create outside evidence dir");
    let outside_note = outside.join("participant-1.md");
    std::fs::write(&outside_note, "outside raw benchmark notes\n")
        .expect("write outside participant notes");
    std::os::unix::fs::symlink(&outside_note, evidence_dir.join("participant-1.md"))
        .expect("symlink outside participant notes");
    std::fs::write(
        evidence_dir.join("participant-2.md"),
        "# Shop Benchmark Participant Notes\n\n## Participant\n\n- participant_id: participant-2\n- run_id: run-2\n- started_at: 2026-05-18T11:00:00Z\n- completed_at: 2026-05-18T12:20:00Z\n\n## Task Notes\n\nParticipant 2 completed the shop flow and retained real observations.\n",
    )
    .expect("write participant 2 notes");
    let mut evidence = serde_json::json!({
        "data": deploy_benchmark::evidence_data_value(),
    });
    evidence["data"]["docs_help_lookups"] = serde_json::json!(1);
    evidence["data"]["compiler_runtime_errors"] = serde_json::json!(0);
    evidence["data"]["ai_assistance_used"] = serde_json::json!(false);
    evidence["data"]["generated_artifact_edits"] = serde_json::json!(false);
    evidence["data"]["manual_undocumented_security_steps"] = serde_json::json!(false);
    evidence["data"]["manual_config_edits"] = serde_json::json!([]);
    evidence["data"]["participant_notes"] =
        serde_json::json!("one raw note artifact points outside the build dir");
    evidence["data"]["smoke_test_output"] = serde_json::json!(
        "orv deploy smoke test passed\nbuild_dir=/tmp/orv-build\nbase_url=http://127.0.0.1:8080\ngraph_contract=verified\ndap_summary=verified\ndap_source_bundle=verified\nserver_routes=1\ntrace_stream_requested=1\n"
    );
    fill_benchmark_participant_runs(&mut evidence);

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

    assert_eq!(status.status, "incomplete");
    assert!(data_report["missing_data"]
        .as_array()
        .expect("missing data")
        .iter()
        .any(|item| item == "participant_runs[0].raw_notes_artifact.retained"));
    assert_eq!(
        data_report["participant_raw_notes_artifacts"][0]["path_safe"],
        true
    );
    assert_eq!(
        data_report["participant_raw_notes_artifacts"][0]["checked"],
        true
    );
    assert_eq!(
        data_report["participant_raw_notes_artifacts"][0]["retained"],
        false
    );
    assert!(data_report["participant_raw_notes_artifacts"][0]["non_empty"].is_null());
    assert!(data_report["participant_raw_notes_artifacts"][0]["template_filled"].is_null());
    assert!(data_report["participant_raw_notes_artifacts"][0]["size_bytes"].is_null());
    assert_eq!(
        data_report["participant_raw_notes_artifacts"][1]["retained"],
        true
    );
    assert_eq!(
        data_report["participant_raw_notes_artifacts"][1]["non_empty"],
        true
    );
    assert_eq!(
        data_report["participant_raw_notes_artifacts"][1]["template_filled"],
        true
    );
    let _ = std::fs::remove_dir_all(out);
    let _ = std::fs::remove_dir_all(outside);
}

#[test]
fn benchmark_report_marks_recorded_evidence_passed() {
    let (src_dir, path) = prod_server_source("benchmark-report-passed-source");
    let out = temp_output_dir("benchmark-report-passed");

    cmd_build_with_profile(&path, &out, BuildProfile::Production).expect("prod build");
    let evidence_path = out.join("deploy").join("benchmark-evidence.json");
    let mut evidence = read_json_value(&evidence_path).expect("benchmark evidence");
    evidence["recording_status"] = serde_json::json!("recorded");
    fill_benchmark_task_entries(&mut evidence);
    evidence["data"]["docs_help_lookups"] = serde_json::json!(2);
    evidence["data"]["compiler_runtime_errors"] = serde_json::json!(0);
    evidence["data"]["ai_assistance_used"] = serde_json::json!(false);
    evidence["data"]["generated_artifact_edits"] = serde_json::json!(false);
    evidence["data"]["manual_undocumented_security_steps"] = serde_json::json!(false);
    evidence["data"]["manual_config_edits"] = serde_json::json!([]);
    evidence["data"]["smoke_test_output"] = serde_json::json!(benchmark_smoke_output_for(&out, 1));
    evidence["data"]["participant_notes"] = serde_json::json!("no blockers");
    fill_benchmark_human_evidence_review(&mut evidence);
    fill_benchmark_participant_runs(&mut evidence);
    write_benchmark_participant_note_artifacts(&out, &mut evidence);
    write_json(&evidence_path, &evidence).expect("write recorded benchmark evidence");

    let report = benchmark_report_value(&out).expect("benchmark report");

    assert_eq!(report["status"], "passed");
    assert_eq!(
        report["smoke_output_contract"]["output"],
        "deploy/smoke-output.txt"
    );
    assert_eq!(
        report["smoke_output_contract"]["required_markers"],
        serde_json::json!(deploy_benchmark::SMOKE_REQUIRED_MARKERS)
    );
    assert_eq!(report["time_over_limit"], false);
    assert_eq!(report["total_elapsed_minutes"], 100.0);
    assert_eq!(report["tasks"]["recorded_task_count"], 10);
    assert_eq!(report["tasks"]["missing_task_count"], 0);
    assert_eq!(report["tasks"]["failed_task_count"], 0);
    assert_eq!(report["data"]["smoke_test_summary"]["passed_marker"], true);
    assert_eq!(
        report["data"]["smoke_test_required_markers"],
        serde_json::json!(deploy_benchmark::SMOKE_REQUIRED_MARKERS)
    );
    assert_eq!(
        report["data"]["smoke_test_summary"]["required_markers"],
        serde_json::json!(deploy_benchmark::SMOKE_REQUIRED_MARKERS)
    );
    assert_eq!(
        report["data"]["smoke_test_summary"]["graph_contract_verified"],
        true
    );
    assert_eq!(
        report["data"]["smoke_test_summary"]["dap_summary_verified"],
        true
    );
    assert_eq!(
        report["data"]["smoke_test_summary"]["dap_source_bundle_verified"],
        true
    );
    assert_eq!(report["data"]["smoke_test_summary"]["server_routes"], 1);
    assert_eq!(
        report["data"]["expected_build_dir"],
        serde_json::json!(canonical_build_dir_string(&out))
    );
    assert_eq!(
        report["data"]["smoke_test_summary"]["build_dir"],
        serde_json::json!(canonical_build_dir_string(&out))
    );
    assert_eq!(
        report["data"]["missing_data"]
            .as_array()
            .expect("missing data")
            .len(),
        0
    );
    assert_eq!(
        report["data"]["participant_summary"]["recorded_run_count"],
        serde_json::json!(2)
    );
    assert_eq!(
        report["data"]["participant_raw_notes_artifacts"][0]["checked"],
        true
    );
    assert_eq!(
        report["data"]["participant_raw_notes_artifacts"][0]["retained"],
        true
    );
    assert_eq!(
        report["data"]["participant_raw_notes_artifacts"][0]["non_empty"],
        true
    );
    assert_eq!(
        report["data"]["participant_raw_notes_artifacts"][0]["template_filled"],
        true
    );
    assert_eq!(
        report["data"]["participant_raw_notes_artifacts"][0]["identity_match"],
        true
    );
    assert!(
        report["data"]["participant_raw_notes_artifacts"][0]["size_bytes"]
            .as_u64()
            .is_some_and(|size| size > 0)
    );
    assert_eq!(
        report["data"]["participant_raw_notes_artifacts"][1]["retained"],
        true
    );
    assert_eq!(
        report["data"]["participant_raw_notes_artifacts"][1]["template_filled"],
        true
    );
    assert_eq!(
        report["data"]["participant_raw_notes_artifacts"][1]["identity_match"],
        true
    );
    assert_eq!(
        benchmark_report_passed_inventory(&report),
        shop_benchmark_report_passed_golden(),
        "Shop benchmark recorded-evidence report golden drift"
    );
    cmd_benchmark_report(&out, true).expect("require pass accepts recorded evidence");
    let _ = std::fs::remove_dir_all(src_dir);
    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn benchmark_report_rejects_smoke_route_count_mismatch() {
    let (src_dir, path) = prod_server_source("benchmark-report-route-count-source");
    let out = temp_output_dir("benchmark-report-route-count");

    cmd_build_with_profile(&path, &out, BuildProfile::Production).expect("prod build");
    let evidence_path = out.join("deploy").join("benchmark-evidence.json");
    let mut evidence = read_json_value(&evidence_path).expect("benchmark evidence");
    evidence["recording_status"] = serde_json::json!("recorded");
    fill_benchmark_task_entries(&mut evidence);
    fill_benchmark_report_observation_data(&mut evidence);
    evidence["data"]["docs_help_lookups"] = serde_json::json!(2);
    evidence["data"]["smoke_test_output"] = serde_json::json!(benchmark_smoke_output_for(&out, 2));
    write_benchmark_participant_note_artifacts(&out, &mut evidence);
    write_json(&evidence_path, &evidence).expect("write recorded benchmark evidence");

    let report = benchmark_report_value(&out).expect("benchmark report");

    assert_eq!(report["status"], "incomplete");
    assert_eq!(
        report["data"]["expected_server_routes"],
        serde_json::json!(1)
    );
    assert_eq!(
        report["data"]["smoke_test_summary"]["server_routes"],
        serde_json::json!(2)
    );
    assert_eq!(
        report["data"]["expected_build_dir"],
        serde_json::json!(canonical_build_dir_string(&out))
    );
    assert_eq!(
        report["data"]["smoke_test_summary"]["build_dir"],
        serde_json::json!(canonical_build_dir_string(&out))
    );
    assert!(report["data"]["missing_data"]
        .as_array()
        .expect("missing data")
        .iter()
        .all(|item| item != "smoke_test_output.build_dir.match"));
    assert!(report["data"]["missing_data"]
        .as_array()
        .expect("missing data")
        .iter()
        .any(|item| item == "smoke_test_output.server_routes.match"));
    assert!(cmd_benchmark_report(&out, true)
        .expect_err("require pass rejects mismatched route count")
        .to_string()
        .contains("benchmark report status must be passed"));
    let _ = std::fs::remove_dir_all(src_dir);
    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn benchmark_report_rejects_smoke_build_dir_mismatch() {
    let (src_dir, path) = prod_server_source("benchmark-report-build-dir-source");
    let out = temp_output_dir("benchmark-report-build-dir");

    cmd_build_with_profile(&path, &out, BuildProfile::Production).expect("prod build");
    let evidence_path = out.join("deploy").join("benchmark-evidence.json");
    let mut evidence = read_json_value(&evidence_path).expect("benchmark evidence");
    evidence["recording_status"] = serde_json::json!("recorded");
    fill_benchmark_task_entries(&mut evidence);
    fill_benchmark_report_observation_data(&mut evidence);
    evidence["data"]["docs_help_lookups"] = serde_json::json!(2);
    evidence["data"]["smoke_test_output"] = serde_json::json!(
        "orv deploy smoke test passed\nbuild_dir=/tmp/orv-other-build\nbase_url=http://127.0.0.1:8080\ngraph_contract=verified\ndap_summary=verified\ndap_source_bundle=verified\nserver_routes=1\ntrace_stream_requested=1\n"
    );
    write_benchmark_participant_note_artifacts(&out, &mut evidence);
    write_json(&evidence_path, &evidence).expect("write recorded benchmark evidence");

    let report = benchmark_report_value(&out).expect("benchmark report");

    assert_eq!(report["status"], "incomplete");
    assert_eq!(
        report["data"]["expected_build_dir"],
        serde_json::json!(canonical_build_dir_string(&out))
    );
    assert_eq!(
        report["data"]["smoke_test_summary"]["build_dir"],
        "/tmp/orv-other-build"
    );
    assert!(report["data"]["missing_data"]
        .as_array()
        .expect("missing data")
        .iter()
        .any(|item| item == "smoke_test_output.build_dir.match"));
    assert!(cmd_benchmark_report(&out, true)
        .expect_err("require pass rejects mismatched build dir")
        .to_string()
        .contains("benchmark report status must be passed"));
    let _ = std::fs::remove_dir_all(src_dir);
    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn benchmark_report_marks_weak_smoke_output_incomplete() {
    let (src_dir, path) = prod_server_source("benchmark-report-weak-smoke-source");
    let out = temp_output_dir("benchmark-report-weak-smoke");

    cmd_build_with_profile(&path, &out, BuildProfile::Production).expect("prod build");
    let evidence_path = out.join("deploy").join("benchmark-evidence.json");
    let mut evidence = read_json_value(&evidence_path).expect("benchmark evidence");
    evidence["recording_status"] = serde_json::json!("recorded");
    fill_benchmark_task_entries(&mut evidence);
    evidence["data"]["docs_help_lookups"] = serde_json::json!(2);
    evidence["data"]["compiler_runtime_errors"] = serde_json::json!(0);
    evidence["data"]["manual_config_edits"] = serde_json::json!([]);
    evidence["data"]["smoke_test_output"] = serde_json::json!("smoke passed");
    evidence["data"]["participant_notes"] = serde_json::json!("weak smoke output");
    fill_benchmark_human_evidence_review(&mut evidence);
    write_json(&evidence_path, &evidence).expect("write recorded benchmark evidence");

    let report = benchmark_report_value(&out).expect("benchmark report");

    assert_eq!(report["status"], "incomplete");
    assert_eq!(report["data"]["smoke_test_summary"]["passed_marker"], false);
    assert!(report["data"]["missing_data"]
        .as_array()
        .expect("missing data")
        .iter()
        .any(|item| item == "smoke_test_output.graph_contract"));
    assert!(report["data"]["missing_data"]
        .as_array()
        .expect("missing data")
        .iter()
        .any(|item| item == "smoke_test_output.dap_summary"));
    assert!(report["data"]["missing_data"]
        .as_array()
        .expect("missing data")
        .iter()
        .any(|item| item == "smoke_test_output.dap_source_bundle"));
    assert!(cmd_benchmark_report(&out, true)
        .expect_err("require pass rejects weak smoke output")
        .to_string()
        .contains("benchmark report status must be passed"));
    let _ = std::fs::remove_dir_all(src_dir);
    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn benchmark_smoke_output_requires_http_base_url() {
    let summary = benchmark_smoke_test_output_summary(&serde_json::json!(
        "orv deploy smoke test passed\nbuild_dir=/tmp/orv-build\nbase_url=localhost:8080\ngraph_contract=verified\ndap_summary=verified\ndap_source_bundle=verified\nserver_routes=1\ntrace_stream_requested=1\n"
    ));

    assert!(summary["base_url"].is_null());
    assert!(summary["missing_markers"]
        .as_array()
        .expect("missing markers")
        .iter()
        .any(|item| item == "base_url"));
}

#[test]
fn benchmark_smoke_output_matches_published_shop_smoke_output_fixture() {
    const SHOP_SMOKE_OUTPUT_GOLDEN: &str =
        include_str!("../../../../docs/samples/shop-smoke-output-v1.golden.txt");
    const SHOP_SMOKE_OUTPUT_SUMMARY_GOLDEN: &str =
        include_str!("../../../../docs/samples/shop-smoke-output-summary-v1.golden.json");

    let summary = benchmark_smoke_test_output_summary(&serde_json::json!(SHOP_SMOKE_OUTPUT_GOLDEN));
    let expected: serde_json::Value = serde_json::from_str(SHOP_SMOKE_OUTPUT_SUMMARY_GOLDEN)
        .expect("smoke output summary golden");

    assert_eq!(summary, expected, "shop smoke output summary golden drift");
}

#[test]
fn benchmark_smoke_output_requires_absolute_build_dir() {
    let summary = benchmark_smoke_test_output_summary(&serde_json::json!(
        "orv deploy smoke test passed\nbuild_dir=dist\nbase_url=http://127.0.0.1:8080\ngraph_contract=verified\ndap_summary=verified\ndap_source_bundle=verified\nserver_routes=1\ntrace_stream_requested=1\n"
    ));

    assert!(summary["build_dir"].is_null());
    assert!(summary["missing_markers"]
        .as_array()
        .expect("missing markers")
        .iter()
        .any(|item| item == "build_dir"));
}

#[test]
fn benchmark_smoke_output_rejects_duplicate_marker_fields() {
    let summary = benchmark_smoke_test_output_summary(&serde_json::json!(
        "orv deploy smoke test passed\nbuild_dir=/tmp/orv-build\nbase_url=http://127.0.0.1:8080\ngraph_contract=missing\ngraph_contract=verified\ndap_summary=verified\ndap_source_bundle=verified\nserver_routes=1\ntrace_stream_requested=1\n"
    ));

    assert_eq!(
        summary["duplicate_fields"],
        serde_json::json!(["graph_contract"])
    );
    assert!(summary["missing_markers"]
        .as_array()
        .expect("missing markers")
        .iter()
        .any(|item| item == "graph_contract"));
}

#[test]
fn benchmark_smoke_output_requires_trace_stream_requested() {
    let summary = benchmark_smoke_test_output_summary(&serde_json::json!(
        "orv deploy smoke test passed\nbuild_dir=/tmp/orv-build\nbase_url=http://127.0.0.1:8080\ngraph_contract=verified\ndap_summary=verified\ndap_source_bundle=verified\nserver_routes=1\ntrace_stream_requested=0\n"
    ));

    assert_eq!(summary["trace_stream_requested"], serde_json::json!(false));
    assert!(summary["missing_markers"]
        .as_array()
        .expect("missing markers")
        .iter()
        .any(|item| item == "trace_stream_requested"));
}

#[test]
fn benchmark_report_uses_generated_smoke_output_artifact() {
    let (src_dir, path) = prod_server_source("benchmark-report-smoke-output-source");
    let out = temp_output_dir("benchmark-report-smoke-output");

    cmd_build_with_profile(&path, &out, BuildProfile::Production).expect("prod build");
    let evidence_path = out.join("deploy").join("benchmark-evidence.json");
    let smoke_output_path = out.join("deploy").join("smoke-output.txt");
    let mut evidence = read_json_value(&evidence_path).expect("benchmark evidence");
    evidence["recording_status"] = serde_json::json!("recorded");
    fill_benchmark_task_entries(&mut evidence);
    evidence["data"]["docs_help_lookups"] = serde_json::json!(1);
    evidence["data"]["compiler_runtime_errors"] = serde_json::json!(0);
    evidence["data"]["ai_assistance_used"] = serde_json::json!(false);
    evidence["data"]["generated_artifact_edits"] = serde_json::json!(false);
    evidence["data"]["manual_undocumented_security_steps"] = serde_json::json!(false);
    evidence["data"]["manual_config_edits"] = serde_json::json!([]);
    evidence["data"]["participant_notes"] = serde_json::json!("smoke output from artifact");
    fill_benchmark_human_evidence_review(&mut evidence);
    fill_benchmark_participant_runs(&mut evidence);
    write_benchmark_participant_note_artifacts(&out, &mut evidence);
    write_json(&evidence_path, &evidence).expect("write recorded benchmark evidence");
    let smoke_output = benchmark_smoke_output_for(&out, 1);
    std::fs::write(&smoke_output_path, &smoke_output).expect("write smoke output");

    let report = benchmark_report_value(&out).expect("benchmark report");

    assert_eq!(report["status"], "passed");
    assert_eq!(
        report["data"]["smoke_test_output"],
        serde_json::json!(smoke_output)
    );
    assert_eq!(
        report["data"]["smoke_test_output_source"],
        "deploy/smoke-output.txt"
    );
    assert_eq!(
        report["data"]["smoke_test_output_artifact_path"],
        "deploy/smoke-output.txt"
    );
    assert!(report["data"]["smoke_test_output_artifact_match"].is_null());
    assert_eq!(
        report["data"]["smoke_test_summary"]["trace_stream_requested"],
        true
    );
    assert_eq!(
        report["data"]["smoke_test_summary"]["dap_summary_verified"],
        true
    );
    assert_eq!(
        report["data"]["smoke_test_summary"]["dap_source_bundle_verified"],
        true
    );
    assert_eq!(
        report["data"]["expected_build_dir"],
        serde_json::json!(canonical_build_dir_string(&out))
    );
    assert_eq!(
        report["data"]["smoke_test_summary"]["build_dir"],
        serde_json::json!(canonical_build_dir_string(&out))
    );
    assert_eq!(
        report["data"]["missing_data"]
            .as_array()
            .expect("missing data")
            .len(),
        0
    );
    cmd_benchmark_report(&out, true).expect("require pass accepts generated smoke output artifact");
    let _ = std::fs::remove_dir_all(src_dir);
    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn benchmark_report_rejects_smoke_output_artifact_mismatch() {
    let (src_dir, path) = prod_server_source("benchmark-report-smoke-output-mismatch-source");
    let out = temp_output_dir("benchmark-report-smoke-output-mismatch");

    cmd_build_with_profile(&path, &out, BuildProfile::Production).expect("prod build");
    let evidence_path = out.join("deploy").join("benchmark-evidence.json");
    let smoke_output_path = out.join("deploy").join("smoke-output.txt");
    let mut evidence = read_json_value(&evidence_path).expect("benchmark evidence");
    evidence["recording_status"] = serde_json::json!("recorded");
    fill_benchmark_task_entries(&mut evidence);
    evidence["data"]["docs_help_lookups"] = serde_json::json!(2);
    evidence["data"]["compiler_runtime_errors"] = serde_json::json!(0);
    evidence["data"]["ai_assistance_used"] = serde_json::json!(false);
    evidence["data"]["generated_artifact_edits"] = serde_json::json!(false);
    evidence["data"]["manual_undocumented_security_steps"] = serde_json::json!(false);
    evidence["data"]["manual_config_edits"] = serde_json::json!([]);
    evidence["data"]["smoke_test_output"] = serde_json::json!(benchmark_smoke_output_for(&out, 1));
    evidence["data"]["participant_notes"] = serde_json::json!("copied smoke output is stale");
    fill_benchmark_human_evidence_review(&mut evidence);
    fill_benchmark_participant_runs(&mut evidence);
    write_benchmark_participant_note_artifacts(&out, &mut evidence);
    write_json(&evidence_path, &evidence).expect("write recorded benchmark evidence");
    std::fs::write(&smoke_output_path, benchmark_smoke_output_for(&out, 2))
        .expect("write mismatched smoke output");

    let report = benchmark_report_value(&out).expect("benchmark report");

    assert_eq!(report["status"], "failed");
    assert_eq!(report["data"]["smoke_test_output_source"], "evidence");
    assert_eq!(
        report["data"]["smoke_test_output_artifact_path"],
        "deploy/smoke-output.txt"
    );
    assert_eq!(
        report["data"]["smoke_test_output_artifact_match"],
        serde_json::json!(false)
    );
    assert!(report["data"]["failed_data"]
        .as_array()
        .expect("failed data")
        .iter()
        .any(|item| item == "smoke_test_output.artifact_match"));
    assert!(report["data"]["missing_data"]
        .as_array()
        .expect("missing data")
        .iter()
        .all(|item| item != "smoke_test_output.server_routes.match"));
    assert!(cmd_benchmark_report(&out, true)
        .expect_err("require pass rejects mismatched smoke output artifact")
        .to_string()
        .contains("benchmark report status must be passed"));
    let _ = std::fs::remove_dir_all(src_dir);
    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn reveal_benchmark_summary_exposes_smoke_output_artifact_match() {
    let (src_dir, path) = prod_server_source("reveal-benchmark-smoke-output-mismatch-source");
    let out = temp_output_dir("reveal-benchmark-smoke-output-mismatch");

    cmd_build_with_profile(&path, &out, BuildProfile::Production).expect("prod build");
    let preflight_path = out.join("deploy").join("preflight.json");
    let evidence_path = out.join("deploy").join("benchmark-evidence.json");
    let smoke_output_path = out.join("deploy").join("smoke-output.txt");
    let preflight = read_json_value(&preflight_path).expect("preflight");
    let mut evidence = read_json_value(&evidence_path).expect("benchmark evidence");
    evidence["recording_status"] = serde_json::json!("recorded");
    fill_benchmark_task_entries(&mut evidence);
    fill_benchmark_report_observation_data(&mut evidence);
    evidence["data"]["docs_help_lookups"] = serde_json::json!(2);
    evidence["data"]["smoke_test_output"] = serde_json::json!(benchmark_smoke_output_for(&out, 1));
    write_benchmark_participant_note_artifacts(&out, &mut evidence);
    write_json(&evidence_path, &evidence).expect("write recorded benchmark evidence");
    std::fs::write(&smoke_output_path, benchmark_smoke_output_for(&out, 2))
        .expect("write mismatched smoke output");

    let summary = reveal_benchmark_evidence_summary(&out, &preflight).expect("benchmark summary");

    assert_eq!(summary["report_status"], "failed");
    assert_eq!(summary["smoke_test_output_source"], "evidence");
    assert_eq!(
        summary["smoke_test_output_artifact_path"],
        "deploy/smoke-output.txt"
    );
    assert_eq!(
        summary["smoke_test_output_artifact_match"],
        serde_json::json!(false)
    );
    assert!(summary["failed_data"]
        .as_array()
        .expect("failed data")
        .iter()
        .any(|item| item == "smoke_test_output.artifact_match"));
    let _ = std::fs::remove_dir_all(src_dir);
    let _ = std::fs::remove_dir_all(&out);
}
