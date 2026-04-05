# Phase 2: Parser and AST — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Parse the orv language surface into a recoverable AST. The parser consumes the token stream produced by the Phase 1 lexer and produces a typed AST annotated with source spans. Error recovery allows partial parsing of malformed input. A new `orv dump ast` CLI command enables inspection.

**Architecture:** The parser lives in the existing `orv-syntax` crate alongside the lexer. It is a recursive-descent parser with a Pratt sub-parser for expressions. The parser consumes `Vec<Spanned<TokenKind>>` and produces `(Module, DiagnosticBag)`. AST nodes are wrapped in `Spanned<T>` from `orv-span` for source location tracking. On syntax errors, the parser records a diagnostic and skips tokens to a synchronization point (closing delimiter or newline), inserting an `Expr::Error` or `Stmt::Error` placeholder so downstream phases always receive a complete tree.

**Tech Stack:** Rust (edition 2024, nightly, MSRV 1.88.0), `orv-span` (FileId, Span, Spanned), `orv-diagnostics` (DiagnosticBag, Diagnostic, Label), `pretty_assertions` for tests. Workspace lints: `unsafe_code = "forbid"`, `clippy::all = "deny"`, `clippy::pedantic = "warn"`, `clippy::nursery = "warn"`.

---

## File Structure

```text
crates/
  orv-syntax/
    src/
      lib.rs            — add `pub mod ast;` and `pub mod parser;`
      ast.rs            — AST node type definitions
      parser.rs         — recursive-descent parser with Pratt expression parsing
    tests/
      parser_fixtures.rs — fixture-based integration tests
crates/
  orv-cli/
    src/
      main.rs           — add `dump ast` subcommand
fixtures/
  parser/
    hello.orv           — symlink or copy of fixtures/ok/hello.orv content
    counter.orv         — symlink or copy of fixtures/ok/counter.orv content
    server-basic.orv    — symlink or copy of fixtures/ok/server-basic.orv content
```

---

### Task 1: Define AST node types

**Files:**
- Create: `crates/orv-syntax/src/ast.rs`
- Modify: `crates/orv-syntax/src/lib.rs`

- [ ] **Step 1: Add `pub mod ast;` to `crates/orv-syntax/src/lib.rs`**

Replace the entire contents of `crates/orv-syntax/src/lib.rs` with:

```rust
pub mod ast;
pub mod lexer;
pub mod parser;
pub mod token;
```

Note: `pub mod parser;` is also added here so the crate compiles in subsequent tasks. We will create an empty `parser.rs` stub in Step 3.

- [ ] **Step 2: Create `crates/orv-syntax/src/ast.rs`**

