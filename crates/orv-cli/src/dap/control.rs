#![allow(clippy::redundant_pub_crate, clippy::wildcard_imports)]

use super::*;

pub(crate) fn editor_dap_sources(files: &[SourceFile]) -> Vec<DapSourceInfo> {
    files
        .iter()
        .enumerate()
        .map(|(index, file)| dap_source_info(file, u64::try_from(index + 1).unwrap_or(u64::MAX)))
        .collect()
}

pub(crate) fn verify_editor_debug_dap_source_contract_keys(
    source: &serde_json::Value,
    context: &str,
    allow_line: bool,
) -> anyhow::Result<()> {
    if allow_line {
        verify_json_object_keys_exact(
            source,
            &[
                "name",
                "path",
                "sourceReference",
                "uri",
                "checksums",
                "line",
            ],
            context,
        )?;
    } else {
        verify_json_object_keys_exact(
            source,
            &["name", "path", "sourceReference", "uri", "checksums"],
            context,
        )?;
    }
    let checksums = source
        .get("checksums")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| anyhow::anyhow!("{context}.checksums must be an array"))?;
    for (index, checksum) in checksums.iter().enumerate() {
        verify_json_object_keys_exact(
            checksum,
            &["algorithm", "checksum"],
            &format!("{context}.checksums[{index}]"),
        )?;
    }
    Ok(())
}

pub(crate) fn verify_editor_debug_dap_request_contract_keys(
    request: &serde_json::Value,
    context: &str,
) -> anyhow::Result<()> {
    verify_json_object_keys_exact(request, &["seq", "type", "command", "arguments"], context)
}

pub(crate) fn dap_hir_call_name(expr: &orv_hir::HirExpr) -> String {
    match &expr.kind {
        orv_hir::HirExprKind::Ident(ident) => ident.name.clone(),
        orv_hir::HirExprKind::Field { target, field, .. } => {
            format!("{}.{}", dap_hir_call_name(target), field)
        }
        orv_hir::HirExprKind::OptionalField { target, field, .. } => {
            format!("{}?.{}", dap_hir_call_name(target), field)
        }
        orv_hir::HirExprKind::Domain { name, .. } => format!("@{name}"),
        orv_hir::HirExprKind::TypeName(name) => name.clone(),
        _ => "<expr>".to_string(),
    }
}

pub(crate) fn dap_string_port(expr: &orv_hir::HirExpr) -> Option<u64> {
    let orv_hir::HirExprKind::String(segments) = &expr.kind else {
        return None;
    };
    let [orv_hir::HirStringSegment::Str(raw)] = segments.as_slice() else {
        return None;
    };
    raw.parse::<u64>().ok()
}

pub(crate) fn dap_long_running_frame(
    span: Span,
    files: &[SourceFile],
    sources: &[DapSourceInfo],
) -> Option<DapFrameState> {
    let source = dap_source_for_span(span, files, sources)?;
    let line = dap_span_line(span, files)?;
    Some(DapFrameState {
        source: source.clone(),
        line,
        locals: Vec::new(),
        stack: vec![DapStackFrameState {
            name: "server runtime".to_string(),
            source,
            line,
        }],
        output: String::new(),
    })
}

pub(crate) fn dap_server_request_frames_display(
    frames: &[orv_runtime::server::ServerRequestFrame],
) -> String {
    frames
        .iter()
        .enumerate()
        .map(|(index, frame)| {
            format!(
                "#{} {}",
                index.saturating_add(1),
                dap_server_request_frame_display(frame)
            )
        })
        .collect::<Vec<_>>()
        .join(", ")
}

pub(crate) fn dap_server_request_frame_display(
    frame: &orv_runtime::server::ServerRequestFrame,
) -> String {
    let mut parts = vec![format!(
        "{} {} -> {}",
        frame.method, frame.path, frame.status
    )];
    if let (Some(method), Some(path)) = (&frame.route_method, &frame.route_path) {
        parts.push(format!("route {method} {path}"));
    }
    if let Some(origin_id) = &frame.response_origin_id {
        parts.push(format!("response {origin_id}"));
    }
    if !frame.params.is_empty() {
        parts.push(format!("params {}", dap_string_map_display(&frame.params)));
    }
    if !frame.query.is_empty() {
        parts.push(format!("query {}", dap_string_map_display(&frame.query)));
    }
    if !frame.body.is_empty() {
        parts.push(format!("body {}", frame.body));
    }
    parts.join(" ")
}

