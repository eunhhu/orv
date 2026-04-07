use std::collections::HashMap;

use orv_hir::{
    AssignOp, BinaryOp, Binding, Expr, ForStmt, IfStmt, Pattern, Stmt, StringPart, UnaryOp,
    WhenArm, WhileStmt,
};

use crate::{Route, RouteAction};

// ── Value ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum Value {
    Void,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Array(Vec<Value>),
    Map(HashMap<String, Value>),
    Object(HashMap<String, Value>),
    Function {
        params: Vec<String>,
        body: Box<Expr>,
        env: Env,
    },
    BuiltinFn(String),
    Node {
        name: String,
        properties: HashMap<String, Value>,
        children: Vec<Value>,
    },
    RouteRef {
        name: String,
        method: String,
        path: String,
    },
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Void, Self::Void) => true,
            (Self::Bool(a), Self::Bool(b)) => a == b,
            (Self::Int(a), Self::Int(b)) => a == b,
            (Self::Float(a), Self::Float(b)) => a == b,
            (Self::String(a), Self::String(b)) => a == b,
            (Self::Array(a), Self::Array(b)) => a == b,
            (
                Self::RouteRef {
                    name: n1,
                    method: m1,
                    path: p1,
                },
                Self::RouteRef {
                    name: n2,
                    method: m2,
                    path: p2,
                },
            ) => n1 == n2 && m1 == m2 && p1 == p2,
            _ => false,
        }
    }
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Void => write!(f, "void"),
            Self::Bool(b) => write!(f, "{b}"),
            Self::Int(n) => write!(f, "{n}"),
            Self::Float(n) => write!(f, "{n}"),
            Self::String(s) => write!(f, "{s}"),
            Self::Array(items) => {
                write!(f, "[")?;
                for (i, v) in items.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{v}")?;
                }
                write!(f, "]")
            }
            Self::Map(_) => write!(f, "#{{...}}"),
            Self::Object(_) => write!(f, "{{...}}"),
            Self::Function { .. } => write!(f, "<function>"),
            Self::BuiltinFn(name) => write!(f, "<builtin:{name}>"),
            Self::Node { name, .. } => write!(f, "<node:{name}>"),
            Self::RouteRef { method, path, .. } => write!(f, "<route:{method} {path}>"),
        }
    }
}

// ── Environment ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default)]
pub struct Env {
    scopes: Vec<HashMap<String, Value>>,
}

impl Env {
    pub fn new() -> Self {
        Self {
            scopes: vec![HashMap::new()],
        }
    }

    pub fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    pub fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    pub fn set(&mut self, name: String, value: Value) {
        self.scopes.last_mut().unwrap().insert(name, value);
    }

    pub fn get(&self, name: &str) -> Option<&Value> {
        for scope in self.scopes.iter().rev() {
            if let Some(v) = scope.get(name) {
                return Some(v);
            }
        }
        None
    }

    pub fn update(&mut self, name: &str, value: Value) -> bool {
        for scope in self.scopes.iter_mut().rev() {
            if scope.contains_key(name) {
                scope.insert(name.to_owned(), value);
                return true;
            }
        }
        false
    }
}

// ── Signal Store ─────────────────────────────────────────────────────────────

/// Unique identifier for a signal.
pub type SignalId = usize;

/// A reactive signal with its current value and subscriber tracking.
#[derive(Debug, Clone)]
pub struct Signal {
    pub name: String,
    pub value: Value,
    pub subscribers: Vec<String>,
}

/// Reactive signal store that tracks signal values and dependencies.
#[derive(Debug, Clone, Default)]
pub struct SignalStore {
    signals: Vec<Signal>,
    name_to_id: HashMap<String, SignalId>,
    /// When set, any signal read is recorded as a dependency of this context.
    tracking_context: Option<String>,
    /// Signals that were modified and need their subscribers notified.
    dirty: Vec<SignalId>,
}

impl SignalStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a new signal, returning its id.
    pub fn create(&mut self, name: String, initial: Value) -> SignalId {
        let id = self.signals.len();
        self.signals.push(Signal {
            name: name.clone(),
            value: initial,
            subscribers: Vec::new(),
        });
        self.name_to_id.insert(name, id);
        id
    }

    /// Check if a name is a registered signal.
    pub fn is_signal(&self, name: &str) -> bool {
        self.name_to_id.contains_key(name)
    }

    /// Get the current value of a signal, recording the read if tracking.
    pub fn get(&mut self, name: &str) -> Option<Value> {
        let id = *self.name_to_id.get(name)?;
        // Track dependency if in a tracking context
        if let Some(ref ctx) = self.tracking_context {
            let signal = &mut self.signals[id];
            if !signal.subscribers.contains(ctx) {
                signal.subscribers.push(ctx.clone());
            }
        }
        Some(self.signals[id].value.clone())
    }

    /// Update a signal's value. Returns the list of affected subscriber contexts.
    pub fn set(&mut self, name: &str, value: Value) -> Vec<String> {
        let Some(&id) = self.name_to_id.get(name) else {
            return Vec::new();
        };
        self.signals[id].value = value;
        self.dirty.push(id);
        self.signals[id].subscribers.clone()
    }

    /// Start tracking signal reads for a given render context.
    pub fn start_tracking(&mut self, context: String) {
        self.tracking_context = Some(context);
    }

    /// Stop tracking and return the context name.
    pub fn stop_tracking(&mut self) -> Option<String> {
        self.tracking_context.take()
    }

    /// Drain and return all dirty signal ids.
    pub fn drain_dirty(&mut self) -> Vec<SignalId> {
        std::mem::take(&mut self.dirty)
    }

    /// Get signal metadata by id.
    pub fn get_by_id(&self, id: SignalId) -> Option<&Signal> {
        self.signals.get(id)
    }

    /// Get all signal names and their current values (for serialization to client).
    pub fn snapshot(&self) -> Vec<(String, Value)> {
        self.signals
            .iter()
            .map(|s| (s.name.clone(), s.value.clone()))
            .collect()
    }
}

// ── EvalError ────────────────────────────────────────────────────────────────

#[derive(Debug)]
pub enum EvalError {
    UndefinedVariable(String),
    TypeMismatch(String),
    DivisionByZero,
    IndexOutOfBounds,
    NotCallable,
    Return(Value),
    Custom(String),
}

impl std::fmt::Display for EvalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UndefinedVariable(name) => write!(f, "undefined variable: {name}"),
            Self::TypeMismatch(msg) => write!(f, "type mismatch: {msg}"),
            Self::DivisionByZero => write!(f, "division by zero"),
            Self::IndexOutOfBounds => write!(f, "index out of bounds"),
            Self::NotCallable => write!(f, "value is not callable"),
            Self::Return(v) => write!(f, "return {v}"),
            Self::Custom(msg) => write!(f, "{msg}"),
        }
    }
}

// ── Evaluator ────────────────────────────────────────────────────────────────

/// A design token extracted from `@design { @theme { @color "name" "value" } }`.
#[derive(Debug, Clone)]
pub struct DesignToken {
    pub category: String, // "color", "size", "font", "spacing"
    pub name: String,
    pub value: String,
}

pub struct Evaluator {
    pub env: Env,
    pub routes: Vec<Route>,
    pub signals: SignalStore,
    pub design_tokens: Vec<DesignToken>,
}

/// Sanitize a string for use as a CSS custom property name segment.
/// Only allows `[a-zA-Z0-9_-]`; other characters are replaced with `-`.
fn sanitize_css_ident(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '-'
            }
        })
        .collect()
}

