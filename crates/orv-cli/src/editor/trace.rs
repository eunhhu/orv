#![allow(clippy::redundant_pub_crate, clippy::wildcard_imports)]

use super::*;

pub(crate) fn cmd_editor_trace(dir: &Path, trace: &Path) -> anyhow::Result<()> {
    let value = editor_trace_json(dir, trace)?;
    println!("{}", serde_json::to_string_pretty(&value)?);
    Ok(())
}

pub(crate) fn cmd_editor_trace_stream(dir: &Path, events: &Path) -> anyhow::Result<()> {
    let value = editor_trace_stream_json(dir, events)?;
    println!("{}", serde_json::to_string_pretty(&value)?);
    Ok(())
}

pub(crate) fn editor_trace_json(dir: &Path, trace: &Path) -> anyhow::Result<serde_json::Value> {
    let trace_value = read_json_value(trace)?;
    let trace_path = trace.display().to_string();
    let live_refresh = editor_trace_live_refresh_json(dir, trace)?;
    editor_trace_payload_json(dir, &trace_path, &trace_value, &live_refresh)
}

pub(crate) fn editor_trace_payload_json(
    dir: &Path,
    trace_path: &str,
    trace_value: &serde_json::Value,
    live_refresh: &serde_json::Value,
) -> anyhow::Result<serde_json::Value> {
    verify_editor_runtime_trace_document_contract_keys(trace_value, "trace JSON")?;
    let frames = trace_value
        .get("frames")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| anyhow::anyhow!("trace JSON must contain frames array"))?;
    let mut editor_frames = Vec::with_capacity(frames.len());
    let mut status_counts = EditorTraceStatusCounts::default();
    for (index, frame) in frames.iter().enumerate() {
        let origin_id = editor_trace_frame_origin_id(frame);
        let response_origin_id = editor_trace_frame_response_origin_id(frame);
        let db_operation_origin_id =
            editor_trace_frame_named_origin_id(frame, "db_operation_origin_id");
        let commerce_adapter_origin_id =
            editor_trace_frame_named_origin_id(frame, "commerce_adapter_origin_id");
        let navigation = match origin_id {
            Some(origin_id) => editor_reveal_json(dir, origin_id)?,
            None => serde_json::Value::Null,
        };
        let response_navigation = match response_origin_id {
            Some(origin_id) => editor_reveal_json(dir, origin_id)?,
            None => serde_json::Value::Null,
        };
        let db_navigation = match db_operation_origin_id {
            Some(origin_id) => editor_reveal_json(dir, origin_id)?,
            None => serde_json::Value::Null,
        };
        let commerce_navigation = match commerce_adapter_origin_id {
            Some(origin_id) => editor_reveal_json(dir, origin_id)?,
            None => serde_json::Value::Null,
        };
        let request = editor_trace_request_json(frame);
        let summary = editor_trace_summary_json(
            &request,
            origin_id,
            response_origin_id,
            db_operation_origin_id,
            commerce_adapter_origin_id,
        );
        let frame_index = serde_json::json!(index);
        let build_dir = dir.display().to_string();
        let reveal_command = editor_trace_frame_reveal_command_json(&build_dir, origin_id);
        let response_reveal_command =
            editor_trace_frame_reveal_command_json(&build_dir, response_origin_id);
        let db_reveal_command =
            editor_trace_frame_reveal_command_json(&build_dir, db_operation_origin_id);
        let commerce_reveal_command =
            editor_trace_frame_reveal_command_json(&build_dir, commerce_adapter_origin_id);
        let actions = editor_native_host_trace_frame_actions_json(
            &frame_index,
            &build_dir,
            [
                (
                    "route",
                    "Reveal route source",
                    origin_id,
                    &navigation,
                    &reveal_command,
                ),
                (
                    "response",
                    "Reveal response source",
                    response_origin_id,
                    &response_navigation,
                    &response_reveal_command,
                ),
                (
                    "db",
                    "Reveal DB operation source",
                    db_operation_origin_id,
                    &db_navigation,
                    &db_reveal_command,
                ),
                (
                    "commerce",
                    "Reveal commerce adapter source",
                    commerce_adapter_origin_id,
                    &commerce_navigation,
                    &commerce_reveal_command,
                ),
            ],
        );
        status_counts.record(request.get("status").and_then(serde_json::Value::as_u64));
        editor_frames.push(serde_json::json!({
            "index": frame_index,
            "origin_id": origin_id,
            "response_origin_id": response_origin_id,
            "db_operation_origin_id": db_operation_origin_id,
            "commerce_adapter_origin_id": commerce_adapter_origin_id,
            "request": request,
            "summary": summary,
            "reveal_command": reveal_command,
            "response_reveal_command": response_reveal_command,
            "db_reveal_command": db_reveal_command,
            "commerce_reveal_command": commerce_reveal_command,
            "actions": actions,
            "navigation": navigation,
            "response_navigation": response_navigation,
            "db_navigation": db_navigation,
            "commerce_navigation": commerce_navigation,
        }));
    }
    let actions = editor_native_host_trace_actions_json(&editor_frames);
    let action_count = actions.len();
    Ok(serde_json::json!({
        "schema_version": 1,
        "kind": "orv.editor.trace",
        "build_dir": dir.display().to_string(),
        "trace": {
            "path": trace_path,
            "kind": trace_value.get("kind").and_then(serde_json::Value::as_str).unwrap_or("unknown"),
            "frame_count": editor_frames.len(),
            "status_counts": editor_trace_status_counts_json(&status_counts),
        },
        "live_refresh": live_refresh,
        "stream_runner": editor_trace_stream_runner_json(dir, live_refresh),
        "actions": actions,
        "action_count": action_count,
        "frames": editor_frames,
    }))
}