pub(crate) fn dap_server_request_trace_display(
    frames: &[orv_runtime::server::ServerRequestFrame],
) -> String {
    serde_json::to_string(&orv_runtime::server::request_trace_json(frames)).unwrap_or_else(|_| {
        "{\"schema_version\":1,\"kind\":\"orv.production.trace\",\"frame_count\":0,\"frames\":[]}"
            .to_string()
    })
}

pub(crate) fn dap_string_map_display(values: &HashMap<String, String>) -> String {
    let mut entries = values
        .iter()
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>();
    entries.sort();
    entries.join(",")
}

pub(crate) fn dap_source_for_span(
    span: Span,
    files: &[SourceFile],
    sources: &[DapSourceInfo],
) -> Option<DapSourceInfo> {
    let file = files.iter().find(|file| file.id == span.file)?;
    sources
        .iter()
        .find(|source| dap_normalize_path(&file.path) == dap_normalize_path(&source.path))
        .cloned()
}

pub(crate) fn dap_current_source_and_line(launched: &DapLaunchState) -> (DapSourceInfo, u64) {
    if let Some(frame) = launched.frames.get(launched.current_frame_index) {
        return (frame.source.clone(), frame.line);
    }
    let source = dap_entry_source(launched);
    (source, launched.stopped_line)
}

pub(crate) fn dap_entry_source(launched: &DapLaunchState) -> DapSourceInfo {
    launched
        .sources
        .iter()
        .find(|source| dap_normalize_path(&source.path) == dap_normalize_path(&launched.path))
        .cloned()
        .unwrap_or_else(|| DapSourceInfo {
            reference: 0,
            name: launched.name.clone(),
            path: launched.path.clone(),
            uri: launched.uri.clone(),
            checksum: String::new(),
        })
}

pub(crate) fn dap_stack_call_for_frame_id(
    launched: &DapLaunchState,
    frame_id: u64,
) -> Option<DapStackFrameState> {
    if frame_id <= 1 {
        return None;
    }
    let current_frame = launched.frames.get(launched.current_frame_index)?;
    let stack_index = usize::try_from(frame_id.saturating_sub(2)).ok()?;
    current_frame
        .stack
        .iter()
        .rev()
        .skip(1)
        .nth(stack_index)
        .cloned()
}

pub(crate) fn dap_stack_entry_frame_id(launched: &DapLaunchState) -> Option<u64> {
    let current_frame = launched.frames.get(launched.current_frame_index)?;
    (!current_frame.stack.is_empty())
        .then(|| u64::try_from(current_frame.stack.len().saturating_add(1)).ok())
        .flatten()
}

pub(crate) fn dap_str_argument<'a>(request: &'a serde_json::Value, name: &str) -> Option<&'a str> {
    request
        .get("arguments")
        .and_then(|arguments| arguments.get(name))
        .and_then(serde_json::Value::as_str)
}

pub(crate) fn dap_usize_argument(request: &serde_json::Value, name: &str) -> Option<usize> {
    request
        .get("arguments")
        .and_then(|arguments| arguments.get(name))
        .and_then(serde_json::Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())
}

pub(crate) fn dap_disassemble_start_index(
    memory_reference: &str,
    instruction_offset: i64,
) -> anyhow::Result<usize> {
    let base = dap_memory_reference_frame_index(memory_reference, "disassemble")?;
    if instruction_offset < 0 {
        Ok(base.saturating_sub(
            usize::try_from(instruction_offset.saturating_abs()).unwrap_or(usize::MAX),
        ))
    } else {
        Ok(base.saturating_add(usize::try_from(instruction_offset).unwrap_or(usize::MAX)))
    }
}

pub(crate) fn dap_memory_reference_frame_index(
    memory_reference: &str,
    command: &str,
) -> anyhow::Result<usize> {
    let frame = memory_reference
        .strip_prefix("orv:frame:")
        .ok_or_else(|| {
            anyhow::anyhow!("unsupported ORV {command} memoryReference `{memory_reference}`")
        })?
        .parse::<usize>()
        .map_err(|_| {
            anyhow::anyhow!("invalid ORV {command} memoryReference `{memory_reference}`")
        })?;
    if frame == 0 {
        anyhow::bail!("invalid ORV {command} memoryReference `{memory_reference}`");
    }
    Ok(frame - 1)
}

