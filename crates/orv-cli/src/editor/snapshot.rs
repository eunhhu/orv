#![allow(clippy::redundant_pub_crate, clippy::wildcard_imports)]

use super::*;

pub(crate) fn cmd_editor_snapshot(path: &Path) -> anyhow::Result<()> {
    let entry = project_entry_path(path)?;
    let value = editor_snapshot_json(&entry)?;
    println!("{}", serde_json::to_string_pretty(&value)?);
    Ok(())
}

pub(crate) fn cmd_editor_reveal(dir: &Path, origin_id: &str) -> anyhow::Result<()> {
    let value = editor_reveal_json(dir, origin_id)?;
    println!("{}", serde_json::to_string_pretty(&value)?);
    Ok(())
}

pub(crate) fn cmd_editor_runtime(path: &Path) -> anyhow::Result<()> {
    let entry = project_entry_path(path)?;
    let value = editor_runtime_json(&entry)?;
    println!("{}", serde_json::to_string_pretty(&value)?);
    Ok(())
}

pub(crate) fn cmd_editor_run_action(
    host: &Path,
    action: &str,
    frame_index: Option<u64>,
    slot: Option<&str>,
) -> anyhow::Result<()> {
    let value = editor_native_host_run_action_json(host, action, frame_index, slot)?;
    write_editor_trace_action_result_if_configured(host, &value)?;
    write_editor_trace_action_result_html_if_configured(host, &value)?;
    println!("{}", serde_json::to_string_pretty(&value)?);
    Ok(())
}

pub(crate) fn read_first_json_value_from_reader<R: std::io::Read>(
    reader: &mut R,
) -> anyhow::Result<serde_json::Value> {
    let mut bytes = Vec::new();
    let mut buffer = [0_u8; 1];
    let mut started = false;
    let mut depth = 0_i32;
    let mut in_string = false;
    let mut escaped = false;
    loop {
        let read = std::io::Read::read(reader, &mut buffer)?;
        if read == 0 {
            break;
        }
        let byte = buffer[0];
        if !started {
            if byte.is_ascii_whitespace() {
                continue;
            }
            if byte != b'{' && byte != b'[' {
                anyhow::bail!(
                    "desktop host ready payload must start with JSON object or array, got byte {byte}"
                );
            }
            started = true;
            depth = 1;
            bytes.push(byte);
            continue;
        }
        bytes.push(byte);
        if in_string {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                in_string = false;
            }
            continue;
        }
        match byte {
            b'"' => in_string = true,
            b'{' | b'[' => depth += 1,
            b'}' | b']' => {
                depth -= 1;
                if depth == 0 {
                    break;
                }
            }
            _ => {}
        }
    }
    if bytes.is_empty() || depth != 0 {
        anyhow::bail!("desktop host did not emit a complete ready JSON payload");
    }
    serde_json::from_slice(&bytes)
        .map_err(|e| anyhow::anyhow!("invalid desktop host ready JSON: {e}"))
}

pub(crate) fn editor_snapshot_json(path: &Path) -> anyhow::Result<serde_json::Value> {
    let loaded = orv_project::load_project(path).map_err(|e| anyhow::anyhow!("{e}"))?;
    let resolved = orv_resolve::resolve(&loaded.program);
    let lowered = orv_analyzer::lower_with_diagnostics(&loaded.program, &resolved);
    let origin_map = orv_compiler::origin_map(&lowered.program);
    let mut diagnostics = Vec::new();
    diagnostics.extend(lsp_diagnostics_json(&loaded.diagnostics, &loaded.files));
    diagnostics.extend(lsp_diagnostics_json(&resolved.diagnostics, &loaded.files));
    diagnostics.extend(lsp_diagnostics_json(&lowered.diagnostics, &loaded.files));
    let project_graph = project_graph_json(&loaded.graph, &origin_map);
    let live_refresh = editor_live_refresh_json(&loaded.files, &project_graph)?;
    Ok(serde_json::json!({
        "schema_version": 1,
        "entry": {
            "path": path.display().to_string(),
            "uri": lsp_file_uri_for_path(path),
        },
        "diagnostics": diagnostics,
        "project_graph": project_graph,
        "live_refresh": live_refresh,
        "panels": {
            "files": editor_files_panel_json(&loaded.files, &loaded.graph),
            "routes": editor_routes_panel_json(&origin_map, &loaded.files),
            "schema": editor_schema_panel_json(&loaded.graph, &loaded.files),
            "domains": editor_domains_panel_json(&loaded.graph, &loaded.files),
        },
    }))
}

