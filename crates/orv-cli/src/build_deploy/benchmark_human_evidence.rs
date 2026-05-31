use super::*;

pub(crate) fn benchmark_participant_raw_notes_artifacts(
    participant_summary: &serde_json::Value,
    build_dir: Option<&Path>,
) -> Vec<serde_json::Value> {
    participant_summary
        .get("runs")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|run| {
            let index = run.get("index").and_then(serde_json::Value::as_u64)?;
            let raw_notes_artifact = run
                .get("raw_notes_artifact")
                .and_then(serde_json::Value::as_str);
            let path_safe = raw_notes_artifact.map(benchmark_raw_notes_artifact_path_is_safe);
            let recorded = run
                .get("recorded")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false);
            let checked = recorded && build_dir.is_some() && raw_notes_artifact.is_some();
            let expected_participant_id = run
                .get("participant_id")
                .and_then(serde_json::Value::as_str);
            let expected_run_id = run.get("run_id").and_then(serde_json::Value::as_str);
            let (retained, non_empty, template_filled, identity_match, size_bytes) =
                match (checked, build_dir, raw_notes_artifact) {
                    (true, Some(build_dir), Some(raw_notes_artifact)) => {
                        benchmark_raw_notes_artifact_status(
                            build_dir,
                            raw_notes_artifact,
                            expected_participant_id,
                            expected_run_id,
                        )
                    }
                    _ => (
                        serde_json::Value::Null,
                        serde_json::Value::Null,
                        serde_json::Value::Null,
                        serde_json::Value::Null,
                        serde_json::Value::Null,
                    ),
                };
            Some(serde_json::json!({
                "index": index,
                "run_id": run
                    .get("run_id")
                    .cloned()
                    .unwrap_or(serde_json::Value::Null),
                "participant_id": run
                    .get("participant_id")
                    .cloned()
                    .unwrap_or(serde_json::Value::Null),
                "recorded": recorded,
                "path": raw_notes_artifact
                    .map(serde_json::Value::from)
                    .unwrap_or(serde_json::Value::Null),
                "path_safe": path_safe
                    .map(serde_json::Value::from)
                    .unwrap_or(serde_json::Value::Null),
                "checked": checked,
                "retained": retained,
                "non_empty": non_empty,
                "template_filled": template_filled,
                "identity_match": identity_match,
                "size_bytes": size_bytes,
            }))
        })
        .collect()
}

pub(crate) fn benchmark_raw_notes_artifact_status(
    build_dir: &Path,
    artifact: &str,
    expected_participant_id: Option<&str>,
    expected_run_id: Option<&str>,
) -> (
    serde_json::Value,
    serde_json::Value,
    serde_json::Value,
    serde_json::Value,
    serde_json::Value,
) {
    if !benchmark_raw_notes_artifact_path_is_safe(artifact) {
        return raw_notes_missing_file_status();
    }
    let path = build_dir.join(Path::new(artifact.trim()));
    let Ok(build_dir) = std::fs::canonicalize(build_dir) else {
        return raw_notes_missing_file_status();
    };
    let Ok(path) = std::fs::canonicalize(path) else {
        return raw_notes_missing_file_status();
    };
    if !path.starts_with(&build_dir) {
        return raw_notes_missing_file_status();
    }
    let Ok(metadata) = std::fs::metadata(&path) else {
        return raw_notes_missing_file_status();
    };
    if !metadata.is_file() {
        return raw_notes_missing_file_status();
    }
    let size = metadata.len();
    let content = std::fs::read_to_string(&path).unwrap_or_default();
    let template_filled = benchmark_raw_notes_artifact_template_filled(&content);
    let identity_match =
        benchmark_raw_notes_identity_matches(&content, expected_participant_id, expected_run_id);
    (
        serde_json::Value::Bool(true),
        serde_json::Value::Bool(size > 0),
        serde_json::Value::Bool(template_filled),
        serde_json::Value::Bool(identity_match),
        serde_json::Value::from(size),
    )
}

fn raw_notes_missing_file_status() -> (
    serde_json::Value,
    serde_json::Value,
    serde_json::Value,
    serde_json::Value,
    serde_json::Value,
) {
    (
        serde_json::Value::Bool(false),
        serde_json::Value::Null,
        serde_json::Value::Null,
        serde_json::Value::Null,
        serde_json::Value::Null,
    )
}

pub(crate) fn benchmark_raw_notes_artifact_template_filled(content: &str) -> bool {
    if content.trim().is_empty() {
        return false;
    }
    !content.lines().map(str::trim).any(|line| {
        matches!(
            line,
            "- started_at: YYYY-MM-DDTHH:MM:SSZ"
                | "- completed_at: YYYY-MM-DDTHH:MM:SSZ"
                | "- failure_classification.primary:"
                | "- failure_classification.notes:"
        )
    })
}

pub(crate) fn benchmark_raw_notes_identity_matches(
    content: &str,
    expected_participant_id: Option<&str>,
    expected_run_id: Option<&str>,
) -> bool {
    let Some(expected_participant_id) = expected_participant_id.map(str::trim) else {
        return false;
    };
    let Some(expected_run_id) = expected_run_id.map(str::trim) else {
        return false;
    };
    if expected_participant_id.is_empty() || expected_run_id.is_empty() {
        return false;
    }
    benchmark_raw_notes_field(content, "participant_id") == Some(expected_participant_id)
        && benchmark_raw_notes_field(content, "run_id") == Some(expected_run_id)
}

fn benchmark_raw_notes_field<'a>(content: &'a str, key: &str) -> Option<&'a str> {
    content.lines().find_map(|line| {
        let line = line.trim().strip_prefix("- ")?;
        let (name, value) = line.split_once(':')?;
        (name.trim() == key).then_some(value.trim())
    })
}

pub(crate) fn benchmark_raw_notes_artifact_path_is_safe(artifact: &str) -> bool {
    let artifact = artifact.trim();
    if artifact.is_empty() {
        return false;
    }
    if artifact.contains('\\') || artifact.as_bytes().get(1) == Some(&b':') {
        return false;
    }
    let artifact_path = Path::new(artifact);
    !artifact_path.is_absolute()
        && !artifact_path
            .components()
            .any(|component| matches!(component, std::path::Component::ParentDir))
}
