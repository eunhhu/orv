//! Client island data model and JS lowering for SSR.
//!
//! An *island* is a `@html` define that contains client-only constructs:
//! signals (`let sig`), event handler props (`%onClick`, ...), or signal
//! interpolations inside text. Islands are the only places where any client
//! JavaScript is emitted.
//!
//! For each island we collect:
//!   - **props**: values evaluated server-side and passed in via the call site
//!     (e.g. `@Home %count={userCount()}` where `userCount()` is server-pure
//!     and resolves to an `i32`).
//!   - **sigs**: `let sig` bindings declared inside the island, with their
//!     initial values (the *original* expression value, not any post-mutation
//!     state — handler bodies are never executed during SSR).
//!   - **handlers**: each `%onEvent={...}` body, lowered into a tiny JS
//!     function whose closure refers to the island's signal store.
//!
//! The hydration payload for an island is exactly `{ "props": ..., "sigs": ... }`
//! — nothing else is shipped to the client. Server-only data (DB rows, request
//! state, etc.) never reaches this struct.

use std::collections::HashSet;

use orv_hir::{
    AssignOp, BinaryOp, Binding, DefineItem, Expr, ItemKind, Module, NodeExpr, Stmt, StringPart,
    UnaryOp,
};

use crate::eval::Value;

/// Per-handler lowering: a single `%onEvent={body}` reduced to an
/// equivalent JS function source.
#[derive(Debug, Clone)]
pub struct HandlerSpec {
    /// The DOM event name (lowercased), e.g. `"click"`.
    pub event: String,
    /// Stable id within the island, e.g. `"h0"`.
    pub handler_id: String,
    /// Compiled JS body — uses `state.<sig>` for signal access/mutation.
    pub js_body: String,
}

/// All client-side data for one island instance on the page.
#[derive(Debug, Clone)]
pub struct IslandData {
    /// Stable id across the page, e.g. `"i0"`.
    pub id: String,
    /// The define name this island instantiates (for diagnostics only).
    pub define_name: String,
    /// Props evaluated server-side and shipped to the client as initial state.
    pub props: Vec<(String, Value)>,
    /// `let sig` bindings declared in the island body, with initial values.
    pub sigs: Vec<(String, Value)>,
    /// Compiled event handlers attached to elements inside this island.
    pub handlers: Vec<HandlerSpec>,
}

/// Mutable, ordered registry for islands collected during an SSR walk.
///
/// The runner pushes a new island when it enters an island-classified define
/// and finalizes it when leaving. Within an island, signal bindings, props,
/// and handlers are appended in encounter order.
#[derive(Debug, Default)]
pub struct IslandRegistry {
    islands: Vec<IslandData>,
    /// Stack of indices into `islands` representing currently-active islands.
    stack: Vec<usize>,
    next_handler_id: Vec<usize>,
}

