#![allow(clippy::redundant_pub_crate, clippy::wildcard_imports)]

use super::*;

pub(crate) fn lsp_snapshot_json(path: &Path) -> anyhow::Result<serde_json::Value> {
    let loaded = orv_project::load_project(path).map_err(|e| anyhow::anyhow!("{e}"))?;
    let resolved = orv_resolve::resolve(&loaded.program);
    let lowered = orv_analyzer::lower_with_diagnostics(&loaded.program, &resolved);
    let origin_map = orv_compiler::origin_map(&lowered.program);
    let mut diagnostics = Vec::new();
    diagnostics.extend(lsp_diagnostics_json(&loaded.diagnostics, &loaded.files));
    diagnostics.extend(lsp_diagnostics_json(&resolved.diagnostics, &loaded.files));
    diagnostics.extend(lsp_diagnostics_json(&lowered.diagnostics, &loaded.files));
    Ok(serde_json::json!({
        "schema_version": 1,
        "uri": path.display().to_string(),
        "diagnostics": diagnostics,
        "project_graph": project_graph_json(&loaded.graph, &origin_map),
        "document_symbols": lsp_document_symbols_json(&loaded.graph, &loaded.files),
    }))
}

pub(crate) fn lsp_reveal_json(dir: &Path, origin_id: &str) -> anyhow::Result<serde_json::Value> {
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
    Ok(serde_json::json!({
        "schema_version": 1,
        "origin": reveal.get("origin").cloned().unwrap_or(serde_json::Value::Null),
        "location": {
            "uri": lsp_file_uri_for_path(Path::new(path)),
            "range": lsp_range_for_source(&source_text, start, end),
        },
        "project_graph": reveal.get("project_graph").cloned().unwrap_or(serde_json::Value::Null),
        "production": reveal.get("production").cloned().unwrap_or(serde_json::Value::Null),
    }))
}

pub(crate) fn lsp_text_document_uri(request: &serde_json::Value) -> anyhow::Result<&str> {
    request
        .pointer("/params/textDocument/uri")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("textDocument.uri must be a file URI"))
}

pub(crate) fn lsp_text_document_position(
    request: &serde_json::Value,
) -> anyhow::Result<(usize, usize)> {
    let position = request
        .pointer("/params/position")
        .ok_or_else(|| anyhow::anyhow!("position must be an object"))?;
    lsp_position_value(position)
}

pub(crate) fn lsp_request_range(
    request: &serde_json::Value,
) -> anyhow::Result<((usize, usize), (usize, usize))> {
    let start = request
        .pointer("/params/range/start")
        .ok_or_else(|| anyhow::anyhow!("range.start must be an object"))?;
    let end = request
        .pointer("/params/range/end")
        .ok_or_else(|| anyhow::anyhow!("range.end must be an object"))?;
    Ok((lsp_position_value(start)?, lsp_position_value(end)?))
}

pub(crate) fn lsp_position_value(value: &serde_json::Value) -> anyhow::Result<(usize, usize)> {
    let line = value
        .get("line")
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| anyhow::anyhow!("position.line must be an integer"))?;
    let character = value
        .get("character")
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| anyhow::anyhow!("position.character must be an integer"))?;
    Ok((
        usize::try_from(line).map_err(|_| anyhow::anyhow!("position.line is too large"))?,
        usize::try_from(character)
            .map_err(|_| anyhow::anyhow!("position.character is too large"))?,
    ))
}

pub(crate) fn lsp_leading_closing_braces(trimmed: &str) -> usize {
    trimmed.chars().take_while(|ch| *ch == '}').count()
}

pub(crate) fn lsp_line_brace_counts(line: &str) -> (usize, usize) {
    let mut opens = 0usize;
    let mut closes = 0usize;
    let mut quote = None;
    let mut escaped = false;
    let mut chars = line.chars().peekable();
    while let Some(ch) = chars.next() {
        if let Some(quote_ch) = quote {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == quote_ch {
                quote = None;
            }
            continue;
        }
        match ch {
            '"' | '\'' => quote = Some(ch),
            '/' if chars.peek() == Some(&'/') => break,
            '{' => opens += 1,
            '}' => closes += 1,
            _ => {}
        }
    }
    (opens, closes)
}

pub(crate) fn lsp_full_document_range(source: &str) -> serde_json::Value {
    lsp_range_for_source(source, 0, u32::try_from(source.len()).unwrap_or(u32::MAX))
}