pub(crate) fn editor_trace_stream_runner_json(
    dir: &Path,
    live_refresh: &serde_json::Value,
) -> serde_json::Value {
    serde_json::json!({
        "schema_version": 1,
        "kind": "orv.editor.native_host.trace_stream_runner",
        "event_stream": EDITOR_TRACE_STREAM_EVENTS_PATH,
        "command": [
            "orv",
            "editor",
            "trace-stream",
            dir.display().to_string(),
            "--events",
            EDITOR_TRACE_STREAM_EVENTS_PATH,
        ],
        "transport": live_refresh
            .get("transport")
            .cloned()
            .unwrap_or(serde_json::Value::Null),
    })
}

pub(crate) fn editor_trace_stream_json(
    dir: &Path,
    events: &Path,
) -> anyhow::Result<serde_json::Value> {
    let bytes = std::fs::read(events)
        .map_err(|e| anyhow::anyhow!("failed to read {}: {e}", events.display()))?;
    let content_hash = format!("fnv1a64:{:016x}", fnv1a64(&bytes));
    let body = String::from_utf8(bytes)
        .map_err(|e| anyhow::anyhow!("event stream {} must be UTF-8: {e}", events.display()))?;
    let parsed_events = parse_editor_event_source_events(&body);
    let mut trace_events = Vec::new();
    let mut trace_frame_events = Vec::new();
    let mut merged_frames = Vec::new();
    for (index, event) in parsed_events.iter().enumerate() {
        match event.event.as_str() {
            "orv:trace" => {
                let trace_value: serde_json::Value =
                    serde_json::from_str(&event.data).map_err(|e| {
                        anyhow::anyhow!("failed to parse trace event {index} data as JSON: {e}")
                    })?;
                let trace_path = format!("{}#event:{index}", events.display());
                let live_refresh =
                    editor_trace_stream_live_refresh_json(dir, events, &content_hash)?;
                let trace =
                    editor_trace_payload_json(dir, &trace_path, &trace_value, &live_refresh)?;
                merged_frames = trace_value
                    .get("frames")
                    .and_then(serde_json::Value::as_array)
                    .cloned()
                    .unwrap_or_default();
                trace_events.push(serde_json::json!({
                    "index": index,
                    "event": event.event,
                    "data_bytes": event.data.len(),
                    "trace": trace,
                }));
            }
            "orv:trace.frame" => {
                let frame_value: serde_json::Value =
                    serde_json::from_str(&event.data).map_err(|e| {
                        anyhow::anyhow!(
                            "failed to parse trace frame event {index} data as JSON: {e}"
                        )
                    })?;
                let (frame_index, frame) = editor_trace_stream_frame_event_frame(
                    &frame_value,
                    &format!("trace frame event {index}"),
                )?;
                let merged_index = usize::try_from(frame_index)
                    .map_err(|_| anyhow::anyhow!("trace frame event index is too large"))?;
                match merged_index.cmp(&merged_frames.len()) {
                    std::cmp::Ordering::Less => {
                        if merged_frames.get(merged_index) != Some(&frame) {
                            anyhow::bail!(
                                "trace frame event {index} frame must match snapshot frame at index"
                            );
                        }
                    }
                    std::cmp::Ordering::Equal => merged_frames.push(frame.clone()),
                    std::cmp::Ordering::Greater => {
                        anyhow::bail!(
                            "trace frame event {index} index must match frame event order"
                        );
                    }
                }
                trace_frame_events.push(serde_json::json!({
                    "index": index,
                    "event": event.event,
                    "data_bytes": event.data.len(),
                    "frame": frame,
                }));
            }
            _ => {}
        }
    }
    let latest = if trace_frame_events.is_empty() {
        trace_events
            .last()
            .and_then(|event| event.get("trace"))
            .cloned()
            .unwrap_or(serde_json::Value::Null)
    } else {
        let trace_value = serde_json::json!({
            "schema_version": 1,
            "kind": "orv.production.trace",
            "frame_count": merged_frames.len(),
            "frames": merged_frames,
        });
        let trace_path = format!("{}#frames", events.display());
        let live_refresh = editor_trace_stream_live_refresh_json(dir, events, &content_hash)?;
        editor_trace_payload_json(dir, &trace_path, &trace_value, &live_refresh)?
    };
    let mut event_values = Vec::with_capacity(trace_events.len() + trace_frame_events.len());
    event_values.extend(trace_events);
    event_values.extend(trace_frame_events);
    event_values.sort_by_key(|event| {
        event
            .get("index")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(u64::MAX)
    });
    Ok(serde_json::json!({
        "schema_version": 1,
        "kind": "orv.editor.trace.stream",
        "build_dir": dir.display().to_string(),
        "event_stream": {
            "path": events.display().to_string(),
            "content_type": "text/event-stream",
            "content_hash": content_hash,
            "event_count": parsed_events.len(),
            "trace_event_count": event_values.iter().filter(|event| event["event"] == "orv:trace").count(),
            "trace_frame_event_count": event_values.iter().filter(|event| event["event"] == "orv:trace.frame").count(),
        },
        "latest": latest,
        "events": event_values,
    }))
}