pub(crate) fn dap_base64_encode(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut encoded = String::with_capacity(bytes.len().div_ceil(3).saturating_mul(4));
    for chunk in bytes.chunks(3) {
        let first = chunk[0];
        let second = chunk.get(1).copied().unwrap_or(0);
        let third = chunk.get(2).copied().unwrap_or(0);
        encoded.push(char::from(TABLE[usize::from(first >> 2)]));
        encoded.push(char::from(
            TABLE[usize::from(((first & 0b0000_0011) << 4) | (second >> 4))],
        ));
        if chunk.len() > 1 {
            encoded.push(char::from(
                TABLE[usize::from(((second & 0b0000_1111) << 2) | (third >> 6))],
            ));
        } else {
            encoded.push('=');
        }
        if chunk.len() > 2 {
            encoded.push(char::from(TABLE[usize::from(third & 0b0011_1111)]));
        } else {
            encoded.push('=');
        }
    }
    encoded
}

pub(crate) fn dap_disassembled_instruction_json(
    index: usize,
    frame: &DapFrameState,
) -> serde_json::Value {
    let name = frame
        .stack
        .last()
        .map_or("orv entry", |stack| stack.name.as_str());
    serde_json::json!({
        "address": format!("orv:frame:{}", index.saturating_add(1)),
        "instruction": format!("{name} line {}", frame.line),
        "location": dap_source_json_with_reference(&frame.source, 0),
        "line": frame.line,
        "column": 1,
    })
}

pub(crate) fn dap_step_in_target_id(frame_index: usize) -> u64 {
    u64::try_from(frame_index.saturating_add(1)).unwrap_or(u64::MAX)
}

pub(crate) fn dap_step_in_target_indices(launched: &DapLaunchState) -> Vec<usize> {
    let Some(current_frame) = launched.frames.get(launched.current_frame_index) else {
        return Vec::new();
    };
    let current_depth = current_frame.stack.len();
    let mut seen = Vec::<(String, u64, u64)>::new();
    let mut targets = Vec::new();
    for (index, frame) in launched
        .frames
        .iter()
        .enumerate()
        .skip(launched.current_frame_index.saturating_add(1))
    {
        let depth = frame.stack.len();
        if depth <= current_depth {
            break;
        }
        if depth != current_depth.saturating_add(1) {
            continue;
        }
        let Some(call_frame) = frame.stack.last() else {
            continue;
        };
        let key = (
            call_frame.name.clone(),
            call_frame.source.reference,
            call_frame.line,
        );
        if seen.contains(&key) {
            continue;
        }
        seen.push(key);
        targets.push(index);
    }
    targets
}

pub(crate) fn dap_step_in_targets_json(launched: &DapLaunchState) -> Vec<serde_json::Value> {
    dap_step_in_target_indices(launched)
        .into_iter()
        .filter_map(|index| {
            let frame = launched.frames.get(index)?;
            let call_frame = frame.stack.last()?;
            Some(serde_json::json!({
                "id": dap_step_in_target_id(index),
                "label": call_frame.name,
                "line": call_frame.line,
                "column": 1,
                "source": dap_source_json_with_reference(&call_frame.source, 0),
            }))
        })
        .collect()
}

pub(crate) fn dap_restart_frame_target_index(
    launched: &DapLaunchState,
    frame_id: u64,
) -> Option<usize> {
    if frame_id != 1 {
        return dap_non_current_restart_frame_target_index(launched, frame_id);
    }
    let current_index = launched.current_frame_index;
    let current_frame = launched.frames.get(current_index)?;
    let Some(current_call) = current_frame.stack.last() else {
        return Some(0);
    };
    let current_depth = current_frame.stack.len();
    let mut target = current_index;
    for index in (0..=current_index).rev() {
        let frame = launched.frames.get(index)?;
        if frame.stack.len() < current_depth {
            break;
        }
        let Some(call) = frame.stack.last() else {
            continue;
        };
        if call.name == current_call.name
            && call.source.reference == current_call.source.reference
            && call.line == current_call.line
        {
            target = index;
        }
    }
    Some(target)
}

pub(crate) fn dap_non_current_restart_frame_target_index(
    launched: &DapLaunchState,
    frame_id: u64,
) -> Option<usize> {
    if dap_stack_entry_frame_id(launched) == Some(frame_id) {
        return Some(0);
    }
    let target_call = dap_stack_call_for_frame_id(launched, frame_id)?;
    let current_index = launched.current_frame_index;
    let mut target = None;
    for index in (0..=current_index).rev() {
        let frame = launched.frames.get(index)?;
        let Some(call) = frame.stack.last() else {
            continue;
        };
        if dap_same_stack_call(call, &target_call) {
            target = Some(index);
        }
    }
    target
}