pub(crate) fn lsp_line_start_byte(source: &str, target_line: usize) -> usize {
    if target_line == 0 {
        return 0;
    }
    let mut line = 0usize;
    for (byte, ch) in source.char_indices() {
        if ch == '\n' {
            line += 1;
            if line == target_line {
                return byte.saturating_add(1).min(source.len());
            }
        }
    }
    source.len()
}

pub(crate) fn lsp_source_file_for_path<'a>(
    files: &'a [SourceFile],
    path: &Path,
) -> Option<&'a SourceFile> {
    let normalized = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    files
        .iter()
        .find(|file| file.path == path || file.path == normalized)
}

pub(crate) fn lsp_definition_node<'a>(
    graph: &'a ProjectGraph,
    name: &str,
) -> Option<&'a orv_project::ProjectNode> {
    graph.nodes.iter().find(|node| {
        node.name == name
            && matches!(
                node.kind,
                ProjectNodeKind::Struct
                    | ProjectNodeKind::Enum
                    | ProjectNodeKind::TypeAlias
                    | ProjectNodeKind::Function
                    | ProjectNodeKind::Define
            )
    })
}

pub(crate) fn lsp_type_definition_node<'a>(
    graph: &'a ProjectGraph,
    name: &str,
) -> Option<&'a orv_project::ProjectNode> {
    graph.nodes.iter().find(|node| {
        node.name == name
            && matches!(
                node.kind,
                ProjectNodeKind::Struct | ProjectNodeKind::Enum | ProjectNodeKind::TypeAlias
            )
    })
}

pub(crate) fn lsp_function_stmt_by_name<'a>(
    program: &'a Program,
    name: &str,
) -> Option<&'a FunctionStmt> {
    program.items.iter().find_map(|stmt| match stmt {
        Stmt::Function(function) if function.name.name == name => Some(function.as_ref()),
        _ => None,
    })
}

pub(crate) fn lsp_function_stmts(program: &Program) -> Vec<&FunctionStmt> {
    program
        .items
        .iter()
        .filter_map(|stmt| match stmt {
            Stmt::Function(function) => Some(function.as_ref()),
            _ => None,
        })
        .collect()
}

pub(crate) fn lsp_call_hierarchy_item_json(
    function: &FunctionStmt,
    files: &[SourceFile],
) -> serde_json::Value {
    let uri = files
        .iter()
        .find(|file| file.id == function.span.file)
        .map_or_else(
            || "file://<unknown>".to_string(),
            |file| lsp_file_uri_for_path(&file.path),
        );
    serde_json::json!({
        "name": function.name.name,
        "kind": 12,
        "detail": "function",
        "uri": uri,
        "range": lsp_range_json(function.span, files),
        "selectionRange": lsp_range_json(function.name.span, files),
    })
}

pub(crate) fn lsp_type_hierarchy_item_json(
    node: &orv_project::ProjectNode,
    files: &[SourceFile],
) -> serde_json::Value {
    let uri = files.iter().find(|file| file.id == node.file).map_or_else(
        || "file://<unknown>".to_string(),
        |file| lsp_file_uri_for_path(&file.path),
    );
    let selection_range = files
        .iter()
        .find(|file| file.id == node.file)
        .and_then(|file| {
            lsp_node_name_span(&file.source, node)
                .map(|(start, end)| lsp_range_for_source(&file.source, start, end))
        })
        .unwrap_or_else(|| lsp_range_json(node.span, files));
    serde_json::json!({
        "name": node.name,
        "kind": lsp_symbol_kind_code(node.kind).unwrap_or(23),
        "detail": lsp_symbol_kind(node.kind).unwrap_or("Type"),
        "uri": uri,
        "range": lsp_range_json(node.span, files),
        "selectionRange": selection_range,
        "data": {
            "source_node": node.id,
        },
    })
}

pub(crate) fn lsp_moniker_json(node: &orv_project::ProjectNode) -> serde_json::Value {
    serde_json::json!({
        "scheme": "orv",
        "identifier": format!("{}:{}", lsp_moniker_symbol_kind(node.kind), node.name),
        "unique": "project",
        "kind": "export",
        "data": {
            "source_node": node.id,
        },
    })
}

pub(crate) fn lsp_hex_pair(high: u8, low: u8) -> Option<u8> {
    Some(lsp_hex_value(high)? * 16 + lsp_hex_value(low)?)
}

