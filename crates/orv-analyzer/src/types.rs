use std::collections::{BTreeMap, HashMap, HashSet};

use orv_diagnostics::{Diagnostic, DiagnosticBag, Label};
use orv_span::Spanned;
use orv_syntax::ast::{
    AssignOp, BindingStmt, Expr, FunctionItem, Item, Module, NodeExpr, Stmt, StructItem, TypeExpr,
};

pub fn type_check(module: &Module) -> DiagnosticBag {
    let registry = Registry::new(module);
    let mut checker = TypeChecker::new(registry);
    checker.check_module(module);
    checker.diagnostics
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Ty {
    Unknown,
    Error,
    Void,
    Bool,
    String,
    Int(String),
    Float(String),
    Named(String),
    Struct(String),
    Nullable(Box<Ty>),
    Vec(Box<Ty>),
    HashMap(Box<Ty>, Box<Ty>),
    Function { params: Vec<Ty>, ret: Box<Ty> },
    Node(String),
    Object(BTreeMap<String, Ty>),
}

impl Ty {
    fn display(&self) -> String {
        match self {
            Self::Unknown => "<unknown>".to_owned(),
            Self::Error => "<error>".to_owned(),
            Self::Void => "void".to_owned(),
            Self::Bool => "bool".to_owned(),
            Self::String => "string".to_owned(),
            Self::Int(name) | Self::Float(name) | Self::Named(name) | Self::Struct(name) => {
                name.clone()
            }
            Self::Nullable(inner) => format!("{}?", inner.display()),
            Self::Vec(inner) => format!("Vec<{}>", inner.display()),
            Self::HashMap(key, value) => {
                format!("HashMap<{}, {}>", key.display(), value.display())
            }
            Self::Function { params, ret } => format!(
                "({}) -> {}",
                params
                    .iter()
                    .map(Self::display)
                    .collect::<Vec<_>>()
                    .join(", "),
                ret.display()
            ),
            Self::Node(name) => format!("@{name}"),
            Self::Object(fields) => format!(
                "{{ {} }}",
                fields
                    .iter()
                    .map(|(key, value)| format!("{key}: {}", value.display()))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        }
    }

    fn is_numeric(&self) -> bool {
        matches!(self, Self::Int(_) | Self::Float(_))
    }
}

#[derive(Debug, Clone)]
struct FunctionSig {
    params: Vec<Ty>,
    ret: Ty,
}

#[derive(Debug, Clone)]
struct StructSpec {
    fields: BTreeMap<String, Ty>,
}

struct Registry<'a> {
    structs: HashMap<String, &'a StructItem>,
    aliases: HashMap<String, &'a Spanned<TypeExpr>>,
    functions: HashMap<String, FunctionSig>,
}

impl<'a> Registry<'a> {
    fn new(module: &'a Module) -> Self {
        let mut structs = HashMap::new();
        let mut aliases = HashMap::new();
        for item in &module.items {
            match item.node() {
                Item::Struct(struct_item) => {
                    structs.insert(struct_item.name.node().clone(), struct_item);
                }
                Item::TypeAlias(alias) => {
                    aliases.insert(alias.name.node().clone(), &alias.ty);
                }
                _ => {}
            }
        }

        let mut registry = Self {
            structs,
            aliases,
            functions: HashMap::new(),
        };
        registry.collect_functions(module);
        registry
    }

    fn collect_functions(&mut self, module: &Module) {
        for item in &module.items {
            if let Item::Function(function) = item.node() {
                let params = function
                    .params
                    .iter()
                    .map(|param| {
                        param
                            .node()
                            .ty
                            .as_ref()
                            .map_or(Ty::Unknown, |ty| self.resolve_type_expr(ty.node()))
                    })
                    .collect();
                let ret = function
                    .return_type
                    .as_ref()
                    .map_or(Ty::Unknown, |ty| self.resolve_type_expr(ty.node()));
                self.functions
                    .insert(function.name.node().clone(), FunctionSig { params, ret });
            }
        }
    }

    fn resolve_type_expr(&self, expr: &TypeExpr) -> Ty {
        self.resolve_type_expr_inner(expr, &mut HashSet::new())
    }

    fn resolve_type_expr_inner(&self, expr: &TypeExpr, visiting: &mut HashSet<String>) -> Ty {
        match expr {
            TypeExpr::Named(name) => self.resolve_named_type(name, visiting),
            TypeExpr::Nullable(inner) => Ty::Nullable(Box::new(
                self.resolve_type_expr_inner(inner.node(), visiting),
            )),
            TypeExpr::Generic { name, args } if name.node() == "Vec" && args.len() == 1 => Ty::Vec(
                Box::new(self.resolve_type_expr_inner(args[0].node(), visiting)),
            ),
            TypeExpr::Generic { name, args } if name.node() == "HashMap" && args.len() == 2 => {
                Ty::HashMap(
                    Box::new(self.resolve_type_expr_inner(args[0].node(), visiting)),
                    Box::new(self.resolve_type_expr_inner(args[1].node(), visiting)),
                )
            }
            TypeExpr::Generic { name, args } => Ty::Named(format!(
                "{}<{}>",
                name.node(),
                args.iter()
                    .map(|arg| self.resolve_type_expr_inner(arg.node(), visiting).display())
                    .collect::<Vec<_>>()
                    .join(", ")
            )),
            TypeExpr::Function { params, ret } => Ty::Function {
                params: params
                    .iter()
                    .map(|param| self.resolve_type_expr_inner(param.node(), visiting))
                    .collect(),
                ret: Box::new(self.resolve_type_expr_inner(ret.node(), visiting)),
            },
            TypeExpr::Node(name) => Ty::Node(name.node().to_string()),
            TypeExpr::Error => Ty::Error,
        }
    }

    fn resolve_named_type(&self, name: &str, visiting: &mut HashSet<String>) -> Ty {
        if name == "string" {
            return Ty::String;
        }
        if name == "bool" {
            return Ty::Bool;
        }
        if name == "void" {
            return Ty::Void;
        }
        if is_int_type(name) {
            return Ty::Int(name.to_owned());
        }
        if is_float_type(name) {
            return Ty::Float(name.to_owned());
        }
        if self.structs.contains_key(name) {
            return Ty::Struct(name.to_owned());
        }
        if let Some(alias) = self.aliases.get(name) {
            if !visiting.insert(name.to_owned()) {
                return Ty::Error;
            }
            let ty = self.resolve_type_expr_inner(alias.node(), visiting);
            visiting.remove(name);
            return ty;
        }
        Ty::Named(name.to_owned())
    }

    fn struct_spec(&self, name: &str) -> Option<StructSpec> {
        let struct_item = self.structs.get(name)?;
        let fields = struct_item
            .fields
            .iter()
            .map(|field| {
                (
                    field.node().name.node().clone(),
                    self.resolve_type_expr(field.node().ty.node()),
                )
            })
            .collect();
        Some(StructSpec { fields })
    }

    fn function_sig(&self, name: &str) -> Option<&FunctionSig> {
        self.functions.get(name)
    }
}

struct TypeChecker<'a> {
    registry: Registry<'a>,
    diagnostics: DiagnosticBag,
    scopes: Vec<HashMap<String, Ty>>,
}

