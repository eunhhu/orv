#![allow(clippy::redundant_pub_crate, clippy::wildcard_imports)]

use super::*;

pub(crate) fn cmd_editor_debug(
    path: &Path,
    controls: &[EditorDebugControl],
    breakpoints: &[EditorDebugBreakpoint],
    function_breakpoints: &[String],
    data_breakpoints: &[String],
    exception_filters: &[String],
    watch_expressions: &[String],
) -> anyhow::Result<()> {
    let entry = project_entry_path(path)?;
    let value = editor_debug_session_json(
        &entry,
        controls,
        breakpoints,
        function_breakpoints,
        data_breakpoints,
        exception_filters,
        watch_expressions,
    )?;
    println!("{}", serde_json::to_string_pretty(&value)?);
    Ok(())
}

pub(crate) fn cmd_editor_run_debug(
    state: &Path,
    controls: &[EditorDebugControl],
    breakpoints: &[EditorDebugBreakpoint],
    function_breakpoints: &[String],
    data_breakpoints: &[String],
    exception_filters: &[String],
    watch_expressions: &[String],
) -> anyhow::Result<()> {
    let value = editor_debug_runner_session_json(
        state,
        controls,
        breakpoints,
        function_breakpoints,
        data_breakpoints,
        exception_filters,
        watch_expressions,
    )?;
    write_editor_debug_runner_result_if_configured(state, &value)?;
    write_editor_debug_runner_result_html_if_configured(state, &value)?;
    println!("{}", serde_json::to_string_pretty(&value)?);
    Ok(())
}

pub(crate) fn editor_debug_json(path: &Path) -> anyhow::Result<serde_json::Value> {
    let loaded = orv_project::load_project(path).map_err(|e| anyhow::anyhow!("{e}"))?;
    Ok(serde_json::json!({
        "schema_version": 1,
        "adapter": editor_debug_adapter_json(),
        "capabilities": editor_debug_capabilities_json(),
        "session_runner": editor_debug_session_runner_json(path),
        "result_artifact": editor_debug_result_artifact_json(),
        "configurations": editor_debug_configurations_json(path),
        "source_inventory": editor_debug_source_inventory_json(&loaded.files),
        "controls": editor_debug_controls_json(),
        "breakpoint_sources": editor_debug_breakpoint_sources_json(&loaded.files),
        "function_breakpoints": editor_debug_function_breakpoints_json(&loaded),
        "data_breakpoints": editor_debug_data_breakpoints_json(&loaded),
        "exception_filters": editor_debug_exception_filters_json(),
    }))
}

pub(crate) fn editor_debug_session_json(
    path: &Path,
    controls: &[EditorDebugControl],
    breakpoints: &[EditorDebugBreakpoint],
    function_breakpoints: &[String],
    data_breakpoints: &[String],
    exception_filters: &[String],
    watch_expressions: &[String],
) -> anyhow::Result<serde_json::Value> {
    editor_debug_session_json_with_source_bundle(EditorDebugSessionInput {
        path,
        controls,
        breakpoints,
        function_breakpoints,
        data_breakpoints,
        exception_filters,
        watch_expressions,
        source_bundle_path: None,
    })
}

pub(crate) struct EditorDebugSessionInput<'a> {
    pub(crate) path: &'a Path,
    pub(crate) controls: &'a [EditorDebugControl],
    pub(crate) breakpoints: &'a [EditorDebugBreakpoint],
    pub(crate) function_breakpoints: &'a [String],
    pub(crate) data_breakpoints: &'a [String],
    pub(crate) exception_filters: &'a [String],
    pub(crate) watch_expressions: &'a [String],
    pub(crate) source_bundle_path: Option<&'a Path>,
}

pub(crate) fn editor_debug_session_json_with_source_bundle(
    input: EditorDebugSessionInput<'_>,
) -> anyhow::Result<serde_json::Value> {
    let EditorDebugSessionInput {
        path,
        controls,
        breakpoints,
        function_breakpoints,
        data_breakpoints,
        exception_filters,
        watch_expressions,
        source_bundle_path,
    } = input;
    let loaded = if let Some(source_bundle_path) = source_bundle_path {
        let source_bundle = read_source_bundle_artifact(source_bundle_path)?;
        load_project_from_source_bundle_artifact(&source_bundle)?
    } else {
        orv_project::load_project(path).map_err(|e| anyhow::anyhow!("{e}"))?
    };
    let sources = editor_dap_sources(&loaded.files);
    let controls = if controls.is_empty() {
        vec![EditorDebugControl::Next]
    } else {
        controls.to_vec()
    };
    let mut requests = vec![serde_json::json!({
        "seq": 1,
        "type": "request",
        "command": "initialize",
        "arguments": {},
    })];
    let mut next_seq = 2_u64;
    let exception_filter_requests = editor_debug_push_exception_filter_requests(
        &mut requests,
        &mut next_seq,
        exception_filters,
    );
    let function_breakpoint_requests = editor_debug_push_function_breakpoint_requests(
        &mut requests,
        &mut next_seq,
        function_breakpoints,
    );
    let launch_seq = next_seq;
    next_seq += 1;
    let mut launch_arguments = serde_json::json!({
        "program": format!("file://{}", path.display()),
        "live": true,
    });
    if let Some(source_bundle_path) = source_bundle_path {
        launch_arguments["sourceBundle"] =
            serde_json::json!(source_bundle_path.display().to_string());
    }
    requests.push(serde_json::json!({
        "seq": launch_seq,
        "type": "request",
        "command": "launch",
        "arguments": launch_arguments,
    }));
    let loaded_sources_seq = next_seq;
    next_seq += 1;
    requests.push(editor_debug_loaded_sources_request_json(loaded_sources_seq));
    let source_requests = editor_debug_push_source_requests(&mut requests, &mut next_seq, &sources);
    let breakpoint_requests =
        editor_debug_push_breakpoint_requests(&mut requests, &mut next_seq, breakpoints);
    let (data_breakpoint_info_requests, data_breakpoint_set_request) =
        editor_debug_push_data_breakpoint_requests(&mut requests, &mut next_seq, data_breakpoints);
    let control_requests =
        editor_debug_push_control_requests(&mut requests, &mut next_seq, &controls);
    let stack_seq = next_seq;
    requests.push(serde_json::json!({
        "seq": stack_seq,
        "type": "request",
        "command": "stackTrace",
        "arguments": {
            "threadId": 1,
        },
    }));
    let scopes_seq = next_seq + 1;
    requests.push(serde_json::json!({
        "seq": scopes_seq,
        "type": "request",
        "command": "scopes",
        "arguments": {
            "frameId": 1,
        },
    }));
    let project_variables_seq = next_seq + 2;
    requests.push(serde_json::json!({
        "seq": project_variables_seq,
        "type": "request",
        "command": "variables",
        "arguments": {
            "variablesReference": 1,
        },
    }));
    let locals_variables_seq = next_seq + 3;
    requests.push(serde_json::json!({
        "seq": locals_variables_seq,
        "type": "request",
        "command": "variables",
        "arguments": {
            "variablesReference": 2,
        },
    }));
    let mut next_inspection_seq = next_seq + 4;
    let watch_expression_requests = editor_debug_push_watch_expression_requests(
        &mut requests,
        &mut next_inspection_seq,
        watch_expressions,
    );
    let input = dap_protocol_input_frames(&requests)?;
    let mut reader = std::io::Cursor::new(input.as_bytes());
    let mut writer = Vec::new();
    dap_serve_stdio_stream(&mut reader, &mut writer)?;
    let output =
        String::from_utf8(writer).map_err(|e| anyhow::anyhow!("invalid DAP output: {e}"))?;
    let frames = dap_protocol_output_frames(&output)?;
    let breakpoint_summaries = editor_debug_breakpoint_summaries(&frames, breakpoint_requests);
    let function_breakpoint_summaries =
        editor_debug_function_breakpoint_summaries(&frames, function_breakpoint_requests);
    let data_breakpoint_summaries = editor_debug_data_breakpoint_summaries(
        &frames,
        data_breakpoint_info_requests,
        data_breakpoint_set_request,
    );
    let exception_filter_summaries =
        editor_debug_exception_filter_summaries(&frames, exception_filter_requests);
    let control_summaries = editor_debug_control_summaries(&frames, control_requests);
    let watch_expression_summaries =
        editor_debug_watch_expression_summaries(&frames, watch_expression_requests);
    let launch =
        dap_response_for_request_seq(&frames, launch_seq).unwrap_or_else(|| serde_json::json!({}));
    let loaded_sources = dap_response_for_request_seq(&frames, loaded_sources_seq)
        .and_then(|response| response.get("body").cloned())
        .unwrap_or_else(|| serde_json::json!({}));
    let source_snapshot_summaries =
        editor_debug_source_snapshot_summaries(&frames, source_requests);
    let stack = dap_response_for_request_seq(&frames, stack_seq)
        .and_then(|response| response.get("body").cloned())
        .unwrap_or_else(|| serde_json::json!({}));
    let scopes = dap_response_for_request_seq(&frames, scopes_seq)
        .and_then(|response| response.get("body").cloned())
        .unwrap_or_else(|| serde_json::json!({}));
    let project_variables = dap_response_for_request_seq(&frames, project_variables_seq)
        .and_then(|response| response.pointer("/body/variables").cloned())
        .unwrap_or_else(|| serde_json::json!([]));
    let locals = dap_response_for_request_seq(&frames, locals_variables_seq)
        .and_then(|response| response.pointer("/body/variables").cloned())
        .unwrap_or_else(|| serde_json::json!([]));
    let first_control = control_summaries
        .first()
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));
    Ok(serde_json::json!({
        "schema_version": 1,
        "kind": "orv.editor.debug",
        "program": path.display().to_string(),
        "adapter": editor_debug_adapter_json(),
        "transport": {
            "protocol": "dap",
            "framing": "content-length",
            "request_count": requests.len(),
            "frame_count": frames.len(),
        },
        "breakpoints": breakpoint_summaries,
        "function_breakpoints": function_breakpoint_summaries,
        "data_breakpoints": data_breakpoint_summaries,
        "exception_filters": exception_filter_summaries,
        "launch": launch,
        "loaded_sources": loaded_sources,
        "source_snapshots": source_snapshot_summaries,
        "control": first_control,
        "controls": control_summaries,
        "watch_expressions": watch_expression_summaries,
        "stack": stack,
        "scopes": scopes,
        "project_variables": project_variables,
        "locals": locals,
        "frames": frames,
    }))
}

pub(crate) fn editor_debug_runner_session_json(
    state_path: &Path,
    controls: &[EditorDebugControl],
    breakpoints: &[EditorDebugBreakpoint],
    function_breakpoints: &[String],
    data_breakpoints: &[String],
    exception_filters: &[String],
    watch_expressions: &[String],
) -> anyhow::Result<serde_json::Value> {
    let runner = if state_path.is_dir() {
        editor_debug_runner_from_build_dir(state_path)?
    } else {
        let state = read_json_value(state_path)?;
        match state.get("kind").and_then(serde_json::Value::as_str) {
            Some("orv.editor.export") => {
                if state
                    .get("schema_version")
                    .and_then(serde_json::Value::as_u64)
                    != Some(1)
                {
                    anyhow::bail!("editor export state schema_version must be 1");
                }
                verify_editor_export_state_contract_keys(&state)?;
                state
                    .pointer("/debug/session_runner")
                    .cloned()
                    .ok_or_else(|| {
                        anyhow::anyhow!("editor export state missing debug.session_runner")
                    })?
            }
        Some("orv.editor.debug.runner") => state.clone(),
        _ => anyhow::bail!(
            "editor debug runner input must be a build dir, orv.editor.export state, or orv.editor.debug.runner artifact"
        ),
        }
    };
    if runner.get("kind").and_then(serde_json::Value::as_str) != Some("orv.editor.debug.runner") {
        anyhow::bail!("editor debug runner kind is invalid");
    }
    if runner
        .get("schema_version")
        .and_then(serde_json::Value::as_u64)
        != Some(1)
    {
        anyhow::bail!("editor debug runner schema_version must be 1");
    }
    verify_editor_debug_runner_contract_keys(&runner)?;
    let program = json_str(&runner, "program", "editor debug runner")?;
    let source_bundle = runner
        .get("source_bundle")
        .and_then(serde_json::Value::as_str)
        .filter(|path| !path.trim().is_empty())
        .map(PathBuf::from);
    let debug = editor_debug_session_json_with_source_bundle(EditorDebugSessionInput {
        path: Path::new(program),
        controls,
        breakpoints,
        function_breakpoints,
        data_breakpoints,
        exception_filters,
        watch_expressions,
        source_bundle_path: source_bundle.as_deref(),
    })?;
    Ok(serde_json::json!({
        "schema_version": 1,
        "kind": "orv.editor.debug.runner.result",
        "state": state_path.display().to_string(),
        "runner": runner,
        "production_context": runner
            .get("production_context")
            .cloned()
            .unwrap_or(serde_json::Value::Null),
        "debug": debug,
        "panels": editor_debug_runner_result_panels_json(&runner, &debug),
    }))
}

