//! Live HTTP server runner for orv programs.
//!
//! Wires together the HIR evaluator, route compiler, and HTTP server to run
//! an orv program as a real HTTP server.

use std::collections::HashMap;
use std::sync::Arc;

use orv_hir::{Expr, ItemKind, Module, NodeExpr, Stmt};
use thiserror::Error;

use crate::eval::{EvalError, Evaluator, Value};
use crate::html::{HtmlNode, is_self_closing, layout_classes, node_to_tag, render_document};
use crate::server::{HttpRequest, HttpResponse, HttpServer, RouteHandler};

// ── RunError ─────────────────────────────────────────────────────────────────

#[derive(Debug, Error)]
pub enum RunError {
    #[error("{0}")]
    Message(String),
    #[error("i/o error: {0}")]
    Io(#[from] std::io::Error),
}

impl From<EvalError> for RunError {
    fn from(e: EvalError) -> Self {
        Self::Message(e.to_string())
    }
}

// ── Public entry point ────────────────────────────────────────────────────────

/// Run `hir` as a live HTTP server. Blocks until the process is killed.
pub fn run_server(hir: &Module) -> Result<(), RunError> {
    run_server_on_port(hir, None)
}

/// Run `hir` as a live HTTP server on the given port (or the port from `@listen`).
pub fn run_server_on_port(hir: &Module, port_override: Option<u16>) -> Result<(), RunError> {
    let server_node = find_server_node(hir)
        .ok_or_else(|| RunError::Message("no top-level @server entry was found".to_owned()))?;

    let port = port_override.map_or_else(|| extract_listen_port(server_node), Ok)?;
    let env_bindings = extract_env_bindings(server_node)?;

    let mut http_server = HttpServer::new();

    let routes = collect_routes(server_node);
    let env_bindings = Arc::new(env_bindings);

    for (method, path, body_stmts) in routes {
        let env_bindings = Arc::clone(&env_bindings);
        let handler: RouteHandler =
            Box::new(move |req: &HttpRequest| handle_route(req, &body_stmts, &env_bindings));
        http_server.route(&method, &path, handler);
    }

    http_server.listen(port)?;
    Ok(())
}

// ── Server node helpers ───────────────────────────────────────────────────────

fn find_server_node(module: &Module) -> Option<&NodeExpr> {
    module.items.iter().find_map(|item| match &item.kind {
        ItemKind::Stmt(Stmt::Expr(Expr::Node(node))) if node.name == "server" => Some(node),
        _ => None,
    })
}

/// Extract the port from `@listen <port>` inside a @server block, defaulting to 3000.
fn extract_listen_port(server_node: &NodeExpr) -> Result<u16, RunError> {
    let stmts = server_body_stmts(server_node)?;
    for stmt in stmts {
        if let Stmt::Expr(Expr::Node(child)) = stmt
            && child.name == "listen"
            && let Some(port_expr) = child.positional.first()
        {
            let mut eval = Evaluator::new();
            let val = eval.eval_expr(port_expr)?;
            return match val {
                Value::Int(n) => u16::try_from(n)
                    .map_err(|_| RunError::Message(format!("invalid listen port `{n}`"))),
                other => Err(RunError::Message(format!(
                    "@listen port must be an integer, got {other}"
                ))),
            };
        }
    }
    Ok(3000)
}

/// Extract `@env NAME ?? default` bindings from the @server block.
/// Returns a map of variable name -> resolved string value.
fn extract_env_bindings(server_node: &NodeExpr) -> Result<HashMap<String, String>, RunError> {
    let stmts = server_body_stmts(server_node)?;
    let mut bindings = HashMap::new();

    for stmt in stmts {
        if let Stmt::Expr(Expr::Node(child)) = stmt
            && child.name == "env"
            && let Some(Expr::StringLiteral(var_name)) = child.positional.first()
        {
            let env_val = std::env::var(var_name).ok();
            if let Some(val) = env_val {
                bindings.insert(var_name.clone(), val);
            }
            // If not set, the ?? default in the expression handles it at eval time
        }
    }

    Ok(bindings)
}

/// Collect `(method, path, body_stmts)` for each @route in the @server block.
fn collect_routes(server_node: &NodeExpr) -> Vec<(String, String, Vec<Stmt>)> {
    let Ok(stmts) = server_body_stmts(server_node) else {
        return Vec::new();
    };

    let mut routes = Vec::new();
    for stmt in stmts {
        if let Stmt::Expr(Expr::Node(child)) = stmt
            && child.name == "route"
            && child.positional.len() >= 2
        {
            let method = match &child.positional[0] {
                Expr::Ident(n) => n.name.clone(),
                Expr::StringLiteral(s) => s.clone(),
                _ => continue,
            };
            let path = match &child.positional[1] {
                Expr::Ident(n) => n.name.clone(),
                Expr::StringLiteral(s) => s.clone(),
                _ => continue,
            };
            let body_stmts = match child.body.as_deref() {
                Some(Expr::Block { stmts, .. }) => stmts.clone(),
                _ => continue,
            };
            routes.push((method, path, body_stmts));
        }
    }

    routes
}

fn server_body_stmts(server_node: &NodeExpr) -> Result<&[Stmt], RunError> {
    match server_node.body.as_deref() {
        Some(Expr::Block { stmts, .. }) => Ok(stmts),
        other => Err(RunError::Message(format!(
            "@server body must be a block, got {other:?}"
        ))),
    }
}

// ── Request handler ───────────────────────────────────────────────────────────

fn handle_route(
    req: &HttpRequest,
    body_stmts: &[Stmt],
    env_bindings: &HashMap<String, String>,
) -> HttpResponse {
    let mut eval = Evaluator::new();

    // Bind @env values from environment variables
    for (name, value) in env_bindings {
        eval.env.set(name.clone(), Value::String(value.clone()));
    }

    // Bind request accessors as special variables
    bind_request_accessors(&mut eval, req);

    // Evaluate statements until we find @respond or @serve
    for stmt in body_stmts {
        // Run non-action statements for their side effects (bindings, etc.)
        if let Some(response) = try_action_stmt(stmt, &mut eval, req) {
            return response;
        }
        // Execute the statement for side effects (ignore errors in non-action stmts)
        if let Stmt::Expr(Expr::Node(node)) = stmt
            && (node.name == "respond" || node.name == "serve")
        {
            // Already handled above
            continue;
        }
        let _ = eval.eval_stmt(stmt);
    }

    HttpResponse::internal_error("route body did not produce a response")
}

/// Bind the request accessor magic variables into the evaluator environment.
fn bind_request_accessors(eval: &mut Evaluator, req: &HttpRequest) {
    // Bind @method and @path as simple variables accessible in expressions
    eval.env
        .set("__method".to_owned(), Value::String(req.method.clone()));
    eval.env
        .set("__path".to_owned(), Value::String(req.path.clone()));
    eval.env
        .set("__body".to_owned(), Value::String(req.body.clone()));

    // Bind path params map
    let path_params: HashMap<String, Value> = req
        .path_params
        .iter()
        .map(|(k, v)| (k.clone(), Value::String(v.clone())))
        .collect();
    eval.env
        .set("__path_params".to_owned(), Value::Map(path_params));

    // Bind query params map
    let query_params: HashMap<String, Value> = req
        .query_params
        .iter()
        .map(|(k, v)| (k.clone(), Value::String(v.clone())))
        .collect();
    eval.env
        .set("__query_params".to_owned(), Value::Map(query_params));

    // Bind headers map
    let headers: HashMap<String, Value> = req
        .headers
        .iter()
        .map(|(k, v)| (k.clone(), Value::String(v.clone())))
        .collect();
    eval.env.set("__headers".to_owned(), Value::Map(headers));
}

/// Try to interpret a statement as a @respond or @serve action.
/// Returns Some(HttpResponse) if it is one, None otherwise.
fn try_action_stmt(stmt: &Stmt, eval: &mut Evaluator, req: &HttpRequest) -> Option<HttpResponse> {
    let Stmt::Expr(Expr::Node(node)) = stmt else {
        return None;
    };
    match node.name.as_str() {
        "respond" => Some(eval_respond(node, eval)),
        "serve" => Some(eval_serve(node, eval, req)),
        _ => None,
    }
}

// ── @respond ──────────────────────────────────────────────────────────────────

fn eval_respond(node: &NodeExpr, eval: &mut Evaluator) -> HttpResponse {
    let status = match node.positional.first() {
        Some(expr) => match eval.eval_expr(expr) {
            Ok(Value::Int(n)) => match u16::try_from(n) {
                Ok(s) => s,
                Err(_) => {
                    return HttpResponse::internal_error(&format!("invalid status code {n}"));
                }
            },
            Ok(other) => {
                return HttpResponse::internal_error(&format!(
                    "@respond status must be integer, got {other}"
                ));
            }
            Err(e) => return HttpResponse::internal_error(&e.to_string()),
        },
        None => return HttpResponse::internal_error("@respond requires a status code"),
    };

    match node.body.as_deref() {
        None => HttpResponse::json(status, ""),
        Some(body_expr) => match eval.eval_expr(body_expr) {
            Ok(val) => {
                let json_body = value_to_json_string(&val);
                HttpResponse::json(status, &json_body)
            }
            Err(e) => HttpResponse::internal_error(&e.to_string()),
        },
    }
}

fn value_to_json_string(val: &Value) -> String {
    match val {
        Value::Void => "null".to_owned(),
        Value::Bool(b) => b.to_string(),
        Value::Int(n) => n.to_string(),
        Value::Float(n) => n.to_string(),
        Value::String(s) => serde_json::to_string(s).unwrap_or_else(|_| "\"\"".to_owned()),
        Value::Array(items) => {
            let parts: Vec<String> = items.iter().map(value_to_json_string).collect();
            format!("[{}]", parts.join(","))
        }
        Value::Map(map) | Value::Object(map) => {
            let parts: Vec<String> = map
                .iter()
                .map(|(k, v)| {
                    format!(
                        "{}:{}",
                        serde_json::to_string(k).unwrap_or_default(),
                        value_to_json_string(v)
                    )
                })
                .collect();
            format!("{{{}}}", parts.join(","))
        }
        Value::Function { .. }
        | Value::BuiltinFn(_)
        | Value::Node { .. }
        | Value::RouteRef { .. } => "null".to_owned(),
    }
}

// ── @serve ────────────────────────────────────────────────────────────────────

fn eval_serve(node: &NodeExpr, eval: &mut Evaluator, _req: &HttpRequest) -> HttpResponse {
    let target = match node.positional.first() {
        Some(t) => t,
        None => return HttpResponse::internal_error("@serve requires a target"),
    };

    match target {
        // @serve @html { ... }
        Expr::Node(html_node) if is_html_node(&html_node.name) => {
            let mut html = render_orv_html_node(html_node, eval);
            // Inject design token CSS into <head>
            let design_css = eval.design_tokens_to_css();
            if !design_css.is_empty() {
                let style_tag = format!("  <style>\n{design_css}  </style>\n");
                if let Some(pos) = html.find("</head>") {
                    html.insert_str(pos, &style_tag);
                }
            }
            // Inject signal state for client-side hydration
            let snapshot = eval.signals.snapshot();
            if !snapshot.is_empty() {
                let mut json_parts = Vec::new();
                for (name, val) in &snapshot {
                    let json_key = serde_json::to_string(name).unwrap_or_default();
                    let json_val = value_to_json_string(val);
                    json_parts.push(format!("{json_key}:{json_val}"));
                }
                let signal_json = format!("{{{}}}", json_parts.join(","));
                // Use <script type="application/json"> to avoid attribute injection
                // Escape </script> sequences to prevent breakout
                let safe_json = signal_json.replace("</", "<\\/");
                let injection = format!(
                    "  <script type=\"application/json\" id=\"orv-signals\">{safe_json}</script>\n  <script src=\"/orv-runtime.js\"></script>\n"
                );
                if let Some(pos) = html.find("</body>") {
                    html.insert_str(pos, &injection);
                }
            }
            HttpResponse::html(200, &html)
        }
        // @serve "./public/file.html" or similar path string
        Expr::StringLiteral(path_str) => serve_static_file(path_str),
        // @serve /path (ident that looks like a path)
        Expr::Ident(name) if looks_like_path(&name.name) => serve_static_file(&name.name),
        other => {
            // Try evaluating as an expression and serve JSON
            match eval.eval_expr(other) {
                Ok(val) => {
                    let json = value_to_json_string(&val);
                    HttpResponse::json(200, &json)
                }
                Err(e) => HttpResponse::internal_error(&e.to_string()),
            }
        }
    }
}

fn is_html_node(name: &str) -> bool {
    matches!(
        name,
        "html"
            | "head"
            | "body"
            | "div"
            | "span"
            | "section"
            | "article"
            | "nav"
            | "main"
            | "header"
            | "footer"
            | "aside"
            | "h1"
            | "h2"
            | "h3"
            | "h4"
            | "h5"
            | "h6"
            | "p"
            | "text"
            | "button"
            | "input"
            | "form"
            | "select"
            | "option"
            | "textarea"
            | "label"
            | "a"
            | "vstack"
            | "hstack"
            | "img"
            | "video"
            | "audio"
            | "table"
            | "tr"
            | "td"
            | "th"
            | "title"
            | "meta"
            | "link"
            | "script"
            | "style"
            | "ul"
            | "ol"
            | "li"
    )
}

fn looks_like_path(name: &str) -> bool {
    name.starts_with('/') || name.starts_with("./") || name.starts_with("../")
}

fn content_type_for_path(path: &str) -> &'static str {
    if path.ends_with(".html") {
        "text/html; charset=utf-8"
    } else if path.ends_with(".css") {
        "text/css; charset=utf-8"
    } else if path.ends_with(".js") {
        "application/javascript"
    } else if path.ends_with(".json") {
        "application/json"
    } else {
        "text/plain; charset=utf-8"
    }
}

