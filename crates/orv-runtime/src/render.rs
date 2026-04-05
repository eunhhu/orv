//! Bridges the tree-walking evaluator and the HTML renderer.
//!
//! Converts `eval::Value` trees produced by the interpreter into
//! `html::HtmlNode` trees, and then to rendered HTML strings.

use crate::eval::Value;
use crate::html::{self, HtmlNode, is_self_closing, layout_classes, node_to_tag};

// serde_json is used for safe JSON escaping in signal injection
use serde_json;

// ── Core conversion ───────────────────────────────────────────────────────────

/// Convert an eval `Value` into an `HtmlNode`.
///
/// - `Value::Node { name, properties, children }` → `HtmlNode::Element`
/// - `Value::String(s)` → `HtmlNode::Text(s)`
/// - `Value::Array(items)` → each item rendered and wrapped in a fragment div
///   (callers that want a flat list should call `render_children` instead)
/// - `Value::Void` → empty text (callers should skip these)
/// - All other scalars → `HtmlNode::Text(value.to_string())`
pub fn render_value_to_html(value: &Value) -> HtmlNode {
    match value {
        Value::Node {
            name,
            properties,
            children,
        } => render_node(name, properties, children),
        Value::String(s) => HtmlNode::Text(s.clone()),
        Value::Array(items) => {
            // Wrap multiple items in a transparent fragment div.
            // The outer caller can flatten if needed.
            let child_nodes = render_children(items);
            HtmlNode::element("div").with_children(child_nodes)
        }
        Value::Void => HtmlNode::Text(String::new()),
        other => HtmlNode::Text(other.to_string()),
    }
}

/// Render a list of `Value`s, flattening arrays and skipping voids.
pub fn render_children(values: &[Value]) -> Vec<HtmlNode> {
    let mut out = Vec::new();
    for value in values {
        match value {
            Value::Void => {}
            Value::Array(items) => out.extend(render_children(items)),
            Value::Node {
                name,
                properties,
                children,
            } => {
                // `@text` positional args become inline text nodes
                if name == "text" {
                    for child in children {
                        if let Value::String(s) = child {
                            out.push(HtmlNode::Text(s.clone()));
                        } else {
                            out.push(render_value_to_html(child));
                        }
                    }
                } else {
                    out.push(render_node(name, properties, children));
                }
            }
            other => out.push(render_value_to_html(other)),
        }
    }
    out
}

// ── Node rendering ────────────────────────────────────────────────────────────