pub(crate) fn verify_editor_runtime_trace_document_contract_keys(
    trace: &serde_json::Value,
    context: &str,
) -> anyhow::Result<()> {
    verify_json_object_keys_exact(
        trace,
        &["schema_version", "kind", "frame_count", "frames"],
        context,
    )?;
    if trace
        .get("schema_version")
        .and_then(serde_json::Value::as_u64)
        != Some(1)
    {
        anyhow::bail!("{context} schema_version must be 1");
    }
    if json_str(trace, "kind", context)? != "orv.production.trace" {
        anyhow::bail!("{context} kind must be orv.production.trace");
    }
    let frames = trace
        .get("frames")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| anyhow::anyhow!("{context} must contain frames array"))?;
    let frame_count = trace
        .get("frame_count")
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| anyhow::anyhow!("{context} frame_count must be an unsigned integer"))?;
    if frame_count != frames.len() as u64 {
        anyhow::bail!("{context} frame_count must match frames length");
    }
    for (index, frame) in frames.iter().enumerate() {
        verify_editor_runtime_trace_frame_contract_keys(
            frame,
            &format!("{context} frames[{index}]"),
        )?;
    }
    Ok(())
}

pub(crate) fn editor_trace_stream_frame_event_frame(
    value: &serde_json::Value,
    context: &str,
) -> anyhow::Result<(u64, serde_json::Value)> {
    verify_json_object_keys_exact(
        value,
        &["schema_version", "kind", "index", "frame"],
        context,
    )?;
    if value
        .get("schema_version")
        .and_then(serde_json::Value::as_u64)
        != Some(1)
    {
        anyhow::bail!("{context} schema_version must be 1");
    }
    if json_str(value, "kind", context)? != "orv.production.trace.frame" {
        anyhow::bail!("{context} kind must be orv.production.trace.frame");
    }
    let index = value
        .get("index")
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| anyhow::anyhow!("{context} index must be an unsigned integer"))?;
    let frame = value
        .get("frame")
        .ok_or_else(|| anyhow::anyhow!("{context} frame must be an object"))?;
    verify_editor_runtime_trace_frame_contract_keys(frame, &format!("{context}.frame"))?;
    Ok((index, frame.clone()))
}

pub(super) const EDITOR_RUNTIME_TRACE_FRAME_REQUIRED_KEYS: [&str; 10] = [
    "method",
    "path",
    "status",
    "route_method",
    "route_path",
    "route_origin_id",
    "response_origin_id",
    "params",
    "query",
    "body",
];

pub(super) const EDITOR_RUNTIME_TRACE_FRAME_ALLOWED_KEYS: [&str; 12] = [
    "method",
    "path",
    "status",
    "route_method",
    "route_path",
    "route_origin_id",
    "response_origin_id",
    "params",
    "query",
    "body",
    "db_operation_origin_id",
    "commerce_adapter_origin_id",
];

pub(crate) fn verify_editor_runtime_trace_frame_contract_keys(
    frame: &serde_json::Value,
    context: &str,
) -> anyhow::Result<()> {
    let object = frame
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("{context} must be an object"))?;
    if object
        .keys()
        .any(|key| !EDITOR_RUNTIME_TRACE_FRAME_ALLOWED_KEYS.contains(&key.as_str()))
    {
        anyhow::bail!("{context} keys must match contract");
    }
    for key in EDITOR_RUNTIME_TRACE_FRAME_REQUIRED_KEYS {
        if !object.contains_key(key) {
            anyhow::bail!("{context}.{key} is required");
        }
    }
    for key in ["method", "path", "body"] {
        verify_optional_trace_string(frame, key, context)?;
    }
    for key in [
        "route_method",
        "route_path",
        "route_origin_id",
        "response_origin_id",
        "db_operation_origin_id",
        "commerce_adapter_origin_id",
    ] {
        verify_optional_trace_string_or_null(frame, key, context)?;
    }
    if frame
        .get("status")
        .is_some_and(|status| status.as_u64().is_none())
    {
        anyhow::bail!("{context}.status must be an unsigned integer");
    }
    for key in ["params", "query"] {
        verify_optional_trace_string_map(frame, key, context)?;
    }
    Ok(())
}

pub(crate) fn verify_optional_trace_string(
    frame: &serde_json::Value,
    key: &str,
    context: &str,
) -> anyhow::Result<()> {
    if frame.get(key).is_some_and(|value| value.as_str().is_none()) {
        anyhow::bail!("{context}.{key} must be a string");
    }
    Ok(())
}

pub(crate) fn verify_optional_trace_string_or_null(
    frame: &serde_json::Value,
    key: &str,
    context: &str,
) -> anyhow::Result<()> {
    if frame
        .get(key)
        .is_some_and(|value| !value.is_null() && value.as_str().is_none())
    {
        anyhow::bail!("{context}.{key} must be a string or null");
    }
    Ok(())
}

pub(crate) fn verify_optional_trace_string_map(
    frame: &serde_json::Value,
    key: &str,
    context: &str,
) -> anyhow::Result<()> {
    let Some(value) = frame.get(key) else {
        return Ok(());
    };
    let object = value
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("{context}.{key} must be an object"))?;
    if object.values().any(|value| value.as_str().is_none()) {
        anyhow::bail!("{context}.{key} values must be strings");
    }
    Ok(())
}

pub(crate) fn editor_trace_stream_live_refresh_json(
    dir: &Path,
    events: &Path,
    content_hash: &str,
) -> anyhow::Result<serde_json::Value> {
    let mut refresh = serde_json::json!({
        "strategy": "event-source-snapshot",
        "watch": {
            "event_stream": {
                "path": events.display().to_string(),
                "content_hash": content_hash,
            },
        },
    });
    if let Some(transport) = editor_trace_live_transport_json(dir)? {
        refresh["transport"] = transport;
    }
    Ok(refresh)
}

