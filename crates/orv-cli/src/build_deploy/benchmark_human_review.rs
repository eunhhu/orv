use super::*;

pub(crate) fn benchmark_report_apply_human_evidence_review(
    data: &serde_json::Map<String, serde_json::Value>,
    missing: &mut Vec<String>,
    failed: &mut Vec<String>,
) {
    let Some(review) = data
        .get("human_evidence_review")
        .and_then(serde_json::Value::as_object)
    else {
        missing.push("human_evidence_review".to_string());
        return;
    };
    if review
        .get("reviewer")
        .and_then(serde_json::Value::as_str)
        .is_none_or(|value| value.trim().is_empty())
    {
        missing.push("human_evidence_review.reviewer".to_string());
    }
    let reviewed_at = match review.get("reviewed_at") {
        Some(value) if value.is_null() => {
            missing.push("human_evidence_review.reviewed_at".to_string());
            None
        }
        Some(value) => match value.as_str() {
            Some(timestamp) if benchmark_participant_timestamp_is_valid(timestamp) => {
                Some(timestamp)
            }
            Some(_) => {
                failed.push("human_evidence_review.reviewed_at.utc".to_string());
                None
            }
            None => {
                failed.push("human_evidence_review.reviewed_at.string".to_string());
                None
            }
        },
        None => {
            missing.push("human_evidence_review.reviewed_at".to_string());
            None
        }
    };
    if reviewed_at.is_some_and(|reviewed_at| {
        benchmark_review_has_participant_completion_after_reviewed_at(data, reviewed_at)
    }) {
        failed.push("human_evidence_review.reviewed_at.after_participants".to_string());
    }
    for key in [
        "raw_notes_reviewed",
        "smoke_output_reviewed",
        "participant_identity_reviewed",
        "no_ai_assistance_confirmed",
    ] {
        benchmark_report_apply_required_true_bool(review, key, missing, failed);
    }
    if review
        .get("notes")
        .and_then(serde_json::Value::as_str)
        .is_none_or(|value| value.trim().is_empty())
    {
        missing.push("human_evidence_review.notes".to_string());
    }
}

fn benchmark_review_has_participant_completion_after_reviewed_at(
    data: &serde_json::Map<String, serde_json::Value>,
    reviewed_at: &str,
) -> bool {
    let Some(participant_runs) = data
        .get("participant_runs")
        .and_then(serde_json::Value::as_array)
    else {
        return false;
    };
    for run in participant_runs {
        let Some(completed_at) = run
            .as_object()
            .and_then(|run| run.get("completed_at"))
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
        else {
            continue;
        };
        if benchmark_participant_timestamp_is_valid(completed_at) && completed_at > reviewed_at {
            return true;
        }
    }
    false
}

pub(crate) fn benchmark_report_apply_required_true_bool(
    review: &serde_json::Map<String, serde_json::Value>,
    key: &str,
    missing: &mut Vec<String>,
    failed: &mut Vec<String>,
) {
    match review.get(key).and_then(serde_json::Value::as_bool) {
        Some(true) => {}
        Some(false) => failed.push(format!("human_evidence_review.{key}")),
        None => missing.push(format!("human_evidence_review.{key}")),
    }
}

pub(crate) fn verify_deploy_benchmark_human_evidence_review(
    data: &serde_json::Map<String, serde_json::Value>,
    require_recorded_review: bool,
) -> anyhow::Result<()> {
    let review_value = data.get("human_evidence_review").ok_or_else(|| {
        anyhow::anyhow!("deploy benchmark evidence data human_evidence_review must be an object")
    })?;
    let review = review_value.as_object().ok_or_else(|| {
        anyhow::anyhow!("deploy benchmark evidence data human_evidence_review must be an object")
    })?;
    verify_json_object_keys_exact(
        review_value,
        &[
            "reviewer",
            "reviewed_at",
            "raw_notes_reviewed",
            "smoke_output_reviewed",
            "participant_identity_reviewed",
            "no_ai_assistance_confirmed",
            "notes",
        ],
        "deploy benchmark evidence data human_evidence_review",
    )?;
    for key in ["reviewer", "notes"] {
        let Some(value) = review.get(key).and_then(serde_json::Value::as_str) else {
            anyhow::bail!(
                "deploy benchmark evidence data human_evidence_review {key} must be a string"
            );
        };
        if require_recorded_review && value.trim().is_empty() {
            anyhow::bail!(
                "deploy benchmark evidence data human_evidence_review {key} must be a non-empty string"
            );
        }
    }
    let reviewed_at = match review.get("reviewed_at") {
        Some(value) if value.is_null() => None,
        Some(value) => {
            let Some(timestamp) = value.as_str() else {
                anyhow::bail!(
                    "deploy benchmark evidence data human_evidence_review reviewed_at must be null or an RFC3339 UTC timestamp"
                );
            };
            if !benchmark_participant_timestamp_is_valid(timestamp) {
                anyhow::bail!(
                    "deploy benchmark evidence data human_evidence_review reviewed_at must be null or an RFC3339 UTC timestamp"
                );
            }
            Some(timestamp)
        }
        None => {
            anyhow::bail!(
                "deploy benchmark evidence data human_evidence_review reviewed_at must be null or an RFC3339 UTC timestamp"
            );
        }
    };
    if reviewed_at.is_some_and(|reviewed_at| {
        benchmark_review_has_participant_completion_after_reviewed_at(data, reviewed_at)
    }) {
        anyhow::bail!(
            "deploy benchmark evidence data human_evidence_review reviewed_at must be >= participant_runs[].completed_at"
        );
    }
    for key in [
        "raw_notes_reviewed",
        "smoke_output_reviewed",
        "participant_identity_reviewed",
        "no_ai_assistance_confirmed",
    ] {
        let Some(value) = review.get(key) else {
            anyhow::bail!(
                "deploy benchmark evidence data human_evidence_review {key} must be null or a bool"
            );
        };
        if require_recorded_review {
            if value.as_bool() != Some(true) {
                anyhow::bail!(
                    "deploy benchmark evidence data human_evidence_review {key} must be true for recorded evidence"
                );
            }
        } else if !json_null_or_bool(value) {
            anyhow::bail!(
                "deploy benchmark evidence data human_evidence_review {key} must be null or a bool"
            );
        }
    }
    Ok(())
}

pub(crate) fn verify_deploy_benchmark_failed_participant_classification(
    data: &serde_json::Map<String, serde_json::Value>,
    primary_category: Option<&str>,
) -> anyhow::Result<()> {
    if primary_category.is_some() || !benchmark_data_has_failed_participant_run(data) {
        return Ok(());
    }
    anyhow::bail!(
        "deploy benchmark evidence data failure_classification primary is required when participant_runs contain failed runs"
    );
}

fn benchmark_data_has_failed_participant_run(
    data: &serde_json::Map<String, serde_json::Value>,
) -> bool {
    data.get("participant_runs")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(serde_json::Value::as_object)
        .filter_map(|run| run.get("status").and_then(serde_json::Value::as_str))
        .any(|status| {
            !benchmark_report_status_is_missing(status) && benchmark_report_status_is_failed(status)
        })
}
