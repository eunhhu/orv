use super::*;

pub(crate) fn cmd_benchmark_report(dir: &Path, require_pass: bool) -> anyhow::Result<()> {
    let report = benchmark_report_value(dir)?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    if require_pass && report.get("status").and_then(serde_json::Value::as_str) != Some("passed") {
        anyhow::bail!("benchmark report status must be passed");
    }
    Ok(())
}

pub(crate) fn cmd_benchmark_prepare(dir: &Path, participants: usize) -> anyhow::Result<()> {
    let prepared = benchmark_prepare_participants_value(dir, participants)?;
    println!("{}", serde_json::to_string_pretty(&prepared)?);
    Ok(())
}

pub(crate) fn benchmark_prepare_participants_value(
    dir: &Path,
    participants: usize,
) -> anyhow::Result<serde_json::Value> {
    let minimum = usize::try_from(deploy_benchmark::RECOMMENDED_PARTICIPANT_MINIMUM)
        .expect("recommended participant minimum fits usize");
    if participants < minimum {
        anyhow::bail!("benchmark prepare participants must be at least {minimum}");
    }
    verify_build_dir(dir)?;
    let deploy = read_json_value(&dir.join("deploy").join("manifest.json"))?;
    let server = deploy
        .get("server")
        .filter(|value| !value.is_null())
        .ok_or_else(|| anyhow::anyhow!("deploy manifest server target is required"))?;
    let evidence_rel = json_str(server, "benchmark_evidence", "deploy server")?;
    let template_rel = json_str(server, "participant_notes_template", "deploy server")?;
    let template_path = dir.join(template_rel);
    let notes_template = std::fs::read_to_string(&template_path)
        .map_err(|e| anyhow::anyhow!("failed to read {}: {e}", template_path.display()))?;
    let evidence_path = dir.join(evidence_rel);
    let mut evidence = read_json_value(&evidence_path)?;
    let data = evidence
        .get_mut("data")
        .and_then(serde_json::Value::as_object_mut)
        .ok_or_else(|| anyhow::anyhow!("benchmark evidence data must be an object"))?;
    let runs = data
        .get_mut("participant_runs")
        .and_then(serde_json::Value::as_array_mut)
        .ok_or_else(|| anyhow::anyhow!("benchmark evidence participant_runs must be an array"))?;
    while runs.len() < participants {
        let index = runs.len() + 1;
        runs.push(benchmark_prepare_empty_participant_run(index));
    }

    let mut raw_notes_artifacts = Vec::with_capacity(participants);
    for (index, run) in runs.iter_mut().take(participants).enumerate() {
        let number = index + 1;
        let run = run.as_object_mut().ok_or_else(|| {
            anyhow::anyhow!("benchmark evidence participant_runs[{index}] must be an object")
        })?;
        let participant_id = benchmark_prepare_set_string_if_missing(
            run,
            "participant_id",
            format!("participant-{number}"),
        );
        let run_id =
            benchmark_prepare_set_string_if_missing(run, "run_id", format!("run-{number}"));
        let status = benchmark_prepare_status(run);
        let raw_notes_artifact = benchmark_prepare_set_string_if_missing(
            run,
            "raw_notes_artifact",
            format!("deploy/evidence/participant-{number}.md"),
        );
        if !benchmark_raw_notes_artifact_path_is_safe(&raw_notes_artifact) {
            anyhow::bail!(
                "benchmark evidence participant_runs[{index}] raw_notes_artifact must be a safe relative path"
            );
        }
        let created = benchmark_prepare_write_participant_notes(
            dir,
            &raw_notes_artifact,
            &notes_template,
            &participant_id,
            &run_id,
        )?;
        raw_notes_artifacts.push(serde_json::json!({
            "index": index,
            "participant_id": participant_id,
            "run_id": run_id,
            "status": status,
            "path": raw_notes_artifact,
            "created": created,
        }));
    }
    let participants_total = runs.len();
    let fields_to_record = evidence
        .pointer("/benchmark/data_to_record")
        .cloned()
        .unwrap_or_else(|| serde_json::json!([]));
    let success_criteria = evidence
        .pointer("/benchmark/success_criteria")
        .cloned()
        .unwrap_or_else(|| serde_json::json!([]));
    let recording_status = evidence
        .get("recording_status")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("not_recorded")
        .to_string();
    write_json(&evidence_path, &evidence)?;
    verify_build_dir(dir)?;

    Ok(serde_json::json!({
        "schema_version": 1,
        "kind": "orv.benchmark.shop_5h.prepare",
        "build_dir": dir.display().to_string(),
        "evidence": evidence_rel,
        "participant_notes_template": template_rel,
        "participants_requested": participants,
        "participants_total": participants_total,
        "raw_notes_artifacts": raw_notes_artifacts,
        "recording_handoff": {
            "evidence": evidence_rel,
            "recording_status": recording_status,
            "set_recording_status_after_human_run": "recorded",
            "report_command": "orv benchmark-report .",
            "require_pass_command": "orv benchmark-report . --require-pass",
            "task_entry_fields": ["elapsed_minutes", "status", "notes"],
            "participant_run_fields": [
                "run_id",
                "participant_id",
                "participant_profile",
                "status",
                "started_at",
                "completed_at",
                "raw_notes_artifact",
                "raw_notes_sha256",
            ],
            "observation_fields": [
                "docs_help_lookups",
                "compiler_runtime_errors",
                "first_error_to_fix_minutes",
                "ai_assistance_used",
                "generated_artifact_edits",
                "manual_undocumented_security_steps",
                "manual_config_edits",
                "smoke_test_output",
                "human_evidence_review",
                "failure_classification",
                "participant_notes",
            ],
            "fields_to_record": fields_to_record,
            "success_criteria": success_criteria,
            "raw_notes_rule": "each recorded raw_notes_artifact must point to a retained non-empty relative file under the build directory; generated placeholder fields or generated template instruction prose must be removed if present; Task Notes must contain participant-specific observations; participant_id and run_id must each appear exactly once and match the evidence row; after final notes are written, set raw_notes_sha256 to the retained file hash as sha256:<64 lowercase hex>; if data.smoke_test_output is copied into evidence, it must match the retained deploy/smoke-output.txt artifact or the benchmark report fails",
        },
    }))
}

