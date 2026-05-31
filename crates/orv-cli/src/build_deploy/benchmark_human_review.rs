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