```rust
//! AST node definitions for the orv language.
//!
//! Every node is wrapped in `Spanned<T>` at the point of use so that source
//! locations are available without cluttering the node definitions themselves.

use orv_span::Spanned;

/// A unique numeric id for an AST node, assigned during parsing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(u32);

impl NodeId {
    /// Creates a new `NodeId` from a raw value.
    pub const fn new(raw: u32) -> Self {
        Self(raw)
    }

    /// Returns the underlying raw value.
    pub const fn raw(self) -> u32 {
        self.0
    }
}

// ── Top-level ───────────────────────────────────────────────────────────────

/// A parsed source file.
#[derive(Debug, Clone)]
pub struct Module {
    pub items: Vec<Spanned<Item>>,
}

// ── Items ───────────────────────────────────────────────────────────────────

/// A top-level declaration.
#[derive(Debug, Clone)]
pub enum Item {
    /// `import path.to.{symbols}`
    Import(ImportItem),
    /// `[pub] function name(params): RetTy -> body`
    Function(FunctionItem),
    /// `[pub] define Name(params) -> @node { body }`
    Define(DefineItem),
    /// `[pub] struct Name { fields }`
    Struct(StructItem),
    /// `[pub] enum Name { variants }`
    Enum(EnumItem),
    /// `[pub] type Name = Type`
    TypeAlias(TypeAliasItem),
    /// `[pub] let/const binding`
    Binding(BindingStmt),
    /// A bare statement at top level (expression-statement, node, etc.)
    Stmt(Stmt),
    /// Placeholder for a declaration that failed to parse.
    Error,
}

/// `import path.to.{symbols}` or `import path.to.Name`
#[derive(Debug, Clone)]
pub struct ImportItem {
    /// Dot-separated path segments, e.g. `["components", "Button"]`
    pub path: Vec<Spanned<String>>,
    /// If the import ends with `.{A, B}`, the individual names.
    /// Empty if the import is a single symbol (the last path segment).
    pub names: Vec<Spanned<String>>,
    /// Optional alias: `import foo.Bar as Baz`
    pub alias: Option<Spanned<String>>,
}

/// `[pub] function name(params): RetTy -> body`
#[derive(Debug, Clone)]
pub struct FunctionItem {
    pub is_pub: bool,
    pub is_async: bool,
    pub name: Spanned<String>,
    pub params: Vec<Spanned<Param>>,
    pub return_type: Option<Spanned<TypeExpr>>,
    pub body: Spanned<Expr>,
}

/// `[pub] define Name(params) -> @node { body }`
#[derive(Debug, Clone)]
pub struct DefineItem {
    pub is_pub: bool,
    pub name: Spanned<String>,
    pub params: Vec<Spanned<Param>>,
    /// The domain hint, e.g. `@html` in `-> @html`.
    pub return_domain: Option<Spanned<NodeName>>,
    pub body: Spanned<Expr>,
}

/// `[pub] struct Name { fields }`
#[derive(Debug, Clone)]
pub struct StructItem {
    pub is_pub: bool,
    pub name: Spanned<String>,
    pub fields: Vec<Spanned<StructField>>,
}

/// A single field in a struct definition.
#[derive(Debug, Clone)]
pub struct StructField {
    pub name: Spanned<String>,
    pub ty: Spanned<TypeExpr>,
}

/// `[pub] enum Name { variants }`
#[derive(Debug, Clone)]
pub struct EnumItem {
    pub is_pub: bool,
    pub name: Spanned<String>,
    pub variants: Vec<Spanned<EnumVariant>>,
}

/// A single variant in an enum definition.
#[derive(Debug, Clone)]
pub struct EnumVariant {
    pub name: Spanned<String>,
    /// Optional payload types, e.g. `Ok(T)`.
    pub fields: Vec<Spanned<TypeExpr>>,
}

/// `[pub] type Name = Type`
#[derive(Debug, Clone)]
pub struct TypeAliasItem {
    pub is_pub: bool,
    pub name: Spanned<String>,
    pub ty: Spanned<TypeExpr>,
}

// ── Params ──────────────────────────────────────────────────────────────────

/// A function or define parameter.
#[derive(Debug, Clone)]
pub struct Param {
    pub name: Spanned<String>,
    pub ty: Option<Spanned<TypeExpr>>,
    pub default: Option<Spanned<Expr>>,
}

// ── Statements ──────────────────────────────────────────────────────────────

/// A statement within a block.
#[derive(Debug, Clone)]
pub enum Stmt {
    /// `let [mut] [sig] name [: Type] = expr`
    Binding(BindingStmt),
    /// `expr`
    Return(Option<Spanned<Expr>>),
    /// `if cond { body } [else { body }]`
    If(IfStmt),
    /// `for pattern of iterable { body }`
    For(ForStmt),
    /// `while cond { body }`
    While(WhileStmt),
    /// A bare expression used as a statement.
    Expr(Spanned<Expr>),
    /// Placeholder for a statement that failed to parse.
    Error,
}

/// `let [mut] [sig] name [: Type] = expr`
#[derive(Debug, Clone)]
pub struct BindingStmt {
    pub is_pub: bool,
    pub is_const: bool,
    pub is_mut: bool,
    pub is_sig: bool,
    pub name: Spanned<String>,
    pub ty: Option<Spanned<TypeExpr>>,
    pub value: Option<Spanned<Expr>>,
}

/// `if cond { body } [else if cond { body }]* [else { body }]`
#[derive(Debug, Clone)]
pub struct IfStmt {
    pub condition: Spanned<Expr>,
    pub then_body: Spanned<Expr>,
    pub else_body: Option<Spanned<Expr>>,
}

/// `for pattern of iterable { body }`
#[derive(Debug, Clone)]
pub struct ForStmt {
    pub binding: Spanned<String>,
    pub iterable: Spanned<Expr>,
    pub body: Spanned<Expr>,
}

/// `while cond { body }`
#[derive(Debug, Clone)]
pub struct WhileStmt {
    pub condition: Spanned<Expr>,
    pub body: Spanned<Expr>,
}

// ── Expressions ─────────────────────────────────────────────────────────────

/// An expression.
#[derive(Debug, Clone)]
pub enum Expr {
    /// Integer literal: `42`
    IntLiteral(i64),
    /// Float literal: `3.14`
    FloatLiteral(f64),
    /// String literal: `"hello"`
    StringLiteral(String),
    /// Interpolated string: `"Hello {name}!"`
    StringInterp(Vec<StringPart>),
    /// Boolean literal: `true` / `false`
    BoolLiteral(bool),
    /// `void`
    Void,
    /// An identifier: `foo`
    Ident(String),
    /// Binary operation: `a + b`
    Binary {
        left: Box<Spanned<Expr>>,
        op: Spanned<BinOp>,
        right: Box<Spanned<Expr>>,
    },
    /// Unary operation: `-x`, `!x`
    Unary {
        op: Spanned<UnaryOp>,
        operand: Box<Spanned<Expr>>,
    },
    /// Assignment: `x = expr`, `x += expr`
    Assign {
        target: Box<Spanned<Expr>>,
        op: Spanned<AssignOp>,
        value: Box<Spanned<Expr>>,
    },
    /// Function call: `foo(args)`
    Call {
        callee: Box<Spanned<Expr>>,
        args: Vec<Spanned<CallArg>>,
    },
    /// Field access: `a.b`
    Field {
        object: Box<Spanned<Expr>>,
        field: Spanned<String>,
    },
    /// Index access: `a[b]`
    Index {
        object: Box<Spanned<Expr>>,
        index: Box<Spanned<Expr>>,
    },
    /// Block: `{ stmts... }`
    Block(Vec<Spanned<Stmt>>),
    /// Object literal: `{ key: value, ... }`
    Object(Vec<Spanned<ObjectField>>),
    /// Array literal: `[a, b, c]`
    Array(Vec<Spanned<Expr>>),
    /// Node expression: `@name tokens... { body }`
    Node(NodeExpr),
    /// Parenthesized expression: `(expr)`
    Paren(Box<Spanned<Expr>>),
    /// `await expr`
    Await(Box<Spanned<Expr>>),
    /// Placeholder for an expression that failed to parse.
    Error,
}

/// A part of an interpolated string.
#[derive(Debug, Clone)]
pub enum StringPart {
    /// A literal text segment.
    Lit(String),
    /// An interpolated expression.
    Expr(Spanned<Expr>),
}

/// A call argument, possibly named.
#[derive(Debug, Clone)]
pub struct CallArg {
    /// Named argument: `foo(name=value)`.
    pub name: Option<Spanned<String>>,
    pub value: Spanned<Expr>,
}

/// A key-value pair in an object literal.
#[derive(Debug, Clone)]
pub struct ObjectField {
    pub key: Spanned<String>,
    pub value: Spanned<Expr>,
}

// ── Node expressions ────────────────────────────────────────────────────────

/// A dot-separated node name, e.g. `io.out`, `html`, `respond`.
#[derive(Debug, Clone)]
pub struct NodeName {
    pub segments: Vec<Spanned<String>>,
}

impl std::fmt::Display for NodeName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (i, seg) in self.segments.iter().enumerate() {
            if i > 0 {
                write!(f, ".")?;
            }
            write!(f, "{}", seg.node())?;
        }
        Ok(())
    }
}

/// `@name positional_tokens... %props... { body }`
#[derive(Debug, Clone)]
pub struct NodeExpr {
    /// The node name after `@`, e.g. `io.out`, `button`, `route`.
    pub name: Spanned<NodeName>,
    /// Positional tokens: string literals, integers, identifiers, paths,
    /// and other bare tokens that appear before `{` or end of line.
    pub positional: Vec<Spanned<Expr>>,
    /// Inline properties: `%key=value` on the same line.
    pub properties: Vec<Spanned<Property>>,
    /// The body block, if present: `{ children and statements }`.
    pub body: Option<Spanned<Expr>>,
}

/// A `%key=value` property binding on a node.
#[derive(Debug, Clone)]
pub struct Property {
    pub name: Spanned<String>,
    pub value: Spanned<Expr>,
}

// ── Types ───────────────────────────────────────────────────────────────────

/// A type expression in an annotation.
#[derive(Debug, Clone)]
pub enum TypeExpr {
    /// A simple named type: `i32`, `string`, `User`
    Named(String),
    /// A nullable type: `T?`
    Nullable(Box<Spanned<TypeExpr>>),
    /// A generic type: `Vec<T>`, `HashMap<K, V>`
    Generic {
        name: Spanned<String>,
        args: Vec<Spanned<TypeExpr>>,
    },
    /// A function type: `(A, B) -> C`
    Function {
        params: Vec<Spanned<TypeExpr>>,
        ret: Box<Spanned<TypeExpr>>,
    },
    /// A node type reference: `@html`
    Node(Spanned<NodeName>),
    /// Placeholder for a type that failed to parse.
    Error,
}

// ── Operators ───────────────────────────────────────────────────────────────

/// Binary operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Eq,
    NotEq,
    Lt,
    LtEq,
    Gt,
    GtEq,
    And,
    Or,
    Pipe,
}

/// Unary operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Neg,
    Not,
}

/// Assignment operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssignOp {
    Assign,
    AddAssign,
    SubAssign,
}

// ── Display helpers for AST dump ────────────────────────────────────────────

impl std::fmt::Display for BinOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Add => write!(f, "+"),
            Self::Sub => write!(f, "-"),
            Self::Mul => write!(f, "*"),
            Self::Div => write!(f, "/"),
            Self::Eq => write!(f, "=="),
            Self::NotEq => write!(f, "!="),
            Self::Lt => write!(f, "<"),
            Self::LtEq => write!(f, "<="),
            Self::Gt => write!(f, ">"),
            Self::GtEq => write!(f, ">="),
            Self::And => write!(f, "&&"),
            Self::Or => write!(f, "||"),
            Self::Pipe => write!(f, "|>"),
        }
    }
}

impl std::fmt::Display for UnaryOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Neg => write!(f, "-"),
            Self::Not => write!(f, "!"),
        }
    }
}

impl std::fmt::Display for AssignOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Assign => write!(f, "="),
            Self::AddAssign => write!(f, "+="),
            Self::SubAssign => write!(f, "-="),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use orv_span::{FileId, Span, Spanned};

    #[test]
    fn node_name_display() {
        let file = FileId::new(0);
        let name = NodeName {
            segments: vec![
                Spanned::new("io".to_owned(), Span::new(file, 0, 2)),
                Spanned::new("out".to_owned(), Span::new(file, 3, 6)),
            ],
        };
        assert_eq!(name.to_string(), "io.out");
    }

    #[test]
    fn single_segment_node_name() {
        let file = FileId::new(0);
        let name = NodeName {
            segments: vec![Spanned::new("button".to_owned(), Span::new(file, 0, 6))],
        };
        assert_eq!(name.to_string(), "button");
    }

    #[test]
    fn binop_display() {
        assert_eq!(BinOp::Add.to_string(), "+");
        assert_eq!(BinOp::And.to_string(), "&&");
        assert_eq!(BinOp::Pipe.to_string(), "|>");
    }

    #[test]
    fn assign_op_display() {
        assert_eq!(AssignOp::Assign.to_string(), "=");
        assert_eq!(AssignOp::AddAssign.to_string(), "+=");
        assert_eq!(AssignOp::SubAssign.to_string(), "-=");
    }
}
```

- [ ] **Step 3: Create an empty `crates/orv-syntax/src/parser.rs` stub**

```rust
//! Recursive-descent parser for the orv language.
//!
//! Consumes `Vec<Spanned<TokenKind>>` and produces `(Module, DiagnosticBag)`.
//! Full implementation follows in subsequent tasks.
```

- [ ] **Step 4: Verify**

Run: `cargo test -p orv-syntax`
Expected: All existing lexer tests pass plus 4 new AST tests pass.

Run: `cargo clippy -p orv-syntax`
Expected: Clean.

- [ ] **Step 5: Commit**

```bash
git add crates/orv-syntax/src/ast.rs crates/orv-syntax/src/parser.rs crates/orv-syntax/src/lib.rs
git commit -m "feat(syntax): add AST node type definitions for orv parser"
```

---

### Task 2: Parser skeleton — cursor, helpers, and module entry point

**Files:**
- Modify: `crates/orv-syntax/src/parser.rs`

- [ ] **Step 1: Implement the `Parser` struct with token cursor and helpers**

Replace the entire contents of `crates/orv-syntax/src/parser.rs` with:

```rust
//! Recursive-descent parser for the orv language.
//!
//! Consumes `Vec<Spanned<TokenKind>>` and produces `(Module, DiagnosticBag)`.

use orv_diagnostics::{Diagnostic, DiagnosticBag, Label};
use orv_span::{Span, Spanned};

use crate::ast::{
    AssignOp, BinOp, BindingStmt, CallArg, DefineItem, EnumItem, EnumVariant, Expr, ForStmt,
    FunctionItem, IfStmt, ImportItem, Item, Module, NodeExpr, NodeName, ObjectField, Param,
    Property, Stmt, StringPart, StructField, StructItem, TypeAliasItem, TypeExpr, UnaryOp,
    WhileStmt,
};
use crate::token::TokenKind;

/// Parses a token stream into a `Module` AST with diagnostics.
pub fn parse(tokens: Vec<Spanned<TokenKind>>) -> (Module, DiagnosticBag) {
    let mut parser = Parser::new(tokens);
    let module = parser.parse_module();
    (module, parser.diagnostics)
}

/// The recursive-descent parser state.
struct Parser {
    tokens: Vec<Spanned<TokenKind>>,
    pos: usize,
    diagnostics: DiagnosticBag,
}

impl Parser {
    fn new(tokens: Vec<Spanned<TokenKind>>) -> Self {
        Self {
            tokens,
            pos: 0,
            diagnostics: DiagnosticBag::new(),
        }
    }

    // ── Token cursor ────────────────────────────────────────────────────

    /// Returns a reference to the current token kind, or `Eof` if past the end.
    fn peek(&self) -> &TokenKind {
        self.tokens
            .get(self.pos)
            .map_or(&TokenKind::Eof, |t| t.node())
    }

    /// Returns a reference to the token kind at offset `n` ahead of current.
    fn peek_at(&self, n: usize) -> &TokenKind {
        self.tokens
            .get(self.pos + n)
            .map_or(&TokenKind::Eof, |t| t.node())
    }

    /// Returns the span of the current token.
    fn current_span(&self) -> Span {
        self.tokens
            .get(self.pos)
            .map_or_else(|| self.eof_span(), Spanned::span)
    }

    /// Returns a zero-width span at the end of the last token (for EOF).
    fn eof_span(&self) -> Span {
        self.tokens
            .last()
            .map_or(Span::new(orv_span::FileId::new(0), 0, 0), |t| {
                let s = t.span();
                Span::new(s.file(), s.end(), s.end())
            })
    }

    /// Advances the cursor by one token and returns the consumed token.
    fn bump(&mut self) -> &Spanned<TokenKind> {
        let tok = &self.tokens[self.pos];
        self.pos += 1;
        tok
    }

    /// Returns `true` if the current token matches the expected kind.
    fn at(&self, kind: &TokenKind) -> bool {
        std::mem::discriminant(self.peek()) == std::mem::discriminant(kind)
    }

    /// Returns `true` if the current position is at end of file.
    fn at_eof(&self) -> bool {
        matches!(self.peek(), TokenKind::Eof)
    }

    /// Consumes the current token if it matches the expected kind.
    /// Returns `true` if consumed, `false` otherwise.
    fn eat(&mut self, kind: &TokenKind) -> bool {
        if self.at(kind) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    /// Consumes the current token if it matches the expected kind, returning
    /// the spanned token. Otherwise, emits a diagnostic and returns `None`.
    fn expect(&mut self, kind: &TokenKind, context: &str) -> Option<Span> {
        if self.at(kind) {
            let span = self.current_span();
            self.pos += 1;
            Some(span)
        } else {
            let span = self.current_span();
            self.diagnostics.push(
                Diagnostic::error(format!("expected {context}, found {:?}", self.peek()))
                    .with_label(Label::primary(span, format!("expected {context}"))),
            );
            None
        }
    }

    /// Consumes an `Ident` token and returns its name and span.
    /// On failure, emits a diagnostic and returns `None`.
    fn expect_ident(&mut self, context: &str) -> Option<Spanned<String>> {
        if let TokenKind::Ident(name) = self.peek().clone() {
            let span = self.current_span();
            self.pos += 1;
            Some(Spanned::new(name, span))
        } else {
            let span = self.current_span();
            self.diagnostics.push(
                Diagnostic::error(format!("expected identifier for {context}, found {:?}", self.peek()))
                    .with_label(Label::primary(span, "expected identifier")),
            );
            None
        }
    }

    /// Skips newline tokens at the current position.
    fn skip_newlines(&mut self) {
        while self.at(&TokenKind::Newline) {
            self.pos += 1;
        }
    }

    /// Expects a newline or EOF to terminate a statement.
    /// Returns `true` if found.
    fn expect_newline_or_eof(&mut self) -> bool {
        if self.at(&TokenKind::Newline) || self.at_eof() || self.at(&TokenKind::RBrace) {
            self.eat(&TokenKind::Newline);
            true
        } else {
            let span = self.current_span();
            self.diagnostics.push(
                Diagnostic::error(format!(
                    "expected newline or end of statement, found {:?}",
                    self.peek()
                ))
                .with_label(Label::primary(span, "expected newline")),
            );
            // Recovery: skip to next newline
            self.synchronize_to_newline();
            false
        }
    }

    // ── Error recovery ──────────────────────────────────────────────────

    /// Skips tokens until a newline, closing brace, or EOF is found.
    fn synchronize_to_newline(&mut self) {
        while !self.at_eof() && !self.at(&TokenKind::Newline) && !self.at(&TokenKind::RBrace) {
            self.pos += 1;
        }
        // Consume the newline if present
        self.eat(&TokenKind::Newline);
    }

    /// Skips tokens until a closing brace at the correct nesting depth, or EOF.
    fn synchronize_to_close_brace(&mut self) {
        let mut depth: u32 = 1;
        while !self.at_eof() {
            match self.peek() {
                TokenKind::LBrace => {
                    depth += 1;
                    self.pos += 1;
                }
                TokenKind::RBrace => {
                    depth -= 1;
                    if depth == 0 {
                        self.pos += 1; // consume the closing brace
                        return;
                    }
                    self.pos += 1;
                }
                _ => {
                    self.pos += 1;
                }
            }
        }
    }

    // ── Module parsing ──────────────────────────────────────────────────

    fn parse_module(&mut self) -> Module {
        let mut items = Vec::new();
        self.skip_newlines();

        while !self.at_eof() {
            let item = self.parse_item();
            items.push(item);
            self.skip_newlines();
        }

        Module { items }
    }

    // ── Item parsing ────────────────────────────────────────────────────

    fn parse_item(&mut self) -> Spanned<Item> {
        let start = self.current_span();

        // Check for `pub` modifier
        let is_pub = self.eat(&TokenKind::Pub);

        let item = match self.peek().clone() {
            TokenKind::Import => self.parse_import_item(),
            TokenKind::Function => self.parse_function_item(is_pub, false),
            TokenKind::Async => {
                self.pos += 1;
                if self.at(&TokenKind::Function) {
                    self.parse_function_item(is_pub, true)
                } else {
                    self.diagnostics.push(
                        Diagnostic::error("expected `function` after `async`")
                            .with_label(Label::primary(self.current_span(), "expected `function`")),
                    );
                    self.synchronize_to_newline();
                    Item::Error
                }
            }
            TokenKind::Define => self.parse_define_item(is_pub),
            TokenKind::Struct => self.parse_struct_item(is_pub),
            TokenKind::Enum => self.parse_enum_item(is_pub),
            TokenKind::Type => self.parse_type_alias_item(is_pub),
            TokenKind::Let | TokenKind::Const => self.parse_binding_item(is_pub),
            _ => {
                if is_pub {
                    self.diagnostics.push(
                        Diagnostic::error(format!(
                            "expected declaration after `pub`, found {:?}",
                            self.peek()
                        ))
                        .with_label(Label::primary(self.current_span(), "expected declaration")),
                    );
                    self.synchronize_to_newline();
                    Item::Error
                } else {
                    // Bare statement at top level
                    let stmt = self.parse_stmt();
                    Item::Stmt(stmt.node().clone())
                }
            }
        };

        let end = self.current_span();
        Spanned::new(item, start.merge(end))
    }

    // ── Import ──────────────────────────────────────────────────────────

    fn parse_import_item(&mut self) -> Item {
        // Consume `import`
        self.pos += 1;

        let mut path = Vec::new();
        let mut names = Vec::new();
        let mut alias = None;

        // Parse first segment
        let Some(first) = self.expect_ident("import path") else {
            self.synchronize_to_newline();
            Item::Error;
        };
        path.push(first);

        // Parse remaining dot-separated segments
        while self.eat(&TokenKind::Dot) {
            // Check for `{A, B}` destructured imports
            if self.at(&TokenKind::LBrace) {
                self.pos += 1; // consume `{`
                self.skip_newlines();
                while !self.at(&TokenKind::RBrace) && !self.at_eof() {
                    if let Some(name) = self.expect_ident("import name") {
                        names.push(name);
                    }
                    self.skip_newlines();
                    if !self.eat(&TokenKind::Comma) {
                        self.skip_newlines();
                        break;
                    }
                    self.skip_newlines();
                }
                self.expect(&TokenKind::RBrace, "`}`");
                break;
            }

            // Check for wildcard `*`
            if self.at(&TokenKind::Star) {
                let span = self.current_span();
                self.pos += 1;
                names.push(Spanned::new("*".to_owned(), span));
                break;
            }

            let Some(segment) = self.expect_ident("import path segment") else {
                self.synchronize_to_newline();
                Item::Error;
            };
            path.push(segment);
        }

        // Check for `as Alias`
        if let TokenKind::Ident(ref s) = self.peek().clone() {
            if s == "as" {
                self.pos += 1; // consume `as`
                if let Some(a) = self.expect_ident("import alias") {
                    alias = Some(a);
                }
            }
        }

        self.expect_newline_or_eof();

        Item::Import(ImportItem { path, names, alias })
    }

    // ── Function ────────────────────────────────────────────────────────

    fn parse_function_item(&mut self, is_pub: bool, is_async: bool) -> Item {
        // Consume `function`
        self.pos += 1;

        let Some(name) = self.expect_ident("function name") else {
            self.synchronize_to_newline();
            Item::Error;
        };

        // Parse params
        let params = if self.at(&TokenKind::LParen) {
            self.parse_param_list()
        } else {
            Vec::new()
        };

        // Parse optional type: `: RetTy`
        let return_type = if self.eat(&TokenKind::Colon) {
            Some(self.parse_type_expr())
        } else {
            None
        };

        // Expect `->`
        if !self.eat(&TokenKind::Arrow) {
            let span = self.current_span();
            self.diagnostics.push(
                Diagnostic::error("expected `->` in function declaration")
                    .with_label(Label::primary(span, "expected `->`")),
            );
        }

        // Parse body
        let body = self.parse_expr();

        self.expect_newline_or_eof();

        Item::Function(FunctionItem {
            is_pub,
            is_async,
            name,
            params,
            return_type,
            body,
        })
    }

    // ── Define ──────────────────────────────────────────────────────────

    fn parse_define_item(&mut self, is_pub: bool) -> Item {
        // Consume `define`
        self.pos += 1;

        let Some(name) = self.expect_ident("define name") else {
            self.synchronize_to_newline();
            Item::Error;
        };

        // Parse params
        let params = if self.at(&TokenKind::LParen) {
            self.parse_param_list()
        } else {
            Vec::new()
        };

        // Parse optional domain: `-> @node`
        let return_domain = if self.eat(&TokenKind::Arrow) {
            if self.at(&TokenKind::At) {
                Some(self.parse_node_name())
            } else {
                let span = self.current_span();
                self.diagnostics.push(
                    Diagnostic::error("expected `@node` after `->` in define")
                        .with_label(Label::primary(span, "expected `@node`")),
                );
                None
            }
        } else {
            None
        };

        // Parse body (must be a block)
        let body = self.parse_expr();

        self.expect_newline_or_eof();

        Item::Define(DefineItem {
            is_pub,
            name,
            params,
            return_domain,
            body,
        })
    }

    // ── Struct ──────────────────────────────────────────────────────────

    fn parse_struct_item(&mut self, is_pub: bool) -> Item {
        // Consume `struct`
        self.pos += 1;

        let Some(name) = self.expect_ident("struct name") else {
            self.synchronize_to_newline();
            Item::Error;
        };

        let Some(_) = self.expect(&TokenKind::LBrace, "`{`") else {
            self.synchronize_to_newline();
            Item::Error;
        };

        let mut fields = Vec::new();
        self.skip_newlines();

        while !self.at(&TokenKind::RBrace) && !self.at_eof() {
            let field_start = self.current_span();

            let Some(field_name) = self.expect_ident("field name") else {
                self.synchronize_to_newline();
                self.skip_newlines();
                continue;
            };

            if self.expect(&TokenKind::Colon, "`:` after field name").is_none() {
                self.synchronize_to_newline();
                self.skip_newlines();
                continue;
            }

            let ty = self.parse_type_expr();
            let field_end = ty.span();

            fields.push(Spanned::new(
                StructField { name: field_name, ty },
                field_start.merge(field_end),
            ));

            self.skip_newlines();
        }

        self.expect(&TokenKind::RBrace, "`}`");
        self.expect_newline_or_eof();

        Item::Struct(StructItem {
            is_pub,
            name,
            fields,
        })
    }

    // ── Enum ────────────────────────────────────────────────────────────

    fn parse_enum_item(&mut self, is_pub: bool) -> Item {
        // Consume `enum`
        self.pos += 1;

        let Some(name) = self.expect_ident("enum name") else {
            self.synchronize_to_newline();
            Item::Error;
        };

        let Some(_) = self.expect(&TokenKind::LBrace, "`{`") else {
            self.synchronize_to_newline();
            Item::Error;
        };

        let mut variants = Vec::new();
        self.skip_newlines();

        while !self.at(&TokenKind::RBrace) && !self.at_eof() {
            let var_start = self.current_span();

            let Some(var_name) = self.expect_ident("variant name") else {
                self.synchronize_to_newline();
                self.skip_newlines();
                continue;
            };

            // Optional payload: `(Type1, Type2)`
            let fields = if self.at(&TokenKind::LParen) {
                self.pos += 1; // consume `(`
                let mut f = Vec::new();
                while !self.at(&TokenKind::RParen) && !self.at_eof() {
                    f.push(self.parse_type_expr());
                    if !self.eat(&TokenKind::Comma) {
                        break;
                    }
                }
                self.expect(&TokenKind::RParen, "`)`");
                f
            } else {
                Vec::new()
            };

            let var_end = self.current_span();
            variants.push(Spanned::new(
                EnumVariant {
                    name: var_name,
                    fields,
                },
                var_start.merge(var_end),
            ));

            self.skip_newlines();
        }

        self.expect(&TokenKind::RBrace, "`}`");
        self.expect_newline_or_eof();

        Item::Enum(EnumItem {
            is_pub,
            name,
            variants,
        })
    }

    // ── Type alias ──────────────────────────────────────────────────────

    fn parse_type_alias_item(&mut self, is_pub: bool) -> Item {
        // Consume `type`
        self.pos += 1;

        let Some(name) = self.expect_ident("type alias name") else {
            self.synchronize_to_newline();
            Item::Error;
        };

        if self.expect(&TokenKind::Eq, "`=`").is_none() {
            self.synchronize_to_newline();
            Item::Error;
        }

        let ty = self.parse_type_expr();
        self.expect_newline_or_eof();

        Item::TypeAlias(TypeAliasItem { is_pub, name, ty })
    }

    // ── Binding (let/const) ─────────────────────────────────────────────

    fn parse_binding_item(&mut self, is_pub: bool) -> Item {
        let binding = self.parse_binding_stmt(is_pub);
        Item::Binding(binding)
    }

    fn parse_binding_stmt(&mut self, is_pub: bool) -> BindingStmt {
        let is_const = self.eat(&TokenKind::Const);
        if !is_const {
            self.eat(&TokenKind::Let); // consume `let` if present
        }

        let is_mut = self.eat(&TokenKind::Mut);
        let is_sig = self.eat(&TokenKind::Sig);

        let name = self
            .expect_ident("variable name")
            .unwrap_or_else(|| Spanned::new("<error>".to_owned(), self.current_span()));

        // Optional type annotation: `: Type`
        let ty = if self.eat(&TokenKind::Colon) {
            Some(self.parse_type_expr())
        } else {
            None
        };

        // Optional initializer: `= expr`
        let value = if self.eat(&TokenKind::Eq) {
            Some(self.parse_expr())
        } else {
            None
        };

        self.expect_newline_or_eof();

        BindingStmt {
            is_pub,
            is_const,
            is_mut,
            is_sig,
            name,
            ty,
            value,
        }
    }

    // ── Statement parsing ───────────────────────────────────────────────

    fn parse_stmt(&mut self) -> Spanned<Stmt> {
        let start = self.current_span();

        let stmt = match self.peek().clone() {
            TokenKind::Let | TokenKind::Const => {
                let binding = self.parse_binding_stmt(false);
                Stmt::Binding(binding)
            }
            TokenKind::Return => {
                self.pos += 1;
                if self.at(&TokenKind::Newline) || self.at_eof() || self.at(&TokenKind::RBrace) {
                    self.eat(&TokenKind::Newline);
                    Stmt::Return(None)
                } else {
                    let expr = self.parse_expr();
                    self.expect_newline_or_eof();
                    Stmt::Return(Some(expr))
                }
            }
            TokenKind::If => {
                let if_stmt = self.parse_if_stmt();
                Stmt::If(if_stmt)
            }
            TokenKind::For => {
                let for_stmt = self.parse_for_stmt();
                Stmt::For(for_stmt)
            }
            TokenKind::While => {
                let while_stmt = self.parse_while_stmt();
                Stmt::While(while_stmt)
            }
            _ => {
                let expr = self.parse_expr();
                self.expect_newline_or_eof();
                Stmt::Expr(expr)
            }
        };

        let end = self.current_span();
        Spanned::new(stmt, start.merge(end))
    }

    fn parse_if_stmt(&mut self) -> IfStmt {
        // Consume `if`
        self.pos += 1;

        let condition = self.parse_expr();
        let then_body = self.parse_block_expr();

        let else_body = if self.eat(&TokenKind::Else) {
            if self.at(&TokenKind::If) {
                // `else if` chain: wrap in a synthetic block containing the if stmt
                let nested = self.parse_if_stmt();
                let span = self.current_span();
                Some(Spanned::new(
                    Expr::Block(vec![Spanned::new(Stmt::If(nested), span)]),
                    span,
                ))
            } else {
                Some(self.parse_block_expr())
            }
        } else {
            None
        };

        IfStmt {
            condition,
            then_body,
            else_body,
        }
    }

    fn parse_for_stmt(&mut self) -> ForStmt {
        // Consume `for`
        self.pos += 1;

        let binding = self
            .expect_ident("loop variable")
            .unwrap_or_else(|| Spanned::new("<error>".to_owned(), self.current_span()));

        if !self.eat(&TokenKind::Of) {
            let span = self.current_span();
            self.diagnostics.push(
                Diagnostic::error("expected `of` in for loop")
                    .with_label(Label::primary(span, "expected `of`")),
            );
        }

        let iterable = self.parse_expr();
        let body = self.parse_block_expr();

        ForStmt {
            binding,
            iterable,
            body,
        }
    }

    fn parse_while_stmt(&mut self) -> WhileStmt {
        // Consume `while`
        self.pos += 1;

        let condition = self.parse_expr();
        let body = self.parse_block_expr();

        WhileStmt { condition, body }
    }

    // ── Block and body parsing ──────────────────────────────────────────

    /// Parses a `{ ... }` block, returning either a `Block` or `Object` expression.
    fn parse_block_expr(&mut self) -> Spanned<Expr> {
        let start = self.current_span();

        if self.expect(&TokenKind::LBrace, "`{`").is_none() {
            Spanned::new(Expr::Error, start);
        }

        self.skip_newlines();

        // Empty block
        if self.at(&TokenKind::RBrace) {
            let end = self.current_span();
            self.pos += 1;
            Spanned::new(Expr::Block(Vec::new()), start.merge(end));
        }

        // Disambiguate object literal vs code block:
        // If the first content is `Ident Colon` (and NOT `Ident Colon Colon`),
        // treat as object literal.
        let is_object = self.is_object_literal_start();

        if is_object {
            self.parse_object_literal_body(start)
        } else {
            self.parse_code_block_body(start)
        }
    }

    /// Checks if the current position looks like the start of an object literal:
    /// `Ident :` but not `Ident ::`.
    fn is_object_literal_start(&self) -> bool {
        matches!(self.peek(), TokenKind::Ident(_))
            && matches!(self.peek_at(1), TokenKind::Colon)
            && !matches!(self.peek_at(2), TokenKind::Colon)
    }

    fn parse_object_literal_body(&mut self, start: Span) -> Spanned<Expr> {
        let mut fields = Vec::new();

        while !self.at(&TokenKind::RBrace) && !self.at_eof() {
            let field_start = self.current_span();

            // Parse a string key or ident key
            let key = if let TokenKind::StringLiteral(s) = self.peek().clone() {
                let span = self.current_span();
                self.pos += 1;
                Spanned::new(s, span)
            } else if let Some(ident) = self.expect_ident("object key") {
                ident
            } else {
                self.synchronize_to_newline();
                self.skip_newlines();
                continue;
            };

            if self.expect(&TokenKind::Colon, "`:` after object key").is_none() {
                self.synchronize_to_newline();
                self.skip_newlines();
                continue;
            }

            let value = self.parse_expr();
            let field_end = value.span();

            fields.push(Spanned::new(
                ObjectField { key, value },
                field_start.merge(field_end),
            ));

            // Allow comma or newline as separator
            if !self.eat(&TokenKind::Comma) {
                self.skip_newlines();
                // Don't require separator before `}`
                if self.at(&TokenKind::RBrace) {
                    break;
                }
            } else {
                self.skip_newlines();
            }
        }

        let end = self.current_span();
        self.expect(&TokenKind::RBrace, "`}`");
        Spanned::new(Expr::Object(fields), start.merge(end))
    }

    fn parse_code_block_body(&mut self, start: Span) -> Spanned<Expr> {
        let mut stmts = Vec::new();

        while !self.at(&TokenKind::RBrace) && !self.at_eof() {
            self.skip_newlines();
            if self.at(&TokenKind::RBrace) || self.at_eof() {
                break;
            }
            let stmt = self.parse_stmt();
            stmts.push(stmt);
            self.skip_newlines();
        }

        let end = self.current_span();
        self.expect(&TokenKind::RBrace, "`}`");
        Spanned::new(Expr::Block(stmts), start.merge(end))
    }

    // ── Parameter list ──────────────────────────────────────────────────

    fn parse_param_list(&mut self) -> Vec<Spanned<Param>> {
        self.pos += 1; // consume `(`
        let mut params = Vec::new();

        while !self.at(&TokenKind::RParen) && !self.at_eof() {
            let param_start = self.current_span();

            let Some(name) = self.expect_ident("parameter name") else {
                self.synchronize_to_newline();
                break;
            };

            let ty = if self.eat(&TokenKind::Colon) {
                Some(self.parse_type_expr())
            } else {
                None
            };

            let default = if self.eat(&TokenKind::Eq) {
                Some(self.parse_expr())
            } else {
                None
            };

            let param_end = self.current_span();
            params.push(Spanned::new(
                Param { name, ty, default },
                param_start.merge(param_end),
            ));

            if !self.eat(&TokenKind::Comma) {
                break;
            }
        }

        self.expect(&TokenKind::RParen, "`)`");
        params
    }

    // ── Node name parsing ───────────────────────────────────────────────

    /// Parses `@name` or `@name.sub.path`, consuming the `@` and the
    /// dot-separated identifier segments. Returns a `Spanned<NodeName>`.
    fn parse_node_name(&mut self) -> Spanned<NodeName> {
        let at_span = self.current_span();
        self.pos += 1; // consume `@`

        let mut segments = Vec::new();

        let Some(first) = self.expect_ident("node name") else {
            Spanned::new(
                NodeName {
                    segments: vec![Spanned::new("<error>".to_owned(), self.current_span())],
                },
                at_span,
            );
        };
        let mut end_span = first.span();
        segments.push(first);

        // Parse dot-separated segments: `@io.out`
        while self.at(&TokenKind::Dot) {
            // Look ahead: only continue if next is ident (avoid consuming `.` in
            // path contexts like `./public`)
            if !matches!(self.peek_at(1), TokenKind::Ident(_)) {
                break;
            }
            self.pos += 1; // consume `.`
            if let Some(seg) = self.expect_ident("node name segment") {
                end_span = seg.span();
                segments.push(seg);
            } else {
                break;
            }
        }

        Spanned::new(NodeName { segments }, at_span.merge(end_span))
    }

    // ── Expression parsing (Pratt parser) ───────────────────────────────

    /// Entry point for expression parsing.
    fn parse_expr(&mut self) -> Spanned<Expr> {
        self.parse_assignment_expr()
    }

    /// Parses assignment expressions: `target = value`, `target += value`.
    fn parse_assignment_expr(&mut self) -> Spanned<Expr> {
        let left = self.parse_pipe_expr();

        let assign_op = match self.peek() {
            TokenKind::Eq => {
                // Disambiguate: `==` is handled in binary, but `=` is assignment.
                // We already have separate Eq and EqEq tokens, so this is safe.
                Some(AssignOp::Assign)
            }
            TokenKind::PlusEq => Some(AssignOp::AddAssign),
            TokenKind::MinusEq => Some(AssignOp::SubAssign),
            _ => None,
        };

        if let Some(op) = assign_op {
            let op_span = self.current_span();
            self.pos += 1;
            let right = self.parse_assignment_expr(); // right-associative
            let span = left.span().merge(right.span());
            Spanned::new(
                Expr::Assign {
                    target: Box::new(left),
                    op: Spanned::new(op, op_span),
                    value: Box::new(right),
                },
                span,
            )
        } else {
            left
        }
    }

    /// Parses pipe expressions: `a |> b |> c`
    fn parse_pipe_expr(&mut self) -> Spanned<Expr> {
        let mut left = self.parse_or_expr();

        while self.at(&TokenKind::PipeGt) {
            let op_span = self.current_span();
            self.pos += 1;
            let right = self.parse_or_expr();
            let span = left.span().merge(right.span());
            left = Spanned::new(
                Expr::Binary {
                    left: Box::new(left),
                    op: Spanned::new(BinOp::Pipe, op_span),
                    right: Box::new(right),
                },
                span,
            );
        }

        left
    }

    /// Parses logical OR: `a || b`
    fn parse_or_expr(&mut self) -> Spanned<Expr> {
        let mut left = self.parse_and_expr();

        while self.at(&TokenKind::PipePipe) {
            let op_span = self.current_span();
            self.pos += 1;
            let right = self.parse_and_expr();
            let span = left.span().merge(right.span());
            left = Spanned::new(
                Expr::Binary {
                    left: Box::new(left),
                    op: Spanned::new(BinOp::Or, op_span),
                    right: Box::new(right),
                },
                span,
            );
        }

        left
    }

    /// Parses logical AND: `a && b`
    fn parse_and_expr(&mut self) -> Spanned<Expr> {
        let mut left = self.parse_equality_expr();

        while self.at(&TokenKind::AmpAmp) {
            let op_span = self.current_span();
            self.pos += 1;
            let right = self.parse_equality_expr();
            let span = left.span().merge(right.span());
            left = Spanned::new(
                Expr::Binary {
                    left: Box::new(left),
                    op: Spanned::new(BinOp::And, op_span),
                    right: Box::new(right),
                },
                span,
            );
        }

        left
    }

    /// Parses equality: `a == b`, `a != b`
    fn parse_equality_expr(&mut self) -> Spanned<Expr> {
        let mut left = self.parse_comparison_expr();

        loop {
            let op = match self.peek() {
                TokenKind::EqEq => BinOp::Eq,
                TokenKind::BangEq => BinOp::NotEq,
                _ => break,
            };
            let op_span = self.current_span();
            self.pos += 1;
            let right = self.parse_comparison_expr();
            let span = left.span().merge(right.span());
            left = Spanned::new(
                Expr::Binary {
                    left: Box::new(left),
                    op: Spanned::new(op, op_span),
                    right: Box::new(right),
                },
                span,
            );
        }

        left
    }

    /// Parses comparison: `a < b`, `a <= b`, `a > b`, `a >= b`
    fn parse_comparison_expr(&mut self) -> Spanned<Expr> {
        let mut left = self.parse_additive_expr();

        loop {
            let op = match self.peek() {
                TokenKind::Lt => BinOp::Lt,
                TokenKind::LtEq => BinOp::LtEq,
                TokenKind::Gt => BinOp::Gt,
                TokenKind::GtEq => BinOp::GtEq,
                _ => break,
            };
            let op_span = self.current_span();
            self.pos += 1;
            let right = self.parse_additive_expr();
            let span = left.span().merge(right.span());
            left = Spanned::new(
                Expr::Binary {
                    left: Box::new(left),
                    op: Spanned::new(op, op_span),
                    right: Box::new(right),
                },
                span,
            );
        }

        left
    }

    /// Parses addition/subtraction: `a + b`, `a - b`
    fn parse_additive_expr(&mut self) -> Spanned<Expr> {
        let mut left = self.parse_multiplicative_expr();

        loop {
            let op = match self.peek() {
                TokenKind::Plus => BinOp::Add,
                TokenKind::Minus => BinOp::Sub,
                _ => break,
            };
            let op_span = self.current_span();
            self.pos += 1;
            let right = self.parse_multiplicative_expr();
            let span = left.span().merge(right.span());
            left = Spanned::new(
                Expr::Binary {
                    left: Box::new(left),
                    op: Spanned::new(op, op_span),
                    right: Box::new(right),
                },
                span,
            );
        }

        left
    }

    /// Parses multiplication/division: `a * b`, `a / b`
    fn parse_multiplicative_expr(&mut self) -> Spanned<Expr> {
        let mut left = self.parse_unary_expr();

        loop {
            let op = match self.peek() {
                TokenKind::Star => BinOp::Mul,
                TokenKind::Slash => BinOp::Div,
                _ => break,
            };
            let op_span = self.current_span();
            self.pos += 1;
            let right = self.parse_unary_expr();
            let span = left.span().merge(right.span());
            left = Spanned::new(
                Expr::Binary {
                    left: Box::new(left),
                    op: Spanned::new(op, op_span),
                    right: Box::new(right),
                },
                span,
            );
        }

        left
    }

    /// Parses unary expressions: `-x`, `!x`, `await x`
    fn parse_unary_expr(&mut self) -> Spanned<Expr> {
        match self.peek() {
            TokenKind::Minus => {
                let op_span = self.current_span();
                self.pos += 1;
                let operand = self.parse_unary_expr();
                let span = op_span.merge(operand.span());
                Spanned::new(
                    Expr::Unary {
                        op: Spanned::new(UnaryOp::Neg, op_span),
                        operand: Box::new(operand),
                    },
                    span,
                )
            }
            TokenKind::Bang => {
                let op_span = self.current_span();
                self.pos += 1;
                let operand = self.parse_unary_expr();
                let span = op_span.merge(operand.span());
                Spanned::new(
                    Expr::Unary {
                        op: Spanned::new(UnaryOp::Not, op_span),
                        operand: Box::new(operand),
                    },
                    span,
                )
            }
            TokenKind::Await => {
                let await_span = self.current_span();
                self.pos += 1;
                let operand = self.parse_unary_expr();
                let span = await_span.merge(operand.span());
                Spanned::new(Expr::Await(Box::new(operand)), span)
            }
            _ => self.parse_postfix_expr(),
        }
    }

    /// Parses postfix expressions: calls `foo(args)`, field access `a.b`, index `a[b]`
    fn parse_postfix_expr(&mut self) -> Spanned<Expr> {
        let mut expr = self.parse_primary_expr();

        loop {
            match self.peek() {
                TokenKind::LParen => {
                    expr = self.parse_call_expr(expr);
                }
                TokenKind::Dot => {
                    // Only consume `.` if followed by an identifier (field access)
                    if matches!(self.peek_at(1), TokenKind::Ident(_)) {
                        self.pos += 1; // consume `.`
                        let field = self
                            .expect_ident("field name")
                            .unwrap_or_else(|| Spanned::new("<error>".to_owned(), self.current_span()));
                        let span = expr.span().merge(field.span());
                        expr = Spanned::new(
                            Expr::Field {
                                object: Box::new(expr),
                                field,
                            },
                            span,
                        );
                    } else {
                        break;
                    }
                }
                TokenKind::LBracket => {
                    self.pos += 1; // consume `[`
                    let index = self.parse_expr();
                    let end = self.current_span();
                    self.expect(&TokenKind::RBracket, "`]`");
                    let span = expr.span().merge(end);
                    expr = Spanned::new(
                        Expr::Index {
                            object: Box::new(expr),
                            index: Box::new(index),
                        },
                        span,
                    );
                }
                _ => break,
            }
        }

        expr
    }

    fn parse_call_expr(&mut self, callee: Spanned<Expr>) -> Spanned<Expr> {
        self.pos += 1; // consume `(`
        let mut args = Vec::new();

        while !self.at(&TokenKind::RParen) && !self.at_eof() {
            let arg_start = self.current_span();

            // Check for named argument: `name=value`
            let name = if matches!(self.peek(), TokenKind::Ident(_))
                && matches!(self.peek_at(1), TokenKind::Eq)
            {
                let n = self.expect_ident("argument name");
                self.pos += 1; // consume `=`
                n
            } else {
                None
            };

            let value = self.parse_expr();
            let arg_end = value.span();

            args.push(Spanned::new(
                CallArg { name, value },
                arg_start.merge(arg_end),
            ));

            if !self.eat(&TokenKind::Comma) {
                break;
            }
        }

        let end = self.current_span();
        self.expect(&TokenKind::RParen, "`)`");
        let span = callee.span().merge(end);

        Spanned::new(
            Expr::Call {
                callee: Box::new(callee),
                args,
            },
            span,
        )
    }

    /// Parses primary expressions: literals, identifiers, nodes, parens, arrays, blocks.
    fn parse_primary_expr(&mut self) -> Spanned<Expr> {
        let start = self.current_span();

        match self.peek().clone() {
            TokenKind::IntLiteral(n) => {
                self.pos += 1;
                Spanned::new(Expr::IntLiteral(n), start)
            }
            TokenKind::FloatLiteral(n) => {
                self.pos += 1;
                Spanned::new(Expr::FloatLiteral(n), start)
            }
            TokenKind::StringLiteral(s) => {
                self.pos += 1;
                Spanned::new(Expr::StringLiteral(s), start)
            }
            TokenKind::StringInterpStart(s) => {
                self.parse_string_interpolation(s, start)
            }
            TokenKind::True => {
                self.pos += 1;
                Spanned::new(Expr::BoolLiteral(true), start)
            }
            TokenKind::False => {
                self.pos += 1;
                Spanned::new(Expr::BoolLiteral(false), start)
            }
            TokenKind::Void => {
                self.pos += 1;
                Spanned::new(Expr::Void, start)
            }
            TokenKind::Ident(name) => {
                self.pos += 1;
                Spanned::new(Expr::Ident(name), start)
            }
            TokenKind::At => {
                self.parse_node_expr()
            }
            TokenKind::LParen => {
                self.pos += 1;
                let inner = self.parse_expr();
                let end = self.current_span();
                self.expect(&TokenKind::RParen, "`)`");
                Spanned::new(Expr::Paren(Box::new(inner)), start.merge(end))
            }
            TokenKind::LBracket => {
                self.parse_array_literal()
            }
            TokenKind::LBrace => {
                self.parse_block_expr()
            }
            _ => {
                self.diagnostics.push(
                    Diagnostic::error(format!("unexpected token {:?}", self.peek()))
                        .with_label(Label::primary(start, "unexpected token")),
                );
                self.pos += 1;
                Spanned::new(Expr::Error, start)
            }
        }
    }

    // ── String interpolation ────────────────────────────────────────────

    fn parse_string_interpolation(
        &mut self,
        initial_text: String,
        start: Span,
    ) -> Spanned<Expr> {
        self.pos += 1; // consume StringInterpStart
        let mut parts = Vec::new();

        if !initial_text.is_empty() {
            parts.push(StringPart::Lit(initial_text));
        }

        loop {
            // Parse the interpolated expression
            let expr = self.parse_expr();
            parts.push(StringPart::Expr(expr));

            // After the expression, we expect StringInterpMiddle or StringInterpEnd
            match self.peek().clone() {
                TokenKind::StringInterpMiddle(text) => {
                    self.pos += 1;
                    if !text.is_empty() {
                        parts.push(StringPart::Lit(text));
                    }
                    // Continue loop for next interpolation
                }
                TokenKind::StringInterpEnd(text) => {
                    let end = self.current_span();
                    self.pos += 1;
                    if !text.is_empty() {
                        parts.push(StringPart::Lit(text));
                    }
                    Spanned::new(Expr::StringInterp(parts), start.merge(end));
                }
                _ => {
                    // Malformed interpolation — emit diagnostic and bail
                    let end = self.current_span();
                    self.diagnostics.push(
                        Diagnostic::error("unterminated string interpolation")
                            .with_label(Label::primary(end, "expected closing `\"`")),
                    );
                    Spanned::new(Expr::StringInterp(parts), start.merge(end));
                }
            }
        }
    }

    // ── Array literal ───────────────────────────────────────────────────

    fn parse_array_literal(&mut self) -> Spanned<Expr> {
        let start = self.current_span();
        self.pos += 1; // consume `[`

        let mut elements = Vec::new();
        self.skip_newlines();

        while !self.at(&TokenKind::RBracket) && !self.at_eof() {
            elements.push(self.parse_expr());
            if !self.eat(&TokenKind::Comma) {
                self.skip_newlines();
                break;
            }
            self.skip_newlines();
        }

        let end = self.current_span();
        self.expect(&TokenKind::RBracket, "`]`");
        Spanned::new(Expr::Array(elements), start.merge(end))
    }

    // ── Node expression ─────────────────────────────────────────────────

    /// Parses `@name positional... %prop=value... { body }`.
    fn parse_node_expr(&mut self) -> Spanned<Expr> {
        let name = self.parse_node_name();
        let start = name.span();

        let mut positional = Vec::new();
        let mut properties = Vec::new();

        // Parse positional tokens and inline properties until we hit `{`, newline,
        // `}`, or EOF. Positional tokens are expressions that appear between the
        // node name and the body block.
        loop {
            match self.peek() {
                // Body block starts
                TokenKind::LBrace => break,
                // Statement boundary
                TokenKind::Newline | TokenKind::Eof | TokenKind::RBrace => break,
                // Inline property: `%key=value`
                TokenKind::Percent => {
                    let prop = self.parse_property();
                    properties.push(prop);
                }
                // Positional tokens: literals, identifiers, other inline values
                _ => {
                    let expr = self.parse_node_positional_token();
                    positional.push(expr);
                }
            }
        }

        // Parse optional body block
        let body = if self.at(&TokenKind::LBrace) {
            Some(self.parse_block_expr())
        } else {
            None
        };

        let end = body
            .as_ref()
            .map_or_else(|| self.current_span(), Spanned::span);

        Spanned::new(
            Expr::Node(NodeExpr {
                name,
                positional,
                properties,
                body,
            }),
            start.merge(end),
        )
    }

    /// Parses a single positional token in a node expression.
    /// This handles literals, identifiers, dot-paths (like `./public`),
    /// and slash-paths (like `/api/health`).
    fn parse_node_positional_token(&mut self) -> Spanned<Expr> {
        let start = self.current_span();

        match self.peek().clone() {
            TokenKind::IntLiteral(n) => {
                self.pos += 1;
                Spanned::new(Expr::IntLiteral(n), start)
            }
            TokenKind::FloatLiteral(n) => {
                self.pos += 1;
                Spanned::new(Expr::FloatLiteral(n), start)
            }
            TokenKind::StringLiteral(s) => {
                self.pos += 1;
                Spanned::new(Expr::StringLiteral(s), start)
            }
            TokenKind::StringInterpStart(s) => {
                self.parse_string_interpolation(s, start)
            }
            TokenKind::Ident(name) => {
                self.pos += 1;
                Spanned::new(Expr::Ident(name), start)
            }
            TokenKind::Dot => {
                // Path like `./public` — collect as an identifier representing a path
                let mut path_str = String::from(".");
                self.pos += 1;
                // Consume `/` and path segments
                while self.at(&TokenKind::Slash) || matches!(self.peek(), TokenKind::Ident(_)) {
                    match self.peek().clone() {
                        TokenKind::Slash => {
                            path_str.push('/');
                            self.pos += 1;
                        }
                        TokenKind::Ident(s) => {
                            path_str.push_str(&s);
                            self.pos += 1;
                        }
                        _ => break,
                    }
                }
                let end = self.current_span();
                Spanned::new(Expr::Ident(path_str), start.merge(end))
            }
            TokenKind::Slash => {
                // Path like `/api/health` — collect as an identifier representing a path
                let mut path_str = String::from("/");
                self.pos += 1;
                loop {
                    match self.peek().clone() {
                        TokenKind::Ident(s) => {
                            path_str.push_str(&s);
                            self.pos += 1;
                        }
                        TokenKind::Slash => {
                            path_str.push('/');
                            self.pos += 1;
                        }
                        TokenKind::Dot => {
                            path_str.push('.');
                            self.pos += 1;
                        }
                        _ => break,
                    }
                }
                let end = self.current_span();
                Spanned::new(Expr::Ident(path_str), start.merge(end))
            }
            TokenKind::Star => {
                // Wildcard in route context: `@route *`
                self.pos += 1;
                Spanned::new(Expr::Ident("*".to_owned()), start)
            }
            TokenKind::At => {
                // Nested node: `@respond 200 { ... }`
                self.parse_node_expr()
            }
            TokenKind::True => {
                self.pos += 1;
                Spanned::new(Expr::BoolLiteral(true), start)
            }
            TokenKind::False => {
                self.pos += 1;
                Spanned::new(Expr::BoolLiteral(false), start)
            }
            _ => {
                // Unknown positional token — treat as error and advance
                self.diagnostics.push(
                    Diagnostic::error(format!(
                        "unexpected token {:?} in node position",
                        self.peek()
                    ))
                    .with_label(Label::primary(start, "unexpected in node")),
                );
                self.pos += 1;
                Spanned::new(Expr::Error, start)
            }
        }
    }

    // ── Property parsing ────────────────────────────────────────────────

    /// Parses `%key=value`.
    fn parse_property(&mut self) -> Spanned<Property> {
        let start = self.current_span();
        self.pos += 1; // consume `%`

        let name = self
            .expect_ident("property name")
            .unwrap_or_else(|| Spanned::new("<error>".to_owned(), self.current_span()));

        let value = if self.eat(&TokenKind::Eq) {
            self.parse_property_value()
        } else {
            // Boolean shorthand: `%disabled` means `%disabled=true`
            Spanned::new(Expr::BoolLiteral(true), name.span())
        };

        let end = value.span();
        Spanned::new(Property { name, value }, start.merge(end))
    }

    /// Parses the value side of `%key=value`. The value can be a `{expr}` block
    /// or a simple literal/identifier.
    fn parse_property_value(&mut self) -> Spanned<Expr> {
        if self.at(&TokenKind::LBrace) {
            self.parse_block_expr()
        } else {
            self.parse_primary_expr()
        }
    }

    // ── Type expression parsing ─────────────────────────────────────────

    fn parse_type_expr(&mut self) -> Spanned<TypeExpr> {
        let start = self.current_span();

        let base = match self.peek().clone() {
            TokenKind::Ident(name) => {
                self.pos += 1;

                // Check for generic: `Name<A, B>`
                if self.at(&TokenKind::Lt) {
                    self.pos += 1; // consume `<`
                    let mut args = Vec::new();
                    while !self.at(&TokenKind::Gt) && !self.at_eof() {
                        args.push(self.parse_type_expr());
                        if !self.eat(&TokenKind::Comma) {
                            break;
                        }
                    }
                    let end = self.current_span();
                    self.expect(&TokenKind::Gt, "`>`");
                    Spanned::new(
                        TypeExpr::Generic {
                            name: Spanned::new(name, start),
                            args,
                        },
                        start.merge(end),
                    )
                } else {
                    Spanned::new(TypeExpr::Named(name), start)
                }
            }
            TokenKind::At => {
                // Node type: `@html`
                let node_name = self.parse_node_name();
                let span = node_name.span();
                Spanned::new(TypeExpr::Node(node_name), span)
            }
            TokenKind::LParen => {
                // Function type: `(A, B) -> C`
                self.pos += 1;
                let mut params = Vec::new();
                while !self.at(&TokenKind::RParen) && !self.at_eof() {
                    params.push(self.parse_type_expr());
                    if !self.eat(&TokenKind::Comma) {
                        break;
                    }
                }
                self.expect(&TokenKind::RParen, "`)`");

                if self.eat(&TokenKind::Arrow) {
                    let ret = self.parse_type_expr();
                    let end = ret.span();
                    Spanned::new(
                        TypeExpr::Function {
                            params,
                            ret: Box::new(ret),
                        },
                        start.merge(end),
                    )
                } else {
                    // Parenthesized single type if one param, or error
                    if params.len() == 1 {
                        params.into_iter().next().unwrap()
                    } else {
                        self.diagnostics.push(
                            Diagnostic::error("expected `->` for function type")
                                .with_label(Label::primary(self.current_span(), "expected `->`")),
                        );
                        Spanned::new(TypeExpr::Error, start)
                    }
                }
            }
            _ => {
                self.diagnostics.push(
                    Diagnostic::error(format!("expected type, found {:?}", self.peek()))
                        .with_label(Label::primary(start, "expected type")),
                );
                self.pos += 1;
                Spanned::new(TypeExpr::Error, start)
            }
        };

        // Check for nullable suffix: `T?`
        if self.at(&TokenKind::Question) {
            let end = self.current_span();
            self.pos += 1;
            Spanned::new(TypeExpr::Nullable(Box::new(base)), start.merge(end))
        } else {
            base
        }
    }
}

