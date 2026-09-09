#![allow(clippy::redundant_pub_crate, clippy::wildcard_imports)]

use super::*;

pub(crate) fn lsp_document_colors_json(source: &str) -> Vec<serde_json::Value> {
    let bytes = source.as_bytes();
    let mut colors = Vec::new();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'#' {
            if let Some((length, red, green, blue)) = lsp_hex_color_at(bytes, index) {
                let start = u32::try_from(index).unwrap_or(u32::MAX);
                let end = u32::try_from(index.saturating_add(length)).unwrap_or(u32::MAX);
                colors.push(serde_json::json!({
                    "range": lsp_range_for_source(source, start, end),
                    "color": {
                        "red": f64::from(red) / 255.0,
                        "green": f64::from(green) / 255.0,
                        "blue": f64::from(blue) / 255.0,
                        "alpha": 1.0,
                    },
                }));
                index = index.saturating_add(length);
                continue;
            }
        }
        index = index.saturating_add(1);
    }
    colors
}

pub(crate) fn lsp_hex_color_at(bytes: &[u8], index: usize) -> Option<(usize, u8, u8, u8)> {
    let start = index.checked_add(1)?;
    lsp_hex_color_with_digits(bytes, start, 6)
        .map(|(red, green, blue)| (7, red, green, blue))
        .or_else(|| {
            lsp_hex_color_with_digits(bytes, start, 3)
                .map(|(red, green, blue)| (4, red, green, blue))
        })
}

pub(crate) fn lsp_hex_color_with_digits(
    bytes: &[u8],
    start: usize,
    digits: usize,
) -> Option<(u8, u8, u8)> {
    let end = start.checked_add(digits)?;
    if end > bytes.len()
        || bytes
            .get(end)
            .and_then(|byte| lsp_hex_value(*byte))
            .is_some()
    {
        return None;
    }
    match digits {
        6 => Some((
            lsp_hex_pair(bytes[start], bytes[start + 1])?,
            lsp_hex_pair(bytes[start + 2], bytes[start + 3])?,
            lsp_hex_pair(bytes[start + 4], bytes[start + 5])?,
        )),
        3 => {
            let red = lsp_hex_value(bytes[start])?;
            let green = lsp_hex_value(bytes[start + 1])?;
            let blue = lsp_hex_value(bytes[start + 2])?;
            Some((red * 17, green * 17, blue * 17))
        }
        _ => None,
    }
}

pub(crate) fn lsp_color_param(request: &serde_json::Value) -> anyhow::Result<(u8, u8, u8, u8)> {
    let color = request
        .pointer("/params/color")
        .ok_or_else(|| anyhow::anyhow!("color must be an object"))?;
    Ok((
        lsp_color_channel_param(color, "red")?,
        lsp_color_channel_param(color, "green")?,
        lsp_color_channel_param(color, "blue")?,
        lsp_color_channel_param(color, "alpha")?,
    ))
}

pub(crate) fn lsp_color_channel_param(
    color: &serde_json::Value,
    field: &str,
) -> anyhow::Result<u8> {
    let value = color
        .get(field)
        .and_then(serde_json::Value::as_f64)
        .ok_or_else(|| anyhow::anyhow!("color.{field} must be a number"))?;
    Ok(lsp_color_channel(value))
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub(crate) fn lsp_color_channel(value: f64) -> u8 {
    (value.clamp(0.0, 1.0) * 255.0).round() as u8
}

pub(crate) fn lsp_hex_color_label(red: u8, green: u8, blue: u8, alpha: u8) -> String {
    if alpha == u8::MAX {
        format!("#{red:02x}{green:02x}{blue:02x}")
    } else {
        format!("#{red:02x}{green:02x}{blue:02x}{alpha:02x}")
    }
}

pub(crate) fn lsp_linked_editing_range_json(source: &str, name: &str) -> serde_json::Value {
    serde_json::json!({
        "ranges": lsp_identifier_ranges_json(source, name),
        "wordPattern": "[A-Za-z_][A-Za-z0-9_]*",
    })
}

pub(crate) fn lsp_document_links_json(
    graph: &ProjectGraph,
    files: &[SourceFile],
    file_id: FileId,
) -> Vec<serde_json::Value> {
    graph
        .nodes
        .iter()
        .filter(|node| node.kind == ProjectNodeKind::Import && node.file == file_id)
        .filter_map(|node| {
            let target = graph
                .edges
                .iter()
                .find(|edge| edge.kind == ProjectEdgeKind::Imports && edge.from == node.id)?;
            let target_node = graph
                .nodes
                .iter()
                .find(|candidate| candidate.id == target.to)?;
            let target_file = files.iter().find(|file| file.id == target_node.file)?;
            Some(serde_json::json!({
                "range": lsp_range_json(node.span, files),
                "target": lsp_file_uri_for_path(&target_file.path),
                "tooltip": format!("Open {}", target_node.name),
            }))
        })
        .collect()
}

pub(crate) fn lsp_folding_ranges_json(
    graph: &ProjectGraph,
    files: &[SourceFile],
    file_id: FileId,
) -> Vec<serde_json::Value> {
    graph
        .nodes
        .iter()
        .filter(|node| node.file == file_id)
        .filter(|node| {
            matches!(
                node.kind,
                ProjectNodeKind::Struct
                    | ProjectNodeKind::Enum
                    | ProjectNodeKind::TypeAlias
                    | ProjectNodeKind::Function
                    | ProjectNodeKind::Define
                    | ProjectNodeKind::Domain
            )
        })
        .filter_map(|node| lsp_folding_range_json(node.span, files))
        .collect()
}

pub(crate) fn lsp_folding_range_json(
    span: Span,
    files: &[SourceFile],
) -> Option<serde_json::Value> {
    let file = files.iter().find(|file| file.id == span.file)?;
    let start = lsp_byte_position(&file.source, span.range.start);
    let end = lsp_byte_position(&file.source, span.range.end);
    if end.0 <= start.0 {
        return None;
    }
    Some(serde_json::json!({
        "startLine": start.0,
        "startCharacter": start.1,
        "endLine": end.0,
        "endCharacter": end.1,
        "kind": "region",
    }))
}

pub(crate) fn lsp_selection_range_json(
    graph: &ProjectGraph,
    files: &[SourceFile],
    file_id: FileId,
    byte: usize,
) -> Option<serde_json::Value> {
    let byte = u32::try_from(byte).unwrap_or(u32::MAX);
    let mut nodes: Vec<_> = graph
        .nodes
        .iter()
        .filter(|node| node.file == file_id)
        .filter(|node| lsp_selectable_node_kind(node.kind))
        .filter(|node| node.span.range.start <= byte && byte <= node.span.range.end)
        .collect();
    nodes.sort_by_key(|node| node.span.range.end.saturating_sub(node.span.range.start));

    let mut current = None;
    for node in nodes.into_iter().rev() {
        current = Some(serde_json::json!({
            "range": lsp_range_json(node.span, files),
            "parent": current.unwrap_or(serde_json::Value::Null),
        }));
    }
    current
}
