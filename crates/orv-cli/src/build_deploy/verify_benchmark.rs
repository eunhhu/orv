use super::*;

pub(crate) fn verify_deploy_benchmark_evidence_artifact(
    dir: &Path,
    path: &str,
    artifacts: &DeployRunbookArtifacts<'_>,
    artifact: &orv_compiler::ServerRuntimeArtifact,
    persistence: &DeployPersistence,
    client: Option<&serde_json::Value>,
) -> anyhow::Result<()> {
    let evidence_path = dir.join(path);
    if !evidence_path.is_file() {
        anyhow::bail!(
            "missing deploy benchmark evidence artifact: {}",
            evidence_path.display()
        );
    }
    let evidence = read_json_value(&evidence_path)?;
    if evidence
        .get("schema_version")
        .and_then(serde_json::Value::as_u64)
        != Some(1)
    {
        anyhow::bail!("deploy benchmark evidence schema_version must be 1");
    }
    if json_str(&evidence, "kind", "deploy benchmark evidence")? != "orv.benchmark.shop_5h.evidence"
    {
        anyhow::bail!("deploy benchmark evidence kind must be orv.benchmark.shop_5h.evidence");
    }
    verify_json_pointer_str(
        &evidence,
        "/preflight",
        artifacts.preflight,
        "deploy benchmark evidence preflight",
    )?;
    let expected_preflight =
        deploy_preflight_artifact_value(artifacts, artifact, persistence, client);
    let expected_preflight_hash = stable_json_hash(&expected_preflight)?;
    verify_json_pointer_str(
        &evidence,
        "/preflight_hash",
        &expected_preflight_hash,
        "deploy benchmark evidence preflight_hash",
    )?;
    if evidence.get("benchmark") != Some(&deploy_preflight_benchmark_value()) {
        anyhow::bail!("deploy benchmark evidence benchmark does not match 5-hour shop contract");
    }
    if evidence.get("commands") != Some(&deploy_preflight_commands_value(artifacts)) {
        anyhow::bail!("deploy benchmark evidence commands do not match deploy preflight");
    }
    if evidence.get("artifacts") != Some(&deploy_preflight_artifacts_value(artifacts)) {
        anyhow::bail!("deploy benchmark evidence artifacts do not match deploy preflight");
    }
    verify_deploy_smoke_output_contract_keys(
        evidence.get("smoke_output_contract").ok_or_else(|| {
            anyhow::anyhow!("deploy benchmark evidence smoke_output_contract must be an object")
        })?,
        "deploy benchmark evidence smoke_output_contract",
    )?;
    if evidence.get("smoke_output_contract") != Some(&deploy_smoke_output_contract_value(artifacts))
    {
        anyhow::bail!(
            "deploy benchmark evidence smoke_output_contract must match smoke output contract"
        );
    }
    verify_deploy_benchmark_evidence_task_entries(&evidence)?;
    verify_deploy_benchmark_evidence_data(&evidence)?;
    let recording_status = evidence
        .get("recording_status")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| {
            anyhow::anyhow!("deploy benchmark evidence recording_status must be a string")
        })?;
    if !benchmark_recording_status_is_allowed(recording_status) {
        anyhow::bail!(
            "deploy benchmark evidence recording_status must be not_recorded, sample, or recorded"
        );
    }
    verify_json_object_keys_exact(
        &evidence,
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
        "deploy benchmark evidence",
    )?;
    Ok(())
}

pub(super) fn verify_build_recorded_benchmark_evidence_artifacts(dir: &Path) -> anyhow::Result<()> {
    let deploy_path = dir.join("deploy").join("manifest.json");
    if !deploy_path.is_file() {
        return Ok(());
    }
    let deploy = read_json_value(&deploy_path)?;
    let Some(server) = deploy.get("server").filter(|value| !value.is_null()) else {
        return Ok(());
    };
    let evidence_rel = json_str(server, "benchmark_evidence", "deploy server")?;
    let evidence_path = dir.join(evidence_rel);
    if !evidence_path.is_file() {
        return Ok(());
    }
    let evidence = read_json_value(&evidence_path)?;
    verify_deploy_benchmark_evidence_data_with_artifacts(&evidence, Some(dir))
}