impl<'a> TypeChecker<'a> {
    fn new(registry: Registry<'a>) -> Self {
        Self {
            registry,
            diagnostics: DiagnosticBag::new(),
            scopes: vec![HashMap::new()],
        }
    }

    fn check_module(&mut self, module: &Module) {
        self.predeclare_top_level_bindings(module);
        for item in &module.items {
            self.check_item(item.node());
        }
    }

    fn predeclare_top_level_bindings(&mut self, module: &Module) {
        for item in &module.items {
            match item.node() {
                Item::Binding(binding) => {
                    let ty = binding
                        .ty
                        .as_ref()
                        .map_or(Ty::Unknown, |ty| self.registry.resolve_type_expr(ty.node()));
                    self.bind(binding.name.node().clone(), ty);
                }
                Item::Function(function) => {
                    if let Some(sig) = self.registry.function_sig(function.name.node()) {
                        self.bind(
                            function.name.node().clone(),
                            Ty::Function {
                                params: sig.params.clone(),
                                ret: Box::new(sig.ret.clone()),
                            },
                        );
                    }
                }
                _ => {}
            }
        }
    }

    fn check_item(&mut self, item: &Item) {
        match item {
            Item::Function(function) => self.check_function(function),
            Item::Define(define) => self.check_define(define),
            Item::Binding(binding) => self.check_binding(binding),
            Item::Stmt(stmt) => {
                self.check_stmt(stmt, None);
            }
            Item::Import(_)
            | Item::Struct(_)
            | Item::Enum(_)
            | Item::TypeAlias(_)
            | Item::Error => {}
        }
    }