pub(crate) fn verify_editor_export_debug_contract_keys(
    debug: &serde_json::Value,
) -> anyhow::Result<()> {
    verify_json_object_keys_allowing_optional(
        debug,
        &[
            "schema_version",
            "adapter",
            "capabilities",
            "session_runner",
            "result_artifact",
            "configurations",
            "source_inventory",
            "controls",
            "breakpoint_sources",
            "function_breakpoints",
            "data_breakpoints",
            "exception_filters",
        ],
        &["production_context"],
        "editor export debug",
    )?;
    if debug
        .get("schema_version")
        .and_then(serde_json::Value::as_u64)
        != Some(1)
    {
        anyhow::bail!("editor export debug schema_version must be 1");
    }
    if debug.get("adapter") != Some(&editor_debug_adapter_json()) {
        anyhow::bail!("editor export debug adapter must match generated contract");
    }
    if debug.get("capabilities") != Some(&editor_debug_capabilities_json()) {
        anyhow::bail!("editor export debug capabilities must match generated contract");
    }
    let expected_controls = serde_json::json!(editor_debug_controls_json());
    if debug.get("controls") != Some(&expected_controls) {
        anyhow::bail!("editor export debug controls must match generated contract");
    }
    verify_editor_export_debug_configurations(debug)?;
    verify_editor_export_debug_source_inventory_contract_keys(
        debug.get("source_inventory").ok_or_else(|| {
            anyhow::anyhow!("editor export debug.source_inventory must be an object")
        })?,
    )?;
    verify_editor_export_debug_breakpoint_sources_contract_keys(
        debug.get("breakpoint_sources").ok_or_else(|| {
            anyhow::anyhow!("editor export debug.breakpoint_sources must be an array")
        })?,
    )?;
    verify_editor_export_debug_function_breakpoints_contract_keys(
        debug.get("function_breakpoints").ok_or_else(|| {
            anyhow::anyhow!("editor export debug.function_breakpoints must be an array")
        })?,
    )?;
    verify_editor_export_debug_data_breakpoints_contract_keys(
        debug.get("data_breakpoints").ok_or_else(|| {
            anyhow::anyhow!("editor export debug.data_breakpoints must be an array")
        })?,
    )?;
    verify_editor_export_debug_exception_filters_contract_keys(
        debug.get("exception_filters").ok_or_else(|| {
            anyhow::anyhow!("editor export debug.exception_filters must be an array")
        })?,
    )?;
    verify_editor_debug_runner_contract_keys(
        debug.get("session_runner").ok_or_else(|| {
            anyhow::anyhow!("editor export debug.session_runner must be an object")
        })?,
    )?;
    verify_editor_debug_result_artifact_contract_keys(debug.get("result_artifact").ok_or_else(
        || anyhow::anyhow!("editor export debug.result_artifact must be an object"),
    )?)?;
    if let Some(production_context) = debug
        .get("production_context")
        .filter(|value| !value.is_null())
    {
        verify_editor_debug_production_context_contract_keys(production_context)?;
        if debug
            .pointer("/session_runner/production_context")
            .is_some_and(|runner_context| runner_context != production_context)
        {
            anyhow::bail!(
                "editor export debug production_context must match session runner production_context"
            );
        }
    }
    Ok(())
}

pub(crate) fn verify_editor_export_debug_configurations(
    debug: &serde_json::Value,
) -> anyhow::Result<()> {
    let program = debug
        .pointer("/session_runner/program")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| {
            anyhow::anyhow!("editor export debug.session_runner.program must be a string")
        })?;
    if debug.get("configurations")
        != Some(&serde_json::json!(editor_debug_configurations_json(
            Path::new(program)
        )))
    {
        anyhow::bail!("editor export debug configurations must match generated contract");
    }
    Ok(())
}

pub(crate) fn verify_editor_export_debug_source_inventory_contract_keys(
    inventory: &serde_json::Value,
) -> anyhow::Result<()> {
    verify_json_object_keys_exact(
        inventory,
        &[
            "schema_version",
            "kind",
            "protocol",
            "source_count",
            "loaded_sources_request",
            "sources",
        ],
        "editor export debug source_inventory",
    )?;
    if inventory.get("loaded_sources_request") != Some(&editor_debug_loaded_sources_request_json(0))
    {
        anyhow::bail!(
            "editor export debug source_inventory loaded_sources_request must match generated contract"
        );
    }
    let sources = inventory
        .get("sources")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| {
            anyhow::anyhow!("editor export debug source_inventory.sources must be an array")
        })?;
    if inventory
        .get("source_count")
        .and_then(serde_json::Value::as_u64)
        != Some(u64::try_from(sources.len()).unwrap_or(u64::MAX))
    {
        anyhow::bail!("editor export debug source_inventory source_count must match sources");
    }
    for (index, source) in sources.iter().enumerate() {
        verify_json_object_keys_exact(
            source,
            &[
                "source",
                "source_reference",
                "path",
                "uri",
                "checksum",
                "request",
            ],
            &format!("editor export debug source_inventory.sources[{index}]"),
        )?;
        verify_editor_debug_dap_source_contract_keys(
            source
                .get("source")
                .ok_or_else(|| anyhow::anyhow!("editor export debug source_inventory.sources[{index}].source must be an object"))?,
            &format!("editor export debug source_inventory.sources[{index}].source"),
            false,
        )?;
        verify_json_object_keys_exact(
            source
                .get("checksum")
                .ok_or_else(|| anyhow::anyhow!("editor export debug source_inventory.sources[{index}].checksum must be an object"))?,
            &["algorithm", "value"],
            &format!("editor export debug source_inventory.sources[{index}].checksum"),
        )?;
        verify_editor_debug_dap_request_contract_keys(
            source
                .get("request")
                .ok_or_else(|| anyhow::anyhow!("editor export debug source_inventory.sources[{index}].request must be an object"))?,
            &format!("editor export debug source_inventory.sources[{index}].request"),
        )?;
        verify_editor_export_debug_source_inventory_entry_consistency(source, index)?;
    }
    Ok(())
}

pub(crate) fn verify_editor_export_debug_source_inventory_entry_consistency(
    entry: &serde_json::Value,
    index: usize,
) -> anyhow::Result<()> {
    let context = format!("editor export debug source_inventory.sources[{index}]");
    let source = entry
        .get("source")
        .ok_or_else(|| anyhow::anyhow!("{context}.source must be an object"))?;
    let source_reference = entry
        .get("source_reference")
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| anyhow::anyhow!("{context}.source_reference must be a number"))?;
    if source
        .get("sourceReference")
        .and_then(serde_json::Value::as_u64)
        != Some(source_reference)
    {
        anyhow::bail!("{context} source_reference must match DAP source");
    }
    for field in ["path", "uri"] {
        if entry.get(field) != source.get(field) {
            anyhow::bail!("{context} {field} must match DAP source");
        }
    }
    let checksum = entry
        .get("checksum")
        .ok_or_else(|| anyhow::anyhow!("{context}.checksum must be an object"))?;
    let source_checksum = source
        .get("checksums")
        .and_then(serde_json::Value::as_array)
        .and_then(|checksums| checksums.first())
        .ok_or_else(|| anyhow::anyhow!("{context}.source.checksums must not be empty"))?;
    if checksum.get("algorithm") != source_checksum.get("algorithm")
        || checksum.get("value") != source_checksum.get("checksum")
    {
        anyhow::bail!("{context} checksum must match DAP source");
    }
    let request = entry
        .get("request")
        .ok_or_else(|| anyhow::anyhow!("{context}.request must be an object"))?;
    if request
        .pointer("/arguments/sourceReference")
        .and_then(serde_json::Value::as_u64)
        != Some(source_reference)
    {
        anyhow::bail!("{context} request sourceReference must match DAP source");
    }
    if request.pointer("/arguments/source") != Some(source) {
        anyhow::bail!("{context} request source must match DAP source");
    }
    Ok(())
}

pub(crate) fn verify_editor_export_debug_breakpoint_sources_contract_keys(
    sources: &serde_json::Value,
) -> anyhow::Result<()> {
    let sources = sources.as_array().ok_or_else(|| {
        anyhow::anyhow!("editor export debug breakpoint_sources must be an array")
    })?;
    for (source_index, source) in sources.iter().enumerate() {
        verify_json_object_keys_exact(
            source,
            &["source", "line_count", "lines", "breakpoints"],
            &format!("editor export debug breakpoint_sources[{source_index}]"),
        )?;
        verify_editor_debug_dap_source_contract_keys(
            source
                .get("source")
                .ok_or_else(|| anyhow::anyhow!("editor export debug breakpoint_sources[{source_index}].source must be an object"))?,
            &format!("editor export debug breakpoint_sources[{source_index}].source"),
            false,
        )?;
        let lines = source
            .get("lines")
            .and_then(serde_json::Value::as_array)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "editor export debug breakpoint_sources[{source_index}].lines must be an array"
                )
            })?;
        let breakpoints = source
            .get("breakpoints")
            .and_then(serde_json::Value::as_array)
            .ok_or_else(|| anyhow::anyhow!("editor export debug breakpoint_sources[{source_index}].breakpoints must be an array"))?;
        if source.get("line_count").and_then(serde_json::Value::as_u64)
            != Some(u64::try_from(lines.len()).unwrap_or(u64::MAX))
            || lines.len() != breakpoints.len()
        {
            anyhow::bail!(
                "editor export debug breakpoint_sources[{source_index}] line_count must match breakpoints"
            );
        }
        for (breakpoint_index, breakpoint) in breakpoints.iter().enumerate() {
            verify_json_object_keys_exact(
                breakpoint,
                &["line", "request", "runner_command"],
                &format!(
                    "editor export debug breakpoint_sources[{source_index}].breakpoints[{breakpoint_index}]"
                ),
            )?;
            verify_editor_debug_dap_request_contract_keys(
                breakpoint.get("request").ok_or_else(|| anyhow::anyhow!(
                    "editor export debug breakpoint_sources[{source_index}].breakpoints[{breakpoint_index}].request must be an object"
                ))?,
                &format!("editor export debug breakpoint_sources[{source_index}].breakpoints[{breakpoint_index}].request"),
            )?;
        }
    }
    Ok(())
}

pub(crate) fn verify_editor_export_debug_function_breakpoints_contract_keys(
    breakpoints: &serde_json::Value,
) -> anyhow::Result<()> {
    let breakpoints = breakpoints.as_array().ok_or_else(|| {
        anyhow::anyhow!("editor export debug function_breakpoints must be an array")
    })?;
    for (index, breakpoint) in breakpoints.iter().enumerate() {
        verify_json_object_keys_exact(
            breakpoint,
            &["name", "kind", "source", "request", "runner_command"],
            &format!("editor export debug function_breakpoints[{index}]"),
        )?;
        verify_editor_debug_source_location_contract_keys(
            breakpoint.get("source").ok_or_else(|| {
                anyhow::anyhow!(
                    "editor export debug function_breakpoints[{index}].source must be an object"
                )
            })?,
            &format!("editor export debug function_breakpoints[{index}].source"),
        )?;
        verify_editor_debug_dap_request_contract_keys(
            breakpoint.get("request").ok_or_else(|| {
                anyhow::anyhow!(
                    "editor export debug function_breakpoints[{index}].request must be an object"
                )
            })?,
            &format!("editor export debug function_breakpoints[{index}].request"),
        )?;
    }
    Ok(())
}

