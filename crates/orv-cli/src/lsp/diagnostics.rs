#![allow(clippy::redundant_pub_crate, clippy::wildcard_imports)]

use super::*;

pub(crate) fn lsp_project_diagnostics(
    loaded: &orv_project::LoadedProject,
) -> Vec<orv_diagnostics::Diagnostic> {
    let resolved = orv_resolve::resolve(&loaded.program);
    let lowered = orv_analyzer::lower_with_diagnostics(&loaded.program, &resolved);
    let mut diagnostics = Vec::new();
    diagnostics.extend(loaded.diagnostics.clone());
    diagnostics.extend(resolved.diagnostics);
    diagnostics.extend(lowered.diagnostics);
    diagnostics
}

pub(crate) fn lsp_workspace_diagnostic_items_json(
    loaded: &orv_project::LoadedProject,
) -> Vec<serde_json::Value> {
    let resolved = orv_resolve::resolve(&loaded.program);
    let lowered = orv_analyzer::lower_with_diagnostics(&loaded.program, &resolved);
    loaded
        .files
        .iter()
        .filter_map(|file| {
            let mut diagnostics = Vec::new();
            diagnostics.extend(lsp_diagnostics_json_for_file(
                &loaded.diagnostics,
                &loaded.files,
                file.id,
            ));
            diagnostics.extend(lsp_diagnostics_json_for_file(
                &resolved.diagnostics,
                &loaded.files,
                file.id,
            ));
            diagnostics.extend(lsp_diagnostics_json_for_file(
                &lowered.diagnostics,
                &loaded.files,
                file.id,
            ));
            if diagnostics.is_empty() {
                return None;
            }
            Some(serde_json::json!({
                "uri": lsp_file_uri_for_path(&file.path),
                "version": serde_json::Value::Null,
                "kind": "full",
                "items": diagnostics,
            }))
        })
        .collect()
}

pub(crate) fn lsp_jsonrpc_result(
    id: &serde_json::Value,
    result: &serde_json::Value,
) -> serde_json::Value {
    serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": result,
    })
}

pub(crate) fn lsp_jsonrpc_result_or_invalid_params(
    id: &serde_json::Value,
    result: anyhow::Result<serde_json::Value>,
) -> serde_json::Value {
    match result {
        Ok(result) => lsp_jsonrpc_result(id, &result),
        Err(err) => lsp_jsonrpc_error(id, -32602, &err.to_string()),
    }
}

pub(crate) fn lsp_jsonrpc_method_not_found(
    id: &serde_json::Value,
    method: &str,
) -> serde_json::Value {
    serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {
            "code": -32601,
            "message": "method not found",
            "data": {
                "method": method,
            },
        },
    })
}

pub(crate) fn lsp_jsonrpc_error(
    id: &serde_json::Value,
    code: i32,
    message: &str,
) -> serde_json::Value {
    serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {
            "code": code,
            "message": message,
        },
    })
}

pub(crate) fn lsp_diagnostics_json(
    diagnostics: &[orv_diagnostics::Diagnostic],
    files: &[SourceFile],
) -> Vec<serde_json::Value> {
    diagnostics
        .iter()
        .map(|diagnostic| lsp_diagnostic_json(diagnostic, files))
        .collect()
}

pub(crate) fn lsp_diagnostics_json_for_file(
    diagnostics: &[orv_diagnostics::Diagnostic],
    files: &[SourceFile],
    file_id: FileId,
) -> Vec<serde_json::Value> {
    diagnostics
        .iter()
        .filter(|diagnostic| lsp_diagnostic_file_id(diagnostic) == Some(file_id))
        .map(|diagnostic| lsp_diagnostic_json(diagnostic, files))
        .collect()
}

pub(crate) fn lsp_diagnostic_json(
    diagnostic: &orv_diagnostics::Diagnostic,
    files: &[SourceFile],
) -> serde_json::Value {
    let span = lsp_diagnostic_span(diagnostic);
    serde_json::json!({
        "source": "orv",
        "severity": lsp_severity(diagnostic.severity),
        "code": diagnostic.code,
        "message": diagnostic.message,
        "range": lsp_range_json(span, files),
    })
}

pub(crate) fn lsp_diagnostic_span(diagnostic: &orv_diagnostics::Diagnostic) -> Span {
    diagnostic
        .primary
        .as_ref()
        .map(|label| label.span)
        .or_else(|| diagnostic.secondary.first().map(|label| label.span))
        .unwrap_or(Span::DUMMY)
}

pub(crate) fn lsp_diagnostic_file_id(diagnostic: &orv_diagnostics::Diagnostic) -> Option<FileId> {
    diagnostic
        .primary
        .as_ref()
        .map(|label| label.span.file)
        .or_else(|| diagnostic.secondary.first().map(|label| label.span.file))
}

