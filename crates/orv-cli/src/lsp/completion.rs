#![allow(clippy::redundant_pub_crate, clippy::wildcard_imports)]

use super::*;

pub(crate) fn lsp_signature_help_json(
    function: &FunctionStmt,
    active_parameter: usize,
) -> serde_json::Value {
    let parameters = function
        .params
        .iter()
        .map(lsp_signature_parameter_label)
        .collect::<Vec<_>>();
    let label = lsp_signature_label(function, &parameters);
    let max_parameter = parameters.len().saturating_sub(1);
    serde_json::json!({
        "signatures": [
            {
                "label": label,
                "parameters": parameters
                    .iter()
                    .map(|parameter| serde_json::json!({ "label": parameter }))
                    .collect::<Vec<_>>(),
            },
        ],
        "activeSignature": 0,
        "activeParameter": active_parameter.min(max_parameter),
    })
}

pub(crate) fn lsp_signature_label(function: &FunctionStmt, parameters: &[String]) -> String {
    let mut label = format!("{}({})", function.name.name, parameters.join(", "));
    if let Some(return_ty) = &function.return_ty {
        label.push_str(": ");
        label.push_str(&type_ref_string(return_ty));
    }
    label
}

pub(crate) fn lsp_signature_parameter_label(param: &orv_syntax::ast::Param) -> String {
    param.ty.as_ref().map_or_else(
        || param.name.name.clone(),
        |ty| format!("{}: {}", param.name.name, type_ref_string(ty)),
    )
}

pub(crate) fn lsp_inlay_hints_json(
    program: &Program,
    source: &str,
    start: usize,
    end: usize,
) -> Vec<serde_json::Value> {
    let functions = program
        .items
        .iter()
        .filter_map(|stmt| match stmt {
            Stmt::Function(function) => Some(function.as_ref()),
            _ => None,
        })
        .collect::<Vec<_>>();
    let mut hints = Vec::new();
    for function in functions {
        let mut search_from = start.min(source.len());
        let end = end.min(source.len());
        while let Some(relative) = source[search_from..end].find(function.name.name.as_str()) {
            let name_start = search_from + relative;
            let Some(open) = lsp_call_open_after_name(source, name_start, &function.name.name)
            else {
                search_from = name_start.saturating_add(function.name.name.len());
                continue;
            };
            if lsp_call_is_function_declaration(source, name_start) {
                search_from = open.saturating_add(1);
                continue;
            }
            for (index, argument_start) in lsp_call_argument_starts(source, open, end)
                .into_iter()
                .enumerate()
                .take(function.params.len())
            {
                let label = format!("{}:", function.params[index].name.name);
                let position =
                    lsp_byte_position(source, u32::try_from(argument_start).unwrap_or(u32::MAX));
                hints.push(serde_json::json!({
                    "position": {
                        "line": position.0,
                        "character": position.1,
                    },
                    "label": label,
                    "kind": 2,
                    "paddingRight": true,
                }));
            }
            search_from = open.saturating_add(1);
        }
    }
    hints
}

pub(crate) fn lsp_call_open_after_name(
    source: &str,
    name_start: usize,
    name: &str,
) -> Option<usize> {
    if name_start > 0 && is_identifier_byte(source.as_bytes()[name_start - 1]) {
        return None;
    }
    let name_end = name_start.checked_add(name.len())?;
    if source
        .as_bytes()
        .get(name_end)
        .is_some_and(|byte| is_identifier_byte(*byte))
    {
        return None;
    }
    let offset = source[name_end..].find(|ch: char| !ch.is_whitespace())?;
    let open = name_end + offset;
    (source.as_bytes().get(open) == Some(&b'(')).then_some(open)
}