pub(crate) const fn lsp_hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

pub(crate) fn lsp_call_hierarchy_outgoing_calls(
    caller: &FunctionStmt,
    program: &Program,
    files: &[SourceFile],
) -> Vec<serde_json::Value> {
    let Some(source) = lsp_source_file_for_span(files, caller.span) else {
        return Vec::new();
    };
    lsp_function_stmts(program)
        .into_iter()
        .filter_map(|callee| {
            let ranges = lsp_function_call_ranges(&source.source, caller, &callee.name.name);
            if ranges.is_empty() {
                return None;
            }
            Some(serde_json::json!({
                "to": lsp_call_hierarchy_item_json(callee, files),
                "fromRanges": ranges,
            }))
        })
        .collect()
}

pub(crate) fn lsp_call_hierarchy_incoming_calls(
    callee_name: &str,
    program: &Program,
    files: &[SourceFile],
) -> Vec<serde_json::Value> {
    lsp_function_stmts(program)
        .into_iter()
        .filter_map(|caller| {
            let source = lsp_source_file_for_span(files, caller.span)?;
            let ranges = lsp_function_call_ranges(&source.source, caller, callee_name);
            if ranges.is_empty() {
                return None;
            }
            Some(serde_json::json!({
                "from": lsp_call_hierarchy_item_json(caller, files),
                "fromRanges": ranges,
            }))
        })
        .collect()
}

pub(crate) fn lsp_function_call_ranges(
    source: &str,
    caller: &FunctionStmt,
    callee_name: &str,
) -> Vec<serde_json::Value> {
    let mut ranges = Vec::new();
    let mut search_from = usize::try_from(caller.span.range.start).unwrap_or(usize::MAX);
    let end = usize::try_from(caller.span.range.end)
        .unwrap_or(usize::MAX)
        .min(source.len());
    search_from = search_from.min(end);
    while let Some(relative) = source[search_from..end].find(callee_name) {
        let name_start = search_from + relative;
        let Some(open) = lsp_call_open_after_name(source, name_start, callee_name) else {
            search_from = name_start.saturating_add(callee_name.len());
            continue;
        };
        if lsp_call_is_function_declaration(source, name_start) {
            search_from = open.saturating_add(1);
            continue;
        }
        let name_end = name_start.saturating_add(callee_name.len());
        ranges.push(lsp_range_for_source(
            source,
            u32::try_from(name_start).unwrap_or(u32::MAX),
            u32::try_from(name_end).unwrap_or(u32::MAX),
        ));
        search_from = open.saturating_add(1);
    }
    ranges
}

pub(crate) fn lsp_source_file_for_span(files: &[SourceFile], span: Span) -> Option<&SourceFile> {
    files.iter().find(|file| file.id == span.file)
}

pub(crate) fn lsp_location_json(
    node: &orv_project::ProjectNode,
    files: &[SourceFile],
) -> serde_json::Value {
    let uri = files.iter().find(|file| file.id == node.file).map_or_else(
        || "file://<unknown>".to_string(),
        |file| lsp_file_uri_for_path(&file.path),
    );
    serde_json::json!({
        "uri": uri,
        "range": lsp_range_json(node.span, files),
    })
}

pub(crate) fn lsp_hover_json(
    node: &orv_project::ProjectNode,
    files: &[SourceFile],
) -> serde_json::Value {
    let kind = lsp_symbol_kind(node.kind).unwrap_or("Symbol");
    serde_json::json!({
        "contents": {
            "kind": "markdown",
            "value": format!("**{kind}** `{}`", node.name),
        },
        "range": lsp_range_json(node.span, files),
    })
}

pub(crate) fn lsp_call_is_function_declaration(source: &str, name_start: usize) -> bool {
    source[..name_start]
        .split_whitespace()
        .last()
        .is_some_and(|word| matches!(word, "function" | "define"))
}

pub(crate) fn lsp_file_uri_for_path(path: &Path) -> String {
    format!("file://{}", path.display())
}

pub(crate) fn identifier_at_byte(source: &str, byte: usize) -> Option<&str> {
    identifier_span_at_byte(source, byte).map(|(_, _, name)| name)
}