pub(crate) fn verify_deploy_benchmark_evidence_task_entries(
    evidence: &serde_json::Value,
) -> anyhow::Result<()> {
    let entries = evidence
        .get("task_entries")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| {
            anyhow::anyhow!("deploy benchmark evidence task_entries must be an array")
        })?;
    let expected = deploy_benchmark::evidence_task_entries_value();
    let expected_entries = expected
        .as_array()
        .expect("benchmark evidence task entries are an array");
    if entries.len() != expected_entries.len() {
        anyhow::bail!("deploy benchmark evidence task_entries do not match 5-hour time budget");
    }
    for (index, (entry, expected)) in entries.iter().zip(expected_entries.iter()).enumerate() {
        let context = format!("deploy benchmark evidence task_entries[{index}]");
        verify_json_object_keys_exact(
            entry,
            &[
                "task",
                "target_minutes",
                "elapsed_minutes",
                "status",
                "notes",
            ],
            &context,
        )?;
        if entry.get("task") != expected.get("task")
            || entry.get("target_minutes") != expected.get("target_minutes")
        {
            anyhow::bail!("deploy benchmark evidence task_entries do not match 5-hour time budget");
        }
        if !entry
            .as_object()
            .is_some_and(|object| object.contains_key("elapsed_minutes"))
        {
            anyhow::bail!(
                "deploy benchmark evidence task_entries[{index}] must include elapsed_minutes"
            );
        }
        if !entry
            .get("elapsed_minutes")
            .is_some_and(json_null_or_nonnegative_number)
        {
            anyhow::bail!(
                "deploy benchmark evidence task_entries[{index}] elapsed_minutes must be null or a non-negative number"
            );
        }
        if entry
            .get("status")
            .and_then(serde_json::Value::as_str)
            .is_none()
        {
            anyhow::bail!(
                "deploy benchmark evidence task_entries[{index}] status must be a string"
            );
        }
        let status = entry
            .get("status")
            .and_then(serde_json::Value::as_str)
            .expect("benchmark task status is a string");
        if !benchmark_report_status_is_allowed(status) {
            anyhow::bail!(
                "deploy benchmark evidence task_entries[{index}] status must be an allowed benchmark status"
            );
        }
        let Some(notes) = entry.get("notes").and_then(serde_json::Value::as_str) else {
            anyhow::bail!("deploy benchmark evidence task_entries[{index}] notes must be a string");
        };
        if !benchmark_report_status_is_missing(status) && notes.trim().is_empty() {
            anyhow::bail!(
                "deploy benchmark evidence task_entries[{index}] notes must not be blank"
            );
        }
    }
    Ok(())
}

pub(crate) fn verify_deploy_benchmark_evidence_data(
    evidence: &serde_json::Value,
) -> anyhow::Result<()> {
    verify_deploy_benchmark_evidence_data_with_artifacts(evidence, None)
}