    fn check_function(&mut self, function: &FunctionItem) {
        let declared_return = function
            .return_type
            .as_ref()
            .map(|ty| self.registry.resolve_type_expr(ty.node()));
        self.push_scope();
        for param in &function.params {
            let param_ty = param
                .node()
                .ty
                .as_ref()
                .map_or(Ty::Unknown, |ty| self.registry.resolve_type_expr(ty.node()));
            self.bind(param.node().name.node().clone(), param_ty.clone());
            if let Some(default) = &param.node().default {
                let default_ty = self.infer_expr(default, Some(&param_ty));
                self.expect_assignable(default.span(), &param_ty, &default_ty);
            }
        }

        let body_ty = self.infer_expr(&function.body, declared_return.as_ref());
        if let Some(expected) = declared_return {
            self.expect_assignable(function.body.span(), &expected, &body_ty);
        }
        self.pop_scope();
    }

    fn check_define(&mut self, define: &orv_syntax::ast::DefineItem) {
        self.push_scope();
        for param in &define.params {
            let param_ty = param
                .node()
                .ty
                .as_ref()
                .map_or(Ty::Unknown, |ty| self.registry.resolve_type_expr(ty.node()));
            self.bind(param.node().name.node().clone(), param_ty.clone());
            if let Some(default) = &param.node().default {
                let default_ty = self.infer_expr(default, Some(&param_ty));
                self.expect_assignable(default.span(), &param_ty, &default_ty);
            }
        }
        let _ = self.infer_expr(&define.body, None);
        self.pop_scope();
    }

    fn check_binding(&mut self, binding: &BindingStmt) {
        let declared = binding
            .ty
            .as_ref()
            .map(|ty| self.registry.resolve_type_expr(ty.node()));
        let inferred = binding
            .value
            .as_ref()
            .map(|value| self.infer_expr(value, declared.as_ref()))
            .unwrap_or(Ty::Unknown);
        if let Some(expected) = &declared {
            if let Some(value) = &binding.value {
                self.expect_assignable(value.span(), expected, &inferred);
            }
        }
        self.bind(binding.name.node().clone(), declared.unwrap_or(inferred));
    }

    fn check_stmt(&mut self, stmt: &Stmt, expected: Option<&Ty>) -> Ty {
        match stmt {
            Stmt::Binding(binding) => {
                self.check_binding(binding);
                Ty::Void
            }
            Stmt::Return(expr) => expr
                .as_ref()
                .map_or(Ty::Void, |expr| self.infer_expr(expr, expected)),
            Stmt::If(if_stmt) => {
                let condition_ty = self.infer_expr(&if_stmt.condition, Some(&Ty::Bool));
                self.expect_assignable(if_stmt.condition.span(), &Ty::Bool, &condition_ty);

                self.push_scope();
                let then_ty = self.infer_expr(&if_stmt.then_body, expected);
                self.pop_scope();

                let else_ty = if let Some(else_body) = &if_stmt.else_body {
                    self.push_scope();
                    let ty = self.infer_expr(else_body, expected);
                    self.pop_scope();
                    ty
                } else {
                    Ty::Void
                };

                if same_type(&then_ty, &else_ty) {
                    then_ty
                } else {
                    Ty::Unknown
                }
            }
            Stmt::For(for_stmt) => {
                let iterable_ty = self.infer_expr(&for_stmt.iterable, None);
                let element_ty = match iterable_ty {
                    Ty::Vec(inner) => *inner,
                    _ => Ty::Unknown,
                };
                self.push_scope();
                self.bind(for_stmt.binding.node().clone(), element_ty);
                let _ = self.infer_expr(&for_stmt.body, None);
                self.pop_scope();
                Ty::Void
            }
            Stmt::While(while_stmt) => {
                let condition_ty = self.infer_expr(&while_stmt.condition, Some(&Ty::Bool));
                self.expect_assignable(while_stmt.condition.span(), &Ty::Bool, &condition_ty);
                self.push_scope();
                let _ = self.infer_expr(&while_stmt.body, None);
                self.pop_scope();
                Ty::Void
            }
            Stmt::Expr(expr) => self.infer_expr(expr, expected),
            Stmt::Error => Ty::Error,
        }
    }