pub(crate) struct EditorEventSourceEvent {
    pub(crate) event: String,
    pub(crate) data: String,
}

pub(crate) fn parse_editor_event_source_events(body: &str) -> Vec<EditorEventSourceEvent> {
    let mut events = Vec::new();
    let mut event = String::from("message");
    let mut data_lines = Vec::new();
    for line in body.lines() {
        if line.is_empty() {
            flush_editor_event_source_event(&mut events, &mut event, &mut data_lines);
            continue;
        }
        if line.starts_with(':') {
            continue;
        }
        let (field, value) = line.split_once(':').map_or((line, ""), |(field, value)| {
            (field, value.strip_prefix(' ').unwrap_or(value))
        });
        match field {
            "event" => event = value.to_string(),
            "data" => data_lines.push(value.to_string()),
            _ => {}
        }
    }
    flush_editor_event_source_event(&mut events, &mut event, &mut data_lines);
    events
}

pub(crate) fn flush_editor_event_source_event(
    events: &mut Vec<EditorEventSourceEvent>,
    event: &mut String,
    data_lines: &mut Vec<String>,
) {
    if !data_lines.is_empty() {
        events.push(EditorEventSourceEvent {
            event: event.clone(),
            data: data_lines.join("\n"),
        });
        data_lines.clear();
    }
    *event = String::from("message");
}

pub(crate) fn editor_trace_live_refresh_json(
    dir: &Path,
    trace: &Path,
) -> anyhow::Result<serde_json::Value> {
    let bytes = std::fs::read(trace)
        .map_err(|e| anyhow::anyhow!("failed to read {}: {e}", trace.display()))?;
    let mut refresh = serde_json::json!({
        "strategy": "trace-file-hash",
        "watch": {
            "trace": {
                "path": trace.display().to_string(),
                "content_hash": format!("fnv1a64:{:016x}", fnv1a64(&bytes)),
            },
        },
    });
    if let Some(transport) = editor_trace_live_transport_json(dir)? {
        refresh["transport"] = transport;
    }
    Ok(refresh)
}

pub(crate) fn editor_trace_live_transport_json(
    dir: &Path,
) -> anyhow::Result<Option<serde_json::Value>> {
    let path = dir.join("server").join("app.orv-runtime.json");
    if !path.is_file() {
        return Ok(None);
    }
    let artifact = read_server_artifact(&path)?;
    let Some(listen) = artifact.listen.as_ref() else {
        return Ok(None);
    };
    if listen.port == Some(0) {
        return Ok(None);
    }
    Ok(Some(serde_json::json!({
        "kind": "event-source",
        "event": "orv:trace",
        "url": deploy_runbook_trace_events_url(Some(listen)),
    })))
}

#[derive(Default)]
pub(crate) struct EditorTraceStatusCounts {
    pub(crate) total: usize,
    pub(crate) ok: usize,
    pub(crate) redirect: usize,
    pub(crate) client_error: usize,
    pub(crate) server_error: usize,
    pub(crate) other: usize,
}

impl EditorTraceStatusCounts {
    fn record(&mut self, status: Option<u64>) {
        self.total += 1;
        match editor_trace_status_class(status) {
            "ok" => self.ok += 1,
            "redirect" => self.redirect += 1,
            "client_error" => self.client_error += 1,
            "server_error" => self.server_error += 1,
            _ => self.other += 1,
        }
    }
}

pub(crate) fn editor_trace_status_counts_json(
    counts: &EditorTraceStatusCounts,
) -> serde_json::Value {
    serde_json::json!({
        "total": counts.total,
        "ok": counts.ok,
        "redirect": counts.redirect,
        "client_error": counts.client_error,
        "server_error": counts.server_error,
        "other": counts.other,
    })
}

pub(crate) fn editor_trace_summary_json(
    request: &serde_json::Value,
    origin_id: Option<&str>,
    response_origin_id: Option<&str>,
    db_operation_origin_id: Option<&str>,
    commerce_adapter_origin_id: Option<&str>,
) -> serde_json::Value {
    let method = request
        .get("method")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("");
    let path = request
        .get("path")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("");
    let status = request.get("status").and_then(serde_json::Value::as_u64);
    serde_json::json!({
        "label": editor_trace_request_label(method, path, status),
        "route": editor_trace_route_label(request),
        "status": status,
        "status_class": editor_trace_status_class(status),
        "origin_id": origin_id,
        "response_origin_id": response_origin_id,
        "db_operation_origin_id": db_operation_origin_id,
        "commerce_adapter_origin_id": commerce_adapter_origin_id,
    })
}

pub(crate) fn editor_trace_request_label(method: &str, path: &str, status: Option<u64>) -> String {
    let request = match (method.is_empty(), path.is_empty()) {
        (true, true) => "request".to_string(),
        (true, false) => path.to_string(),
        (false, true) => method.to_string(),
        (false, false) => format!("{method} {path}"),
    };
    if let Some(status) = status {
        format!("{request} -> {status}")
    } else {
        request
    }
}

pub(crate) fn editor_trace_route_label(request: &serde_json::Value) -> Option<String> {
    let method = request
        .get("route_method")
        .and_then(serde_json::Value::as_str)
        .filter(|method| !method.is_empty());
    let path = request
        .get("route_path")
        .and_then(serde_json::Value::as_str)
        .filter(|path| !path.is_empty());
    match (method, path) {
        (Some(method), Some(path)) => Some(format!("{method} {path}")),
        (Some(method), None) => Some(method.to_string()),
        (None, Some(path)) => Some(path.to_string()),
        (None, None) => None,
    }
}