pub(crate) fn lsp_call_argument_starts(source: &str, open: usize, end: usize) -> Vec<usize> {
    let mut starts = Vec::new();
    let bytes = source.as_bytes();
    let limit = end.min(bytes.len());
    let mut depth = 0usize;
    let mut index = open.saturating_add(1);
    while index < limit {
        match bytes[index] {
            b' ' | b'\t' | b'\n' | b'\r' if depth == 0 => {
                index += 1;
            }
            b')' if depth == 0 => break,
            _ => break,
        }
    }
    if index < limit && bytes[index] != b')' {
        starts.push(index);
    }
    while index < limit {
        match bytes[index] {
            b'(' | b'[' | b'{' => depth = depth.saturating_add(1),
            b')' if depth == 0 => break,
            b')' | b']' | b'}' => depth = depth.saturating_sub(1),
            b',' if depth == 0 => {
                index += 1;
                while index < limit && bytes[index].is_ascii_whitespace() {
                    index += 1;
                }
                if index < limit && bytes[index] != b')' {
                    starts.push(index);
                }
                continue;
            }
            _ => {}
        }
        index += 1;
    }
    starts
}

pub(crate) fn lsp_call_signature_context(source: &str, byte: usize) -> Option<(&str, usize)> {
    let open = lsp_call_open_paren(source, byte)?;
    let name_end = source[..open].trim_end().len();
    let name = identifier_span_at_byte(source, name_end.checked_sub(1)?)?.2;
    let active_parameter = lsp_active_parameter_index(&source[open.saturating_add(1)..byte]);
    Some((name, active_parameter))
}

