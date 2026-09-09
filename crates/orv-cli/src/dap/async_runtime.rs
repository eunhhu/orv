#![allow(clippy::redundant_pub_crate, clippy::wildcard_imports)]

use super::*;

pub(crate) fn dap_async_runtime_state(
    program: &orv_hir::HirProgram,
    long_running: bool,
) -> Option<DapAsyncRuntimeState> {
    long_running.then(|| {
        DapAsyncRuntimeState::server(
            dap_async_server_listen(program),
            dap_async_server_routes(program),
        )
    })
}

pub(crate) fn dap_async_server_listen(
    program: &orv_hir::HirProgram,
) -> Option<DapAsyncListenState> {
    program.items.iter().find_map(|stmt| match stmt {
        orv_hir::HirStmt::Expr(expr) => dap_expr_async_server_listen(expr),
        _ => None,
    })
}

pub(crate) fn dap_expr_async_server_listen(expr: &orv_hir::HirExpr) -> Option<DapAsyncListenState> {
    let orv_hir::HirExprKind::Server { listen, .. } = &expr.kind else {
        return None;
    };
    let listen = listen.as_ref()?;
    if let Some(listen) = dap_async_env_listen(listen) {
        return Some(listen);
    }
    match &listen.kind {
        orv_hir::HirExprKind::Integer(value) => Some(DapAsyncListenState {
            kind: "static".to_string(),
            display: value.clone(),
            port: value.parse::<u64>().ok(),
            variable: None,
            default_port: None,
        }),
        _ => Some(DapAsyncListenState {
            kind: "expression".to_string(),
            display: "<expression>".to_string(),
            port: None,
            variable: None,
            default_port: None,
        }),
    }
}

pub(crate) fn dap_async_env_listen(expr: &orv_hir::HirExpr) -> Option<DapAsyncListenState> {
    let orv_hir::HirExprKind::Call { callee, args } = &expr.kind else {
        return None;
    };
    if dap_hir_call_name(callee) != "int.from" || args.len() != 1 {
        return None;
    }
    let arg = args.first()?;
    let (env_expr, default_port) = match &arg.kind {
        orv_hir::HirExprKind::Binary {
            op: orv_hir::BinaryOp::Coalesce,
            lhs,
            rhs,
        } => (lhs.as_ref(), dap_string_port(rhs.as_ref())),
        _ => (arg, None),
    };
    let variable = dap_env_variable(env_expr)?;
    let display = default_port.map_or_else(
        || variable.clone(),
        |port| format!("{variable} default {port}"),
    );
    Some(DapAsyncListenState {
        kind: "env".to_string(),
        display,
        port: default_port,
        variable: Some(variable),
        default_port,
    })
}

pub(crate) fn dap_async_server_routes(program: &orv_hir::HirProgram) -> Vec<DapAsyncRouteState> {
    program
        .items
        .iter()
        .flat_map(|stmt| match stmt {
            orv_hir::HirStmt::Expr(expr) => dap_expr_async_server_routes(expr),
            _ => Vec::new(),
        })
        .collect()
}

pub(crate) fn dap_expr_async_server_routes(expr: &orv_hir::HirExpr) -> Vec<DapAsyncRouteState> {
    let orv_hir::HirExprKind::Server { routes, .. } = &expr.kind else {
        return Vec::new();
    };
    routes
        .iter()
        .filter_map(|route| {
            let orv_hir::HirExprKind::Route { method, path, .. } = &route.kind else {
                return None;
            };
            Some(DapAsyncRouteState {
                method: method.clone(),
                path: path.clone(),
            })
        })
        .collect()
}

pub(crate) fn dap_async_listen_json(listen: &DapAsyncListenState) -> serde_json::Value {
    let mut value = serde_json::json!({
        "kind": listen.kind,
        "display": listen.display,
    });
    if let Some(port) = listen.port {
        value["port"] = serde_json::json!(port);
    }
    if let Some(variable) = &listen.variable {
        value["variable"] = serde_json::json!(variable);
    }
    if let Some(default_port) = listen.default_port {
        value["default_port"] = serde_json::json!(default_port);
    }
    value
}

pub(crate) fn dap_async_route_json(route: &DapAsyncRouteState) -> serde_json::Value {
    serde_json::json!({
        "method": route.method,
        "path": route.path,
    })
}

pub(crate) fn dap_async_transport_json(transport: &DapAsyncTransportState) -> serde_json::Value {
    let mut value = serde_json::json!({
        "kind": transport.kind,
        "state": transport.state,
    });
    if let Some(process_id) = transport.process_id {
        value["process_id"] = serde_json::json!(process_id);
    }
    if let Some(address) = &transport.address {
        value["address"] = serde_json::json!(address);
    }
    value
}

pub(crate) fn dap_async_routes_display(routes: &[DapAsyncRouteState]) -> String {
    routes
        .iter()
        .map(|route| format!("{} {}", route.method, route.path))
        .collect::<Vec<_>>()
        .join(", ")
}