pub(crate) fn editor_live_refresh_json(
    files: &[SourceFile],
    project_graph: &serde_json::Value,
) -> anyhow::Result<serde_json::Value> {
    Ok(serde_json::json!({
        "strategy": "source-hash",
        "project_graph_hash": stable_json_hash(project_graph)?,
        "watch": {
            "sources": editor_source_watch_json(files),
        },
    }))
}

pub(crate) fn editor_source_watch_json(files: &[SourceFile]) -> Vec<serde_json::Value> {
    files
        .iter()
        .map(|file| {
            serde_json::json!({
                "file": file.id.0,
                "path": file.path.display().to_string(),
                "uri": lsp_file_uri_for_path(&file.path),
                "content_hash": format!("fnv1a64:{:016x}", fnv1a64(file.source.as_bytes())),
            })
        })
        .collect()
}

pub(crate) fn editor_files_panel_json(
    files: &[SourceFile],
    graph: &ProjectGraph,
) -> Vec<serde_json::Value> {
    files
        .iter()
        .map(|file| {
            let node_id = graph
                .nodes
                .iter()
                .find(|node| node.kind == ProjectNodeKind::File && node.file == file.id)
                .map(|node| node.id);
            serde_json::json!({
                "file": file.id.0,
                "name": file.path.file_name().and_then(std::ffi::OsStr::to_str).unwrap_or(""),
                "path": file.path.display().to_string(),
                "uri": lsp_file_uri_for_path(&file.path),
                "node_id": node_id,
            })
        })
        .collect()
}

pub(crate) fn editor_routes_panel_json(
    origin_map: &orv_compiler::OriginMap,
    files: &[SourceFile],
) -> Vec<serde_json::Value> {
    origin_map
        .entries
        .iter()
        .filter(|entry| entry.kind == "route")
        .map(|entry| {
            let (method, path) = entry
                .name
                .split_once(' ')
                .unwrap_or((entry.name.as_str(), ""));
            serde_json::json!({
                "origin_id": entry.id,
                "method": method,
                "path": path,
                "name": entry.name,
                "location": editor_origin_location_json(entry.span, files),
            })
        })
        .collect()
}

pub(crate) fn editor_schema_panel_json(
    graph: &ProjectGraph,
    files: &[SourceFile],
) -> Vec<serde_json::Value> {
    graph
        .nodes
        .iter()
        .filter(|node| {
            matches!(
                node.kind,
                ProjectNodeKind::Struct | ProjectNodeKind::Enum | ProjectNodeKind::TypeAlias
            )
        })
        .map(|node| editor_project_node_panel_item(node, files))
        .collect()
}

pub(crate) fn editor_domains_panel_json(
    graph: &ProjectGraph,
    files: &[SourceFile],
) -> Vec<serde_json::Value> {
    graph
        .nodes
        .iter()
        .filter(|node| matches!(node.kind, ProjectNodeKind::Define | ProjectNodeKind::Domain))
        .map(|node| editor_project_node_panel_item(node, files))
        .collect()
}

pub(crate) fn editor_project_node_panel_item(
    node: &orv_project::ProjectNode,
    files: &[SourceFile],
) -> serde_json::Value {
    serde_json::json!({
        "node_id": node.id,
        "kind": node_kind(node.kind),
        "name": node.name,
        "location": lsp_location_json(node, files),
    })
}

pub(crate) fn editor_origin_location_json(
    span: orv_compiler::OriginSpan,
    files: &[SourceFile],
) -> serde_json::Value {
    let span = Span::new(FileId(span.file), ByteRange::new(span.start, span.end));
    let uri = files.iter().find(|file| file.id == span.file).map_or_else(
        || "file://<unknown>".to_string(),
        |file| lsp_file_uri_for_path(&file.path),
    );
    serde_json::json!({
        "uri": uri,
        "range": lsp_range_json(span, files),
    })
}

