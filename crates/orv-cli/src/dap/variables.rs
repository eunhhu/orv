#![allow(clippy::redundant_pub_crate, clippy::wildcard_imports)]

use super::*;

pub(crate) fn dap_env_variable(expr: &orv_hir::HirExpr) -> Option<String> {
    let orv_hir::HirExprKind::Field { target, field, .. } = &expr.kind else {
        return None;
    };
    let orv_hir::HirExprKind::Domain { name, args, .. } = &target.kind else {
        return None;
    };
    (name == "env" && args.is_empty()).then(|| field.clone())
}

pub(crate) fn dap_project_variables(launched: &DapLaunchState) -> Vec<serde_json::Value> {
    let mut variables = vec![
        serde_json::json!({
            "name": "entry",
            "value": launched.path.display().to_string(),
            "type": "source",
            "variablesReference": 0,
        }),
        serde_json::json!({
            "name": "projectGraphNodes",
            "value": launched.node_count.to_string(),
            "type": "usize",
            "variablesReference": 0,
        }),
        serde_json::json!({
            "name": "diagnostics",
            "value": launched.diagnostic_count.to_string(),
            "type": "usize",
            "variablesReference": 0,
        }),
        serde_json::json!({
            "name": "runtimeStatus",
            "value": launched.runtime.status,
            "type": "string",
            "variablesReference": 0,
        }),
        serde_json::json!({
            "name": "stdout",
            "value": launched.runtime.stdout,
            "type": "string",
            "variablesReference": 0,
        }),
        serde_json::json!({
            "name": "runtimeError",
            "value": launched.runtime.error,
            "type": "string",
            "variablesReference": 0,
        }),
    ];
    if let Some(async_runtime) = &launched.async_runtime {
        variables.extend(dap_async_runtime_variables(launched, async_runtime));
    }
    variables
}

pub(crate) fn dap_runtime_request_variables(launched: &DapLaunchState) -> Vec<serde_json::Value> {
    let request_frames = dap_runtime_request_frames(launched);
    let mut variables = vec![
        serde_json::json!({
            "name": "runtimeRequestCount",
            "value": request_frames.len().to_string(),
            "type": "usize",
            "variablesReference": 0,
        }),
        serde_json::json!({
            "name": "runtimeLastRequest",
            "value": request_frames
                .last()
                .map_or_else(String::new, dap_server_request_frame_display),
            "type": "string",
            "variablesReference": 0,
        }),
        serde_json::json!({
            "name": "runtimeRequestFrames",
            "value": dap_server_request_frames_display(&request_frames),
            "type": "string",
            "variablesReference": 0,
        }),
        serde_json::json!({
            "name": "runtimeRequestTrace",
            "value": dap_server_request_trace_display(&request_frames),
            "type": "json",
            "variablesReference": 0,
        }),
    ];
    if let Some(path) = &launched.runtime_request_trace_path {
        variables.push(serde_json::json!({
            "name": "runtimeRequestTracePath",
            "value": path.display().to_string(),
            "type": "path",
            "variablesReference": 0,
        }));
    }
    variables
}

pub(crate) fn dap_runtime_variable(
    variable: &orv_runtime::DebugVariable,
    line: u64,
) -> DapVariable {
    let (value, value_type) = dap_runtime_value_display(&variable.value);
    DapVariable {
        name: variable.name.clone(),
        value,
        value_type,
        line,
        variables_reference: 0,
    }
}

pub(crate) fn dap_runtime_value_display(value: &orv_runtime::Value) -> (String, String) {
    match value {
        orv_runtime::Value::Int(value) => (value.to_string(), "int".to_string()),
        orv_runtime::Value::Float(value) => (value.to_string(), "float".to_string()),
        orv_runtime::Value::Str(value) => (
            serde_json::to_string(value).unwrap_or_else(|_| format!("\"{value}\"")),
            "string".to_string(),
        ),
        orv_runtime::Value::Regex { pattern, flags } => {
            (format!("r\"{pattern}\"{flags}"), "regex".to_string())
        }
        orv_runtime::Value::Bool(value) => (value.to_string(), "bool".to_string()),
        orv_runtime::Value::Void => ("void".to_string(), "void".to_string()),
        orv_runtime::Value::Array(items) => {
            let items = items
                .iter()
                .map(|item| dap_runtime_value_display(item).0)
                .collect::<Vec<_>>()
                .join(", ");
            (format!("[{items}]"), "array".to_string())
        }
        orv_runtime::Value::Tuple(items) => {
            let items = items
                .iter()
                .map(|item| dap_runtime_value_display(item).0)
                .collect::<Vec<_>>()
                .join(", ");
            (format!("({items})"), "tuple".to_string())
        }
        orv_runtime::Value::Object(fields) => {
            let fields = fields
                .iter()
                .map(|(name, value)| {
                    let (value, _) = dap_runtime_value_display(value);
                    format!("{name}: {value}")
                })
                .collect::<Vec<_>>()
                .join(", ");
            (format!("{{ {fields} }}"), "object".to_string())
        }
        orv_runtime::Value::Function(_)
        | orv_runtime::Value::Lambda(_)
        | orv_runtime::Value::BoundMethod { .. }
        | orv_runtime::Value::Db(_)
        | orv_runtime::Value::TypeName(_)
        | orv_runtime::Value::Builtin(_) => (value.to_string(), "runtime".to_string()),
    }
}

