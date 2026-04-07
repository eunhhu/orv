//! Live HTTP server runner for orv programs.
//!
//! Wires together the HIR evaluator, route compiler, and HTTP server to run
//! an orv program as a real HTTP server.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use orv_hir::{DefineItem, Expr, FunctionItem, ItemKind, Module, NodeExpr, Stmt, StringPart, WhenArm};
use thiserror::Error;

use crate::eval::{EvalError, Evaluator, Value, match_pattern};
use crate::html::{HtmlNode, is_self_closing, layout_classes, node_to_tag, render_document};
use crate::island::{
    self, IslandData, IslandRegistry, RuntimeFeatures, event_name_from_prop,
    is_event_handler_prop as island_is_event_prop, lower_handler_to_js, minified_runtime_source,
    render_handlers_inline, render_handlers_js, render_island_payload_json,
};
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

// ── Workspace execution ─────────────────────────────────────────────────────

/// A collected route ready to be registered with the HTTP server.
#[derive(Clone)]
struct CollectedRoute {
    method: String,
    path: String,
    body_stmts: Vec<Stmt>,
    before: Vec<DefineItem>,
    after: Vec<DefineItem>,
}

/// Run a workspace (multiple modules) as a live HTTP server.
/// `modules` is a list of `(module_name, hir)` pairs where the first is the entry.
pub fn run_workspace(
    modules: &[(String, Module)],
    port_override: Option<u16>,
) -> Result<(), RunError> {
    if modules.is_empty() {
        return Err(RunError::Message("no modules provided".to_owned()));
    }

    // Phase 1: Register all defines and functions from all modules
    let mut global_eval = Evaluator::new();
    let mut all_defines: HashMap<String, DefineItem> = HashMap::new();
    let mut all_functions: HashMap<String, FunctionItem> = HashMap::new();

    for (_name, module) in modules {
        register_module_symbols(module, &mut global_eval, &mut all_defines, &mut all_functions);
    }

    // Phase 1b: Build import-name aliases so that e.g. `@User` resolves to
    // the `UserController` define that was imported as `User`.
    register_import_aliases(modules, &mut global_eval, &mut all_defines);

    // Phase 2: Evaluate entry module top-level bindings (let port = @env PORT ?? 3000)
    let entry_module = &modules[0].1;
    eval_top_level_bindings(entry_module, &mut global_eval)?;

    // Phase 3: Find @server and execute
    let server_node = find_server_node(entry_module)
        .ok_or_else(|| RunError::Message("no top-level @server entry was found".to_owned()))?;

    // Extract port: evaluate @listen expression with global env
    let port = port_override.map_or_else(
        || extract_listen_port_eval(server_node, &global_eval),
        Ok,
    )?;

    // Phase 4: Collect routes (including route groups and define expansions)
    let routes = collect_workspace_routes(server_node, &all_defines);

    // Phase 5: Build HTTP server
    let mut http_server = HttpServer::new();
    let global_env = Arc::new(global_eval.env.clone());
    let defines = Arc::new(all_defines.clone());

    eprintln!();
    eprintln!("  orv server");
    eprintln!("  ─────────────────────────");
    eprintln!("  url:    http://127.0.0.1:{port}");
    eprintln!("  routes:");
    for r in &routes {
        eprintln!("    {} {}", r.method, r.path);
    }
    eprintln!("  ─────────────────────────");
    eprintln!();

    for route in routes {
        let method = route.method.clone();
        let path = route.path.clone();
        let env = Arc::clone(&global_env);
        let defs = Arc::clone(&defines);
        let handler: RouteHandler = Box::new(move |req: &HttpRequest| {
            handle_workspace_route(req, &route, &env, &defs)
        });
        http_server.route(&method, &path, handler);
    }

    http_server.listen(port)?;
    Ok(())
}

/// Register define/function items from a module into the evaluator.
fn register_module_symbols(
    module: &Module,
    eval: &mut Evaluator,
    defines: &mut HashMap<String, DefineItem>,
    functions: &mut HashMap<String, FunctionItem>,
) {
    for item in &module.items {
        match &item.kind {
            ItemKind::Define(define) => {
                let param_names: Vec<String> =
                    define.params.iter().map(|p| p.name.clone()).collect();
                eval.env.set(
                    define.name.clone(),
                    Value::Function {
                        params: param_names,
                        body: Box::new(define.body.clone()),
                        env: eval.env.clone(),
                    },
                );
                defines.insert(define.name.clone(), define.clone());
            }
            ItemKind::Function(func) => {
                let param_names: Vec<String> =
                    func.params.iter().map(|p| p.name.clone()).collect();
                eval.env.set(
                    func.name.clone(),
                    Value::Function {
                        params: param_names,
                        body: Box::new(func.body.clone()),
                        env: eval.env.clone(),
                    },
                );
                functions.insert(func.name.clone(), func.clone());
            }
            _ => {}
        }
    }
}

/// Build import-name → define-name aliases.
///
/// When `import routes.user.{User}` is encountered and the `routes/user/index`
/// module has `pub define UserController`, we register `UserController` under
/// the alias `User` so that `@User` in the entry module resolves correctly.
fn register_import_aliases(
    modules: &[(String, Module)],
    eval: &mut Evaluator,
    defines: &mut HashMap<String, DefineItem>,
) {
    // Collect pub defines per module: module_name → Vec<define_name>
    let mut module_pub_defines: HashMap<String, Vec<String>> = HashMap::new();
    for (name, module) in modules {
        let pub_defs: Vec<String> = module
            .items
            .iter()
            .filter_map(|item| match &item.kind {
                ItemKind::Define(d) if d.is_pub => Some(d.name.clone()),
                _ => None,
            })
            .collect();
        module_pub_defines.insert(name.clone(), pub_defs);
    }

    // For each module, look at its imports and build aliases
    for (_name, module) in modules {
        for item in &module.items {
            if let ItemKind::Import(import) = &item.kind {
                for import_name in &import.names {
                    // Skip sub-path imports like "count.userCount"
                    if import_name.contains('.') {
                        continue;
                    }
                    // Skip if already registered
                    if defines.contains_key(import_name) {
                        continue;
                    }

                    let import_path_str = import.path.join("/");

                    // Look for the module that matches the import path.
                    // Module names may be like "routes/user/index.orv" or "routes/user/index"
                    for (mod_name, pub_defs) in &module_pub_defines {
                        // Normalize: strip .orv extension, replace dots with slashes
                        let mod_clean = mod_name
                            .trim_end_matches(".orv")
                            .replace('.', "/");
                        // Strip trailing /index if present
                        let mod_stem = mod_clean
                            .strip_suffix("/index")
                            .unwrap_or(&mod_clean);

                        let is_match = mod_stem == import_path_str
                            || mod_stem.ends_with(&import_path_str);
                        if !is_match {
                            continue;
                        }
                        // Found the target module. If the import name doesn't
                        // match any pub define, alias the first pub define.
                        if !pub_defs.contains(import_name)
                            && pub_defs.len() == 1
                            && let Some(define) = defines.get(&pub_defs[0]).cloned()
                        {
                            let param_names: Vec<String> =
                                define.params.iter().map(|p| p.name.clone()).collect();
                            eval.env.set(
                                import_name.clone(),
                                Value::Function {
                                    params: param_names,
                                    body: Box::new(define.body.clone()),
                                    env: eval.env.clone(),
                                },
                            );
                            defines.insert(import_name.clone(), define);
                        }
                        break;
                    }
                }
            }
        }
    }
}

/// Evaluate top-level bindings (let, const) in the entry module.
fn eval_top_level_bindings(module: &Module, eval: &mut Evaluator) -> Result<(), RunError> {
    for item in &module.items {
        if let ItemKind::Binding(binding) = &item.kind {
            let val = match &binding.value {
                Some(expr) => eval_with_env_nodes(expr, eval)?,
                None => Value::Void,
            };
            eval.env.set(binding.name.clone(), val);
        }
    }
    Ok(())
}

/// Evaluate an expression, handling @env nodes specially.
fn eval_with_env_nodes(expr: &Expr, eval: &mut Evaluator) -> Result<Value, RunError> {
    match expr {
        Expr::Node(node) if node.name == "env" => {
            let var_name = node
                .positional
                .first()
                .and_then(|e| match e {
                    Expr::Ident(r) => Some(r.name.clone()),
                    Expr::StringLiteral(s) => Some(s.clone()),
                    _ => None,
                })
                .ok_or_else(|| RunError::Message("@env requires a variable name".to_owned()))?;
            Ok(std::env::var(&var_name)
                .map(Value::String)
                .unwrap_or(Value::Void))
        }
        Expr::Binary {
            left,
            op: orv_hir::BinaryOp::NullCoalesce,
            right,
        } => {
            let lv = eval_with_env_nodes(left, eval)?;
            match lv {
                Value::Void => eval_with_env_nodes(right, eval),
                other => Ok(other),
            }
        }
        other => Ok(eval.eval_expr(other)?),
    }
}

