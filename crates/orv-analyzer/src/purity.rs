//! Server-purity analysis across a workspace of HIR modules.
//!
//! Each function and define is classified as either *server-dependent* (needs
//! HTTP request/db/etc. context to evaluate) or *client-pure* (can be safely
//! evaluated in a browser).  The classification is **transitive**: a function
//! that calls a server-dependent function is itself server-dependent.
//!
//! This information drives the SSR/island split:
//!   - Expressions inside an `@html` block whose evaluation is server-pure can
//!     run on the client; the rest must be evaluated by the SSR pass and the
//!     resulting *value* shipped to the client as island props.
//!   - Defines that contain client-only constructs (`let sig`, event handlers,
//!     signal interpolation) become client islands.
//!
//! The analysis is intentionally simple: it walks the HIR by name, treating
//! cross-module calls as references to whatever symbol with the same name is
//! visible in the workspace.  The orv resolver guarantees that, after import
//! aliasing, every callable name is unambiguous within a project, so this
//! lookup is sound for the project-e2e style of code we currently emit.

use std::collections::{HashMap, HashSet};

use orv_hir::{
    DefineItem, Expr, FunctionItem, IfStmt, ItemKind, Module, NodeExpr, Stmt, StringPart, WhenArm,
};

/// Per-symbol server-purity classification.
#[derive(Debug, Default, Clone)]
pub struct PurityMap {
    /// Function/define name → true if its evaluation requires server context.
    server_dependent: HashSet<String>,
}

impl PurityMap {
    /// Returns true if the named function/define is server-dependent.
    #[must_use]
    pub fn is_server(&self, name: &str) -> bool {
        self.server_dependent.contains(name)
    }

    /// Insert a name as server-dependent.
    pub fn mark_server(&mut self, name: impl Into<String>) {
        self.server_dependent.insert(name.into());
    }
}

/// Server-only domain node names.  Any expression that *directly* uses one of
/// these accessor/builder nodes is server-dependent.
fn is_server_domain_node(name: &str) -> bool {
    let head = name.split('.').next().unwrap_or(name);
    matches!(
        head,
        "db" | "response"
            | "request"
            | "context"
            | "env"
            | "cookie"
            | "header"
            | "param"
            | "query"
            | "body"
            | "method"
            | "path"
    )
}