/// Sanitize a string for use as a CSS property value.
/// Rejects characters that could break out of a CSS declaration:
/// `}`, `{`, `;`, `<`, `>`, and newlines.
fn sanitize_css_value(s: &str) -> String {
    s.chars()
        .filter(|c| !matches!(c, '}' | '{' | ';' | '<' | '>' | '\n' | '\r'))
        .collect()
}

impl Evaluator {
    pub fn new() -> Self {
        Self {
            env: Env::new(),
            routes: Vec::new(),
            signals: SignalStore::new(),
            design_tokens: Vec::new(),
        }
    }

    pub fn with_routes(routes: Vec<Route>) -> Self {
        Self {
            env: Env::new(),
            routes,
            signals: SignalStore::new(),
            design_tokens: Vec::new(),
        }
    }

    /// Generate CSS custom properties from design tokens.
    ///
    /// Token names are sanitized to `[a-zA-Z0-9_-]` and values are sanitized
    /// to reject characters that could break out of a CSS declaration.
    /// Duplicate token names are de-duplicated — only the *last* definition
    /// survives, matching CSS cascade semantics while removing wasted bytes.
    pub fn design_tokens_to_css(&self) -> String {
        if self.design_tokens.is_empty() {
            return String::new();
        }
        // Walk in original order, but only keep the *latest* definition for
        // each fully-qualified name. We materialize a small dedup map and
        // then re-order according to first-seen index for stable output.
        let mut last: std::collections::HashMap<String, (usize, String)> =
            std::collections::HashMap::new();
        let mut order: Vec<String> = Vec::new();
        for token in &self.design_tokens {
            let safe_cat = sanitize_css_ident(&token.category);
            let safe_name = sanitize_css_ident(&token.name);
            let safe_value = sanitize_css_value(&token.value);
            let key = format!("--orv-{safe_cat}-{safe_name}");
            if !last.contains_key(&key) {
                order.push(key.clone());
            }
            let idx = last.get(&key).map_or(order.len() - 1, |(i, _)| *i);
            last.insert(key, (idx, safe_value));
        }
        let mut css = String::from(":root {\n");
        for key in &order {
            if let Some((_, value)) = last.get(key) {
                css.push_str(&format!("  {key}: {value};\n"));
            }
        }
        css.push_str("}\n");
        css
    }

    /// Like [`design_tokens_to_css`], but only emits properties whose names
    /// appear in the provided `used` set. Returns the empty string when no
    /// tokens survive — callers can then skip the `<style>` block entirely.
    ///
    /// `used` should contain the *full* CSS custom property name including
    /// the leading `--`, e.g. `"--orv-color-primary"`.
    pub fn design_tokens_to_css_filtered(
        &self,
        used: &std::collections::HashSet<String>,
    ) -> String {
        if self.design_tokens.is_empty() || used.is_empty() {
            return String::new();
        }
        let mut last: std::collections::HashMap<String, (usize, String)> =
            std::collections::HashMap::new();
        let mut order: Vec<String> = Vec::new();
        for token in &self.design_tokens {
            let safe_cat = sanitize_css_ident(&token.category);
            let safe_name = sanitize_css_ident(&token.name);
            let safe_value = sanitize_css_value(&token.value);
            let key = format!("--orv-{safe_cat}-{safe_name}");
            if !used.contains(&key) {
                continue;
            }
            if !last.contains_key(&key) {
                order.push(key.clone());
            }
            let idx = last.get(&key).map_or(order.len() - 1, |(i, _)| *i);
            last.insert(key, (idx, safe_value));
        }
        if order.is_empty() {
            return String::new();
        }
        // Minified output: no newlines, no extra whitespace.
        let mut css = String::from(":root{");
        for key in &order {
            if let Some((_, value)) = last.get(key) {
                css.push_str(key);
                css.push(':');
                css.push_str(value);
                css.push(';');
            }
        }
        css.push('}');
        css
    }