pub(crate) fn editor_reveal_json(dir: &Path, origin_id: &str) -> anyhow::Result<serde_json::Value> {
    let reveal = reveal_origin_json(dir, origin_id)?;
    let source = reveal
        .get("source")
        .ok_or_else(|| anyhow::anyhow!("reveal source missing"))?;
    let path = json_str(source, "path", "reveal source")?;
    let start = json_u32(source, "start", "reveal source")?;
    let end = json_u32(source, "end", "reveal source")?;
    let source_text = source
        .get("content")
        .and_then(serde_json::Value::as_str)
        .map(ToString::to_string)
        .map_or_else(
            || {
                std::fs::read_to_string(path)
                    .map_err(|e| anyhow::anyhow!("failed to read reveal source {path}: {e}"))
            },
            Ok,
        )?;
    let origin = reveal
        .get("origin")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let project_graph = reveal
        .get("project_graph")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let production = reveal
        .get("production")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    Ok(serde_json::json!({
        "schema_version": 1,
        "origin": origin,
        "focus": editor_reveal_focus_json(&origin, &project_graph, origin_id),
        "source": {
            "file": source.get("file").cloned().unwrap_or(serde_json::Value::Null),
            "path": path,
            "snippet": source.get("snippet").cloned().unwrap_or(serde_json::Value::Null),
            "location": {
                "uri": lsp_file_uri_for_path(Path::new(path)),
                "range": lsp_range_for_source(&source_text, start, end),
            },
        },
        "project_graph": project_graph,
        "production": production,
    }))
}

pub(crate) fn editor_reveal_focus_json(
    origin: &serde_json::Value,
    project_graph: &serde_json::Value,
    origin_id: &str,
) -> serde_json::Value {
    let origin_kind = origin
        .get("kind")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("");
    let panel = match origin_kind {
        "route" => "routes",
        "struct" | "enum" | "type_alias" => "schema",
        "define" | "domain" => "domains",
        _ => "source",
    };
    serde_json::json!({
        "origin_id": origin_id,
        "panel": panel,
        "node_id": project_graph.get("id").cloned().unwrap_or(serde_json::Value::Null),
    })
}

pub(crate) fn editor_runtime_json(path: &Path) -> anyhow::Result<serde_json::Value> {
    let loaded = orv_project::load_project(path).map_err(|e| anyhow::anyhow!("{e}"))?;
    let resolved = orv_resolve::resolve(&loaded.program);
    let lowered = orv_analyzer::lower_with_diagnostics(&loaded.program, &resolved);
    let diagnostic_count =
        loaded.diagnostics.len() + resolved.diagnostics.len() + lowered.diagnostics.len();
    let sources = editor_dap_sources(&loaded.files);
    let (runtime, frames, _live, long_running) =
        dap_launch_runtime_state(&lowered, diagnostic_count, &loaded.files, &sources, false);
    let async_runtime = dap_async_runtime_state(&lowered.program, long_running);
    Ok(serde_json::json!({
        "schema_version": 1,
        "entry": {
            "path": path.display().to_string(),
            "uri": lsp_file_uri_for_path(path),
        },
        "runtime": dap_runtime_json(&runtime, async_runtime.as_ref()),
        "frames": editor_runtime_frames_json(&frames),
        "panels": {
            "runtime": editor_runtime_panel_json(&runtime, async_runtime.as_ref(), &frames),
        },
    }))
}

pub(crate) fn editor_runtime_panel_json(
    runtime: &DapRuntimeState,
    async_runtime: Option<&DapAsyncRuntimeState>,
    frames: &[DapFrameState],
) -> serde_json::Value {
    serde_json::json!({
        "status": runtime.status,
        "stdout": runtime.stdout,
        "error": runtime.error,
        "frame_count": frames.len(),
        "async": async_runtime.map(editor_async_runtime_json),
    })
}

pub(crate) fn editor_async_runtime_json(runtime: &DapAsyncRuntimeState) -> serde_json::Value {
    serde_json::json!({
        "kind": runtime.kind,
        "state": runtime.state,
        "listen": runtime.listen.as_ref().map(dap_async_listen_json),
        "route_count": runtime.routes.len(),
        "routes": runtime.routes.iter().map(dap_async_route_json).collect::<Vec<_>>(),
    })
}

pub(crate) fn editor_runtime_frames_json(frames: &[DapFrameState]) -> Vec<serde_json::Value> {
    frames
        .iter()
        .enumerate()
        .map(|(index, frame)| {
            serde_json::json!({
                "index": index,
                "source": dap_source_json(&frame.source),
                "line": frame.line,
                "locals": frame.locals.iter().map(editor_runtime_variable_json).collect::<Vec<_>>(),
                "stack": frame.stack.iter().map(editor_runtime_stack_json).collect::<Vec<_>>(),
                "output": frame.output,
            })
        })
        .collect()
}