pub(crate) fn benchmark_prepare_empty_participant_run(index: usize) -> serde_json::Value {
    serde_json::json!({
        "run_id": format!("run-{index}"),
        "participant_id": format!("participant-{index}"),
        "participant_profile": deploy_benchmark::PARTICIPANT_PROFILE_NON_DEVELOPER,
        "status": "todo",
        "started_at": null,
        "completed_at": null,
        "raw_notes_artifact": format!("deploy/evidence/participant-{index}.md"),
        "raw_notes_sha256": null,
    })
}

pub(crate) fn benchmark_prepare_set_string_if_missing(
    run: &mut serde_json::Map<String, serde_json::Value>,
    key: &str,
    fallback: String,
) -> String {
    let existing = run
        .get(key)
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned);
    if let Some(existing) = existing {
        existing
    } else {
        run.insert(key.to_string(), serde_json::json!(fallback));
        fallback
    }
}

pub(crate) fn benchmark_prepare_status(
    run: &mut serde_json::Map<String, serde_json::Value>,
) -> String {
    let status = run
        .get("status")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| "not_recorded".to_string());
    if status == "not_recorded" {
        run.insert("status".to_string(), serde_json::json!("todo"));
        "todo".to_string()
    } else {
        status
    }
}

pub(crate) fn benchmark_prepare_write_participant_notes(
    dir: &Path,
    artifact: &str,
    template: &str,
    participant_id: &str,
    run_id: &str,
) -> anyhow::Result<bool> {
    let path = dir.join(artifact);
    if path.exists() {
        return Ok(false);
    }
    let content = template
        .replace(
            "- participant_id:",
            &format!("- participant_id: {participant_id}"),
        )
        .replace("- run_id:", &format!("- run_id: {run_id}"));
    write_text(&path, &content)?;
    Ok(true)
}

pub(crate) fn benchmark_report_value(dir: &Path) -> anyhow::Result<serde_json::Value> {
    verify_build_dir(dir)?;
    let deploy = read_json_value(&dir.join("deploy").join("manifest.json"))?;
    let server = deploy
        .get("server")
        .filter(|value| !value.is_null())
        .ok_or_else(|| anyhow::anyhow!("deploy manifest server target is required"))?;
    let evidence_rel = json_str(server, "benchmark_evidence", "deploy server")?;
    let evidence = read_json_value(&dir.join(evidence_rel))?;
    let preflight_rel = json_str(&evidence, "preflight", "benchmark evidence")?;
    let benchmark = evidence
        .get("benchmark")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));
    let max_elapsed_minutes = benchmark
        .get("max_elapsed_minutes")
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(300.0);
    let task_report = benchmark_report_tasks(&evidence, max_elapsed_minutes)?;
    let smoke_output_rel = evidence
        .pointer("/artifacts/smoke_output")
        .and_then(serde_json::Value::as_str);
    let smoke_output_contract = evidence
        .get("smoke_output_contract")
        .cloned()
        .or_else(|| smoke_output_rel.map(smoke_output_contract_value))
        .unwrap_or(serde_json::Value::Null);
    let mut data_report = benchmark_report_data(&evidence, Some(dir), smoke_output_rel)?;
    benchmark_report_apply_smoke_route_count_requirement(
        &mut data_report,
        benchmark_expected_route_count(server),
    );
    benchmark_report_apply_smoke_build_dir_requirement(
        &mut data_report,
        benchmark_expected_build_dir(dir),
    );
    benchmark_report_apply_recording_status_requirement(&evidence, &mut data_report);
    benchmark_report_apply_failure_classification_requirement(&task_report, &mut data_report);
    let status = benchmark_report_status_summary(&task_report, &data_report, max_elapsed_minutes);
    Ok(serde_json::json!({
        "schema_version": 1,
        "kind": "orv.benchmark.shop_5h.report",
        "build_dir": dir.display().to_string(),
        "status": status.status,
        "contract_verified": true,
        "evidence": evidence_rel,
        "preflight": preflight_rel,
        "preflight_hash": evidence
            .get("preflight_hash")
            .cloned()
            .unwrap_or(serde_json::Value::Null),
        "smoke_output_contract": smoke_output_contract,
        "recording_status": evidence
            .get("recording_status")
            .cloned()
            .unwrap_or(serde_json::Value::Null),
        "max_elapsed_minutes": max_elapsed_minutes,
        "total_elapsed_minutes": task_report
            .get("total_elapsed_minutes")
            .cloned()
            .unwrap_or(serde_json::Value::Null),
        "time_over_limit": status.time_over_limit,
        "tasks": task_report,
        "data": data_report,
        "automated_gate": benchmark
            .get("automated_gate")
            .cloned()
            .unwrap_or_else(|| serde_json::json!([])),
        "success_criteria": benchmark
            .get("success_criteria")
            .cloned()
            .unwrap_or_else(|| serde_json::json!([])),
        "limitations": [
            "benchmark-report verifies artifact/evidence shape and retained participant notes paths; it does not run the generated smoke test",
            "human-run claims still require reviewers to inspect retained raw participant notes/output content",
        ],
    }))
}

pub(crate) struct BenchmarkReportStatusSummary {
    pub(crate) status: &'static str,
    pub(crate) failed_task_count: usize,
    pub(crate) failed_data_count: usize,
    pub(crate) missing_task_count: usize,
    pub(crate) missing_data_count: usize,
    pub(crate) total_elapsed_minutes: Option<f64>,
    pub(crate) time_over_limit: bool,
}

pub(crate) fn benchmark_report_status_summary(
    task_report: &serde_json::Value,
    data_report: &serde_json::Value,
    max_elapsed_minutes: f64,
) -> BenchmarkReportStatusSummary {
    let failed_task_count = json_array_count(task_report.get("failed_tasks"));
    let failed_data_count = json_array_count(data_report.get("failed_data"));
    let missing_task_count = json_array_count(task_report.get("missing_tasks"));
    let missing_data_count = json_array_count(data_report.get("missing_data"));
    let total_elapsed_minutes = task_report
        .get("total_elapsed_minutes")
        .and_then(serde_json::Value::as_f64);
    let time_over_limit = total_elapsed_minutes.is_some_and(|value| value > max_elapsed_minutes);
    let status = if failed_task_count > 0 || failed_data_count > 0 || time_over_limit {
        "failed"
    } else if missing_task_count > 0 || missing_data_count > 0 {
        "incomplete"
    } else {
        "passed"
    };
    BenchmarkReportStatusSummary {
        status,
        failed_task_count,
        failed_data_count,
        missing_task_count,
        missing_data_count,
        total_elapsed_minutes,
        time_over_limit,
    }
}