pub(crate) fn verify_editor_export_debug_data_breakpoints_contract_keys(
    breakpoints: &serde_json::Value,
) -> anyhow::Result<()> {
    let breakpoints = breakpoints
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("editor export debug data_breakpoints must be an array"))?;
    for (index, breakpoint) in breakpoints.iter().enumerate() {
        verify_json_object_keys_exact(
            breakpoint,
            &[
                "name",
                "data_id",
                "value",
                "type",
                "source",
                "info_request",
                "request",
                "runner_command",
            ],
            &format!("editor export debug data_breakpoints[{index}]"),
        )?;
        verify_editor_debug_dap_source_contract_keys(
            breakpoint.get("source").ok_or_else(|| {
                anyhow::anyhow!(
                    "editor export debug data_breakpoints[{index}].source must be an object"
                )
            })?,
            &format!("editor export debug data_breakpoints[{index}].source"),
            true,
        )?;
        verify_editor_debug_dap_request_contract_keys(
            breakpoint.get("info_request").ok_or_else(|| {
                anyhow::anyhow!(
                    "editor export debug data_breakpoints[{index}].info_request must be an object"
                )
            })?,
            &format!("editor export debug data_breakpoints[{index}].info_request"),
        )?;
        verify_editor_debug_dap_request_contract_keys(
            breakpoint.get("request").ok_or_else(|| {
                anyhow::anyhow!(
                    "editor export debug data_breakpoints[{index}].request must be an object"
                )
            })?,
            &format!("editor export debug data_breakpoints[{index}].request"),
        )?;
    }
    Ok(())
}

pub(crate) fn verify_editor_export_debug_exception_filters_contract_keys(
    filters: &serde_json::Value,
) -> anyhow::Result<()> {
    let filters = filters
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("editor export debug exception_filters must be an array"))?;
    for (index, filter) in filters.iter().enumerate() {
        verify_json_object_keys_exact(
            filter,
            &["filter", "label", "default", "request", "runner_command"],
            &format!("editor export debug exception_filters[{index}]"),
        )?;
        verify_editor_debug_dap_request_contract_keys(
            filter.get("request").ok_or_else(|| {
                anyhow::anyhow!(
                    "editor export debug exception_filters[{index}].request must be an object"
                )
            })?,
            &format!("editor export debug exception_filters[{index}].request"),
        )?;
    }
    Ok(())
}

pub(crate) fn verify_editor_debug_source_location_contract_keys(
    source: &serde_json::Value,
    context: &str,
) -> anyhow::Result<()> {
    verify_json_object_keys_exact(source, &["path", "line"], context)
}

pub(crate) fn verify_editor_debug_runner_contract_keys(
    runner: &serde_json::Value,
) -> anyhow::Result<()> {
    let has_export_runner_keys = ["transport", "command", "session", "controls"]
        .iter()
        .any(|key| runner.get(*key).is_some());
    let has_production_keys =
        runner.get("source_bundle").is_some() || runner.get("production_context").is_some();
    let expected_root_keys = match (has_export_runner_keys, has_production_keys) {
        (true, true) => &[
            "schema_version",
            "kind",
            "program",
            "transport",
            "command",
            "result",
            "session",
            "controls",
            "source_bundle",
            "production_context",
        ][..],
        (true, false) => &[
            "schema_version",
            "kind",
            "program",
            "transport",
            "command",
            "result",
            "session",
            "controls",
        ][..],
        (false, true) => &[
            "schema_version",
            "kind",
            "program",
            "source_bundle",
            "production_context",
            "result",
        ][..],
        (false, false) => &["schema_version", "kind", "program", "result"][..],
    };
    verify_json_object_keys_exact(runner, expected_root_keys, "editor debug runner")?;
    if has_export_runner_keys {
        verify_json_object_keys_exact(
            runner.get("transport").ok_or_else(|| {
                anyhow::anyhow!("editor debug runner transport must be an object")
            })?,
            &["protocol", "framing"],
            "editor debug runner transport",
        )?;
        let expected_transport = editor_debug_runner_transport_json();
        if runner.get("transport") != Some(&expected_transport) {
            anyhow::bail!("editor debug runner transport must match generated contract");
        }
        if runner.get("command")
            != Some(&editor_debug_control_runner_command(
                EditorDebugControl::Next,
            ))
        {
            anyhow::bail!("editor debug runner command must match generated contract");
        }
        verify_json_object_keys_exact(
            runner
                .get("session")
                .ok_or_else(|| anyhow::anyhow!("editor debug runner session must be an object"))?,
            &[
                "launch",
                "thread_id",
                "breakpoint_argument",
                "breakpoint_format",
                "function_breakpoint_argument",
                "function_breakpoint_format",
                "data_breakpoint_argument",
                "data_breakpoint_format",
                "exception_filter_argument",
                "exception_filter_format",
                "watch_expression_argument",
                "watch_expression_format",
                "reuse_session",
            ],
            "editor debug runner session",
        )?;
        verify_json_object_keys_exact(
            runner.pointer("/session/launch").ok_or_else(|| {
                anyhow::anyhow!("editor debug runner session.launch must be an object")
            })?,
            &["live"],
            "editor debug runner session.launch",
        )?;
        let expected_session = editor_debug_runner_session_contract_json();
        if runner.get("session") != Some(&expected_session) {
            anyhow::bail!("editor debug runner session must match generated contract");
        }
        let controls = runner
            .get("controls")
            .and_then(serde_json::Value::as_array)
            .ok_or_else(|| anyhow::anyhow!("editor debug runner controls must be an array"))?;
        for (index, control) in controls.iter().enumerate() {
            verify_json_object_keys_exact(
                control,
                &["name", "value", "command", "request"],
                &format!("editor debug runner controls[{index}]"),
            )?;
        }
        let expected_controls = serde_json::json!(editor_debug_session_runner_controls_json());
        if runner.get("controls") != Some(&expected_controls) {
            anyhow::bail!("editor debug runner controls must match generated contract");
        }
    }
    verify_editor_debug_result_artifact_contract_keys(
        runner
            .get("result")
            .ok_or_else(|| anyhow::anyhow!("editor debug runner result must be an object"))?,
    )?;
    if let Some(production_context) = runner
        .get("production_context")
        .filter(|value| !value.is_null())
    {
        verify_editor_debug_production_context_contract_keys(production_context)?;
    }
    Ok(())
}

pub(crate) fn verify_editor_debug_result_artifact_contract_keys(
    result: &serde_json::Value,
) -> anyhow::Result<()> {
    verify_json_object_keys_exact(
        result,
        &[
            "path",
            "html_path",
            "kind",
            "media_type",
            "panels",
            "panel_contract",
        ],
        "editor debug runner result artifact",
    )?;
    verify_json_object_keys_exact(
        result.get("panel_contract").ok_or_else(|| {
            anyhow::anyhow!("editor debug runner panel_contract must be an object")
        })?,
        &["schema_version", "root", "sections"],
        "editor debug runner panel_contract",
    )?;
    let sections = result
        .pointer("/panel_contract/sections")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| {
            anyhow::anyhow!("editor debug runner panel_contract.sections must be an array")
        })?;
    for (index, section) in sections.iter().enumerate() {
        verify_json_object_keys_exact(
            section,
            &["name", "path", "kind"],
            &format!("editor debug runner panel_contract.sections[{index}]"),
        )?;
    }
    if result != &editor_debug_result_artifact_json() {
        anyhow::bail!("editor debug runner result artifact must match generated contract");
    }
    Ok(())
}

pub(crate) fn editor_debug_runner_from_build_dir(
    build_dir: &Path,
) -> anyhow::Result<serde_json::Value> {
    let source_bundle_path = build_dir.join(SOURCE_BUNDLE_PATH);
    let source_bundle = read_source_bundle_artifact(&source_bundle_path)?;
    let entry = source_bundle_entry_path(&source_bundle)?;
    let production = editor_production_summary_json(build_dir)?;
    Ok(serde_json::json!({
        "schema_version": 1,
        "kind": "orv.editor.debug.runner",
        "program": entry.display().to_string(),
        "source_bundle": source_bundle_path.display().to_string(),
        "production_context": editor_debug_production_context_json(&production),
        "result": editor_debug_result_artifact_json(),
    }))
}

pub(crate) fn editor_debug_runner_result_panels_json(
    runner: &serde_json::Value,
    debug: &serde_json::Value,
) -> serde_json::Value {
    let stack_frames = debug
        .pointer("/stack/stackFrames")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    let selected_frame = stack_frames
        .first()
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let stopped_events = editor_debug_event_frames(debug, "stopped");
    let output_events = editor_debug_event_frames(debug, "output");
    let events = editor_debug_all_event_frames(debug);
    let controls = debug
        .get("controls")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    let breakpoints = debug
        .get("breakpoints")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    let function_breakpoints = debug
        .get("function_breakpoints")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    let data_breakpoints = debug
        .get("data_breakpoints")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    let exception_filters = debug
        .get("exception_filters")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    let watch_expressions = debug
        .get("watch_expressions")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    let loaded_sources = debug
        .get("loaded_sources")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));
    let source_snapshots = debug
        .get("source_snapshots")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    let production_context = runner
        .get("production_context")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let production_summary = editor_debug_production_summary_from_context(&production_context);
    let source_bundle = editor_debug_launch_source_bundle(debug);
    let session_summary = editor_debug_session_summary_json(
        debug,
        &selected_frame,
        &events,
        &stopped_events,
        &output_events,
    );
    let source_navigation = editor_debug_source_navigation_json(&selected_frame, &stack_frames);
    serde_json::json!({
        "debug": {
            "schema_version": 1,
            "production_context": production_context,
            "production_summary": production_summary,
            "session_summary": session_summary,
            "source_bundle": source_bundle,
            "result_artifact": runner
                .get("result")
                .cloned()
                .unwrap_or_else(|| serde_json::json!({
                    "path": EDITOR_DEBUG_SESSION_RESULT_PATH,
                    "kind": "orv.editor.debug.runner.result",
                    "media_type": "application/json",
                })),
            "selected_frame": selected_frame,
            "stack_frames": stack_frames,
            "source_navigation": source_navigation,
            "scopes": debug
                .get("scopes")
                .cloned()
                .unwrap_or_else(|| serde_json::json!({})),
            "project_variables": debug
                .get("project_variables")
                .cloned()
                .unwrap_or_else(|| serde_json::json!([])),
            "locals": debug
                .get("locals")
                .cloned()
                .unwrap_or_else(|| serde_json::json!([])),
            "control_count": controls.len(),
            "breakpoint_count": breakpoints.len(),
            "function_breakpoint_count": function_breakpoints.len(),
            "data_breakpoint_count": data_breakpoints.len(),
            "exception_filter_count": exception_filters.len(),
            "watch_expression_count": watch_expressions.len(),
            "loaded_source_count": json_array_count(loaded_sources.get("sources")),
            "source_snapshot_count": source_snapshots.len(),
            "controls": controls,
            "breakpoints": breakpoints,
            "function_breakpoints": function_breakpoints,
            "data_breakpoints": data_breakpoints,
            "exception_filters": exception_filters,
            "watch_expressions": watch_expressions,
            "loaded_sources": loaded_sources,
            "source_snapshots": source_snapshots,
            "event_count": events.len(),
            "stopped_event_count": stopped_events.len(),
            "output_event_count": output_events.len(),
            "events": events,
            "stopped_events": stopped_events,
            "output_events": output_events,
        },
    })
}