fn serve_static_file(path: &str) -> HttpResponse {
    // Confine file serving to the current working directory to prevent path traversal.
    let root = match std::env::current_dir() {
        Ok(r) => r,
        Err(_) => return HttpResponse::internal_error("unable to determine working directory"),
    };
    let requested = root.join(path);
    let canonical = match requested.canonicalize() {
        Ok(p) => p,
        Err(_) => return HttpResponse::not_found(),
    };
    let root_canonical = match root.canonicalize() {
        Ok(p) => p,
        Err(_) => return HttpResponse::not_found(),
    };
    if !canonical.starts_with(&root_canonical) {
        return HttpResponse::not_found();
    }
    match std::fs::read_to_string(&canonical) {
        Ok(content) => {
            let ct = content_type_for_path(path);
            HttpResponse {
                status: 200,
                content_type: ct.to_owned(),
                body: content,
                headers: HashMap::new(),
            }
        }
        Err(_) => HttpResponse::not_found(),
    }
}

// ── HTML rendering from eval'd HIR nodes ─────────────────────────────────────

fn render_orv_html_node(node: &NodeExpr, eval: &mut Evaluator) -> String {
    if node.name == "html" {
        // Full document: collect head and body children
        let mut head_nodes = Vec::new();
        let mut body_nodes = Vec::new();

        if let Some(Expr::Block { stmts, .. }) = node.body.as_deref() {
            for stmt in stmts {
                if let Stmt::Expr(Expr::Node(child)) = stmt {
                    let child_node = orv_node_to_html(child, eval);
                    if child.name == "head" {
                        head_nodes.push(child_node);
                    } else {
                        body_nodes.push(child_node);
                    }
                }
            }
        }

        render_document(&head_nodes, &body_nodes)
    } else {
        // Just render the node as a fragment
        let html_node = orv_node_to_html(node, eval);
        crate::html::render(&html_node)
    }
}