pub(crate) fn benchmark_report_tasks(
    evidence: &serde_json::Value,
    max_elapsed_minutes: f64,
) -> anyhow::Result<serde_json::Value> {
    let entries = evidence
        .get("task_entries")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| anyhow::anyhow!("benchmark evidence task_entries must be an array"))?;
    let mut tasks = Vec::with_capacity(entries.len());
    let mut missing_tasks = Vec::new();
    let mut failed_tasks = Vec::new();
    let mut over_budget_tasks = Vec::new();
    let mut recorded_task_count = 0usize;
    let mut total_elapsed_minutes = 0.0f64;
    let mut all_elapsed_recorded = true;
    for entry in entries {
        let task = json_str(entry, "task", "benchmark task")?;
        let target_minutes = entry
            .get("target_minutes")
            .and_then(serde_json::Value::as_f64)
            .ok_or_else(|| anyhow::anyhow!("benchmark task target_minutes must be a number"))?;
        let elapsed_minutes = entry
            .get("elapsed_minutes")
            .and_then(serde_json::Value::as_f64);
        let invalid_elapsed_minutes = elapsed_minutes.is_some_and(|elapsed| elapsed < 0.0);
        let status = entry
            .get("status")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("not_recorded");
        let notes = entry
            .get("notes")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("");
        let status_allowed = benchmark_report_status_is_allowed(status);
        let status_missing = benchmark_report_status_is_missing(status);
        let missing_notes = !status_missing && notes.trim().is_empty();
        let recorded = elapsed_minutes.is_some()
            && !invalid_elapsed_minutes
            && !status_missing
            && status_allowed
            && !missing_notes;
        if recorded {
            recorded_task_count += 1;
        } else {
            all_elapsed_recorded = false;
            missing_tasks.push(serde_json::json!({
                "task": task,
                "status": status,
                "elapsed_minutes": elapsed_minutes,
                "invalid_status": !status.trim().is_empty() && !status_allowed,
                "invalid_elapsed_minutes": invalid_elapsed_minutes,
                "missing_notes": missing_notes,
            }));
        }
        if let Some(elapsed) = elapsed_minutes {
            if elapsed >= 0.0 {
                total_elapsed_minutes += elapsed;
            } else {
                all_elapsed_recorded = false;
                failed_tasks.push(serde_json::json!({
                    "task": task,
                    "status": status,
                    "elapsed_minutes": elapsed,
                    "invalid_elapsed_minutes": true,
                }));
            }
            if elapsed >= 0.0 && elapsed > target_minutes {
                over_budget_tasks.push(serde_json::json!({
                    "task": task,
                    "target_minutes": target_minutes,
                    "elapsed_minutes": elapsed,
                    "over_by_minutes": elapsed - target_minutes,
                }));
            }
        } else {
            all_elapsed_recorded = false;
        }
        if benchmark_report_status_is_failed(status) {
            failed_tasks.push(serde_json::json!({
                "task": task,
                "status": status,
                "elapsed_minutes": elapsed_minutes,
            }));
        }
        tasks.push(serde_json::json!({
            "task": task,
            "target_minutes": target_minutes,
            "elapsed_minutes": elapsed_minutes,
            "status": status,
            "notes": notes,
            "recorded": recorded,
            "invalid_elapsed_minutes": invalid_elapsed_minutes,
            "missing_notes": missing_notes,
        }));
    }
    let total = if all_elapsed_recorded {
        serde_json::json!(total_elapsed_minutes)
    } else {
        serde_json::Value::Null
    };
    Ok(serde_json::json!({
        "task_count": entries.len(),
        "recorded_task_count": recorded_task_count,
        "missing_task_count": missing_tasks.len(),
        "failed_task_count": failed_tasks.len(),
        "max_elapsed_minutes": max_elapsed_minutes,
        "total_elapsed_minutes": total,
        "missing_tasks": missing_tasks,
        "failed_tasks": failed_tasks,
        "over_budget_tasks": over_budget_tasks,
        "entries": tasks,
    }))
}