pub(crate) fn editor_debug_all_event_frames(debug: &serde_json::Value) -> Vec<serde_json::Value> {
    debug
        .get("frames")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter(|frame| frame.get("type").and_then(serde_json::Value::as_str) == Some("event"))
        .cloned()
        .collect()
}

pub(crate) fn editor_debug_event_frames(
    debug: &serde_json::Value,
    event_name: &str,
) -> Vec<serde_json::Value> {
    debug
        .get("frames")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter(|frame| frame.get("type").and_then(serde_json::Value::as_str) == Some("event"))
        .filter(|frame| frame.get("event").and_then(serde_json::Value::as_str) == Some(event_name))
        .cloned()
        .collect()
}

pub(crate) fn editor_debug_result_artifact_json() -> serde_json::Value {
    serde_json::json!({
        "path": EDITOR_DEBUG_SESSION_RESULT_PATH,
        "html_path": EDITOR_DEBUG_SESSION_RESULT_HTML_PATH,
        "kind": "orv.editor.debug.runner.result",
        "media_type": "application/json",
        "panels": ["debug"],
        "panel_contract": editor_debug_result_panel_contract_json(),
    })
}

pub(crate) fn editor_debug_result_panel_contract_json() -> serde_json::Value {
    serde_json::json!({
        "schema_version": 1,
        "root": "panels.debug",
        "sections": [
            {
                "name": "production_context",
                "path": "panels.debug.production_context",
                "kind": "object",
            },
            {
                "name": "production_summary",
                "path": "panels.debug.production_summary",
                "kind": "object",
            },
            {
                "name": "session_summary",
                "path": "panels.debug.session_summary",
                "kind": "object",
            },
            {
                "name": "source_bundle",
                "path": "panels.debug.source_bundle",
                "kind": "object",
            },
            {
                "name": "selected_frame",
                "path": "panels.debug.selected_frame",
                "kind": "object",
            },
            {
                "name": "stack_frames",
                "path": "panels.debug.stack_frames",
                "kind": "array",
            },
            {
                "name": "source_navigation",
                "path": "panels.debug.source_navigation",
                "kind": "object",
            },
            {
                "name": "scopes",
                "path": "panels.debug.scopes",
                "kind": "object",
            },
            {
                "name": "locals",
                "path": "panels.debug.locals",
                "kind": "array",
            },
            {
                "name": "project_variables",
                "path": "panels.debug.project_variables",
                "kind": "array",
            },
            {
                "name": "controls",
                "path": "panels.debug.controls",
                "kind": "array",
            },
            {
                "name": "breakpoints",
                "path": "panels.debug.breakpoints",
                "kind": "array",
            },
            {
                "name": "function_breakpoints",
                "path": "panels.debug.function_breakpoints",
                "kind": "array",
            },
            {
                "name": "data_breakpoints",
                "path": "panels.debug.data_breakpoints",
                "kind": "array",
            },
            {
                "name": "exception_filters",
                "path": "panels.debug.exception_filters",
                "kind": "array",
            },
            {
                "name": "watch_expressions",
                "path": "panels.debug.watch_expressions",
                "kind": "array",
            },
            {
                "name": "loaded_sources",
                "path": "panels.debug.loaded_sources",
                "kind": "object",
            },
            {
                "name": "source_snapshots",
                "path": "panels.debug.source_snapshots",
                "kind": "array",
            },
            {
                "name": "stopped_events",
                "path": "panels.debug.stopped_events",
                "kind": "array",
            },
            {
                "name": "events",
                "path": "panels.debug.events",
                "kind": "array",
            },
            {
                "name": "output_events",
                "path": "panels.debug.output_events",
                "kind": "array",
            },
        ],
    })
}

pub(crate) fn write_editor_debug_runner_result_if_configured(
    state_path: &Path,
    value: &serde_json::Value,
) -> anyhow::Result<()> {
    let Some(result_path) = value
        .pointer("/runner/result/path")
        .and_then(serde_json::Value::as_str)
        .filter(|path| !path.trim().is_empty())
    else {
        return Ok(());
    };
    write_json(
        &resolve_editor_debug_runner_result_path(state_path, result_path),
        value,
    )
}

pub(crate) fn write_editor_debug_runner_result_html_if_configured(
    state_path: &Path,
    value: &serde_json::Value,
) -> anyhow::Result<()> {
    let html_path = value
        .pointer("/runner/result/html_path")
        .and_then(serde_json::Value::as_str)
        .filter(|path| !path.trim().is_empty())
        .unwrap_or(EDITOR_DEBUG_SESSION_RESULT_HTML_PATH);
    let path = resolve_editor_debug_runner_result_path(state_path, html_path);
    write_text(&path, &editor_debug_runner_result_html(value)?)
}

pub(crate) fn editor_debug_runner_result_html(value: &serde_json::Value) -> anyhow::Result<String> {
    let selected_frame = value
        .pointer("/panels/debug/selected_frame")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let stack_frames = value
        .pointer("/panels/debug/stack_frames")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    let stopped_events = value
        .pointer("/panels/debug/stopped_events")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    let events = value
        .pointer("/panels/debug/events")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    let output_events = value
        .pointer("/panels/debug/output_events")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    let session_summary = value
        .pointer("/panels/debug/session_summary")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));
    let source_bundle = value
        .pointer("/panels/debug/source_bundle")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));
    let production_context = value
        .pointer("/panels/debug/production_context")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let production_summary = value
        .pointer("/panels/debug/production_summary")
        .cloned()
        .unwrap_or_else(|| editor_debug_production_summary_from_context(&production_context));
    let source_navigation = value
        .pointer("/panels/debug/source_navigation")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));
    let locals = value
        .pointer("/panels/debug/locals")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    let scopes = value
        .pointer("/panels/debug/scopes/scopes")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    let project_variables = value
        .pointer("/panels/debug/project_variables")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    let controls = value
        .pointer("/panels/debug/controls")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    let breakpoints = value
        .pointer("/panels/debug/breakpoints")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    let function_breakpoints = value
        .pointer("/panels/debug/function_breakpoints")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    let data_breakpoints = value
        .pointer("/panels/debug/data_breakpoints")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    let exception_filters = value
        .pointer("/panels/debug/exception_filters")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    let watch_expressions = value
        .pointer("/panels/debug/watch_expressions")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    let source_snapshots = value
        .pointer("/panels/debug/source_snapshots")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    let control_count = value
        .pointer("/panels/debug/control_count")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0);
    let breakpoint_count = value
        .pointer("/panels/debug/breakpoint_count")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0);
    let function_breakpoint_count = value
        .pointer("/panels/debug/function_breakpoint_count")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0);
    let data_breakpoint_count = value
        .pointer("/panels/debug/data_breakpoint_count")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0);
    let exception_filter_count = value
        .pointer("/panels/debug/exception_filter_count")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0);
    let loaded_source_count = value
        .pointer("/panels/debug/loaded_source_count")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0);
    let source_snapshot_count = value
        .pointer("/panels/debug/source_snapshot_count")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0);
    let production_client_target_count =
        json_usize_field(&production_summary, "client_target_count");
    let production_client_manifest_count =
        json_usize_field(&production_summary, "client_manifest_count");
    let production_native_server_target_count =
        json_usize_field(&production_summary, "native_server_target_count");
    let production_native_server_route_count =
        json_usize_field(&production_summary, "native_server_route_count");
    let production_static_target_count =
        json_usize_field(&production_summary, "static_target_count");
    let production_static_verified_count =
        json_usize_field(&production_summary, "static_verified_count");
    let production_preflight_target_count =
        json_usize_field(&production_summary, "preflight_target_count");
    let production_preflight_smoke_present_count =
        json_usize_field(&production_summary, "preflight_smoke_summary_present_count");
    let production_preflight_smoke_gap_count =
        json_usize_field(&production_summary, "preflight_smoke_summary_missing_count")
            + json_usize_field(
                &production_summary,
                "preflight_smoke_summary_missing_marker_count",
            );
    let mut html = String::new();
    html.push_str("<!doctype html>\n<html lang=\"en\"><head><meta charset=\"utf-8\">\n");
    html.push_str("<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n");
    html.push_str("<title>orv debug result</title>\n");
    html.push_str("<style>body{margin:0;background:#f7f8fb;color:#18202f;font:14px/1.45 ui-sans-serif,system-ui,-apple-system,BlinkMacSystemFont,\"Segoe UI\",sans-serif}.shell{padding:20px;display:grid;gap:14px;grid-template-columns:repeat(2,minmax(0,1fr))}.panel{border:1px solid #d7dce5;background:#fff;border-radius:8px;padding:14px}.wide{grid-column:1/-1}h1{font-size:18px;margin:0}.metric{font-size:26px;font-weight:700}.muted{color:#687386}pre{white-space:pre-wrap;word-break:break-word;margin:0;max-height:320px;overflow:auto;background:#f1f5f9;border:1px solid #d7dce5;padding:10px}.list{list-style:none;margin:0;padding:0;display:grid;gap:6px}.list li{border-top:1px solid #d7dce5;padding-top:6px;color:#475569}@media(max-width:760px){.shell{grid-template-columns:1fr}}</style>\n");
    html.push_str("</head><body><main id=\"orv-debug-result\" class=\"shell\">\n");
    write!(
        &mut html,
        "<section class=\"panel wide\"><h1>Debug Result</h1><p class=\"muted\">DAP runner result rendered for native editor hosts.</p></section>"
    )?;
    write!(
        &mut html,
        "<section class=\"panel\"><h2>Controls</h2><div class=\"metric\">{control_count}</div><p class=\"muted\">executed controls</p></section>"
    )?;
    write!(
        &mut html,
        "<section class=\"panel\"><h2>Breakpoints</h2><div class=\"metric\">{breakpoint_count}</div><p class=\"muted\">requested breakpoints</p></section>"
    )?;
    write!(
        &mut html,
        "<section class=\"panel\"><h2>Function Breakpoints</h2><div class=\"metric\">{function_breakpoint_count}</div><p class=\"muted\">requested function breakpoints</p></section>"
    )?;
    write!(
        &mut html,
        "<section class=\"panel\"><h2>Data Breakpoints</h2><div class=\"metric\">{data_breakpoint_count}</div><p class=\"muted\">requested local data breakpoints</p></section>"
    )?;
    write!(
        &mut html,
        "<section class=\"panel\"><h2>Exception Filters</h2><div class=\"metric\">{exception_filter_count}</div><p class=\"muted\">configured exception filters</p></section>"
    )?;
    write!(
        &mut html,
        "<section class=\"panel\"><h2>Loaded Sources</h2><div class=\"metric\">{loaded_source_count}</div><p class=\"muted\">DAP loadedSources entries</p></section>"
    )?;
    write!(
        &mut html,
        "<section class=\"panel\"><h2>Source Snapshots</h2><div class=\"metric\">{source_snapshot_count}</div><p class=\"muted\">DAP source responses</p></section>"
    )?;
    html.push_str("<section class=\"panel wide\"><h2>Session Summary</h2><pre>");
    html.push_str(&html_escape_text(&editor_debug_session_summary_text(
        &session_summary,
    )));
    html.push_str("</pre></section>\n");
    html.push_str("<section class=\"panel wide\"><h2>Source Bundle</h2><pre>");
    html.push_str(&html_escape_text(&serde_json::to_string_pretty(
        &source_bundle,
    )?));
    html.push_str("</pre></section>\n");
    if !production_context.is_null() {
        writeln!(
            &mut html,
            "<section class=\"panel wide\"><h2>Production Summary</h2><div class=\"metric\">{production_client_target_count}</div><p class=\"muted\">client targets, {production_client_manifest_count} manifests</p><div class=\"metric\">{production_native_server_target_count}</div><p class=\"muted\">native plans, {production_native_server_route_count} routes</p><div class=\"metric\">{production_static_verified_count}/{production_static_target_count}</div><p class=\"muted\">verified static pages</p><div class=\"metric\">{production_preflight_smoke_present_count}/{production_preflight_target_count}</div><p class=\"muted\">smoke summaries, {production_preflight_smoke_gap_count} gaps</p><pre>{}</pre></section>",
            html_escape_text(&serde_json::to_string_pretty(&production_summary)?),
        )?;
        html.push_str("<section class=\"panel wide\"><h2>Production Context</h2><pre>");
        html.push_str(&html_escape_text(&serde_json::to_string_pretty(
            &production_context,
        )?));
        html.push_str("</pre></section>\n");
    }
    html.push_str("<section class=\"panel\"><h2>Selected Frame</h2><pre>");
    html.push_str(&html_escape_text(&editor_debug_frame_summary(
        &selected_frame,
    )));
    html.push_str("</pre></section>\n");
    html.push_str("<section class=\"panel\"><h2>Source Navigation</h2><pre>");
    html.push_str(&html_escape_text(&editor_debug_source_navigation_summary(
        &source_navigation,
    )));
    html.push_str("</pre></section>\n");
    html.push_str("<section class=\"panel\"><h2>Stack Frames</h2><ul class=\"list\">");
    for frame in stack_frames {
        write!(
            &mut html,
            "<li>{}</li>",
            html_escape_text(&editor_debug_frame_summary(&frame))
        )?;
    }
    html.push_str("</ul></section>\n");
    html.push_str("<section class=\"panel\"><h2>Scopes</h2><ul class=\"list\">");
    for scope in scopes {
        write!(
            &mut html,
            "<li>{}</li>",
            html_escape_text(&editor_debug_scope_summary(&scope))
        )?;
    }
    html.push_str("</ul></section>\n");
    html.push_str("<section class=\"panel\"><h2>Locals</h2><ul class=\"list\">");
    for local in locals {
        write!(
            &mut html,
            "<li>{}</li>",
            html_escape_text(&editor_debug_variable_summary(&local))
        )?;
    }
    html.push_str("</ul></section>\n");
    html.push_str("<section class=\"panel\"><h2>Project Variables</h2><ul class=\"list\">");
    for variable in project_variables {
        write!(
            &mut html,
            "<li>{}</li>",
            html_escape_text(&editor_debug_variable_summary(&variable))
        )?;
    }
    html.push_str("</ul></section>\n");
    html.push_str("<section class=\"panel\"><h2>Executed Controls</h2><ul class=\"list\">");
    for control in controls {
        write!(
            &mut html,
            "<li>{}</li>",
            html_escape_text(&editor_debug_control_summary(&control))
        )?;
    }
    html.push_str("</ul></section>\n");
    html.push_str("<section class=\"panel\"><h2>Requested Breakpoints</h2><ul class=\"list\">");
    for breakpoint in breakpoints {
        write!(
            &mut html,
            "<li>{}</li>",
            html_escape_text(&editor_debug_breakpoint_summary(&breakpoint))
        )?;
    }
    html.push_str("</ul></section>\n");
    html.push_str("<section class=\"panel\"><h2>Function Breakpoints</h2><ul class=\"list\">");
    for breakpoint in function_breakpoints {
        write!(
            &mut html,
            "<li>{}</li>",
            html_escape_text(&editor_debug_function_breakpoint_summary(&breakpoint))
        )?;
    }
    html.push_str("</ul></section>\n");
    html.push_str("<section class=\"panel\"><h2>Data Breakpoints</h2><ul class=\"list\">");
    for breakpoint in data_breakpoints {
        write!(
            &mut html,
            "<li>{}</li>",
            html_escape_text(&editor_debug_data_breakpoint_summary(&breakpoint))
        )?;
    }
    html.push_str("</ul></section>\n");
    html.push_str("<section class=\"panel\"><h2>Exception Filters</h2><ul class=\"list\">");
    for filter in exception_filters {
        write!(
            &mut html,
            "<li>{}</li>",
            html_escape_text(&editor_debug_exception_filter_summary(&filter))
        )?;
    }
    html.push_str("</ul></section>\n");
    html.push_str("<section class=\"panel\"><h2>Watch Expressions</h2><ul class=\"list\">");
    for expression in watch_expressions {
        write!(
            &mut html,
            "<li>{}</li>",
            html_escape_text(&editor_debug_watch_expression_summary(&expression))
        )?;
    }
    html.push_str("</ul></section>\n");
    html.push_str("<section class=\"panel wide\"><h2>Source Snapshots</h2><ul class=\"list\">");
    for snapshot in source_snapshots {
        write!(
            &mut html,
            "<li>{}</li>",
            html_escape_text(&editor_debug_source_snapshot_summary(&snapshot))
        )?;
    }
    html.push_str("</ul></section>\n");
    html.push_str("<section class=\"panel\"><h2>Stopped Events</h2><ul class=\"list\">");
    for event in stopped_events {
        write!(
            &mut html,
            "<li>{}</li>",
            html_escape_text(&editor_debug_event_summary(&event))
        )?;
    }
    html.push_str("</ul></section>\n");
    html.push_str("<section class=\"panel\"><h2>All Events</h2><ul class=\"list\">");
    for event in events {
        write!(
            &mut html,
            "<li>{}</li>",
            html_escape_text(&editor_debug_event_summary(&event))
        )?;
    }
    html.push_str("</ul></section>\n");
    html.push_str("<section class=\"panel\"><h2>Output Events</h2><ul class=\"list\">");
    for event in output_events {
        write!(
            &mut html,
            "<li>{}</li>",
            html_escape_text(&editor_debug_event_summary(&event))
        )?;
    }
    html.push_str("</ul></section>\n</main></body></html>\n");
    Ok(html)
}