fn orv_node_to_html(node: &NodeExpr, eval: &mut Evaluator) -> HtmlNode {
    let tag = node_to_tag(&node.name);
    let self_closing = is_self_closing(tag);

    let mut element = HtmlNode::element(tag);

    if let Some(layout_class) = layout_classes(&node.name) {
        element = element.with_attr("style", layout_class);
    }

    if self_closing {
        // Bind attributes from properties
        for prop in &node.properties {
            if let Ok(val) = eval.eval_expr(&prop.value) {
                element = element.with_attr(&prop.name, &val.to_string());
            }
        }
        return element.self_closing();
    }

    // Bind attributes from properties
    for prop in &node.properties {
        if let Ok(val) = eval.eval_expr(&prop.value) {
            element = element.with_attr(&prop.name, &val.to_string());
        }
    }

    // Collect children from positional args (text content)
    for pos in &node.positional {
        match pos {
            Expr::StringLiteral(s) => {
                element = element.with_child(HtmlNode::Text(s.clone()));
            }
            Expr::Node(child_node) => {
                element = element.with_child(orv_node_to_html(child_node, eval));
            }
            other => {
                if let Ok(val) = eval.eval_expr(other) {
                    element = element.with_child(HtmlNode::Text(val.to_string()));
                }
            }
        }
    }

    // Collect children from body block
    if let Some(body) = node.body.as_deref() {
        let children = collect_html_children(body, eval);
        element = element.with_children(children);
    }

    element
}