impl IslandRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Begin a new island with the given define name. Returns the island id
    /// (e.g. `"i0"`).
    pub fn begin(&mut self, define_name: &str) -> String {
        let idx = self.islands.len();
        let id = format!("i{idx}");
        self.islands.push(IslandData {
            id: id.clone(),
            define_name: define_name.to_owned(),
            props: Vec::new(),
            sigs: Vec::new(),
            handlers: Vec::new(),
        });
        self.stack.push(idx);
        self.next_handler_id.push(0);
        id
    }

    /// End the most recently begun island.
    pub fn end(&mut self) {
        self.stack.pop();
        self.next_handler_id.pop();
    }

    /// Returns the id of the currently-active island, if any.
    #[must_use]
    pub fn current_id(&self) -> Option<&str> {
        self.stack.last().map(|&i| self.islands[i].id.as_str())
    }

    /// Record a prop value (already evaluated server-side).
    pub fn record_prop(&mut self, name: &str, value: Value) {
        if let Some(&idx) = self.stack.last() {
            self.islands[idx].props.push((name.to_owned(), value));
        }
    }

    /// Record a `let sig` binding.
    pub fn record_sig(&mut self, name: &str, value: Value) {
        if let Some(&idx) = self.stack.last() {
            self.islands[idx].sigs.push((name.to_owned(), value));
        }
    }

    /// Returns true if `name` is a sig in the current island.
    #[must_use]
    pub fn is_current_sig(&self, name: &str) -> bool {
        self.stack
            .last()
            .is_some_and(|&i| self.islands[i].sigs.iter().any(|(n, _)| n == name))
            || self
                .stack
                .last()
                .is_some_and(|&i| self.islands[i].props.iter().any(|(n, _)| n == name))
    }

    /// Returns the set of sig + prop names in the current island.
    #[must_use]
    pub fn current_state_names(&self) -> HashSet<String> {
        let Some(&idx) = self.stack.last() else {
            return HashSet::new();
        };
        let isl = &self.islands[idx];
        let mut out = HashSet::new();
        for (n, _) in &isl.sigs {
            out.insert(n.clone());
        }
        for (n, _) in &isl.props {
            out.insert(n.clone());
        }
        out
    }

    /// Allocate a new handler id within the current island and append the
    /// `HandlerSpec`. Returns the full attribute value `iN:event:hM`.
    pub fn record_handler(&mut self, event: &str, js_body: String) -> Option<String> {
        let &idx = self.stack.last()?;
        let h_idx = self.next_handler_id.last_mut()?;
        let handler_id = format!("h{h_idx}");
        *h_idx += 1;
        let attr = format!("{}:{}:{}", self.islands[idx].id, event, handler_id);
        self.islands[idx].handlers.push(HandlerSpec {
            event: event.to_owned(),
            handler_id,
            js_body,
        });
        Some(attr)
    }

    /// Consume the registry and return the collected islands.
    #[must_use]
    pub fn into_islands(self) -> Vec<IslandData> {
        self.islands
    }

    #[must_use]
    pub fn islands(&self) -> &[IslandData] {
        &self.islands
    }
}

/// Set of define names that are islands (must be hydrated on the client).
pub fn collect_island_defines(modules: &[(String, Module)]) -> HashSet<String> {
    let mut islands = HashSet::new();
    for (_, module) in modules {
        for item in &module.items {
            if let ItemKind::Define(define) = &item.kind
                && define.return_domain.as_deref() == Some("html")
                && define_is_island(define)
            {
                islands.insert(define.name.clone());
            }
        }
    }
    islands
}

/// Check whether a define's body contains any client-only construct.
fn define_is_island(define: &DefineItem) -> bool {
    expr_has_client_construct(&define.body)
}

fn expr_has_client_construct(expr: &Expr) -> bool {
    match expr {
        Expr::Block { stmts, .. } => stmts.iter().any(stmt_has_client_construct),
        Expr::Node(node) => node_has_client_construct(node),
        Expr::Paren(inner) | Expr::Await(inner) => expr_has_client_construct(inner),
        Expr::When { subject, arms } => {
            expr_has_client_construct(subject)
                || arms.iter().any(|arm| {
                    arm.guard.as_ref().is_some_and(expr_has_client_construct)
                        || expr_has_client_construct(&arm.body)
                })
        }
        Expr::Binary { left, right, .. } => {
            expr_has_client_construct(left) || expr_has_client_construct(right)
        }
        Expr::Unary { operand, .. } => expr_has_client_construct(operand),
        Expr::Assign { target, value, .. } => {
            expr_has_client_construct(target) || expr_has_client_construct(value)
        }
        Expr::Call { callee, args } => {
            expr_has_client_construct(callee)
                || args.iter().any(|a| expr_has_client_construct(&a.value))
        }
        Expr::Field { object, .. } => expr_has_client_construct(object),
        Expr::Index { object, index } => {
            expr_has_client_construct(object) || expr_has_client_construct(index)
        }
        Expr::Object(fields) | Expr::Map(fields) => fields
            .iter()
            .any(|f| expr_has_client_construct(&f.value)),
        Expr::Array(items) => items.iter().any(expr_has_client_construct),
        Expr::TryCatch {
            body, catch_body, ..
        } => expr_has_client_construct(body) || expr_has_client_construct(catch_body),
        Expr::Closure { body, .. } => expr_has_client_construct(body),
        Expr::StringInterp(_)
        | Expr::IntLiteral(_)
        | Expr::FloatLiteral(_)
        | Expr::StringLiteral(_)
        | Expr::BoolLiteral(_)
        | Expr::Void
        | Expr::Ident(_)
        | Expr::Error => false,
    }
}