    pub fn eval_expr(&mut self, expr: &Expr) -> Result<Value, EvalError> {
        match expr {
            Expr::IntLiteral(n) => Ok(Value::Int(*n)),
            Expr::FloatLiteral(n) => Ok(Value::Float(*n)),
            Expr::StringLiteral(s) => Ok(Value::String(s.clone())),
            Expr::BoolLiteral(b) => Ok(Value::Bool(*b)),
            Expr::Void => Ok(Value::Void),

            Expr::StringInterp(parts) => {
                let mut result = String::new();
                for part in parts {
                    match part {
                        StringPart::Lit(s) => result.push_str(s),
                        StringPart::Expr(e) => {
                            let v = self.eval_expr(e)?;
                            result.push_str(&v.to_string());
                        }
                    }
                }
                Ok(Value::String(result))
            }

            Expr::Ident(resolved) => {
                // If this is a signal, read through signal store for dependency tracking
                if self.signals.is_signal(&resolved.name) {
                    return self
                        .signals
                        .get(&resolved.name)
                        .ok_or_else(|| EvalError::UndefinedVariable(resolved.name.clone()));
                }
                self.env
                    .get(&resolved.name)
                    .cloned()
                    .ok_or_else(|| EvalError::UndefinedVariable(resolved.name.clone()))
            }

            Expr::Binary { left, op, right } => self.eval_binary(left, *op, right),

            Expr::Unary { op, operand } => {
                let val = self.eval_expr(operand)?;
                match op {
                    UnaryOp::Neg => match val {
                        Value::Int(n) => Ok(Value::Int(-n)),
                        Value::Float(n) => Ok(Value::Float(-n)),
                        other => Err(EvalError::TypeMismatch(format!("cannot negate {other}"))),
                    },
                    UnaryOp::Not => match val {
                        Value::Bool(b) => Ok(Value::Bool(!b)),
                        other => Err(EvalError::TypeMismatch(format!(
                            "cannot apply ! to {other}"
                        ))),
                    },
                }
            }

            Expr::Assign { target, op, value } => {
                let val = self.eval_expr(value)?;
                let name = match target.as_ref() {
                    Expr::Ident(resolved) => resolved.name.clone(),
                    other => {
                        return Err(EvalError::TypeMismatch(format!(
                            "assignment target must be an identifier, got {other:?}"
                        )));
                    }
                };
                let new_val = match op {
                    AssignOp::Assign => val,
                    AssignOp::AddAssign => {
                        let existing = self
                            .env
                            .get(&name)
                            .cloned()
                            .ok_or_else(|| EvalError::UndefinedVariable(name.clone()))?;
                        add_values(existing, val)?
                    }
                    AssignOp::SubAssign => {
                        let existing = self
                            .env
                            .get(&name)
                            .cloned()
                            .ok_or_else(|| EvalError::UndefinedVariable(name.clone()))?;
                        sub_values(existing, val)?
                    }
                };
                // Update signal store if this is a signal
                if self.signals.is_signal(&name) {
                    self.signals.set(&name, new_val.clone());
                }
                if !self.env.update(&name, new_val.clone()) {
                    self.env.set(name, new_val.clone());
                }
                Ok(new_val)
            }

            Expr::Call { callee, args } => self.eval_call(callee, args),

            Expr::Field { object, field } => {
                let obj_val = self.eval_expr(object)?;
                match obj_val {
                    Value::Object(map) | Value::Map(map) => map
                        .get(field)
                        .cloned()
                        .ok_or_else(|| EvalError::Custom(format!("field `{field}` not found"))),
                    other => Err(EvalError::TypeMismatch(format!(
                        "cannot access field on {other}"
                    ))),
                }
            }

            Expr::Index { object, index } => {
                let obj_val = self.eval_expr(object)?;
                let idx_val = self.eval_expr(index)?;
                match (obj_val, idx_val) {
                    (Value::Array(items), Value::Int(i)) => {
                        let idx = usize::try_from(i).map_err(|_| EvalError::IndexOutOfBounds)?;
                        items.get(idx).cloned().ok_or(EvalError::IndexOutOfBounds)
                    }
                    (Value::Map(map), Value::String(key)) => map
                        .get(&key)
                        .cloned()
                        .ok_or_else(|| EvalError::Custom(format!("key `{key}` not found"))),
                    (Value::Object(map), Value::String(key)) => map
                        .get(&key)
                        .cloned()
                        .ok_or_else(|| EvalError::Custom(format!("key `{key}` not found"))),
                    (obj, idx) => Err(EvalError::TypeMismatch(format!(
                        "cannot index {obj} with {idx}"
                    ))),
                }
            }

            Expr::Block { stmts, .. } => {
                self.env.push_scope();
                let result = self.eval_block(stmts);
                self.env.pop_scope();
                result
            }

            Expr::When { subject, arms } => {
                let subject_val = self.eval_expr(subject)?;
                self.eval_when(&subject_val, arms)
            }

            Expr::Object(fields) => {
                let mut map = HashMap::new();
                for field in fields {
                    let val = self.eval_expr(&field.value)?;
                    map.insert(field.key.clone(), val);
                }
                Ok(Value::Object(map))
            }

            Expr::Map(fields) => {
                let mut map = HashMap::new();
                for field in fields {
                    let val = self.eval_expr(&field.value)?;
                    map.insert(field.key.clone(), val);
                }
                Ok(Value::Map(map))
            }

            Expr::Array(items) => {
                let mut values = Vec::with_capacity(items.len());
                for item in items {
                    values.push(self.eval_expr(item)?);
                }
                Ok(Value::Array(values))
            }

            Expr::Node(node) => {
                // @design nodes extract tokens and return Void
                if node.name == "design" {
                    self.extract_design_tokens(node);
                    return Ok(Value::Void);
                }

                // @route nodes with at least method + path become RouteRef values
                if node.name == "route" && node.positional.len() >= 2 {
                    let method = match &node.positional[0] {
                        Expr::Ident(r) => r.name.clone(),
                        Expr::StringLiteral(s) => s.clone(),
                        other => {
                            return Err(EvalError::TypeMismatch(format!(
                                "@route method must be an identifier, got {other:?}"
                            )));
                        }
                    };
                    let path = match &node.positional[1] {
                        Expr::Ident(r) => r.name.clone(),
                        Expr::StringLiteral(s) => s.clone(),
                        other => {
                            return Err(EvalError::TypeMismatch(format!(
                                "@route path must be an identifier, got {other:?}"
                            )));
                        }
                    };
                    return Ok(Value::RouteRef {
                        name: format!("{method} {path}"),
                        method,
                        path,
                    });
                }

                let mut children = Vec::new();
                for expr in &node.positional {
                    children.push(self.eval_expr(expr)?);
                }
                if let Some(body) = &node.body {
                    children.push(self.eval_expr(body)?);
                }
                let mut properties = HashMap::new();
                for prop in &node.properties {
                    properties.insert(prop.name.clone(), self.eval_expr(&prop.value)?);
                }
                Ok(Value::Node {
                    name: node.name.clone(),
                    properties,
                    children,
                })
            }

            Expr::Paren(inner) => self.eval_expr(inner),

            Expr::Await(inner) => self.eval_expr(inner),

            Expr::TryCatch {
                body,
                catch_binding,
                catch_body,
                ..
            } => match self.eval_expr(body) {
                Ok(v) => Ok(v),
                Err(e) => {
                    let err_msg = e.to_string();
                    self.env.push_scope();
                    self.env.set(catch_binding.clone(), Value::String(err_msg));
                    let result = self.eval_expr(catch_body);
                    self.env.pop_scope();
                    result
                }
            },

            Expr::Closure { params, body } => {
                let param_names: Vec<String> = params.iter().map(|p| p.name.clone()).collect();
                Ok(Value::Function {
                    params: param_names,
                    body: Box::new(*body.clone()),
                    env: self.env.clone(),
                })
            }

            Expr::Error => Err(EvalError::Custom("encountered error node".to_owned())),
        }
    }

    fn eval_binary(&mut self, left: &Expr, op: BinaryOp, right: &Expr) -> Result<Value, EvalError> {
        // Short-circuit logical operators
        match op {
            BinaryOp::And => {
                let lv = self.eval_expr(left)?;
                match lv {
                    Value::Bool(false) => return Ok(Value::Bool(false)),
                    Value::Bool(true) => {
                        let rv = self.eval_expr(right)?;
                        return match rv {
                            Value::Bool(b) => Ok(Value::Bool(b)),
                            other => Err(EvalError::TypeMismatch(format!(
                                "&& requires bool, got {other}"
                            ))),
                        };
                    }
                    other => {
                        return Err(EvalError::TypeMismatch(format!(
                            "&& requires bool, got {other}"
                        )));
                    }
                }
            }
            BinaryOp::Or => {
                let lv = self.eval_expr(left)?;
                match lv {
                    Value::Bool(true) => return Ok(Value::Bool(true)),
                    Value::Bool(false) => {
                        let rv = self.eval_expr(right)?;
                        return match rv {
                            Value::Bool(b) => Ok(Value::Bool(b)),
                            other => Err(EvalError::TypeMismatch(format!(
                                "|| requires bool, got {other}"
                            ))),
                        };
                    }
                    other => {
                        return Err(EvalError::TypeMismatch(format!(
                            "|| requires bool, got {other}"
                        )));
                    }
                }
            }
            BinaryOp::NullCoalesce => {
                let lv = self.eval_expr(left)?;
                return match lv {
                    Value::Void => self.eval_expr(right),
                    other => Ok(other),
                };
            }
            BinaryOp::Pipe => {
                let lv = self.eval_expr(left)?;
                // Pipe: call right with left as sole positional argument
                return self.call_value_with_args(right, vec![lv]);
            }
            _ => {}
        }

        let lv = self.eval_expr(left)?;
        let rv = self.eval_expr(right)?;

        match op {
            BinaryOp::Add => add_values(lv, rv),
            BinaryOp::Sub => sub_values(lv, rv),
            BinaryOp::Mul => mul_values(lv, rv),
            BinaryOp::Div => div_values(lv, rv),
            BinaryOp::Eq => Ok(Value::Bool(lv == rv)),
            BinaryOp::NotEq => Ok(Value::Bool(lv != rv)),
            BinaryOp::Lt => cmp_values(&lv, &rv, op),
            BinaryOp::LtEq => cmp_values(&lv, &rv, op),
            BinaryOp::Gt => cmp_values(&lv, &rv, op),
            BinaryOp::GtEq => cmp_values(&lv, &rv, op),
            BinaryOp::Range => make_range(&lv, &rv, false),
            BinaryOp::RangeInclusive => make_range(&lv, &rv, true),
            // Already handled above
            BinaryOp::And | BinaryOp::Or | BinaryOp::NullCoalesce | BinaryOp::Pipe => {
                unreachable!()
            }
        }
    }

    /// Evaluate `right` as a callee expression, passing `args` as positional args.
    fn call_value_with_args(
        &mut self,
        callee_expr: &Expr,
        args: Vec<Value>,
    ) -> Result<Value, EvalError> {
        let callee = self.eval_expr(callee_expr)?;
        self.invoke(callee, args)
    }