pub(crate) fn editor_runtime_variable_json(variable: &DapVariable) -> serde_json::Value {
    serde_json::json!({
        "name": variable.name,
        "value": variable.value,
        "type": variable.value_type,
        "line": variable.line,
    })
}

pub(crate) fn editor_runtime_stack_json(frame: &DapStackFrameState) -> serde_json::Value {
    serde_json::json!({
        "name": frame.name,
        "source": dap_source_json(&frame.source),
        "line": frame.line,
    })
}

pub(crate) fn add_editor_source_bundle_contract_fields(
    artifact: &serde_json::Value,
    target: &mut serde_json::Value,
) {
    target["schema_version"] = artifact
        .get("schema_version")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    target["entry"] = artifact
        .get("entry")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let files = artifact
        .get("files")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    target["file_count"] = serde_json::json!(files.len());
    target["files"] = serde_json::Value::Array(
        files
            .iter()
            .map(|file| {
                serde_json::json!({
                    "path": file.get("path").cloned().unwrap_or(serde_json::Value::Null),
                    "content_hash": file
                        .get("content_hash")
                        .cloned()
                        .unwrap_or(serde_json::Value::Null),
                })
            })
            .collect(),
    );
}

pub(crate) fn add_editor_project_graph_contract_fields(
    artifact: &serde_json::Value,
    target: &mut serde_json::Value,
) {
    target["schema_version"] = artifact
        .get("schema_version")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    target["stats"] = artifact
        .get("stats")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));
    target["node_count"] = serde_json::json!(json_array_count(artifact.get("nodes")));
    target["edge_count"] = serde_json::json!(json_array_count(artifact.get("edges")));
    target["semantic_origin_count"] = serde_json::json!(json_array_count(
        artifact.pointer("/semantic/origin_map/entries")
    ));
    target["semantic_edge_count"] =
        serde_json::json!(json_array_count(artifact.pointer("/semantic/origin_edges")));
    target["semantic_origin_link_count"] =
        serde_json::json!(json_array_count(artifact.pointer("/semantic/origin_links")));
}

pub(crate) fn add_editor_origin_map_contract_fields(
    artifact: &serde_json::Value,
    target: &mut serde_json::Value,
) {
    target["version"] = artifact
        .get("version")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    target["entry_count"] = serde_json::json!(json_array_count(artifact.get("entries")));
    target["edge_count"] = serde_json::json!(json_array_count(artifact.get("edges")));
    let call_edges = artifact
        .get("edges")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter(|edge| edge.get("kind").and_then(serde_json::Value::as_str) == Some("calls"))
        .count();
    target["call_edge_count"] = serde_json::json!(call_edges);
}

pub(crate) fn editor_runtime_panel_artifact_json() -> serde_json::Value {
    serde_json::json!({
        "schema_version": 1,
        "kind": "orv.editor.runtime.panel",
        "path": EDITOR_RUNTIME_PANEL_HTML_PATH,
        "media_type": "text/html",
        "source": "native-host.runtime",
        "panel_contract": editor_native_host_runtime_panel_contract_json(),
    })
}

pub(crate) fn editor_reveal_command_json(
    build_dir: &str,
    origin_id: Option<&str>,
) -> serde_json::Value {
    let Some(origin_id) = origin_id else {
        return serde_json::Value::Null;
    };
    if build_dir.is_empty() {
        return serde_json::Value::Null;
    }
    serde_json::json!(["orv", "editor", "reveal", build_dir, origin_id])
}

pub(crate) fn json_usize_field(value: &serde_json::Value, key: &str) -> usize {
    value
        .get(key)
        .and_then(serde_json::Value::as_u64)
        .and_then(|count| usize::try_from(count).ok())
        .unwrap_or(0)
}

pub(crate) fn json_u64_field(value: &serde_json::Value, key: &str) -> u64 {
    value
        .get(key)
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0)
}

pub(crate) fn json_str_or_empty<'a>(value: &'a serde_json::Value, key: &str) -> &'a str {
    value
        .get(key)
        .and_then(serde_json::Value::as_str)
        .unwrap_or("")
}

pub(crate) fn json_array_count(value: Option<&serde_json::Value>) -> usize {
    value
        .and_then(serde_json::Value::as_array)
        .map_or(0, Vec::len)
}

pub(crate) fn json_object_count(value: Option<&serde_json::Value>) -> usize {
    value
        .and_then(serde_json::Value::as_object)
        .map_or(0, serde_json::Map::len)
}
