#![allow(clippy::redundant_pub_crate, clippy::wildcard_imports)]

use super::*;

pub(crate) fn dap_data_breakpoint_local_name(data_id: &str) -> Option<&str> {
    data_id
        .strip_prefix("local:")
        .map(str::trim)
        .filter(|name| !name.is_empty())
}

pub(crate) fn dap_breakpoint_condition_matches(
    frame: &DapFrameState,
    condition: Option<&str>,
) -> bool {
    let Some(condition) = condition
        .map(str::trim)
        .filter(|condition| !condition.is_empty())
    else {
        return true;
    };
    match condition {
        "true" => return true,
        "false" => return false,
        _ => {}
    }
    for op in ["==", "!=", ">=", "<=", ">", "<"] {
        if let Some((left, right)) = condition.split_once(op) {
            return dap_compare_breakpoint_condition(frame, left.trim(), op, right.trim());
        }
    }
    dap_frame_local_value(frame, condition).is_some_and(dap_condition_value_truthy)
}

pub(crate) fn dap_compare_breakpoint_condition(
    frame: &DapFrameState,
    left: &str,
    op: &str,
    right: &str,
) -> bool {
    let Some(left_value) = dap_frame_local_value(frame, left) else {
        return false;
    };
    if matches!(op, ">" | "<" | ">=" | "<=") {
        let Some(result) = dap_compare_condition_numbers(left_value, op, right) else {
            return false;
        };
        return result;
    }
    let right_value = dap_normalize_condition_literal(right);
    match op {
        "==" => left_value == right_value,
        "!=" => left_value != right_value,
        _ => false,
    }
}

pub(crate) fn dap_set_exception_breakpoints_result(
    request: &serde_json::Value,
) -> serde_json::Value {
    let breakpoints = request
        .pointer("/arguments/filters")
        .and_then(serde_json::Value::as_array)
        .map_or_else(Vec::new, |filters| {
            filters
                .iter()
                .filter_map(serde_json::Value::as_str)
                .map(|filter| {
                    let verified = matches!(filter, "orv.diagnostics" | "orv.runtime");
                    let mut breakpoint = serde_json::json!({
                        "verified": verified,
                        "filter": filter,
                    });
                    if !verified {
                        breakpoint["message"] = serde_json::Value::String(
                            "unsupported ORV exception filter".to_string(),
                        );
                    }
                    breakpoint
                })
                .collect()
        });
    serde_json::json!({
        "breakpoints": breakpoints,
    })
}

pub(crate) fn dap_instruction_breakpoint(
    id: u64,
    instruction_reference: String,
    offset: i64,
    frame_count: Option<usize>,
) -> DapInstructionBreakpoint {
    let frame_index = frame_count.and_then(|count| {
        dap_instruction_breakpoint_frame_index(count, &instruction_reference, offset)
    });
    let verified = frame_index.is_some();
    let message = if verified {
        None
    } else if frame_count.is_none() {
        Some("launch is required before verifying ORV instruction breakpoints".to_string())
    } else {
        Some(format!(
            "unknown ORV instructionReference `{instruction_reference}`"
        ))
    };
    DapInstructionBreakpoint {
        id,
        instruction_reference,
        offset,
        frame_index,
        verified,
        message,
    }
}

pub(crate) fn dap_instruction_breakpoint_frame_index(
    frame_count: usize,
    instruction_reference: &str,
    offset: i64,
) -> Option<usize> {
    let index = dap_disassemble_start_index(instruction_reference, offset).ok()?;
    (index < frame_count).then_some(index)
}

pub(crate) fn dap_instruction_breakpoint_json(
    breakpoint: &DapInstructionBreakpoint,
) -> serde_json::Value {
    let mut value = serde_json::json!({
        "id": breakpoint.id,
        "verified": breakpoint.verified,
        "instructionReference": breakpoint.instruction_reference.as_str(),
        "offset": breakpoint.offset,
    });
    if let Some(message) = &breakpoint.message {
        value["message"] = serde_json::Value::String(message.clone());
    }
    value
}

pub(crate) fn dap_exception_info_json(runtime: &DapRuntimeState) -> serde_json::Value {
    let (exception_id, description, break_mode) = match runtime.status.as_str() {
        "diagnostics" => ("orv.diagnostics", "diagnostics present", "always"),
        "error" => ("orv.runtime", runtime.error.as_str(), "always"),
        _ => ("orv.none", "no exception", "never"),
    };
    serde_json::json!({
        "exceptionId": exception_id,
        "description": description,
        "breakMode": break_mode,
        "details": {
            "message": description,
            "typeName": runtime.status,
            "stackTrace": "",
        },
    })
}