pub(crate) fn dap_same_stack_call(left: &DapStackFrameState, right: &DapStackFrameState) -> bool {
    left.name == right.name
        && left.source.reference == right.source.reference
        && left.line == right.line
}

pub(crate) fn dap_current_locals(launched: &DapLaunchState) -> &[DapVariable] {
    launched
        .frames
        .get(launched.current_frame_index)
        .map_or(&[], |frame| frame.locals.as_slice())
}

pub(crate) fn dap_logpoint_output(message: &str) -> String {
    let mut output = message.to_string();
    if !output.ends_with('\n') {
        output.push('\n');
    }
    output
}

pub(crate) fn dap_compare_condition_numbers(left: &str, op: &str, right: &str) -> Option<bool> {
    let left = left.parse::<f64>().ok()?;
    let right = right.parse::<f64>().ok()?;
    Some(match op {
        ">" => left > right,
        "<" => left < right,
        ">=" => left >= right,
        "<=" => left <= right,
        _ => return None,
    })
}

pub(crate) fn dap_normalize_condition_literal(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.starts_with('"') && trimmed.ends_with('"') && trimmed.len() >= 2 {
        let decoded = serde_json::from_str::<String>(trimmed)
            .unwrap_or_else(|_| trimmed.trim_matches('"').to_string());
        return serde_json::to_string(&decoded).unwrap_or(decoded);
    }
    trimmed.to_string()
}

pub(crate) fn dap_hit_condition_matches(condition: &str, hit_count: usize) -> bool {
    let condition = condition.trim();
    if let Some(modulo) = condition
        .strip_prefix('%')
        .and_then(|value| value.trim_start_matches('=').trim().parse::<usize>().ok())
    {
        return modulo > 0 && hit_count % modulo == 0;
    }
    for op in [">=", "<=", ">", "<", "==", "="] {
        if let Some((_, right)) = condition.split_once(op) {
            let Some(expected) = right.trim().parse::<usize>().ok() else {
                return false;
            };
            return match op {
                ">=" => hit_count >= expected,
                "<=" => hit_count <= expected,
                ">" => hit_count > expected,
                "<" => hit_count < expected,
                "==" | "=" => hit_count == expected,
                _ => false,
            };
        }
    }
    condition
        .parse::<usize>()
        .is_ok_and(|expected| hit_count == expected)
}

pub(crate) fn dap_span_line(span: Span, files: &[SourceFile]) -> Option<u64> {
    let file = files.iter().find(|file| file.id == span.file)?;
    let start = byte_position(&file.source, span.range.start);
    Some(u64::try_from(start.0 + 1).unwrap_or(u64::MAX))
}

pub(crate) fn dap_completion_targets_json(
    launched: &DapLaunchState,
    prefix: &str,
) -> Vec<serde_json::Value> {
    const EXPRESSIONS: &[&str] = &[
        "entry",
        "projectGraphNodes",
        "diagnostics",
        "runtimeStatus",
        "stdout",
        "runtimeError",
    ];
    let mut targets = EXPRESSIONS
        .iter()
        .filter(|expression| expression.starts_with(prefix))
        .map(|expression| {
            serde_json::json!({
                "label": expression,
                "type": "property",
                "sortText": expression,
            })
        })
        .collect::<Vec<_>>();
    if launched.async_runtime.is_some() {
        targets.extend(
            [
                "runtimeKind",
                "runtimeAsyncState",
                "runtimeResumeCount",
                "runtimePauseCount",
                "runtimeRouteCount",
                "runtimeRoutes",
                "runtimeRequestCount",
                "runtimeLastRequest",
                "runtimeRequestFrames",
                "runtimeRequestTrace",
                "runtimeRequestTracePath",
                "runtimeListen",
                "runtimeListenPort",
                "runtimeTransport",
                "runtimeProcessId",
            ]
            .into_iter()
            .filter(|expression| expression.starts_with(prefix))
            .map(|expression| {
                serde_json::json!({
                    "label": expression,
                    "type": "property",
                    "sortText": expression,
                })
            }),
        );
    }
    targets.extend(
        dap_current_locals(launched)
            .iter()
            .filter(|local| local.name.starts_with(prefix))
            .map(|local| {
                serde_json::json!({
                    "label": local.name,
                    "type": "variable",
                    "sortText": local.name,
                })
            }),
    );
    targets.sort_by_key(|target| {
        target
            .get("sortText")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("")
            .to_string()
    });
    targets.dedup_by(|left, right| left["label"] == right["label"]);
    targets
}