    fn infer_expr(&mut self, expr: &Spanned<Expr>, expected: Option<&Ty>) -> Ty {
        match expr.node() {
            Expr::IntLiteral(_) => contextual_int(expected),
            Expr::FloatLiteral(_) => contextual_float(expected),
            Expr::StringLiteral(_) => Ty::String,
            Expr::StringInterp(parts) => {
                for part in parts {
                    if let orv_syntax::ast::StringPart::Expr(expr) = part {
                        let _ = self.infer_expr(expr, None);
                    }
                }
                Ty::String
            }
            Expr::BoolLiteral(_) => Ty::Bool,
            Expr::Void => Ty::Void,
            Expr::Ident(name) => self.lookup(name).unwrap_or_else(|| {
                self.registry
                    .function_sig(name)
                    .map(|sig| Ty::Function {
                        params: sig.params.clone(),
                        ret: Box::new(sig.ret.clone()),
                    })
                    .unwrap_or(Ty::Unknown)
            }),
            Expr::Binary { left, op, right } => {
                let left_ty = self.infer_expr(left, expected);
                let right_ty = self.infer_expr(right, Some(&left_ty));
                match op.node() {
                    orv_syntax::ast::BinOp::Add
                    | orv_syntax::ast::BinOp::Sub
                    | orv_syntax::ast::BinOp::Mul
                    | orv_syntax::ast::BinOp::Div => {
                        if left_ty.is_numeric() && right_ty.is_numeric() {
                            if same_type(&left_ty, &right_ty) || matches!(right_ty, Ty::Unknown) {
                                left_ty
                            } else {
                                self.type_mismatch(expr.span(), &left_ty, &right_ty);
                                Ty::Error
                            }
                        } else {
                            self.emit_type_error(
                                expr.span(),
                                "arithmetic operators require numeric operands",
                            );
                            Ty::Error
                        }
                    }
                    orv_syntax::ast::BinOp::Eq
                    | orv_syntax::ast::BinOp::NotEq
                    | orv_syntax::ast::BinOp::Lt
                    | orv_syntax::ast::BinOp::LtEq
                    | orv_syntax::ast::BinOp::Gt
                    | orv_syntax::ast::BinOp::GtEq => {
                        if !same_type(&left_ty, &right_ty) && !matches!(right_ty, Ty::Unknown) {
                            self.type_mismatch(expr.span(), &left_ty, &right_ty);
                        }
                        Ty::Bool
                    }
                    orv_syntax::ast::BinOp::And | orv_syntax::ast::BinOp::Or => {
                        self.expect_assignable(left.span(), &Ty::Bool, &left_ty);
                        self.expect_assignable(right.span(), &Ty::Bool, &right_ty);
                        Ty::Bool
                    }
                    orv_syntax::ast::BinOp::Pipe => right_ty,
                }
            }
            Expr::Unary { op, operand } => {
                let operand_ty = self.infer_expr(operand, expected);
                match op.node() {
                    orv_syntax::ast::UnaryOp::Neg => {
                        if operand_ty.is_numeric() {
                            operand_ty
                        } else {
                            self.emit_type_error(
                                expr.span(),
                                "negation requires a numeric operand",
                            );
                            Ty::Error
                        }
                    }
                    orv_syntax::ast::UnaryOp::Not => {
                        self.expect_assignable(operand.span(), &Ty::Bool, &operand_ty);
                        Ty::Bool
                    }
                }
            }
            Expr::Assign { target, op, value } => {
                let target_ty = self.infer_expr(target, None);
                let value_ty = self.infer_expr(value, Some(&target_ty));
                match op.node() {
                    AssignOp::Assign => self.expect_assignable(value.span(), &target_ty, &value_ty),
                    AssignOp::AddAssign | AssignOp::SubAssign => {
                        if !target_ty.is_numeric() {
                            self.emit_type_error(
                                target.span(),
                                "compound assignment requires a numeric target",
                            );
                        }
                        self.expect_assignable(value.span(), &target_ty, &value_ty);
                    }
                }
                target_ty
            }
            Expr::Call { callee, args } => self.infer_call(expr, callee, args),
            Expr::Field { object, field } => {
                let object_ty = self.infer_expr(object, None);
                self.field_type(expr.span(), &object_ty, field.node())
            }
            Expr::Index { object, index } => {
                let object_ty = self.infer_expr(object, None);
                let index_ty = self.infer_expr(index, None);
                match object_ty {
                    Ty::Vec(inner) => {
                        if !matches!(index_ty, Ty::Int(_)) {
                            self.emit_type_error(index.span(), "vector indices must be integers");
                        }
                        *inner
                    }
                    Ty::HashMap(key, value) => {
                        self.expect_assignable(index.span(), &key, &index_ty);
                        *value
                    }
                    _ => Ty::Unknown,
                }
            }
            Expr::Block(stmts) => {
                self.push_scope();
                let mut tail = Ty::Void;
                for stmt in stmts {
                    tail = self.check_stmt(stmt.node(), expected);
                }
                self.pop_scope();
                tail
            }
            Expr::Object(fields) => self.infer_object(expr.span(), fields, expected),
            Expr::Map(fields) => self.infer_map(expr.span(), fields, expected),
            Expr::Array(items) => self.infer_array(expr.span(), items, expected),
            Expr::Node(node) => self.infer_node(node, expr.span(), expected),
            Expr::Paren(inner) => self.infer_expr(inner, expected),
            Expr::Await(inner) => self.infer_expr(inner, expected),
            Expr::Error => Ty::Error,
        }
    }