    /// Extract design tokens from a `@design { @theme { @color "name" "value" } }` node.
    fn extract_design_tokens(&mut self, node: &orv_hir::NodeExpr) {
        // @design body contains @theme blocks
        if let Some(body) = &node.body {
            self.extract_tokens_from_expr(body);
        }
        for child in &node.positional {
            self.extract_tokens_from_expr(child);
        }
    }

    pub fn extract_tokens_from_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::Block { stmts, .. } => {
                for stmt in stmts {
                    if let Stmt::Expr(inner) = stmt {
                        self.extract_tokens_from_expr(inner);
                    }
                }
            }
            Expr::Node(child) => {
                if child.name == "theme" {
                    // @theme contains token definitions
                    if let Some(body) = &child.body {
                        self.extract_tokens_from_expr(body);
                    }
                    for pos in &child.positional {
                        self.extract_tokens_from_expr(pos);
                    }
                } else if matches!(child.name.as_str(), "color" | "size" | "font" | "spacing") {
                    // @color "name" "value", @size "name" "value", etc.
                    // @font "name" "family" "size" — 3rd positional becomes a -size token
                    if child.positional.len() >= 2 {
                        let name = match &child.positional[0] {
                            Expr::StringLiteral(s) => s.clone(),
                            Expr::Ident(r) => r.name.clone(),
                            _ => return,
                        };
                        let value = match &child.positional[1] {
                            Expr::StringLiteral(s) => s.clone(),
                            Expr::Ident(r) => r.name.clone(),
                            _ => return,
                        };
                        self.design_tokens.push(DesignToken {
                            category: child.name.clone(),
                            name: name.clone(),
                            value,
                        });
                        // @font with 3rd positional: emit a companion font-size token
                        if child.name == "font"
                            && child.positional.len() >= 3
                            && let Some(size_val) = (match &child.positional[2] {
                                Expr::StringLiteral(s) => Some(s.clone()),
                                Expr::Ident(r) => Some(r.name.clone()),
                                _ => None,
                            })
                        {
                            self.design_tokens.push(DesignToken {
                                category: "font-size".to_owned(),
                                name,
                                value: size_val,
                            });
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn eval_call(
        &mut self,
        callee_expr: &Expr,
        args: &[orv_hir::CallArg],
    ) -> Result<Value, EvalError> {
        // Check for method calls: `object.method(args)`
        if let Expr::Field { object, field } = callee_expr {
            let obj_val = self.eval_expr(object)?;
            let mut arg_vals = Vec::with_capacity(args.len());
            for arg in args {
                arg_vals.push(self.eval_expr(&arg.value)?);
            }
            return self.call_method(obj_val, field, arg_vals);
        }

        let callee = self.eval_expr(callee_expr)?;
        let mut arg_vals = Vec::with_capacity(args.len());
        for arg in args {
            arg_vals.push(self.eval_expr(&arg.value)?);
        }
        self.invoke(callee, arg_vals)
    }

    fn invoke(&mut self, callee: Value, args: Vec<Value>) -> Result<Value, EvalError> {
        match callee {
            Value::Function { params, body, env } => {
                let saved_env = std::mem::replace(&mut self.env, env);
                self.env.push_scope();
                for (param, arg) in params.iter().zip(args.iter()) {
                    self.env.set(param.clone(), arg.clone());
                }
                let result = self.eval_expr(&body);
                self.env.pop_scope();
                self.env = saved_env;
                match result {
                    Err(EvalError::Return(v)) => Ok(v),
                    other => other,
                }
            }
            Value::BuiltinFn(name) => Err(EvalError::Custom(format!(
                "builtin function `{name}` cannot be called without a receiver"
            ))),
            other => Err(EvalError::TypeMismatch(format!("{other} is not callable"))),
        }
    }

    fn call_method(
        &mut self,
        receiver: Value,
        method: &str,
        args: Vec<Value>,
    ) -> Result<Value, EvalError> {
        match (&receiver, method) {
            // ── Array methods ────────────────────────────────────────────────
            (Value::Array(_), "len") => {
                let Value::Array(items) = receiver else {
                    unreachable!()
                };
                Ok(Value::Int(items.len() as i64))
            }
            (Value::Array(_), "push") => {
                let Value::Array(mut items) = receiver else {
                    unreachable!()
                };
                let arg = args
                    .into_iter()
                    .next()
                    .ok_or_else(|| EvalError::Custom("push requires an argument".to_owned()))?;
                items.push(arg);
                Ok(Value::Array(items))
            }
            (Value::Array(_), "pop") => {
                let Value::Array(mut items) = receiver else {
                    unreachable!()
                };
                Ok(items.pop().unwrap_or(Value::Void))
            }
            (Value::Array(_), "map") => {
                let Value::Array(items) = receiver else {
                    unreachable!()
                };
                let func = args.into_iter().next().ok_or_else(|| {
                    EvalError::Custom("map requires a function argument".to_owned())
                })?;
                let mut result = Vec::with_capacity(items.len());
                for item in items {
                    result.push(self.invoke(func.clone(), vec![item])?);
                }
                Ok(Value::Array(result))
            }
            (Value::Array(_), "filter") => {
                let Value::Array(items) = receiver else {
                    unreachable!()
                };
                let func = args.into_iter().next().ok_or_else(|| {
                    EvalError::Custom("filter requires a function argument".to_owned())
                })?;
                let mut result = Vec::new();
                for item in items {
                    let keep = self.invoke(func.clone(), vec![item.clone()])?;
                    match keep {
                        Value::Bool(true) => result.push(item),
                        Value::Bool(false) => {}
                        other => {
                            return Err(EvalError::TypeMismatch(format!(
                                "filter predicate must return bool, got {other}"
                            )));
                        }
                    }
                }
                Ok(Value::Array(result))
            }
            (Value::Array(_), "contains") => {
                let Value::Array(items) = receiver else {
                    unreachable!()
                };
                let target = args
                    .into_iter()
                    .next()
                    .ok_or_else(|| EvalError::Custom("contains requires an argument".to_owned()))?;
                Ok(Value::Bool(items.contains(&target)))
            }

            // ── String methods ───────────────────────────────────────────────
            (Value::String(_), "len") => {
                let Value::String(s) = receiver else {
                    unreachable!()
                };
                Ok(Value::Int(s.len() as i64))
            }
            (Value::String(_), "split") => {
                let Value::String(s) = receiver else {
                    unreachable!()
                };
                let sep = match args.into_iter().next() {
                    Some(Value::String(sep)) => sep,
                    _ => {
                        return Err(EvalError::Custom(
                            "split requires a string separator".to_owned(),
                        ));
                    }
                };
                Ok(Value::Array(
                    s.split(sep.as_str())
                        .map(|p| Value::String(p.to_owned()))
                        .collect(),
                ))
            }
            (Value::String(_), "trim") => {
                let Value::String(s) = receiver else {
                    unreachable!()
                };
                Ok(Value::String(s.trim().to_owned()))
            }
            (Value::String(_), "contains") => {
                let Value::String(s) = receiver else {
                    unreachable!()
                };
                let needle = match args.into_iter().next() {
                    Some(Value::String(n)) => n,
                    _ => {
                        return Err(EvalError::Custom(
                            "contains requires a string argument".to_owned(),
                        ));
                    }
                };
                Ok(Value::Bool(s.contains(needle.as_str())))
            }
            (Value::String(_), "to_upper") => {
                let Value::String(s) = receiver else {
                    unreachable!()
                };
                Ok(Value::String(s.to_uppercase()))
            }
            (Value::String(_), "to_lower") => {
                let Value::String(s) = receiver else {
                    unreachable!()
                };
                Ok(Value::String(s.to_lowercase()))
            }
            (Value::String(_), "replace") => {
                let Value::String(s) = receiver else {
                    unreachable!()
                };
                let mut it = args.into_iter();
                let from = match it.next() {
                    Some(Value::String(f)) => f,
                    _ => {
                        return Err(EvalError::Custom(
                            "replace requires two string arguments".to_owned(),
                        ));
                    }
                };
                let to = match it.next() {
                    Some(Value::String(t)) => t,
                    _ => {
                        return Err(EvalError::Custom(
                            "replace requires two string arguments".to_owned(),
                        ));
                    }
                };
                Ok(Value::String(s.replace(from.as_str(), to.as_str())))
            }

            // ── Map / Object methods ─────────────────────────────────────────
            (Value::Map(_) | Value::Object(_), "len") => {
                let len = match receiver {
                    Value::Map(m) => m.len(),
                    Value::Object(m) => m.len(),
                    _ => unreachable!(),
                };
                Ok(Value::Int(len as i64))
            }
            (Value::Map(_) | Value::Object(_), "keys") => {
                let keys: Vec<Value> = match receiver {
                    Value::Map(m) => m.into_keys().map(Value::String).collect(),
                    Value::Object(m) => m.into_keys().map(Value::String).collect(),
                    _ => unreachable!(),
                };
                Ok(Value::Array(keys))
            }
            (Value::Map(_) | Value::Object(_), "values") => {
                let vals: Vec<Value> = match receiver {
                    Value::Map(m) => m.into_values().collect(),
                    Value::Object(m) => m.into_values().collect(),
                    _ => unreachable!(),
                };
                Ok(Value::Array(vals))
            }
            (Value::Map(_) | Value::Object(_), "contains_key") => {
                let key = match args.into_iter().next() {
                    Some(Value::String(k)) => k,
                    _ => {
                        return Err(EvalError::Custom(
                            "contains_key requires a string argument".to_owned(),
                        ));
                    }
                };
                let found = match receiver {
                    Value::Map(m) => m.contains_key(&key),
                    Value::Object(m) => m.contains_key(&key),
                    _ => unreachable!(),
                };
                Ok(Value::Bool(found))
            }

            // ── RouteRef methods ─────────────────────────────────────────────
            (Value::RouteRef { .. }, "fetch") => {
                let Value::RouteRef {
                    method: route_method,
                    path: route_path,
                    ..
                } = receiver
                else {
                    unreachable!()
                };
                // Find matching route in route table
                let route = self
                    .routes
                    .iter()
                    .find(|r| r.method == route_method && r.path == route_path)
                    .cloned();
                match route {
                    Some(r) => match &r.action {
                        RouteAction::JsonResponse { body, .. } => {
                            // Parse the baked JSON body back into a Value
                            match serde_json::from_str::<serde_json::Value>(body) {
                                Ok(json_val) => Ok(json_to_value(json_val)),
                                Err(_) => Ok(Value::String(body.clone())),
                            }
                        }
                        RouteAction::StaticServe { target } => {
                            Ok(Value::String(format!("serve {target}")))
                        }
                        RouteAction::HtmlServe { html } => Ok(Value::String(html.clone())),
                    },
                    None => Err(EvalError::Custom(format!(
                        "no route matched {route_method} {route_path}"
                    ))),
                }
            }
            (Value::RouteRef { .. }, _) => Err(EvalError::Custom(format!(
                "no method `{method}` on route reference"
            ))),

            _ => Err(EvalError::Custom(format!(
                "no method `{method}` on {}",
                receiver
            ))),
        }
    }

    fn eval_block(&mut self, stmts: &[Stmt]) -> Result<Value, EvalError> {
        let mut last = Value::Void;
        for stmt in stmts {
            last = self.eval_stmt(stmt)?;
        }
        Ok(last)
    }

    pub fn eval_when(&mut self, subject: &Value, arms: &[WhenArm]) -> Result<Value, EvalError> {
        for arm in arms {
            self.env.push_scope();
            let matched = match_pattern(&arm.pattern, subject, &mut self.env);
            if matched {
                // Evaluate guard if present
                if let Some(guard) = &arm.guard {
                    match self.eval_expr(guard)? {
                        Value::Bool(true) => {}
                        Value::Bool(false) => {
                            self.env.pop_scope();
                            continue;
                        }
                        other => {
                            self.env.pop_scope();
                            return Err(EvalError::TypeMismatch(format!(
                                "when guard must be bool, got {other}"
                            )));
                        }
                    }
                }
                let result = self.eval_expr(&arm.body);
                self.env.pop_scope();
                return result;
            }
            self.env.pop_scope();
        }
        Ok(Value::Void)
    }

    pub fn eval_stmt(&mut self, stmt: &Stmt) -> Result<Value, EvalError> {
        match stmt {
            Stmt::Binding(binding) => self.eval_binding(binding),
            Stmt::Return(expr) => {
                let val = match expr {
                    Some(e) => self.eval_expr(e)?,
                    None => Value::Void,
                };
                Err(EvalError::Return(val))
            }
            Stmt::If(if_stmt) => self.eval_if(if_stmt),
            Stmt::For(for_stmt) => self.eval_for(for_stmt),
            Stmt::While(while_stmt) => self.eval_while(while_stmt),
            Stmt::Expr(expr) => self.eval_expr(expr),
            Stmt::Error => Err(EvalError::Custom("encountered error statement".to_owned())),
        }
    }

    fn eval_binding(&mut self, binding: &Binding) -> Result<Value, EvalError> {
        let val = match &binding.value {
            Some(expr) => self.eval_expr(expr)?,
            None => Value::Void,
        };
        if binding.is_sig {
            self.signals.create(binding.name.clone(), val.clone());
        }
        self.env.set(binding.name.clone(), val);
        Ok(Value::Void)
    }

    fn eval_if(&mut self, if_stmt: &IfStmt) -> Result<Value, EvalError> {
        let condition = self.eval_expr(&if_stmt.condition)?;
        match condition {
            Value::Bool(true) => {
                self.env.push_scope();
                let result = self.eval_expr(&if_stmt.then_body);
                self.env.pop_scope();
                result
            }
            Value::Bool(false) => match &if_stmt.else_body {
                Some(else_expr) => {
                    self.env.push_scope();
                    let result = self.eval_expr(else_expr);
                    self.env.pop_scope();
                    result
                }
                None => Ok(Value::Void),
            },
            other => Err(EvalError::TypeMismatch(format!(
                "if condition must be bool, got {other}"
            ))),
        }
    }

    fn eval_for(&mut self, for_stmt: &ForStmt) -> Result<Value, EvalError> {
        let iterable = self.eval_expr(&for_stmt.iterable)?;
        let items = match iterable {
            Value::Array(items) => items,
            other => {
                return Err(EvalError::TypeMismatch(format!(
                    "for loop requires an array, got {other}"
                )));
            }
        };
        let mut last = Value::Void;
        for item in items {
            self.env.push_scope();
            self.env.set(for_stmt.binding.clone(), item);
            last = match self.eval_expr(&for_stmt.body) {
                Ok(v) => v,
                Err(EvalError::Return(v)) => {
                    self.env.pop_scope();
                    return Err(EvalError::Return(v));
                }
                Err(e) => {
                    self.env.pop_scope();
                    return Err(e);
                }
            };
            self.env.pop_scope();
        }
        Ok(last)
    }

    fn eval_while(&mut self, while_stmt: &WhileStmt) -> Result<Value, EvalError> {
        let mut last = Value::Void;
        loop {
            let cond = self.eval_expr(&while_stmt.condition)?;
            match cond {
                Value::Bool(true) => {}
                Value::Bool(false) => break,
                other => {
                    return Err(EvalError::TypeMismatch(format!(
                        "while condition must be bool, got {other}"
                    )));
                }
            }
            self.env.push_scope();
            last = match self.eval_expr(&while_stmt.body) {
                Ok(v) => v,
                Err(EvalError::Return(v)) => {
                    self.env.pop_scope();
                    return Err(EvalError::Return(v));
                }
                Err(e) => {
                    self.env.pop_scope();
                    return Err(e);
                }
            };
            self.env.pop_scope();
        }
        Ok(last)
    }
}

impl Default for Evaluator {
    fn default() -> Self {
        Self::new()
    }
}

// ── Pattern matching ─────────────────────────────────────────────────────────

pub fn match_pattern(pattern: &Pattern, value: &Value, env: &mut Env) -> bool {
    match pattern {
        Pattern::Wildcard => true,
        Pattern::Binding(name) => {
            env.set(name.clone(), value.clone());
            true
        }
        Pattern::IntLiteral(n) => matches!(value, Value::Int(v) if v == n),
        Pattern::FloatLiteral(n) => matches!(value, Value::Float(v) if v == n),
        Pattern::StringLiteral(s) => matches!(value, Value::String(v) if v == s),
        Pattern::BoolLiteral(b) => matches!(value, Value::Bool(v) if v == b),
        Pattern::Void => matches!(value, Value::Void),
        Pattern::Variant { path, fields } => {
            // For a node value, check name matches last path segment and recurse on children
            if let Value::Node { name, children, .. } = value {
                let variant_name = path.last().map(String::as_str).unwrap_or("");
                if name != variant_name {
                    return false;
                }
                if fields.len() != children.len() {
                    return false;
                }
                for (field_pat, child_val) in fields.iter().zip(children.iter()) {
                    if !match_pattern(field_pat, child_val, env) {
                        return false;
                    }
                }
                true
            } else {
                false
            }
        }
        Pattern::Or(patterns) => patterns.iter().any(|p| match_pattern(p, value, env)),
        Pattern::Range {
            start,
            end,
            inclusive,
        } => match value {
            Value::Int(n) => {
                let in_start = match start.as_ref() {
                    Pattern::IntLiteral(s) => n >= s,
                    _ => false,
                };
                let in_end = match end.as_ref() {
                    Pattern::IntLiteral(e) => {
                        if *inclusive {
                            n <= e
                        } else {
                            n < e
                        }
                    }
                    _ => false,
                };
                in_start && in_end
            }
            _ => false,
        },
        Pattern::Error => false,
    }
}

// ── JSON helper ──────────────────────────────────────────────────────────────

fn json_to_value(v: serde_json::Value) -> Value {
    match v {
        serde_json::Value::Null => Value::Void,
        serde_json::Value::Bool(b) => Value::Bool(b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Value::Int(i)
            } else {
                Value::Float(n.as_f64().unwrap_or(0.0))
            }
        }
        serde_json::Value::String(s) => Value::String(s),
        serde_json::Value::Array(items) => {
            Value::Array(items.into_iter().map(json_to_value).collect())
        }
        serde_json::Value::Object(map) => Value::Object(
            map.into_iter()
                .map(|(k, v)| (k, json_to_value(v)))
                .collect(),
        ),
    }
}

// ── Arithmetic helpers ───────────────────────────────────────────────────────

fn add_values(lv: Value, rv: Value) -> Result<Value, EvalError> {
    match (lv, rv) {
        (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a + b)),
        (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a + b)),
        (Value::Int(a), Value::Float(b)) => Ok(Value::Float(a as f64 + b)),
        (Value::Float(a), Value::Int(b)) => Ok(Value::Float(a + b as f64)),
        (Value::String(a), Value::String(b)) => Ok(Value::String(a + &b)),
        (a, b) => Err(EvalError::TypeMismatch(format!("cannot add {a} and {b}"))),
    }
}

fn sub_values(lv: Value, rv: Value) -> Result<Value, EvalError> {
    match (lv, rv) {
        (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a - b)),
        (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a - b)),
        (Value::Int(a), Value::Float(b)) => Ok(Value::Float(a as f64 - b)),
        (Value::Float(a), Value::Int(b)) => Ok(Value::Float(a - b as f64)),
        (a, b) => Err(EvalError::TypeMismatch(format!(
            "cannot subtract {a} and {b}"
        ))),
    }
}