fn stmt_has_client_construct(stmt: &Stmt) -> bool {
    match stmt {
        // `let sig` is the canonical client-side binding.
        Stmt::Binding(Binding { is_sig: true, .. }) => true,
        Stmt::Binding(Binding { value, .. }) => {
            value.as_ref().is_some_and(expr_has_client_construct)
        }
        Stmt::Expr(e) | Stmt::Return(Some(e)) => expr_has_client_construct(e),
        Stmt::Return(None) | Stmt::Error => false,
        Stmt::If(s) => {
            expr_has_client_construct(&s.condition)
                || expr_has_client_construct(&s.then_body)
                || s.else_body.as_ref().is_some_and(expr_has_client_construct)
        }
        Stmt::For(s) => {
            expr_has_client_construct(&s.iterable) || expr_has_client_construct(&s.body)
        }
        Stmt::While(s) => {
            expr_has_client_construct(&s.condition) || expr_has_client_construct(&s.body)
        }
    }
}

fn node_has_client_construct(node: &NodeExpr) -> bool {
    // Event handler props are client-only.
    if node
        .properties
        .iter()
        .any(|p| is_event_handler_prop(&p.name))
    {
        return true;
    }
    if node
        .positional
        .iter()
        .any(expr_has_client_construct)
    {
        return true;
    }
    if node
        .properties
        .iter()
        .any(|p| expr_has_client_construct(&p.value))
    {
        return true;
    }
    if let Some(body) = &node.body
        && expr_has_client_construct(body)
    {
        return true;
    }
    false
}

/// Returns true for property names like `onClick`, `onChange`, `onSubmit`.
pub fn is_event_handler_prop(name: &str) -> bool {
    name.starts_with("on") && name.len() > 2 && name.as_bytes()[2].is_ascii_uppercase()
}

/// Convert a property name like `onClick` into the lowercase event name `click`.
pub fn event_name_from_prop(prop: &str) -> String {
    debug_assert!(is_event_handler_prop(prop));
    prop[2..].to_ascii_lowercase()
}

// ── Handler lowering: Expr → JS source ───────────────────────────────────────

/// Errors that can arise while lowering a handler expression to JS.
#[derive(Debug, Clone)]
pub enum LowerError {
    UnsupportedExpr(&'static str),
}

impl std::fmt::Display for LowerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedExpr(kind) => {
                write!(f, "expression kind `{kind}` is not allowed inside an event handler")
            }
        }
    }
}

impl std::error::Error for LowerError {}

/// Lower an event handler expression into a JS function body whose only
/// free names are `state.<sig>` (signal store) and the JS standard library.
///
/// The supported subset is intentionally narrow:
///   - Literals (`int`, `float`, `string`, `bool`)
///   - Identifiers (treated as `state.<name>`)
///   - Binary ops (arithmetic, comparison, logical)
///   - Unary ops (`-`, `!`)
///   - Assign / += / -= against an identifier target (signal mutation)
///   - Parens
///   - A block of expression statements (treated as sequential)
///
/// Anything else (function calls, await, when, define calls, domain nodes)
/// returns `LowerError::UnsupportedExpr`.
pub fn lower_handler_to_js(expr: &Expr, sig_names: &HashSet<String>) -> Result<String, LowerError> {
    let mut out = String::new();
    lower_stmt_expr(expr, sig_names, &mut out)?;
    Ok(out)
}