pub(crate) const fn editor_trace_status_class(status: Option<u64>) -> &'static str {
    match status {
        Some(200..=299) => "ok",
        Some(300..=399) => "redirect",
        Some(400..=499) => "client_error",
        Some(500..=599) => "server_error",
        _ => "other",
    }
}

pub(crate) fn editor_trace_frame_origin_id(frame: &serde_json::Value) -> Option<&str> {
    frame
        .get("route_origin_id")
        .or_else(|| frame.get("origin_id"))
        .and_then(serde_json::Value::as_str)
        .filter(|origin_id| !origin_id.is_empty())
}

pub(crate) fn editor_trace_frame_response_origin_id(frame: &serde_json::Value) -> Option<&str> {
    editor_trace_frame_named_origin_id(frame, "response_origin_id")
}

pub(crate) fn editor_trace_frame_named_origin_id<'a>(
    frame: &'a serde_json::Value,
    key: &str,
) -> Option<&'a str> {
    frame
        .get(key)
        .and_then(serde_json::Value::as_str)
        .filter(|origin_id| !origin_id.is_empty())
}

pub(crate) fn editor_trace_request_json(frame: &serde_json::Value) -> serde_json::Value {
    let mut request = serde_json::Map::new();
    for key in [
        "method",
        "path",
        "status",
        "route_method",
        "route_path",
        "route_origin_id",
        "response_origin_id",
        "db_operation_origin_id",
        "commerce_adapter_origin_id",
        "params",
        "query",
        "body",
    ] {
        if let Some(value) = frame.get(key) {
            request.insert(key.to_string(), value.clone());
        }
    }
    serde_json::Value::Object(request)
}

pub(crate) fn editor_export_state_json_with_trace(
    path: &Path,
    build: Option<&Path>,
    trace: Option<&Path>,
) -> anyhow::Result<serde_json::Value> {
    let mut state = editor_export_state_json(path)?;
    if let Some(build) = build {
        state
            .as_object_mut()
            .expect("editor export state is object")
            .insert(
                "production".to_string(),
                editor_production_summary_json(build)?,
            );
        editor_debug_attach_production_context(&mut state);
    }
    if let Some(trace) = trace {
        let build = build.ok_or_else(|| anyhow::anyhow!("--build is required with --trace"))?;
        state
            .as_object_mut()
            .expect("editor export state is object")
            .insert("trace".to_string(), editor_trace_json(build, trace)?);
    }
    Ok(state)
}

pub(crate) fn editor_trace_panel_artifact_json() -> serde_json::Value {
    serde_json::json!({
        "schema_version": 1,
        "kind": "orv.editor.trace.panel",
        "path": EDITOR_TRACE_PANEL_HTML_PATH,
        "media_type": "text/html",
        "source": "native-host.trace",
        "panel_contract": editor_native_host_trace_panel_contract_json(),
    })
}

pub(crate) fn editor_trace_action_result_artifact_json() -> serde_json::Value {
    serde_json::json!({
        "schema_version": 1,
        "kind": "orv.editor.trace.action.result",
        "path": EDITOR_TRACE_ACTION_RESULT_PATH,
        "html_path": EDITOR_TRACE_ACTION_RESULT_HTML_PATH,
        "media_type": "application/json",
        "source": "native-host.trace.actions",
        "panel_contract": editor_trace_action_result_panel_contract_json(),
    })
}

pub(crate) fn editor_trace_action_runner_json() -> serde_json::Value {
    serde_json::json!({
        "schema_version": 1,
        "kind": "orv.editor.native_host.trace_action_runner",
        "input": EDITOR_NATIVE_HOST_MANIFEST_PATH,
        "result": editor_trace_action_result_artifact_json(),
        "command_format": [
            "orv",
            "editor",
            "run-action",
            EDITOR_NATIVE_HOST_MANIFEST_PATH,
            "--action",
            "<trace.*.reveal>",
            "--frame-index",
            "<index>",
            "--slot",
            "<route|response|db|commerce>",
        ],
    })
}

pub(crate) fn editor_trace_action_runner_command_json(
    frame_index: &serde_json::Value,
    action: &str,
    slot: &str,
) -> serde_json::Value {
    let Some(frame_index) = frame_index.as_u64() else {
        return serde_json::Value::Null;
    };
    serde_json::json!([
        "orv",
        "editor",
        "run-action",
        EDITOR_NATIVE_HOST_MANIFEST_PATH,
        "--action",
        action,
        "--frame-index",
        frame_index,
        "--slot",
        slot,
    ])
}

pub(crate) fn editor_trace_action_result_panel_contract_json() -> serde_json::Value {
    serde_json::json!({
        "schema_version": 1,
        "root": "panels.trace_action",
        "sections": [
            {
                "name": "summary",
                "path": "panels.trace_action.summary",
                "kind": "object",
            },
            {
                "name": "action",
                "path": "panels.trace_action.action",
                "kind": "object",
            },
            {
                "name": "command",
                "path": "panels.trace_action.command",
                "kind": "array",
            },
            {
                "name": "navigation",
                "path": "panels.trace_action.navigation",
                "kind": "object",
            },
            {
                "name": "source",
                "path": "panels.trace_action.source",
                "kind": "object",
            },
            {
                "name": "production",
                "path": "panels.trace_action.production",
                "kind": "object",
            },
        ],
    })
}

pub(crate) fn editor_trace_action_result_root(host: &Path) -> Option<PathBuf> {
    if host.is_dir() {
        Some(host.to_path_buf())
    } else {
        host.parent().map(Path::to_path_buf)
    }
}