    fn infer_call(
        &mut self,
        expr: &Spanned<Expr>,
        callee: &Spanned<Expr>,
        args: &[Spanned<orv_syntax::ast::CallArg>],
    ) -> Ty {
        if let Expr::Ident(name) = callee.node()
            && let Some(sig) = self.registry.function_sig(name).cloned()
        {
            if args.len() != sig.params.len() {
                self.emit_type_error(
                    expr.span(),
                    format!(
                        "function `{name}` expects {} argument(s), got {}",
                        sig.params.len(),
                        args.len()
                    ),
                );
            }
            for (arg, expected_ty) in args.iter().zip(&sig.params) {
                let actual = self.infer_expr(&arg.node().value, Some(expected_ty));
                self.expect_assignable(arg.node().value.span(), expected_ty, &actual);
            }
            return sig.ret.clone();
        }

        if let Expr::Field { object, field } = callee.node() {
            let object_ty = self.infer_expr(object, None);
            if field.node() == "len" && args.is_empty() {
                if matches!(
                    object_ty,
                    Ty::Vec(_) | Ty::String | Ty::Object(_) | Ty::HashMap(_, _)
                ) {
                    return Ty::Int("i32".to_owned());
                }
            }
        }

        let callee_ty = self.infer_expr(callee, None);
        match callee_ty {
            Ty::Function { params, ret } => {
                for (arg, expected_ty) in args.iter().zip(&params) {
                    let actual = self.infer_expr(&arg.node().value, Some(expected_ty));
                    self.expect_assignable(arg.node().value.span(), expected_ty, &actual);
                }
                *ret
            }
            Ty::Unknown => Ty::Unknown,
            _ => {
                self.emit_type_error(expr.span(), "attempted to call a non-callable value");
                Ty::Error
            }
        }
    }

    fn infer_object(
        &mut self,
        span: orv_span::Span,
        fields: &[Spanned<orv_syntax::ast::ObjectField>],
        expected: Option<&Ty>,
    ) -> Ty {
        if let Some(Ty::Struct(name)) = expected {
            return self.check_struct_object(span, fields, name);
        }

        let mut object_fields = BTreeMap::new();
        for field in fields {
            let value_ty = self.infer_expr(&field.node().value, None);
            object_fields.insert(field.node().key.node().clone(), value_ty);
        }
        Ty::Object(object_fields)
    }

    fn infer_map(
        &mut self,
        span: orv_span::Span,
        fields: &[Spanned<orv_syntax::ast::ObjectField>],
        expected: Option<&Ty>,
    ) -> Ty {
        let expected_value = match expected {
            Some(Ty::HashMap(key, value)) => {
                if !matches!(key.as_ref(), Ty::String) {
                    self.emit_type_error(
                        span,
                        format!(
                            "map literals currently require `string` keys, found `{}`",
                            key.display()
                        ),
                    );
                }
                Some(value.as_ref())
            }
            _ => None,
        };

        if fields.is_empty() {
            if let Some(value) = expected_value {
                return Ty::HashMap(Box::new(Ty::String), Box::new(value.clone()));
            }
            self.emit_type_error(span, "cannot infer the value type of an empty map literal");
            return Ty::HashMap(Box::new(Ty::String), Box::new(Ty::Unknown));
        }

        let first_ty = self.infer_expr(&fields[0].node().value, expected_value);
        for field in &fields[1..] {
            let actual_ty = self.infer_expr(&field.node().value, Some(&first_ty));
            if !same_type(&first_ty, &actual_ty) && !matches!(actual_ty, Ty::Unknown) {
                self.type_mismatch(field.node().value.span(), &first_ty, &actual_ty);
            }
        }

        Ty::HashMap(Box::new(Ty::String), Box::new(first_ty))
    }