pub(crate) fn verify_deploy_benchmark_evidence_data_with_artifacts(
    evidence: &serde_json::Value,
    build_dir: Option<&Path>,
) -> anyhow::Result<()> {
    let data = evidence
        .get("data")
        .and_then(serde_json::Value::as_object)
        .ok_or_else(|| anyhow::anyhow!("deploy benchmark evidence data must be an object"))?;
    for key in [
        "elapsed_time_per_task",
        "docs_help_lookups",
        "compiler_runtime_errors",
        "first_error_to_fix_minutes",
        "ai_assistance_used",
        "generated_artifact_edits",
        "manual_undocumented_security_steps",
        "manual_config_edits",
        "smoke_test_output",
        "smoke_test_required_markers",
        "recommended_participant_count",
        "participant_runs",
        "human_evidence_review",
        "failure_classification",
        "participant_notes",
    ] {
        if !data.contains_key(key) {
            anyhow::bail!("deploy benchmark evidence data must include {key}");
        }
    }
    verify_json_object_keys_exact(
        evidence
            .get("data")
            .expect("benchmark evidence data exists"),
        &[
            "elapsed_time_per_task",
            "docs_help_lookups",
            "compiler_runtime_errors",
            "first_error_to_fix_minutes",
            "ai_assistance_used",
            "generated_artifact_edits",
            "manual_undocumented_security_steps",
            "manual_config_edits",
            "smoke_test_output",
            "smoke_test_required_markers",
            "recommended_participant_count",
            "participant_runs",
            "human_evidence_review",
            "failure_classification",
            "participant_notes",
        ],
        "deploy benchmark evidence data",
    )?;
    if data
        .get("elapsed_time_per_task")
        .and_then(serde_json::Value::as_str)
        != Some("task_entries[*].elapsed_minutes")
    {
        anyhow::bail!(
            "deploy benchmark evidence data elapsed_time_per_task must reference task_entries"
        );
    }
    for key in ["docs_help_lookups", "compiler_runtime_errors"] {
        if !data.get(key).is_some_and(json_null_or_nonnegative_integer) {
            anyhow::bail!(
                "deploy benchmark evidence data {key} must be null or a non-negative integer"
            );
        }
    }
    if !data
        .get("first_error_to_fix_minutes")
        .is_some_and(json_null_or_nonnegative_number)
    {
        anyhow::bail!(
            "deploy benchmark evidence data first_error_to_fix_minutes must be null or a non-negative number"
        );
    }
    for key in [
        "ai_assistance_used",
        "generated_artifact_edits",
        "manual_undocumented_security_steps",
    ] {
        if !data.get(key).is_some_and(json_null_or_bool) {
            anyhow::bail!("deploy benchmark evidence data {key} must be null or a bool");
        }
    }
    if !data
        .get("manual_config_edits")
        .is_some_and(serde_json::Value::is_array)
    {
        anyhow::bail!("deploy benchmark evidence data manual_config_edits must be an array");
    }
    for (index, edit) in data
        .get("manual_config_edits")
        .and_then(serde_json::Value::as_array)
        .expect("manual config edits is an array")
        .iter()
        .enumerate()
    {
        let Some(edit) = edit.as_str() else {
            anyhow::bail!(
                "deploy benchmark evidence data manual_config_edits[{index}] must be a string"
            );
        };
        if edit.trim().is_empty() {
            anyhow::bail!(
                "deploy benchmark evidence data manual_config_edits[{index}] must not be blank"
            );
        }
    }
    if !data
        .get("smoke_test_output")
        .is_some_and(json_null_or_string)
    {
        anyhow::bail!("deploy benchmark evidence data smoke_test_output must be null or a string");
    }
    let expected_smoke_required_markers = deploy_benchmark::smoke_required_markers_value();
    if data.get("smoke_test_required_markers") != Some(&expected_smoke_required_markers) {
        anyhow::bail!(
            "deploy benchmark evidence data smoke_test_required_markers must match smoke output contract"
        );
    }
    let recommended = data
        .get("recommended_participant_count")
        .and_then(serde_json::Value::as_object)
        .ok_or_else(|| {
            anyhow::anyhow!(
                "deploy benchmark evidence data recommended_participant_count must be an object"
            )
        })?;
    verify_json_object_keys_exact(
        data.get("recommended_participant_count")
            .expect("benchmark evidence recommended participant count exists"),
        &["minimum", "target"],
        "deploy benchmark evidence data recommended_participant_count",
    )?;
    let minimum = json_u64_value(recommended.get("minimum")).ok_or_else(|| {
        anyhow::anyhow!(
            "deploy benchmark evidence data recommended_participant_count minimum must be an integer"
        )
    })?;
    let target = json_u64_value(recommended.get("target")).ok_or_else(|| {
        anyhow::anyhow!(
            "deploy benchmark evidence data recommended_participant_count target must be an integer"
        )
    })?;
    if minimum == 0 || target < minimum {
        anyhow::bail!(
            "deploy benchmark evidence data recommended_participant_count target must be >= minimum > 0"
        );
    }
    if minimum != deploy_benchmark::RECOMMENDED_PARTICIPANT_MINIMUM
        || target != deploy_benchmark::RECOMMENDED_PARTICIPANT_TARGET
    {
        anyhow::bail!(
            "deploy benchmark evidence data recommended_participant_count must match benchmark contract"
        );
    }
    let participant_runs = data
        .get("participant_runs")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| {
            anyhow::anyhow!("deploy benchmark evidence data participant_runs must be an array")
        })?;
    for (index, run) in participant_runs.iter().enumerate() {
        let run = run.as_object().ok_or_else(|| {
            anyhow::anyhow!(
                "deploy benchmark evidence data participant_runs[{index}] must be an object"
            )
        })?;
        let context = format!("deploy benchmark evidence data participant_runs[{index}]");
        verify_json_object_keys_exact(
            participant_runs.get(index).expect("participant run exists"),
            &[
                "run_id",
                "participant_id",
                "participant_profile",
                "status",
                "started_at",
                "completed_at",
                "raw_notes_artifact",
                "raw_notes_sha256",
            ],
            &context,
        )?;
        for key in [
            "run_id",
            "participant_id",
            "started_at",
            "completed_at",
            "raw_notes_artifact",
            "raw_notes_sha256",
        ] {
            if !run.get(key).is_some_and(json_null_or_string) {
                anyhow::bail!(
                    "deploy benchmark evidence data participant_runs[{index}] {key} must be null or a string"
                );
            }
        }
        for key in ["started_at", "completed_at"] {
            if let Some(timestamp) = run.get(key).and_then(serde_json::Value::as_str) {
                if !benchmark_participant_timestamp_is_valid(timestamp) {
                    anyhow::bail!(
                        "deploy benchmark evidence data participant_runs[{index}] {key} must be null or an RFC3339 UTC timestamp"
                    );
                }
            }
        }
        let started_at = run.get("started_at").and_then(serde_json::Value::as_str);
        let completed_at = run.get("completed_at").and_then(serde_json::Value::as_str);
        if let (Some(started_at), Some(completed_at)) = (started_at, completed_at) {
            if completed_at < started_at {
                anyhow::bail!(
                    "deploy benchmark evidence data participant_runs[{index}] completed_at must be >= started_at"
                );
            }
        }
        if let Some(raw_notes_artifact) = run
            .get("raw_notes_artifact")
            .and_then(serde_json::Value::as_str)
        {
            if !benchmark_raw_notes_artifact_path_is_safe(raw_notes_artifact) {
                anyhow::bail!(
                    "deploy benchmark evidence data participant_runs[{index}] raw_notes_artifact must be null or a relative path under the build directory"
                );
            }
        }
        if let Some(raw_notes_sha256) = run
            .get("raw_notes_sha256")
            .and_then(serde_json::Value::as_str)
        {
            if !benchmark_raw_notes_sha256_is_valid(raw_notes_sha256) {
                anyhow::bail!(
                    "deploy benchmark evidence data participant_runs[{index}] raw_notes_sha256 must be null or sha256:<64 lowercase hex>"
                );
            }
        }
        for key in ["participant_profile", "status"] {
            if !run.get(key).is_some_and(serde_json::Value::is_string) {
                anyhow::bail!(
                    "deploy benchmark evidence data participant_runs[{index}] {key} must be a string"
                );
            }
        }
        let participant_profile = run
            .get("participant_profile")
            .and_then(serde_json::Value::as_str)
            .expect("participant profile is a string");
        if !benchmark_participant_profile_is_allowed(participant_profile) {
            anyhow::bail!(
                "deploy benchmark evidence data participant_runs[{index}] participant_profile must be non_developer"
            );
        }
        let status = run
            .get("status")
            .and_then(serde_json::Value::as_str)
            .expect("participant status is a string");
        if !benchmark_report_status_is_allowed(status) {
            anyhow::bail!(
                "deploy benchmark evidence data participant_runs[{index}] status must be an allowed benchmark status"
            );
        }
    }
    let mut run_ids = BTreeSet::new();
    let mut participant_ids = BTreeSet::new();
    for (index, run) in participant_runs.iter().enumerate() {
        let run = run
            .as_object()
            .expect("participant run is already an object");
        let status = run
            .get("status")
            .and_then(serde_json::Value::as_str)
            .expect("participant status is a string");
        if benchmark_report_status_is_missing(status) {
            continue;
        }
        if let Some(run_id) = run
            .get("run_id")
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            if !run_ids.insert(run_id.to_string()) {
                anyhow::bail!(
                    "deploy benchmark evidence data participant_runs[{index}] run_id must be unique"
                );
            }
        }
        if let Some(participant_id) = run
            .get("participant_id")
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            if !participant_ids.insert(participant_id.to_string()) {
                anyhow::bail!(
                    "deploy benchmark evidence data participant_runs[{index}] participant_id must be unique"
                );
            }
        }
    }
    let require_recorded_review = evidence
        .get("recording_status")
        .and_then(serde_json::Value::as_str)
        .is_some_and(|status| status.trim().eq_ignore_ascii_case("recorded"));
    verify_deploy_benchmark_human_evidence_review(data, require_recorded_review)?;
    if require_recorded_review {
        if let Some(build_dir) = build_dir {
            verify_deploy_benchmark_recorded_raw_notes_artifacts(data, build_dir)?;
        }
    }
    let failure = data
        .get("failure_classification")
        .and_then(serde_json::Value::as_object)
        .ok_or_else(|| {
            anyhow::anyhow!(
                "deploy benchmark evidence data failure_classification must be an object"
            )
        })?;
    verify_json_object_keys_exact(
        data.get("failure_classification")
            .expect("benchmark evidence failure classification exists"),
        &["primary", "allowed_categories", "notes"],
        "deploy benchmark evidence data failure_classification",
    )?;
    let expected_categories =
        serde_json::json!(deploy_benchmark::FAILURE_CLASSIFICATION_CATEGORIES);
    if failure.get("allowed_categories") != Some(&expected_categories) {
        anyhow::bail!(
            "deploy benchmark evidence data failure_classification allowed_categories must match benchmark contract"
        );
    }
    let primary_category = if let Some(primary) =
        failure.get("primary").filter(|value| !value.is_null())
    {
        let primary = primary.as_str().ok_or_else(|| {
            anyhow::anyhow!(
                "deploy benchmark evidence data failure_classification primary must be null or a string"
            )
        })?;
        if !deploy_benchmark::FAILURE_CLASSIFICATION_CATEGORIES.contains(&primary) {
            anyhow::bail!(
                "deploy benchmark evidence data failure_classification primary must be an allowed category"
            );
        }
        Some(primary)
    } else {
        None
    };
    if !failure
        .get("notes")
        .is_some_and(serde_json::Value::is_string)
    {
        anyhow::bail!(
            "deploy benchmark evidence data failure_classification notes must be a string"
        );
    }
    if primary_category == Some("other")
        && failure
            .get("notes")
            .and_then(serde_json::Value::as_str)
            .is_none_or(|value| value.trim().is_empty())
    {
        anyhow::bail!(
            "deploy benchmark evidence data failure_classification notes must explain other"
        );
    }
    verify_deploy_benchmark_failed_participant_classification(data, primary_category)?;
    if !data
        .get("participant_notes")
        .is_some_and(serde_json::Value::is_string)
    {
        anyhow::bail!("deploy benchmark evidence data participant_notes must be a string");
    }
    Ok(())
}

