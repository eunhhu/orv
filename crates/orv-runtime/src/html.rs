//! HTML rendering engine for orv's @html domain.
//!
//! Converts orv node trees into complete HTML documents.

use std::collections::HashMap;
use std::fmt::Write;

/// An HTML node in the render tree.
#[derive(Debug, Clone)]
pub enum HtmlNode {
    /// A complete HTML document: `<!DOCTYPE html><html>...</html>`
    Document {
        head: Vec<HtmlNode>,
        body: Vec<HtmlNode>,
    },
    /// An HTML element: `<tag attrs>children</tag>`
    Element {
        tag: String,
        attributes: HashMap<String, String>,
        classes: Vec<String>,
        children: Vec<HtmlNode>,
        self_closing: bool,
    },
    /// Raw text content
    Text(String),
    /// A script tag with inline JavaScript
    Script(String),
    /// A style tag with inline CSS
    Style(String),
}

impl HtmlNode {
    pub fn element(tag: &str) -> Self {
        Self::Element {
            tag: tag.to_owned(),
            attributes: HashMap::new(),
            classes: Vec::new(),
            children: Vec::new(),
            self_closing: false,
        }
    }

    pub fn with_attr(mut self, key: &str, value: &str) -> Self {
        if let Self::Element { attributes, .. } = &mut self {
            attributes.insert(key.to_owned(), value.to_owned());
        }
        self
    }

    pub fn with_class(mut self, class: &str) -> Self {
        if let Self::Element { classes, .. } = &mut self {
            classes.push(class.to_owned());
        }
        self
    }

    pub fn with_child(mut self, child: HtmlNode) -> Self {
        if let Self::Element { children, .. } = &mut self {
            children.push(child);
        }
        self
    }

    pub fn with_children(mut self, new_children: Vec<HtmlNode>) -> Self {
        if let Self::Element { children, .. } = &mut self {
            children.extend(new_children);
        }
        self
    }

    pub fn self_closing(mut self) -> Self {
        if let Self::Element { self_closing, .. } = &mut self {
            *self_closing = true;
        }
        self
    }
}

/// Maps orv node names to HTML tags.
pub fn node_to_tag(name: &str) -> &str {
    match name {
        // Structural
        "html" => "html",
        "head" => "head",
        "body" => "body",
        "div" => "div",
        "span" => "span",
        "section" => "section",
        "article" => "article",
        "nav" => "nav",
        "main" => "main",
        "header" => "header",
        "footer" => "footer",
        "aside" => "aside",
        // Text
        "h1" => "h1",
        "h2" => "h2",
        "h3" => "h3",
        "h4" => "h4",
        "h5" => "h5",
        "h6" => "h6",
        "p" => "p",
        "text" => "span",
        // Interactive
        "button" => "button",
        "input" => "input",
        "form" => "form",
        "select" => "select",
        "option" => "option",
        "textarea" => "textarea",
        "label" => "label",
        "a" => "a",
        // Layout (orv-specific)
        "vstack" => "div",
        "hstack" => "div",
        // Media
        "img" => "img",
        "video" => "video",
        "audio" => "audio",
        // Table
        "table" => "table",
        "tr" => "tr",
        "td" => "td",
        "th" => "th",
        // Meta
        "title" => "title",
        "meta" => "meta",
        "link" => "link",
        "script" => "script",
        "style" => "style",
        // Lists
        "ul" => "ul",
        "ol" => "ol",
        "li" => "li",
        // Default
        _ => "div",
    }
}

/// Check if an HTML element is self-closing.
pub fn is_self_closing(tag: &str) -> bool {
    matches!(
        tag,
        "meta"
            | "link"
            | "input"
            | "img"
            | "br"
            | "hr"
            | "area"
            | "base"
            | "col"
            | "embed"
            | "source"
            | "track"
            | "wbr"
    )
}

/// Maps orv layout node names to their CSS classes.
pub fn layout_classes(name: &str) -> Option<&str> {
    match name {
        "vstack" => Some("display:flex;flex-direction:column;"),
        "hstack" => Some("display:flex;flex-direction:row;"),
        _ => None,
    }
}

/// Render an `HtmlNode` tree to an HTML string.
pub fn render(node: &HtmlNode) -> String {
    let mut out = String::new();
    render_node(node, &mut out, 0);
    out
}

/// Render a complete HTML document.
pub fn render_document(head: &[HtmlNode], body: &[HtmlNode]) -> String {
    let mut out = String::with_capacity(4096);
    out.push_str("<!DOCTYPE html>\n<html>\n<head>\n");
    out.push_str("  <meta charset=\"utf-8\">\n");
    for node in head {
        render_node(node, &mut out, 1);
    }
    out.push_str("</head>\n<body>\n");
    for node in body {
        render_node(node, &mut out, 1);
    }
    out.push_str("</body>\n</html>\n");
    out
}