fn render_node(
    name: &str,
    properties: &std::collections::HashMap<String, Value>,
    children: &[Value],
) -> HtmlNode {
    // Special cases first
    match name {
        // @text "content" → plain text
        "text" => {
            let text: String = children
                .iter()
                .filter_map(|c| {
                    if let Value::String(s) = c {
                        Some(s.as_str())
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
                .join("");
            return HtmlNode::Text(text);
        }
        // @title "text" → <title>text</title>
        "title" => {
            let text: String = children
                .iter()
                .filter_map(|c| {
                    if let Value::String(s) = c {
                        Some(s.as_str())
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
                .join("");
            return HtmlNode::element("title").with_child(HtmlNode::Text(text));
        }
        _ => {}
    }

    let tag = node_to_tag(name);
    let mut element = HtmlNode::element(tag);

    // Layout classes for vstack / hstack
    if let Some(style) = layout_classes(name) {
        element = element.with_attr("style", style);
    }

    // Properties → HTML attributes
    for (key, val) in properties {
        let attr_val = match val {
            Value::String(s) => s.clone(),
            other => other.to_string(),
        };
        match key.as_str() {
            "class" => element = element.with_class(&attr_val),
            _ => element = element.with_attr(key, &attr_val),
        }
    }

    // Self-closing tags carry no children
    if is_self_closing(tag) {
        return element.self_closing();
    }

    // Render children (blocks return a single Value from eval; unwrap arrays)
    let child_nodes = render_children(children);
    element.with_children(child_nodes)
}

// ── Signal stub ───────────────────────────────────────────────────────────────

/// Insert a reactivity placeholder comment for a signal variable.
///
/// When `let sig count = 0` is encountered in an HTML context the evaluator
/// still stores the initial scalar value. Call this to emit a marker comment
/// alongside the rendered initial value so future JS codegen can locate it.
pub fn render_signal_placeholder(name: &str, initial: &Value) -> Vec<HtmlNode> {
    vec![
        // The comment tag is not a first-class HtmlNode variant; embed it as
        // raw text wrapped in a zero-width span so we don't modify html.rs.
        HtmlNode::Text(format!("<!-- sig: {name} -->")),
        render_value_to_html(initial),
    ]
}

// ── Page rendering ────────────────────────────────────────────────────────────

/// Produce a complete `<!DOCTYPE html>` page from a root `Value`.
///
/// If the root value is a `@html` node we split its children into `<head>`
/// and `<body>` sections; otherwise every rendered node goes into `<body>`.
pub fn render_page(value: &Value) -> String {
    match value {
        Value::Node { name, children, .. } if name == "html" => {
            let mut head: Vec<HtmlNode> = Vec::new();
            let mut body: Vec<HtmlNode> = Vec::new();
            for child in children {
                match child {
                    Value::Node {
                        name,
                        properties,
                        children: grandchildren,
                    } if name == "head" => {
                        head.extend(render_children(grandchildren));
                        // also apply any properties on the head node itself
                        let _ = properties; // head element has no special props
                    }
                    Value::Node {
                        name,
                        properties,
                        children: grandchildren,
                    } if name == "body" => {
                        let _ = properties;
                        body.extend(render_children(grandchildren));
                    }
                    other => {
                        // Loose children go into body
                        body.push(render_value_to_html(other));
                    }
                }
            }
            html::render_document(&head, &body)
        }

        // Top-level array: split head / body nodes by name
        Value::Array(items) => {
            let mut head: Vec<HtmlNode> = Vec::new();
            let mut body: Vec<HtmlNode> = Vec::new();
            for item in items {
                match item {
                    Value::Node {
                        name,
                        properties: _,
                        children,
                    } if name == "head" => {
                        head.extend(render_children(children));
                    }
                    other => body.push(render_value_to_html(other)),
                }
            }
            html::render_document(&head, &body)
        }

        // Anything else: render as body
        other => {
            let body = vec![render_value_to_html(other)];
            html::render_document(&[], &body)
        }
    }
}

/// Produce a complete page with signal state injected for client-side hydration.
///
/// Adds a `<script type="application/json">` tag containing the signal snapshot
/// and a `<script>` tag referencing orv-runtime.js.
pub fn render_page_with_signals(value: &Value, signal_snapshot: &[(String, Value)]) -> String {
    let mut page = render_page(value);

    if !signal_snapshot.is_empty() {
        // Build signal JSON using serde for proper escaping
        let mut json_parts = Vec::new();
        for (name, val) in signal_snapshot {
            let json_key = serde_json::to_string(name).unwrap_or_default();
            let json_val = match val {
                Value::Int(n) => n.to_string(),
                Value::Float(n) => n.to_string(),
                Value::Bool(b) => b.to_string(),
                Value::String(s) => serde_json::to_string(s).unwrap_or_default(),
                Value::Void => "null".to_owned(),
                _ => serde_json::to_string(&val.to_string()).unwrap_or_default(),
            };
            json_parts.push(format!("{json_key}:{json_val}"));
        }
        let signal_json = format!("{{{}}}", json_parts.join(","));
        // Escape </ sequences to prevent script breakout
        let safe_json = signal_json.replace("</", "<\\/");

        // Use <script type="application/json"> to avoid attribute injection
        let injection = format!(
            "  <script type=\"application/json\" id=\"orv-signals\">{safe_json}</script>\n\
             \x20 <script src=\"/orv-runtime.js\"></script>\n"
        );
        if let Some(pos) = page.find("</body>") {
            page.insert_str(pos, &injection);
        }
    }

    page
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::html::render;

    fn node(name: &str, children: Vec<Value>) -> Value {
        Value::Node {
            name: name.to_owned(),
            properties: HashMap::new(),
            children,
        }
    }

    fn node_with_props(name: &str, props: Vec<(&str, &str)>, children: Vec<Value>) -> Value {
        let properties = props
            .into_iter()
            .map(|(k, v)| (k.to_owned(), Value::String(v.to_owned())))
            .collect();
        Value::Node {
            name: name.to_owned(),
            properties,
            children,
        }
    }

    // ── simple div with text ─────────────────────────────────────────────────

    #[test]
    fn simple_div_with_text() {
        let val = node("div", vec![Value::String("Hello".to_owned())]);
        let html_node = render_value_to_html(&val);
        let html = render(&html_node);
        assert!(html.contains("<div>"), "expected <div> in: {html}");
        assert!(html.contains("Hello"), "expected text in: {html}");
        assert!(html.contains("</div>"), "expected </div> in: {html}");
    }

    // ── nested elements ──────────────────────────────────────────────────────

    #[test]
    fn nested_elements() {
        let inner = node("p", vec![Value::String("World".to_owned())]);
        let outer = node("div", vec![inner]);
        let html_node = render_value_to_html(&outer);
        let html = render(&html_node);
        assert!(html.contains("<div>"), "no <div>: {html}");
        assert!(html.contains("<p>World</p>"), "no <p>: {html}");
        assert!(html.contains("</div>"), "no </div>: {html}");
    }

    // ── node with properties → HTML attributes ───────────────────────────────

    #[test]
    fn node_properties_become_attributes() {
        let val = node_with_props(
            "input",
            vec![("type", "text"), ("placeholder", "Enter name")],
            vec![],
        );
        let html_node = render_value_to_html(&val);
        let html = render(&html_node);
        assert!(html.contains("type=\"text\""), "no type attr: {html}");
        assert!(
            html.contains("placeholder=\"Enter name\""),
            "no placeholder: {html}"
        );
    }

    #[test]
    fn class_property_uses_class_attr() {
        let val = node_with_props("div", vec![("class", "container")], vec![]);
        let html_node = render_value_to_html(&val);
        let html = render(&html_node);
        assert!(html.contains("class=\"container\""), "no class: {html}");
    }

    #[test]
    fn onclick_property_becomes_onclick_attr() {
        let val = node_with_props("button", vec![("onClick", "doThing()")], vec![]);
        let html_node = render_value_to_html(&val);
        let html = render(&html_node);
        assert!(html.contains("onClick=\"doThing()\""), "no onclick: {html}");
    }

    // ── vstack / hstack layout ───────────────────────────────────────────────

    #[test]
    fn vstack_gets_flex_column_style() {
        let val = node("vstack", vec![]);
        let html_node = render_value_to_html(&val);
        let html = render(&html_node);
        assert!(
            html.contains("flex-direction:column"),
            "no flex-column: {html}"
        );
    }

    #[test]
    fn hstack_gets_flex_row_style() {
        let val = node("hstack", vec![]);
        let html_node = render_value_to_html(&val);
        let html = render(&html_node);
        assert!(html.contains("flex-direction:row"), "no flex-row: {html}");
    }

    // ── complete page rendering ──────────────────────────────────────────────

    #[test]
    fn render_page_produces_doctype() {
        let val = Value::String("hi".to_owned());
        let page = render_page(&val);
        assert!(page.starts_with("<!DOCTYPE html>"), "no doctype: {page}");
        assert!(page.contains("hi"), "no content: {page}");
    }

    #[test]
    fn render_page_html_node_splits_head_body() {
        let title_node = node("title", vec![Value::String("My Page".to_owned())]);
        let head_node = node("head", vec![title_node]);
        let h1_node = node("h1", vec![Value::String("Hello".to_owned())]);
        let body_node = node("body", vec![h1_node]);
        let html_val = node("html", vec![head_node, body_node]);

        let page = render_page(&html_val);
        assert!(page.starts_with("<!DOCTYPE html>"), "no doctype: {page}");
        assert!(page.contains("<head>"), "no head: {page}");
        assert!(page.contains("<title>My Page</title>"), "no title: {page}");
        assert!(page.contains("<body>"), "no body: {page}");
        assert!(page.contains("<h1>Hello</h1>"), "no h1: {page}");
    }

    // ── conditional rendering (if → value or void) ───────────────────────────

    #[test]
    fn void_children_are_skipped() {
        let val = node(
            "div",
            vec![
                Value::Void,
                Value::String("visible".to_owned()),
                Value::Void,
            ],
        );
        let html_node = render_value_to_html(&val);
        let html = render(&html_node);
        assert!(html.contains("visible"), "content missing: {html}");
        // void should not produce extra whitespace or tags beyond normal
        assert!(!html.contains("void"), "void leaked: {html}");
    }

    // ── list rendering (array of nodes) ─────────────────────────────────────

    #[test]
    fn array_of_nodes_renders_all_items() {
        let items = Value::Array(vec![
            node("li", vec![Value::String("one".to_owned())]),
            node("li", vec![Value::String("two".to_owned())]),
            node("li", vec![Value::String("three".to_owned())]),
        ]);
        let list = node("ul", vec![items]);
        let html_node = render_value_to_html(&list);
        let html = render(&html_node);
        assert!(html.contains("<ul>"), "no ul: {html}");
        assert!(html.contains("<li>one</li>"), "no li one: {html}");
        assert!(html.contains("<li>two</li>"), "no li two: {html}");
        assert!(html.contains("<li>three</li>"), "no li three: {html}");
    }

    // ── signal stub ─────────────────────────────────────────────────────────

    #[test]
    fn signal_placeholder_contains_comment_and_value() {
        let nodes = render_signal_placeholder("count", &Value::Int(0));
        assert_eq!(nodes.len(), 2, "expected comment + value");
        // The comment is stored as a Text node; verify the raw text content.
        match &nodes[0] {
            HtmlNode::Text(t) => assert!(t.contains("<!-- sig: count -->"), "no comment: {t}"),
            other => panic!("expected Text node, got {other:?}"),
        }
        let value_html = render(&nodes[1]);
        assert!(value_html.contains('0'), "no value: {value_html}");
    }
}