fn mul_values(lv: Value, rv: Value) -> Result<Value, EvalError> {
    match (lv, rv) {
        (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a * b)),
        (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a * b)),
        (Value::Int(a), Value::Float(b)) => Ok(Value::Float(a as f64 * b)),
        (Value::Float(a), Value::Int(b)) => Ok(Value::Float(a * b as f64)),
        (a, b) => Err(EvalError::TypeMismatch(format!(
            "cannot multiply {a} and {b}"
        ))),
    }
}

fn div_values(lv: Value, rv: Value) -> Result<Value, EvalError> {
    match (lv, rv) {
        (_, Value::Int(0)) => Err(EvalError::DivisionByZero),
        (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a / b)),
        (Value::Float(a), Value::Float(b)) => {
            if b == 0.0 {
                Err(EvalError::DivisionByZero)
            } else {
                Ok(Value::Float(a / b))
            }
        }
        (Value::Int(a), Value::Float(b)) => {
            if b == 0.0 {
                Err(EvalError::DivisionByZero)
            } else {
                Ok(Value::Float(a as f64 / b))
            }
        }
        (Value::Float(a), Value::Int(b)) => Ok(Value::Float(a / b as f64)),
        (a, b) => Err(EvalError::TypeMismatch(format!(
            "cannot divide {a} and {b}"
        ))),
    }
}