    fn check_struct_object(
        &mut self,
        span: orv_span::Span,
        fields: &[Spanned<orv_syntax::ast::ObjectField>],
        struct_name: &str,
    ) -> Ty {
        let Some(spec) = self.registry.struct_spec(struct_name) else {
            return Ty::Struct(struct_name.to_owned());
        };

        let mut seen = HashSet::new();
        for field in fields {
            let key = field.node().key.node().clone();
            seen.insert(key.clone());
            if let Some(expected_ty) = spec.fields.get(&key) {
                let actual_ty = self.infer_expr(&field.node().value, Some(expected_ty));
                self.expect_assignable(field.node().value.span(), expected_ty, &actual_ty);
            } else {
                self.emit_type_error(
                    field.span(),
                    format!("extra field `{key}` in `{struct_name}` object literal"),
                );
            }
        }

        for field_name in spec.fields.keys() {
            if !seen.contains(field_name) {
                self.emit_type_error(
                    span,
                    format!("missing field `{field_name}` for `{struct_name}`"),
                );
            }
        }

        Ty::Struct(struct_name.to_owned())
    }

    fn infer_array(
        &mut self,
        span: orv_span::Span,
        items: &[Spanned<Expr>],
        expected: Option<&Ty>,
    ) -> Ty {
        let expected_elem = match expected {
            Some(Ty::Vec(inner)) => Some(inner.as_ref()),
            _ => None,
        };

        if items.is_empty() {
            if let Some(inner) = expected_elem {
                return Ty::Vec(Box::new(inner.clone()));
            }
            self.emit_type_error(
                span,
                "cannot infer the element type of an empty array literal",
            );
            return Ty::Vec(Box::new(Ty::Unknown));
        }

        let first_ty = self.infer_expr(&items[0], expected_elem);
        for item in &items[1..] {
            let item_ty = self.infer_expr(item, Some(&first_ty));
            if !same_type(&first_ty, &item_ty) && !matches!(item_ty, Ty::Unknown) {
                self.type_mismatch(item.span(), &first_ty, &item_ty);
            }
        }
        Ty::Vec(Box::new(first_ty))
    }

    fn infer_node(&mut self, node: &NodeExpr, span: orv_span::Span, expected: Option<&Ty>) -> Ty {
        let name = node.name.node().to_string();
        if name == "env" {
            if let Some(value) = node.positional.first() {
                let _ = self.infer_expr(value, None);
            }
            return expected.cloned().unwrap_or(Ty::String);
        }

        for positional in &node.positional {
            let _ = self.infer_expr(positional, None);
        }
        for property in &node.properties {
            let _ = self.infer_expr(&property.node().value, None);
        }
        if let Some(body) = &node.body {
            let _ = self.infer_expr(body, None);
        }

        if name == "response" {
            return Ty::Node("response".to_owned());
        }
        if name == "serve" {
            return Ty::Node("serve".to_owned());
        }
        if name == "html" || matches!(expected, Some(Ty::Node(_))) {
            return Ty::Node(name);
        }

        if name.is_empty() {
            self.emit_type_error(span, "invalid node expression");
            return Ty::Error;
        }

        Ty::Node(name)
    }

    fn field_type(&mut self, span: orv_span::Span, object_ty: &Ty, field: &str) -> Ty {
        match object_ty {
            Ty::Struct(name) => self
                .registry
                .struct_spec(name)
                .and_then(|spec| spec.fields.get(field).cloned())
                .unwrap_or_else(|| {
                    self.emit_type_error(span, format!("unknown field `{field}` on `{name}`"));
                    Ty::Unknown
                }),
            Ty::Object(fields) => fields.get(field).cloned().unwrap_or(Ty::Unknown),
            Ty::Vec(_) | Ty::String if field == "len" => Ty::Function {
                params: Vec::new(),
                ret: Box::new(Ty::Int("i32".to_owned())),
            },
            Ty::HashMap(key, _) if field == "keys" => Ty::Function {
                params: Vec::new(),
                ret: Box::new(Ty::Vec(Box::new(key.as_ref().clone()))),
            },
            Ty::HashMap(_, value) if field == "values" => Ty::Function {
                params: Vec::new(),
                ret: Box::new(Ty::Vec(Box::new(value.as_ref().clone()))),
            },
            Ty::HashMap(_, _) if field == "len" => Ty::Function {
                params: Vec::new(),
                ret: Box::new(Ty::Int("i32".to_owned())),
            },
            _ => Ty::Unknown,
        }
    }