fn lower_stmt_expr(
    expr: &Expr,
    sig_names: &HashSet<String>,
    out: &mut String,
) -> Result<(), LowerError> {
    match expr {
        Expr::Block { stmts, .. } => {
            for stmt in stmts {
                match stmt {
                    Stmt::Expr(e) => {
                        lower_stmt_expr(e, sig_names, out)?;
                        out.push_str(";\n");
                    }
                    _ => return Err(LowerError::UnsupportedExpr("block-stmt-non-expr")),
                }
            }
            Ok(())
        }
        other => lower_value_expr(other, sig_names, out),
    }
}

fn lower_value_expr(
    expr: &Expr,
    sig_names: &HashSet<String>,
    out: &mut String,
) -> Result<(), LowerError> {
    match expr {
        Expr::IntLiteral(v) => {
            out.push_str(&v.to_string());
            Ok(())
        }
        Expr::FloatLiteral(v) => {
            out.push_str(&v.to_string());
            Ok(())
        }
        Expr::BoolLiteral(v) => {
            out.push_str(if *v { "true" } else { "false" });
            Ok(())
        }
        Expr::StringLiteral(s) => {
            out.push_str(&js_string_literal(s));
            Ok(())
        }
        Expr::Void => {
            out.push_str("null");
            Ok(())
        }
        Expr::Ident(resolved) => {
            // Identifiers in handlers must refer to signals (the only client
            // state available). Non-signal idents would be undefined at
            // runtime, so we still emit them but the analyzer should warn.
            if sig_names.contains(&resolved.name) {
                out.push_str("state.");
                out.push_str(&resolved.name);
            } else {
                // Allow as-is — pure literal lookup; emit as state.<name>
                // anyway so missing identifiers fail loudly at runtime.
                out.push_str("state.");
                out.push_str(&resolved.name);
            }
            Ok(())
        }
        Expr::Paren(inner) => {
            out.push('(');
            lower_value_expr(inner, sig_names, out)?;
            out.push(')');
            Ok(())
        }
        Expr::Unary { op, operand } => {
            let op_str = match op {
                UnaryOp::Neg => "-",
                UnaryOp::Not => "!",
            };
            out.push_str(op_str);
            out.push('(');
            lower_value_expr(operand, sig_names, out)?;
            out.push(')');
            Ok(())
        }
        Expr::Binary { left, op, right } => {
            let op_str = match op {
                BinaryOp::Add => "+",
                BinaryOp::Sub => "-",
                BinaryOp::Mul => "*",
                BinaryOp::Div => "/",
                BinaryOp::Eq => "===",
                BinaryOp::NotEq => "!==",
                BinaryOp::Lt => "<",
                BinaryOp::LtEq => "<=",
                BinaryOp::Gt => ">",
                BinaryOp::GtEq => ">=",
                _ => return Err(LowerError::UnsupportedExpr("binary-op")),
            };
            out.push('(');
            lower_value_expr(left, sig_names, out)?;
            out.push(' ');
            out.push_str(op_str);
            out.push(' ');
            lower_value_expr(right, sig_names, out)?;
            out.push(')');
            Ok(())
        }
        Expr::Assign { target, op, value } => {
            // Target must be an identifier (signal name).
            let Expr::Ident(name) = target.as_ref() else {
                return Err(LowerError::UnsupportedExpr("assign-non-ident-target"));
            };
            if !sig_names.contains(&name.name) {
                // Not a signal — refuse: handlers should only mutate signals.
                return Err(LowerError::UnsupportedExpr("assign-non-signal"));
            }
            let op_str = match op {
                AssignOp::Assign => "=",
                AssignOp::AddAssign => "+=",
                AssignOp::SubAssign => "-=",
            };
            out.push_str("state.");
            out.push_str(&name.name);
            out.push(' ');
            out.push_str(op_str);
            out.push(' ');
            lower_value_expr(value, sig_names, out)?;
            Ok(())
        }
        Expr::StringInterp(parts) => {
            // Build a JS template literal: `Click: ${state.count}`
            out.push('`');
            for part in parts {
                match part {
                    StringPart::Lit(s) => {
                        // Escape backticks and ${
                        for ch in s.chars() {
                            if ch == '`' {
                                out.push_str("\\`");
                            } else if ch == '\\' {
                                out.push_str("\\\\");
                            } else {
                                out.push(ch);
                            }
                        }
                    }
                    StringPart::Expr(e) => {
                        out.push_str("${");
                        lower_value_expr(e, sig_names, out)?;
                        out.push('}');
                    }
                }
            }
            out.push('`');
            Ok(())
        }
        Expr::Call { .. } => Err(LowerError::UnsupportedExpr("call")),
        Expr::Await(_) => Err(LowerError::UnsupportedExpr("await")),
        Expr::When { .. } => Err(LowerError::UnsupportedExpr("when")),
        Expr::Object(_) | Expr::Map(_) => Err(LowerError::UnsupportedExpr("object")),
        Expr::Array(_) => Err(LowerError::UnsupportedExpr("array")),
        Expr::Node(_) => Err(LowerError::UnsupportedExpr("domain-node")),
        Expr::Field { .. } => Err(LowerError::UnsupportedExpr("field")),
        Expr::Index { .. } => Err(LowerError::UnsupportedExpr("index")),
        Expr::Block { .. } => Err(LowerError::UnsupportedExpr("nested-block")),
        Expr::TryCatch { .. } => Err(LowerError::UnsupportedExpr("try-catch")),
        Expr::Closure { .. } => Err(LowerError::UnsupportedExpr("closure")),
        Expr::Error => Err(LowerError::UnsupportedExpr("error-expr")),
    }
}