fn collect_html_children(expr: &Expr, eval: &mut Evaluator) -> Vec<HtmlNode> {
    match expr {
        Expr::Block { stmts, .. } => {
            let mut children = Vec::new();
            for stmt in stmts {
                match stmt {
                    Stmt::Expr(Expr::Node(child)) => {
                        children.push(orv_node_to_html(child, eval));
                    }
                    Stmt::Expr(Expr::StringLiteral(s)) => {
                        children.push(HtmlNode::Text(s.clone()));
                    }
                    Stmt::Expr(other) => {
                        if let Ok(val) = eval.eval_expr(other) {
                            children.push(HtmlNode::Text(val.to_string()));
                        }
                    }
                    _ => {}
                }
            }
            children
        }
        Expr::Node(child) => vec![orv_node_to_html(child, eval)],
        Expr::StringLiteral(s) => vec![HtmlNode::Text(s.clone())],
        other => {
            if let Ok(val) = eval.eval_expr(other) {
                vec![HtmlNode::Text(val.to_string())]
            } else {
                Vec::new()
            }
        }
    }
}

// ── Request accessor node evaluation ─────────────────────────────────────────

/// Evaluate a node expression that may be a request accessor.
/// Returns Some(Value) if handled, None to fall through to normal eval.
pub fn eval_accessor_node(
    node: &NodeExpr,
    _eval: &mut Evaluator,
    req: &HttpRequest,
) -> Option<Value> {
    match node.name.as_str() {
        "param" => {
            let key = positional_string(&node.positional, 0)?;
            Some(
                req.path_params
                    .get(&key)
                    .map(|v| Value::String(v.clone()))
                    .unwrap_or(Value::Void),
            )
        }
        "query" => {
            let key = positional_string(&node.positional, 0)?;
            Some(
                req.query_params
                    .get(&key)
                    .map(|v| Value::String(v.clone()))
                    .unwrap_or(Value::Void),
            )
        }
        "header" => {
            let key = positional_string(&node.positional, 0)?.to_lowercase();
            Some(
                req.headers
                    .get(&key)
                    .map(|v| Value::String(v.clone()))
                    .unwrap_or(Value::Void),
            )
        }
        "body" => Some(Value::String(req.body.clone())),
        "method" => Some(Value::String(req.method.clone())),
        "path" => Some(Value::String(req.path.clone())),
        "env" => {
            let var_name = positional_string(&node.positional, 0)?;
            Some(
                std::env::var(&var_name)
                    .map(Value::String)
                    .unwrap_or(Value::Void),
            )
        }
        _ => None,
    }
}

fn positional_string(positional: &[Expr], idx: usize) -> Option<String> {
    match positional.get(idx)? {
        Expr::StringLiteral(s) => Some(s.clone()),
        Expr::Ident(n) => Some(n.name.clone()),
        _ => None,
    }
}