pub(crate) fn lsp_code_lenses_json(
    graph: &ProjectGraph,
    files: &[SourceFile],
    file_id: FileId,
) -> Vec<serde_json::Value> {
    graph
        .nodes
        .iter()
        .filter(|node| node.file == file_id)
        .filter_map(|node| {
            let kind = lsp_symbol_kind(node.kind)?;
            Some(serde_json::json!({
                "range": lsp_range_json(node.span, files),
                "command": {
                    "title": format!("Reveal {kind} {}", node.name),
                    "command": "orv.revealSourceNode",
                    "arguments": [node.id, node.name],
                },
                "data": {
                    "source_node": node.id,
                },
            }))
        })
        .collect()
}

pub(crate) fn lsp_code_actions_json(
    loaded: &orv_project::LoadedProject,
    file: &SourceFile,
    requested_start: usize,
    requested_end: usize,
) -> Vec<serde_json::Value> {
    let uri = lsp_file_uri_for_path(&file.path);
    let start = u32::try_from(requested_start.min(requested_end)).unwrap_or(u32::MAX);
    let end = u32::try_from(requested_start.max(requested_end)).unwrap_or(u32::MAX);
    lsp_project_diagnostics(loaded)
        .iter()
        .filter(|diagnostic| lsp_diagnostic_file_id(diagnostic) == Some(file.id))
        .filter(|diagnostic| lsp_span_overlaps_range(lsp_diagnostic_span(diagnostic), start, end))
        .flat_map(|diagnostic| {
            let diagnostic_json = lsp_diagnostic_json(diagnostic, &loaded.files);
            let range = diagnostic_json
                .get("range")
                .cloned()
                .unwrap_or_else(|| lsp_range_for_source(&file.source, start, end));
            let mut actions =
                lsp_diagnostic_edit_code_actions_json(diagnostic, &diagnostic_json, &uri, &range);
            actions.push(serde_json::json!({
                "title": format!("Reveal diagnostic: {}", diagnostic.message),
                "kind": "quickfix",
                "diagnostics": [diagnostic_json],
                "command": {
                    "title": "Reveal diagnostic",
                    "command": "orv.revealDiagnostic",
                    "arguments": [
                        uri,
                        range,
                        diagnostic.code.clone().unwrap_or_default(),
                        diagnostic.message,
                    ],
                },
            }));
            actions
        })
        .collect()
}

pub(crate) fn lsp_diagnostic_edit_code_actions_json(
    diagnostic: &orv_diagnostics::Diagnostic,
    diagnostic_json: &serde_json::Value,
    uri: &str,
    range: &serde_json::Value,
) -> Vec<serde_json::Value> {
    match (diagnostic.code.as_deref(), diagnostic.message.as_str()) {
        (Some("syntax/route-method"), _) | (None, "expected HTTP method after `@route`") => {
            lsp_insert_text_code_action_json(
                "Insert default GET route head",
                uri,
                range,
                "GET /path ",
                diagnostic_json,
            )
            .into_iter()
            .collect()
        }
        (Some("syntax/route-path"), _)
        | (None, "expected path starting with `/` or `*` after HTTP method") => {
            lsp_insert_text_code_action_json(
                "Insert default route path",
                uri,
                range,
                "/path ",
                diagnostic_json,
            )
            .into_iter()
            .collect()
        }
        _ => Vec::new(),
    }
}

pub(crate) fn lsp_insert_text_code_action_json(
    title: &str,
    uri: &str,
    range: &serde_json::Value,
    new_text: &str,
    diagnostic_json: &serde_json::Value,
) -> Option<serde_json::Value> {
    let start = range.get("start")?.clone();
    let edit_range = serde_json::json!({
        "start": start,
        "end": start,
    });
    let mut changes = serde_json::Map::new();
    changes.insert(
        uri.to_string(),
        serde_json::json!([{
            "range": edit_range,
            "newText": new_text,
        }]),
    );
    Some(serde_json::json!({
        "title": title,
        "kind": "quickfix",
        "diagnostics": [diagnostic_json.clone()],
        "edit": {
            "changes": changes,
        },
    }))
}

pub(crate) fn lsp_execute_reveal_diagnostic_json(request: &serde_json::Value) -> serde_json::Value {
    let uri = request
        .pointer("/params/arguments/0")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let range = request
        .pointer("/params/arguments/1")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let code = request
        .pointer("/params/arguments/2")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let message = request
        .pointer("/params/arguments/3")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    serde_json::json!({
        "command": "orv.revealDiagnostic",
        "uri": uri,
        "range": range,
        "code": code,
        "message": message,
    })
}

pub(crate) const fn lsp_severity(severity: orv_diagnostics::Severity) -> u8 {
    match severity {
        orv_diagnostics::Severity::Error => 1,
        orv_diagnostics::Severity::Warning => 2,
        orv_diagnostics::Severity::Note => 3,
        orv_diagnostics::Severity::Help => 4,
    }
}