pub(crate) fn editor_debug_session_summary_json(
    debug: &serde_json::Value,
    selected_frame: &serde_json::Value,
    events: &[serde_json::Value],
    stopped_events: &[serde_json::Value],
    output_events: &[serde_json::Value],
) -> serde_json::Value {
    let selected_line = selected_frame
        .get("line")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let selected_frame_id = selected_frame
        .get("id")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let selected_frame_name = selected_frame
        .get("name")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let selected_source = selected_frame
        .pointer("/source/path")
        .cloned()
        .or_else(|| selected_frame.pointer("/source/name").cloned())
        .unwrap_or(serde_json::Value::Null);
    let last_event = events
        .last()
        .and_then(|event| event.get("event"))
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let last_stopped_reason = stopped_events
        .last()
        .and_then(|event| event.pointer("/body/reason"))
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let source_bundle = editor_debug_launch_source_bundle(debug);
    let source_bundle_file_count = source_bundle
        .get("fileCount")
        .cloned()
        .unwrap_or_else(|| serde_json::json!(0));
    serde_json::json!({
        "schema_version": 1,
        "program": debug.get("program").cloned().unwrap_or(serde_json::Value::Null),
        "source_bundle": source_bundle,
        "source_bundle_file_count": source_bundle_file_count,
        "selected_frame_id": selected_frame_id,
        "selected_frame": selected_frame_name,
        "selected_line": selected_line,
        "selected_source": selected_source,
        "last_event": last_event,
        "last_stopped_reason": last_stopped_reason,
        "request_count": debug
            .pointer("/transport/request_count")
            .cloned()
            .unwrap_or_else(|| serde_json::json!(0)),
        "frame_count": debug
            .pointer("/transport/frame_count")
            .cloned()
            .unwrap_or_else(|| serde_json::json!(0)),
        "control_count": json_array_count(debug.get("controls")),
        "breakpoint_count": json_array_count(debug.get("breakpoints")),
        "function_breakpoint_count": json_array_count(debug.get("function_breakpoints")),
        "data_breakpoint_count": json_array_count(debug.get("data_breakpoints")),
        "exception_filter_count": json_array_count(debug.get("exception_filters")),
        "watch_expression_count": json_array_count(debug.get("watch_expressions")),
        "event_count": events.len(),
        "stopped_event_count": stopped_events.len(),
        "output_event_count": output_events.len(),
    })
}

pub(crate) fn editor_debug_launch_source_bundle(debug: &serde_json::Value) -> serde_json::Value {
    debug
        .pointer("/launch/body/sourceBundle")
        .cloned()
        .filter(|value| !value.is_null())
        .unwrap_or(serde_json::Value::Null)
}

pub(crate) fn editor_debug_source_navigation_json(
    selected_frame: &serde_json::Value,
    stack_frames: &[serde_json::Value],
) -> serde_json::Value {
    let frames = stack_frames
        .iter()
        .filter_map(editor_debug_source_navigation_frame_json)
        .collect::<Vec<_>>();
    serde_json::json!({
        "schema_version": 1,
        "selected": editor_debug_source_navigation_frame_json(selected_frame)
            .unwrap_or_else(|| serde_json::json!({})),
        "frame_count": frames.len(),
        "frames": frames,
    })
}

pub(crate) fn editor_debug_source_navigation_frame_json(
    frame: &serde_json::Value,
) -> Option<serde_json::Value> {
    let source_path = frame
        .pointer("/source/path")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("");
    let source_name = frame
        .pointer("/source/name")
        .and_then(serde_json::Value::as_str)
        .unwrap_or(source_path);
    if source_path.is_empty() && source_name.is_empty() {
        return None;
    }
    Some(serde_json::json!({
        "frame_id": frame.get("id").cloned().unwrap_or(serde_json::Value::Null),
        "frame_name": frame
            .get("name")
            .cloned()
            .unwrap_or_else(|| serde_json::json!("frame")),
        "source": {
            "path": source_path,
            "name": source_name,
        },
        "line": frame.get("line").cloned().unwrap_or(serde_json::Value::Null),
        "column": frame
            .get("column")
            .cloned()
            .unwrap_or_else(|| serde_json::json!(1)),
    }))
}

pub(crate) fn editor_debug_session_summary_text(summary: &serde_json::Value) -> String {
    let selected_line = summary
        .get("selected_line")
        .and_then(serde_json::Value::as_u64)
        .map_or_else(|| "line ?".to_string(), |line| format!("line {line}"));
    let source_bundle = summary
        .get("source_bundle")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let source_bundle_line = source_bundle
        .get("path")
        .and_then(serde_json::Value::as_str)
        .map(|path| {
            format!(
                "source_bundle {} files {} hash {}",
                path,
                json_u64_field(&source_bundle, "fileCount"),
                json_str_or_empty(&source_bundle, "hash")
            )
        })
        .unwrap_or_default();
    [
        format!("program {}", json_str_or_empty(summary, "program")),
        source_bundle_line,
        format!(
            "selected {} {}",
            json_str_or_empty(summary, "selected_frame"),
            selected_line
        ),
        format!("source {}", json_str_or_empty(summary, "selected_source")),
        format!("last_event {}", json_str_or_empty(summary, "last_event")),
        format!(
            "last_stop {}",
            json_str_or_empty(summary, "last_stopped_reason")
        ),
        format!(
            "requests {} frames {}",
            json_u64_field(summary, "request_count"),
            json_u64_field(summary, "frame_count")
        ),
        format!(
            "controls {} breakpoints {} function_breakpoints {} data_breakpoints {} exception_filters {} watches {} events {} stopped {} output {}",
            json_u64_field(summary, "control_count"),
            json_u64_field(summary, "breakpoint_count"),
            json_u64_field(summary, "function_breakpoint_count"),
            json_u64_field(summary, "data_breakpoint_count"),
            json_u64_field(summary, "exception_filter_count"),
            json_u64_field(summary, "watch_expression_count"),
            json_u64_field(summary, "event_count"),
            json_u64_field(summary, "stopped_event_count"),
            json_u64_field(summary, "output_event_count")
        ),
    ]
    .into_iter()
    .filter(|line| !line.trim().is_empty())
    .collect::<Vec<_>>()
    .join("\n")
}