pub(crate) fn identifier_span_at_byte(source: &str, byte: usize) -> Option<(usize, usize, &str)> {
    let bytes = source.as_bytes();
    let byte = byte.min(bytes.len());
    let mut start = byte;
    while start > 0 && is_identifier_byte(bytes[start - 1]) {
        start -= 1;
    }
    let mut end = byte;
    while end < bytes.len() && is_identifier_byte(bytes[end]) {
        end += 1;
    }
    if start == end {
        return None;
    }
    source.get(start..end).map(|name| (start, end, name))
}

pub(crate) fn lsp_renamable_identifier_span_at_byte(
    source: &str,
    byte: usize,
) -> Option<(usize, usize, &str)> {
    let (start, end, name) = identifier_span_at_byte(source, byte)?;
    if !lsp_renamable_identifier_name(name)
        || lsp_is_builtin_domain_identifier(source, start, name)
        || lsp_domain_field_kind_at_name_start(source, start).is_some()
    {
        return None;
    }
    Some((start, end, name))
}

pub(crate) fn lsp_reference_locations_json(
    files: &[SourceFile],
    name: &str,
) -> Vec<serde_json::Value> {
    files
        .iter()
        .flat_map(|file| {
            lsp_identifier_ranges_json(&file.source, name)
                .into_iter()
                .map(move |range| {
                    serde_json::json!({
                        "uri": lsp_file_uri_for_path(&file.path),
                        "range": range,
                    })
                })
        })
        .collect()
}

pub(crate) fn lsp_identifier_ranges_json(source: &str, name: &str) -> Vec<serde_json::Value> {
    identifier_occurrences(source, name)
        .into_iter()
        .map(|(start, end)| {
            lsp_range_for_source(
                source,
                u32::try_from(start).unwrap_or(u32::MAX),
                u32::try_from(end).unwrap_or(u32::MAX),
            )
        })
        .collect()
}

pub(crate) fn identifier_occurrences(source: &str, name: &str) -> Vec<(usize, usize)> {
    let bytes = source.as_bytes();
    let mut out = Vec::new();
    let mut index = 0;
    while index < bytes.len() {
        if is_identifier_byte(bytes[index]) {
            let start = index;
            index += 1;
            while index < bytes.len() && is_identifier_byte(bytes[index]) {
                index += 1;
            }
            if source.get(start..index) == Some(name) {
                out.push((start, index));
            }
        } else {
            index += 1;
        }
    }
    out
}

pub(crate) const fn is_identifier_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

pub(crate) fn lsp_valid_identifier_name(name: &str) -> bool {
    let mut bytes = name.bytes();
    let Some(first) = bytes.next() else {
        return false;
    };
    (first.is_ascii_alphabetic() || first == b'_') && bytes.all(is_identifier_byte)
}

pub(crate) fn lsp_renamable_identifier_name(name: &str) -> bool {
    lsp_valid_identifier_name(name) && !lsp_reserved_identifier_name(name)
}

pub(crate) fn lsp_reserved_identifier_name(name: &str) -> bool {
    matches!(
        name,
        "let"
            | "mut"
            | "sig"
            | "const"
            | "function"
            | "async"
            | "await"
            | "return"
            | "if"
            | "else"
            | "when"
            | "for"
            | "in"
            | "while"
            | "break"
            | "continue"
            | "try"
            | "catch"
            | "throw"
            | "struct"
            | "enum"
            | "type"
            | "define"
            | "pub"
            | "import"
            | "void"
            | "as"
            | "true"
            | "false"
            | "null"
            | "int"
            | "float"
            | "string"
            | "bool"
    )
}

pub(crate) fn lsp_is_builtin_domain_identifier(source: &str, start: usize, name: &str) -> bool {
    let Some(previous) = start.checked_sub(1) else {
        return false;
    };
    if source.as_bytes().get(previous) != Some(&b'@') {
        return false;
    }
    name.bytes()
        .next()
        .is_some_and(|first| first.is_ascii_lowercase())
}

pub(crate) fn lsp_file_uri_path(uri: &str) -> anyhow::Result<PathBuf> {
    let raw_path = uri
        .strip_prefix("file://")
        .ok_or_else(|| anyhow::anyhow!("textDocument.uri must use file://"))?;
    Ok(PathBuf::from(percent_decode_uri_path(raw_path)?))
}