fn js_string_literal(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                out.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

// ── JS module emit ───────────────────────────────────────────────────────────

// ── Hydration payload (props + sigs only) ───────────────────────────────────

/// Serialize a `Value` into a JSON literal suitable for the hydration payload.
///
/// Only safe primitive types and collections of primitives are emitted.
/// Function/Node/RouteRef/etc. become `null` to avoid leaking server state.
fn value_to_json(v: &Value) -> String {
    match v {
        Value::Void => "null".to_owned(),
        Value::Bool(b) => b.to_string(),
        Value::Int(n) => n.to_string(),
        Value::Float(n) => {
            if n.is_finite() {
                n.to_string()
            } else {
                "null".to_owned()
            }
        }
        Value::String(s) => json_escape_string(s),
        Value::Array(items) => {
            let parts: Vec<String> = items.iter().map(value_to_json).collect();
            format!("[{}]", parts.join(","))
        }
        Value::Object(map) | Value::Map(map) => {
            let parts: Vec<String> = map
                .iter()
                .map(|(k, v)| format!("{}:{}", json_escape_string(k), value_to_json(v)))
                .collect();
            format!("{{{}}}", parts.join(","))
        }
        Value::Function { .. }
        | Value::BuiltinFn(_)
        | Value::Node { .. }
        | Value::RouteRef { .. } => "null".to_owned(),
    }
}

fn json_escape_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '/' => out.push_str("\\/"),
            c if (c as u32) < 0x20 => {
                out.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Render the full hydration payload (one JSON object per island) into a
/// single inline `<script type="application/json" id="orv-islands">` body.
///
/// Output shape:
/// ```json
/// { "i0": { "props": { "count": 3 }, "sigs": { "count": 0 } } }
/// ```
#[must_use]
pub fn render_island_payload_json(islands: &[IslandData]) -> String {
    if islands.is_empty() {
        return String::new();
    }
    let mut out = String::new();
    out.push('{');
    for (i, island) in islands.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push_str(&json_escape_string(&island.id));
        out.push(':');
        out.push('{');
        out.push_str("\"props\":{");
        for (j, (name, val)) in island.props.iter().enumerate() {
            if j > 0 {
                out.push(',');
            }
            out.push_str(&json_escape_string(name));
            out.push(':');
            out.push_str(&value_to_json(val));
        }
        out.push_str("},\"sigs\":{");
        for (j, (name, val)) in island.sigs.iter().enumerate() {
            if j > 0 {
                out.push(',');
            }
            out.push_str(&json_escape_string(name));
            out.push(':');
            out.push_str(&value_to_json(val));
        }
        out.push('}');
        out.push('}');
    }
    out.push('}');
    out
}

/// Render the full handlers.js source for a list of islands.
///
/// Each island becomes one exported object whose keys are handler ids and
/// whose values are functions taking the island's `state` proxy:
///
/// ```js
/// export const i0 = {
///   h0: function (state) { state.count += 1 },
/// };
/// ```
#[must_use]
pub fn render_handlers_js(islands: &[IslandData]) -> String {
    if islands.is_empty() {
        return String::new();
    }
    let mut out = String::new();
    out.push_str("// orv handlers — auto-generated, do not edit\n");
    out.push_str("\"use strict\";\n\n");
    for island in islands {
        if island.handlers.is_empty() {
            continue;
        }
        out.push_str(&format!("export const {} = {{\n", island.id));
        for h in &island.handlers {
            out.push_str(&format!(
                "  {}: function (state) {{ {} }},\n",
                h.handler_id, h.js_body
            ));
        }
        out.push_str("};\n\n");
    }
    out
}

/// Compile-time feature flags describing what the client runtime *needs* to
/// support for a given page. Anything `false` is dropped from the inline JS
/// to keep the wire payload as small as possible — critical on slow links.
#[derive(Debug, Default, Clone, Copy)]
pub struct RuntimeFeatures {
    /// At least one island has reactive text spans (sigs or signal interp).
    pub has_text: bool,
    /// At least one island has event handler bindings.
    pub has_handlers: bool,
    /// Some `data-orv-h` attribute holds more than one comma-separated entry,
    /// requiring split logic. When false, every entry is exactly `iN:event:hM`.
    pub has_multi_handler: bool,
}

impl RuntimeFeatures {
    /// Walk the islands list and decide which runtime features are needed.
    ///
    /// `multi_handler_seen` should be set by the SSR walker when it joins
    /// multiple `data-orv-h` entries on the same element. Conservatively
    /// pass `true` if you don't know.
    #[must_use]
    pub fn analyze(islands: &[IslandData], multi_handler_seen: bool) -> Self {
        let mut features = Self::default();
        for island in islands {
            if !island.sigs.is_empty() || !island.props.is_empty() {
                // Sigs and props power reactive text. Without either there's
                // nothing to bind to.
                if !island.sigs.is_empty() {
                    features.has_text = true;
                }
            }
            if !island.handlers.is_empty() {
                features.has_handlers = true;
            }
        }
        features.has_multi_handler = multi_handler_seen;
        features
    }
}

/// Build a minimum-viable client runtime that supports exactly the features
/// in `features`. Output is heavily minified — no comments, no extra
/// whitespace, single-letter locals where it does not hurt readability of
/// generated diagnostics.
#[must_use]
pub fn minified_runtime_source(features: RuntimeFeatures) -> String {
    if !features.has_text && !features.has_handlers {
        return String::new();
    }
    let mut out = String::new();
    out.push_str("(function(){");
    out.push_str("var P=document.getElementById('orv-islands');");
    out.push_str("if(!P)return;");
    out.push_str("var D;try{D=JSON.parse(P.textContent||'{}')}catch(e){return}");
    // Per-island bind closure.
    out.push_str("function B(r){");
    out.push_str("var i=r.getAttribute('data-orv-i');");
    out.push_str("var d=D[i];if(!d)return;");
    out.push_str("var s=Object.assign({},d.props||{},d.sigs||{});");
    if features.has_text {
        out.push_str("var u={};");
    }
    out.push_str("var t=new Proxy(s,{set:function(o,k,v){");
    if features.has_text {
        out.push_str("if(o[k]===v)return true;o[k]=v;var l=u[k];if(l)for(var j=0;j<l.length;j++)l[j](v);return true");
    } else {
        out.push_str("o[k]=v;return true");
    }
    out.push_str("}});");
    if features.has_text {
        // Bind reactive text spans.
        out.push_str("r.querySelectorAll('[data-orv-text]').forEach(function(e){");
        out.push_str("var p=e.getAttribute('data-orv-text').split(':');");
        out.push_str("if(p[0]!==i)return;");
        out.push_str("var n=p[1];");
        out.push_str("if(n in s)e.textContent=String(s[n]);");
        out.push_str("(u[n]=u[n]||[]).push(function(v){e.textContent=String(v)})");
        out.push_str("});");
    }
    if features.has_handlers {
        // Bind event handlers.
        out.push_str("var H=(window.__orvHandlers||{})[i]||{};");
        out.push_str("r.querySelectorAll('[data-orv-h]').forEach(function(e){");
        out.push_str("var v=e.getAttribute('data-orv-h');");
        if features.has_multi_handler {
            out.push_str("v.split(',').forEach(function(x){");
            out.push_str("var q=x.split(':');if(q.length!==3||q[0]!==i)return;");
            out.push_str("var f=H[q[2]];if(typeof f==='function')e.addEventListener(q[1],function(){f(t)})");
            out.push_str("})");
        } else {
            out.push_str("var q=v.split(':');if(q.length!==3||q[0]!==i)return;");
            out.push_str("var f=H[q[2]];if(typeof f==='function')e.addEventListener(q[1],function(){f(t)})");
        }
        out.push_str("})");
    }
    out.push_str("}");
    out.push_str("function I(){document.querySelectorAll('[data-orv-i]').forEach(B)}");
    out.push_str("if(document.readyState==='loading')document.addEventListener('DOMContentLoaded',I);else I()");
    out.push_str("})();");
    out
}

/// Render handlers as a tiny inline assignment block (no module exports, no
/// `"use strict"` banner, no comments). Designed for direct inline injection
/// where it never goes through any other tooling.
///
/// Output shape:
/// ```js
/// window.__orvHandlers={i0:{h0:function(state){state.count+=1}}};
/// ```
#[must_use]
pub fn render_handlers_inline(islands: &[IslandData]) -> String {
    let with_handlers: Vec<&IslandData> = islands.iter().filter(|i| !i.handlers.is_empty()).collect();
    if with_handlers.is_empty() {
        return String::new();
    }
    let mut out = String::from("window.__orvHandlers={");
    for (i, island) in with_handlers.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push_str(&island.id);
        out.push_str(":{");
        for (j, h) in island.handlers.iter().enumerate() {
            if j > 0 {
                out.push(',');
            }
            out.push_str(&h.handler_id);
            out.push_str(":function(state){");
            // h.js_body may have trailing semicolons/newlines from lowering.
            // Compact whitespace into single spaces and trim.
            let body = h.js_body.trim();
            // Strip trailing semicolons (we'll add one if needed below).
            let body = body.trim_end_matches(';');
            out.push_str(body);
            out.push('}');
        }
        out.push('}');
    }
    out.push_str("};");
    out
}