fn cmp_values(lv: &Value, rv: &Value, op: BinaryOp) -> Result<Value, EvalError> {
    let ord = match (lv, rv) {
        (Value::Int(a), Value::Int(b)) => a.cmp(b),
        (Value::Float(a), Value::Float(b)) => a
            .partial_cmp(b)
            .ok_or_else(|| EvalError::TypeMismatch("NaN comparison".to_owned()))?,
        (Value::Int(a), Value::Float(b)) => (*a as f64)
            .partial_cmp(b)
            .ok_or_else(|| EvalError::TypeMismatch("NaN comparison".to_owned()))?,
        (Value::Float(a), Value::Int(b)) => a
            .partial_cmp(&(*b as f64))
            .ok_or_else(|| EvalError::TypeMismatch("NaN comparison".to_owned()))?,
        (Value::String(a), Value::String(b)) => a.cmp(b),
        (a, b) => {
            return Err(EvalError::TypeMismatch(format!(
                "cannot compare {a} and {b}"
            )));
        }
    };
    let result = match op {
        BinaryOp::Lt => ord.is_lt(),
        BinaryOp::LtEq => ord.is_le(),
        BinaryOp::Gt => ord.is_gt(),
        BinaryOp::GtEq => ord.is_ge(),
        _ => unreachable!(),
    };
    Ok(Value::Bool(result))
}

