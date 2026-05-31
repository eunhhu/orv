use super::*;

pub(crate) fn benchmark_participant_summary(
    data: &serde_json::Map<String, serde_json::Value>,
) -> anyhow::Result<serde_json::Value> {
    let recommended = data
        .get("recommended_participant_count")
        .and_then(serde_json::Value::as_object)
        .ok_or_else(|| {
            anyhow::anyhow!(
                "benchmark evidence data recommended_participant_count must be an object"
            )
        })?;
    let recommended_minimum = json_u64_value(recommended.get("minimum")).ok_or_else(|| {
        anyhow::anyhow!(
            "benchmark evidence data recommended_participant_count minimum must be an integer"
        )
    })?;
    let recommended_target = json_u64_value(recommended.get("target")).ok_or_else(|| {
        anyhow::anyhow!(
            "benchmark evidence data recommended_participant_count target must be an integer"
        )
    })?;
    if recommended_minimum != deploy_benchmark::RECOMMENDED_PARTICIPANT_MINIMUM
        || recommended_target != deploy_benchmark::RECOMMENDED_PARTICIPANT_TARGET
    {
        anyhow::bail!(
            "benchmark evidence data recommended_participant_count must match benchmark contract"
        );
    }
    let runs = data
        .get("participant_runs")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| {
            anyhow::anyhow!("benchmark evidence data participant_runs must be an array")
        })?;
    let mut recorded_run_count = 0u64;
    let mut missing_run_count = 0u64;
    let mut failed_run_count = 0u64;
    let mut run_summaries = Vec::with_capacity(runs.len());
    let mut seen_run_ids = BTreeSet::new();
    let mut seen_participant_ids = BTreeSet::new();
    for (index, run) in runs.iter().enumerate() {
        let object = run.as_object().ok_or_else(|| {
            anyhow::anyhow!("benchmark evidence data participant_runs[{index}] must be an object")
        })?;
        let status = object
            .get("status")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("not_recorded");
        let status_missing = benchmark_report_status_is_missing(status);
        let status_allowed = benchmark_report_status_is_allowed(status);
        let status_failed = benchmark_report_status_is_failed(status);
        let participant_profile = object
            .get("participant_profile")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("");
        let participant_profile_allowed =
            benchmark_participant_profile_is_allowed(participant_profile);
        let started_at = object
            .get("started_at")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("");
        let completed_at = object
            .get("completed_at")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("");
        let run_id = object
            .get("run_id")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("");
        let participant_id = object
            .get("participant_id")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("");
        let mut missing_fields = Vec::new();
        if !status.trim().is_empty() && !status_allowed {
            missing_fields.push("status.allowed");
        }
        if participant_profile.trim().is_empty() {
            missing_fields.push("participant_profile");
        } else if !participant_profile_allowed {
            missing_fields.push("participant_profile.allowed");
        }
        if !status_missing {
            benchmark_participant_recording_missing_fields(object, &mut missing_fields);
            if !started_at.trim().is_empty()
                && !benchmark_participant_timestamp_is_valid(started_at)
            {
                missing_fields.push("started_at.utc");
            }
            if !completed_at.trim().is_empty()
                && !benchmark_participant_timestamp_is_valid(completed_at)
            {
                missing_fields.push("completed_at.utc");
            }
            if benchmark_participant_timestamp_is_valid(started_at)
                && benchmark_participant_timestamp_is_valid(completed_at)
                && completed_at < started_at
            {
                missing_fields.push("completed_at.order");
            }
            if !run_id.trim().is_empty() && !seen_run_ids.insert(run_id.trim().to_string()) {
                missing_fields.push("run_id.unique");
            }
            if !participant_id.trim().is_empty()
                && !seen_participant_ids.insert(participant_id.trim().to_string())
            {
                missing_fields.push("participant_id.unique");
            }
        }
        let recorded = !status_missing
            && status_allowed
            && participant_profile_allowed
            && missing_fields.is_empty();
        if recorded {
            recorded_run_count += 1;
        } else {
            missing_run_count += 1;
        }
        if status_failed {
            failed_run_count += 1;
        }
        run_summaries.push(benchmark_participant_run_summary(
            index,
            object,
            status,
            recorded,
            status_failed,
            missing_fields,
        ));
    }
    Ok(serde_json::json!({
        "recommended_minimum": recommended_minimum,
        "recommended_target": recommended_target,
        "run_count": runs.len(),
        "recorded_run_count": recorded_run_count,
        "missing_run_count": missing_run_count,
        "failed_run_count": failed_run_count,
        "runs": run_summaries,
    }))
}

fn benchmark_participant_recording_missing_fields(
    object: &serde_json::Map<String, serde_json::Value>,
    missing_fields: &mut Vec<&'static str>,
) {
    for field in [
        "run_id",
        "participant_id",
        "started_at",
        "completed_at",
        "raw_notes_artifact",
    ] {
        if object
            .get(field)
            .and_then(serde_json::Value::as_str)
            .is_none_or(|value| value.trim().is_empty())
        {
            missing_fields.push(field);
        }
    }
    if let Some(raw_notes_sha256) = object
        .get("raw_notes_sha256")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        if !benchmark_raw_notes_sha256_is_valid(raw_notes_sha256) {
            missing_fields.push("raw_notes_sha256.format");
        }
    }
}

fn benchmark_participant_run_summary(
    index: usize,
    object: &serde_json::Map<String, serde_json::Value>,
    status: &str,
    recorded: bool,
    status_failed: bool,
    missing_fields: Vec<&'static str>,
) -> serde_json::Value {
    serde_json::json!({
        "index": index,
        "run_id": object
            .get("run_id")
            .cloned()
            .unwrap_or(serde_json::Value::Null),
        "participant_id": object
            .get("participant_id")
            .cloned()
            .unwrap_or(serde_json::Value::Null),
        "participant_profile": object
            .get("participant_profile")
            .cloned()
            .unwrap_or(serde_json::Value::Null),
        "status": status,
        "recorded": recorded,
        "failed": status_failed,
        "raw_notes_artifact": object
            .get("raw_notes_artifact")
            .cloned()
            .unwrap_or(serde_json::Value::Null),
        "raw_notes_sha256": object
            .get("raw_notes_sha256")
            .cloned()
            .unwrap_or(serde_json::Value::Null),
        "missing_fields": missing_fields,
    })
}