pub(crate) fn lsp_call_open_paren(source: &str, byte: usize) -> Option<usize> {
    let bytes = source.as_bytes();
    let mut depth = 0usize;
    let mut index = byte.min(bytes.len());
    while index > 0 {
        index -= 1;
        match bytes[index] {
            b')' | b']' | b'}' => depth = depth.saturating_add(1),
            b'(' if depth == 0 => return Some(index),
            b'(' | b'[' | b'{' => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    None
}

pub(crate) fn lsp_active_parameter_index(source: &str) -> usize {
    let mut depth = 0usize;
    let mut active = 0usize;
    for byte in source.bytes() {
        match byte {
            b'(' | b'[' | b'{' => depth = depth.saturating_add(1),
            b')' | b']' | b'}' => depth = depth.saturating_sub(1),
            b',' if depth == 0 => active = active.saturating_add(1),
            _ => {}
        }
    }
    active
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub(crate) enum LspCompletionContext {
    General,
    Directive,
    RouteMethod,
    BodyField,
    ParamField,
    QueryField,
    EnvField,
}

pub(crate) struct LspStaticCompletion {
    pub(crate) label: &'static str,
    pub(crate) kind: u8,
    pub(crate) detail: &'static str,
    pub(crate) insert_text: Option<&'static str>,
}

pub(super) const LSP_GENERAL_COMPLETIONS: &[LspStaticCompletion] = &[
    LspStaticCompletion {
        label: "import",
        kind: 14,
        detail: "Keyword",
        insert_text: Some("import \"${1:path}\""),
    },
    LspStaticCompletion {
        label: "pub",
        kind: 14,
        detail: "Keyword",
        insert_text: None,
    },
    LspStaticCompletion {
        label: "struct",
        kind: 15,
        detail: "Struct declaration",
        insert_text: Some("struct ${1:Name} {\n  ${2:id}: ${3:int}\n}"),
    },
    LspStaticCompletion {
        label: "enum",
        kind: 15,
        detail: "Enum declaration",
        insert_text: Some("enum ${1:Name} {\n  ${2:Variant}\n}"),
    },
    LspStaticCompletion {
        label: "type",
        kind: 15,
        detail: "Type alias",
        insert_text: Some("type ${1:Name} = ${2:int}"),
    },
    LspStaticCompletion {
        label: "let",
        kind: 15,
        detail: "Binding",
        insert_text: Some("let ${1:name} = ${2:value}"),
    },
    LspStaticCompletion {
        label: "let sig",
        kind: 15,
        detail: "Reactive signal",
        insert_text: Some("let sig ${1:name} = ${2:value}"),
    },
    LspStaticCompletion {
        label: "const",
        kind: 15,
        detail: "Constant",
        insert_text: Some("const ${1:name} = ${2:value}"),
    },
    LspStaticCompletion {
        label: "function",
        kind: 15,
        detail: "Function declaration",
        insert_text: Some("function ${1:name}(${2:input}: ${3:int}): ${4:int} -> {\n  $0\n}"),
    },
    LspStaticCompletion {
        label: "async function",
        kind: 15,
        detail: "Async function declaration",
        insert_text: Some("async function ${1:name}(${2:input}: ${3:int}): ${4:int} -> {\n  $0\n}"),
    },
    LspStaticCompletion {
        label: "define",
        kind: 15,
        detail: "Token-aware define declaration",
        insert_text: Some("define ${1:name}(${2:input}: ${3:int}): ${4:int} -> {\n  $0\n}"),
    },
    LspStaticCompletion {
        label: "domain",
        kind: 15,
        detail: "Domain declaration",
        insert_text: Some("domain ${1:Name} {\n  $0\n}"),
    },
    LspStaticCompletion {
        label: "if",
        kind: 15,
        detail: "Conditional",
        insert_text: Some("if ${1:condition} {\n  $0\n}"),
    },
    LspStaticCompletion {
        label: "for",
        kind: 15,
        detail: "Loop",
        insert_text: Some("for ${1:item} in ${2:items} {\n  $0\n}"),
    },
    LspStaticCompletion {
        label: "while",
        kind: 15,
        detail: "Loop",
        insert_text: Some("while ${1:condition} {\n  $0\n}"),
    },
    LspStaticCompletion {
        label: "return",
        kind: 14,
        detail: "Keyword",
        insert_text: None,
    },
    LspStaticCompletion {
        label: "await",
        kind: 14,
        detail: "Keyword",
        insert_text: None,
    },
    LspStaticCompletion {
        label: "test",
        kind: 15,
        detail: "Test block",
        insert_text: Some("test \"${1:name}\" {\n  $0\n}"),
    },
    LspStaticCompletion {
        label: "true",
        kind: 14,
        detail: "Boolean literal",
        insert_text: None,
    },
    LspStaticCompletion {
        label: "false",
        kind: 14,
        detail: "Boolean literal",
        insert_text: None,
    },
    LspStaticCompletion {
        label: "null",
        kind: 14,
        detail: "Null literal",
        insert_text: None,
    },
];

pub(super) const LSP_DIRECTIVE_COMPLETIONS: &[LspStaticCompletion] = &[
    LspStaticCompletion {
        label: "@server",
        kind: 15,
        detail: "Server block",
        insert_text: Some("@server {\n  $0\n}"),
    },
    LspStaticCompletion {
        label: "@listen",
        kind: 15,
        detail: "Server listen port",
        insert_text: Some("@listen ${1:8080}"),
    },
    LspStaticCompletion {
        label: "@route",
        kind: 15,
        detail: "HTTP route",
        insert_text: Some("@route ${1:GET} ${2:/path} {\n  $0\n}"),
    },
    LspStaticCompletion {
        label: "@respond",
        kind: 15,
        detail: "HTTP response",
        insert_text: Some("@respond ${1:200} ${2:{ ok: true }}"),
    },
    LspStaticCompletion {
        label: "@serve",
        kind: 15,
        detail: "HTML response",
        insert_text: Some("@serve @html {\n  @body {\n    $0\n  }\n}"),
    },
    LspStaticCompletion {
        label: "@db.connect",
        kind: 15,
        detail: "Database adapter",
        insert_text: Some("@db.connect(@env.${1:DATABASE_URL} ?? \"sqlite://data/app.sqlite\")"),
    },
    LspStaticCompletion {
        label: "@payment.connect",
        kind: 15,
        detail: "Payment adapter",
        insert_text: Some(
            "@payment.connect(@env.PAYMENT_ADAPTER_URL ?? \"file://data/payments.jsonl\")",
        ),
    },
    LspStaticCompletion {
        label: "@shipping.connect",
        kind: 15,
        detail: "Shipping adapter",
        insert_text: Some(
            "@shipping.connect(@env.SHIPPING_ADAPTER_URL ?? \"file://data/shipments.jsonl\")",
        ),
    },
    LspStaticCompletion {
        label: "@env",
        kind: 6,
        detail: "Environment value",
        insert_text: Some("@env.${1:NAME}"),
    },
    LspStaticCompletion {
        label: "@body",
        kind: 6,
        detail: "Request body",
        insert_text: None,
    },
    LspStaticCompletion {
        label: "@param",
        kind: 6,
        detail: "Route parameter",
        insert_text: Some("@param.${1:name}"),
    },
    LspStaticCompletion {
        label: "@query",
        kind: 6,
        detail: "Query parameter",
        insert_text: Some("@query.${1:name}"),
    },
    LspStaticCompletion {
        label: "@request.rawBody",
        kind: 6,
        detail: "Raw request body",
        insert_text: None,
    },
    LspStaticCompletion {
        label: "@html",
        kind: 14,
        detail: "HTML domain",
        insert_text: None,
    },
    LspStaticCompletion {
        label: "@body block",
        kind: 15,
        detail: "HTML body",
        insert_text: Some("@body {\n  $0\n}"),
    },
    LspStaticCompletion {
        label: "@section",
        kind: 15,
        detail: "HTML section",
        insert_text: Some("@section {\n  $0\n}"),
    },
    LspStaticCompletion {
        label: "@form",
        kind: 15,
        detail: "HTML form",
        insert_text: Some("@form action=\"${1:/path}\" method=post {\n  $0\n}"),
    },
    LspStaticCompletion {
        label: "@input",
        kind: 15,
        detail: "HTML input",
        insert_text: Some("@input type=${1:text} name=${2:name}"),
    },
    LspStaticCompletion {
        label: "@button",
        kind: 15,
        detail: "HTML button",
        insert_text: Some("@button type=submit \"${1:Submit}\""),
    },
    LspStaticCompletion {
        label: "@a",
        kind: 15,
        detail: "HTML anchor",
        insert_text: Some("@a href=\"${1:/}\" \"${2:Link}\""),
    },
    LspStaticCompletion {
        label: "@h1",
        kind: 15,
        detail: "HTML heading",
        insert_text: Some("@h1 \"${1:Heading}\""),
    },
    LspStaticCompletion {
        label: "@p",
        kind: 15,
        detail: "HTML paragraph",
        insert_text: Some("@p \"${1:Text}\""),
    },
    LspStaticCompletion {
        label: "@ul",
        kind: 15,
        detail: "HTML list",
        insert_text: Some("@ul {\n  $0\n}"),
    },
    LspStaticCompletion {
        label: "@li",
        kind: 15,
        detail: "HTML list item",
        insert_text: Some("@li \"${1:Item}\""),
    },
    LspStaticCompletion {
        label: "@label",
        kind: 15,
        detail: "HTML label",
        insert_text: Some("@label \"${1:Label}\""),
    },
];

pub(super) const LSP_ROUTE_METHOD_COMPLETIONS: &[LspStaticCompletion] = &[
    LspStaticCompletion {
        label: "GET",
        kind: 14,
        detail: "HTTP method",
        insert_text: None,
    },
    LspStaticCompletion {
        label: "POST",
        kind: 14,
        detail: "HTTP method",
        insert_text: None,
    },
    LspStaticCompletion {
        label: "PUT",
        kind: 14,
        detail: "HTTP method",
        insert_text: None,
    },
    LspStaticCompletion {
        label: "PATCH",
        kind: 14,
        detail: "HTTP method",
        insert_text: None,
    },
    LspStaticCompletion {
        label: "DELETE",
        kind: 14,
        detail: "HTTP method",
        insert_text: None,
    },
    LspStaticCompletion {
        label: "OPTIONS",
        kind: 14,
        detail: "HTTP method",
        insert_text: None,
    },
    LspStaticCompletion {
        label: "HEAD",
        kind: 14,
        detail: "HTTP method",
        insert_text: None,
    },
];

pub(crate) fn lsp_completion_items_json(
    graph: &ProjectGraph,
    files: &[SourceFile],
    context: LspCompletionContext,
) -> Vec<serde_json::Value> {
    let mut items = lsp_context_completion_items_json(files, context);
    if matches!(
        context,
        LspCompletionContext::RouteMethod
            | LspCompletionContext::BodyField
            | LspCompletionContext::ParamField
            | LspCompletionContext::QueryField
            | LspCompletionContext::EnvField
    ) {
        return items;
    }
    for node in &graph.nodes {
        let Some(kind) = lsp_completion_item_kind_code(node.kind) else {
            continue;
        };
        if lsp_completion_item_exists(&items, node.name.as_str(), kind) {
            continue;
        }
        items.push(serde_json::json!({
            "label": node.name.clone(),
            "kind": kind,
            "detail": lsp_symbol_kind(node.kind).unwrap_or("Symbol"),
            "data": {
                "source_node": node.id,
            },
        }));
    }
    items
}

pub(crate) fn lsp_context_completion_items_json(
    files: &[SourceFile],
    context: LspCompletionContext,
) -> Vec<serde_json::Value> {
    match context {
        LspCompletionContext::BodyField => {
            lsp_domain_field_completion_items_json(files, "body", 10, "@body field")
        }
        LspCompletionContext::ParamField => {
            lsp_domain_field_completion_items_json(files, "param", 10, "@param field")
        }
        LspCompletionContext::QueryField => {
            lsp_domain_field_completion_items_json(files, "query", 10, "@query field")
        }
        LspCompletionContext::EnvField => {
            lsp_domain_field_completion_items_json(files, "env", 21, "@env value")
        }
        LspCompletionContext::General
        | LspCompletionContext::Directive
        | LspCompletionContext::RouteMethod => lsp_static_completion_items_json(context),
    }
}

pub(crate) fn lsp_static_completion_items_json(
    context: LspCompletionContext,
) -> Vec<serde_json::Value> {
    let specs = match context {
        LspCompletionContext::General => LSP_GENERAL_COMPLETIONS,
        LspCompletionContext::Directive => LSP_DIRECTIVE_COMPLETIONS,
        LspCompletionContext::RouteMethod => LSP_ROUTE_METHOD_COMPLETIONS,
        LspCompletionContext::BodyField
        | LspCompletionContext::ParamField
        | LspCompletionContext::QueryField
        | LspCompletionContext::EnvField => &[],
    };
    let mut items = Vec::new();
    for spec in specs {
        if lsp_completion_item_exists(&items, spec.label, spec.kind) {
            continue;
        }
        let mut item = serde_json::json!({
            "label": spec.label,
            "kind": spec.kind,
            "detail": spec.detail,
        });
        if let Some(insert_text) = spec.insert_text {
            item["insertText"] = serde_json::json!(insert_text);
            item["insertTextFormat"] = serde_json::json!(2);
        }
        items.push(item);
    }
    items
}

pub(crate) fn lsp_completion_item_exists(
    items: &[serde_json::Value],
    label: &str,
    kind: u8,
) -> bool {
    items.iter().any(|item| {
        item.get("label").and_then(serde_json::Value::as_str) == Some(label)
            && item.get("kind").and_then(serde_json::Value::as_u64) == Some(u64::from(kind))
    })
}

pub(crate) fn lsp_completion_context(source: &str, byte: usize) -> LspCompletionContext {
    let prefix = &source[..byte.min(source.len())];
    let line_start = prefix.rfind('\n').map_or(0, |index| index + 1);
    let line_prefix = &prefix[line_start..];
    let trimmed = line_prefix.trim_start();
    if let Some(context) = lsp_domain_field_completion_context(line_prefix) {
        return context;
    }
    if lsp_is_route_method_completion(trimmed) {
        return LspCompletionContext::RouteMethod;
    }
    if lsp_line_has_open_at_token(line_prefix) {
        return LspCompletionContext::Directive;
    }
    LspCompletionContext::General
}

pub(crate) fn lsp_is_route_method_completion(trimmed_line_prefix: &str) -> bool {
    let Some(rest) = trimmed_line_prefix.strip_prefix("@route") else {
        return false;
    };
    if !rest.is_empty() && !rest.starts_with(char::is_whitespace) {
        return false;
    }
    let after_route = rest.trim_start();
    after_route.is_empty() || !after_route.contains(char::is_whitespace)
}

pub(crate) const fn lsp_completion_item_kind_code(kind: ProjectNodeKind) -> Option<u8> {
    match kind {
        ProjectNodeKind::Struct | ProjectNodeKind::TypeAlias => Some(22),
        ProjectNodeKind::Enum => Some(13),
        ProjectNodeKind::Function | ProjectNodeKind::Define => Some(3),
        ProjectNodeKind::Domain => Some(23),
        ProjectNodeKind::File | ProjectNodeKind::Import => None,
    }
}