/// Extract port from @listen, evaluating the expression with the global env.
fn extract_listen_port_eval(
    server_node: &NodeExpr,
    global_eval: &Evaluator,
) -> Result<u16, RunError> {
    let stmts = server_body_stmts(server_node)?;
    for stmt in stmts {
        if let Stmt::Expr(Expr::Node(child)) = stmt
            && child.name == "listen"
            && let Some(port_expr) = child.positional.first()
        {
            // Create a temporary evaluator with the global env to resolve variables
            let mut eval = Evaluator::new();
            eval.env = global_eval.env.clone();
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

/// Collect routes from @server body, expanding route groups and define references.
fn collect_workspace_routes(
    server_node: &NodeExpr,
    defines: &HashMap<String, DefineItem>,
) -> Vec<CollectedRoute> {
    let Ok(stmts) = server_body_stmts(server_node) else {
        return Vec::new();
    };
    collect_routes_from_stmts(stmts, "", &[], &[], defines)
}

/// Recursively collect routes from a list of statements.
/// `prefix` is the accumulated path prefix from parent route groups.
/// `parent_before`/`parent_after` are inherited middleware.
fn collect_routes_from_stmts(
    stmts: &[Stmt],
    prefix: &str,
    parent_before: &[DefineItem],
    parent_after: &[DefineItem],
    defines: &HashMap<String, DefineItem>,
) -> Vec<CollectedRoute> {
    let mut routes = Vec::new();
    let mut local_before: Vec<DefineItem> = parent_before.to_vec();
    let mut local_after: Vec<DefineItem> = parent_after.to_vec();

    for stmt in stmts {
        match stmt {
            // @route ...
            Stmt::Expr(Expr::Node(node)) if node.name == "route" => {
                match node.positional.len() {
                    // Route group: @route /prefix { ... }
                    1 => {
                        let group_path = positional_atom(&node.positional[0]).unwrap_or_default();
                        let new_prefix = format!("{prefix}{group_path}");
                        if let Some(Expr::Block { stmts, .. }) = node.body.as_deref() {
                            let sub = collect_routes_from_stmts(
                                stmts,
                                &new_prefix,
                                &local_before,
                                &local_after,
                                defines,
                            );
                            routes.extend(sub);
                        }
                    }
                    // Full route: @route METHOD /path { ... }
                    2 => {
                        let method =
                            positional_atom(&node.positional[0]).unwrap_or_else(|| "GET".into());
                        let path = positional_atom(&node.positional[1]).unwrap_or_default();
                        let full_path = format!("{prefix}{path}");
                        let body_stmts = match node.body.as_deref() {
                            Some(Expr::Block { stmts, .. }) => stmts.clone(),
                            _ => Vec::new(),
                        };
                        routes.push(CollectedRoute {
                            method,
                            path: full_path,
                            body_stmts,
                            before: local_before.clone(),
                            after: local_after.clone(),
                        });
                    }
                    _ => {}
                }
            }
            // @node reference — could be a define that produces routes or middleware
            Stmt::Expr(Expr::Node(node)) => {
                if let Some(define) = defines.get(&node.name) {
                    match define.return_domain.as_deref() {
                        // Route define: expand into routes
                        Some("route") => {
                            let expanded =
                                expand_route_define(define, prefix, &local_before, &local_after, defines);
                            routes.extend(expanded);
                        }
                        // Middleware: add to before/after lists
                        Some("before") => {
                            local_before.push(define.clone());
                        }
                        Some("after") => {
                            local_after.push(define.clone());
                        }
                        _ => {}
                    }
                }
            }
            // Bindings in route groups
            Stmt::Binding(_) => {}
            _ => {}
        }
    }

    routes
}

/// Expand a route-domain define into routes.
fn expand_route_define(
    define: &DefineItem,
    prefix: &str,
    parent_before: &[DefineItem],
    parent_after: &[DefineItem],
    all_defines: &HashMap<String, DefineItem>,
) -> Vec<CollectedRoute> {
    // A route define's body is essentially a @route node.
    // The define's positional args come from the DefineItem metadata.
    // But the body itself contains the route's statements.
    //
    // For defines like:
    //   pub define Health() -> @route GET /health { @respond 200 "OK" }
    // The body is a Block containing @respond.
    //
    // For defines like:
    //   pub define User() -> @route /user { @List @Delete }
    // The body is a Block with nested route references.
    //
    // The return_domain tells us it's a route, and the body was parsed
    // as a @route node in the HIR.

    match &define.body {
        Expr::Block { stmts, .. } => {
            // Check if stmts contain @route nodes or direct actions
            let has_routes = stmts.iter().any(|s| {
                matches!(s, Stmt::Expr(Expr::Node(n)) if n.name == "route")
            });

            if has_routes {
                // This is a route group define — expand its routes
                collect_routes_from_stmts(stmts, prefix, parent_before, parent_after, all_defines)
            } else {
                // Check if there are node references that are route defines
                let has_route_defines = stmts.iter().any(|s| {
                    if let Stmt::Expr(Expr::Node(n)) = s {
                        all_defines
                            .get(&n.name)
                            .is_some_and(|d| d.return_domain.as_deref() == Some("route"))
                    } else {
                        false
                    }
                });

                if has_route_defines {
                    collect_routes_from_stmts(
                        stmts,
                        prefix,
                        parent_before,
                        parent_after,
                        all_defines,
                    )
                } else {
                    // This is a leaf route with action statements
                    // Need to extract method/path from the original define
                    // Route defines have form: define X() -> @route METHOD /path { body }
                    // But after HIR lowering, the body is the block content
                    // and the route metadata is in the define's positional encoding.
                    //
                    // Actually, route defines are structured differently.
                    // Let's look at the body as the route handler.
                    vec![CollectedRoute {
                        method: "*".into(),
                        path: prefix.to_owned(),
                        body_stmts: stmts.clone(),
                        before: parent_before.to_vec(),
                        after: parent_after.to_vec(),
                    }]
                }
            }
        }
        Expr::Node(node) if node.name == "route" || is_route_like_node(&node.name) => {
            // Direct route node in the define body
            let mut routes = Vec::new();
            match node.positional.len() {
                1 => {
                    let group_path = positional_atom(&node.positional[0]).unwrap_or_default();
                    let new_prefix = format!("{prefix}{group_path}");
                    if let Some(Expr::Block { stmts, .. }) = node.body.as_deref() {
                        routes.extend(collect_routes_from_stmts(
                            stmts,
                            &new_prefix,
                            parent_before,
                            parent_after,
                            all_defines,
                        ));
                    }
                }
                n if n >= 2 => {
                    let method =
                        positional_atom(&node.positional[0]).unwrap_or_else(|| "GET".into());
                    let path = positional_atom(&node.positional[1]).unwrap_or_default();
                    let full_path = format!("{prefix}{path}");
                    let body_stmts = match node.body.as_deref() {
                        Some(Expr::Block { stmts, .. }) => stmts.clone(),
                        _ => Vec::new(),
                    };
                    routes.push(CollectedRoute {
                        method,
                        path: full_path,
                        body_stmts,
                        before: parent_before.to_vec(),
                        after: parent_after.to_vec(),
                    });
                }
                _ => {}
            }
            routes
        }
        _ => Vec::new(),
    }
}

fn is_route_like_node(_name: &str) -> bool {
    false
}

// ── Workspace request handler ────────────────────────────────────────────────

fn handle_workspace_route(
    req: &HttpRequest,
    route: &CollectedRoute,
    global_env: &crate::eval::Env,
    defines: &HashMap<String, DefineItem>,
) -> HttpResponse {
    let mut eval = Evaluator::new();
    eval.env = global_env.clone();
    eval.env.push_scope();

    // Bind request accessors
    bind_request_accessors(&mut eval, req);

    // Run @before middleware
    for before in &route.before {
        if let Some(response) = run_middleware(before, &mut eval, req, defines) {
            return response;
        }
    }

    // Evaluate route body
    let response = eval_route_body(&route.body_stmts, &mut eval, req, defines);

    // Run @after middleware
    for after in &route.after {
        if let Some(resp) = run_middleware(after, &mut eval, req, defines) {
            return resp;
        }
    }

    eval.env.pop_scope();
    response
}

/// Run a middleware define. Returns Some(HttpResponse) if middleware short-circuits.
fn run_middleware(
    define: &DefineItem,
    eval: &mut Evaluator,
    req: &HttpRequest,
    defines: &HashMap<String, DefineItem>,
) -> Option<HttpResponse> {
    eval.env.push_scope();

    // Evaluate middleware params with defaults
    for param in &define.params {
        if let Some(default) = &param.default
            && let Ok(val) = eval.eval_expr(default)
        {
            eval.env.set(param.name.clone(), val);
        }
    }

    let result = eval_stmts_for_response(&extract_block_stmts(&define.body), eval, req, defines);
    eval.env.pop_scope();
    result
}

/// Evaluate a list of statements looking for @respond/@serve.
fn eval_route_body(
    stmts: &[Stmt],
    eval: &mut Evaluator,
    req: &HttpRequest,
    defines: &HashMap<String, DefineItem>,
) -> HttpResponse {
    match eval_stmts_for_response(stmts, eval, req, defines) {
        Some(response) => response,
        None => HttpResponse::internal_error("route body did not produce a response"),
    }
}

/// Evaluate statements until an @respond or @serve is found.
/// Handles define calls, if/else branching, etc.
fn eval_stmts_for_response(
    stmts: &[Stmt],
    eval: &mut Evaluator,
    req: &HttpRequest,
    defines: &HashMap<String, DefineItem>,
) -> Option<HttpResponse> {
    for stmt in stmts {
        match stmt {
            Stmt::Expr(Expr::Node(node)) if node.name == "respond" => {
                return Some(eval_respond(node, eval));
            }
            Stmt::Expr(Expr::Node(node)) if node.name == "serve" => {
                return Some(eval_serve(node, eval, req, defines));
            }
            // Request accessor nodes: @param, @query, @header, @body, @method, @path, @env, @cookie, @context
            Stmt::Expr(Expr::Node(node)) if is_accessor_node(&node.name) => {
                if let Some(val) = eval_accessor_in_context(node, eval, req) {
                    // If it's a standalone accessor, just eval for side effect
                    let _ = val;
                }
            }
            // Define call as node: @Health, @User, etc.
            Stmt::Expr(Expr::Node(node)) if defines.contains_key(&node.name) => {
                let define = &defines[&node.name];
                match define.return_domain.as_deref() {
                    Some("before") | Some("after") => {
                        // Inline middleware — run it
                        if let Some(response) = run_middleware(define, eval, req, defines) {
                            return Some(response);
                        }
                    }
                    Some("route") => {
                        // Route define — should have been expanded during route collection
                        // but handle inline for safety
                    }
                    Some("html") => {
                        // HTML define — evaluate and ignore (used for rendering)
                    }
                    Some("design") => {
                        // Design token define — extract tokens
                        eval.env.push_scope();
                        let _ = eval.eval_expr(&define.body);
                        eval.env.pop_scope();
                    }
                    _ => {
                        // Generic define call
                        eval.env.push_scope();
                        // Bind positional args to params
                        for (param, arg) in define.params.iter().zip(node.positional.iter()) {
                            if let Ok(val) = eval.eval_expr(arg) {
                                eval.env.set(param.name.clone(), val);
                            }
                        }
                        if let Some(resp) =
                            eval_stmts_for_response(&extract_block_stmts(&define.body), eval, req, defines)
                        {
                            eval.env.pop_scope();
                            return Some(resp);
                        }
                        eval.env.pop_scope();
                    }
                }
            }
            Stmt::If(if_stmt) => {
                let cond = eval
                    .eval_expr(&if_stmt.condition)
                    .unwrap_or(Value::Bool(false));
                let is_truthy = match &cond {
                    Value::Bool(b) => *b,
                    Value::Void => false,
                    Value::Int(n) => *n != 0,
                    Value::String(s) => !s.is_empty(),
                    _ => true,
                };
                if is_truthy {
                    let then_stmts = extract_block_stmts(&if_stmt.then_body);
                    if let Some(resp) =
                        eval_stmts_for_response(&then_stmts, eval, req, defines)
                    {
                        return Some(resp);
                    }
                } else if let Some(else_body) = &if_stmt.else_body {
                    let else_stmts = extract_block_stmts(else_body);
                    if let Some(resp) =
                        eval_stmts_for_response(&else_stmts, eval, req, defines)
                    {
                        return Some(resp);
                    }
                }
            }
            // Bindings — intercept accessor nodes in value expressions
            Stmt::Binding(binding) => {
                let val = if let Some(value_expr) = &binding.value {
                    eval_expr_with_accessors(value_expr, eval, req)
                } else {
                    Value::Void
                };
                eval.env.set(binding.name.clone(), val);
            }
            // Regular statement — evaluate for side effects
            _ => {
                let _ = eval.eval_stmt(stmt);
            }
        }
    }
    None
}

/// Evaluate an expression, resolving request accessor nodes and handling
/// unknown domain nodes gracefully (returning Void instead of error).
fn eval_expr_with_accessors(
    expr: &Expr,
    eval: &mut Evaluator,
    req: &HttpRequest,
) -> Value {
    match expr {
        // Direct accessor: @query skip, @param id, @env PORT, etc.
        Expr::Node(node) if is_accessor_node(&node.name) => {
            eval_accessor_in_context(node, eval, req).unwrap_or(Value::Void)
        }
        // Null-coalesce: @query skip ?? 0
        Expr::Binary {
            left,
            op: orv_hir::BinaryOp::NullCoalesce,
            right,
        } => {
            let lv = eval_expr_with_accessors(left, eval, req);
            match lv {
                Value::Void => eval_expr_with_accessors(right, eval, req),
                other => other,
            }
        }
        // Unknown domain nodes like @db.find, @transaction — return Void
        Expr::Node(node) if node.name.contains('.') || is_domain_node(&node.name) => {
            Value::Void
        }
        // Await expressions — unwrap and evaluate inner
        Expr::Await(inner) => eval_expr_with_accessors(inner, eval, req),
        // Everything else — use the regular evaluator
        other => eval.eval_expr(other).unwrap_or(Value::Void),
    }
}

fn is_domain_node(name: &str) -> bool {
    matches!(
        name,
        "db" | "transaction" | "io" | "fs" | "http" | "ws" | "dom" | "process" | "token"
    )
}

fn extract_block_stmts(expr: &Expr) -> Vec<Stmt> {
    match expr {
        Expr::Block { stmts, .. } => stmts.clone(),
        // Single expression — wrap as statement
        other => vec![Stmt::Expr(other.clone())],
    }
}

fn is_accessor_node(name: &str) -> bool {
    let first = name.split('.').next().unwrap_or(name);
    matches!(
        first,
        "param" | "query" | "header" | "body" | "method" | "path" | "env" | "cookie" | "context" | "request"
    )
}

/// Evaluate an accessor node in request context.
fn eval_accessor_in_context(
    node: &NodeExpr,
    eval: &mut Evaluator,
    req: &HttpRequest,
) -> Option<Value> {
    let first = node.name.split('.').next().unwrap_or(&node.name);
    match first {
        "env" => {
            let var_name = positional_string(&node.positional, 0)?;
            Some(
                std::env::var(&var_name)
                    .map(Value::String)
                    .unwrap_or(Value::Void),
            )
        }
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
        "cookie" => {
            // Parse cookie from Cookie header
            let cookie_name = positional_string(&node.positional, 0)?;
            let cookie_header = req.headers.get("cookie").cloned().unwrap_or_default();
            let value = cookie_header
                .split(';')
                .filter_map(|pair| {
                    let mut parts = pair.trim().splitn(2, '=');
                    let name = parts.next()?.trim();
                    let val = parts.next()?.trim();
                    if name == cookie_name {
                        Some(val.to_owned())
                    } else {
                        None
                    }
                })
                .next();
            Some(value.map(Value::String).unwrap_or(Value::Void))
        }
        "body" => Some(Value::String(req.body.clone())),
        "method" => Some(Value::String(req.method.clone())),
        "path" => {
            // @path returns array of path segments
            let segments: Vec<Value> = req
                .path
                .split('/')
                .filter(|s| !s.is_empty())
                .map(|s| Value::String(s.to_owned()))
                .collect();
            Some(Value::Array(segments))
        }
        "request" => {
            let sub = node.name.strip_prefix("request.").unwrap_or("");
            match sub {
                "ip" => Some(Value::String("127.0.0.1".to_owned())),
                _ => Some(Value::Void),
            }
        }
        "context" => {
            // @context { key } — set context
            // @context.key — get context
            if node.name.contains('.') {
                let key = node.name.strip_prefix("context.").unwrap_or("");
                Some(
                    eval.env
                        .get(&format!("__ctx_{key}"))
                        .cloned()
                        .unwrap_or(Value::Void),
                )
            } else if let Some(body) = &node.body {
                // @context { payload } — set context from body fields
                if let Expr::Block { stmts, .. } = body.as_ref() {
                    for s in stmts {
                        if let Stmt::Expr(Expr::Ident(resolved)) = s {
                            let val = eval
                                .env
                                .get(&resolved.name)
                                .cloned()
                                .unwrap_or(Value::Void);
                            eval.env
                                .set(format!("__ctx_{}", resolved.name), val);
                        }
                    }
                }
                Some(Value::Void)
            } else {
                let key = positional_string(&node.positional, 0)?;
                Some(
                    eval.env
                        .get(&format!("__ctx_{key}"))
                        .cloned()
                        .unwrap_or(Value::Void),
                )
            }
        }
        _ => None,
    }
}

// ── Legacy single-module entry points (kept for backward compat) ─────────────

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

// ── Legacy request handler ───────────────────────────────────────────────────

fn handle_route(
    req: &HttpRequest,
    body_stmts: &[Stmt],
    env_bindings: &HashMap<String, String>,
) -> HttpResponse {
    let mut eval = Evaluator::new();

    for (name, value) in env_bindings {
        eval.env.set(name.clone(), Value::String(value.clone()));
    }

    bind_request_accessors(&mut eval, req);

    for stmt in body_stmts {
        if let Some(response) = try_action_stmt(stmt, &mut eval, req) {
            return response;
        }
        if let Stmt::Expr(Expr::Node(node)) = stmt
            && (node.name == "respond" || node.name == "serve")
        {
            continue;
        }
        let _ = eval.eval_stmt(stmt);
    }

    HttpResponse::internal_error("route body did not produce a response")
}

/// Bind the request accessor magic variables into the evaluator environment.
fn bind_request_accessors(eval: &mut Evaluator, req: &HttpRequest) {
    eval.env
        .set("__method".to_owned(), Value::String(req.method.clone()));
    eval.env
        .set("__path".to_owned(), Value::String(req.path.clone()));
    eval.env
        .set("__body".to_owned(), Value::String(req.body.clone()));

    let path_params: HashMap<String, Value> = req
        .path_params
        .iter()
        .map(|(k, v)| (k.clone(), Value::String(v.clone())))
        .collect();
    eval.env
        .set("__path_params".to_owned(), Value::Map(path_params));

    let query_params: HashMap<String, Value> = req
        .query_params
        .iter()
        .map(|(k, v)| (k.clone(), Value::String(v.clone())))
        .collect();
    eval.env
        .set("__query_params".to_owned(), Value::Map(query_params));

    let headers: HashMap<String, Value> = req
        .headers
        .iter()
        .map(|(k, v)| (k.clone(), Value::String(v.clone())))
        .collect();
    eval.env.set("__headers".to_owned(), Value::Map(headers));
}

fn try_action_stmt(stmt: &Stmt, eval: &mut Evaluator, req: &HttpRequest) -> Option<HttpResponse> {
    let Stmt::Expr(Expr::Node(node)) = stmt else {
        return None;
    };
    match node.name.as_str() {
        "respond" => Some(eval_respond(node, eval)),
        "serve" => Some(eval_serve_legacy(node, eval, req)),
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

    // Check for inline body (2nd positional): @respond 200 "OK"
    if let Some(inline_body) = node.positional.get(1) {
        return match eval.eval_expr(inline_body) {
            Ok(val) => {
                let json_body = value_to_json_string(&val);
                HttpResponse::json(status, &json_body)
            }
            Err(e) => HttpResponse::internal_error(&e.to_string()),
        };
    }

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

fn eval_serve(
    node: &NodeExpr,
    eval: &mut Evaluator,
    req: &HttpRequest,
    defines: &HashMap<String, DefineItem>,
) -> HttpResponse {
    let target = match node.positional.first() {
        Some(t) => t,
        None => return HttpResponse::internal_error("@serve requires a target"),
    };

    match target {
        // @serve @html { ... }
        Expr::Node(html_node) if is_html_node(&html_node.name) => {
            // Live server path: derive island defines on the fly so the same
            // hydration assets get served as the prerender output. This is
            // O(defines) per request which is acceptable for development.
            let island_defines: HashSet<String> = defines
                .values()
                .filter(|d| {
                    d.return_domain.as_deref() == Some("html")
                        && island_define_has_client(d)
                })
                .map(|d| d.name.clone())
                .collect();
            let mut islands = IslandRegistry::new();
            let mut ctx = RenderCtx::new(defines, &island_defines, &mut islands);
            let mut html = render_orv_html_node_workspace(html_node, eval, req, &mut ctx);
            let collected = islands.into_islands();
            inject_design_and_islands(&mut html, eval, &collected);
            HttpResponse::html(200, &html)
        }
        Expr::StringLiteral(path_str) => serve_static_file(path_str),
        Expr::Ident(name) if looks_like_path(&name.name) => serve_static_file(&name.name),
        other => match eval.eval_expr(other) {
            Ok(val) => {
                let json = value_to_json_string(&val);
                HttpResponse::json(200, &json)
            }
            Err(e) => HttpResponse::internal_error(&e.to_string()),
        },
    }
}

/// True if a `@html` define contains client-only constructs (sigs, event
/// handler props, signal interpolation). Light wrapper around the AST walk
/// in `island::collect_island_defines` for single defines.
fn island_define_has_client(define: &DefineItem) -> bool {
    let module = Module {
        items: vec![orv_hir::Item {
            symbol: None,
            kind: ItemKind::Define(define.clone()),
        }],
    };
    let set = island::collect_island_defines(&[(String::new(), module)]);
    set.contains(&define.name)
}

fn eval_serve_legacy(
    node: &NodeExpr,
    eval: &mut Evaluator,
    _req: &HttpRequest,
) -> HttpResponse {
    let target = match node.positional.first() {
        Some(t) => t,
        None => return HttpResponse::internal_error("@serve requires a target"),
    };

    match target {
        Expr::Node(html_node) if is_html_node(&html_node.name) => {
            let mut html = render_orv_html_node(html_node, eval);
            inject_design_and_signals(&mut html, eval);
            HttpResponse::html(200, &html)
        }
        Expr::StringLiteral(path_str) => serve_static_file(path_str),
        Expr::Ident(name) if looks_like_path(&name.name) => serve_static_file(&name.name),
        other => match eval.eval_expr(other) {
            Ok(val) => {
                let json = value_to_json_string(&val);
                HttpResponse::json(200, &json)
            }
            Err(e) => HttpResponse::internal_error(&e.to_string()),
        },
    }
}

/// Returns true for property names that should be treated as client-side
/// event handlers (e.g. `onClick`, `onChange`, `onInput`, `onSubmit`).
///
/// Such props must never be evaluated during SSR — their bodies often perform
/// signal mutations and should instead be lowered into client JS by the island
/// pass.
fn is_event_handler_prop(name: &str) -> bool {
    name.starts_with("on") && name.len() > 2 && name.as_bytes()[2].is_ascii_uppercase()
}

/// Inject only public assets (design tokens CSS) into the rendered HTML.
///
/// Signal state and handler bodies are intentionally NOT serialized into the
/// HTML — that path was a serious data-exposure vector (raw DB results, server
/// state, and arbitrary handler source could leak to clients). Hydration is
/// handled by the island system instead, which only ships pre-declared props
/// for explicitly client-side defines.
fn inject_design_and_signals(html: &mut String, eval: &Evaluator) {
    let design_css = eval.design_tokens_to_css();
    if !design_css.is_empty() {
        let style_tag = format!("  <style>\n{design_css}  </style>\n");
        if let Some(pos) = html.find("</head>") {
            html.insert_str(pos, &style_tag);
        }
    }
}

/// Inject the design CSS plus island hydration assets, *minimized* and
/// tree-shaken to the smallest payload that still works.
///
/// Optimizations applied:
///   - **CSS tree-shaking**: only design tokens that the rendered HTML
///     actually references via `var(--orv-…)` survive. If none survive,
///     the `<style>` block is omitted entirely.
///   - **Runtime feature flags**: the runtime is regenerated per-page and
///     drops text-binding code when no sigs exist, drops handler-binding
///     code when no handlers exist, and drops `split(',')` logic when
///     every element has at most one handler.
///   - **Inline handlers**: handlers ship as a single
///     `window.__orvHandlers={…};` assignment with no module ceremony,
///     no comments, no `"use strict"` banner.
///   - **HTML compaction**: the entire response is whitespace-collapsed
///     in a final pass before serialization.
///
/// Even on a 1 GHz mobile chip over 24 kbit/s, the resulting page should
/// be small enough to deliver in well under a second.
fn inject_design_and_islands(html: &mut String, eval: &Evaluator, islands: &[IslandData]) {
    // ── CSS: tree-shake to only the tokens used in the rendered HTML ────
    let used = collect_used_css_vars(html);
    let design_css = eval.design_tokens_to_css_filtered(&used);
    let mut head_blob = String::new();
    if !design_css.is_empty() {
        head_blob.push_str("<style>");
        head_blob.push_str(&design_css);
        head_blob.push_str("</style>");
    }

    // ── JS: feature-flagged hydration ──────────────────────────────────
    let body_blob = if islands.is_empty() {
        String::new()
    } else {
        // Conservative: assume no element uses multi-handler joining
        // unless it actually does. Currently the SSR walker emits one
        // handler per element, so this stays false. If multi-handler
        // joining is added later, set this to true at the call site.
        let features = RuntimeFeatures::analyze(islands, false);
        let runtime_js = minified_runtime_source(features);
        if runtime_js.is_empty() {
            // No feature requires JS — emit nothing.
            String::new()
        } else {
            let payload = render_island_payload_json(islands);
            let handlers_inline = render_handlers_inline(islands);
            let mut blob = String::new();
            blob.push_str("<script type=\"application/json\" id=\"orv-islands\">");
            blob.push_str(&payload);
            blob.push_str("</script>");
            // Single inline classic script: handlers assignment + runtime,
            // executed in source order, no module overhead.
            blob.push_str("<script>");
            if !handlers_inline.is_empty() {
                blob.push_str(&handlers_inline);
            }
            blob.push_str(&runtime_js);
            blob.push_str("</script>");
            blob
        }
    };

    if !head_blob.is_empty()
        && let Some(pos) = html.find("</head>")
    {
        html.insert_str(pos, &head_blob);
    }
    if !body_blob.is_empty() {
        if let Some(pos) = html.find("</body>") {
            html.insert_str(pos, &body_blob);
        } else {
            html.push_str(&body_blob);
        }
    }

    // ── Final whitespace pass: collapse cosmetic indentation/newlines ──
    *html = compact_html(html);
}

/// Scan a rendered HTML string for `var(--…)` references and return the
/// set of CSS custom property names actually in use. Property names include
/// the leading `--`.
fn collect_used_css_vars(html: &str) -> HashSet<String> {
    let mut out = HashSet::new();
    let bytes = html.as_bytes();
    let mut i = 0;
    while i + 6 < bytes.len() {
        // Look for `var(--`
        if &bytes[i..i + 6] == b"var(--" {
            let start = i + 4; // points at the first `-`
            let mut end = start;
            while end < bytes.len() {
                let c = bytes[end];
                if c.is_ascii_alphanumeric() || c == b'-' || c == b'_' {
                    end += 1;
                } else {
                    break;
                }
            }
            if end > start {
                if let Ok(s) = std::str::from_utf8(&bytes[start..end]) {
                    out.insert(s.to_owned());
                }
            }
            i = end;
        } else {
            i += 1;
        }
    }
    out
}

/// Whitespace-compact a rendered HTML document.
///
/// We never touch the inside of `<script>`, `<style>`, `<pre>`, or
/// `<textarea>` tags (their content is whitespace-significant). Outside
/// those, runs of whitespace between tags are collapsed.
fn compact_html(html: &str) -> String {
    let bytes = html.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    let preserve_tags: &[&[u8]] = &[b"script", b"style", b"pre", b"textarea"];
    while i < bytes.len() {
        let c = bytes[i];
        if c == b'<' {
            // Check for a preserve-content tag opener.
            let tag_start = i + 1;
            let mut tag_end = tag_start;
            while tag_end < bytes.len()
                && (bytes[tag_end].is_ascii_alphabetic() || bytes[tag_end].is_ascii_digit())
            {
                tag_end += 1;
            }
            let tag_name = &bytes[tag_start..tag_end];
            let preserve = preserve_tags
                .iter()
                .any(|t| tag_name.eq_ignore_ascii_case(t));
            if preserve {
                // Find matching `</tag>` (case-insensitive) and copy verbatim.
                let mut close = Vec::with_capacity(tag_name.len() + 3);
                close.extend_from_slice(b"</");
                close.extend_from_slice(tag_name);
                let close_lower: Vec<u8> =
                    close.iter().map(|b| b.to_ascii_lowercase()).collect();
                let mut j = tag_end;
                while j + close_lower.len() <= bytes.len() {
                    let slice: Vec<u8> = bytes[j..j + close_lower.len()]
                        .iter()
                        .map(|b| b.to_ascii_lowercase())
                        .collect();
                    if slice == close_lower {
                        break;
                    }
                    j += 1;
                }
                let end = if j + close_lower.len() <= bytes.len() {
                    let mut k = j;
                    while k < bytes.len() && bytes[k] != b'>' {
                        k += 1;
                    }
                    if k < bytes.len() { k + 1 } else { k }
                } else {
                    bytes.len()
                };
                out.extend_from_slice(&bytes[i..end]);
                i = end;
                continue;
            }
            // Otherwise: copy this tag verbatim up to and including its `>`.
            let mut k = i;
            while k < bytes.len() && bytes[k] != b'>' {
                k += 1;
            }
            if k < bytes.len() {
                k += 1;
            }
            out.extend_from_slice(&bytes[i..k]);
            i = k;
            continue;
        }
        // Outside any tag: collapse runs of whitespace.
        if c.is_ascii_whitespace() {
            let prev = out.last().copied();
            while i < bytes.len() && bytes[i].is_ascii_whitespace() {
                i += 1;
            }
            let next = bytes.get(i).copied();
            // Only drop whitespace entirely when it sits *between two tags*
            // (or at the document edges). Whitespace adjacent to text on
            // either side is semantically significant — `Click: <span>`
            // must keep the space before `<span>` because the rendered
            // text node is `Click: ` followed by the span's content.
            let prev_is_tag_or_edge = matches!(prev, Some(b'>') | None);
            let next_is_tag_or_edge = matches!(next, Some(b'<') | None);
            if prev_is_tag_or_edge && next_is_tag_or_edge {
                // Pure inter-tag whitespace — safe to drop.
                continue;
            }
            // Otherwise collapse to a single space.
            out.push(b' ');
            continue;
        }
        out.push(c);
        i += 1;
    }
    String::from_utf8(out).unwrap_or_else(|_| html.to_owned())
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

// ── SSR render context ───────────────────────────────────────────────────────

/// Context threaded through the workspace HTML render pass.
///
/// Bundles the things every render helper needs so signatures don't grow
/// unbounded as the island system gains responsibilities. The runtime path
/// (live HTTP) and the prerender path both build one of these.
pub(crate) struct RenderCtx<'a> {
    pub(crate) defines: &'a HashMap<String, DefineItem>,
    pub(crate) island_defines: &'a HashSet<String>,
    pub(crate) islands: &'a mut IslandRegistry,
}

impl<'a> RenderCtx<'a> {
    fn new(
        defines: &'a HashMap<String, DefineItem>,
        island_defines: &'a HashSet<String>,
        islands: &'a mut IslandRegistry,
    ) -> Self {
        Self {
            defines,
            island_defines,
            islands,
        }
    }
}

// ── Workspace HTML rendering ─────────────────────────────────────────────────

fn render_orv_html_node_workspace(
    node: &NodeExpr,
    eval: &mut Evaluator,
    req: &HttpRequest,
    ctx: &mut RenderCtx<'_>,
) -> String {
    if node.name == "html" {
        let mut head_nodes = Vec::new();
        let mut body_nodes = Vec::new();

        if let Some(Expr::Block { stmts, .. }) = node.body.as_deref() {
            for stmt in stmts {
                if let Stmt::Expr(Expr::Node(child)) = stmt {
                    if child.name == "head" || child.name == "body" {
                        // Extract the children of @head/@body rather than
                        // wrapping them in another <head>/<body> tag —
                        // render_document already provides the outer tags.
                        let children = if let Some(body_expr) = child.body.as_deref() {
                            collect_html_children_ws(body_expr, eval, req, ctx)
                        } else {
                            Vec::new()
                        };
                        // Also collect positional children (e.g. @title "text")
                        let mut pos_children: Vec<HtmlNode> = Vec::new();
                        for pos in &child.positional {
                            match pos {
                                Expr::StringLiteral(s) => {
                                    pos_children.push(HtmlNode::Text(s.clone()));
                                }
                                Expr::Node(cn) => {
                                    pos_children.push(orv_node_to_html_ws(cn, eval, req, ctx));
                                }
                                other => {
                                    if let Ok(val) = eval.eval_expr(other) {
                                        pos_children.push(HtmlNode::Text(val.to_string()));
                                    }
                                }
                            }
                        }
                        // Collect properties as children (e.g. @meta attributes)
                        let mut all_children = pos_children;
                        all_children.extend(children);

                        if child.name == "head" {
                            // For @head, render define calls (like @DS) and
                            // plain nodes (like @title, @meta) as head children.
                            head_nodes.extend(all_children);
                        } else {
                            body_nodes.extend(all_children);
                        }
                    } else {
                        let child_node = orv_node_to_html_ws(child, eval, req, ctx);
                        body_nodes.push(child_node);
                    }
                }
            }
        }

        render_document(&head_nodes, &body_nodes)
    } else {
        let html_node = orv_node_to_html_ws(node, eval, req, ctx);
        crate::html::render(&html_node)
    }
}

fn orv_node_to_html_ws(
    node: &NodeExpr,
    eval: &mut Evaluator,
    req: &HttpRequest,
    ctx: &mut RenderCtx<'_>,
) -> HtmlNode {
    // Check if this is a define call (like @Home, @NotFound, @DS)
    if let Some(define) = ctx.defines.get(&node.name).cloned() {
        return expand_html_define(&define, node, eval, req, ctx);
    }

    let tag = node_to_tag(&node.name);
    let self_closing = is_self_closing(tag);

    let mut element = HtmlNode::element(tag);

    if let Some(layout_class) = layout_classes(&node.name) {
        element = element.with_attr("style", layout_class);
    }

    // Inside an island, capture event handler props as client-side handlers.
    // Outside an island, drop them (no JS to attach to).
    let in_island = ctx.islands.current_id().is_some();
    let mut handler_attrs: Vec<(String, String)> = Vec::new();
    if in_island {
        let state_names = ctx.islands.current_state_names();
        for prop in &node.properties {
            if !island_is_event_prop(&prop.name) {
                continue;
            }
            match lower_handler_to_js(&prop.value, &state_names) {
                Ok(js) => {
                    let event = event_name_from_prop(&prop.name);
                    if let Some(attr) = ctx.islands.record_handler(&event, js) {
                        handler_attrs.push(("data-orv-h".to_owned(), attr));
                    }
                }
                Err(e) => {
                    eprintln!(
                        "warning: failed to lower handler `%{}` in island: {e}",
                        prop.name
                    );
                }
            }
        }
    }

    // Bind non-event attributes from properties.
    for prop in &node.properties {
        if island_is_event_prop(&prop.name) {
            continue;
        }
        if let Ok(val) = eval.eval_expr(&prop.value) {
            element = element.with_attr(&prop.name, &val.to_string());
        }
    }

    // Multiple handlers on one element are joined into a comma-separated list.
    if !handler_attrs.is_empty() {
        let joined: Vec<String> = handler_attrs
            .iter()
            .map(|(_, v)| v.clone())
            .collect();
        element = element.with_attr("data-orv-h", &joined.join(","));
    }

    if self_closing {
        // For @meta, map positional args to name/content attributes.
        // @meta %charset="utf-8"       → <meta charset="utf-8"> (via properties)
        // @meta "name" "content"       → <meta name="name" content="content">
        if tag == "meta" && !node.positional.is_empty() {
            match node.positional.len() {
                1 => {
                    if let Some(val) = positional_string(&node.positional, 0) {
                        element = element.with_attr("charset", &val);
                    }
                }
                _ => {
                    // name before content for canonical ordering
                    if let Some(name_val) = positional_string(&node.positional, 0)
                        && let Some(content_val) = positional_string(&node.positional, 1)
                    {
                        element = element.with_attr("name", &name_val);
                        element = element.with_attr("content", &content_val);
                    }
                }
            }
        }
        return element.self_closing();
    }

    // Collect children from positional args
    for pos in &node.positional {
        match pos {
            Expr::StringLiteral(s) => {
                element = element.with_child(HtmlNode::Text(s.clone()));
            }
            Expr::Node(child_node) => {
                element = element.with_child(orv_node_to_html_ws(child_node, eval, req, ctx));
            }
            // Inside an island: signal-aware text expressions become reactive spans.
            Expr::Ident(resolved) if in_island && ctx.islands.is_current_sig(&resolved.name) => {
                let initial = eval
                    .eval_expr(pos)
                    .map(|v| v.to_string())
                    .unwrap_or_default();
                element = element.with_child(reactive_text_span(
                    ctx.islands.current_id().unwrap(),
                    &resolved.name,
                    &initial,
                ));
            }
            Expr::StringInterp(parts) if in_island => {
                element = element.with_children(string_interp_children(
                    parts,
                    eval,
                    ctx.islands.current_id().unwrap(),
                    &ctx.islands.current_state_names(),
                ));
            }
            other => {
                if let Ok(val) = eval.eval_expr(other) {
                    element = element.with_child(HtmlNode::Text(val.to_string()));
                }
            }
        }
    }

    // Collect children from body
    if let Some(body) = node.body.as_deref() {
        let children = collect_html_children_ws(body, eval, req, ctx);
        element = element.with_children(children);
    }

    element
}

fn expand_html_define(
    define: &DefineItem,
    call_node: &NodeExpr,
    eval: &mut Evaluator,
    req: &HttpRequest,
    ctx: &mut RenderCtx<'_>,
) -> HtmlNode {
    eval.env.push_scope();

    // If this define is classified as a client island, begin a new island
    // BEFORE evaluating any of the body so that prop/sig recording attaches
    // to the correct island id.
    let is_island = ctx.island_defines.contains(&define.name);
    if is_island {
        ctx.islands.begin(&define.name);
    }

    // Bind positional args to params (and record as island props if applicable).
    // We always register the param *name* with the registry — even if the
    // expression fails to evaluate at build time (e.g. because it touches
    // server-only state like a DB query that has no fixtures). This keeps the
    // SSR walk able to emit reactive text spans for `{paramName}` interps:
    // the runtime will read the actual value from the hydration payload, or
    // fall back to the prop default we record here.
    for (param, arg) in define.params.iter().zip(call_node.positional.iter()) {
        let val = eval.eval_expr(arg).unwrap_or_else(|e| {
            eprintln!(
                "warning: failed to evaluate positional arg `{}` for define `{}`: {e}",
                param.name, define.name
            );
            Value::Void
        });
        if is_island {
            ctx.islands.record_prop(&param.name, val.clone());
        }
        eval.env.set(param.name.clone(), val);
    }

    // Bind named properties to params (same registration policy as above).
    for prop in &call_node.properties {
        let val = eval.eval_expr(&prop.value).unwrap_or_else(|e| {
            eprintln!(
                "warning: failed to evaluate property `{}` for define `{}`: {e}",
                prop.name, define.name
            );
            Value::Void
        });
        if is_island {
            ctx.islands.record_prop(&prop.name, val.clone());
        }
        eval.env.set(prop.name.clone(), val);
    }

    // Bind default values for missing params
    for (i, param) in define.params.iter().enumerate() {
        if i >= call_node.positional.len()
            && !call_node.properties.iter().any(|p| p.name == param.name)
            && let Some(default) = &param.default
            && let Ok(val) = eval.eval_expr(default)
        {
            if is_island {
                ctx.islands.record_prop(&param.name, val.clone());
            }
            eval.env.set(param.name.clone(), val);
        }
    }

    let mut result = match define.return_domain.as_deref() {
        Some("design") => {
            // Design define — extract tokens by treating the body as @design content
            eval.extract_tokens_from_expr(&define.body);
            HtmlNode::Fragment(Vec::new())
        }
        _ => {
            // HTML or generic define — render body as HTML
            match &define.body {
                Expr::Block { stmts, .. } => {
                    let mut children = Vec::new();
                    for stmt in stmts {
                        match stmt {
                            // `let sig name = expr` — evaluate, store as island sig.
                            Stmt::Binding(binding) if binding.is_sig => {
                                let val = match &binding.value {
                                    Some(expr) => eval.eval_expr(expr).unwrap_or(Value::Void),
                                    None => Value::Void,
                                };
                                if is_island {
                                    ctx.islands.record_sig(&binding.name, val.clone());
                                }
                                eval.env.set(binding.name.clone(), val);
                            }
                            Stmt::Expr(Expr::Node(child)) => {
                                children.push(orv_node_to_html_ws(child, eval, req, ctx));
                            }
                            Stmt::Expr(Expr::StringLiteral(s)) => {
                                children.push(HtmlNode::Text(s.clone()));
                            }
                            Stmt::Expr(other) => {
                                if let Ok(val) = eval.eval_expr(other) {
                                    children.push(HtmlNode::Text(val.to_string()));
                                }
                            }
                            _ => {
                                let _ = eval.eval_stmt(stmt);
                            }
                        }
                    }
                    if children.len() == 1 {
                        children.into_iter().next().unwrap()
                    } else {
                        HtmlNode::Fragment(children)
                    }
                }
                Expr::Node(node) => orv_node_to_html_ws(node, eval, req, ctx),
                other => {
                    if let Ok(val) = eval.eval_expr(other) {
                        HtmlNode::Text(val.to_string())
                    } else {
                        HtmlNode::Fragment(Vec::new())
                    }
                }
            }
        }
    };

    // Tag the island root with `data-orv-i=iN` so the client runtime can find
    // it during hydration. The root is the first non-fragment element in the
    // returned HtmlNode tree.
    if is_island {
        let id = ctx
            .islands
            .current_id()
            .map(std::string::ToString::to_string);
        if let Some(id) = id {
            mark_island_root(&mut result, &id);
        }
        ctx.islands.end();
    }

    eval.env.pop_scope();
    result
}

/// Insert a `<span data-orv-text="iN:name">value</span>` placeholder for a
/// reactive text node bound to a signal.
fn reactive_text_span(island_id: &str, sig_name: &str, initial: &str) -> HtmlNode {
    HtmlNode::element("span")
        .with_attr("data-orv-text", &format!("{island_id}:{sig_name}"))
        .with_child(HtmlNode::Text(initial.to_owned()))
}

/// Convert a `StringInterp` AST inside an island into a sequence of HTML
/// children: literal `Text` nodes interleaved with reactive spans for any
/// signal references.
fn string_interp_children(
    parts: &[StringPart],
    eval: &mut Evaluator,
    island_id: &str,
    state_names: &HashSet<String>,
) -> Vec<HtmlNode> {
    let mut out = Vec::new();
    for part in parts {
        match part {
            StringPart::Lit(s) => {
                if !s.is_empty() {
                    out.push(HtmlNode::Text(s.clone()));
                }
            }
            StringPart::Expr(Expr::Ident(resolved)) if state_names.contains(&resolved.name) => {
                // Evaluate the signal/prop reference for the SSR placeholder.
                // `Value::Void` (e.g. a prop whose evaluation failed at SSR
                // time, like a DB-backed expression) renders as an empty
                // string instead of the literal text "void", so the page
                // doesn't expose internal placeholder values to users.
                let initial = match eval.eval_expr(&Expr::Ident(resolved.clone())) {
                    Ok(Value::Void) => String::new(),
                    Ok(v) => v.to_string(),
                    Err(_) => String::new(),
                };
                out.push(reactive_text_span(island_id, &resolved.name, &initial));
            }
            StringPart::Expr(other) => {
                if let Ok(val) = eval.eval_expr(other) {
                    out.push(HtmlNode::Text(val.to_string()));
                }
            }
        }
    }
    out
}

/// Walk an `HtmlNode` tree and add `data-orv-i=island_id` to the first
/// element encountered. If the root is a fragment, its first element child
/// (recursively) is marked.
fn mark_island_root(node: &mut HtmlNode, island_id: &str) -> bool {
    match node {
        HtmlNode::Element { attributes, .. } => {
            attributes.insert("data-orv-i".to_owned(), island_id.to_owned());
            true
        }
        HtmlNode::Fragment(children) => {
            for child in children {
                if mark_island_root(child, island_id) {
                    return true;
                }
            }
            false
        }
        _ => false,
    }
}

fn collect_html_children_ws(
    expr: &Expr,
    eval: &mut Evaluator,
    req: &HttpRequest,
    ctx: &mut RenderCtx<'_>,
) -> Vec<HtmlNode> {
    match expr {
        Expr::Block { stmts, .. } => {
            let mut children = Vec::new();
            for stmt in stmts {
                match stmt {
                    // Inside an island, `let sig` becomes a recorded signal.
                    Stmt::Binding(binding) if binding.is_sig => {
                        let val = match &binding.value {
                            Some(e) => eval.eval_expr(e).unwrap_or(Value::Void),
                            None => Value::Void,
                        };
                        if ctx.islands.current_id().is_some() {
                            ctx.islands.record_sig(&binding.name, val.clone());
                        }
                        eval.env.set(binding.name.clone(), val);
                    }
                    Stmt::Expr(Expr::Node(child)) => {
                        children.push(orv_node_to_html_ws(child, eval, req, ctx));
                    }
                    Stmt::Expr(Expr::StringLiteral(s)) => {
                        children.push(HtmlNode::Text(s.clone()));
                    }
                    Stmt::Expr(Expr::StringInterp(parts)) if ctx.islands.current_id().is_some() => {
                        let id = ctx.islands.current_id().unwrap().to_owned();
                        let names = ctx.islands.current_state_names();
                        let kids = string_interp_children(parts, eval, &id, &names);
                        children.extend(kids);
                    }
                    Stmt::Expr(Expr::When { subject, arms }) => {
                        // SSR-aware when: evaluate subject with request context,
                        // then render matched arm body as HTML nodes directly.
                        if let Ok(subject_val) = eval_expr_with_req(subject, eval, req)
                            && let Some(arm) = find_matching_arm(&subject_val, arms, eval)
                        {
                            // Render arm body directly as HTML nodes
                            // (find_matching_arm left a scope pushed for pattern bindings)
                            let arm_children = collect_html_children_ws_from_expr(&arm.body, eval, req, ctx);
                            children.extend(arm_children);
                            eval.env.pop_scope();
                        }
                    }
                    Stmt::Expr(other) => {
                        if let Ok(val) = eval.eval_expr(other) {
                            children.push(HtmlNode::Text(val.to_string()));
                        }
                    }
                    Stmt::Binding(_) => {
                        let _ = eval.eval_stmt(stmt);
                    }
                    _ => {
                        let _ = eval.eval_stmt(stmt);
                    }
                }
            }
            children
        }
        Expr::Node(child) => vec![orv_node_to_html_ws(child, eval, req, ctx)],
        Expr::StringLiteral(s) => vec![HtmlNode::Text(s.clone())],
        Expr::StringInterp(parts) if ctx.islands.current_id().is_some() => {
            let id = ctx.islands.current_id().unwrap().to_owned();
            let names = ctx.islands.current_state_names();
            string_interp_children(parts, eval, &id, &names)
        }
        other => {
            if let Ok(val) = eval.eval_expr(other) {
                vec![HtmlNode::Text(val.to_string())]
            } else {
                Vec::new()
            }
        }
    }
}

/// Evaluate an expression with request-accessor awareness.
///
/// When the expression is `@path` (or similar accessor nodes), resolve it
/// from the HTTP request instead of falling through to the generic
/// `Value::Node` branch in `eval_expr`.
fn eval_expr_with_req(
    expr: &Expr,
    eval: &mut Evaluator,
    req: &HttpRequest,
) -> Result<Value, EvalError> {
    match expr {
        // Direct accessor node: `@path`, `@method`, etc.
        Expr::Node(node) if is_accessor_node(&node.name) => {
            if let Some(val) = eval_accessor_in_context(node, eval, req) {
                Ok(val)
            } else {
                eval.eval_expr(expr)
            }
        }
        // Index into accessor: `@path[0]`
        Expr::Index { object, index } => {
            let obj_val = eval_expr_with_req(object, eval, req)?;
            let idx_val = eval.eval_expr(index)?;
            match (obj_val, idx_val) {
                (Value::Array(items), Value::Int(i)) => {
                    let idx = usize::try_from(i).map_err(|_| EvalError::IndexOutOfBounds)?;
                    // For SSR: empty path segments (root "/") → index 0 returns ""
                    // so `when @path[0] { "" -> ... }` matches the root route.
                    items
                        .get(idx)
                        .cloned()
                        .ok_or(EvalError::IndexOutOfBounds)
                        .or_else(|_| {
                            if items.is_empty() && idx == 0 {
                                Ok(Value::String(String::new()))
                            } else {
                                Err(EvalError::IndexOutOfBounds)
                            }
                        })
                }
                (Value::Map(map), Value::String(key)) => map
                    .get(&key)
                    .cloned()
                    .ok_or_else(|| EvalError::Custom(format!("key `{key}` not found"))),
                (Value::String(s), Value::Int(i)) => {
                    // String indexing — return single-char string
                    let idx = usize::try_from(i).map_err(|_| EvalError::IndexOutOfBounds)?;
                    s.chars()
                        .nth(idx)
                        .map(|c| Value::String(c.to_string()))
                        .ok_or(EvalError::IndexOutOfBounds)
                }
                (obj, idx) => Err(EvalError::TypeMismatch(format!(
                    "cannot index {obj} with {idx}"
                ))),
            }
        }
        _ => eval.eval_expr(expr),
    }
}

/// Find the first `WhenArm` whose pattern matches the given subject value.
fn find_matching_arm<'a>(
    subject: &Value,
    arms: &'a [WhenArm],
    eval: &mut Evaluator,
) -> Option<&'a WhenArm> {
    for arm in arms {
        eval.env.push_scope();
        let matched = match_pattern(&arm.pattern, subject, &mut eval.env);
        if matched {
            if let Some(guard) = &arm.guard {
                match eval.eval_expr(guard) {
                    Ok(Value::Bool(true)) => {
                        // Don't pop scope — caller needs bindings for body eval
                        return Some(arm);
                    }
                    _ => {
                        eval.env.pop_scope();
                        continue;
                    }
                }
            }
            return Some(arm);
        }
        eval.env.pop_scope();
    }
    None
}

/// Render an `Expr` into HTML nodes (SSR-aware: handles define calls and nodes).
fn collect_html_children_ws_from_expr(
    expr: &Expr,
    eval: &mut Evaluator,
    req: &HttpRequest,
    ctx: &mut RenderCtx<'_>,
) -> Vec<HtmlNode> {
    match expr {
        Expr::Node(child) => vec![orv_node_to_html_ws(child, eval, req, ctx)],
        Expr::Block { stmts, .. } => {
            let mut children = Vec::new();
            for stmt in stmts {
                match stmt {
                    Stmt::Expr(Expr::Node(child)) => {
                        children.push(orv_node_to_html_ws(child, eval, req, ctx));
                    }
                    Stmt::Expr(Expr::StringLiteral(s)) => {
                        children.push(HtmlNode::Text(s.clone()));
                    }
                    Stmt::Expr(other) => {
                        if let Ok(val) = eval.eval_expr(other) {
                            children.push(HtmlNode::Text(val.to_string()));
                        }
                    }
                    _ => {
                        let _ = eval.eval_stmt(stmt);
                    }
                }
            }
            children
        }
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

// ── Legacy HTML rendering ────────────────────────────────────────────────────

fn render_orv_html_node(node: &NodeExpr, eval: &mut Evaluator) -> String {
    if node.name == "html" {
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
        for prop in &node.properties {
            if is_event_handler_prop(&prop.name) {
                continue;
            }
            if let Ok(val) = eval.eval_expr(&prop.value) {
                element = element.with_attr(&prop.name, &val.to_string());
            }
        }
        return element.self_closing();
    }

    for prop in &node.properties {
        if is_event_handler_prop(&prop.name) {
            continue;
        }
        if let Ok(val) = eval.eval_expr(&prop.value) {
            element = element.with_attr(&prop.name, &val.to_string());
        }
    }

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

fn positional_atom(expr: &Expr) -> Option<String> {
    match expr {
        Expr::Ident(n) => Some(n.name.clone()),
        Expr::StringLiteral(s) => Some(s.clone()),
        _ => None,
    }
}


// ── Pre-render (compile-time HTML/CSS extraction) ───────────────────────────

/// Result of pre-rendering a workspace at compile time.
pub struct PrerenderResult {
    /// The rendered HTML for the @serve @html route (empty if none found).
    pub html: String,
    /// The CSS generated from @design tokens (empty if none found).
    pub css: String,
    /// Compiled JS source for client island handlers (empty if no islands).
    pub handlers_js: String,
    /// Set of island ids actually rendered (for client runtime emit).
    pub islands: Vec<IslandData>,
}

/// Pre-render a workspace's HTML pages and design tokens without starting a server.
///
/// This walks the entry module's @server block, evaluates defines and bindings,
/// finds `@serve @html` nodes and renders them, extracts @design CSS tokens.
/// Used by the emit phase to embed rendered HTML/CSS into the generated server binary.
pub fn prerender_workspace(
    modules: &[(String, Module)],
) -> Result<PrerenderResult, RunError> {
    if modules.is_empty() {
        return Ok(PrerenderResult {
            html: String::new(),
            css: String::new(),
            handlers_js: String::new(),
            islands: Vec::new(),
        });
    }

    // Phase 1: Register all defines and functions
    let mut eval = Evaluator::new();
    let mut all_defines: HashMap<String, DefineItem> = HashMap::new();
    let mut all_functions: HashMap<String, FunctionItem> = HashMap::new();
    for (_name, module) in modules {
        register_module_symbols(module, &mut eval, &mut all_defines, &mut all_functions);
    }
    register_import_aliases(modules, &mut eval, &mut all_defines);

    // Phase 1c: Identify which `@html` defines are client islands.
    let island_defines = island::collect_island_defines(modules);

    // Phase 2: Evaluate entry module top-level bindings
    let entry_module = &modules[0].1;
    let _ = eval_top_level_bindings(entry_module, &mut eval);

    // Phase 3: Find @server and extract HTML from @serve @html (collecting
    // any island instances rendered along the way).
    let mut islands_registry = IslandRegistry::new();
    let html = if let Some(server_node) = find_server_node(entry_module) {
        let mut ctx = RenderCtx::new(&all_defines, &island_defines, &mut islands_registry);
        prerender_serve_html(server_node, &mut eval, &mut ctx)
    } else {
        String::new()
    };

    let collected = islands_registry.into_islands();
    // Re-inject head/body assets now that islands are known. The earlier
    // `prerender_serve_html` returned the bare HTML; assets are added here
    // so the registry can finalize first.
    let mut html = html;
    inject_design_and_islands(&mut html, &eval, &collected);

    // Phase 4: Extract design CSS (also embedded directly above; emit kept
    // separate so callers that want raw CSS can still get it).
    let css = eval.design_tokens_to_css();

    let handlers_js = render_handlers_js(&collected);

    Ok(PrerenderResult {
        html,
        css,
        handlers_js,
        islands: collected,
    })
}

/// Walk the @server block looking for @serve @html nodes and render them.
fn prerender_serve_html(
    server_node: &NodeExpr,
    eval: &mut Evaluator,
    ctx: &mut RenderCtx<'_>,
) -> String {
    let stmts = match server_body_stmts(server_node) {
        Ok(s) => s,
        Err(_) => return String::new(),
    };
    prerender_serve_from_stmts(stmts, eval, ctx)
}

fn prerender_serve_from_stmts(
    stmts: &[Stmt],
    eval: &mut Evaluator,
    ctx: &mut RenderCtx<'_>,
) -> String {
    let dummy_req = HttpRequest {
        method: "GET".to_owned(),
        path: "/".to_owned(),
        headers: HashMap::new(),
        query_params: HashMap::new(),
        path_params: HashMap::new(),
        body: String::new(),
    };

    for stmt in stmts {
        match stmt {
            Stmt::Expr(Expr::Node(node)) if node.name == "route" => {
                // Check route body for @serve @html
                if let Some(Expr::Block { stmts: body_stmts, .. }) = node.body.as_deref() {
                    for body_stmt in body_stmts {
                        if let Stmt::Expr(Expr::Node(serve_node)) = body_stmt
                            && serve_node.name == "serve"
                            && let Some(Expr::Node(html_node)) = serve_node.positional.first()
                            && is_html_node(&html_node.name)
                        {
                            // Caller injects design + island assets after we
                            // return so the registry has captured everything.
                            return render_orv_html_node_workspace(
                                html_node, eval, &dummy_req, ctx,
                            );
                        }
                    }
                }
                // Also check nested route groups
                if let Some(Expr::Block { stmts: group_stmts, .. }) = node.body.as_deref() {
                    let result = prerender_serve_from_stmts(group_stmts, eval, ctx);
                    if !result.is_empty() {
                        return result;
                    }
                }
            }
            _ => {}
        }
    }
    String::new()
}

#[cfg(test)]
mod compact_tests {
    use super::{collect_used_css_vars, compact_html};

    #[test]
    fn collapses_inter_tag_whitespace() {
        let input = "<div>\n  <p>hi</p>\n</div>";
        let out = compact_html(input);
        assert_eq!(out, "<div><p>hi</p></div>");
    }

    #[test]
    fn preserves_text_adjacent_whitespace() {
        // The space before <span> is part of the rendered text node
        // ("Click: ") and must survive compaction.
        let input = "<button>Click: <span>0</span></button>";
        let out = compact_html(input);
        assert_eq!(out, "<button>Click: <span>0</span></button>");
    }

    #[test]
    fn preserves_text_adjacent_whitespace_after_indent() {
        let input = "<button>\n  Click: <span>0</span>\n</button>";
        let out = compact_html(input);
        // Whitespace adjacent to text is collapsed to a single space (it
        // becomes part of the rendered text node). Pure inter-tag whitespace
        // (after </span> and before </button>) is dropped entirely.
        assert_eq!(out, "<button> Click: <span>0</span></button>");
    }

    #[test]
    fn preserves_script_contents_verbatim() {
        let input = "<head>\n  <script>let x = 1;\nlet y = 2;</script>\n</head>";
        let out = compact_html(input);
        assert_eq!(out, "<head><script>let x = 1;\nlet y = 2;</script></head>");
    }

    #[test]
    fn collect_css_vars_finds_used() {
        let html = "<div style=\"color: var(--orv-color-fg); background: var(--orv-bg);\">";
        let used = collect_used_css_vars(html);
        assert!(used.contains("--orv-color-fg"));
        assert!(used.contains("--orv-bg"));
        assert_eq!(used.len(), 2);
    }

    #[test]
    fn collect_css_vars_empty_when_unused() {
        let html = "<div>plain</div>";
        let used = collect_used_css_vars(html);
        assert!(used.is_empty());
    }
}