// ── AST pretty-printer ─────────────────────────────────────────────────────

/// Formats an AST `Module` as an indented string for the `dump ast` command.
pub fn dump_ast(module: &Module) -> String {
    let mut out = String::new();
    dump_module(module, &mut out, 0);
    out
}

fn indent(out: &mut String, depth: usize) {
    for _ in 0..depth {
        out.push_str("  ");
    }
}

fn dump_module(module: &Module, out: &mut String, depth: usize) {
    indent(out, depth);
    out.push_str("Module\n");
    for item in &module.items {
        dump_item(item.node(), out, depth + 1);
    }
}

fn dump_item(item: &Item, out: &mut String, depth: usize) {
    match item {
        Item::Import(imp) => {
            indent(out, depth);
            let path: Vec<&str> = imp.path.iter().map(|s| s.node().as_str()).collect();
            out.push_str(&format!("Import {}\n", path.join(".")));
            if !imp.names.is_empty() {
                indent(out, depth + 1);
                let names: Vec<&str> = imp.names.iter().map(|s| s.node().as_str()).collect();
                out.push_str(&format!("names: {{{}}}\n", names.join(", ")));
            }
            if let Some(alias) = &imp.alias {
                indent(out, depth + 1);
                out.push_str(&format!("as {}\n", alias.node()));
            }
        }
        Item::Function(func) => {
            indent(out, depth);
            let vis = if func.is_pub { "pub " } else { "" };
            let async_kw = if func.is_async { "async " } else { "" };
            out.push_str(&format!(
                "{}{}Function {}\n",
                vis,
                async_kw,
                func.name.node()
            ));
            for param in &func.params {
                dump_param(param.node(), out, depth + 1);
            }
            if let Some(ret) = &func.return_type {
                indent(out, depth + 1);
                out.push_str(&format!("returns: {}\n", format_type(ret.node())));
            }
            indent(out, depth + 1);
            out.push_str("body:\n");
            dump_expr(func.body.node(), out, depth + 2);
        }
        Item::Define(def) => {
            indent(out, depth);
            let vis = if def.is_pub { "pub " } else { "" };
            out.push_str(&format!("{}Define {}\n", vis, def.name.node()));
            for param in &def.params {
                dump_param(param.node(), out, depth + 1);
            }
            if let Some(domain) = &def.return_domain {
                indent(out, depth + 1);
                out.push_str(&format!("domain: @{}\n", domain.node()));
            }
            indent(out, depth + 1);
            out.push_str("body:\n");
            dump_expr(def.body.node(), out, depth + 2);
        }
        Item::Struct(s) => {
            indent(out, depth);
            let vis = if s.is_pub { "pub " } else { "" };
            out.push_str(&format!("{}Struct {}\n", vis, s.name.node()));
            for field in &s.fields {
                indent(out, depth + 1);
                out.push_str(&format!(
                    "{}: {}\n",
                    field.node().name.node(),
                    format_type(field.node().ty.node())
                ));
            }
        }
        Item::Enum(e) => {
            indent(out, depth);
            let vis = if e.is_pub { "pub " } else { "" };
            out.push_str(&format!("{}Enum {}\n", vis, e.name.node()));
            for variant in &e.variants {
                indent(out, depth + 1);
                let v = variant.node();
                if v.fields.is_empty() {
                    out.push_str(&format!("{}\n", v.name.node()));
                } else {
                    let types: Vec<String> =
                        v.fields.iter().map(|t| format_type(t.node())).collect();
                    out.push_str(&format!("{}({})\n", v.name.node(), types.join(", ")));
                }
            }
        }
        Item::TypeAlias(ta) => {
            indent(out, depth);
            let vis = if ta.is_pub { "pub " } else { "" };
            out.push_str(&format!(
                "{}TypeAlias {} = {}\n",
                vis,
                ta.name.node(),
                format_type(ta.ty.node())
            ));
        }
        Item::Binding(b) => {
            dump_binding(b, out, depth);
        }
        Item::Stmt(stmt) => {
            dump_stmt(stmt, out, depth);
        }
        Item::Error => {
            indent(out, depth);
            out.push_str("<error>\n");
        }
    }
}