/// The client-side hydration runtime, as a JS module body.
///
/// This source is shared between the inline injection used by SSR (so the
/// same response that ships handlers can also boot the runtime in document
/// order) and the standalone `public/orv-runtime.js` artifact emitted by
/// the build pipeline.
#[must_use]
pub fn client_runtime_source() -> String {
    r#"// orv client runtime - auto-generated, do not edit
"use strict";

(function () {
  function readPayload() {
    const el = document.getElementById('orv-islands');
    if (!el) return {};
    try { return JSON.parse(el.textContent || '{}'); }
    catch (_) { return {}; }
  }

  function makeIsland(islandId, data) {
    const initial = Object.assign({}, data.props || {}, data.sigs || {});
    const subs = {};
    const state = new Proxy(initial, {
      set(target, key, value) {
        const prev = target[key];
        if (prev === value) return true;
        target[key] = value;
        const list = subs[key] || [];
        for (const fn of list) fn(value, prev);
        return true;
      },
    });
    function subscribe(name, fn) {
      (subs[name] = subs[name] || []).push(fn);
    }
    return { state, subscribe };
  }

  function bindIsland(rootEl) {
    const islandId = rootEl.getAttribute('data-orv-i');
    if (!islandId) return;
    const payload = window.__orvPayload || (window.__orvPayload = readPayload());
    const data = payload[islandId];
    if (!data) return;
    const { state, subscribe } = makeIsland(islandId, data);

    // Reactive text spans inside this island.
    const texts = rootEl.querySelectorAll('[data-orv-text]');
    texts.forEach(function (el) {
      const ref = el.getAttribute('data-orv-text');
      const colon = ref.indexOf(':');
      if (colon < 0) return;
      const owner = ref.slice(0, colon);
      const name = ref.slice(colon + 1);
      if (owner !== islandId) return;
      if (name in state) el.textContent = String(state[name]);
      subscribe(name, function (v) { el.textContent = String(v); });
    });
    if (rootEl.hasAttribute('data-orv-text')) {
      const ref = rootEl.getAttribute('data-orv-text');
      const colon = ref.indexOf(':');
      if (colon >= 0) {
        const owner = ref.slice(0, colon);
        const name = ref.slice(colon + 1);
        if (owner === islandId) {
          if (name in state) rootEl.textContent = String(state[name]);
          subscribe(name, function (v) { rootEl.textContent = String(v); });
        }
      }
    }

    // Event handlers inside this island.
    const handlers = rootEl.querySelectorAll('[data-orv-h]');
    function bindOne(el) {
      const spec = el.getAttribute('data-orv-h');
      if (!spec) return;
      const islandHandlers = (window.__orvHandlers || {})[islandId] || {};
      spec.split(',').forEach(function (entry) {
        const parts = entry.split(':');
        if (parts.length !== 3) return;
        const owner = parts[0];
        const event = parts[1];
        const hid = parts[2];
        if (owner !== islandId) return;
        const fn = islandHandlers[hid];
        if (typeof fn !== 'function') return;
        el.addEventListener(event, function () { fn(state); });
      });
    }
    handlers.forEach(bindOne);
    if (rootEl.hasAttribute('data-orv-h')) bindOne(rootEl);
  }

  function init() {
    document.querySelectorAll('[data-orv-i]').forEach(bindIsland);
  }

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', init);
  } else {
    init();
  }
})();
"#
    .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_handler_prop_detection() {
        assert!(is_event_handler_prop("onClick"));
        assert!(is_event_handler_prop("onChange"));
        assert!(!is_event_handler_prop("on"));
        assert!(!is_event_handler_prop("once"));
        assert!(!is_event_handler_prop("class"));
    }

    #[test]
    fn event_name_lowercase() {
        assert_eq!(event_name_from_prop("onClick"), "click");
        assert_eq!(event_name_from_prop("onMouseDown"), "mousedown");
    }

    #[test]
    fn js_string_escaping() {
        assert_eq!(js_string_literal("hello"), "\"hello\"");
        assert_eq!(js_string_literal("a\"b"), "\"a\\\"b\"");
        assert_eq!(js_string_literal("a\nb"), "\"a\\nb\"");
    }

    #[test]
    fn empty_islands_yields_empty_handlers_js() {
        assert!(render_handlers_js(&[]).is_empty());
    }
}