pub(crate) fn write_editor_trace_action_result_if_configured(
    host: &Path,
    value: &serde_json::Value,
) -> anyhow::Result<bool> {
    let Some(root) = editor_trace_action_result_root(host) else {
        return Ok(false);
    };
    write_json(&root.join(EDITOR_TRACE_ACTION_RESULT_PATH), value)?;
    Ok(true)
}

pub(crate) fn write_editor_trace_action_result_html_if_configured(
    host: &Path,
    value: &serde_json::Value,
) -> anyhow::Result<bool> {
    let Some(root) = editor_trace_action_result_root(host) else {
        return Ok(false);
    };
    write_text(
        &root.join(EDITOR_TRACE_ACTION_RESULT_HTML_PATH),
        &editor_trace_action_result_html(value)?,
    )?;
    Ok(true)
}

pub(crate) fn editor_trace_action_result_html(value: &serde_json::Value) -> anyhow::Result<String> {
    let panel = value
        .pointer("/panels/trace_action")
        .unwrap_or(&serde_json::Value::Null);
    let summary_json = html_escape_text(&serde_json::to_string_pretty(
        panel.get("summary").unwrap_or(&serde_json::Value::Null),
    )?);
    let action_json = html_escape_text(&serde_json::to_string_pretty(
        panel.get("action").unwrap_or(&serde_json::Value::Null),
    )?);
    let navigation_json = html_escape_text(&serde_json::to_string_pretty(
        panel.get("navigation").unwrap_or(&serde_json::Value::Null),
    )?);
    let source = panel.get("source").unwrap_or(&serde_json::Value::Null);
    let source_path = html_escape_text(
        source
            .get("path")
            .and_then(serde_json::Value::as_str)
            .unwrap_or(""),
    );
    let source_snippet = html_escape_text(
        source
            .get("snippet")
            .and_then(serde_json::Value::as_str)
            .unwrap_or(""),
    );
    let command = panel
        .get("command")
        .and_then(serde_json::Value::as_array)
        .map(|parts| {
            parts
                .iter()
                .filter_map(serde_json::Value::as_str)
                .collect::<Vec<_>>()
                .join(" ")
        })
        .unwrap_or_default();
    let command = html_escape_text(&command);
    Ok(format!(
        "<!doctype html>\n<html lang=\"en\">\n<head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width, initial-scale=1\"><title>orv Trace Action Result</title><style>:root{{color-scheme:light dark;--bg:#f7f8fb;--fg:#18202f;--panel:#fff;--line:#d7dce5;--muted:#687386;}}@media (prefers-color-scheme: dark){{:root{{--bg:#111827;--fg:#f8fafc;--panel:#1f2937;--line:#334155;--muted:#cbd5e1;}}}}body{{margin:0;background:var(--bg);color:var(--fg);font:14px/1.45 ui-sans-serif,system-ui,-apple-system,BlinkMacSystemFont,\"Segoe UI\",sans-serif}}header{{padding:22px 28px;border-bottom:1px solid var(--line)}}main{{display:grid;grid-template-columns:1fr 1fr;gap:16px;padding:18px 28px}}section{{border:1px solid var(--line);background:var(--panel);border-radius:8px;padding:14px}}.wide{{grid-column:1/-1}}h1{{font-size:22px;margin:0 0 6px}}h2{{font-size:13px;text-transform:uppercase;color:var(--muted);margin:0 0 10px}}pre{{white-space:pre-wrap;word-break:break-word;margin:0;overflow:auto}}@media(max-width:820px){{main{{grid-template-columns:1fr;padding:14px}}}}</style></head>\n<body><header><h1>Trace Action Result</h1><p>{command}</p><p>{source_path}</p></header><main><section><h2>Summary</h2><pre>{summary_json}</pre></section><section><h2>Action</h2><pre>{action_json}</pre></section><section class=\"wide\"><h2>Source</h2><pre>{source_snippet}</pre></section><section class=\"wide\"><h2>Navigation</h2><pre>{navigation_json}</pre></section></main></body></html>\n"
    ))
}

pub(crate) fn editor_trace_frame_reveal_command_json(
    build_dir: &str,
    origin_id: Option<&str>,
) -> serde_json::Value {
    editor_reveal_command_json(build_dir, origin_id)
}

pub(crate) fn write_editor_trace_panel_html_if_configured(
    out: &Path,
    state: &serde_json::Value,
) -> anyhow::Result<bool> {
    if state.get("trace").is_none() {
        return Ok(false);
    }
    let trace = editor_native_host_trace_json(state);
    let html = editor_trace_panel_html(&trace)?;
    write_text(&out.join(EDITOR_TRACE_PANEL_HTML_PATH), &html)?;
    Ok(true)
}