pub(crate) fn benchmark_report_data(
    evidence: &serde_json::Value,
    build_dir: Option<&Path>,
    smoke_output_rel: Option<&str>,
) -> anyhow::Result<serde_json::Value> {
    let data = evidence
        .get("data")
        .and_then(serde_json::Value::as_object)
        .ok_or_else(|| anyhow::anyhow!("benchmark evidence data must be an object"))?;
    let mut missing = Vec::new();
    let mut failed = Vec::new();
    for key in ["docs_help_lookups", "compiler_runtime_errors"] {
        benchmark_report_apply_nonnegative_integer(data, key, &mut missing, &mut failed);
    }
    let compiler_errors = data
        .get("compiler_runtime_errors")
        .and_then(serde_json::Value::as_u64)
        .or_else(|| {
            data.get("compiler_runtime_errors")
                .and_then(serde_json::Value::as_i64)
                .and_then(|value| u64::try_from(value).ok())
        });
    if compiler_errors.is_some_and(|count| count > 0)
        && data
            .get("first_error_to_fix_minutes")
            .is_none_or(serde_json::Value::is_null)
    {
        missing.push("first_error_to_fix_minutes".to_string());
    }
    if data
        .get("first_error_to_fix_minutes")
        .is_some_and(|value| !value.is_null() && !json_nonnegative_number(value))
    {
        failed.push("first_error_to_fix_minutes.non_negative_number".to_string());
    }
    for key in [
        "ai_assistance_used",
        "generated_artifact_edits",
        "manual_undocumented_security_steps",
    ] {
        benchmark_report_apply_required_false_bool(data, key, &mut missing, &mut failed);
    }
    benchmark_report_apply_manual_config_edits(data, &mut missing, &mut failed);
    let (smoke_test_output, smoke_test_output_source) =
        benchmark_smoke_test_output_value(data, build_dir, smoke_output_rel);
    let smoke_test_output_artifact =
        benchmark_smoke_test_output_artifact(build_dir, smoke_output_rel);
    let smoke_test_output_artifact_match =
        benchmark_smoke_test_output_artifact_match(data, smoke_test_output_artifact.as_deref());
    if smoke_test_output_artifact_match == Some(false) {
        failed.push("smoke_test_output.artifact_match".to_string());
    }
    let expected_smoke_required_markers = deploy_benchmark::smoke_required_markers_value();
    match data.get("smoke_test_required_markers") {
        Some(value) if value == &expected_smoke_required_markers => {}
        Some(value) if value.is_null() => missing.push("smoke_test_required_markers".to_string()),
        Some(_) => failed.push("smoke_test_required_markers.contract".to_string()),
        None => missing.push("smoke_test_required_markers".to_string()),
    }
    if smoke_test_output
        .as_str()
        .is_none_or(|value| value.trim().is_empty())
    {
        missing.push("smoke_test_output".to_string());
    }
    let smoke_test_summary = benchmark_smoke_test_output_summary(&smoke_test_output);
    for marker in smoke_test_summary
        .get("missing_markers")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(serde_json::Value::as_str)
    {
        missing.push(format!("smoke_test_output.{marker}"));
    }
    let participant_summary = benchmark_participant_summary(data)?;
    let recommended_minimum = participant_summary
        .get("recommended_minimum")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0);
    let recorded_run_count = participant_summary
        .get("recorded_run_count")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0);
    let failed_run_count = participant_summary
        .get("failed_run_count")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0);
    benchmark_report_apply_human_evidence_review(data, &mut missing, &mut failed);
    if recorded_run_count < recommended_minimum {
        missing.push("participant_runs.minimum".to_string());
    }
    if failed_run_count > 0 {
        failed.push("participant_runs.failed".to_string());
    }
    let participant_raw_notes_artifacts =
        benchmark_participant_raw_notes_artifacts(&participant_summary, build_dir);
    for run in participant_summary
        .get("runs")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
    {
        let Some(index) = run.get("index").and_then(serde_json::Value::as_u64) else {
            continue;
        };
        for field in run
            .get("missing_fields")
            .and_then(serde_json::Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(serde_json::Value::as_str)
        {
            missing.push(format!("participant_runs[{index}].{field}"));
        }
    }
    for artifact in &participant_raw_notes_artifacts {
        if artifact
            .get("checked")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false)
            && artifact
                .get("retained")
                .and_then(serde_json::Value::as_bool)
                != Some(true)
        {
            if let Some(index) = artifact.get("index").and_then(serde_json::Value::as_u64) {
                missing.push(format!(
                    "participant_runs[{index}].raw_notes_artifact.retained"
                ));
            }
        }
        if artifact
            .get("checked")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false)
            && artifact
                .get("non_empty")
                .and_then(serde_json::Value::as_bool)
                != Some(true)
        {
            if let Some(index) = artifact.get("index").and_then(serde_json::Value::as_u64) {
                missing.push(format!(
                    "participant_runs[{index}].raw_notes_artifact.non_empty"
                ));
            }
        }
        if artifact
            .get("checked")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false)
            && artifact
                .get("template_filled")
                .and_then(serde_json::Value::as_bool)
                != Some(true)
        {
            if let Some(index) = artifact.get("index").and_then(serde_json::Value::as_u64) {
                missing.push(format!(
                    "participant_runs[{index}].raw_notes_artifact.template_filled"
                ));
            }
        }
        if artifact
            .get("checked")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false)
            && artifact
                .get("identity_match")
                .and_then(serde_json::Value::as_bool)
                != Some(true)
        {
            if let Some(index) = artifact.get("index").and_then(serde_json::Value::as_u64) {
                missing.push(format!(
                    "participant_runs[{index}].raw_notes_artifact.identity_match"
                ));
            }
        }
        if artifact
            .get("checked")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false)
            && artifact
                .get("sha256_match")
                .and_then(serde_json::Value::as_bool)
                != Some(true)
        {
            if let Some(index) = artifact.get("index").and_then(serde_json::Value::as_u64) {
                if artifact["expected_sha256"].is_null() {
                    missing.push(format!("participant_runs[{index}].raw_notes_sha256"));
                } else if artifact["actual_sha256"].is_null() {
                    missing.push(format!(
                        "participant_runs[{index}].raw_notes_artifact.sha256_match"
                    ));
                } else {
                    failed.push(format!(
                        "participant_runs[{index}].raw_notes_artifact.sha256_match"
                    ));
                }
            }
        }
    }
    if data
        .get("participant_notes")
        .and_then(serde_json::Value::as_str)
        .is_none_or(|value| value.trim().is_empty())
    {
        missing.push("participant_notes".to_string());
    }
    let failure_classification = benchmark_failure_classification_value(data)?;
    if failed_run_count > 0
        && failure_classification
            .get("primary")
            .is_none_or(serde_json::Value::is_null)
    {
        missing.push("failure_classification.primary".to_string());
    }
    if failure_classification
        .get("primary")
        .and_then(serde_json::Value::as_str)
        == Some("other")
        && failure_classification
            .get("notes")
            .and_then(serde_json::Value::as_str)
            .is_none_or(|value| value.trim().is_empty())
    {
        missing.push("failure_classification.notes".to_string());
    }
    Ok(serde_json::json!({
        "missing_data": missing,
        "failed_data": failed,
        "docs_help_lookups": data.get("docs_help_lookups").cloned().unwrap_or(serde_json::Value::Null),
        "compiler_runtime_errors": data.get("compiler_runtime_errors").cloned().unwrap_or(serde_json::Value::Null),
        "first_error_to_fix_minutes": data.get("first_error_to_fix_minutes").cloned().unwrap_or(serde_json::Value::Null),
        "ai_assistance_used": data.get("ai_assistance_used").cloned().unwrap_or(serde_json::Value::Null),
        "generated_artifact_edits": data.get("generated_artifact_edits").cloned().unwrap_or(serde_json::Value::Null),
        "manual_undocumented_security_steps": data.get("manual_undocumented_security_steps").cloned().unwrap_or(serde_json::Value::Null),
        "manual_config_edits": data.get("manual_config_edits").cloned().unwrap_or_else(|| serde_json::json!([])),
        "human_evidence_review": data.get("human_evidence_review").cloned().unwrap_or(serde_json::Value::Null),
        "smoke_test_required_markers": data
            .get("smoke_test_required_markers")
            .cloned()
            .unwrap_or_else(deploy_benchmark::smoke_required_markers_value),
        "smoke_test_output": smoke_test_output,
        "smoke_test_output_source": smoke_test_output_source,
        "smoke_test_output_artifact_path": smoke_test_output_artifact
            .as_ref()
            .and(smoke_output_rel)
            .map_or(serde_json::Value::Null, serde_json::Value::from),
        "smoke_test_output_artifact_match": smoke_test_output_artifact_match
            .map_or(serde_json::Value::Null, serde_json::Value::from),
        "smoke_test_summary": smoke_test_summary,
        "recommended_participant_count": data
            .get("recommended_participant_count")
            .cloned()
            .unwrap_or(serde_json::Value::Null),
        "participant_runs": data
            .get("participant_runs")
            .cloned()
            .unwrap_or(serde_json::Value::Null),
        "participant_summary": participant_summary,
        "participant_raw_notes_artifacts": participant_raw_notes_artifacts,
        "failure_classification": failure_classification,
        "participant_notes": data.get("participant_notes").cloned().unwrap_or(serde_json::Value::Null),
    }))
}