pub(crate) fn percent_decode_uri_path(raw: &str) -> anyhow::Result<String> {
    let bytes = raw.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            let hi = bytes
                .get(index + 1)
                .and_then(|byte| uri_hex_value(*byte))
                .ok_or_else(|| anyhow::anyhow!("invalid percent escape in file URI"))?;
            let lo = bytes
                .get(index + 2)
                .and_then(|byte| uri_hex_value(*byte))
                .ok_or_else(|| anyhow::anyhow!("invalid percent escape in file URI"))?;
            out.push((hi << 4) | lo);
            index += 3;
        } else {
            out.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8(out).map_err(|e| anyhow::anyhow!("file URI path is not UTF-8: {e}"))
}

pub(crate) const fn uri_hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

pub(crate) const fn lsp_span_overlaps_range(span: Span, start: u32, end: u32) -> bool {
    span.range.start <= end && start <= span.range.end
}

pub(crate) const fn lsp_selectable_node_kind(kind: ProjectNodeKind) -> bool {
    matches!(
        kind,
        ProjectNodeKind::Struct
            | ProjectNodeKind::Enum
            | ProjectNodeKind::TypeAlias
            | ProjectNodeKind::Function
            | ProjectNodeKind::Define
            | ProjectNodeKind::Domain
            | ProjectNodeKind::Import
    )
}

pub(crate) fn lsp_node_name_span(
    source: &str,
    node: &orv_project::ProjectNode,
) -> Option<(u32, u32)> {
    let start = usize::try_from(node.span.range.start)
        .ok()?
        .min(source.len());
    let end = usize::try_from(node.span.range.end).ok()?.min(source.len());
    let span_source = source.get(start..end)?;
    let offset = span_source.find(&node.name)?;
    let start = start + offset;
    let end = start + node.name.len();
    Some((u32::try_from(start).ok()?, u32::try_from(end).ok()?))
}

pub(crate) fn lsp_route_path_param_names(source: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut search_from = 0usize;
    while let Some(offset) = source[search_from..].find("@route") {
        let route_start = search_from + offset;
        let route_tail = &source[route_start..];
        let head_end = route_tail
            .find('{')
            .or_else(|| route_tail.find('\n'))
            .unwrap_or(route_tail.len());
        names.extend(lsp_route_head_param_names(&route_tail[..head_end]));
        search_from = route_start + "@route".len();
    }
    names
}

pub(crate) fn lsp_route_head_param_names(route_head: &str) -> Vec<String> {
    let bytes = route_head.as_bytes();
    let mut names = Vec::new();
    let mut index = 0usize;
    while index < bytes.len() {
        if bytes[index] != b':' {
            index += 1;
            continue;
        }
        let name_start = index + 1;
        let Some(first) = bytes.get(name_start) else {
            break;
        };
        if !(first.is_ascii_alphabetic() || *first == b'_') {
            index = name_start;
            continue;
        }
        let mut name_end = name_start + 1;
        while bytes
            .get(name_end)
            .is_some_and(|byte| is_identifier_byte(*byte))
        {
            name_end += 1;
        }
        if let Some(name) = route_head.get(name_start..name_end) {
            names.push(name.to_string());
        }
        index = name_end;
    }
    names
}

pub(crate) fn lsp_line_has_open_at_token(line_prefix: &str) -> bool {
    line_prefix
        .rsplit(|ch: char| ch.is_whitespace() || matches!(ch, '(' | '{' | '[' | ',' | ':' | '='))
        .next()
        .is_some_and(|token| token.starts_with('@'))
}

pub(crate) fn lsp_range_json(span: Span, files: &[SourceFile]) -> serde_json::Value {
    let Some(file) = files.iter().find(|file| file.id == span.file) else {
        return serde_json::json!({
            "start": { "line": 0, "character": 0 },
            "end": { "line": 0, "character": 0 },
        });
    };
    let start = lsp_byte_position(&file.source, span.range.start);
    let end = lsp_byte_position(&file.source, span.range.end);
    lsp_range_from_positions(start, end)
}

pub(crate) fn lsp_range_for_source(source: &str, start: u32, end: u32) -> serde_json::Value {
    lsp_range_from_positions(
        lsp_byte_position(source, start),
        lsp_byte_position(source, end),
    )
}

pub(crate) fn lsp_range_from_positions(
    start: (usize, usize),
    end: (usize, usize),
) -> serde_json::Value {
    serde_json::json!({
        "start": {
            "line": start.0,
            "character": start.1,
        },
        "end": {
            "line": end.0,
            "character": end.1,
        },
    })
}