fn dump_param(param: &Param, out: &mut String, depth: usize) {
    indent(out, depth);
    out.push_str(&format!("param {}", param.name.node()));
    if let Some(ty) = &param.ty {
        out.push_str(&format!(": {}", format_type(ty.node())));
    }
    if param.default.is_some() {
        out.push_str(" = <default>");
    }
    out.push('\n');
}

fn dump_binding(b: &BindingStmt, out: &mut String, depth: usize) {
    indent(out, depth);
    let vis = if b.is_pub { "pub " } else { "" };
    let kind = if b.is_const { "const" } else { "let" };
    let modifiers = match (b.is_mut, b.is_sig) {
        (true, true) => " mut sig",
        (true, false) => " mut",
        (false, true) => " sig",
        (false, false) => "",
    };
    out.push_str(&format!("{vis}{kind}{modifiers} {}", b.name.node()));
    if let Some(ty) = &b.ty {
        out.push_str(&format!(": {}", format_type(ty.node())));
    }
    out.push('\n');
    if let Some(value) = &b.value {
        indent(out, depth + 1);
        out.push_str("= ");
        dump_expr_inline(value.node(), out);
        out.push('\n');
    }
}

fn dump_stmt(stmt: &Stmt, out: &mut String, depth: usize) {
    match stmt {
        Stmt::Binding(b) => dump_binding(b, out, depth),
        Stmt::Return(expr) => {
            indent(out, depth);
            out.push_str("Return");
            if let Some(e) = expr {
                out.push(' ');
                dump_expr_inline(e.node(), out);
            }
            out.push('\n');
        }
        Stmt::If(if_stmt) => {
            indent(out, depth);
            out.push_str("If\n");
            indent(out, depth + 1);
            out.push_str("condition: ");
            dump_expr_inline(if_stmt.condition.node(), out);
            out.push('\n');
            indent(out, depth + 1);
            out.push_str("then:\n");
            dump_expr(if_stmt.then_body.node(), out, depth + 2);
            if let Some(else_body) = &if_stmt.else_body {
                indent(out, depth + 1);
                out.push_str("else:\n");
                dump_expr(else_body.node(), out, depth + 2);
            }
        }
        Stmt::For(for_stmt) => {
            indent(out, depth);
            out.push_str(&format!("For {} of\n", for_stmt.binding.node()));
            indent(out, depth + 1);
            out.push_str("iterable: ");
            dump_expr_inline(for_stmt.iterable.node(), out);
            out.push('\n');
            indent(out, depth + 1);
            out.push_str("body:\n");
            dump_expr(for_stmt.body.node(), out, depth + 2);
        }
        Stmt::While(while_stmt) => {
            indent(out, depth);
            out.push_str("While\n");
            indent(out, depth + 1);
            out.push_str("condition: ");
            dump_expr_inline(while_stmt.condition.node(), out);
            out.push('\n');
            indent(out, depth + 1);
            out.push_str("body:\n");
            dump_expr(while_stmt.body.node(), out, depth + 2);
        }
        Stmt::Expr(expr) => {
            dump_expr(expr.node(), out, depth);
        }
        Stmt::Error => {
            indent(out, depth);
            out.push_str("<error>\n");
        }
    }
}