pub(crate) fn benchmark_report_apply_required_false_bool(
    data: &serde_json::Map<String, serde_json::Value>,
    key: &str,
    missing: &mut Vec<String>,
    failed: &mut Vec<String>,
) {
    match data.get(key).and_then(serde_json::Value::as_bool) {
        Some(false) => {}
        Some(true) => failed.push(key.to_string()),
        None => missing.push(key.to_string()),
    }
}

pub(crate) fn benchmark_report_apply_nonnegative_integer(
    data: &serde_json::Map<String, serde_json::Value>,
    key: &str,
    missing: &mut Vec<String>,
    failed: &mut Vec<String>,
) {
    let Some(value) = data.get(key) else {
        missing.push(key.to_string());
        return;
    };
    if value.is_null() {
        missing.push(key.to_string());
    } else if !json_nonnegative_integer(value) {
        failed.push(format!("{key}.non_negative_integer"));
    }
}

pub(crate) fn benchmark_report_apply_manual_config_edits(
    data: &serde_json::Map<String, serde_json::Value>,
    missing: &mut Vec<String>,
    failed: &mut Vec<String>,
) {
    let Some(edits) = data
        .get("manual_config_edits")
        .and_then(serde_json::Value::as_array)
    else {
        missing.push("manual_config_edits".to_string());
        return;
    };
    for (index, edit) in edits.iter().enumerate() {
        let Some(edit) = edit.as_str() else {
            failed.push(format!("manual_config_edits[{index}].string"));
            continue;
        };
        if edit.trim().is_empty() {
            missing.push(format!("manual_config_edits[{index}].non_empty"));
        }
    }
}

pub(crate) fn benchmark_report_apply_failure_classification_requirement(
    task_report: &serde_json::Value,
    data_report: &mut serde_json::Value,
) {
    let failed_task_count = json_array_count(task_report.get("failed_tasks"));
    let failed_data_count = json_array_count(data_report.get("failed_data"));
    if failed_task_count == 0 && failed_data_count == 0 {
        return;
    }
    let primary_recorded = data_report
        .pointer("/failure_classification/primary")
        .and_then(serde_json::Value::as_str)
        .is_some_and(|value| !value.trim().is_empty());
    if primary_recorded {
        return;
    }
    let Some(missing) = data_report
        .get_mut("missing_data")
        .and_then(serde_json::Value::as_array_mut)
    else {
        return;
    };
    if !missing
        .iter()
        .any(|item| item == "failure_classification.primary")
    {
        missing.push(serde_json::json!("failure_classification.primary"));
    }
}

pub(crate) fn benchmark_report_apply_recording_status_requirement(
    evidence: &serde_json::Value,
    data_report: &mut serde_json::Value,
) {
    if evidence
        .get("recording_status")
        .and_then(serde_json::Value::as_str)
        == Some("recorded")
    {
        return;
    }
    let Some(missing) = data_report
        .get_mut("missing_data")
        .and_then(serde_json::Value::as_array_mut)
    else {
        return;
    };
    if !missing
        .iter()
        .any(|item| item == "recording_status.recorded")
    {
        missing.push(serde_json::json!("recording_status.recorded"));
    }
}

pub(crate) fn benchmark_recording_status_is_allowed(status: &str) -> bool {
    matches!(
        status.trim().to_ascii_lowercase().as_str(),
        "not_recorded" | "sample" | "recorded"
    )
}

pub(crate) fn benchmark_expected_route_count(value: &serde_json::Value) -> Option<u64> {
    value
        .get("routes")
        .and_then(serde_json::Value::as_array)
        .map(|routes| routes.len() as u64)
}

pub(crate) fn benchmark_expected_build_dir(dir: &Path) -> Option<String> {
    std::fs::canonicalize(dir)
        .ok()
        .map(|dir| dir.display().to_string())
}

pub(crate) fn benchmark_report_apply_smoke_build_dir_requirement(
    data_report: &mut serde_json::Value,
    expected_build_dir: Option<String>,
) {
    let Some(expected_build_dir) = expected_build_dir else {
        return;
    };
    if let Some(object) = data_report.as_object_mut() {
        object.insert(
            "expected_build_dir".to_string(),
            serde_json::json!(expected_build_dir),
        );
    }
    let Some(actual_build_dir) = data_report
        .pointer("/smoke_test_summary/build_dir")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
    else {
        return;
    };
    if actual_build_dir == expected_build_dir.as_str() {
        return;
    }
    let Some(missing) = data_report
        .get_mut("missing_data")
        .and_then(serde_json::Value::as_array_mut)
    else {
        return;
    };
    if !missing
        .iter()
        .any(|item| item == "smoke_test_output.build_dir.match")
    {
        missing.push(serde_json::json!("smoke_test_output.build_dir.match"));
    }
}

pub(crate) fn benchmark_report_apply_smoke_route_count_requirement(
    data_report: &mut serde_json::Value,
    expected_route_count: Option<u64>,
) {
    let Some(expected_route_count) = expected_route_count else {
        return;
    };
    if let Some(object) = data_report.as_object_mut() {
        object.insert(
            "expected_server_routes".to_string(),
            serde_json::json!(expected_route_count),
        );
    }
    let Some(actual_route_count) = data_report
        .pointer("/smoke_test_summary/server_routes")
        .and_then(serde_json::Value::as_u64)
    else {
        return;
    };
    if actual_route_count == expected_route_count {
        return;
    }
    let Some(missing) = data_report
        .get_mut("missing_data")
        .and_then(serde_json::Value::as_array_mut)
    else {
        return;
    };
    if !missing
        .iter()
        .any(|item| item == "smoke_test_output.server_routes.match")
    {
        missing.push(serde_json::json!("smoke_test_output.server_routes.match"));
    }
}