fn make_range(lv: &Value, rv: &Value, inclusive: bool) -> Result<Value, EvalError> {
    match (lv, rv) {
        (Value::Int(start), Value::Int(end)) => {
            let items: Vec<Value> = if inclusive {
                (*start..=*end).map(Value::Int).collect()
            } else {
                (*start..*end).map(Value::Int).collect()
            };
            Ok(Value::Array(items))
        }
        (a, b) => Err(EvalError::TypeMismatch(format!(
            "range requires integers, got {a} and {b}"
        ))),
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use orv_hir::{
        BinaryOp, Binding, CallArg, Expr, ForStmt, IfStmt, ObjectField, Param, Pattern,
        ResolvedName, Stmt, WhenArm, WhileStmt,
    };

    fn int(n: i64) -> Expr {
        Expr::IntLiteral(n)
    }

    fn float(n: f64) -> Expr {
        Expr::FloatLiteral(n)
    }

    fn string(s: &str) -> Expr {
        Expr::StringLiteral(s.to_owned())
    }

    fn bool_lit(b: bool) -> Expr {
        Expr::BoolLiteral(b)
    }

    fn ident(name: &str) -> Expr {
        Expr::Ident(ResolvedName {
            name: name.to_owned(),
            symbol: None,
        })
    }

    fn binary(left: Expr, op: BinaryOp, right: Expr) -> Expr {
        Expr::Binary {
            left: Box::new(left),
            op,
            right: Box::new(right),
        }
    }

    fn block(stmts: Vec<Stmt>) -> Expr {
        Expr::Block { scope: 0, stmts }
    }

    #[test]
    fn arithmetic_add() {
        let mut eval = Evaluator::new();
        let expr = binary(int(1), BinaryOp::Add, int(2));
        assert_eq!(eval.eval_expr(&expr).unwrap(), Value::Int(3));
    }

    #[test]
    fn arithmetic_sub() {
        let mut eval = Evaluator::new();
        let expr = binary(int(10), BinaryOp::Sub, int(3));
        assert_eq!(eval.eval_expr(&expr).unwrap(), Value::Int(7));
    }

    #[test]
    fn arithmetic_mul() {
        let mut eval = Evaluator::new();
        let expr = binary(int(4), BinaryOp::Mul, int(5));
        assert_eq!(eval.eval_expr(&expr).unwrap(), Value::Int(20));
    }

    #[test]
    fn arithmetic_div() {
        let mut eval = Evaluator::new();
        let expr = binary(int(10), BinaryOp::Div, int(2));
        assert_eq!(eval.eval_expr(&expr).unwrap(), Value::Int(5));
    }

    #[test]
    fn arithmetic_div_by_zero() {
        let mut eval = Evaluator::new();
        let expr = binary(int(10), BinaryOp::Div, int(0));
        assert!(matches!(
            eval.eval_expr(&expr),
            Err(EvalError::DivisionByZero)
        ));
    }

    #[test]
    fn string_concat() {
        let mut eval = Evaluator::new();
        let expr = binary(string("hello"), BinaryOp::Add, string(" world"));
        assert_eq!(
            eval.eval_expr(&expr).unwrap(),
            Value::String("hello world".to_owned())
        );
    }

    #[test]
    fn float_arithmetic() {
        let mut eval = Evaluator::new();
        let expr = binary(float(1.5), BinaryOp::Add, float(2.5));
        assert_eq!(eval.eval_expr(&expr).unwrap(), Value::Float(4.0));
    }

    #[test]
    fn int_float_coercion() {
        let mut eval = Evaluator::new();
        let expr = binary(int(1), BinaryOp::Add, float(2.5));
        assert_eq!(eval.eval_expr(&expr).unwrap(), Value::Float(3.5));
    }

    #[test]
    fn comparison_eq() {
        let mut eval = Evaluator::new();
        let expr = binary(int(3), BinaryOp::Eq, int(3));
        assert_eq!(eval.eval_expr(&expr).unwrap(), Value::Bool(true));
    }

    #[test]
    fn comparison_lt() {
        let mut eval = Evaluator::new();
        let expr = binary(int(2), BinaryOp::Lt, int(5));
        assert_eq!(eval.eval_expr(&expr).unwrap(), Value::Bool(true));
    }

    #[test]
    fn variable_binding_and_lookup() {
        let mut eval = Evaluator::new();
        let binding_stmt = Stmt::Binding(Binding {
            symbol: None,
            name: "x".to_owned(),
            is_pub: false,
            is_const: false,
            is_mut: true,
            is_sig: false,
            ty: None,
            value: Some(int(42)),
        });
        eval.eval_stmt(&binding_stmt).unwrap();
        let lookup = ident("x");
        assert_eq!(eval.eval_expr(&lookup).unwrap(), Value::Int(42));
    }

    #[test]
    fn if_then() {
        let mut eval = Evaluator::new();
        let stmt = Stmt::If(IfStmt {
            condition: bool_lit(true),
            then_scope: 0,
            then_body: int(1),
            else_scope: None,
            else_body: Some(int(2)),
        });
        let result = eval.eval_stmt(&stmt).unwrap();
        assert_eq!(result, Value::Int(1));
    }

    #[test]
    fn if_else() {
        let mut eval = Evaluator::new();
        let stmt = Stmt::If(IfStmt {
            condition: bool_lit(false),
            then_scope: 0,
            then_body: int(1),
            else_scope: None,
            else_body: Some(int(2)),
        });
        let result = eval.eval_stmt(&stmt).unwrap();
        assert_eq!(result, Value::Int(2));
    }

    #[test]
    fn for_loop_with_array() {
        let mut eval = Evaluator::new();
        // Bind `sum = 0`
        eval.eval_stmt(&Stmt::Binding(Binding {
            symbol: None,
            name: "sum".to_owned(),
            is_pub: false,
            is_const: false,
            is_mut: true,
            is_sig: false,
            ty: None,
            value: Some(int(0)),
        }))
        .unwrap();

        // for x of [1,2,3] { sum += x }
        let for_stmt = Stmt::For(ForStmt {
            scope: 0,
            binding: "x".to_owned(),
            binding_symbol: None,
            iterable: Expr::Array(vec![int(1), int(2), int(3)]),
            body: block(vec![Stmt::Expr(Expr::Assign {
                target: Box::new(ident("sum")),
                op: AssignOp::AddAssign,
                value: Box::new(ident("x")),
            })]),
        });
        eval.eval_stmt(&for_stmt).unwrap();

        assert_eq!(eval.env.get("sum").cloned().unwrap(), Value::Int(6));
    }

    #[test]
    fn when_pattern_matching_int() {
        let mut eval = Evaluator::new();
        let expr = Expr::When {
            subject: Box::new(int(2)),
            arms: vec![
                WhenArm {
                    scope: 0,
                    pattern: Pattern::IntLiteral(1),
                    guard: None,
                    body: string("one"),
                },
                WhenArm {
                    scope: 0,
                    pattern: Pattern::IntLiteral(2),
                    guard: None,
                    body: string("two"),
                },
                WhenArm {
                    scope: 0,
                    pattern: Pattern::Wildcard,
                    guard: None,
                    body: string("other"),
                },
            ],
        };
        assert_eq!(
            eval.eval_expr(&expr).unwrap(),
            Value::String("two".to_owned())
        );
    }

    #[test]
    fn when_wildcard_fallthrough() {
        let mut eval = Evaluator::new();
        let expr = Expr::When {
            subject: Box::new(int(99)),
            arms: vec![
                WhenArm {
                    scope: 0,
                    pattern: Pattern::IntLiteral(1),
                    guard: None,
                    body: string("one"),
                },
                WhenArm {
                    scope: 0,
                    pattern: Pattern::Wildcard,
                    guard: None,
                    body: string("other"),
                },
            ],
        };
        assert_eq!(
            eval.eval_expr(&expr).unwrap(),
            Value::String("other".to_owned())
        );
    }

    #[test]
    fn function_call() {
        let mut eval = Evaluator::new();
        // Define: function add(a, b) -> a + b
        let func = Value::Function {
            params: vec!["a".to_owned(), "b".to_owned()],
            body: Box::new(binary(ident("a"), BinaryOp::Add, ident("b"))),
            env: Env::new(),
        };
        eval.env.set("add".to_owned(), func);

        let call_expr = Expr::Call {
            callee: Box::new(ident("add")),
            args: vec![
                CallArg {
                    name: None,
                    value: int(3),
                },
                CallArg {
                    name: None,
                    value: int(4),
                },
            ],
        };
        assert_eq!(eval.eval_expr(&call_expr).unwrap(), Value::Int(7));
    }

    #[test]
    fn closure_capture() {
        let mut eval = Evaluator::new();
        // let offset = 10; let adder = (x) -> x + offset; adder(5) == 15
        eval.env.set("offset".to_owned(), Value::Int(10));

        let closure_expr = Expr::Closure {
            params: vec![Param {
                symbol: None,
                name: "x".to_owned(),
                ty: None,
                default: None,
            }],
            body: Box::new(binary(ident("x"), BinaryOp::Add, ident("offset"))),
        };
        let closure_val = eval.eval_expr(&closure_expr).unwrap();
        eval.env.set("adder".to_owned(), closure_val);

        let call_expr = Expr::Call {
            callee: Box::new(ident("adder")),
            args: vec![CallArg {
                name: None,
                value: int(5),
            }],
        };
        assert_eq!(eval.eval_expr(&call_expr).unwrap(), Value::Int(15));
    }

    #[test]
    fn null_coalesce_void() {
        let mut eval = Evaluator::new();
        let expr = binary(Expr::Void, BinaryOp::NullCoalesce, int(42));
        assert_eq!(eval.eval_expr(&expr).unwrap(), Value::Int(42));
    }

    #[test]
    fn null_coalesce_non_void() {
        let mut eval = Evaluator::new();
        let expr = binary(int(7), BinaryOp::NullCoalesce, int(42));
        assert_eq!(eval.eval_expr(&expr).unwrap(), Value::Int(7));
    }

    #[test]
    fn range_exclusive() {
        let mut eval = Evaluator::new();
        let expr = binary(int(1), BinaryOp::Range, int(4));
        assert_eq!(
            eval.eval_expr(&expr).unwrap(),
            Value::Array(vec![Value::Int(1), Value::Int(2), Value::Int(3)])
        );
    }

    #[test]
    fn range_inclusive() {
        let mut eval = Evaluator::new();
        let expr = binary(int(1), BinaryOp::RangeInclusive, int(3));
        assert_eq!(
            eval.eval_expr(&expr).unwrap(),
            Value::Array(vec![Value::Int(1), Value::Int(2), Value::Int(3)])
        );
    }

    #[test]
    fn logical_and_short_circuit() {
        let mut eval = Evaluator::new();
        let expr = binary(bool_lit(false), BinaryOp::And, bool_lit(true));
        assert_eq!(eval.eval_expr(&expr).unwrap(), Value::Bool(false));
    }

    #[test]
    fn logical_or_short_circuit() {
        let mut eval = Evaluator::new();
        let expr = binary(bool_lit(true), BinaryOp::Or, bool_lit(false));
        assert_eq!(eval.eval_expr(&expr).unwrap(), Value::Bool(true));
    }

    #[test]
    fn unary_neg() {
        let mut eval = Evaluator::new();
        let expr = Expr::Unary {
            op: UnaryOp::Neg,
            operand: Box::new(int(5)),
        };
        assert_eq!(eval.eval_expr(&expr).unwrap(), Value::Int(-5));
    }

    #[test]
    fn unary_not() {
        let mut eval = Evaluator::new();
        let expr = Expr::Unary {
            op: UnaryOp::Not,
            operand: Box::new(bool_lit(true)),
        };
        assert_eq!(eval.eval_expr(&expr).unwrap(), Value::Bool(false));
    }

    #[test]
    fn string_interp() {
        let mut eval = Evaluator::new();
        eval.env
            .set("name".to_owned(), Value::String("world".to_owned()));
        let expr = Expr::StringInterp(vec![
            StringPart::Lit("Hello, ".to_owned()),
            StringPart::Expr(ident("name")),
            StringPart::Lit("!".to_owned()),
        ]);
        assert_eq!(
            eval.eval_expr(&expr).unwrap(),
            Value::String("Hello, world!".to_owned())
        );
    }

    #[test]
    fn array_method_len() {
        let mut eval = Evaluator::new();
        let expr = Expr::Call {
            callee: Box::new(Expr::Field {
                object: Box::new(Expr::Array(vec![int(1), int(2), int(3)])),
                field: "len".to_owned(),
            }),
            args: vec![],
        };
        assert_eq!(eval.eval_expr(&expr).unwrap(), Value::Int(3));
    }

    #[test]
    fn array_method_map() {
        let mut eval = Evaluator::new();
        let double = Value::Function {
            params: vec!["x".to_owned()],
            body: Box::new(binary(ident("x"), BinaryOp::Mul, int(2))),
            env: Env::new(),
        };
        eval.env.set("double".to_owned(), double);

        let expr = Expr::Call {
            callee: Box::new(Expr::Field {
                object: Box::new(Expr::Array(vec![int(1), int(2), int(3)])),
                field: "map".to_owned(),
            }),
            args: vec![CallArg {
                name: None,
                value: ident("double"),
            }],
        };
        assert_eq!(
            eval.eval_expr(&expr).unwrap(),
            Value::Array(vec![Value::Int(2), Value::Int(4), Value::Int(6)])
        );
    }

    #[test]
    fn while_loop() {
        let mut eval = Evaluator::new();
        eval.eval_stmt(&Stmt::Binding(Binding {
            symbol: None,
            name: "i".to_owned(),
            is_pub: false,
            is_const: false,
            is_mut: true,
            is_sig: false,
            ty: None,
            value: Some(int(0)),
        }))
        .unwrap();

        let while_stmt = Stmt::While(WhileStmt {
            scope: 0,
            condition: binary(ident("i"), BinaryOp::Lt, int(3)),
            body: block(vec![Stmt::Expr(Expr::Assign {
                target: Box::new(ident("i")),
                op: AssignOp::AddAssign,
                value: Box::new(int(1)),
            })]),
        });
        eval.eval_stmt(&while_stmt).unwrap();
        assert_eq!(eval.env.get("i").cloned().unwrap(), Value::Int(3));
    }

    #[test]
    fn object_literal_and_field_access() {
        let mut eval = Evaluator::new();
        let obj_expr = Expr::Object(vec![
            ObjectField {
                key: "x".to_owned(),
                value: int(10),
            },
            ObjectField {
                key: "y".to_owned(),
                value: int(20),
            },
        ]);
        eval.eval_stmt(&Stmt::Binding(Binding {
            symbol: None,
            name: "pt".to_owned(),
            is_pub: false,
            is_const: false,
            is_mut: false,
            is_sig: false,
            ty: None,
            value: Some(obj_expr),
        }))
        .unwrap();

        let field_expr = Expr::Field {
            object: Box::new(ident("pt")),
            field: "x".to_owned(),
        };
        assert_eq!(eval.eval_expr(&field_expr).unwrap(), Value::Int(10));
    }
}