fn dump_expr(expr: &Expr, out: &mut String, depth: usize) {
    match expr {
        Expr::Block(stmts) => {
            indent(out, depth);
            out.push_str("Block\n");
            for stmt in stmts {
                dump_stmt(stmt.node(), out, depth + 1);
            }
        }
        Expr::Object(fields) => {
            indent(out, depth);
            out.push_str("Object\n");
            for field in fields {
                indent(out, depth + 1);
                out.push_str(&format!("{}: ", field.node().key.node()));
                dump_expr_inline(field.node().value.node(), out);
                out.push('\n');
            }
        }
        Expr::Node(node) => {
            indent(out, depth);
            out.push_str(&format!("@{}", node.name.node()));
            for pos in &node.positional {
                out.push(' ');
                dump_expr_inline(pos.node(), out);
            }
            out.push('\n');
            for prop in &node.properties {
                indent(out, depth + 1);
                out.push_str(&format!("%{}=", prop.node().name.node()));
                dump_expr_inline(prop.node().value.node(), out);
                out.push('\n');
            }
            if let Some(body) = &node.body {
                dump_expr(body.node(), out, depth + 1);
            }
        }
        _ => {
            indent(out, depth);
            dump_expr_inline(expr, out);
            out.push('\n');
        }
    }
}