pub(crate) fn dap_push_span_line(
    span: Span,
    file_id: FileId,
    files: &[SourceFile],
    lines: &mut Vec<u64>,
) {
    if span.file != file_id {
        return;
    }
    let Some(file) = files.iter().find(|file| file.id == span.file) else {
        return;
    };
    let start = lsp_byte_position(&file.source, span.range.start);
    lines.push(u64::try_from(start.0 + 1).unwrap_or(u64::MAX));
}

pub(crate) fn dap_following_executable_line(lines: &[u64], current: u64) -> Option<u64> {
    lines.iter().copied().find(|line| *line > current)
}

pub(crate) fn dap_source_json(source: &DapSourceInfo) -> serde_json::Value {
    dap_source_json_with_reference(source, source.reference)
}

pub(crate) fn dap_source_json_with_reference(
    source: &DapSourceInfo,
    source_reference: u64,
) -> serde_json::Value {
    serde_json::json!({
        "name": source.name,
        "path": source.path.display().to_string(),
        "sourceReference": source_reference,
        "uri": source.uri,
        "checksums": [
            {
                "algorithm": "SHA256",
                "checksum": source.checksum,
            },
        ],
    })
}

pub(crate) fn dap_module_json(source: &DapSourceInfo) -> serde_json::Value {
    serde_json::json!({
        "id": source.reference,
        "name": source.name,
        "path": source.path.display().to_string(),
        "isUserCode": true,
        "symbolStatus": "loaded",
    })
}

pub(crate) fn dap_goto_target_json(source: &DapSourceInfo, line: u64) -> serde_json::Value {
    serde_json::json!({
        "id": dap_goto_target_id(source.reference, line),
        "label": format!("{}:{line}", source.name),
        "line": line,
        "column": 1,
    })
}

pub(crate) const fn dap_goto_target_id(source_reference: u64, line: u64) -> u64 {
    source_reference
        .saturating_mul(1_000_000)
        .saturating_add(line)
}

pub(crate) fn dap_program_path(request: &serde_json::Value) -> anyhow::Result<PathBuf> {
    let program = request
        .pointer("/arguments/program")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("launch.arguments.program must be a path or file URI"))?;
    dap_path_from_protocol_string(program)
}

pub(crate) fn dap_source_path(request: &serde_json::Value) -> anyhow::Result<PathBuf> {
    let path = request
        .pointer("/arguments/source/path")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("source.path must be a path or file URI"))?;
    dap_path_from_protocol_string(path)
}

pub(crate) fn dap_source_reference(request: &serde_json::Value) -> Option<u64> {
    request
        .pointer("/arguments/sourceReference")
        .and_then(serde_json::Value::as_u64)
        .filter(|reference| *reference > 0)
}

pub(crate) fn dap_path_from_protocol_string(path: &str) -> anyhow::Result<PathBuf> {
    if path.starts_with("file://") {
        lsp_file_uri_path(path)
    } else {
        Ok(PathBuf::from(path))
    }
}

pub(crate) fn dap_normalize_path(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

pub(crate) fn dap_success_response(
    seq: u64,
    request_seq: u64,
    command: &str,
    body: &serde_json::Value,
) -> serde_json::Value {
    serde_json::json!({
        "seq": seq,
        "type": "response",
        "request_seq": request_seq,
        "success": true,
        "command": command,
        "body": body,
    })
}

pub(crate) fn dap_error_response(
    seq: u64,
    request_seq: u64,
    command: &str,
    message: &str,
) -> serde_json::Value {
    serde_json::json!({
        "seq": seq,
        "type": "response",
        "request_seq": request_seq,
        "success": false,
        "command": command,
        "message": message,
    })
}

pub(crate) fn dap_event_response(
    seq: u64,
    event: &str,
    body: &serde_json::Value,
) -> serde_json::Value {
    serde_json::json!({
        "seq": seq,
        "type": "event",
        "event": event,
        "body": body,
    })
}

pub(crate) fn dap_response_for_request_seq(
    frames: &[serde_json::Value],
    request_seq: u64,
) -> Option<serde_json::Value> {
    frames
        .iter()
        .find(|frame| {
            frame.get("type").and_then(serde_json::Value::as_str) == Some("response")
                && frame.get("request_seq").and_then(serde_json::Value::as_u64) == Some(request_seq)
        })
        .cloned()
}