pub(crate) fn dap_breakpoint_locations_json(
    graph: &ProjectGraph,
    files: &[SourceFile],
    file_id: FileId,
    line: u64,
    end_line: u64,
) -> Vec<serde_json::Value> {
    let start_line = line.min(end_line);
    let end_line = line.max(end_line);
    let mut locations = graph
        .nodes
        .iter()
        .filter(|node| node.file == file_id)
        .filter(|node| lsp_selectable_node_kind(node.kind))
        .filter_map(|node| {
            let file = files.iter().find(|file| file.id == node.file)?;
            let start = lsp_byte_position(&file.source, node.span.range.start);
            let line = u64::try_from(start.0 + 1).unwrap_or(u64::MAX);
            let column = u64::try_from(start.1 + 1).unwrap_or(u64::MAX);
            if line < start_line || line > end_line {
                return None;
            }
            Some(serde_json::json!({
                "line": line,
                "column": column,
            }))
        })
        .collect::<Vec<_>>();
    locations.sort_by_key(|location| {
        (
            location
                .get("line")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(u64::MAX),
            location
                .get("column")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(u64::MAX),
        )
    });
    locations
        .dedup_by(|left, right| left["line"] == right["line"] && left["column"] == right["column"]);
    locations
}

pub(crate) fn dap_verified_breakpoint_lines(path: &Path) -> anyhow::Result<Vec<u64>> {
    let loaded = orv_project::load_project(path).map_err(|e| anyhow::anyhow!("{e}"))?;
    let file = lsp_source_file_for_path(&loaded.files, path)
        .ok_or_else(|| anyhow::anyhow!("breakpoint source is not part of loaded project"))?;
    let mut lines = loaded
        .graph
        .nodes
        .iter()
        .filter(|node| node.file == file.id)
        .filter(|node| lsp_selectable_node_kind(node.kind))
        .filter_map(|node| {
            let file = loaded.files.iter().find(|file| file.id == node.file)?;
            let start = lsp_byte_position(&file.source, node.span.range.start);
            Some(u64::try_from(start.0 + 1).unwrap_or(u64::MAX))
        })
        .collect::<Vec<_>>();
    for stmt in &loaded.program.items {
        dap_collect_stmt_breakpoint_lines(stmt, file.id, &loaded.files, &mut lines);
    }
    lines.sort_unstable();
    lines.dedup();
    Ok(lines)
}

pub(crate) fn dap_collect_stmt_breakpoint_lines(
    stmt: &Stmt,
    file_id: FileId,
    files: &[SourceFile],
    lines: &mut Vec<u64>,
) {
    dap_push_span_line(stmt.span(), file_id, files, lines);
    match stmt {
        Stmt::Let(stmt) => dap_collect_expr_breakpoint_lines(&stmt.init, file_id, files, lines),
        Stmt::Const(stmt) => dap_collect_expr_breakpoint_lines(&stmt.init, file_id, files, lines),
        Stmt::Function(stmt) => {
            dap_collect_function_body_breakpoint_lines(&stmt.body, file_id, files, lines);
        }
        Stmt::Enum(stmt) => {
            for variant in &stmt.variants {
                dap_collect_expr_breakpoint_lines(&variant.value, file_id, files, lines);
            }
        }
        Stmt::Return(stmt) => {
            if let Some(value) = &stmt.value {
                dap_collect_expr_breakpoint_lines(value, file_id, files, lines);
            }
        }
        Stmt::Expr(expr) => dap_collect_expr_breakpoint_lines(expr, file_id, files, lines),
        Stmt::Struct(_) | Stmt::TypeAlias(_) | Stmt::Import(_) => {}
    }
}

pub(crate) fn dap_collect_function_body_breakpoint_lines(
    body: &FunctionBody,
    file_id: FileId,
    files: &[SourceFile],
    lines: &mut Vec<u64>,
) {
    match body {
        FunctionBody::Block(block) => {
            dap_collect_block_breakpoint_lines(block, file_id, files, lines);
        }
        FunctionBody::Expr(expr) => dap_collect_expr_breakpoint_lines(expr, file_id, files, lines),
    }
}

pub(crate) fn dap_collect_block_breakpoint_lines(
    block: &Block,
    file_id: FileId,
    files: &[SourceFile],
    lines: &mut Vec<u64>,
) {
    for stmt in &block.stmts {
        dap_collect_stmt_breakpoint_lines(stmt, file_id, files, lines);
    }
}