fn dump_expr_inline(expr: &Expr, out: &mut String) {
    match expr {
        Expr::IntLiteral(n) => out.push_str(&n.to_string()),
        Expr::FloatLiteral(n) => out.push_str(&n.to_string()),
        Expr::StringLiteral(s) => out.push_str(&format!("\"{s}\"")),
        Expr::StringInterp(parts) => {
            out.push('"');
            for part in parts {
                match part {
                    StringPart::Lit(s) => out.push_str(s),
                    StringPart::Expr(e) => {
                        out.push('{');
                        dump_expr_inline(e.node(), out);
                        out.push('}');
                    }
                }
            }
            out.push('"');
        }
        Expr::BoolLiteral(b) => out.push_str(&b.to_string()),
        Expr::Void => out.push_str("void"),
        Expr::Ident(name) => out.push_str(name),
        Expr::Binary { left, op, right } => {
            out.push('(');
            dump_expr_inline(left.node(), out);
            out.push_str(&format!(" {} ", op.node()));
            dump_expr_inline(right.node(), out);
            out.push(')');
        }
        Expr::Unary { op, operand } => {
            out.push_str(&format!("{}", op.node()));
            dump_expr_inline(operand.node(), out);
        }
        Expr::Assign { target, op, value } => {
            dump_expr_inline(target.node(), out);
            out.push_str(&format!(" {} ", op.node()));
            dump_expr_inline(value.node(), out);
        }
        Expr::Call { callee, args } => {
            dump_expr_inline(callee.node(), out);
            out.push('(');
            for (i, arg) in args.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                if let Some(name) = &arg.node().name {
                    out.push_str(&format!("{}=", name.node()));
                }
                dump_expr_inline(arg.node().value.node(), out);
            }
            out.push(')');
        }
        Expr::Field { object, field } => {
            dump_expr_inline(object.node(), out);
            out.push('.');
            out.push_str(field.node());
        }
        Expr::Index { object, index } => {
            dump_expr_inline(object.node(), out);
            out.push('[');
            dump_expr_inline(index.node(), out);
            out.push(']');
        }
        Expr::Paren(inner) => {
            out.push('(');
            dump_expr_inline(inner.node(), out);
            out.push(')');
        }
        Expr::Await(inner) => {
            out.push_str("await ");
            dump_expr_inline(inner.node(), out);
        }
        Expr::Array(elements) => {
            out.push('[');
            for (i, e) in elements.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                dump_expr_inline(e.node(), out);
            }
            out.push(']');
        }
        Expr::Node(node) => {
            out.push_str(&format!("@{}", node.name.node()));
            for pos in &node.positional {
                out.push(' ');
                dump_expr_inline(pos.node(), out);
            }
            if node.body.is_some() {
                out.push_str(" { ... }");
            }
        }
        Expr::Object(fields) => {
            out.push_str("{ ");
            for (i, field) in fields.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                out.push_str(&format!("{}: ", field.node().key.node()));
                dump_expr_inline(field.node().value.node(), out);
            }
            out.push_str(" }");
        }
        Expr::Block(stmts) => {
            out.push_str(&format!("{{ {} stmts }}", stmts.len()));
        }
        Expr::Error => out.push_str("<error>"),
    }
}