pub(crate) fn dap_async_transport_display(transport: &DapAsyncTransportState) -> String {
    if let Some(address) = &transport.address {
        return format!("{} {} {address}", transport.kind, transport.state);
    }
    if let Some(pid) = transport.process_id {
        return format!("{} {} pid {pid}", transport.kind, transport.state);
    }
    format!("{} {}", transport.kind, transport.state)
}

pub(crate) fn dap_async_runtime_variables(
    launched: &DapLaunchState,
    async_runtime: &DapAsyncRuntimeState,
) -> Vec<serde_json::Value> {
    let mut variables = vec![
        serde_json::json!({
            "name": "runtimeKind",
            "value": async_runtime.kind,
            "type": "string",
            "variablesReference": 0,
        }),
        serde_json::json!({
            "name": "runtimeAsyncState",
            "value": async_runtime.state,
            "type": "string",
            "variablesReference": 0,
        }),
        serde_json::json!({
            "name": "runtimeResumeCount",
            "value": async_runtime.resume_count.to_string(),
            "type": "usize",
            "variablesReference": 0,
        }),
        serde_json::json!({
            "name": "runtimePauseCount",
            "value": async_runtime.pause_count.to_string(),
            "type": "usize",
            "variablesReference": 0,
        }),
        serde_json::json!({
            "name": "runtimeRouteCount",
            "value": async_runtime.routes.len().to_string(),
            "type": "usize",
            "variablesReference": 0,
        }),
        serde_json::json!({
            "name": "runtimeRoutes",
            "value": dap_async_routes_display(&async_runtime.routes),
            "type": "string",
            "variablesReference": 0,
        }),
    ];
    variables.extend(dap_runtime_request_variables(launched));
    if let Some(listen) = &async_runtime.listen {
        variables.extend([
            serde_json::json!({
                "name": "runtimeListen",
                "value": listen.display,
                "type": "string",
                "variablesReference": 0,
            }),
            serde_json::json!({
                "name": "runtimeListenPort",
                "value": listen.port.map_or_else(String::new, |port| port.to_string()),
                "type": "usize",
                "variablesReference": 0,
            }),
        ]);
    }
    if let Some(transport) = &async_runtime.transport {
        variables.extend([
            serde_json::json!({
                "name": "runtimeTransport",
                "value": dap_async_transport_display(transport),
                "type": "string",
                "variablesReference": 0,
            }),
            serde_json::json!({
                "name": "runtimeProcessId",
                "value": transport.process_id.map_or_else(String::new, |pid| pid.to_string()),
                "type": "usize",
                "variablesReference": 0,
            }),
        ]);
    }
    variables
}

pub(crate) fn dap_evaluate_async_runtime_value(
    launched: &DapLaunchState,
    expression: &str,
) -> Option<(String, String)> {
    let runtime = launched.async_runtime.as_ref()?;
    match expression {
        "runtimeKind" => Some((runtime.kind.clone(), "string".to_string())),
        "runtimeAsyncState" => Some((runtime.state.clone(), "string".to_string())),
        "runtimeResumeCount" => Some((runtime.resume_count.to_string(), "usize".to_string())),
        "runtimePauseCount" => Some((runtime.pause_count.to_string(), "usize".to_string())),
        "runtimeRouteCount" => Some((runtime.routes.len().to_string(), "usize".to_string())),
        "runtimeRoutes" => Some((
            dap_async_routes_display(&runtime.routes),
            "string".to_string(),
        )),
        "runtimeRequestCount" => Some((
            dap_runtime_request_frames(launched).len().to_string(),
            "usize".to_string(),
        )),
        "runtimeLastRequest" => {
            let frames = dap_runtime_request_frames(launched);
            Some((
                frames
                    .last()
                    .map_or_else(String::new, dap_server_request_frame_display),
                "string".to_string(),
            ))
        }
        "runtimeRequestFrames" => Some((
            dap_server_request_frames_display(&dap_runtime_request_frames(launched)),
            "string".to_string(),
        )),
        "runtimeRequestTrace" => Some((
            dap_server_request_trace_display(&dap_runtime_request_frames(launched)),
            "json".to_string(),
        )),
        "runtimeRequestTracePath" => launched
            .runtime_request_trace_path
            .as_ref()
            .map(|path| (path.display().to_string(), "path".to_string())),
        "runtimeListen" => runtime
            .listen
            .as_ref()
            .map(|listen| (listen.display.clone(), "string".to_string())),
        "runtimeListenPort" => runtime.listen.as_ref().map(|listen| {
            (
                listen
                    .port
                    .map_or_else(String::new, |port| port.to_string()),
                "usize".to_string(),
            )
        }),
        "runtimeTransport" => runtime
            .transport
            .as_ref()
            .map(|transport| (dap_async_transport_display(transport), "string".to_string())),
        "runtimeProcessId" => runtime.transport.as_ref().map(|transport| {
            (
                transport
                    .process_id
                    .map_or_else(String::new, |pid| pid.to_string()),
                "usize".to_string(),
            )
        }),
        _ => None,
    }
}