pub(crate) fn benchmark_failure_classification_value(
    data: &serde_json::Map<String, serde_json::Value>,
) -> anyhow::Result<serde_json::Value> {
    let failure = data
        .get("failure_classification")
        .and_then(serde_json::Value::as_object)
        .ok_or_else(|| {
            anyhow::anyhow!("benchmark evidence data failure_classification must be an object")
        })?;
    let allowed_categories = failure
        .get("allowed_categories")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| {
            anyhow::anyhow!(
                "benchmark evidence data failure_classification allowed_categories must be an array"
            )
        })?;
    let expected_categories =
        serde_json::json!(deploy_benchmark::FAILURE_CLASSIFICATION_CATEGORIES);
    if failure.get("allowed_categories") != Some(&expected_categories) {
        anyhow::bail!(
            "benchmark evidence data failure_classification allowed_categories must match benchmark contract"
        );
    }
    let primary = failure
        .get("primary")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    if let Some(primary) = primary.as_str() {
        if !deploy_benchmark::FAILURE_CLASSIFICATION_CATEGORIES.contains(&primary) {
            anyhow::bail!(
                "benchmark evidence data failure_classification primary must be an allowed category"
            );
        }
    } else if !primary.is_null() {
        anyhow::bail!(
            "benchmark evidence data failure_classification primary must be null or a string"
        );
    }
    let notes = failure
        .get("notes")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| {
            anyhow::anyhow!("benchmark evidence data failure_classification notes must be a string")
        })?;
    Ok(serde_json::json!({
        "primary": primary,
        "allowed_categories": allowed_categories,
        "notes": notes,
    }))
}

pub(crate) fn benchmark_smoke_test_output_summary(output: &serde_json::Value) -> serde_json::Value {
    let Some(output) = output.as_str().filter(|value| !value.trim().is_empty()) else {
        return serde_json::json!({
            "present": false,
            "passed_marker": false,
            "graph_contract_verified": false,
            "dap_summary_verified": false,
            "dap_source_bundle_verified": false,
            "server_routes": null,
            "trace_stream_requested": null,
            "build_dir": null,
            "base_url": null,
            "client": null,
            "required_markers": deploy_benchmark::smoke_required_markers_value(),
            "missing_markers": [],
            "duplicate_fields": [],
        });
    };
    let fields = benchmark_smoke_test_output_fields(output);
    let duplicate_fields = benchmark_smoke_test_output_duplicate_fields(output);
    let passed_marker = output
        .lines()
        .any(|line| line.trim() == "orv deploy smoke test passed");
    let graph_contract_verified = fields
        .get("graph_contract")
        .is_some_and(|value| value == "verified");
    let dap_summary_verified = fields
        .get("dap_summary")
        .is_some_and(|value| value == "verified");
    let dap_source_bundle_verified = fields
        .get("dap_source_bundle")
        .is_some_and(|value| value == "verified");
    let server_routes = fields
        .get("server_routes")
        .and_then(|value| value.parse::<u64>().ok());
    let trace_stream_requested = fields
        .get("trace_stream_requested")
        .and_then(|value| benchmark_smoke_test_output_bool(value));
    let build_dir = fields
        .get("build_dir")
        .filter(|value| benchmark_smoke_test_output_build_dir_is_valid(value))
        .cloned();
    let base_url = fields
        .get("base_url")
        .filter(|value| benchmark_smoke_test_output_base_url_is_valid(value))
        .cloned();
    let missing_markers = deploy_benchmark::SMOKE_REQUIRED_MARKERS
        .iter()
        .copied()
        .filter(|marker| {
            if *marker != "pass_marker"
                && duplicate_fields
                    .iter()
                    .any(|field| field.as_str() == *marker)
            {
                return true;
            }
            match *marker {
                "pass_marker" => !passed_marker,
                "build_dir" => build_dir.is_none(),
                "base_url" => base_url.is_none(),
                "graph_contract" => !graph_contract_verified,
                "dap_summary" => !dap_summary_verified,
                "dap_source_bundle" => !dap_source_bundle_verified,
                "server_routes" => server_routes.is_none_or(|routes| routes == 0),
                "trace_stream_requested" => trace_stream_requested != Some(true),
                marker => fields
                    .get(marker)
                    .is_none_or(|value| value.trim().is_empty()),
            }
        })
        .collect::<Vec<_>>();
    let client = benchmark_smoke_test_output_client_summary(&fields);
    serde_json::json!({
        "present": true,
        "passed_marker": passed_marker,
        "graph_contract_verified": graph_contract_verified,
        "dap_summary_verified": dap_summary_verified,
        "dap_source_bundle_verified": dap_source_bundle_verified,
        "server_routes": server_routes,
        "trace_stream_requested": trace_stream_requested,
        "build_dir": build_dir,
        "base_url": base_url,
        "client": client,
        "required_markers": deploy_benchmark::smoke_required_markers_value(),
        "missing_markers": missing_markers,
        "duplicate_fields": duplicate_fields,
    })
}

pub(crate) fn benchmark_smoke_test_output_fields(output: &str) -> BTreeMap<String, String> {
    output
        .lines()
        .filter_map(|line| line.split_once('='))
        .map(|(key, value)| (key.trim().to_string(), value.trim().to_string()))
        .filter(|(key, _)| !key.is_empty())
        .collect()
}

pub(crate) fn benchmark_smoke_test_output_duplicate_fields(output: &str) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut duplicates = BTreeSet::new();
    for (key, _) in output.lines().filter_map(|line| line.split_once('=')) {
        let key = key.trim();
        if key.is_empty() {
            continue;
        }
        if !seen.insert(key.to_string()) {
            duplicates.insert(key.to_string());
        }
    }
    duplicates.into_iter().collect()
}

pub(crate) fn benchmark_smoke_test_output_bool(value: &str) -> Option<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" => Some(true),
        "0" | "false" | "no" => Some(false),
        _ => None,
    }
}

pub(crate) fn benchmark_smoke_test_output_base_url_is_valid(value: &str) -> bool {
    let value = value.trim();
    value.starts_with("http://") || value.starts_with("https://")
}

pub(crate) fn benchmark_smoke_test_output_build_dir_is_valid(value: &str) -> bool {
    let value = value.trim();
    !value.is_empty() && Path::new(value).is_absolute()
}