fn format_type(ty: &TypeExpr) -> String {
    match ty {
        TypeExpr::Named(name) => name.clone(),
        TypeExpr::Nullable(inner) => format!("{}?", format_type(inner.node())),
        TypeExpr::Generic { name, args } => {
            let arg_strs: Vec<String> = args.iter().map(|a| format_type(a.node())).collect();
            format!("{}<{}>", name.node(), arg_strs.join(", "))
        }
        TypeExpr::Function { params, ret } => {
            let param_strs: Vec<String> = params.iter().map(|p| format_type(p.node())).collect();
            format!("({}) -> {}", param_strs.join(", "), format_type(ret.node()))
        }
        TypeExpr::Node(name) => format!("@{name}"),
        TypeExpr::Error => "<error>".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use orv_span::FileId;

    use crate::lexer::Lexer;

    use super::*;

    fn parse_source(source: &str) -> (Module, DiagnosticBag) {
        let file = FileId::new(0);
        let lexer = Lexer::new(source, file);
        let (tokens, _lex_diags) = lexer.tokenize();
        parse(tokens)
    }

    #[test]
    fn parse_empty_module() {
        let (module, diags) = parse_source("");
        assert!(module.items.is_empty());
        assert!(!diags.has_errors());
    }

    #[test]
    fn parse_simple_node() {
        let (module, diags) = parse_source("@io.out \"Hello, orv!\"");
        assert!(!diags.has_errors(), "should parse without errors");
        assert_eq!(module.items.len(), 1);

        if let Item::Stmt(Stmt::Expr(expr)) = module.items[0].node() {
            if let Expr::Node(node) = expr.node() {
                assert_eq!(node.name.node().to_string(), "io.out");
                assert_eq!(node.positional.len(), 1);
            } else {
                panic!("expected Node expression");
            }
        } else {
            panic!("expected statement item");
        }
    }

    #[test]
    fn parse_let_binding() {
        let (module, diags) = parse_source("let x: i32 = 42");
        assert!(!diags.has_errors());
        assert_eq!(module.items.len(), 1);
    }

    #[test]
    fn parse_let_sig_binding() {
        let (module, diags) = parse_source("let sig count: i32 = 0");
        assert!(!diags.has_errors());

        if let Item::Stmt(Stmt::Binding(b)) = module.items[0].node() {
            assert!(b.is_sig);
            assert_eq!(b.name.node(), "count");
        } else {
            panic!("expected binding statement");
        }
    }

    #[test]
    fn parse_pub_define() {
        let (module, diags) = parse_source(
            "pub define CounterPage() -> @html {\n  @text \"hello\"\n}",
        );
        assert!(!diags.has_errors(), "errors: {:?}", diags.iter().collect::<Vec<_>>());
        assert_eq!(module.items.len(), 1);

        if let Item::Define(def) = module.items[0].node() {
            assert!(def.is_pub);
            assert_eq!(def.name.node(), "CounterPage");
            assert!(def.return_domain.is_some());
            assert_eq!(def.return_domain.as_ref().unwrap().node().to_string(), "html");
        } else {
            panic!("expected Define item");
        }
    }

    #[test]
    fn parse_binary_expr() {
        let (module, diags) = parse_source("1 + 2 * 3");
        assert!(!diags.has_errors());

        // Should parse as `1 + (2 * 3)` due to precedence
        if let Item::Stmt(Stmt::Expr(expr)) = module.items[0].node() {
            if let Expr::Binary { op, .. } = expr.node() {
                assert_eq!(*op.node(), BinOp::Add);
            } else {
                panic!("expected binary expression");
            }
        }
    }

    #[test]
    fn parse_node_with_property() {
        let (module, diags) = parse_source("@button \"+\" %onClick={count += 1}");
        assert!(!diags.has_errors(), "errors: {:?}", diags.iter().collect::<Vec<_>>());

        if let Item::Stmt(Stmt::Expr(expr)) = module.items[0].node() {
            if let Expr::Node(node) = expr.node() {
                assert_eq!(node.name.node().to_string(), "button");
                assert_eq!(node.positional.len(), 1);
                assert_eq!(node.properties.len(), 1);
                assert_eq!(node.properties[0].node().name.node(), "onClick");
            } else {
                panic!("expected Node expression");
            }
        }
    }

    #[test]
    fn parse_object_literal_in_block() {
        let (module, diags) = parse_source("@respond 200 { \"status\": \"ok\" }");
        assert!(!diags.has_errors(), "errors: {:?}", diags.iter().collect::<Vec<_>>());
    }

    #[test]
    fn parse_import() {
        let (module, diags) = parse_source("import components.{Button, Input}");
        assert!(!diags.has_errors());

        if let Item::Import(imp) = module.items[0].node() {
            assert_eq!(imp.path.len(), 1);
            assert_eq!(imp.path[0].node(), "components");
            assert_eq!(imp.names.len(), 2);
        } else {
            panic!("expected Import item");
        }
    }

    #[test]
    fn parse_struct() {
        let (module, diags) = parse_source("pub struct User {\n  name: string\n  age: i32\n}");
        assert!(!diags.has_errors());

        if let Item::Struct(s) = module.items[0].node() {
            assert!(s.is_pub);
            assert_eq!(s.name.node(), "User");
            assert_eq!(s.fields.len(), 2);
        } else {
            panic!("expected Struct item");
        }
    }

    #[test]
    fn parse_string_interpolation() {
        let (module, diags) = parse_source("@text \"{count}\"");
        assert!(!diags.has_errors(), "errors: {:?}", diags.iter().collect::<Vec<_>>());
    }

    #[test]
    fn parse_error_recovery_missing_brace() {
        let (_module, diags) = parse_source("@div {\n  @text \"hello\"\n");
        // Should have an error about missing `}` but not panic
        assert!(diags.has_errors());
    }

    #[test]
    fn dump_ast_produces_output() {
        let (module, _) = parse_source("@io.out \"Hello, orv!\"");
        let output = dump_ast(&module);
        assert!(output.contains("Module"));
        assert!(output.contains("@io.out"));
        assert!(output.contains("Hello, orv!"));
    }
}
```

- [ ] **Step 2: Verify**

Run: `cargo test -p orv-syntax`
Expected: All lexer tests plus 13 new parser tests pass.

Run: `cargo clippy -p orv-syntax`
Expected: Clean.

- [ ] **Step 3: Commit**

```bash
git add crates/orv-syntax/src/parser.rs crates/orv-syntax/src/lib.rs
git commit -m "feat(syntax): add recursive-descent parser with Pratt expression parsing"
```

---

### Task 3: Create parser fixture files

**Files:**
- Create: `fixtures/parser/hello.orv`
- Create: `fixtures/parser/counter.orv`
- Create: `fixtures/parser/server-basic.orv`

- [ ] **Step 1: Create `fixtures/parser/hello.orv`**

```orv
@io.out "Hello, orv!"
```

- [ ] **Step 2: Create `fixtures/parser/counter.orv`**

```orv
pub define CounterPage() -> @html {
  @body {
    let sig count: i32 = 0
    @text "{count}"
    @button "+" %onClick={count += 1}
    @button "-" %onClick={count -= 1}
  }
}
```

- [ ] **Step 3: Create `fixtures/parser/server-basic.orv`**

```orv
@server {
  @listen 8080

  @route GET /api/health {
    @respond 200 { "status": "ok" }
  }

  @route GET / {
    @serve ./public
  }
}
```

- [ ] **Step 4: Verify**

Run: `ls fixtures/parser/`
Expected: `hello.orv  counter.orv  server-basic.orv`

- [ ] **Step 5: Commit**

```bash
git add fixtures/parser/
git commit -m "test(fixtures): add parser fixture files for hello, counter, server-basic"
```

---

### Task 4: Fixture-based integration tests for the parser

**Files:**
- Create: `crates/orv-syntax/tests/parser_fixtures.rs`

- [ ] **Step 1: Create `crates/orv-syntax/tests/parser_fixtures.rs`**

```rust
use std::path::PathBuf;

use orv_span::FileId;
use orv_syntax::lexer::Lexer;
use orv_syntax::parser;

fn fixtures_root() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("fixtures")
}

fn parse_fixture(dir: &str, name: &str) -> (orv_syntax::ast::Module, bool) {
    let path = fixtures_root().join(dir).join(name);
    let source = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
    let file = FileId::new(0);
    let lexer = Lexer::new(&source, file);
    let (tokens, lex_diags) = lexer.tokenize();
    assert!(
        !lex_diags.has_errors(),
        "lexer errors in {name}: {lex_diags:?}"
    );
    let (module, parse_diags) = parser::parse(tokens);
    (module, parse_diags.has_errors())
}

#[test]
fn hello_fixture_parses_cleanly() {
    let (module, has_errors) = parse_fixture("parser", "hello.orv");
    assert!(!has_errors, "hello.orv should parse without errors");
    assert_eq!(module.items.len(), 1, "hello.orv should have 1 item");

    // Verify it's a node expression @io.out with a string positional
    if let orv_syntax::ast::Item::Stmt(orv_syntax::ast::Stmt::Expr(expr)) =
        module.items[0].node()
    {
        if let orv_syntax::ast::Expr::Node(node) = expr.node() {
            assert_eq!(node.name.node().to_string(), "io.out");
            assert_eq!(node.positional.len(), 1);
        } else {
            panic!("expected Node expression in hello.orv");
        }
    } else {
        panic!("expected expression statement in hello.orv");
    }
}

#[test]
fn counter_fixture_parses_cleanly() {
    let (module, has_errors) = parse_fixture("parser", "counter.orv");
    assert!(!has_errors, "counter.orv should parse without errors");
    assert_eq!(module.items.len(), 1, "counter.orv should have 1 item");

    // Should be a pub define
    if let orv_syntax::ast::Item::Define(def) = module.items[0].node() {
        assert!(def.is_pub);
        assert_eq!(def.name.node(), "CounterPage");
        assert!(def.return_domain.is_some());
    } else {
        panic!("expected Define item in counter.orv");
    }
}

#[test]
fn server_basic_fixture_parses_cleanly() {
    let (module, has_errors) = parse_fixture("parser", "server-basic.orv");
    assert!(!has_errors, "server-basic.orv should parse without errors");
    assert_eq!(
        module.items.len(),
        1,
        "server-basic.orv should have 1 top-level item"
    );

    // Should be a @server node
    if let orv_syntax::ast::Item::Stmt(orv_syntax::ast::Stmt::Expr(expr)) =
        module.items[0].node()
    {
        if let orv_syntax::ast::Expr::Node(node) = expr.node() {
            assert_eq!(node.name.node().to_string(), "server");
            assert!(node.body.is_some());
        } else {
            panic!("expected Node expression in server-basic.orv");
        }
    } else {
        panic!("expected expression statement in server-basic.orv");
    }
}

#[test]
fn ok_fixtures_also_parse_cleanly() {
    // The fixtures/ok/ directory contains the canonical test files.
    // They should all parse without errors.
    for name in &["hello.orv", "counter.orv", "server-basic.orv"] {
        let (_, has_errors) = parse_fixture("ok", name);
        assert!(!has_errors, "fixtures/ok/{name} should parse without errors");
    }
}

#[test]
fn dump_ast_roundtrip() {
    let (module, _) = parse_fixture("parser", "hello.orv");
    let output = parser::dump_ast(&module);
    assert!(output.contains("Module"), "dump should contain Module");
    assert!(
        output.contains("@io.out"),
        "dump should contain @io.out node"
    );
}
```

- [ ] **Step 2: Verify**

Run: `cargo test -p orv-syntax`
Expected: All tests pass including the 5 new fixture tests.

- [ ] **Step 3: Commit**

```bash
git add crates/orv-syntax/tests/parser_fixtures.rs
git commit -m "test(syntax): add parser fixture integration tests for all three reference files"
```

---

### Task 5: Add `dump ast` CLI command

**Files:**
- Modify: `crates/orv-cli/src/main.rs`

- [ ] **Step 1: Add the `Ast` variant to `DumpTarget` enum**

In `crates/orv-cli/src/main.rs`, find the `DumpTarget` enum:

```rust
#[derive(clap::Subcommand)]
enum DumpTarget {
    /// Dump source file metadata (file id, line count, spans)
    Source {
        /// Path to the .orv source file
        file: PathBuf,
    },
    /// Dump token stream for a source file
    Tokens {
        /// Path to the .orv source file
        file: PathBuf,
    },
}
```

Replace it with:

```rust
#[derive(clap::Subcommand)]
enum DumpTarget {
    /// Dump source file metadata (file id, line count, spans)
    Source {
        /// Path to the .orv source file
        file: PathBuf,
    },
    /// Dump token stream for a source file
    Tokens {
        /// Path to the .orv source file
        file: PathBuf,
    },
    /// Dump the AST for a source file
    Ast {
        /// Path to the .orv source file
        file: PathBuf,
    },
}
```

- [ ] **Step 2: Add the `Ast` match arm in the `main` function**

In `crates/orv-cli/src/main.rs`, find the `Dump` match arm:

```rust
        Some(Commands::Dump { target }) => match target {
            DumpTarget::Source { file } => {
                run_dump_source(&file);
            }
            DumpTarget::Tokens { file } => {
                run_dump_tokens(&file);
            }
        },