pub(crate) fn dap_collect_expr_breakpoint_lines(
    expr: &Expr,
    file_id: FileId,
    files: &[SourceFile],
    lines: &mut Vec<u64>,
) {
    dap_push_span_line(expr.span, file_id, files, lines);
    match &expr.kind {
        ExprKind::Unary { expr, .. }
        | ExprKind::Paren(expr)
        | ExprKind::Await(expr)
        | ExprKind::Throw(expr)
        | ExprKind::Cast { expr, .. } => {
            dap_collect_expr_breakpoint_lines(expr, file_id, files, lines);
        }
        ExprKind::Binary { lhs, rhs, .. } => {
            dap_collect_expr_breakpoint_lines(lhs, file_id, files, lines);
            dap_collect_expr_breakpoint_lines(rhs, file_id, files, lines);
        }
        ExprKind::Domain { args, .. } | ExprKind::Tuple(args) | ExprKind::Array(args) => {
            for arg in args {
                dap_collect_expr_breakpoint_lines(arg, file_id, files, lines);
            }
        }
        ExprKind::Block(block) => dap_collect_block_breakpoint_lines(block, file_id, files, lines),
        ExprKind::If {
            cond,
            then,
            else_branch,
        } => {
            dap_collect_expr_breakpoint_lines(cond, file_id, files, lines);
            dap_collect_block_breakpoint_lines(then, file_id, files, lines);
            if let Some(else_branch) = else_branch {
                dap_collect_expr_breakpoint_lines(else_branch, file_id, files, lines);
            }
        }
        ExprKind::When { scrutinee, arms } => {
            dap_collect_expr_breakpoint_lines(scrutinee, file_id, files, lines);
            for arm in arms {
                dap_collect_expr_breakpoint_lines(&arm.body, file_id, files, lines);
            }
        }
        ExprKind::Assign { value, .. } => {
            dap_collect_expr_breakpoint_lines(value, file_id, files, lines);
        }
        ExprKind::Call { callee, args } => {
            dap_collect_expr_breakpoint_lines(callee, file_id, files, lines);
            for arg in args {
                dap_collect_expr_breakpoint_lines(arg, file_id, files, lines);
            }
        }
        ExprKind::AssignField { object, value, .. } => {
            dap_collect_expr_breakpoint_lines(object, file_id, files, lines);
            dap_collect_expr_breakpoint_lines(value, file_id, files, lines);
        }
        ExprKind::AssignIndex {
            object,
            index,
            value,
        } => {
            dap_collect_expr_breakpoint_lines(object, file_id, files, lines);
            dap_collect_expr_breakpoint_lines(index, file_id, files, lines);
            dap_collect_expr_breakpoint_lines(value, file_id, files, lines);
        }
        ExprKind::For { iter, body, .. } => {
            dap_collect_expr_breakpoint_lines(iter, file_id, files, lines);
            dap_collect_block_breakpoint_lines(body, file_id, files, lines);
        }
        ExprKind::While { cond, body } => {
            dap_collect_expr_breakpoint_lines(cond, file_id, files, lines);
            dap_collect_block_breakpoint_lines(body, file_id, files, lines);
        }
        ExprKind::Range { start, end, .. } => {
            dap_collect_expr_breakpoint_lines(start, file_id, files, lines);
            dap_collect_expr_breakpoint_lines(end, file_id, files, lines);
        }
        ExprKind::Object(fields) | ExprKind::TypedObject { fields, .. } => {
            for field in fields {
                dap_collect_expr_breakpoint_lines(&field.value, file_id, files, lines);
            }
        }
        ExprKind::Index { target, index } => {
            dap_collect_expr_breakpoint_lines(target, file_id, files, lines);
            dap_collect_expr_breakpoint_lines(index, file_id, files, lines);
        }
        ExprKind::Slice { target, start, end } => {
            dap_collect_expr_breakpoint_lines(target, file_id, files, lines);
            if let Some(start) = start {
                dap_collect_expr_breakpoint_lines(start, file_id, files, lines);
            }
            if let Some(end) = end {
                dap_collect_expr_breakpoint_lines(end, file_id, files, lines);
            }
        }
        ExprKind::Field { target, .. } | ExprKind::OptionalField { target, .. } => {
            dap_collect_expr_breakpoint_lines(target, file_id, files, lines);
        }
        ExprKind::Lambda { body, .. } => {
            dap_collect_function_body_breakpoint_lines(body, file_id, files, lines);
        }
        ExprKind::Try { try_block, catch } => {
            dap_collect_block_breakpoint_lines(try_block, file_id, files, lines);
            if let Some(catch) = catch {
                dap_collect_block_breakpoint_lines(&catch.body, file_id, files, lines);
            }
        }
        ExprKind::Integer(_)
        | ExprKind::Float(_)
        | ExprKind::String(_)
        | ExprKind::Regex { .. }
        | ExprKind::True
        | ExprKind::False
        | ExprKind::Void
        | ExprKind::Ident(_)
        | ExprKind::TypeName(_)
        | ExprKind::Break
        | ExprKind::Continue => {}
    }
}

pub(crate) fn dap_breakpoint_source_path(
    launched: Option<&DapLaunchState>,
    request: &serde_json::Value,
) -> anyhow::Result<PathBuf> {
    if let Some(reference) = request
        .pointer("/arguments/source/sourceReference")
        .and_then(serde_json::Value::as_u64)
        .filter(|reference| *reference > 0)
    {
        let launched = launched
            .ok_or_else(|| anyhow::anyhow!("launch is required before sourceReference lookup"))?;
        return launched
            .sources
            .iter()
            .find(|source| source.reference == reference)
            .map(|source| source.path.clone())
            .ok_or_else(|| anyhow::anyhow!("unknown sourceReference {reference}"));
    }
    let path = request
        .pointer("/arguments/source/path")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("source.path must be a path or file URI"))?;
    dap_path_from_protocol_string(path)
}
