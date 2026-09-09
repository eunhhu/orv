#![allow(clippy::redundant_pub_crate, clippy::wildcard_imports)]

use super::*;

pub(crate) fn dap_send_process_signal(pid: u32, signal: &str) -> anyhow::Result<()> {
    let status = ProcessCommand::new("kill")
        .arg(format!("-{signal}"))
        .arg(pid.to_string())
        .status()
        .map_err(|e| anyhow::anyhow!("failed to signal runtime process {pid}: {e}"))?;
    if !status.success() {
        anyhow::bail!("failed to signal runtime process {pid} with {signal}: {status}");
    }
    Ok(())
}

pub(crate) fn dap_attach_runtime_transport_if_requested(
    async_runtime: &mut Option<DapAsyncRuntimeState>,
    attach_runtime_requested: bool,
    attach_runtime_mode: DapRuntimeAttachMode,
) {
    if !attach_runtime_requested {
        return;
    }
    let Some(async_runtime) = async_runtime.as_mut() else {
        return;
    };
    async_runtime.transport = Some(match attach_runtime_mode {
        DapRuntimeAttachMode::Process => DapAsyncTransportState::process_detached(),
        DapRuntimeAttachMode::InProcess => DapAsyncTransportState::in_process_detached(),
    });
}

pub(crate) fn dap_runtime_state(
    lowered: &orv_analyzer::LowerResult,
    diagnostic_count: usize,
    files: &[SourceFile],
    sources: &[DapSourceInfo],
) -> (DapRuntimeState, Vec<DapFrameState>) {
    if diagnostic_count > 0 {
        return (
            DapRuntimeState {
                status: "diagnostics".to_string(),
                stdout: String::new(),
                error: "diagnostics present".to_string(),
            },
            Vec::new(),
        );
    }
    let mut stdout = Vec::new();
    let (debug, result) = orv_runtime::run_with_debug(&lowered.program, &mut stdout);
    let runtime = match result {
        Ok(()) => DapRuntimeState {
            status: "ok".to_string(),
            stdout: String::from_utf8_lossy(&stdout).into_owned(),
            error: String::new(),
        },
        Err(err) => DapRuntimeState {
            status: "error".to_string(),
            stdout: String::from_utf8_lossy(&stdout).into_owned(),
            error: err.to_string(),
        },
    };
    (
        runtime,
        dap_runtime_frames(debug.frames.as_slice(), files, sources),
    )
}

pub(crate) fn dap_live_runtime_state(
    lowered: &orv_analyzer::LowerResult,
    files: &[SourceFile],
    sources: &[DapSourceInfo],
) -> (DapRuntimeState, Vec<DapFrameState>, Option<DapLiveState>) {
    let mut stepper = orv_runtime::DebugStepper::new(lowered.program.clone(), Vec::new());
    let mut runtime = DapRuntimeState {
        status: "running".to_string(),
        stdout: String::new(),
        error: String::new(),
    };
    match stepper.step() {
        Ok(Some(debug_frame)) => {
            let frames = dap_runtime_frames(&[debug_frame], files, sources);
            for frame in &frames {
                runtime.stdout.push_str(&frame.output);
            }
            (runtime, frames, Some(DapLiveState { stepper }))
        }
        Ok(None) => {
            runtime.status = "ok".to_string();
            (runtime, Vec::new(), None)
        }
        Err(err) => {
            runtime.status = "error".to_string();
            runtime.error = err.to_string();
            (runtime, Vec::new(), None)
        }
    }
}

pub(crate) fn dap_program_has_long_running_runtime(program: &orv_hir::HirProgram) -> bool {
    program.items.iter().any(dap_stmt_has_long_running_runtime)
}

pub(crate) const fn dap_stmt_has_long_running_runtime(stmt: &orv_hir::HirStmt) -> bool {
    match stmt {
        orv_hir::HirStmt::Expr(expr) => dap_expr_has_long_running_runtime(expr),
        _ => false,
    }
}

pub(crate) const fn dap_expr_has_long_running_runtime(expr: &orv_hir::HirExpr) -> bool {
    matches!(expr.kind, orv_hir::HirExprKind::Server { .. })
}

pub(crate) fn dap_long_running_runtime_state(
    program: &orv_hir::HirProgram,
    files: &[SourceFile],
    sources: &[DapSourceInfo],
) -> (DapRuntimeState, Vec<DapFrameState>) {
    let frames = program
        .items
        .iter()
        .filter(|stmt| dap_stmt_has_long_running_runtime(stmt))
        .filter_map(|stmt| dap_long_running_frame(stmt.span(), files, sources))
        .collect::<Vec<_>>();
    (
        DapRuntimeState {
            status: "paused".to_string(),
            stdout: String::new(),
            error: String::new(),
        },
        frames,
    )
}

pub(crate) fn dap_runtime_json(
    runtime: &DapRuntimeState,
    async_runtime: Option<&DapAsyncRuntimeState>,
) -> serde_json::Value {
    let mut value = serde_json::json!({
        "status": runtime.status,
        "stdout": runtime.stdout,
        "error": runtime.error,
    });
    if let Some(async_runtime) = async_runtime {
        value["async"] = serde_json::json!({
            "kind": async_runtime.kind,
            "state": async_runtime.state,
            "resume_count": async_runtime.resume_count,
            "pause_count": async_runtime.pause_count,
            "listen": async_runtime.listen.as_ref().map(dap_async_listen_json),
            "route_count": async_runtime.routes.len(),
            "routes": async_runtime.routes.iter().map(dap_async_route_json).collect::<Vec<_>>(),
            "transport": async_runtime.transport.as_ref().map(dap_async_transport_json),
        });
    }
    value
}

pub(crate) fn dap_runtime_request_frames(
    launched: &DapLaunchState,
) -> Vec<orv_runtime::server::ServerRequestFrame> {
    launched.attached_server.as_ref().map_or_else(
        Vec::new,
        orv_runtime::server::AttachedServer::request_frames,
    )
}

pub(crate) fn dap_runtime_frames(
    frames: &[orv_runtime::DebugFrame],
    files: &[SourceFile],
    sources: &[DapSourceInfo],
) -> Vec<DapFrameState> {
    frames
        .iter()
        .filter_map(|frame| {
            let source = dap_source_for_span(frame.span, files, sources)?;
            let line = dap_span_line(frame.span, files)?;
            let locals = frame
                .locals
                .iter()
                .map(|variable| dap_runtime_variable(variable, line))
                .collect();
            let stack = frame
                .stack
                .iter()
                .filter_map(|stack_frame| {
                    Some(DapStackFrameState {
                        name: stack_frame.name.clone(),
                        source: dap_source_for_span(stack_frame.span, files, sources)?,
                        line: dap_span_line(stack_frame.span, files)?,
                    })
                })
                .collect();
            Some(DapFrameState {
                source,
                line,
                locals,
                stack,
                output: frame.output.clone(),
            })
        })
        .collect()
}