pub(crate) fn editor_trace_panel_html(trace: &serde_json::Value) -> anyhow::Result<String> {
    let summary = trace
        .get("summary")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));
    let transport = trace
        .get("transport")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let stream_runner = trace
        .get("stream_runner")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let panel_contract = trace
        .get("panel_contract")
        .cloned()
        .unwrap_or_else(editor_native_host_trace_panel_contract_json);
    let trace_json = html_script_json(&serde_json::to_string_pretty(trace)?);
    let transport_json = html_escape_text(&serde_json::to_string_pretty(&transport)?);
    let stream_runner_json = html_escape_text(&serde_json::to_string_pretty(&stream_runner)?);
    let panel_contract_json = html_escape_text(&serde_json::to_string_pretty(&panel_contract)?);
    let trace_path = html_escape_text(
        summary
            .get("trace_path")
            .and_then(serde_json::Value::as_str)
            .unwrap_or(""),
    );
    let build_dir = html_escape_text(
        summary
            .get("build_dir")
            .and_then(serde_json::Value::as_str)
            .unwrap_or(""),
    );
    let frame_count = json_usize_field(&summary, "frame_count");
    let status_counts = summary
        .get("status_counts")
        .unwrap_or(&serde_json::Value::Null);
    let ok_count = json_usize_field(status_counts, "ok");
    let client_error_count = json_usize_field(status_counts, "client_error");
    let server_error_count = json_usize_field(status_counts, "server_error");
    let first_request = trace_panel_request_label(summary.get("first_request"));
    let last_request = trace_panel_request_label(summary.get("last_request"));
    let mut html = String::new();
    html.push_str(
        "<!doctype html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n<title>orv Trace Panel</title>\n<style>\n:root{color-scheme:light dark;--bg:#f7f7f4;--fg:#161714;--muted:#6b6f69;--panel:#ffffff;--line:#d7d9d2;--accent:#0d6b5f;--accent-weak:#dcefeb;--bad:#a43434;--warn:#8a5a00;}\n@media (prefers-color-scheme: dark){:root{--bg:#11130f;--fg:#ecefe8;--muted:#a8aea2;--panel:#191c17;--line:#30362d;--accent:#67c7b5;--accent-weak:#203a35;--bad:#ff9d9d;--warn:#e8c06b;}}\n*{box-sizing:border-box}body{margin:0;background:var(--bg);color:var(--fg);font:14px/1.45 ui-sans-serif,system-ui,-apple-system,BlinkMacSystemFont,\"Segoe UI\",sans-serif;}header{padding:24px 28px 12px;border-bottom:1px solid var(--line);}h1{font-size:24px;margin:0 0 8px;}h2{font-size:13px;text-transform:uppercase;letter-spacing:.08em;color:var(--muted);margin:0 0 12px}.muted{color:var(--muted)}main{display:grid;grid-template-columns:minmax(280px,380px) minmax(0,1fr);gap:16px;padding:16px 28px 28px}.panel{background:var(--panel);border:1px solid var(--line);border-radius:8px;padding:16px}.summary{display:grid;grid-template-columns:repeat(4,minmax(0,1fr));gap:10px;margin-top:16px}.metric{border:1px solid var(--line);border-radius:6px;padding:10px;background:var(--bg)}.metric b{display:block;font-size:22px;line-height:1.1}.filterbar{display:flex;flex-wrap:wrap;gap:8px}.filterbar button{border:1px solid var(--line);background:var(--bg);color:var(--fg);border-radius:6px;padding:7px 10px;cursor:pointer}.filterbar button[aria-pressed=\"true\"]{border-color:var(--accent);background:var(--accent-weak)}.list{list-style:none;margin:0;padding:0;display:grid;gap:8px}.list li{border:1px solid var(--line);border-radius:6px;padding:10px;cursor:pointer;background:var(--bg)}.list li:focus,.list li:hover{outline:2px solid var(--accent);outline-offset:1px}.status-client_error,.status-server_error{color:var(--bad)}.status-redirect,.status-other{color:var(--warn)}pre{margin:0;white-space:pre-wrap;overflow:auto;font:12px/1.45 ui-monospace,SFMono-Regular,Menlo,Consolas,monospace}.detail-grid{display:grid;grid-template-columns:1fr 1fr;gap:16px}.wide{grid-column:1/-1}@media (max-width:900px){main{grid-template-columns:1fr;padding:14px}.summary,.detail-grid{grid-template-columns:1fr}header{padding:18px 14px 8px}}\n</style>\n</head>\n<body>\n",
    );
    writeln!(
        &mut html,
        "<header><h1>Trace Panel</h1><div class=\"muted\">{trace_path}</div><div class=\"muted\">{build_dir}</div><section class=\"summary\"><div class=\"metric\"><span>Frames</span><b>{frame_count}</b></div><div class=\"metric\"><span>OK</span><b>{ok_count}</b></div><div class=\"metric\"><span>Client Err</span><b>{client_error_count}</b></div><div class=\"metric\"><span>Server Err</span><b>{server_error_count}</b></div></section></header>"
    )?;
    writeln!(
        &mut html,
        "<main><section class=\"panel\"><h2>Status Filters</h2><div id=\"trace-filterbar\" class=\"filterbar\"></div><p class=\"muted\">First: {}</p><p class=\"muted\">Last: {}</p></section>",
        html_escape_text(&first_request),
        html_escape_text(&last_request)
    )?;
    html.push_str(
        "<section class=\"panel\"><h2>Frame Detail</h2><pre id=\"trace-frame-detail\">No trace frame selected.</pre></section>\n<section class=\"panel\"><h2>Frames</h2><ul id=\"trace-frame-list\" class=\"list\"></ul></section>\n<section class=\"detail-grid\">\n",
    );
    writeln!(
        &mut html,
        "<section class=\"panel\"><h2>Transport</h2><pre>{transport_json}</pre></section><section class=\"panel\"><h2>Trace Stream Runner</h2><pre>{stream_runner_json}</pre></section><section class=\"panel wide\"><h2>Panel Contract</h2><pre>{panel_contract_json}</pre></section></section></main>"
    )?;
    writeln!(
        &mut html,
        "<script id=\"orv-trace\" type=\"application/json\">{trace_json}</script>"
    )?;
    html.push_str(
        "<script>\nconst trace = JSON.parse(document.getElementById('orv-trace').textContent);\nconst frames = Array.isArray(trace.frames) ? trace.frames : [];\nconst filters = Array.isArray(trace.status_filters) ? trace.status_filters : [];\nconst filterbar = document.getElementById('trace-filterbar');\nconst list = document.getElementById('trace-frame-list');\nconst detail = document.getElementById('trace-frame-detail');\nfunction frameLabel(frame){\n  return frame?.summary?.label || `${frame?.request?.method || ''} ${frame?.request?.path || ''}`.trim() || frame?.origin_id || 'request';\n}\nfunction renderDetail(frame){\n  if (!frame) { detail.textContent = 'No trace frame selected.'; return; }\n  const source = frame.source || {};\n  const production = frame.production || {};\n  const request = frame.request || {};\n  const lines = [\n    frameLabel(frame),\n    frame.summary?.status_class ? `status ${frame.summary.status_class}` : '',\n    frame.origin_id ? `origin ${frame.origin_id}` : '',\n    source.path ? `source ${source.path}${source.location?.line ? `:${source.location.line}` : ''}` : '',\n    production.path ? `production ${production.path}` : '',\n    Array.isArray(frame.reveal_command) ? `reveal ${frame.reveal_command.join(' ')}` : '',\n    request.params && Object.keys(request.params).length ? `params ${JSON.stringify(request.params)}` : '',\n    request.query && Object.keys(request.query).length ? `query ${JSON.stringify(request.query)}` : '',\n    request.body ? `body ${request.body}` : '',\n    source.snippet || ''\n  ].filter(Boolean);\n  detail.textContent = lines.join('\\n');\n}\nfunction renderFrames(filter){\n  const rows = filter === 'all' ? frames : frames.filter(frame => frame.summary?.status_class === filter);\n  list.textContent = '';\n  for (const frame of rows) {\n    const row = document.createElement('li');\n    const status = frame.summary?.status_class || 'other';\n    row.className = `status-${status}`;\n    row.textContent = frameLabel(frame);\n    row.tabIndex = 0;\n    row.addEventListener('click', () => renderDetail(frame));\n    row.addEventListener('keydown', event => {\n      if (event.key === 'Enter' || event.key === ' ') { event.preventDefault(); renderDetail(frame); }\n    });\n    list.appendChild(row);\n  }\n  renderDetail(rows[0]);\n}\nfor (const filter of filters) {\n  const button = document.createElement('button');\n  button.type = 'button';\n  button.dataset.filter = filter.name || 'all';\n  button.setAttribute('aria-pressed', button.dataset.filter === 'all' ? 'true' : 'false');\n  button.textContent = `${filter.label || filter.name || 'Filter'} ${filter.count ?? 0}`;\n  button.addEventListener('click', () => {\n    for (const item of filterbar.querySelectorAll('button')) item.setAttribute('aria-pressed', 'false');\n    button.setAttribute('aria-pressed', 'true');\n    renderFrames(button.dataset.filter || 'all');\n  });\n  filterbar.appendChild(button);\n}\nif (!filters.length) {\n  const empty = document.createElement('span');\n  empty.className = 'muted';\n  empty.textContent = 'No trace filters.';\n  filterbar.appendChild(empty);\n}\nrenderFrames('all');\n</script>\n</body>\n</html>\n",
    );
    Ok(html)
}