pub(crate) fn benchmark_smoke_test_output_client_summary(
    fields: &BTreeMap<String, String>,
) -> serde_json::Value {
    let mut client = serde_json::Map::new();
    for (field, key) in [
        ("manifest", "client_manifest"),
        ("reactive_plan", "client_reactive_plan"),
        ("page", "client_page"),
        ("loader", "client_loader"),
        ("wasm", "client_wasm"),
    ] {
        if let Some(value) = fields.get(key).filter(|value| !value.trim().is_empty()) {
            client.insert(field.to_string(), serde_json::json!(value));
        }
    }
    if client.is_empty() {
        serde_json::Value::Null
    } else {
        serde_json::Value::Object(client)
    }
}

pub(crate) fn benchmark_smoke_test_output_value(
    data: &serde_json::Map<String, serde_json::Value>,
    build_dir: Option<&Path>,
    smoke_output_rel: Option<&str>,
) -> (serde_json::Value, serde_json::Value) {
    let evidence_value = data
        .get("smoke_test_output")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    if evidence_value
        .as_str()
        .is_some_and(|value| !value.trim().is_empty())
    {
        return (evidence_value, serde_json::json!("evidence"));
    }
    let Some(build_dir) = build_dir else {
        return (evidence_value, serde_json::Value::Null);
    };
    let Some(smoke_output_rel) = smoke_output_rel else {
        return (evidence_value, serde_json::Value::Null);
    };
    let smoke_output_path = build_dir.join(smoke_output_rel);
    match std::fs::read_to_string(&smoke_output_path) {
        Ok(output) if !output.trim().is_empty() => (
            serde_json::json!(output),
            serde_json::json!(smoke_output_rel),
        ),
        _ => (evidence_value, serde_json::Value::Null),
    }
}

pub(crate) fn benchmark_smoke_test_output_artifact(
    build_dir: Option<&Path>,
    smoke_output_rel: Option<&str>,
) -> Option<String> {
    let build_dir = build_dir?;
    let smoke_output_rel = smoke_output_rel?;
    let output = std::fs::read_to_string(build_dir.join(smoke_output_rel)).ok()?;
    (!output.trim().is_empty()).then_some(output)
}

pub(crate) fn benchmark_smoke_test_output_artifact_match(
    data: &serde_json::Map<String, serde_json::Value>,
    artifact_output: Option<&str>,
) -> Option<bool> {
    let evidence_output = data
        .get("smoke_test_output")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())?;
    let artifact_output = artifact_output
        .map(str::trim)
        .filter(|value| !value.is_empty())?;
    Some(evidence_output == artifact_output)
}

pub(crate) fn benchmark_report_status_is_missing(status: &str) -> bool {
    matches!(
        status.trim().to_ascii_lowercase().as_str(),
        "" | "not_recorded" | "missing" | "todo" | "incomplete"
    )
}

pub(crate) fn benchmark_report_status_is_failed(status: &str) -> bool {
    matches!(
        status.trim().to_ascii_lowercase().as_str(),
        "failed" | "fail" | "blocked"
    )
}

pub(crate) fn benchmark_report_status_is_allowed(status: &str) -> bool {
    matches!(
        status.trim().to_ascii_lowercase().as_str(),
        "not_recorded"
            | "missing"
            | "todo"
            | "incomplete"
            | "recorded"
            | "passed"
            | "pass"
            | "failed"
            | "fail"
            | "blocked"
    )
}

pub(crate) fn benchmark_participant_profile_is_allowed(profile: &str) -> bool {
    profile == deploy_benchmark::PARTICIPANT_PROFILE_NON_DEVELOPER
}

pub(crate) fn benchmark_participant_timestamp_is_valid(timestamp: &str) -> bool {
    let bytes = timestamp.as_bytes();
    if bytes.len() != 20 {
        return false;
    }
    if bytes[4] != b'-'
        || bytes[7] != b'-'
        || bytes[10] != b'T'
        || bytes[13] != b':'
        || bytes[16] != b':'
        || bytes[19] != b'Z'
    {
        return false;
    }
    let Some(year) = benchmark_parse_fixed_digits(&bytes[0..4]) else {
        return false;
    };
    let Some(month) = benchmark_parse_fixed_digits(&bytes[5..7]) else {
        return false;
    };
    let Some(day) = benchmark_parse_fixed_digits(&bytes[8..10]) else {
        return false;
    };
    let Some(hour) = benchmark_parse_fixed_digits(&bytes[11..13]) else {
        return false;
    };
    let Some(minute) = benchmark_parse_fixed_digits(&bytes[14..16]) else {
        return false;
    };
    let Some(second) = benchmark_parse_fixed_digits(&bytes[17..19]) else {
        return false;
    };
    year > 0
        && (1..=12).contains(&month)
        && (1..=benchmark_days_in_month(year, month)).contains(&day)
        && hour <= 23
        && minute <= 59
        && second <= 59
}

pub(crate) fn benchmark_parse_fixed_digits(bytes: &[u8]) -> Option<u32> {
    let mut value = 0u32;
    for byte in bytes {
        if !byte.is_ascii_digit() {
            return None;
        }
        value = value
            .saturating_mul(10)
            .saturating_add(u32::from(byte - b'0'));
    }
    Some(value)
}