/// Run the server-purity analysis over an entire workspace and return the
/// resulting `PurityMap`.
#[must_use]
pub fn analyze_workspace_purity(modules: &[(String, Module)]) -> PurityMap {
    // 1. Collect every named callable (function or define) into a map.
    let mut callables: HashMap<String, &Expr> = HashMap::new();
    for (_, module) in modules {
        for item in &module.items {
            match &item.kind {
                ItemKind::Function(FunctionItem { name, body, .. }) => {
                    callables.entry(name.clone()).or_insert(body);
                }
                ItemKind::Define(DefineItem { name, body, .. }) => {
                    callables.entry(name.clone()).or_insert(body);
                }
                _ => {}
            }
        }
    }

    // 2. Iterate to a fixed point: a callable becomes server-dependent if any
    //    expression it owns either uses a server domain node directly or calls
    //    another callable that is already server-dependent.
    let mut purity = PurityMap::default();
    loop {
        let mut changed = false;
        for (name, body) in &callables {
            if purity.is_server(name) {
                continue;
            }
            if expr_uses_server(body, &purity) {
                purity.mark_server(name);
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    purity
}

/// Returns true if `expr` is server-dependent under the current purity map.
///
/// This is the public entry point used by the SSR/island pass to decide which
/// expressions inside an `@html` block must be evaluated server-side.
#[must_use]
pub fn is_server_expr(expr: &Expr, purity: &PurityMap) -> bool {
    expr_uses_server(expr, purity)
}

fn expr_uses_server(expr: &Expr, purity: &PurityMap) -> bool {
    match expr {
        Expr::IntLiteral(_)
        | Expr::FloatLiteral(_)
        | Expr::StringLiteral(_)
        | Expr::BoolLiteral(_)
        | Expr::Void
        | Expr::Error => false,

        Expr::Ident(resolved) => purity.is_server(&resolved.name),

        Expr::StringInterp(parts) => parts.iter().any(|part| match part {
            StringPart::Lit(_) => false,
            StringPart::Expr(e) => expr_uses_server(e, purity),
        }),

        Expr::Binary { left, right, .. } => {
            expr_uses_server(left, purity) || expr_uses_server(right, purity)
        }
        Expr::Unary { operand, .. } => expr_uses_server(operand, purity),
        Expr::Assign { target, value, .. } => {
            expr_uses_server(target, purity) || expr_uses_server(value, purity)
        }

        Expr::Call { callee, args } => {
            if expr_uses_server(callee, purity) {
                return true;
            }
            // If the callee is a plain identifier, look it up in the purity map.
            if let Expr::Ident(resolved) = callee.as_ref()
                && purity.is_server(&resolved.name)
            {
                return true;
            }
            args.iter().any(|arg| expr_uses_server(&arg.value, purity))
        }

        Expr::Field { object, .. } => expr_uses_server(object, purity),
        Expr::Index { object, index } => {
            expr_uses_server(object, purity) || expr_uses_server(index, purity)
        }
        Expr::Block { stmts, .. } => stmts.iter().any(|stmt| stmt_uses_server(stmt, purity)),

        Expr::When { subject, arms } => {
            if expr_uses_server(subject, purity) {
                return true;
            }
            arms.iter().any(|arm| arm_uses_server(arm, purity))
        }

        Expr::Object(fields) | Expr::Map(fields) => fields
            .iter()
            .any(|f| expr_uses_server(&f.value, purity)),

        Expr::Array(items) => items.iter().any(|e| expr_uses_server(e, purity)),

        Expr::Node(node) => node_uses_server(node, purity),

        Expr::Paren(inner) | Expr::Await(inner) => expr_uses_server(inner, purity),

        Expr::TryCatch {
            body, catch_body, ..
        } => expr_uses_server(body, purity) || expr_uses_server(catch_body, purity),

        Expr::Closure { body, .. } => expr_uses_server(body, purity),
    }
}

fn stmt_uses_server(stmt: &Stmt, purity: &PurityMap) -> bool {
    match stmt {
        Stmt::Binding(b) => b.value.as_ref().is_some_and(|v| expr_uses_server(v, purity)),
        Stmt::Return(Some(e)) | Stmt::Expr(e) => expr_uses_server(e, purity),
        Stmt::Return(None) | Stmt::Error => false,
        Stmt::If(IfStmt {
            condition,
            then_body,
            else_body,
            ..
        }) => {
            expr_uses_server(condition, purity)
                || expr_uses_server(then_body, purity)
                || else_body.as_ref().is_some_and(|e| expr_uses_server(e, purity))
        }
        Stmt::For(for_stmt) => {
            expr_uses_server(&for_stmt.iterable, purity)
                || expr_uses_server(&for_stmt.body, purity)
        }
        Stmt::While(while_stmt) => {
            expr_uses_server(&while_stmt.condition, purity)
                || expr_uses_server(&while_stmt.body, purity)
        }
    }
}

fn arm_uses_server(arm: &WhenArm, purity: &PurityMap) -> bool {
    arm.guard.as_ref().is_some_and(|g| expr_uses_server(g, purity))
        || expr_uses_server(&arm.body, purity)
}

fn node_uses_server(node: &NodeExpr, purity: &PurityMap) -> bool {
    if is_server_domain_node(&node.name) {
        return true;
    }
    // The name might also be a callable (define call like `@Health` or
    // `@User`).  If so, propagate the callee's purity.
    if purity.is_server(&node.name) {
        return true;
    }
    if node
        .positional
        .iter()
        .any(|e| expr_uses_server(e, purity))
    {
        return true;
    }
    if node
        .properties
        .iter()
        .any(|p| expr_uses_server(&p.value, purity))
    {
        return true;
    }
    if let Some(body) = &node.body
        && expr_uses_server(body, purity)
    {
        return true;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_workspace_has_empty_purity() {
        let purity = analyze_workspace_purity(&[]);
        assert!(!purity.is_server("anything"));
    }
}