pub(crate) fn dap_non_current_scopes_result(
    launched: &DapLaunchState,
    frame_id: u64,
) -> anyhow::Result<serde_json::Value> {
    let frame = dap_stack_scope_frame(launched, frame_id)
        .ok_or_else(|| anyhow::anyhow!("unknown ORV frameId {frame_id}"))?;
    Ok(serde_json::json!({
        "scopes": [
            {
                "name": frame.name,
                "variablesReference": 0,
                "namedVariables": 0,
                "expensive": false,
                "source": dap_source_json_with_reference(&frame.source, 0),
                "line": frame.line,
                "column": 1,
            },
        ],
    }))
}

pub(crate) fn dap_stack_scope_frame(
    launched: &DapLaunchState,
    frame_id: u64,
) -> Option<DapScopeFrame> {
    if frame_id <= 1 {
        return None;
    }
    if let Some(stack_frame) = dap_stack_call_for_frame_id(launched, frame_id) {
        return Some(DapScopeFrame {
            name: format!("Frame {}", stack_frame.name),
            source: stack_frame.source,
            line: stack_frame.line,
        });
    }
    (dap_stack_entry_frame_id(launched) == Some(frame_id)).then(|| DapScopeFrame {
        name: "Frame orv entry".to_string(),
        source: dap_entry_source(launched),
        line: 1,
    })
}

pub(crate) fn dap_stack_frames_json(launched: &DapLaunchState) -> Vec<serde_json::Value> {
    let current_frame = launched.frames.get(launched.current_frame_index);
    let (current_source, line) = dap_current_source_and_line(launched);
    let current_name = current_frame
        .and_then(|frame| frame.stack.last())
        .map_or_else(|| "orv entry".to_string(), |frame| frame.name.clone());
    let mut frames = vec![dap_stack_frame_json(
        1,
        &current_name,
        &current_source,
        line,
    )];
    if let Some(current_frame) = current_frame {
        for (index, stack_frame) in current_frame.stack.iter().rev().skip(1).enumerate() {
            frames.push(dap_stack_frame_json(
                u64::try_from(index + 2).unwrap_or(u64::MAX),
                &stack_frame.name,
                &stack_frame.source,
                stack_frame.line,
            ));
        }
        if !current_frame.stack.is_empty() {
            let entry_source = dap_entry_source(launched);
            frames.push(dap_stack_frame_json(
                u64::try_from(frames.len() + 1).unwrap_or(u64::MAX),
                "orv entry",
                &entry_source,
                1,
            ));
        }
    }
    frames
}

pub(crate) fn dap_paginate_json_values(
    values: Vec<serde_json::Value>,
    request: &serde_json::Value,
    start_name: &str,
    count_name: &str,
) -> Vec<serde_json::Value> {
    let total = values.len();
    let start = dap_usize_argument(request, start_name)
        .unwrap_or(0)
        .min(total);
    let count =
        dap_usize_argument(request, count_name).unwrap_or_else(|| total.saturating_sub(start));
    values.into_iter().skip(start).take(count).collect()
}

pub(crate) fn dap_filter_and_paginate_variables(
    values: Vec<serde_json::Value>,
    request: &serde_json::Value,
) -> Vec<serde_json::Value> {
    if dap_str_argument(request, "filter") == Some("indexed") {
        return Vec::new();
    }
    dap_paginate_json_values(values, request, "start", "count")
}

pub(crate) fn dap_stack_frame_json(
    id: u64,
    name: &str,
    source: &DapSourceInfo,
    line: u64,
) -> serde_json::Value {
    serde_json::json!({
        "id": id,
        "name": name,
        "source": dap_source_json_with_reference(source, 0),
        "line": line,
        "column": 1,
    })
}

pub(crate) fn dap_frame_local_value<'a>(frame: &'a DapFrameState, name: &str) -> Option<&'a str> {
    frame
        .locals
        .iter()
        .find(|local| local.name == name)
        .map(|local| local.value.as_str())
}

pub(crate) fn dap_condition_value_truthy(value: &str) -> bool {
    !matches!(value, "" | "false" | "0" | "0.0" | "void" | "\"\"")
}

pub(crate) fn dap_variable_json(variable: &DapVariable) -> serde_json::Value {
    serde_json::json!({
        "name": variable.name,
        "value": variable.value,
        "type": variable.value_type,
        "variablesReference": variable.variables_reference,
    })
}

pub(crate) fn dap_set_value_json(variable: &DapVariable) -> serde_json::Value {
    serde_json::json!({
        "value": variable.value,
        "type": variable.value_type,
        "variablesReference": variable.variables_reference,
    })
}

pub(crate) fn dap_evaluate_project_value(
    launched: &DapLaunchState,
    expression: &str,
) -> Option<(String, String)> {
    if let Some(local) = dap_current_locals(launched)
        .iter()
        .find(|local| local.name == expression)
    {
        return Some((local.value.clone(), local.value_type.clone()));
    }
    match expression {
        "entry" => Some((launched.path.display().to_string(), "source".to_string())),
        "projectGraphNodes" => Some((launched.node_count.to_string(), "usize".to_string())),
        "diagnostics" => Some((launched.diagnostic_count.to_string(), "usize".to_string())),
        "runtimeStatus" => Some((launched.runtime.status.clone(), "string".to_string())),
        "stdout" => Some((launched.runtime.stdout.clone(), "string".to_string())),
        "runtimeError" => Some((launched.runtime.error.clone(), "string".to_string())),
        _ => dap_evaluate_async_runtime_value(launched, expression),
    }
}