pub(crate) fn benchmark_days_in_month(year: u32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if benchmark_is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

pub(crate) fn benchmark_is_leap_year(year: u32) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

pub(crate) fn reveal_benchmark_evidence_summary(
    dir: &Path,
    preflight: &serde_json::Value,
) -> anyhow::Result<serde_json::Value> {
    let Some(path) = preflight
        .pointer("/artifacts/benchmark_evidence")
        .and_then(serde_json::Value::as_str)
    else {
        return Ok(serde_json::Value::Null);
    };
    let target_path = dir.join(path);
    if !target_path.is_file() {
        return Ok(serde_json::json!({
            "path": path,
            "exists": false,
        }));
    }
    let evidence = read_json_value(&target_path)?;
    let task_count = evidence
        .get("task_entries")
        .and_then(serde_json::Value::as_array)
        .map_or(0, Vec::len);
    let data_keys = evidence
        .get("data")
        .and_then(serde_json::Value::as_object)
        .map(|data| data.keys().cloned().collect::<Vec<_>>())
        .unwrap_or_default();
    let max_elapsed_minutes = preflight
        .pointer("/benchmark/max_elapsed_minutes")
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(300.0);
    let task_report = benchmark_report_tasks(&evidence, max_elapsed_minutes)?;
    let smoke_output_rel = preflight
        .pointer("/artifacts/smoke_output")
        .and_then(serde_json::Value::as_str);
    let mut data_report = benchmark_report_data(&evidence, Some(dir), smoke_output_rel)?;
    benchmark_report_apply_smoke_route_count_requirement(
        &mut data_report,
        benchmark_expected_route_count(preflight),
    );
    benchmark_report_apply_smoke_build_dir_requirement(
        &mut data_report,
        benchmark_expected_build_dir(dir),
    );
    benchmark_report_apply_recording_status_requirement(&evidence, &mut data_report);
    benchmark_report_apply_failure_classification_requirement(&task_report, &mut data_report);
    let report_status =
        benchmark_report_status_summary(&task_report, &data_report, max_elapsed_minutes);
    Ok(serde_json::json!({
        "path": path,
        "exists": true,
        "kind": evidence
            .get("kind")
            .cloned()
            .unwrap_or(serde_json::Value::Null),
        "preflight": evidence
            .get("preflight")
            .cloned()
            .unwrap_or(serde_json::Value::Null),
        "preflight_hash": evidence
            .get("preflight_hash")
            .cloned()
            .unwrap_or(serde_json::Value::Null),
        "recording_status": evidence
            .get("recording_status")
            .cloned()
            .unwrap_or(serde_json::Value::Null),
        "report_status": report_status.status,
        "max_elapsed_minutes": max_elapsed_minutes,
        "total_elapsed_minutes": report_status
            .total_elapsed_minutes
            .map_or(serde_json::Value::Null, serde_json::Value::from),
        "time_over_limit": report_status.time_over_limit,
        "task_count": task_count,
        "recorded_task_count": task_report
            .get("recorded_task_count")
            .cloned()
            .unwrap_or(serde_json::Value::Null),
        "missing_task_count": report_status.missing_task_count,
        "failed_task_count": report_status.failed_task_count,
        "failed_data_count": report_status.failed_data_count,
        "failed_data": data_report
            .get("failed_data")
            .cloned()
            .unwrap_or_else(|| serde_json::json!([])),
        "missing_data_count": report_status.missing_data_count,
        "missing_data": data_report
            .get("missing_data")
            .cloned()
            .unwrap_or_else(|| serde_json::json!([])),
        "participant_raw_notes_artifacts": data_report
            .get("participant_raw_notes_artifacts")
            .cloned()
            .unwrap_or_else(|| serde_json::json!([])),
        "smoke_test_output_source": data_report
            .get("smoke_test_output_source")
            .cloned()
            .unwrap_or(serde_json::Value::Null),
        "smoke_test_output_artifact_path": data_report
            .get("smoke_test_output_artifact_path")
            .cloned()
            .unwrap_or(serde_json::Value::Null),
        "smoke_test_output_artifact_match": data_report
            .get("smoke_test_output_artifact_match")
            .cloned()
            .unwrap_or(serde_json::Value::Null),
        "smoke_test_required_markers": data_report
            .get("smoke_test_required_markers")
            .cloned()
            .unwrap_or_else(deploy_benchmark::smoke_required_markers_value),
        "smoke_test_summary": data_report
            .get("smoke_test_summary")
            .cloned()
            .unwrap_or(serde_json::Value::Null),
        "data_keys": data_keys,
    }))
}
pub(crate) const DEPLOY_BENCHMARK_EVIDENCE_PATH: &str = "deploy/benchmark-evidence.json";
pub(crate) const DEPLOY_PARTICIPANT_NOTES_TEMPLATE_PATH: &str =
    "deploy/participant-notes-template.md";

pub(crate) fn write_prod_benchmark_evidence_artifact(
    out: &Path,
    path: &str,
    artifacts: &DeployRunbookArtifacts<'_>,
    server_artifact: &orv_compiler::ServerRuntimeArtifact,
    persistence: &DeployPersistence,
    client: &serde_json::Value,
) -> anyhow::Result<()> {
    let evidence = deploy_benchmark_evidence_artifact_value(
        artifacts,
        server_artifact,
        persistence,
        Some(client),
    )?;
    write_json(&out.join(path), &evidence)
}

pub(crate) fn write_prod_participant_notes_template_artifact(
    out: &Path,
    path: &str,
) -> anyhow::Result<()> {
    write_text(&out.join(path), &participant_notes_template_content())
}

pub(crate) fn participant_notes_template_content() -> String {
    r#"# Shop Benchmark Participant Notes

Copy this file for each participant, for example:

```text
deploy/evidence/participant-1.md
deploy/evidence/participant-2.md
```

Then set each `data.participant_runs[].raw_notes_artifact` entry in
`deploy/benchmark-evidence.json` to that relative path.

## Participant

- participant_id:
- run_id:
- participant_profile: non_developer
- started_at: YYYY-MM-DDTHH:MM:SSZ
- completed_at: YYYY-MM-DDTHH:MM:SSZ

## Task Notes

Record timestamps, blockers, docs/help lookups, compiler/runtime errors, first
error-to-fix time, manual config edits, and confusing concepts.

## Evidence Review

- generated_artifact_edits: false
- manual_undocumented_security_steps: false
- ai_assistance_used: false
- failure_classification.primary:
- failure_classification.notes:
"#
    .to_string()
}

pub(crate) fn deploy_benchmark_evidence_artifact_value(
    artifacts: &DeployRunbookArtifacts<'_>,
    server_artifact: &orv_compiler::ServerRuntimeArtifact,
    persistence: &DeployPersistence,
    client: Option<&serde_json::Value>,
) -> anyhow::Result<serde_json::Value> {
    let preflight =
        deploy_preflight_artifact_value(artifacts, server_artifact, persistence, client);
    let preflight_hash = stable_json_hash(&preflight)?;
    Ok(serde_json::json!({
        "schema_version": 1,
        "kind": "orv.benchmark.shop_5h.evidence",
        "preflight": artifacts.preflight,
        "preflight_hash": preflight_hash,
        "benchmark": deploy_preflight_benchmark_value(),
        "commands": deploy_preflight_commands_value(artifacts),
        "artifacts": deploy_preflight_artifacts_value(artifacts),
        "smoke_output_contract": deploy_smoke_output_contract_value(artifacts),
        "recording_status": "not_recorded",
        "task_entries": deploy_benchmark::evidence_task_entries_value(),
        "data": deploy_benchmark::evidence_data_value(),
    }))
}

pub(crate) fn deploy_preflight_benchmark_value() -> serde_json::Value {
    deploy_benchmark::preflight_contract_value()
}
