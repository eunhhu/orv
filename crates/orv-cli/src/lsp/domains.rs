#![allow(clippy::redundant_pub_crate, clippy::wildcard_imports)]

use super::*;

#[derive(Clone, Copy)]
pub(crate) struct LspDomainFieldKind {
    pub(crate) domain: &'static str,
    pub(crate) marker: &'static str,
    pub(crate) label: &'static str,
}

pub(crate) struct LspDomainField<'a> {
    pub(crate) kind: LspDomainFieldKind,
    pub(crate) start: usize,
    pub(crate) end: usize,
    pub(crate) name: &'a str,
}

pub(super) const LSP_DOMAIN_FIELD_KINDS: &[LspDomainFieldKind] = &[
    LspDomainFieldKind {
        domain: "body",
        marker: "@body.",
        label: "Request body field",
    },
    LspDomainFieldKind {
        domain: "param",
        marker: "@param.",
        label: "Route parameter",
    },
    LspDomainFieldKind {
        domain: "query",
        marker: "@query.",
        label: "Query parameter",
    },
    LspDomainFieldKind {
        domain: "env",
        marker: "@env.",
        label: "Environment value",
    },
];

pub(crate) fn lsp_domain_field_hover_json(source: &str, byte: usize) -> Option<serde_json::Value> {
    let field = lsp_domain_field_at_byte(source, byte)?;
    Some(serde_json::json!({
        "contents": {
            "kind": "markdown",
            "value": format!("**{}** `{}`", field.kind.label, field.name),
        },
        "range": lsp_range_for_source(
            source,
            u32::try_from(field.start).unwrap_or(u32::MAX),
            u32::try_from(field.end).unwrap_or(u32::MAX),
        ),
    }))
}

pub(crate) fn lsp_domain_field_at_byte(source: &str, byte: usize) -> Option<LspDomainField<'_>> {
    let (start, end, name) = identifier_span_at_byte(source, byte)?;
    let kind = lsp_domain_field_kind_at_name_start(source, start)?;
    Some(LspDomainField {
        kind,
        start,
        end,
        name,
    })
}

pub(crate) fn lsp_domain_field_kind_at_name_start(
    source: &str,
    name_start: usize,
) -> Option<LspDomainFieldKind> {
    LSP_DOMAIN_FIELD_KINDS.iter().copied().find(|kind| {
        name_start >= kind.marker.len()
            && source
                .as_bytes()
                .get(name_start - kind.marker.len()..name_start)
                == Some(kind.marker.as_bytes())
    })
}

pub(crate) fn lsp_domain_field_kind_for_domain(domain: &str) -> Option<LspDomainFieldKind> {
    LSP_DOMAIN_FIELD_KINDS
        .iter()
        .copied()
        .find(|kind| kind.domain == domain)
}

pub(crate) fn lsp_domain_field_reference_locations_json(
    files: &[SourceFile],
    kind: LspDomainFieldKind,
    name: &str,
) -> Vec<serde_json::Value> {
    files
        .iter()
        .flat_map(|file| {
            lsp_domain_field_occurrences(&file.source, kind, name)
                .into_iter()
                .map(move |(start, end)| {
                    serde_json::json!({
                        "uri": lsp_file_uri_for_path(&file.path),
                        "range": lsp_range_for_source(
                            &file.source,
                            u32::try_from(start).unwrap_or(u32::MAX),
                            u32::try_from(end).unwrap_or(u32::MAX),
                        ),
                    })
                })
        })
        .collect()
}

pub(crate) fn lsp_domain_field_occurrences(
    source: &str,
    kind: LspDomainFieldKind,
    name: &str,
) -> Vec<(usize, usize)> {
    lsp_domain_field_spans(source, kind)
        .into_iter()
        .filter_map(|(start, end, candidate)| (candidate == name).then_some((start, end)))
        .collect()
}

pub(crate) fn lsp_domain_field_spans(
    source: &str,
    kind: LspDomainFieldKind,
) -> Vec<(usize, usize, &str)> {
    let marker = kind.marker.as_bytes();
    let bytes = source.as_bytes();
    let mut out = Vec::new();
    let mut index = 0usize;
    while index <= bytes.len().saturating_sub(marker.len()) {
        if bytes.get(index..index + marker.len()) != Some(marker) {
            index += 1;
            continue;
        }
        let name_start = index + marker.len();
        let mut name_end = name_start;
        while bytes
            .get(name_end)
            .is_some_and(|byte| is_identifier_byte(*byte))
        {
            name_end += 1;
        }
        if name_end > name_start {
            if let Some(name) = source.get(name_start..name_end) {
                out.push((name_start, name_end, name));
            }
            index = name_end;
        } else {
            index = name_start.saturating_add(1);
        }
    }
    out
}

pub(crate) fn lsp_domain_field_completion_items_json(
    files: &[SourceFile],
    domain: &str,
    kind: u8,
    detail: &str,
) -> Vec<serde_json::Value> {
    lsp_domain_field_names(files, domain)
        .into_iter()
        .map(|label| {
            serde_json::json!({
                "label": label,
                "kind": kind,
                "detail": detail,
            })
        })
        .collect()
}

pub(crate) fn lsp_domain_field_names(files: &[SourceFile], domain: &str) -> Vec<String> {
    let Some(kind) = lsp_domain_field_kind_for_domain(domain) else {
        return Vec::new();
    };
    let mut names = Vec::new();
    for file in files {
        if domain == "param" {
            names.extend(lsp_route_path_param_names(&file.source));
        }
        names.extend(
            lsp_domain_field_spans(&file.source, kind)
                .into_iter()
                .map(|(_, _, name)| name.to_string()),
        );
    }
    names.sort();
    names.dedup();
    names
}

pub(crate) fn lsp_domain_field_completion_context(
    line_prefix: &str,
) -> Option<LspCompletionContext> {
    let token = line_prefix
        .rsplit(|ch: char| ch.is_whitespace() || matches!(ch, '(' | '{' | '[' | ',' | ':' | '='))
        .next()?;
    if token.starts_with("@body.") {
        return Some(LspCompletionContext::BodyField);
    }
    if token.starts_with("@param.") {
        return Some(LspCompletionContext::ParamField);
    }
    if token.starts_with("@query.") {
        return Some(LspCompletionContext::QueryField);
    }
    if token.starts_with("@env.") {
        return Some(LspCompletionContext::EnvField);
    }
    None
}