pub(crate) fn editor_debug_source_navigation_summary(navigation: &serde_json::Value) -> String {
    let selected = navigation
        .get("selected")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));
    let selected_line = selected
        .get("line")
        .and_then(serde_json::Value::as_u64)
        .map_or_else(|| "line ?".to_string(), |line| format!("line {line}"));
    let selected_path = selected
        .pointer("/source/path")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("");
    let mut lines = vec![
        format!("selected {}", json_str_or_empty(&selected, "frame_name")),
        format!("{selected_line} {selected_path}"),
        format!("frames {}", json_u64_field(navigation, "frame_count")),
    ];
    if let Some(frames) = navigation
        .get("frames")
        .and_then(serde_json::Value::as_array)
    {
        for frame in frames {
            let line = frame
                .get("line")
                .and_then(serde_json::Value::as_u64)
                .map_or_else(|| "line ?".to_string(), |line| format!("line {line}"));
            let source = frame
                .pointer("/source/path")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("");
            lines.push(format!(
                "{} {line} {source}",
                json_str_or_empty(frame, "frame_name")
            ));
        }
    }
    lines
        .into_iter()
        .filter(|line| !line.trim().is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

pub(crate) fn editor_debug_frame_summary(frame: &serde_json::Value) -> String {
    let name = frame
        .get("name")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("frame");
    let line = frame
        .get("line")
        .and_then(serde_json::Value::as_u64)
        .map_or_else(|| "line ?".to_string(), |line| format!("line {line}"));
    let source = frame
        .pointer("/source/path")
        .and_then(serde_json::Value::as_str)
        .or_else(|| {
            frame
                .pointer("/source/name")
                .and_then(serde_json::Value::as_str)
        })
        .unwrap_or("");
    [name.to_string(), line, source.to_string()]
        .into_iter()
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

pub(crate) fn editor_debug_scope_summary(scope: &serde_json::Value) -> String {
    let name = scope
        .get("name")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("scope");
    let reference = scope
        .get("variablesReference")
        .and_then(serde_json::Value::as_u64)
        .map_or_else(String::new, |reference| format!("ref {reference}"));
    let source = scope
        .pointer("/source/path")
        .and_then(serde_json::Value::as_str)
        .or_else(|| {
            scope
                .pointer("/source/name")
                .and_then(serde_json::Value::as_str)
        })
        .unwrap_or("");
    [name.to_string(), reference, source.to_string()]
        .into_iter()
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

pub(crate) fn editor_debug_event_summary(event: &serde_json::Value) -> String {
    let name = event
        .get("event")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("event");
    let reason = event
        .pointer("/body/reason")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("");
    let thread = event
        .pointer("/body/threadId")
        .and_then(serde_json::Value::as_u64)
        .map_or_else(String::new, |thread| format!("thread {thread}"));
    [name.to_string(), reason.to_string(), thread]
        .into_iter()
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

pub(crate) fn editor_debug_control_summary(control: &serde_json::Value) -> String {
    let name = control
        .get("name")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("control");
    let command = control
        .pointer("/request/command")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("");
    let success = control
        .pointer("/response/success")
        .and_then(serde_json::Value::as_bool)
        .map_or_else(String::new, |success| {
            format!("success {}", if success { "true" } else { "false" })
        });
    [name.to_string(), command.to_string(), success]
        .into_iter()
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

pub(crate) fn editor_debug_breakpoint_summary(breakpoint: &serde_json::Value) -> String {
    let source = breakpoint
        .pointer("/source/path")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("source");
    let lines = breakpoint
        .get("lines")
        .and_then(serde_json::Value::as_array)
        .map(|lines| {
            lines
                .iter()
                .filter_map(serde_json::Value::as_u64)
                .map(|line| line.to_string())
                .collect::<Vec<_>>()
                .join(",")
        })
        .unwrap_or_default();
    let success = breakpoint
        .pointer("/response/success")
        .and_then(serde_json::Value::as_bool)
        .map_or_else(String::new, |success| {
            format!("success {}", if success { "true" } else { "false" })
        });
    [
        source.to_string(),
        if lines.is_empty() {
            String::new()
        } else {
            format!("lines {lines}")
        },
        success,
    ]
    .into_iter()
    .filter(|part| !part.is_empty())
    .collect::<Vec<_>>()
    .join(" ")
}

pub(crate) fn editor_debug_function_breakpoint_summary(breakpoint: &serde_json::Value) -> String {
    let names = breakpoint
        .get("names")
        .and_then(serde_json::Value::as_array)
        .map(|names| {
            names
                .iter()
                .filter_map(serde_json::Value::as_str)
                .collect::<Vec<_>>()
                .join(",")
        })
        .or_else(|| {
            breakpoint
                .get("name")
                .and_then(serde_json::Value::as_str)
                .map(str::to_string)
        })
        .unwrap_or_else(|| "function".to_string());
    let success = breakpoint
        .pointer("/response/success")
        .and_then(serde_json::Value::as_bool)
        .map_or_else(String::new, |success| {
            format!("success {}", if success { "true" } else { "false" })
        });
    [format!("functions {names}"), success]
        .into_iter()
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

pub(crate) fn editor_debug_data_breakpoint_summary(breakpoint: &serde_json::Value) -> String {
    let names = breakpoint
        .get("names")
        .and_then(serde_json::Value::as_array)
        .map(|names| {
            names
                .iter()
                .filter_map(serde_json::Value::as_str)
                .collect::<Vec<_>>()
                .join(",")
        })
        .or_else(|| {
            breakpoint
                .get("name")
                .and_then(serde_json::Value::as_str)
                .map(str::to_string)
        })
        .unwrap_or_else(|| "local".to_string());
    let success = breakpoint
        .pointer("/response/success")
        .and_then(serde_json::Value::as_bool)
        .map_or_else(String::new, |success| {
            format!("success {}", if success { "true" } else { "false" })
        });
    [format!("locals {names}"), success]
        .into_iter()
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

pub(crate) fn editor_debug_exception_filter_summary(filter: &serde_json::Value) -> String {
    let filters = filter
        .get("filters")
        .and_then(serde_json::Value::as_array)
        .map(|filters| {
            filters
                .iter()
                .filter_map(serde_json::Value::as_str)
                .collect::<Vec<_>>()
                .join(",")
        })
        .or_else(|| {
            filter
                .get("filter")
                .and_then(serde_json::Value::as_str)
                .map(str::to_string)
        })
        .unwrap_or_else(|| "exception".to_string());
    let success = filter
        .pointer("/response/success")
        .and_then(serde_json::Value::as_bool)
        .map_or_else(String::new, |success| {
            format!("success {}", if success { "true" } else { "false" })
        });
    [format!("filters {filters}"), success]
        .into_iter()
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

pub(crate) fn editor_debug_watch_expression_summary(expression: &serde_json::Value) -> String {
    let label = expression
        .get("expression")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("expression");
    let result = expression
        .pointer("/response/body/result")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("");
    let value_type = expression
        .pointer("/response/body/type")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("");
    let success = expression
        .pointer("/response/success")
        .and_then(serde_json::Value::as_bool)
        .map_or_else(String::new, |success| {
            format!("success {}", if success { "true" } else { "false" })
        });
    [
        label.to_string(),
        result.to_string(),
        value_type.to_string(),
        success,
    ]
    .into_iter()
    .filter(|part| !part.is_empty())
    .collect::<Vec<_>>()
    .join(" ")
}

pub(crate) fn editor_debug_source_snapshot_summary(snapshot: &serde_json::Value) -> String {
    let name = snapshot
        .pointer("/source/name")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("source");
    let path = snapshot
        .pointer("/source/path")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("");
    let checksum = snapshot
        .pointer("/checksum/value")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("");
    let length = json_u64_field(snapshot, "content_length");
    let lines = json_u64_field(snapshot, "line_count");
    let success = snapshot
        .pointer("/response/success")
        .and_then(serde_json::Value::as_bool)
        .map_or_else(String::new, |success| {
            format!("success {}", if success { "true" } else { "false" })
        });
    [
        name.to_string(),
        path.to_string(),
        format!("bytes {length}"),
        format!("lines {lines}"),
        checksum.to_string(),
        success,
    ]
    .into_iter()
    .filter(|part| !part.is_empty())
    .collect::<Vec<_>>()
    .join(" ")
}

pub(crate) fn editor_debug_variable_summary(variable: &serde_json::Value) -> String {
    let name = variable
        .get("name")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("variable");
    let value = variable
        .get("value")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("");
    let value_type = variable
        .get("type")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("");
    [name.to_string(), value.to_string(), value_type.to_string()]
        .into_iter()
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

pub(crate) fn resolve_editor_debug_runner_result_path(
    state_path: &Path,
    result_path: &str,
) -> PathBuf {
    let result_path = Path::new(result_path);
    if result_path.is_absolute() {
        return result_path.to_path_buf();
    }
    editor_debug_runner_artifact_root(state_path).join(result_path)
}

pub(crate) fn editor_debug_runner_artifact_root(state_path: &Path) -> PathBuf {
    if state_path.is_dir() {
        return state_path.to_path_buf();
    }
    let parent = state_path.parent().unwrap_or_else(|| Path::new("."));
    let file_name = state_path.file_name().and_then(|name| name.to_str());
    let parent_name = parent.file_name().and_then(|name| name.to_str());
    if file_name == Some("session-runner.json") && parent_name == Some("debug") {
        return parent.parent().unwrap_or(parent).to_path_buf();
    }
    parent.to_path_buf()
}

pub(crate) fn editor_debug_push_source_requests(
    requests: &mut Vec<serde_json::Value>,
    next_seq: &mut u64,
    sources: &[DapSourceInfo],
) -> Vec<(u64, DapSourceInfo, serde_json::Value)> {
    let mut source_requests = Vec::new();
    for source in sources {
        let seq = *next_seq;
        *next_seq += 1;
        let request = editor_debug_source_request_json(seq, source);
        requests.push(request.clone());
        source_requests.push((seq, source.clone(), request));
    }
    source_requests
}

pub(crate) fn editor_debug_push_breakpoint_requests(
    requests: &mut Vec<serde_json::Value>,
    next_seq: &mut u64,
    breakpoints: &[EditorDebugBreakpoint],
) -> Vec<(u64, PathBuf, Vec<u64>, serde_json::Value)> {
    let mut breakpoint_requests = Vec::new();
    for (source_path, lines) in editor_debug_breakpoint_request_groups(breakpoints) {
        let seq = *next_seq;
        *next_seq += 1;
        let request = editor_debug_set_breakpoints_request_json(seq, &source_path, &lines);
        requests.push(request.clone());
        breakpoint_requests.push((seq, source_path, lines, request));
    }
    breakpoint_requests
}

pub(crate) fn editor_debug_push_function_breakpoint_requests(
    requests: &mut Vec<serde_json::Value>,
    next_seq: &mut u64,
    function_breakpoints: &[String],
) -> Vec<(u64, Vec<String>, serde_json::Value)> {
    let names = editor_debug_function_breakpoint_names(function_breakpoints);
    if names.is_empty() {
        return Vec::new();
    }
    let seq = *next_seq;
    *next_seq += 1;
    let request = editor_debug_set_function_breakpoints_request_json(seq, &names);
    requests.push(request.clone());
    vec![(seq, names, request)]
}

pub(crate) fn editor_debug_push_exception_filter_requests(
    requests: &mut Vec<serde_json::Value>,
    next_seq: &mut u64,
    exception_filters: &[String],
) -> Vec<(u64, Vec<String>, serde_json::Value)> {
    let filters = editor_debug_exception_filter_names(exception_filters);
    if filters.is_empty() {
        return Vec::new();
    }
    let seq = *next_seq;
    *next_seq += 1;
    let request = editor_debug_set_exception_breakpoints_request_json(seq, &filters);
    requests.push(request.clone());
    vec![(seq, filters, request)]
}

pub(crate) fn editor_debug_push_data_breakpoint_requests(
    requests: &mut Vec<serde_json::Value>,
    next_seq: &mut u64,
    data_breakpoints: &[String],
) -> (
    Vec<EditorDebugDataBreakpointInfoRequest>,
    Option<EditorDebugDataBreakpointSetRequest>,
) {
    let names = editor_debug_data_breakpoint_names(data_breakpoints);
    if names.is_empty() {
        return (Vec::new(), None);
    }
    let mut info_requests = Vec::new();
    for name in &names {
        let seq = *next_seq;
        *next_seq += 1;
        let request = editor_debug_data_breakpoint_info_request_json(seq, name);
        requests.push(request.clone());
        info_requests.push((seq, name.clone(), request));
    }
    let seq = *next_seq;
    *next_seq += 1;
    let request = editor_debug_set_data_breakpoints_request_json(seq, &names);
    requests.push(request.clone());
    (info_requests, Some((seq, names, request)))
}

pub(crate) fn editor_debug_push_control_requests(
    requests: &mut Vec<serde_json::Value>,
    next_seq: &mut u64,
    controls: &[EditorDebugControl],
) -> Vec<(u64, EditorDebugControl, serde_json::Value)> {
    let mut control_requests = Vec::new();
    for control in controls.iter().copied() {
        let seq = *next_seq;
        *next_seq += 1;
        let control_request = control.request_json();
        let control_command = control_request
            .get("command")
            .cloned()
            .unwrap_or_else(|| serde_json::json!("next"));
        let control_arguments = control_request
            .get("arguments")
            .cloned()
            .unwrap_or_else(|| serde_json::json!({}));
        requests.push(serde_json::json!({
            "seq": seq,
            "type": "request",
            "command": control_command,
            "arguments": control_arguments,
        }));
        control_requests.push((seq, control, control_request));
    }
    control_requests
}

pub(crate) fn editor_debug_push_watch_expression_requests(
    requests: &mut Vec<serde_json::Value>,
    next_seq: &mut u64,
    watch_expressions: &[String],
) -> Vec<(u64, String, serde_json::Value)> {
    let mut watch_requests = Vec::new();
    for expression in watch_expressions
        .iter()
        .map(|expression| expression.trim())
        .filter(|expression| !expression.is_empty())
    {
        let seq = *next_seq;
        *next_seq += 1;
        let request = serde_json::json!({
            "seq": seq,
            "type": "request",
            "command": "evaluate",
            "arguments": {
                "expression": expression,
                "frameId": 1,
                "context": "watch",
            },
        });
        requests.push(request.clone());
        watch_requests.push((seq, expression.to_string(), request));
    }
    watch_requests
}

pub(crate) fn editor_debug_breakpoint_summaries(
    frames: &[serde_json::Value],
    breakpoint_requests: Vec<(u64, PathBuf, Vec<u64>, serde_json::Value)>,
) -> Vec<serde_json::Value> {
    breakpoint_requests
        .into_iter()
        .map(|(seq, source_path, lines, request)| {
            serde_json::json!({
                "source": {
                    "path": source_path.display().to_string(),
                },
                "lines": lines,
                "request": request,
                "response": dap_response_for_request_seq(frames, seq)
                    .unwrap_or(serde_json::Value::Null),
            })
        })
        .collect()
}

pub(crate) fn editor_debug_source_snapshot_summaries(
    frames: &[serde_json::Value],
    source_requests: Vec<(u64, DapSourceInfo, serde_json::Value)>,
) -> Vec<serde_json::Value> {
    source_requests
        .into_iter()
        .map(|(seq, source, request)| {
            let response =
                dap_response_for_request_seq(frames, seq).unwrap_or(serde_json::Value::Null);
            let content = response
                .pointer("/body/content")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("");
            serde_json::json!({
                "source": dap_source_json(&source),
                "request": request,
                "response": response,
                "content_length": content.len(),
                "line_count": content.lines().count(),
                "checksum": {
                    "algorithm": "SHA256",
                    "value": source.checksum,
                },
            })
        })
        .collect()
}

pub(crate) fn editor_debug_function_breakpoint_summaries(
    frames: &[serde_json::Value],
    function_breakpoint_requests: Vec<(u64, Vec<String>, serde_json::Value)>,
) -> Vec<serde_json::Value> {
    function_breakpoint_requests
        .into_iter()
        .map(|(seq, names, request)| {
            serde_json::json!({
                "names": names,
                "request": request,
                "response": dap_response_for_request_seq(frames, seq)
                    .unwrap_or(serde_json::Value::Null),
            })
        })
        .collect()
}

pub(crate) fn editor_debug_data_breakpoint_summaries(
    frames: &[serde_json::Value],
    info_requests: Vec<EditorDebugDataBreakpointInfoRequest>,
    set_request: Option<EditorDebugDataBreakpointSetRequest>,
) -> Vec<serde_json::Value> {
    let Some((set_seq, names, request)) = set_request else {
        return Vec::new();
    };
    let infos = info_requests
        .into_iter()
        .map(|(seq, name, request)| {
            serde_json::json!({
                "name": name,
                "request": request,
                "response": dap_response_for_request_seq(frames, seq)
                    .unwrap_or(serde_json::Value::Null),
            })
        })
        .collect::<Vec<_>>();
    vec![serde_json::json!({
        "names": names,
        "infos": infos,
        "request": request,
        "response": dap_response_for_request_seq(frames, set_seq)
            .unwrap_or(serde_json::Value::Null),
    })]
}

pub(crate) fn editor_debug_exception_filter_summaries(
    frames: &[serde_json::Value],
    exception_filter_requests: Vec<(u64, Vec<String>, serde_json::Value)>,
) -> Vec<serde_json::Value> {
    exception_filter_requests
        .into_iter()
        .map(|(seq, filters, request)| {
            serde_json::json!({
                "filters": filters,
                "request": request,
                "response": dap_response_for_request_seq(frames, seq)
                    .unwrap_or(serde_json::Value::Null),
            })
        })
        .collect()
}

pub(crate) fn editor_debug_watch_expression_summaries(
    frames: &[serde_json::Value],
    watch_requests: Vec<(u64, String, serde_json::Value)>,
) -> Vec<serde_json::Value> {
    watch_requests
        .into_iter()
        .map(|(seq, expression, request)| {
            serde_json::json!({
                "expression": expression,
                "request": request,
                "response": dap_response_for_request_seq(frames, seq)
                    .unwrap_or(serde_json::Value::Null),
            })
        })
        .collect()
}

pub(crate) fn editor_debug_control_summaries(
    frames: &[serde_json::Value],
    control_requests: Vec<(u64, EditorDebugControl, serde_json::Value)>,
) -> Vec<serde_json::Value> {
    control_requests
        .into_iter()
        .map(|(seq, control, control_request)| {
            serde_json::json!({
                "name": control.label(),
                "request": control_request,
                "response": dap_response_for_request_seq(frames, seq)
                    .unwrap_or(serde_json::Value::Null),
            })
        })
        .collect()
}

pub(crate) fn editor_debug_adapter_json() -> serde_json::Value {
    serde_json::json!({
        "protocol": "dap",
        "command": ["orv", "dap", "serve", "--stdio"],
    })
}

pub(crate) fn editor_debug_capabilities_json() -> serde_json::Value {
    serde_json::json!({
        "supportsConfigurationDoneRequest": true,
        "supportsLoadedSourcesRequest": true,
        "supportsBreakpointLocationsRequest": true,
        "supportsConditionalBreakpoints": true,
        "supportsHitConditionalBreakpoints": true,
        "supportsFunctionBreakpoints": true,
        "supportsDataBreakpoints": true,
        "supportsExceptionInfoRequest": true,
        "supportsRestartRequest": true,
        "supportsSetVariable": true,
        "supportsSetExpression": true,
        "supportsModulesRequest": true,
        "supportsGotoTargetsRequest": true,
        "supportsStepBack": true,
        "supportsStepInTargetsRequest": true,
        "supportsRestartFrame": true,
        "supportsPauseRequest": true,
        "supportsCancelRequest": true,
        "supportsInstructionBreakpoints": true,
        "supportsDisassembleRequest": true,
        "supportsReadMemoryRequest": true,
        "supportsOrvRuntimeAttach": true,
        "supportsOrvRuntimeTracePath": true,
        "supportsOrvSourceBundleLaunch": true,
        "exceptionBreakpointFilters": [
            {
                "filter": "orv.diagnostics",
                "label": "ORV diagnostics",
                "default": true,
            },
            {
                "filter": "orv.runtime",
                "label": "ORV runtime errors",
                "default": true,
            },
        ],
    })
}

pub(crate) fn editor_debug_session_runner_json(path: &Path) -> serde_json::Value {
    let program = path.display().to_string();
    serde_json::json!({
        "schema_version": 1,
        "kind": "orv.editor.debug.runner",
        "program": program,
        "transport": editor_debug_runner_transport_json(),
        "command": editor_debug_control_runner_command(EditorDebugControl::Next),
        "result": editor_debug_result_artifact_json(),
        "session": editor_debug_runner_session_contract_json(),
        "controls": editor_debug_session_runner_controls_json(),
    })
}

pub(crate) fn editor_debug_runner_transport_json() -> serde_json::Value {
    serde_json::json!({
        "protocol": "dap",
        "framing": "content-length",
    })
}

pub(crate) fn editor_debug_runner_session_contract_json() -> serde_json::Value {
    serde_json::json!({
        "launch": {
            "live": true,
        },
        "thread_id": 1,
        "breakpoint_argument": "--breakpoint",
        "breakpoint_format": "<path>:<line>",
        "function_breakpoint_argument": "--function-breakpoint",
        "function_breakpoint_format": "<function-name>",
        "data_breakpoint_argument": "--data-breakpoint",
        "data_breakpoint_format": "<local-name>",
        "exception_filter_argument": "--exception-filter",
        "exception_filter_format": "<orv.diagnostics|orv.runtime>",
        "watch_expression_argument": "--watch-expression",
        "watch_expression_format": "<expression>",
        "reuse_session": true,
    })
}

pub(crate) const fn editor_debug_control_order() -> [EditorDebugControl; 13] {
    [
        EditorDebugControl::Continue,
        EditorDebugControl::Pause,
        EditorDebugControl::ReverseContinue,
        EditorDebugControl::Next,
        EditorDebugControl::StepBack,
        EditorDebugControl::StepIn,
        EditorDebugControl::StepInTargets,
        EditorDebugControl::StepOut,
        EditorDebugControl::RestartFrame,
        EditorDebugControl::Restart,
        EditorDebugControl::Terminate,
        EditorDebugControl::TerminateThreads,
        EditorDebugControl::Disconnect,
    ]
}

pub(crate) fn editor_debug_control_runner_command(
    control: EditorDebugControl,
) -> serde_json::Value {
    serde_json::json!([
        "orv",
        "editor",
        "run-debug",
        EDITOR_DEBUG_SESSION_RUNNER_PATH,
        "--control",
        control.cli_value()
    ])
}

pub(crate) fn editor_debug_breakpoint_runner_command(
    path: &Path,
    line: u64,
    control: EditorDebugControl,
) -> serde_json::Value {
    serde_json::json!([
        "orv",
        "editor",
        "run-debug",
        EDITOR_DEBUG_SESSION_RUNNER_PATH,
        "--breakpoint",
        format!("{}:{line}", path.display()),
        "--control",
        control.cli_value()
    ])
}

pub(crate) fn editor_debug_function_breakpoint_runner_command(
    name: &str,
    control: EditorDebugControl,
) -> serde_json::Value {
    serde_json::json!([
        "orv",
        "editor",
        "run-debug",
        EDITOR_DEBUG_SESSION_RUNNER_PATH,
        "--function-breakpoint",
        name,
        "--control",
        control.cli_value()
    ])
}

pub(crate) fn editor_debug_data_breakpoint_runner_command(
    name: &str,
    control: EditorDebugControl,
) -> serde_json::Value {
    serde_json::json!([
        "orv",
        "editor",
        "run-debug",
        EDITOR_DEBUG_SESSION_RUNNER_PATH,
        "--data-breakpoint",
        name,
        "--control",
        control.cli_value()
    ])
}

pub(crate) fn editor_debug_exception_filter_runner_command(
    filter: &str,
    control: EditorDebugControl,
) -> serde_json::Value {
    serde_json::json!([
        "orv",
        "editor",
        "run-debug",
        EDITOR_DEBUG_SESSION_RUNNER_PATH,
        "--exception-filter",
        filter,
        "--control",
        control.cli_value()
    ])
}

pub(crate) fn editor_debug_breakpoint_request_groups(
    breakpoints: &[EditorDebugBreakpoint],
) -> Vec<(PathBuf, Vec<u64>)> {
    let mut grouped = BTreeMap::<PathBuf, BTreeSet<u64>>::new();
    for breakpoint in breakpoints {
        grouped
            .entry(breakpoint.path.clone())
            .or_default()
            .insert(breakpoint.line);
    }
    grouped
        .into_iter()
        .map(|(path, lines)| (path, lines.into_iter().collect()))
        .collect()
}

pub(crate) fn editor_debug_function_breakpoint_names(
    function_breakpoints: &[String],
) -> Vec<String> {
    function_breakpoints
        .iter()
        .map(|name| name.trim())
        .filter(|name| !name.is_empty())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .map(str::to_string)
        .collect()
}

pub(crate) fn editor_debug_data_breakpoint_names(data_breakpoints: &[String]) -> Vec<String> {
    data_breakpoints
        .iter()
        .map(|name| name.trim())
        .filter(|name| !name.is_empty())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .map(str::to_string)
        .collect()
}

pub(crate) fn editor_debug_exception_filter_names(exception_filters: &[String]) -> Vec<String> {
    exception_filters
        .iter()
        .map(|filter| filter.trim())
        .filter(|filter| matches!(*filter, "orv.diagnostics" | "orv.runtime"))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .map(str::to_string)
        .collect()
}

pub(crate) fn editor_debug_set_breakpoints_request_json(
    seq: u64,
    path: &Path,
    lines: &[u64],
) -> serde_json::Value {
    serde_json::json!({
        "seq": seq,
        "type": "request",
        "command": "setBreakpoints",
        "arguments": {
            "source": {
                "path": path.display().to_string(),
            },
            "breakpoints": lines
                .iter()
                .map(|line| serde_json::json!({"line": line}))
                .collect::<Vec<_>>(),
        },
    })
}

pub(crate) fn editor_debug_set_function_breakpoints_request_json(
    seq: u64,
    names: &[String],
) -> serde_json::Value {
    serde_json::json!({
        "seq": seq,
        "type": "request",
        "command": "setFunctionBreakpoints",
        "arguments": {
            "breakpoints": names
                .iter()
                .map(|name| serde_json::json!({"name": name}))
                .collect::<Vec<_>>(),
        },
    })
}

pub(crate) fn editor_debug_data_breakpoint_info_request_json(
    seq: u64,
    name: &str,
) -> serde_json::Value {
    serde_json::json!({
        "seq": seq,
        "type": "request",
        "command": "dataBreakpointInfo",
        "arguments": {
            "variablesReference": 2,
            "name": name,
        },
    })
}

pub(crate) fn editor_debug_set_data_breakpoints_request_json(
    seq: u64,
    names: &[String],
) -> serde_json::Value {
    serde_json::json!({
        "seq": seq,
        "type": "request",
        "command": "setDataBreakpoints",
        "arguments": {
            "breakpoints": names
                .iter()
                .map(|name| {
                    serde_json::json!({
                        "dataId": format!("local:{name}"),
                        "accessType": "write",
                    })
                })
                .collect::<Vec<_>>(),
        },
    })
}

pub(crate) fn editor_debug_set_exception_breakpoints_request_json(
    seq: u64,
    filters: &[String],
) -> serde_json::Value {
    serde_json::json!({
        "seq": seq,
        "type": "request",
        "command": "setExceptionBreakpoints",
        "arguments": {
            "filters": filters,
        },
    })
}

pub(crate) fn editor_debug_session_runner_controls_json() -> Vec<serde_json::Value> {
    editor_debug_control_order()
        .into_iter()
        .map(|control| {
            serde_json::json!({
                "name": control.label(),
                "value": control.cli_value(),
                "command": editor_debug_control_runner_command(control),
                "request": control.request_json(),
            })
        })
        .collect()
}

pub(crate) fn editor_debug_configurations_json(path: &Path) -> Vec<serde_json::Value> {
    let program = path.display().to_string();
    vec![
        serde_json::json!({
            "name": "Launch ORV",
            "type": "orv",
            "request": "launch",
            "program": program.clone(),
        }),
        serde_json::json!({
            "name": "Live Launch ORV",
            "type": "orv",
            "request": "launch",
            "program": program.clone(),
            "live": true,
        }),
        serde_json::json!({
            "name": "Attach ORV Runtime",
            "type": "orv",
            "request": "attach",
            "program": program,
            "attachRuntimeMode": "inProcess",
        }),
    ]
}

pub(crate) fn editor_debug_controls_json() -> Vec<serde_json::Value> {
    editor_debug_control_order()
        .into_iter()
        .map(|control| {
            serde_json::json!({
                "name": control.label(),
                "request": control.request_json(),
                "runner_command": editor_debug_control_runner_command(control),
            })
        })
        .collect()
}

pub(crate) fn editor_debug_source_inventory_json(files: &[SourceFile]) -> serde_json::Value {
    let sources = editor_dap_sources(files);
    serde_json::json!({
        "schema_version": 1,
        "kind": "orv.editor.debug.source_inventory",
        "protocol": "dap",
        "source_count": sources.len(),
        "loaded_sources_request": editor_debug_loaded_sources_request_json(0),
        "sources": sources
            .iter()
            .map(editor_debug_source_inventory_entry_json)
            .collect::<Vec<_>>(),
    })
}

pub(crate) fn editor_debug_source_inventory_entry_json(
    source: &DapSourceInfo,
) -> serde_json::Value {
    serde_json::json!({
        "source": dap_source_json(source),
        "source_reference": source.reference,
        "path": source.path.display().to_string(),
        "uri": source.uri,
        "checksum": {
            "algorithm": "SHA256",
            "value": source.checksum,
        },
        "request": editor_debug_source_request_json(0, source),
    })
}

pub(crate) fn editor_debug_loaded_sources_request_json(seq: u64) -> serde_json::Value {
    serde_json::json!({
        "seq": seq,
        "type": "request",
        "command": "loadedSources",
        "arguments": {},
    })
}

pub(crate) fn editor_debug_source_request_json(
    seq: u64,
    source: &DapSourceInfo,
) -> serde_json::Value {
    serde_json::json!({
        "seq": seq,
        "type": "request",
        "command": "source",
        "arguments": {
            "sourceReference": source.reference,
            "source": dap_source_json(source),
        },
    })
}

pub(crate) fn editor_debug_breakpoint_sources_json(files: &[SourceFile]) -> Vec<serde_json::Value> {
    files
        .iter()
        .enumerate()
        .map(|(index, file)| {
            let source = dap_source_info(file, u64::try_from(index + 1).unwrap_or(u64::MAX));
            let lines = dap_verified_breakpoint_lines(&file.path).unwrap_or_default();
            let breakpoints = lines
                .iter()
                .map(|line| {
                    serde_json::json!({
                        "line": line,
                        "request": editor_debug_set_breakpoints_request_json(0, &file.path, &[*line]),
                        "runner_command": editor_debug_breakpoint_runner_command(
                            &file.path,
                            *line,
                            EditorDebugControl::Continue,
                        ),
                    })
                })
                .collect::<Vec<_>>();
            serde_json::json!({
                "source": dap_source_json(&source),
                "line_count": lines.len(),
                "lines": lines,
                "breakpoints": breakpoints,
            })
        })
        .collect()
}

pub(crate) fn editor_debug_function_breakpoints_json(
    loaded: &orv_project::LoadedProject,
) -> Vec<serde_json::Value> {
    loaded
        .graph
        .nodes
        .iter()
        .filter(|node| {
            matches!(
                node.kind,
                ProjectNodeKind::Function | ProjectNodeKind::Define
            )
        })
        .map(|node| {
            let line = dap_span_line(node.span, &loaded.files).unwrap_or(0);
            let names = vec![node.name.clone()];
            serde_json::json!({
                "name": &node.name,
                "kind": match node.kind {
                    ProjectNodeKind::Define => "define",
                    _ => "function",
                },
                "source": {
                    "path": loaded
                        .files
                        .iter()
                        .find(|file| file.id == node.file)
                        .map(|file| file.path.display().to_string())
                        .unwrap_or_default(),
                    "line": line,
                },
                "request": editor_debug_set_function_breakpoints_request_json(0, &names),
                "runner_command": editor_debug_function_breakpoint_runner_command(
                    &node.name,
                    EditorDebugControl::Continue,
                ),
            })
        })
        .collect()
}

pub(crate) fn editor_debug_data_breakpoints_json(
    loaded: &orv_project::LoadedProject,
) -> Vec<serde_json::Value> {
    let resolved = orv_resolve::resolve(&loaded.program);
    let lowered = orv_analyzer::lower_with_diagnostics(&loaded.program, &resolved);
    let diagnostic_count =
        loaded.diagnostics.len() + resolved.diagnostics.len() + lowered.diagnostics.len();
    let sources = editor_dap_sources(&loaded.files);
    let (_runtime, frames, _live, _long_running) =
        dap_launch_runtime_state(&lowered, diagnostic_count, &loaded.files, &sources, false);
    let mut locals = BTreeMap::new();
    for frame in frames {
        for local in frame.locals {
            locals
                .entry(local.name.clone())
                .or_insert_with(|| (local, frame.source.clone()));
        }
    }
    locals
        .into_iter()
        .map(|(name, (local, source))| {
            let names = vec![name.clone()];
            let mut source_json = dap_source_json(&source);
            if let Some(source_object) = source_json.as_object_mut() {
                source_object.insert("line".to_string(), serde_json::json!(local.line));
            }
            serde_json::json!({
                "name": name,
                "data_id": format!("local:{}", local.name),
                "value": local.value,
                "type": local.value_type,
                "source": source_json,
                "info_request": editor_debug_data_breakpoint_info_request_json(0, &local.name),
                "request": editor_debug_set_data_breakpoints_request_json(0, &names),
                "runner_command": editor_debug_data_breakpoint_runner_command(
                    &local.name,
                    EditorDebugControl::Continue,
                ),
            })
        })
        .collect()
}

pub(crate) fn editor_debug_exception_filters_json() -> Vec<serde_json::Value> {
    [
        ("orv.diagnostics", "ORV diagnostics"),
        ("orv.runtime", "ORV runtime errors"),
    ]
    .into_iter()
    .map(|(filter, label)| {
        let filters = vec![filter.to_string()];
        serde_json::json!({
            "filter": filter,
            "label": label,
            "default": true,
            "request": editor_debug_set_exception_breakpoints_request_json(0, &filters),
            "runner_command": editor_debug_exception_filter_runner_command(
                filter,
                EditorDebugControl::Continue,
            ),
        })
    })
    .collect()
}

pub(crate) fn editor_debug_breakpoint_count_from_state(state: &serde_json::Value) -> usize {
    state
        .pointer("/debug/breakpoint_sources")
        .and_then(serde_json::Value::as_array)
        .map_or(0, |sources| {
            sources
                .iter()
                .map(|source| json_array_count(source.get("lines")))
                .sum()
        })
}

pub(crate) fn editor_debug_function_breakpoint_count_from_state(
    state: &serde_json::Value,
) -> usize {
    json_array_count(state.pointer("/debug/function_breakpoints"))
}

pub(crate) fn editor_debug_data_breakpoint_count_from_state(state: &serde_json::Value) -> usize {
    json_array_count(state.pointer("/debug/data_breakpoints"))
}

pub(crate) fn editor_debug_exception_filter_count_from_state(state: &serde_json::Value) -> usize {
    json_array_count(state.pointer("/debug/exception_filters"))
}

pub(crate) fn editor_debug_capability_count_from_state(state: &serde_json::Value) -> usize {
    state
        .pointer("/debug/capabilities")
        .and_then(serde_json::Value::as_object)
        .map_or(0, |capabilities| {
            capabilities
                .values()
                .filter(|value| value.as_bool() == Some(true) || value.is_array())
                .count()
        })
}