    fn bind(&mut self, name: String, ty: Ty) {
        self.scopes
            .last_mut()
            .expect("type checker scope stack must be non-empty")
            .insert(name, ty);
    }

    fn lookup(&self, name: &str) -> Option<Ty> {
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| scope.get(name).cloned())
    }

    fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    fn expect_assignable(&mut self, span: orv_span::Span, expected: &Ty, actual: &Ty) {
        if !is_assignable(expected, actual) {
            self.type_mismatch(span, expected, actual);
        }
    }

    fn type_mismatch(&mut self, span: orv_span::Span, expected: &Ty, actual: &Ty) {
        self.diagnostics.push(
            Diagnostic::error(format!(
                "type mismatch: expected `{}`, found `{}`",
                expected.display(),
                actual.display()
            ))
            .with_label(Label::primary(span, "incompatible type here")),
        );
    }

    fn emit_type_error(&mut self, span: orv_span::Span, message: impl Into<String>) {
        self.diagnostics
            .push(Diagnostic::error(message.into()).with_label(Label::primary(span, "here")));
    }
}

fn contextual_int(expected: Option<&Ty>) -> Ty {
    match expected {
        Some(Ty::Int(name)) => Ty::Int(name.clone()),
        _ => Ty::Int("i64".to_owned()),
    }
}

fn contextual_float(expected: Option<&Ty>) -> Ty {
    match expected {
        Some(Ty::Float(name)) => Ty::Float(name.clone()),
        _ => Ty::Float("f64".to_owned()),
    }
}

fn is_assignable(expected: &Ty, actual: &Ty) -> bool {
    match (expected, actual) {
        (_, Ty::Unknown | Ty::Error) | (Ty::Unknown | Ty::Error, _) => true,
        (Ty::Void, Ty::Void) | (Ty::Bool, Ty::Bool) | (Ty::String, Ty::String) => true,
        (Ty::Int(a), Ty::Int(b)) | (Ty::Float(a), Ty::Float(b)) => a == b,
        (Ty::Named(a), Ty::Named(b))
        | (Ty::Struct(a), Ty::Struct(b))
        | (Ty::Node(a), Ty::Node(b)) => a == b,
        (Ty::Nullable(inner), Ty::Void) => !matches!(**inner, Ty::Void),
        (Ty::Nullable(inner), other) => is_assignable(inner, other),
        (Ty::Vec(a), Ty::Vec(b)) => is_assignable(a, b),
        (Ty::HashMap(ka, va), Ty::HashMap(kb, vb)) => {
            is_assignable(ka, kb) && is_assignable(va, vb)
        }
        (
            Ty::Function {
                params: expected_params,
                ret: expected_ret,
            },
            Ty::Function {
                params: actual_params,
                ret: actual_ret,
            },
        ) => {
            expected_params.len() == actual_params.len()
                && expected_params
                    .iter()
                    .zip(actual_params)
                    .all(|(expected, actual)| is_assignable(expected, actual))
                && is_assignable(expected_ret, actual_ret)
        }
        (Ty::Object(expected_fields), Ty::Object(actual_fields)) => {
            expected_fields.len() == actual_fields.len()
                && expected_fields.iter().all(|(key, expected)| {
                    actual_fields
                        .get(key)
                        .is_some_and(|actual| is_assignable(expected, actual))
                })
        }
        _ => false,
    }
}

fn same_type(left: &Ty, right: &Ty) -> bool {
    left == right || matches!(left, Ty::Unknown) || matches!(right, Ty::Unknown)
}

fn is_int_type(name: &str) -> bool {
    matches!(
        name,
        "u8" | "u16" | "u32" | "u64" | "usize" | "i8" | "i16" | "i32" | "i64" | "isize"
    )
}

fn is_float_type(name: &str) -> bool {
    matches!(name, "f32" | "f64")
}