pub(crate) fn trace_panel_request_label(value: Option<&serde_json::Value>) -> String {
    value
        .and_then(|value| value.get("label"))
        .and_then(serde_json::Value::as_str)
        .unwrap_or("")
        .to_string()
}

pub(crate) fn write_trace_panel_html(
    html: &mut String,
    trace_count: usize,
    trace_status_counts: &EditorTraceStatusCounts,
) -> anyhow::Result<()> {
    write!(
        html,
        "<section class=\"panel\"><h2>Trace</h2><div class=\"metric\">{trace_count}</div><div id=\"trace-status-summary\" class=\"nav\">"
    )?;
    write!(
        html,
        "<span>OK<b>{}</b></span><span>Client Err<b>{}</b></span><span>Server Err<b>{}</b></span>",
        trace_status_counts.ok, trace_status_counts.client_error, trace_status_counts.server_error
    )?;
    html.push_str("</div><div class=\"filterbar\">");
    for (filter, label, count) in [
        ("all", "All", trace_status_counts.total),
        ("ok", "OK", trace_status_counts.ok),
        ("redirect", "3xx", trace_status_counts.redirect),
        ("client_error", "4xx", trace_status_counts.client_error),
        ("server_error", "5xx", trace_status_counts.server_error),
        ("other", "Other", trace_status_counts.other),
    ] {
        write!(
            html,
            "<button type=\"button\" data-trace-filter=\"{}\" aria-pressed=\"{}\">{}<b>{}</b></button>",
            filter,
            if filter == "all" { "true" } else { "false" },
            label,
            count
        )?;
    }
    html.push_str("</div><ul id=\"trace-list\" class=\"list\"></ul></section>");
    Ok(())
}

pub(crate) fn editor_trace_status_counts_from_state(
    state: &serde_json::Value,
) -> EditorTraceStatusCounts {
    let mut counts = EditorTraceStatusCounts::default();
    let Some(value) = state.pointer("/trace/trace/status_counts") else {
        return counts;
    };
    counts.total = json_usize_field(value, "total");
    counts.ok = json_usize_field(value, "ok");
    counts.redirect = json_usize_field(value, "redirect");
    counts.client_error = json_usize_field(value, "client_error");
    counts.server_error = json_usize_field(value, "server_error");
    counts.other = json_usize_field(value, "other");
    counts
}