pub(super) fn verify_deploy_benchmark_recorded_raw_notes_artifacts(
    data: &serde_json::Map<String, serde_json::Value>,
    build_dir: &Path,
) -> anyhow::Result<()> {
    let participant_summary = benchmark_participant_summary(data)?;
    for run in participant_summary
        .get("runs")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
    {
        let index = run
            .get("index")
            .and_then(serde_json::Value::as_u64)
            .ok_or_else(|| anyhow::anyhow!("benchmark participant summary index missing"))?;
        let status = run
            .get("status")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("not_recorded");
        if benchmark_report_status_is_missing(status) {
            continue;
        }
        if let Some(field) = run
            .get("missing_fields")
            .and_then(serde_json::Value::as_array)
            .into_iter()
            .flatten()
            .find_map(serde_json::Value::as_str)
        {
            anyhow::bail!(
                "deploy benchmark evidence data participant_runs[{index}] {field} must be recorded"
            );
        }
    }

    for artifact in benchmark_participant_raw_notes_artifacts(&participant_summary, Some(build_dir))
    {
        if artifact.get("checked").and_then(serde_json::Value::as_bool) != Some(true) {
            continue;
        }
        let index = artifact
            .get("index")
            .and_then(serde_json::Value::as_u64)
            .ok_or_else(|| anyhow::anyhow!("benchmark raw-notes artifact index missing"))?;
        if artifact
            .get("retained")
            .and_then(serde_json::Value::as_bool)
            != Some(true)
        {
            anyhow::bail!(
                "deploy benchmark evidence data participant_runs[{index}] raw_notes_artifact must point to a retained file"
            );
        }
        if artifact
            .get("non_empty")
            .and_then(serde_json::Value::as_bool)
            != Some(true)
        {
            anyhow::bail!(
                "deploy benchmark evidence data participant_runs[{index}] raw_notes_artifact must be non-empty"
            );
        }
        if artifact
            .get("template_filled")
            .and_then(serde_json::Value::as_bool)
            != Some(true)
        {
            anyhow::bail!(
                "deploy benchmark evidence data participant_runs[{index}] raw_notes_artifact must contain filled participant notes"
            );
        }
        if artifact
            .get("identity_match")
            .and_then(serde_json::Value::as_bool)
            != Some(true)
        {
            anyhow::bail!(
                "deploy benchmark evidence data participant_runs[{index}] raw_notes_artifact participant_id/run_id must match exactly once"
            );
        }
        if artifact
            .get("sha256_match")
            .and_then(serde_json::Value::as_bool)
            != Some(true)
        {
            if artifact
                .get("expected_sha256")
                .is_none_or(serde_json::Value::is_null)
            {
                anyhow::bail!(
                    "deploy benchmark evidence data participant_runs[{index}] raw_notes_sha256 must be recorded for retained raw notes"
                );
            }
            anyhow::bail!(
                "deploy benchmark evidence data participant_runs[{index}] raw_notes_sha256 must match retained raw notes"
            );
        }
    }
    Ok(())
}

pub(crate) fn verify_deploy_participant_notes_template_artifact(
    dir: &Path,
    path: &str,
) -> anyhow::Result<()> {
    let template_path = dir.join(path);
    if !template_path.is_file() {
        anyhow::bail!(
            "missing deploy participant notes template: {}",
            template_path.display()
        );
    }
    let template = std::fs::read_to_string(&template_path)
        .map_err(|e| anyhow::anyhow!("failed to read {}: {e}", template_path.display()))?;
    let expected = participant_notes_template_content();
    if template != expected {
        anyhow::bail!("deploy participant notes template must match generated artifact");
    }
    for marker in [
        "data.participant_runs[].raw_notes_artifact",
        "participant_profile: non_developer",
        "YYYY-MM-DDTHH:MM:SSZ",
        "generated_artifact_edits: false",
        "manual_undocumented_security_steps: false",
        "ai_assistance_used: false",
    ] {
        if !template.contains(marker) {
            anyhow::bail!("deploy participant notes template must document {marker}");
        }
    }
    Ok(())
}
