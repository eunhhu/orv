#![allow(clippy::redundant_pub_crate, clippy::wildcard_imports)]

use super::*;

pub(crate) const fn lsp_moniker_symbol_kind(kind: ProjectNodeKind) -> &'static str {
    match kind {
        ProjectNodeKind::Struct => "struct",
        ProjectNodeKind::Enum => "enum",
        ProjectNodeKind::TypeAlias => "type",
        ProjectNodeKind::Function | ProjectNodeKind::Define => "function",
        ProjectNodeKind::Domain => "domain",
        ProjectNodeKind::File | ProjectNodeKind::Import => "symbol",
    }
}

pub(crate) fn lsp_document_symbols_json(
    graph: &ProjectGraph,
    files: &[SourceFile],
) -> Vec<serde_json::Value> {
    graph
        .nodes
        .iter()
        .filter_map(|node| {
            lsp_symbol_kind(node.kind).map(|kind| {
                serde_json::json!({
                    "name": node.name,
                    "kind": kind,
                    "range": lsp_range_json(node.span, files),
                    "selectionRange": lsp_range_json(node.span, files),
                    "source_node": node.id,
                })
            })
        })
        .collect()
}

pub(crate) fn lsp_document_symbols_protocol_json(
    graph: &ProjectGraph,
    files: &[SourceFile],
) -> Vec<serde_json::Value> {
    graph
        .nodes
        .iter()
        .filter_map(|node| {
            lsp_symbol_kind_code(node.kind).map(|kind| {
                serde_json::json!({
                    "name": node.name,
                    "kind": kind,
                    "range": lsp_range_json(node.span, files),
                    "selectionRange": lsp_range_json(node.span, files),
                    "data": {
                        "source_node": node.id,
                    },
                })
            })
        })
        .collect()
}

#[derive(Clone, Copy)]
pub(crate) struct LspSemanticToken {
    pub(crate) line: usize,
    pub(crate) character: usize,
    pub(crate) length: usize,
    pub(crate) token_type: u32,
    pub(crate) modifiers: u32,
}

pub(crate) fn lsp_semantic_tokens_json(
    graph: &ProjectGraph,
    files: &[SourceFile],
    file_id: FileId,
) -> serde_json::Value {
    let Some(file) = files.iter().find(|file| file.id == file_id) else {
        return serde_json::json!({ "data": [] });
    };
    let mut tokens = graph
        .nodes
        .iter()
        .filter(|node| node.file == file_id)
        .filter_map(|node| {
            let token_type = lsp_semantic_token_type(node.kind)?;
            let (start, end) = lsp_node_name_span(&file.source, node)?;
            let start = lsp_byte_position(&file.source, start);
            let end = lsp_byte_position(&file.source, end);
            if start.0 != end.0 || end.1 <= start.1 {
                return None;
            }
            Some(LspSemanticToken {
                line: start.0,
                character: start.1,
                length: end.1 - start.1,
                token_type,
                modifiers: 1,
            })
        })
        .collect::<Vec<_>>();
    tokens.sort_by_key(|token| (token.line, token.character));

    let mut data = Vec::with_capacity(tokens.len() * 5);
    let mut previous_line = 0;
    let mut previous_character = 0;
    for token in tokens {
        let delta_line = token.line.saturating_sub(previous_line);
        let delta_character = if delta_line == 0 {
            token.character.saturating_sub(previous_character)
        } else {
            token.character
        };
        data.push(u32::try_from(delta_line).unwrap_or(u32::MAX));
        data.push(u32::try_from(delta_character).unwrap_or(u32::MAX));
        data.push(u32::try_from(token.length).unwrap_or(u32::MAX));
        data.push(token.token_type);
        data.push(token.modifiers);
        previous_line = token.line;
        previous_character = token.character;
    }
    serde_json::json!({ "data": data })
}

pub(crate) const fn lsp_semantic_token_type(kind: ProjectNodeKind) -> Option<u32> {
    match kind {
        ProjectNodeKind::Domain => Some(0),
        ProjectNodeKind::Struct | ProjectNodeKind::Enum | ProjectNodeKind::TypeAlias => Some(1),
        ProjectNodeKind::Function | ProjectNodeKind::Define => Some(2),
        ProjectNodeKind::File | ProjectNodeKind::Import => None,
    }
}

pub(crate) fn lsp_workspace_symbols_json(
    graph: &ProjectGraph,
    files: &[SourceFile],
    query: &str,
) -> Vec<serde_json::Value> {
    let normalized_query = query.to_ascii_lowercase();
    graph
        .nodes
        .iter()
        .filter_map(|node| {
            let kind = lsp_symbol_kind_code(node.kind)?;
            if !normalized_query.is_empty()
                && !node
                    .name
                    .to_ascii_lowercase()
                    .contains(normalized_query.as_str())
            {
                return None;
            }
            Some(serde_json::json!({
                "name": node.name,
                "kind": kind,
                "location": lsp_location_json(node, files),
                "data": {
                    "source_node": node.id,
                },
            }))
        })
        .collect()
}

pub(crate) const fn lsp_symbol_kind(kind: ProjectNodeKind) -> Option<&'static str> {
    match kind {
        ProjectNodeKind::Struct => Some("Struct"),
        ProjectNodeKind::Enum => Some("Enum"),
        ProjectNodeKind::TypeAlias => Some("TypeAlias"),
        ProjectNodeKind::Function => Some("Function"),
        ProjectNodeKind::Define => Some("Function"),
        ProjectNodeKind::Domain => Some("Event"),
        ProjectNodeKind::File | ProjectNodeKind::Import => None,
    }
}

pub(crate) const fn lsp_symbol_kind_code(kind: ProjectNodeKind) -> Option<u8> {
    match kind {
        ProjectNodeKind::Struct | ProjectNodeKind::TypeAlias => Some(23),
        ProjectNodeKind::Enum => Some(10),
        ProjectNodeKind::Function | ProjectNodeKind::Define => Some(12),
        ProjectNodeKind::Domain => Some(24),
        ProjectNodeKind::File | ProjectNodeKind::Import => None,
    }
}