fn render_node(node: &HtmlNode, out: &mut String, depth: usize) {
    let indent = "  ".repeat(depth);

    match node {
        HtmlNode::Document { head, body } => {
            out.push_str(&render_document(head, body));
        }
        HtmlNode::Element {
            tag,
            attributes,
            classes,
            children,
            self_closing,
        } => {
            let _ = write!(out, "{indent}<{tag}");

            // Merge classes
            if !classes.is_empty() {
                let class_str = classes.join(" ");
                if let Some(existing) = attributes.get("class") {
                    let _ = write!(
                        out,
                        " class=\"{} {}\"",
                        escape_html(existing),
                        escape_html(&class_str)
                    );
                } else {
                    let _ = write!(out, " class=\"{}\"", escape_html(&class_str));
                }
            }

            // Write attributes (escape values to prevent XSS)
            for (key, value) in attributes {
                if key == "class" && !classes.is_empty() {
                    continue; // Already handled above
                }
                let _ = write!(out, " {}=\"{}\"", escape_html(key), escape_html(value));
            }

            if *self_closing || is_self_closing(tag) {
                out.push_str(">\n");
                return;
            }

            if children.is_empty() {
                let _ = writeln!(out, "></{tag}>");
                return;
            }

            // Check if all children are text
            let all_text = children.iter().all(|c| matches!(c, HtmlNode::Text(_)));
            if all_text && children.len() == 1 {
                out.push('>');
                for child in children {
                    if let HtmlNode::Text(text) = child {
                        out.push_str(&escape_html(text));
                    }
                }
                let _ = writeln!(out, "</{tag}>");
            } else {
                out.push_str(">\n");
                for child in children {
                    render_node(child, out, depth + 1);
                }
                let _ = writeln!(out, "{indent}</{tag}>");
            }
        }
        HtmlNode::Text(text) => {
            let _ = writeln!(out, "{indent}{}", escape_html(text));
        }
        HtmlNode::Script(code) => {
            let _ = write!(out, "{indent}<script>\n{code}\n{indent}</script>\n");
        }
        HtmlNode::Style(css) => {
            let _ = write!(out, "{indent}<style>\n{css}\n{indent}</style>\n");
        }
    }
}

/// Escape HTML special characters in text content and attribute values.
fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_simple_element() {
        let node = HtmlNode::element("div").with_child(HtmlNode::Text("Hello".to_owned()));
        let html = render(&node);
        assert_eq!(html, "<div>Hello</div>\n");
    }

    #[test]
    fn render_element_with_attrs() {
        let node = HtmlNode::element("input")
            .with_attr("type", "text")
            .with_attr("name", "email")
            .self_closing();
        let html = render(&node);
        assert!(html.contains("<input"));
        assert!(html.contains("type=\"text\""));
        assert!(html.contains("name=\"email\""));
    }

    #[test]
    fn render_nested_elements() {
        let inner = HtmlNode::element("p").with_child(HtmlNode::Text("World".to_owned()));
        let outer = HtmlNode::element("div").with_child(inner);
        let html = render(&outer);
        assert!(html.contains("<div>"));
        assert!(html.contains("<p>World</p>"));
        assert!(html.contains("</div>"));
    }

    #[test]
    fn render_element_with_classes() {
        let node = HtmlNode::element("div")
            .with_class("flex")
            .with_class("items-center")
            .with_child(HtmlNode::Text("content".to_owned()));
        let html = render(&node);
        assert!(html.contains("class=\"flex items-center\""));
    }

    #[test]
    fn render_self_closing_meta() {
        let node = HtmlNode::element("meta").with_attr("charset", "utf-8");
        let html = render(&node);
        assert!(html.contains("<meta charset=\"utf-8\">"));
        assert!(!html.contains("</meta>"));
    }

    #[test]
    fn render_document_structure() {
        let head = vec![HtmlNode::element("title").with_child(HtmlNode::Text("Test".to_owned()))];
        let body = vec![HtmlNode::element("h1").with_child(HtmlNode::Text("Hello".to_owned()))];
        let html = render_document(&head, &body);
        assert!(html.starts_with("<!DOCTYPE html>"));
        assert!(html.contains("<html>"));
        assert!(html.contains("<head>"));
        assert!(html.contains("<title>Test</title>"));
        assert!(html.contains("<body>"));
        assert!(html.contains("<h1>Hello</h1>"));
        assert!(html.contains("</html>"));
    }

    #[test]
    fn escape_html_chars() {
        assert_eq!(
            escape_html("<script>alert('xss')</script>"),
            "&lt;script&gt;alert(&#x27;xss&#x27;)&lt;/script&gt;"
        );
        assert_eq!(escape_html("a & b"), "a &amp; b");
        assert_eq!(escape_html(r#"he said "hi""#), "he said &quot;hi&quot;");
    }

    #[test]
    fn node_to_tag_mapping() {
        assert_eq!(node_to_tag("div"), "div");
        assert_eq!(node_to_tag("vstack"), "div");
        assert_eq!(node_to_tag("text"), "span");
        assert_eq!(node_to_tag("unknown_node"), "div");
    }

    #[test]
    fn self_closing_check() {
        assert!(is_self_closing("meta"));
        assert!(is_self_closing("input"));
        assert!(is_self_closing("img"));
        assert!(!is_self_closing("div"));
        assert!(!is_self_closing("span"));
    }

    #[test]
    fn layout_classes_mapping() {
        assert_eq!(
            layout_classes("vstack"),
            Some("display:flex;flex-direction:column;")
        );
        assert_eq!(
            layout_classes("hstack"),
            Some("display:flex;flex-direction:row;")
        );
        assert_eq!(layout_classes("div"), None);
    }
}