```

Replace it with:

```rust
        Some(Commands::Dump { target }) => match target {
            DumpTarget::Source { file } => {
                run_dump_source(&file);
            }
            DumpTarget::Tokens { file } => {
                run_dump_tokens(&file);
            }
            DumpTarget::Ast { file } => {
                run_dump_ast(&file);
            }
        },
```

- [ ] **Step 3: Add the `run_dump_ast` function**

Add this function at the end of `crates/orv-cli/src/main.rs`, after `run_dump_source`:

```rust
fn run_dump_ast(path: &PathBuf) {
    let (loader, file_id) = load_source(path);

    if let Some(id) = file_id {
        let source_map = loader.source_map();
        let source = source_map.source(id);
        let lexer = orv_syntax::lexer::Lexer::new(source, id);
        let (tokens, lex_diags) = lexer.tokenize();

        if lex_diags.has_errors() {
            let diag_vec: Vec<_> = lex_diags.into_vec();
            render_diagnostics(source_map, &diag_vec);
        }

        let (module, parse_diags) = orv_syntax::parser::parse(tokens);

        if parse_diags.has_errors() {
            let diag_vec: Vec<_> = parse_diags.into_vec();
            render_diagnostics(source_map, &diag_vec);
        }

        let output = orv_syntax::parser::dump_ast(&module);
        print!("{output}");
    } else {
        let (source_map, diagnostics) = loader.into_parts();
        render_diagnostics(&source_map, &diagnostics.into_vec());
        process::exit(1);
    }
}
```

- [ ] **Step 4: Verify**

Run: `cargo build -p orv-cli`
Expected: Clean compilation.

Run: `cargo run -p orv-cli -- dump ast fixtures/ok/hello.orv`
Expected output (approximately):
```
Module
  @io.out "Hello, orv!"
```

Run: `cargo run -p orv-cli -- dump ast fixtures/ok/counter.orv`
Expected: Shows the parsed AST for the counter page define.

Run: `cargo run -p orv-cli -- dump ast fixtures/ok/server-basic.orv`
Expected: Shows the parsed AST for the server block.

Run: `cargo clippy -p orv-cli`
Expected: Clean.

- [ ] **Step 5: Commit**

```bash
git add crates/orv-cli/src/main.rs
git commit -m "feat(cli): add 'dump ast' command for AST inspection"
```

---

### Task 6: Edge case handling and error recovery improvements

**Files:**
- Modify: `crates/orv-syntax/src/parser.rs`

This task adds targeted tests for edge cases and ensures the parser handles them gracefully.

- [ ] **Step 1: Add edge case tests to the `tests` module in `parser.rs`**

Append these tests to the `mod tests` block at the bottom of `crates/orv-syntax/src/parser.rs`:

```rust
    #[test]
    fn parse_nested_nodes() {
        let (module, diags) = parse_source("@div {\n  @text \"hello\"\n  @text \"world\"\n}");
        assert!(!diags.has_errors(), "errors: {:?}", diags.iter().collect::<Vec<_>>());

        if let Item::Stmt(Stmt::Expr(expr)) = module.items[0].node() {
            if let Expr::Node(node) = expr.node() {
                assert_eq!(node.name.node().to_string(), "div");
                assert!(node.body.is_some());
            } else {
                panic!("expected Node expression");
            }
        }
    }

    #[test]
    fn parse_empty_block() {
        let (module, diags) = parse_source("@div {\n}");
        assert!(!diags.has_errors());
        assert_eq!(module.items.len(), 1);
    }

    #[test]
    fn parse_multiple_items() {
        let (module, diags) = parse_source("let x = 1\nlet y = 2\n");
        assert!(!diags.has_errors());
        assert_eq!(module.items.len(), 2);
    }

    #[test]
    fn parse_function_declaration() {
        let (module, diags) =
            parse_source("function add(a: i32, b: i32): i32 -> {\n  a + b\n}");
        assert!(!diags.has_errors(), "errors: {:?}", diags.iter().collect::<Vec<_>>());

        if let Item::Function(func) = module.items[0].node() {
            assert_eq!(func.name.node(), "add");
            assert_eq!(func.params.len(), 2);
            assert!(func.return_type.is_some());
        } else {
            panic!("expected Function item");
        }
    }

    #[test]
    fn parse_enum_declaration() {
        let (module, diags) = parse_source("enum Status {\n  Ok(i32)\n  Err(string)\n}");
        assert!(!diags.has_errors());

        if let Item::Enum(e) = module.items[0].node() {
            assert_eq!(e.name.node(), "Status");
            assert_eq!(e.variants.len(), 2);
        } else {
            panic!("expected Enum item");
        }
    }

    #[test]
    fn parse_type_alias() {
        let (module, diags) = parse_source("type UserId = i32");
        assert!(!diags.has_errors());

        if let Item::TypeAlias(ta) = module.items[0].node() {
            assert_eq!(ta.name.node(), "UserId");
        } else {
            panic!("expected TypeAlias item");
        }
    }

    #[test]
    fn parse_nullable_type() {
        let (module, diags) = parse_source("let x: string? = void");
        assert!(!diags.has_errors());

        if let Item::Stmt(Stmt::Binding(b)) = module.items[0].node() {
            if let Some(ty) = &b.ty {
                assert!(matches!(ty.node(), TypeExpr::Nullable(_)));
            } else {
                panic!("expected type annotation");
            }
        }
    }

    #[test]
    fn parse_generic_type() {
        let (module, diags) = parse_source("let x: Vec<i32> = void");
        assert!(!diags.has_errors());

        if let Item::Stmt(Stmt::Binding(b)) = module.items[0].node() {
            if let Some(ty) = &b.ty {
                assert!(matches!(ty.node(), TypeExpr::Generic { .. }));
            } else {
                panic!("expected type annotation");
            }
        }
    }

    #[test]
    fn parse_if_else() {
        let (module, diags) =
            parse_source("if x > 0 {\n  @io.out \"positive\"\n} else {\n  @io.out \"non-positive\"\n}");
        assert!(!diags.has_errors(), "errors: {:?}", diags.iter().collect::<Vec<_>>());

        if let Item::Stmt(Stmt::If(if_stmt)) = module.items[0].node() {
            assert!(if_stmt.else_body.is_some());
        } else {
            panic!("expected If statement");
        }
    }

    #[test]
    fn parse_for_loop() {
        let (module, diags) = parse_source("for item of items {\n  @io.out item\n}");
        assert!(!diags.has_errors(), "errors: {:?}", diags.iter().collect::<Vec<_>>());

        if let Item::Stmt(Stmt::For(for_stmt)) = module.items[0].node() {
            assert_eq!(for_stmt.binding.node(), "item");
        } else {
            panic!("expected For statement");
        }
    }

    #[test]
    fn parse_node_with_path() {
        let (module, diags) = parse_source("@serve ./public");
        assert!(!diags.has_errors(), "errors: {:?}", diags.iter().collect::<Vec<_>>());

        if let Item::Stmt(Stmt::Expr(expr)) = module.items[0].node() {
            if let Expr::Node(node) = expr.node() {
                assert_eq!(node.name.node().to_string(), "serve");
                assert_eq!(node.positional.len(), 1);
            } else {
                panic!("expected Node expression");
            }
        }
    }

    #[test]
    fn parse_node_with_route_path() {
        let (module, diags) = parse_source("@route GET /api/health {\n}");
        assert!(!diags.has_errors(), "errors: {:?}", diags.iter().collect::<Vec<_>>());

        if let Item::Stmt(Stmt::Expr(expr)) = module.items[0].node() {
            if let Expr::Node(node) = expr.node() {
                assert_eq!(node.name.node().to_string(), "route");
                // Should have GET and /api/health as positional tokens
                assert!(node.positional.len() >= 2);
            } else {
                panic!("expected Node expression");
            }
        }
    }

    #[test]
    fn parse_recovery_does_not_panic() {
        // Various malformed inputs that should not panic
        let cases = vec![
            "@",
            "@ {",
            "let",
            "let =",
            "define",
            "function ()",
            "{ { { }",
            "pub",
            "pub pub",
            "import",
            "struct {",
            "enum Foo",
        ];
        for case in cases {
            let (_, _) = parse_source(case);
            // Success = no panic
        }
    }

    #[test]
    fn parse_await_expression() {
        let (module, diags) = parse_source("let x = await fetchData()");
        assert!(!diags.has_errors(), "errors: {:?}", diags.iter().collect::<Vec<_>>());

        if let Item::Stmt(Stmt::Binding(b)) = module.items[0].node() {
            if let Some(val) = &b.value {
                assert!(matches!(val.node(), Expr::Await(_)));
            }
        }
    }

    #[test]
    fn parse_array_literal() {
        let (module, diags) = parse_source("let xs = [1, 2, 3]");
        assert!(!diags.has_errors());

        if let Item::Stmt(Stmt::Binding(b)) = module.items[0].node() {
            if let Some(val) = &b.value {
                if let Expr::Array(elements) = val.node() {
                    assert_eq!(elements.len(), 3);
                } else {
                    panic!("expected Array expression");
                }
            }
        }
    }
```

- [ ] **Step 2: Verify**

Run: `cargo test -p orv-syntax`
Expected: All tests pass (lexer tests + ~30 parser tests).

Run: `cargo clippy -p orv-syntax`
Expected: Clean.

- [ ] **Step 3: Commit**

```bash
git add crates/orv-syntax/src/parser.rs
git commit -m "test(syntax): add edge case and recovery tests for parser"
```

---

### Task 7: Full workspace verification and final polish

**Files:**
- No new files. Verification only.

- [ ] **Step 1: Run full workspace build**

Run: `cargo build --workspace`
Expected: Clean compilation with no errors.

- [ ] **Step 2: Run full workspace tests**

Run: `cargo test --workspace`
Expected: All tests pass.

- [ ] **Step 3: Run workspace clippy**

Run: `cargo clippy --workspace`
Expected: No deny-level or error-level diagnostics.

- [ ] **Step 4: Run the CLI dump commands on all three fixtures**

Run:
```bash
cargo run -p orv-cli -- dump ast fixtures/ok/hello.orv
cargo run -p orv-cli -- dump ast fixtures/ok/counter.orv
cargo run -p orv-cli -- dump ast fixtures/ok/server-basic.orv
```
Expected: All three produce readable AST output with no error diagnostics.

- [ ] **Step 5: Verify error recovery on a malformed file**

Create a temporary test:
```bash
echo '@div {\n  @text "hello"\n  broken %%% stuff\n' | cargo run -p orv-cli -- dump ast /dev/stdin
```
Expected: Produces partial AST output with error diagnostics, does not panic.

- [ ] **Step 6: Commit (if any polish was needed)**

```bash
git add -A
git commit -m "chore(syntax): polish parser and fix clippy warnings from full workspace build"
```

---

## Self-Review Checklist

- [ ] All 3 fixture files (`hello.orv`, `counter.orv`, `server-basic.orv`) parse without errors
- [ ] `orv dump ast` CLI command works
- [ ] Error recovery does not panic on any malformed input
- [ ] AST types cover: module, items (import/function/define/struct/enum/type), statements (let/const/return/if/for/while/expr), expressions (literals/binary/unary/call/field/index/node/block/object/array/paren/await/assign/string-interp), types (named/nullable/generic/function/node), node syntax (@name positional %prop=value { body })
- [ ] Parser uses `Spanned<T>` for all source-located data
- [ ] DiagnosticBag collects syntax errors without halting
- [ ] Object literal vs code block disambiguation uses first-token rule (`Ident Colon` = object)
- [ ] Operator precedence: assignment < pipe < or < and < equality < comparison < add/sub < mul/div < unary < postfix
- [ ] No `unsafe` code
- [ ] All clippy lints clean
- [ ] No debug/temporary code left behind
