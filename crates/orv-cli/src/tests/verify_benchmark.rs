use super::*;

#[test]
fn verify_build_rejects_deploy_benchmark_evidence_smoke_output_contract_mismatch() {
    let (src_dir, path) =
        prod_server_source("deploy-benchmark-evidence-smoke-output-contract-source");
    let out = temp_output_dir("deploy-benchmark-evidence-smoke-output-contract-mismatch");

    cmd_build_with_profile(&path, &out, BuildProfile::Production).expect("prod build");
    let evidence_path = out.join("deploy").join("benchmark-evidence.json");
    let mut evidence = read_json_value(&evidence_path).expect("benchmark evidence");
    evidence["smoke_output_contract"]["output"] = serde_json::json!("deploy/wrong-smoke.txt");
    write_json(&evidence_path, &evidence).expect("write drifted benchmark evidence");

    let err =
        cmd_verify_build(&out).expect_err("benchmark evidence smoke output contract mismatch");

    assert!(err.to_string().contains(
        "deploy benchmark evidence smoke_output_contract must match smoke output contract"
    ));
    let _ = std::fs::remove_dir_all(src_dir);
    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn verify_build_rejects_deploy_benchmark_evidence_extra_smoke_output_contract_key() {
    let (src_dir, path) =
        prod_server_source("deploy-benchmark-evidence-extra-smoke-contract-source");
    let out = temp_output_dir("deploy-benchmark-evidence-extra-smoke-contract");

    cmd_build_with_profile(&path, &out, BuildProfile::Production).expect("prod build");
    let evidence_path = out.join("deploy").join("benchmark-evidence.json");
    let mut evidence = read_json_value(&evidence_path).expect("benchmark evidence");
    evidence["smoke_output_contract"]["unexpected"] = serde_json::json!("drift");
    write_json(&evidence_path, &evidence).expect("write drifted benchmark evidence");

    let err =
        cmd_verify_build(&out).expect_err("extra benchmark evidence smoke contract key must fail");

    assert!(err
        .to_string()
        .contains("deploy benchmark evidence smoke_output_contract keys must match contract"));
    let _ = std::fs::remove_dir_all(src_dir);
    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn verify_deploy_benchmark_evidence_data_rejects_participant_contract_drift() {
    for (key, expected) in [
        (
            "participant_runs",
            "deploy benchmark evidence data must include participant_runs",
        ),
        (
            "failure_classification",
            "deploy benchmark evidence data must include failure_classification",
        ),
    ] {
        let mut evidence = serde_json::json!({
            "data": deploy_benchmark::evidence_data_value(),
        });
        evidence["data"]
            .as_object_mut()
            .expect("benchmark data")
            .remove(key);

        let err = verify_deploy_benchmark_evidence_data(&evidence)
            .expect_err("participant contract drift must fail");

        assert!(err.to_string().contains(expected));
    }
}

#[test]
fn verify_deploy_benchmark_evidence_data_rejects_nested_key_drift() {
    for (evidence, expected) in [
        (
            {
                let mut evidence = serde_json::json!({
                    "data": deploy_benchmark::evidence_data_value(),
                });
                evidence["data"]["recommended_participant_count"]["unexpected"] =
                    serde_json::json!(true);
                evidence
            },
            "deploy benchmark evidence data recommended_participant_count keys must match contract",
        ),
        (
            {
                let mut evidence = serde_json::json!({
                    "data": deploy_benchmark::evidence_data_value(),
                });
                evidence["data"]["failure_classification"]["unexpected"] =
                    serde_json::json!("drift");
                evidence
            },
            "deploy benchmark evidence data failure_classification keys must match contract",
        ),
    ] {
        let err =
            verify_deploy_benchmark_evidence_data(&evidence).expect_err("nested drift must fail");

        assert!(err.to_string().contains(expected), "{err:?}");
    }
}

#[test]
fn verify_deploy_benchmark_evidence_data_rejects_participant_count_drift() {
    let mut evidence = serde_json::json!({
        "data": deploy_benchmark::evidence_data_value(),
    });
    evidence["data"]["recommended_participant_count"]["minimum"] = serde_json::json!(1);

    let err = verify_deploy_benchmark_evidence_data(&evidence)
        .expect_err("participant count drift must fail");

    assert!(
        err.to_string().contains(
            "deploy benchmark evidence data recommended_participant_count must match benchmark contract"
        ),
        "{err:?}"
    );
}

#[test]
fn verify_deploy_benchmark_evidence_data_rejects_other_without_notes() {
    let mut evidence = serde_json::json!({
        "data": deploy_benchmark::evidence_data_value(),
    });
    evidence["data"]["failure_classification"]["primary"] = serde_json::json!("other");
    evidence["data"]["failure_classification"]["notes"] = serde_json::json!(" ");

    let err = verify_deploy_benchmark_evidence_data(&evidence)
        .expect_err("other failure category must explain why");

    assert!(
        err.to_string().contains(
            "deploy benchmark evidence data failure_classification notes must explain other"
        ),
        "{err:?}"
    );
}

#[test]
fn verify_deploy_benchmark_evidence_data_rejects_negative_observation_counts() {
    for key in ["docs_help_lookups", "compiler_runtime_errors"] {
        let mut evidence = serde_json::json!({
            "data": deploy_benchmark::evidence_data_value(),
        });
        evidence["data"][key] = serde_json::json!(-1);

        let err = verify_deploy_benchmark_evidence_data(&evidence)
            .expect_err("negative observation count must fail");

        assert!(
            err.to_string().contains(&format!(
                "deploy benchmark evidence data {key} must be null or a non-negative integer"
            )),
            "{err:?}"
        );
    }
}

#[test]
fn verify_deploy_benchmark_evidence_rejects_negative_time_values() {
    let mut evidence = serde_json::json!({
        "task_entries": deploy_benchmark::evidence_task_entries_value(),
        "data": deploy_benchmark::evidence_data_value(),
    });
    evidence["task_entries"][0]["elapsed_minutes"] = serde_json::json!(-1.0);

    let err = verify_deploy_benchmark_evidence_task_entries(&evidence)
        .expect_err("negative task elapsed time must fail");

    assert!(
        err.to_string().contains(
            "deploy benchmark evidence task_entries[0] elapsed_minutes must be null or a non-negative number"
        ),
        "{err:?}"
    );

    evidence["task_entries"][0]["elapsed_minutes"] = serde_json::Value::Null;
    evidence["data"]["first_error_to_fix_minutes"] = serde_json::json!(-0.5);

    let err = verify_deploy_benchmark_evidence_data(&evidence)
        .expect_err("negative first-error-to-fix time must fail");

    assert!(
        err.to_string().contains(
            "deploy benchmark evidence data first_error_to_fix_minutes must be null or a non-negative number"
        ),
        "{err:?}"
    );
}

#[test]
fn verify_deploy_benchmark_evidence_data_rejects_required_false_gate_type_drift() {
    for key in [
        "ai_assistance_used",
        "generated_artifact_edits",
        "manual_undocumented_security_steps",
    ] {
        let mut evidence = serde_json::json!({
            "data": deploy_benchmark::evidence_data_value(),
        });
        evidence["data"][key] = serde_json::json!("no");

        let err = verify_deploy_benchmark_evidence_data(&evidence)
            .expect_err("required false gate type drift must fail");

        assert!(
            err.to_string().contains(&format!(
                "deploy benchmark evidence data {key} must be null or a bool"
            )),
            "{err:?}"
        );
    }
}

#[test]
fn verify_deploy_benchmark_evidence_data_rejects_bad_manual_config_edits() {
    for (value, expected) in [
        (
            serde_json::json!([42]),
            "deploy benchmark evidence data manual_config_edits[0] must be a string",
        ),
        (
            serde_json::json!(["   "]),
            "deploy benchmark evidence data manual_config_edits[0] must not be blank",
        ),
    ] {
        let mut evidence = serde_json::json!({
            "data": deploy_benchmark::evidence_data_value(),
        });
        evidence["data"]["manual_config_edits"] = value;

        let err = verify_deploy_benchmark_evidence_data(&evidence)
            .expect_err("manual config edit entries must be useful evidence");

        assert!(err.to_string().contains(expected), "{err:?}");
    }
}

#[test]
fn verify_deploy_benchmark_evidence_data_rejects_participant_profile_drift() {
    let mut evidence = serde_json::json!({
        "data": deploy_benchmark::evidence_data_value(),
    });
    evidence["data"]["participant_runs"][0]["participant_profile"] = serde_json::json!("developer");

    let err = verify_deploy_benchmark_evidence_data(&evidence)
        .expect_err("participant profile drift must fail");

    assert!(
        err.to_string().contains(
            "deploy benchmark evidence data participant_runs[0] participant_profile must be non_developer"
        ),
        "{err:?}"
    );
}

#[test]
fn verify_deploy_benchmark_evidence_data_rejects_invalid_participant_timestamps() {
    let mut evidence = serde_json::json!({
        "data": deploy_benchmark::evidence_data_value(),
    });
    evidence["data"]["participant_runs"][0]["started_at"] = serde_json::json!("2026/05/18 09:00");

    let err = verify_deploy_benchmark_evidence_data(&evidence)
        .expect_err("invalid participant timestamp must fail");

    assert!(
        err.to_string().contains(
            "deploy benchmark evidence data participant_runs[0] started_at must be null or an RFC3339 UTC timestamp"
        ),
        "{err:?}"
    );

    evidence["data"]["participant_runs"][0]["started_at"] =
        serde_json::json!("2026-05-18T10:00:00Z");
    evidence["data"]["participant_runs"][0]["completed_at"] =
        serde_json::json!("2026-05-18T09:00:00Z");

    let err = verify_deploy_benchmark_evidence_data(&evidence)
        .expect_err("reversed participant timestamp order must fail");

    assert!(
        err.to_string().contains(
            "deploy benchmark evidence data participant_runs[0] completed_at must be >= started_at"
        ),
        "{err:?}"
    );
}

#[test]
fn verify_deploy_benchmark_evidence_data_rejects_duplicate_participant_ids() {
    let mut evidence = serde_json::json!({
        "data": deploy_benchmark::evidence_data_value(),
    });
    fill_benchmark_participant_runs(&mut evidence);
    evidence["data"]["participant_runs"][1]["run_id"] = serde_json::json!("run-1");

    let err =
        verify_deploy_benchmark_evidence_data(&evidence).expect_err("duplicate run id must fail");

    assert!(
        err.to_string()
            .contains("deploy benchmark evidence data participant_runs[1] run_id must be unique"),
        "{err:?}"
    );

    evidence["data"]["participant_runs"][1]["run_id"] = serde_json::json!("run-2");
    evidence["data"]["participant_runs"][1]["participant_id"] = serde_json::json!("participant-1");

    let err = verify_deploy_benchmark_evidence_data(&evidence)
        .expect_err("duplicate participant id must fail");

    assert!(
        err.to_string().contains(
            "deploy benchmark evidence data participant_runs[1] participant_id must be unique"
        ),
        "{err:?}"
    );
}

#[test]
fn verify_deploy_benchmark_evidence_rejects_unknown_status_values() {
    let mut evidence = serde_json::json!({
        "task_entries": deploy_benchmark::evidence_task_entries_value(),
        "data": deploy_benchmark::evidence_data_value(),
    });
    evidence["task_entries"][0]["status"] = serde_json::json!("maybe");

    let err = verify_deploy_benchmark_evidence_task_entries(&evidence)
        .expect_err("unknown task status must fail");

    assert!(
        err.to_string().contains(
            "deploy benchmark evidence task_entries[0] status must be an allowed benchmark status"
        ),
        "{err:?}"
    );

    evidence["task_entries"][0]["status"] = serde_json::json!("not_recorded");
    evidence["data"]["participant_runs"][0]["status"] = serde_json::json!("maybe");

    let err = verify_deploy_benchmark_evidence_data(&evidence)
        .expect_err("unknown participant status must fail");

    assert!(
        err.to_string().contains(
            "deploy benchmark evidence data participant_runs[0] status must be an allowed benchmark status"
        ),
        "{err:?}"
    );
}

#[test]
fn verify_deploy_benchmark_evidence_data_rejects_unsafe_raw_notes_paths() {
    for raw_notes_artifact in [
        "/tmp/participant.md",
        "../participant.md",
        r"C:\participants\participant.md",
        r"evidence\..\participant.md",
        "",
    ] {
        let mut evidence = serde_json::json!({
            "data": deploy_benchmark::evidence_data_value(),
        });
        evidence["data"]["participant_runs"][0]["raw_notes_artifact"] =
            serde_json::json!(raw_notes_artifact);

        let err = verify_deploy_benchmark_evidence_data(&evidence)
            .expect_err("unsafe raw notes path must fail");

        assert!(
            err.to_string().contains(
                "deploy benchmark evidence data participant_runs[0] raw_notes_artifact must be null or a relative path under the build directory"
            ),
            "{err:?}"
        );
    }
}

#[test]
fn verify_deploy_benchmark_evidence_data_rejects_invalid_raw_notes_sha256() {
    for raw_notes_sha256 in [
        "59afd39ead0f48f4b1b16e732b81711e039251c225f0da5264879d34b8795f14",
        "sha256:59AFD39EAD0F48F4B1B16E732B81711E039251C225F0DA5264879D34B8795F14",
        "sha256:too-short",
    ] {
        let mut evidence = serde_json::json!({
            "data": deploy_benchmark::evidence_data_value(),
        });
        evidence["data"]["participant_runs"][0]["raw_notes_sha256"] =
            serde_json::json!(raw_notes_sha256);

        let err = verify_deploy_benchmark_evidence_data(&evidence)
            .expect_err("invalid raw notes hash must fail");

        assert!(
            err.to_string().contains(
                "deploy benchmark evidence data participant_runs[0] raw_notes_sha256 must be null or sha256:<64 lowercase hex>"
            ),
            "{err:?}"
        );
    }
}

#[test]
fn verify_benchmark_artifact_cases() {
    verify_artifact_cases(
        "verify_benchmark_artifact_cases",
        prod_server_source,
        BuildProfile::Production,
        &[
            json_case(
                "deploy_preflight_benchmark_mismatch",
                "deploy/preflight.json",
                "deploy preflight benchmark does not match 5-hour shop contract",
                |preflight| {
                    preflight["benchmark"]["max_elapsed_minutes"] = serde_json::json!(301);
                },
            ),
            json_case(
                "deploy_benchmark_evidence_mismatch",
                "deploy/benchmark-evidence.json",
                "deploy benchmark evidence benchmark does not match 5-hour shop contract",
                |evidence| {
                    evidence["benchmark"]["max_elapsed_minutes"] = serde_json::json!(301);
                },
            ),
            json_case(
                "deploy_benchmark_evidence_preflight_hash_mismatch",
                "deploy/benchmark-evidence.json",
                "deploy benchmark evidence preflight_hash",
                |evidence| {
                    evidence["preflight_hash"] = serde_json::json!("stale");
                },
            ),
            json_case(
                "deploy_benchmark_evidence_commands_mismatch",
                "deploy/benchmark-evidence.json",
                "deploy benchmark evidence commands do not match deploy preflight",
                |evidence| {
                    evidence["commands"]["trace_stream_smoke"] =
                        serde_json::json!("./deploy/smoke-test.sh");
                },
            ),
            json_case(
                "deploy_benchmark_evidence_artifacts_mismatch",
                "deploy/benchmark-evidence.json",
                "deploy benchmark evidence artifacts do not match deploy preflight",
                |evidence| {
                    evidence["artifacts"]["project_graph"] =
                        serde_json::json!("wrong-project-graph.json");
                },
            ),
            artifact_case(
                "deploy_benchmark_evidence_unknown_recording_status",
                |out| {
                    let evidence_path = out.join("deploy").join("benchmark-evidence.json");
                    let mut evidence = read_json_value(&evidence_path).expect("benchmark evidence");
                    evidence["recording_status"] = serde_json::json!("draft");
                    write_json(&evidence_path, &evidence)
                        .expect("write corrupt benchmark evidence");

                    let err = cmd_verify_build(out)
                        .expect_err("unknown benchmark recording status must fail");

                    assert!(err.to_string().contains(
        "deploy benchmark evidence recording_status must be not_recorded, sample, or recorded"
    ));
                },
            ),
            json_case(
                "deploy_benchmark_evidence_extra_root_key",
                "deploy/benchmark-evidence.json",
                "deploy benchmark evidence keys must match contract",
                |evidence| {
                    evidence["unexpected"] = serde_json::json!(true);
                },
            ),
            json_case(
                "deploy_benchmark_evidence_extra_data_key",
                "deploy/benchmark-evidence.json",
                "deploy benchmark evidence data keys must match contract",
                |evidence| {
                    evidence["data"]["unexpected"] = serde_json::json!("drift");
                },
            ),
            json_case(
                "deploy_benchmark_evidence_extra_task_key",
                "deploy/benchmark-evidence.json",
                "deploy benchmark evidence task_entries[0] keys must match contract",
                |evidence| {
                    evidence["task_entries"][0]["unexpected"] = serde_json::json!(true);
                },
            ),
            json_case(
                "deploy_benchmark_evidence_extra_participant_key",
                "deploy/benchmark-evidence.json",
                "deploy benchmark evidence data participant_runs[0] keys must match contract",
                |evidence| {
                    evidence["data"]["participant_runs"][0]["unexpected"] =
                        serde_json::json!("drift");
                },
            ),
            json_case(
                "deploy_benchmark_evidence_smoke_marker_mismatch",
                "deploy/benchmark-evidence.json",
                "smoke_test_required_markers must match smoke output contract",
                |evidence| {
                    evidence["data"]["smoke_test_required_markers"] =
                        serde_json::json!(["pass_marker", "build_dir", "base_url"]);
                },
            ),
            artifact_case(
                "verify_build_accepts_recorded_deploy_benchmark_evidence_values",
                |out| {
                    let evidence_path = out.join("deploy").join("benchmark-evidence.json");
                    let mut evidence = read_json_value(&evidence_path).expect("benchmark evidence");
                    evidence["recording_status"] = serde_json::json!("recorded");
                    evidence["task_entries"][0]["elapsed_minutes"] = serde_json::json!(12.5);
                    evidence["task_entries"][0]["status"] = serde_json::json!("recorded");
                    evidence["task_entries"][0]["notes"] = serde_json::json!("first run completed");
                    evidence["data"]["docs_help_lookups"] = serde_json::json!(3);
                    evidence["data"]["compiler_runtime_errors"] = serde_json::json!(1);
                    evidence["data"]["first_error_to_fix_minutes"] = serde_json::json!(4.5);
                    evidence["data"]["ai_assistance_used"] = serde_json::json!(false);
                    evidence["data"]["generated_artifact_edits"] = serde_json::json!(false);
                    evidence["data"]["manual_undocumented_security_steps"] =
                        serde_json::json!(false);
                    evidence["data"]["manual_config_edits"] = serde_json::json!(["none"]);
                    evidence["data"]["smoke_test_output"] = serde_json::json!("passed");
                    evidence["data"]["participant_notes"] = serde_json::json!("sample");
                    fill_benchmark_human_evidence_review(&mut evidence);
                    write_json(&evidence_path, &evidence)
                        .expect("write recorded benchmark evidence");

                    cmd_verify_build(out).expect("recorded benchmark evidence still verifies");
                },
            ),
            json_case(
                "deploy_preflight_benchmark_report_command_mismatch",
                "deploy/preflight.json",
                "deploy preflight benchmark_report command",
                |preflight| {
                    preflight["commands"]["benchmark_report"] =
                        serde_json::json!("orv benchmark-report other");
                },
            ),
        ],
    );
}
