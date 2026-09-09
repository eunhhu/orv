#![allow(clippy::redundant_pub_crate, clippy::wildcard_imports)]

use super::*;

pub(crate) fn lsp_formatting_tab_size(request: &serde_json::Value) -> usize {
    request
        .pointer("/params/options/tabSize")
        .and_then(serde_json::Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())
        .unwrap_or(2)
        .clamp(1, 8)
}

pub(crate) fn lsp_formatting_insert_spaces(request: &serde_json::Value) -> bool {
    request
        .pointer("/params/options/insertSpaces")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(true)
}

pub(crate) fn lsp_format_source(source: &str, tab_size: usize, insert_spaces: bool) -> String {
    lsp_format_source_with_initial_indent(source, tab_size, insert_spaces, 0)
}

pub(crate) fn lsp_format_source_with_initial_indent(
    source: &str,
    tab_size: usize,
    insert_spaces: bool,
    initial_indent: usize,
) -> String {
    let indent_unit = if insert_spaces {
        " ".repeat(tab_size)
    } else {
        "\t".to_string()
    };
    let mut formatted = Vec::new();
    let mut indent = initial_indent;
    for line in source.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            formatted.push(String::new());
            continue;
        }
        let (line_indent, next_indent) = lsp_format_line_indent(indent, trimmed);
        let mut next = indent_unit.repeat(line_indent);
        next.push_str(trimmed);
        formatted.push(next);
        indent = next_indent;
    }
    if formatted.is_empty() {
        String::new()
    } else {
        format!("{}\n", formatted.join("\n"))
    }
}

pub(crate) fn lsp_format_line_indent(indent: usize, trimmed: &str) -> (usize, usize) {
    let leading_close = lsp_leading_closing_braces(trimmed).min(indent);
    let line_indent = indent.saturating_sub(leading_close);
    let (opens, closes) = lsp_line_brace_counts(trimmed);
    let non_leading_closes = closes.saturating_sub(leading_close);
    (
        line_indent,
        line_indent
            .saturating_add(opens)
            .saturating_sub(non_leading_closes),
    )
}

pub(crate) fn lsp_indent_level_before(source: &str, byte: usize) -> usize {
    let prefix = source.get(..byte.min(source.len())).unwrap_or(source);
    let mut indent = 0usize;
    for line in prefix.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        indent = lsp_format_line_indent(indent, trimmed).1;
    }
    indent
}

pub(crate) fn lsp_line_range_for_formatting(
    source: &str,
    requested_range: ((usize, usize), (usize, usize)),
) -> (usize, usize, serde_json::Value) {
    let ((start_line, _), end_position) = requested_range;
    let start = lsp_line_start_byte(source, start_line);
    let end_line = if end_position.1 == 0 {
        end_position.0
    } else {
        end_position.0.saturating_add(1)
    };
    let end = lsp_line_start_byte(source, end_line).max(start);
    (
        start,
        end,
        lsp_range_for_source(
            source,
            u32::try_from(start).unwrap_or(u32::MAX),
            u32::try_from(end).unwrap_or(u32::MAX),
        ),
    )
}

pub(crate) fn lsp_current_line_range_for_formatting(
    source: &str,
    line: usize,
) -> (usize, usize, serde_json::Value) {
    let start = lsp_line_start_byte(source, line);
    let end = lsp_line_start_byte(source, line.saturating_add(1)).max(start);
    (
        start,
        end,
        lsp_range_for_source(
            source,
            u32::try_from(start).unwrap_or(u32::MAX),
            u32::try_from(end).unwrap_or(u32::MAX),
        ),
    )
}

pub(crate) fn lsp_newline_on_type_formatting_edit_json(
    source: &str,
    line: usize,
    tab_size: usize,
    insert_spaces: bool,
) -> serde_json::Value {
    let (start, end, edit_range) = lsp_current_line_range_for_formatting(source, line);
    let Some(source_slice) = source.get(start..end) else {
        return serde_json::Value::Array(Vec::new());
    };
    if !source_slice.trim().is_empty() {
        return serde_json::Value::Array(Vec::new());
    }
    let indent_unit = if insert_spaces {
        " ".repeat(tab_size)
    } else {
        "\t".to_string()
    };
    let mut new_text = indent_unit.repeat(lsp_indent_level_before(source, start));
    if source_slice.ends_with('\n') {
        new_text.push('\n');
    }
    if new_text == source_slice {
        return serde_json::Value::Array(Vec::new());
    }
    serde_json::Value::Array(vec![serde_json::json!({
        "range": edit_range,
        "newText": new_text,
    })])
}
