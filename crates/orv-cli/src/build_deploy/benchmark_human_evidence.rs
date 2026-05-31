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
            let expected_sha256 = run
                .get("raw_notes_sha256")
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty());
            let expected_participant_id = run
                .get("participant_id")
                .and_then(serde_json::Value::as_str);
            let expected_run_id = run.get("run_id").and_then(serde_json::Value::as_str);
            let (
                retained,
                non_empty,
                template_filled,
                identity_match,
                size_bytes,
                actual_sha256,
                sha256_match,
            ) = match (checked, build_dir, raw_notes_artifact) {
                (true, Some(build_dir), Some(raw_notes_artifact)) => {
                    benchmark_raw_notes_artifact_status(
                        build_dir,
                        raw_notes_artifact,
                        expected_sha256,
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
                "expected_sha256": expected_sha256
                    .map(serde_json::Value::from)
                    .unwrap_or(serde_json::Value::Null),
                "actual_sha256": actual_sha256,
                "sha256_match": sha256_match,
            }))
        })
        .collect()
}

pub(crate) fn benchmark_raw_notes_artifact_status(
    build_dir: &Path,
    artifact: &str,
    expected_sha256: Option<&str>,
    expected_participant_id: Option<&str>,
    expected_run_id: Option<&str>,
) -> (
    serde_json::Value,
    serde_json::Value,
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
    let Ok(bytes) = std::fs::read(&path) else {
        return raw_notes_missing_file_status();
    };
    let actual_sha256 = format!("sha256:{}", sha256_hex(&bytes));
    let content = String::from_utf8(bytes).unwrap_or_default();
    let template_filled = benchmark_raw_notes_artifact_template_filled(&content);
    let identity_match =
        benchmark_raw_notes_identity_matches(&content, expected_participant_id, expected_run_id);
    let sha256_match = expected_sha256.map(|expected| expected == actual_sha256);
    (
        serde_json::Value::Bool(true),
        serde_json::Value::Bool(size > 0),
        serde_json::Value::Bool(template_filled),
        serde_json::Value::Bool(identity_match),
        serde_json::Value::from(size),
        serde_json::Value::String(actual_sha256),
        sha256_match.map_or(serde_json::Value::Null, serde_json::Value::Bool),
    )
}

fn raw_notes_missing_file_status() -> (
    serde_json::Value,
    serde_json::Value,
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
        serde_json::Value::Null,
        serde_json::Value::Null,
    )
}

pub(crate) fn benchmark_raw_notes_artifact_template_filled(content: &str) -> bool {
    if content.trim().is_empty() {
        return false;
    }
    let has_placeholder_line = content.lines().map(str::trim).any(|line| {
        matches!(
            line,
            "- started_at: YYYY-MM-DDTHH:MM:SSZ"
                | "- completed_at: YYYY-MM-DDTHH:MM:SSZ"
                | "- failure_classification.primary:"
                | "- failure_classification.notes:"
        )
    });
    !has_placeholder_line && !benchmark_raw_notes_has_generated_instruction_residue(content)
}

fn benchmark_raw_notes_has_generated_instruction_residue(content: &str) -> bool {
    const GENERATED_INSTRUCTION_FRAGMENTS: &[&str] = &[
        "Copy this file for each participant",
        "deploy/evidence/participant-1.md",
        "Then set each `data.participant_runs[].raw_notes_artifact` entry",
        "Record timestamps, blockers, docs/help lookups",
    ];
    GENERATED_INSTRUCTION_FRAGMENTS
        .iter()
        .any(|fragment| content.contains(fragment))
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
    benchmark_raw_notes_field_matches_once(content, "participant_id", expected_participant_id)
        && benchmark_raw_notes_field_matches_once(content, "run_id", expected_run_id)
}

fn benchmark_raw_notes_field_matches_once(content: &str, key: &str, expected: &str) -> bool {
    let mut values = content
        .lines()
        .filter_map(|line| benchmark_raw_notes_line_field(line, key));
    let Some(value) = values.next() else {
        return false;
    };
    value == expected && values.next().is_none()
}

fn benchmark_raw_notes_line_field<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    let line = line.trim().strip_prefix("- ")?;
    let (name, value) = line.split_once(':')?;
    (name.trim() == key).then_some(value.trim())
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

pub(crate) fn benchmark_raw_notes_sha256_is_valid(value: &str) -> bool {
    value.strip_prefix("sha256:").is_some_and(|hex| {
        hex.len() == 64
            && hex
                .as_bytes()
                .iter()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(byte))
    })
}

#[cfg(test)]
mod tests;
