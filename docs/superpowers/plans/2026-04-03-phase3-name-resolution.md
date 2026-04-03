# Phase 3: Name Resolution and Module Graph — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Resolve every identifier in a parsed orv AST to its declaration site. Build a scope tree, populate a symbol table, and report diagnostics for unresolved names, duplicate declarations, and import errors. No type checking — just name binding.

**Architecture:** A new library crate `orv-resolve` takes a parsed `Module` and produces a `ResolveResult` containing a `SymbolTable` (flat registry of all named declarations) and a `ScopeMap` (tree of lexical scopes, each mapping names to symbol IDs). Resolution is two-pass: Pass 1 collects all top-level item declarations into the module scope; Pass 2 walks every expression and statement, opening child scopes for blocks/functions/defines, binding parameters, resolving `Ident` references, and emitting diagnostics for anything unbound or duplicated. The crate depends only on `orv-span`, `orv-diagnostics`, and `orv-syntax`.

**Tech Stack:** Rust (edition 2024, nightly), `orv-span` (Span, Spanned, FileId), `orv-diagnostics` (Diagnostic, DiagnosticBag, Label), `orv-syntax` (ast::Module, ast::*), `pretty_assertions` for tests.

---

## File Structure

```text
crates/
  orv-resolve/
    Cargo.toml
    src/
      lib.rs              — public API: resolve(), ResolveResult, re-exports
      symbol.rs           — Symbol, SymbolId, SymbolKind, Visibility, SymbolTable
      scope.rs            — ScopeId, ScopeKind, Scope, ScopeMap
      resolver.rs         — Resolver struct, Pass 1 (collect), Pass 2 (resolve)
      tests.rs            — integration tests with inline orv source
```

---

### Task 1: Create `orv-resolve` crate with Symbol and SymbolTable types

**Files:**
- Create: `crates/orv-resolve/Cargo.toml`
- Create: `crates/orv-resolve/src/lib.rs`
- Create: `crates/orv-resolve/src/symbol.rs`
- Modify: `Cargo.toml` (workspace deps)

- [ ] **Step 1: Create `crates/orv-resolve/Cargo.toml`**

```toml
[package]
name = "orv-resolve"
description = "Name resolution and scope analysis for the orv language"
version.workspace = true
authors.workspace = true
edition.workspace = true
license.workspace = true
repository.workspace = true
rust-version.workspace = true

[dependencies]
orv-span = { workspace = true }
orv-diagnostics = { workspace = true }
orv-syntax = { workspace = true }

[dev-dependencies]
pretty_assertions = { workspace = true }

[lints]
workspace = true
```

- [ ] **Step 2: Add `orv-resolve` to workspace deps in root `Cargo.toml`**

Add to `[workspace.dependencies]`:

```toml
orv-resolve = { path = "crates/orv-resolve" }
```

- [ ] **Step 3: Create `crates/orv-resolve/src/symbol.rs`**

```rust
//! Symbol table: a flat registry of all named declarations.

use orv_span::Span;

/// A unique identifier for a symbol within a compilation session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SymbolId(u32);

impl SymbolId {
    /// Creates a new `SymbolId` from a raw index.
    pub const fn new(raw: u32) -> Self {
        Self(raw)
    }

    /// Returns the underlying raw value.
    pub const fn raw(self) -> u32 {
        self.0
    }
}

/// What kind of declaration a symbol represents.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolKind {
    /// `function name(...)` — a function declaration.
    Function,
    /// `define Name(...)` — a component/define declaration.
    Define,
    /// `struct Name { ... }` — a struct declaration.
    Struct,
    /// `enum Name { ... }` — an enum declaration.
    Enum,
    /// `type Name = ...` — a type alias.
    TypeAlias,
    /// `let name = ...` or `const name = ...` — a variable binding.
    Variable,
    /// A function or define parameter.
    Parameter,
    /// A `for x of ...` loop variable.
    LoopVariable,
    /// An imported name (resolved from another module).
    Import,
}

/// Visibility of a symbol.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Visibility {
    /// Accessible only within the declaring module.
    Private,
    /// Accessible from other modules (`pub`).
    Public,
}

/// A single named declaration in the program.
#[derive(Debug, Clone)]
pub struct Symbol {
    /// The declared name.
    pub name: String,
    /// What kind of thing this symbol is.
    pub kind: SymbolKind,
    /// Whether this symbol is public or private.
    pub visibility: Visibility,
    /// The span of the name token at the declaration site.
    pub def_span: Span,
}

/// A flat, append-only registry of all symbols discovered during resolution.
#[derive(Debug, Default)]
pub struct SymbolTable {
    symbols: Vec<Symbol>,
}

impl SymbolTable {
    /// Creates an empty symbol table.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a symbol and returns its id.
    ///
    /// # Panics
    ///
    /// Panics if the number of symbols exceeds `u32::MAX`.
    pub fn add(&mut self, symbol: Symbol) -> SymbolId {
        let id = u32::try_from(self.symbols.len()).expect("too many symbols");
        self.symbols.push(symbol);
        SymbolId::new(id)
    }

    /// Returns a reference to the symbol with the given id.
    pub fn get(&self, id: SymbolId) -> &Symbol {
        &self.symbols[id.raw() as usize]
    }

    /// Returns the total number of symbols.
    pub const fn len(&self) -> usize {
        self.symbols.len()
    }

    /// Returns `true` if the table is empty.
    pub const fn is_empty(&self) -> bool {
        self.symbols.is_empty()
    }

    /// Iterates over all `(SymbolId, &Symbol)` pairs.
    pub fn iter(&self) -> impl Iterator<Item = (SymbolId, &Symbol)> {
        self.symbols
            .iter()
            .enumerate()
            .map(|(i, s)| (SymbolId::new(i as u32), s))
    }
}

#[cfg(test)]
mod tests {
    use orv_span::{FileId, Span};
    use pretty_assertions::assert_eq;

    use super::*;

    fn dummy_span() -> Span {
        Span::new(FileId::new(0), 0, 3)
    }

    #[test]
    fn symbol_table_add_and_get() {
        let mut table = SymbolTable::new();
        assert!(table.is_empty());

        let id = table.add(Symbol {
            name: "foo".into(),
            kind: SymbolKind::Function,
            visibility: Visibility::Public,
            def_span: dummy_span(),
        });

        assert_eq!(table.len(), 1);
        assert_eq!(id.raw(), 0);

        let sym = table.get(id);
        assert_eq!(sym.name, "foo");
        assert_eq!(sym.kind, SymbolKind::Function);
        assert_eq!(sym.visibility, Visibility::Public);
    }

    #[test]
    fn symbol_table_multiple_entries() {
        let mut table = SymbolTable::new();

        let id_a = table.add(Symbol {
            name: "a".into(),
            kind: SymbolKind::Variable,
            visibility: Visibility::Private,
            def_span: dummy_span(),
        });
        let id_b = table.add(Symbol {
            name: "b".into(),
            kind: SymbolKind::Parameter,
            visibility: Visibility::Private,
            def_span: dummy_span(),
        });

        assert_eq!(table.len(), 2);
        assert_eq!(table.get(id_a).name, "a");
        assert_eq!(table.get(id_b).name, "b");
    }

    #[test]
    fn symbol_table_iter() {
        let mut table = SymbolTable::new();
        table.add(Symbol {
            name: "x".into(),
            kind: SymbolKind::Variable,
            visibility: Visibility::Private,
            def_span: dummy_span(),
        });
        table.add(Symbol {
            name: "y".into(),
            kind: SymbolKind::Function,
            visibility: Visibility::Public,
            def_span: dummy_span(),
        });

        let names: Vec<&str> = table.iter().map(|(_, s)| s.name.as_str()).collect();
        assert_eq!(names, vec!["x", "y"]);
    }

    #[test]
    fn symbol_id_roundtrip() {
        let id = SymbolId::new(42);
        assert_eq!(id.raw(), 42);
    }
}
```

- [ ] **Step 4: Create `crates/orv-resolve/src/lib.rs`**

```rust
//! Name resolution for the orv language.
//!
//! Takes a parsed `Module` AST and resolves every identifier to its
//! declaration site, producing a `SymbolTable` and `ScopeMap`.

pub mod symbol;

pub use symbol::{Symbol, SymbolId, SymbolKind, SymbolTable, Visibility};
```

- [ ] **Step 5: Verify**

Run: `cargo test -p orv-resolve`
Expected: 4 tests pass (symbol_table_add_and_get, symbol_table_multiple_entries, symbol_table_iter, symbol_id_roundtrip).

Run: `cargo clippy -p orv-resolve`
Expected: clean.

- [ ] **Step 6: Commit**

```bash
git add crates/orv-resolve/ Cargo.toml Cargo.lock
git commit -m "feat(resolve): add orv-resolve crate with Symbol and SymbolTable types"
```

---

### Task 2: Scope tree — Scope, ScopeId, ScopeKind, ScopeMap

**Files:**
- Create: `crates/orv-resolve/src/scope.rs`
- Modify: `crates/orv-resolve/src/lib.rs`

- [ ] **Step 1: Create `crates/orv-resolve/src/scope.rs`**

```rust
//! Scope tree: a hierarchy of lexical scopes that map names to symbol IDs.

use std::collections::HashMap;

use crate::symbol::SymbolId;

/// A unique identifier for a scope within the scope tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ScopeId(u32);

impl ScopeId {
    /// Creates a new `ScopeId` from a raw index.
    pub const fn new(raw: u32) -> Self {
        Self(raw)
    }

    /// Returns the underlying raw value.
    pub const fn raw(self) -> u32 {
        self.0
    }
}

/// What kind of syntactic construct introduced this scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScopeKind {
    /// The top-level module scope.
    Module,
    /// A function body scope.
    Function,
    /// A define (component) body scope.
    Define,
    /// A block expression `{ ... }`.
    Block,
    /// An `if` branch body.
    IfBranch,
    /// A `for` loop body.
    ForLoop,
    /// A `while` loop body.
    WhileLoop,
}

/// A single lexical scope containing name-to-symbol bindings.
#[derive(Debug)]
pub struct Scope {
    /// What introduced this scope.
    pub kind: ScopeKind,
    /// The parent scope, if any (`None` for the module root).
    pub parent: Option<ScopeId>,
    /// Names declared directly in this scope, mapped to their symbol IDs.
    bindings: HashMap<String, SymbolId>,
}

impl Scope {
    /// Creates a new scope with the given kind and optional parent.
    fn new(kind: ScopeKind, parent: Option<ScopeId>) -> Self {
        Self {
            kind,
            parent,
            bindings: HashMap::new(),
        }
    }

    /// Inserts a name binding into this scope. Returns `Some(old_id)` if
    /// the name was already bound in this scope (duplicate declaration).
    pub fn insert(&mut self, name: String, id: SymbolId) -> Option<SymbolId> {
        self.bindings.insert(name, id)
    }

    /// Looks up a name in this scope only (not parents).
    pub fn lookup_local(&self, name: &str) -> Option<SymbolId> {
        self.bindings.get(name).copied()
    }

    /// Returns an iterator over all bindings in this scope.
    pub fn bindings(&self) -> impl Iterator<Item = (&str, SymbolId)> {
        self.bindings.iter().map(|(k, v)| (k.as_str(), *v))
    }

    /// Returns the number of bindings in this scope.
    pub fn binding_count(&self) -> usize {
        self.bindings.len()
    }
}

/// A tree of lexical scopes, stored as a flat arena.
#[derive(Debug, Default)]
pub struct ScopeMap {
    scopes: Vec<Scope>,
}

impl ScopeMap {
    /// Creates an empty scope map.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a new scope with the given kind and optional parent. Returns its ID.
    ///
    /// # Panics
    ///
    /// Panics if the number of scopes exceeds `u32::MAX`.
    pub fn add(&mut self, kind: ScopeKind, parent: Option<ScopeId>) -> ScopeId {
        let id = u32::try_from(self.scopes.len()).expect("too many scopes");
        self.scopes.push(Scope::new(kind, parent));
        ScopeId::new(id)
    }

    /// Returns a reference to the scope with the given ID.
    pub fn get(&self, id: ScopeId) -> &Scope {
        &self.scopes[id.raw() as usize]
    }

    /// Returns a mutable reference to the scope with the given ID.
    pub fn get_mut(&mut self, id: ScopeId) -> &mut Scope {
        &mut self.scopes[id.raw() as usize]
    }

    /// Looks up a name starting from the given scope, walking up the parent
    /// chain until found or the root is reached.
    pub fn lookup(&self, start: ScopeId, name: &str) -> Option<SymbolId> {
        let mut current = Some(start);
        while let Some(scope_id) = current {
            let scope = self.get(scope_id);
            if let Some(sym_id) = scope.lookup_local(name) {
                return Some(sym_id);
            }
            current = scope.parent;
        }
        None
    }

    /// Returns the total number of scopes.
    pub const fn len(&self) -> usize {
        self.scopes.len()
    }

    /// Returns `true` if there are no scopes.
    pub const fn is_empty(&self) -> bool {
        self.scopes.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use orv_span::{FileId, Span};
    use pretty_assertions::assert_eq;

    use crate::symbol::{Symbol, SymbolKind, SymbolTable, Visibility};

    use super::*;

    fn dummy_span() -> Span {
        Span::new(FileId::new(0), 0, 1)
    }

    fn make_symbol(table: &mut SymbolTable, name: &str, kind: SymbolKind) -> SymbolId {
        table.add(Symbol {
            name: name.into(),
            kind,
            visibility: Visibility::Private,
            def_span: dummy_span(),
        })
    }

    #[test]
    fn scope_insert_and_lookup_local() {
        let mut symbols = SymbolTable::new();
        let sym_x = make_symbol(&mut symbols, "x", SymbolKind::Variable);

        let mut scope_map = ScopeMap::new();
        let root = scope_map.add(ScopeKind::Module, None);
        scope_map.get_mut(root).insert("x".into(), sym_x);

        assert_eq!(scope_map.get(root).lookup_local("x"), Some(sym_x));
        assert_eq!(scope_map.get(root).lookup_local("y"), None);
    }

    #[test]
    fn scope_parent_chain_lookup() {
        let mut symbols = SymbolTable::new();
        let sym_a = make_symbol(&mut symbols, "a", SymbolKind::Variable);
        let sym_b = make_symbol(&mut symbols, "b", SymbolKind::Variable);

        let mut scope_map = ScopeMap::new();
        let root = scope_map.add(ScopeKind::Module, None);
        scope_map.get_mut(root).insert("a".into(), sym_a);

        let child = scope_map.add(ScopeKind::Block, Some(root));
        scope_map.get_mut(child).insert("b".into(), sym_b);

        // `b` found in child scope
        assert_eq!(scope_map.lookup(child, "b"), Some(sym_b));
        // `a` found by walking up to parent
        assert_eq!(scope_map.lookup(child, "a"), Some(sym_a));
        // `b` not visible from root
        assert_eq!(scope_map.lookup(root, "b"), None);
    }

    #[test]
    fn scope_shadowing() {
        let mut symbols = SymbolTable::new();
        let sym_outer = make_symbol(&mut symbols, "x", SymbolKind::Variable);
        let sym_inner = make_symbol(&mut symbols, "x", SymbolKind::Variable);

        let mut scope_map = ScopeMap::new();
        let root = scope_map.add(ScopeKind::Module, None);
        scope_map.get_mut(root).insert("x".into(), sym_outer);

        let child = scope_map.add(ScopeKind::Block, Some(root));
        scope_map.get_mut(child).insert("x".into(), sym_inner);

        // inner scope sees the shadow
        assert_eq!(scope_map.lookup(child, "x"), Some(sym_inner));
        // outer scope sees original
        assert_eq!(scope_map.lookup(root, "x"), Some(sym_outer));
    }

    #[test]
    fn scope_duplicate_insert_returns_old() {
        let mut symbols = SymbolTable::new();
        let sym1 = make_symbol(&mut symbols, "dup", SymbolKind::Variable);
        let sym2 = make_symbol(&mut symbols, "dup", SymbolKind::Variable);

        let mut scope_map = ScopeMap::new();
        let root = scope_map.add(ScopeKind::Module, None);

        let first = scope_map.get_mut(root).insert("dup".into(), sym1);
        assert!(first.is_none());

        let second = scope_map.get_mut(root).insert("dup".into(), sym2);
        assert_eq!(second, Some(sym1));
    }

    #[test]
    fn scope_kind_and_binding_count() {
        let mut scope_map = ScopeMap::new();
        let root = scope_map.add(ScopeKind::Module, None);
        assert_eq!(scope_map.get(root).kind, ScopeKind::Module);
        assert_eq!(scope_map.get(root).binding_count(), 0);
        assert!(scope_map.get(root).parent.is_none());
    }

    #[test]
    fn scope_map_len() {
        let mut scope_map = ScopeMap::new();
        assert!(scope_map.is_empty());
        scope_map.add(ScopeKind::Module, None);
        scope_map.add(ScopeKind::Block, Some(ScopeId::new(0)));
        assert_eq!(scope_map.len(), 2);
    }

    #[test]
    fn deeply_nested_lookup() {
        let mut symbols = SymbolTable::new();
        let sym = make_symbol(&mut symbols, "deep", SymbolKind::Function);

        let mut scope_map = ScopeMap::new();
        let s0 = scope_map.add(ScopeKind::Module, None);
        scope_map.get_mut(s0).insert("deep".into(), sym);

        let s1 = scope_map.add(ScopeKind::Function, Some(s0));
        let s2 = scope_map.add(ScopeKind::Block, Some(s1));
        let s3 = scope_map.add(ScopeKind::IfBranch, Some(s2));

        // Can find `deep` 3 levels up
        assert_eq!(scope_map.lookup(s3, "deep"), Some(sym));
        // Unknown name fails
        assert_eq!(scope_map.lookup(s3, "nonexistent"), None);
    }
}
```

- [ ] **Step 2: Update `crates/orv-resolve/src/lib.rs`**

```rust
//! Name resolution for the orv language.
//!
//! Takes a parsed `Module` AST and resolves every identifier to its
//! declaration site, producing a `SymbolTable` and `ScopeMap`.

pub mod scope;
pub mod symbol;

pub use scope::{ScopeId, ScopeKind, ScopeMap};
pub use symbol::{Symbol, SymbolId, SymbolKind, SymbolTable, Visibility};
```

- [ ] **Step 3: Verify**

Run: `cargo test -p orv-resolve`
Expected: all symbol tests + 7 scope tests pass.

Run: `cargo clippy -p orv-resolve`
Expected: clean.

- [ ] **Step 4: Commit**

```bash
git add crates/orv-resolve/src/scope.rs crates/orv-resolve/src/lib.rs
git commit -m "feat(resolve): add scope tree with ScopeId, ScopeKind, Scope, and ScopeMap"
```

---

### Task 3: Resolver Pass 1 — collect top-level declarations

**Files:**
- Create: `crates/orv-resolve/src/resolver.rs`
- Modify: `crates/orv-resolve/src/lib.rs`

- [ ] **Step 1: Create `crates/orv-resolve/src/resolver.rs` with Pass 1 logic**

This implements the first pass: iterate over all top-level `Item`s in the `Module` and register each named declaration (function, define, struct, enum, type alias, binding) into the module scope's symbol table.

```rust
//! Two-pass name resolver.
//!
//! - **Pass 1** (`collect_items`): registers all top-level declarations into
//!   the module scope so that forward references work.
//! - **Pass 2** (`resolve_bodies`): walks every expression and statement,
//!   opening child scopes as needed and resolving `Ident` references.

use orv_diagnostics::{Diagnostic, DiagnosticBag, Label};
use orv_span::{Span, Spanned};
use orv_syntax::ast::{
    BindingStmt, DefineItem, EnumItem, Expr, ForStmt, FunctionItem, IfStmt, Item, Module, Stmt,
    StructItem, TypeAliasItem, WhileStmt,
};

use crate::scope::{ScopeId, ScopeKind, ScopeMap};
use crate::symbol::{Symbol, SymbolId, SymbolKind, SymbolTable, Visibility};

/// The result of name resolution.
#[derive(Debug)]
pub struct ResolveResult {
    /// All symbols discovered during resolution.
    pub symbols: SymbolTable,
    /// The scope tree with all name bindings.
    pub scopes: ScopeMap,
    /// The ID of the module-level root scope.
    pub root_scope: ScopeId,
}

/// The resolver state.
pub(crate) struct Resolver {
    pub(crate) symbols: SymbolTable,
    pub(crate) scopes: ScopeMap,
    pub(crate) diagnostics: DiagnosticBag,
    /// The currently active scope.
    pub(crate) current_scope: ScopeId,
}

impl Resolver {
    /// Creates a new resolver with an empty module scope.
    fn new() -> Self {
        let mut scopes = ScopeMap::new();
        let root = scopes.add(ScopeKind::Module, None);
        Self {
            symbols: SymbolTable::new(),
            scopes,
            diagnostics: DiagnosticBag::new(),
            current_scope: root,
        }
    }

    /// Returns the root (module) scope ID.
    fn root_scope(&self) -> ScopeId {
        ScopeId::new(0)
    }

    // ── Helpers ────────────────────────────────────────────────────────

    /// Declares a symbol in the given scope. Emits a duplicate-symbol
    /// diagnostic if the name is already bound in that same scope.
    fn declare_in_scope(
        &mut self,
        scope: ScopeId,
        name: &str,
        name_span: Span,
        kind: SymbolKind,
        visibility: Visibility,
    ) -> SymbolId {
        let sym_id = self.symbols.add(Symbol {
            name: name.to_owned(),
            kind,
            visibility,
            def_span: name_span,
        });

        if let Some(prev_id) = self.scopes.get_mut(scope).insert(name.to_owned(), sym_id) {
            let prev = self.symbols.get(prev_id);
            self.diagnostics.push(
                Diagnostic::error(format!("duplicate declaration of `{name}`"))
                    .with_label(Label::primary(name_span, "redefined here"))
                    .with_label(Label::secondary(prev.def_span, "previously defined here")),
            );
        }

        sym_id
    }

    /// Declares a symbol in the current scope.
    fn declare(&mut self, name: &str, name_span: Span, kind: SymbolKind, visibility: Visibility) -> SymbolId {
        let scope = self.current_scope;
        self.declare_in_scope(scope, name, name_span, kind, visibility)
    }

    /// Opens a new child scope of the given kind, sets it as current,
    /// and returns the new scope ID.
    fn push_scope(&mut self, kind: ScopeKind) -> ScopeId {
        let child = self.scopes.add(kind, Some(self.current_scope));
        self.current_scope = child;
        child
    }

    /// Restores the current scope to the parent of the given scope.
    fn pop_scope(&mut self, scope_id: ScopeId) {
        let parent = self.scopes.get(scope_id).parent
            .expect("cannot pop the root scope");
        self.current_scope = parent;
    }

    // ── Pass 1: Collect top-level items ────────────────────────────────

    /// Registers all top-level declarations in the module scope.
    /// This enables forward references: a function can call another
    /// function declared later in the file.
    fn collect_items(&mut self, module: &Module) {
        let root = self.root_scope();
        for item in &module.items {
            match item.node() {
                Item::Function(func) => {
                    let vis = if func.is_pub { Visibility::Public } else { Visibility::Private };
                    self.declare_in_scope(
                        root,
                        func.name.node(),
                        func.name.span(),
                        SymbolKind::Function,
                        vis,
                    );
                }
                Item::Define(def) => {
                    let vis = if def.is_pub { Visibility::Public } else { Visibility::Private };
                    self.declare_in_scope(
                        root,
                        def.name.node(),
                        def.name.span(),
                        SymbolKind::Define,
                        vis,
                    );
                }
                Item::Struct(s) => {
                    let vis = if s.is_pub { Visibility::Public } else { Visibility::Private };
                    self.declare_in_scope(
                        root,
                        s.name.node(),
                        s.name.span(),
                        SymbolKind::Struct,
                        vis,
                    );
                }
                Item::Enum(e) => {
                    let vis = if e.is_pub { Visibility::Public } else { Visibility::Private };
                    self.declare_in_scope(
                        root,
                        e.name.node(),
                        e.name.span(),
                        SymbolKind::Enum,
                        vis,
                    );
                }
                Item::TypeAlias(t) => {
                    let vis = if t.is_pub { Visibility::Public } else { Visibility::Private };
                    self.declare_in_scope(
                        root,
                        t.name.node(),
                        t.name.span(),
                        SymbolKind::TypeAlias,
                        vis,
                    );
                }
                Item::Binding(b) => {
                    let vis = if b.is_pub { Visibility::Public } else { Visibility::Private };
                    self.declare_in_scope(
                        root,
                        b.name.node(),
                        b.name.span(),
                        SymbolKind::Variable,
                        vis,
                    );
                }
                Item::Import(imp) => {
                    self.collect_import(root, imp);
                }
                Item::Stmt(_) | Item::Error => {}
            }
        }
    }

    /// Registers imported names in the given scope.
    fn collect_import(&mut self, scope: ScopeId, imp: &orv_syntax::ast::ImportItem) {
        if imp.names.is_empty() {
            // Single import: `import components.Button` or `import components.Button as Btn`
            // The binding name is the alias if present, otherwise the last path segment.
            if let Some(alias) = &imp.alias {
                self.declare_in_scope(
                    scope,
                    alias.node(),
                    alias.span(),
                    SymbolKind::Import,
                    Visibility::Private,
                );
            } else if let Some(last) = imp.path.last() {
                self.declare_in_scope(
                    scope,
                    last.node(),
                    last.span(),
                    SymbolKind::Import,
                    Visibility::Private,
                );
            }
        } else {
            // Destructured import: `import components.{Button, Input}`
            for name in &imp.names {
                self.declare_in_scope(
                    scope,
                    name.node(),
                    name.span(),
                    SymbolKind::Import,
                    Visibility::Private,
                );
            }
        }
    }
}

/// Resolves all names in a parsed module.
///
/// Returns the resolution result and any diagnostics emitted.
pub fn resolve(module: &Module) -> (ResolveResult, DiagnosticBag) {
    let mut resolver = Resolver::new();

    // Pass 1: collect top-level declarations.
    resolver.collect_items(module);

    let root = resolver.root_scope();
    let result = ResolveResult {
        symbols: resolver.symbols,
        scopes: resolver.scopes,
        root_scope: root,
    };
    (result, resolver.diagnostics)
}

#[cfg(test)]
mod tests {
    use orv_syntax::parser::parse;
    use orv_syntax::lexer::Lexer;
    use orv_span::FileId;
    use pretty_assertions::assert_eq;

    use super::*;

    /// Helper: lex + parse + resolve, returning the result and diagnostics.
    fn resolve_source(src: &str) -> (ResolveResult, DiagnosticBag) {
        let file = FileId::new(0);
        let lexer = Lexer::new(src, file);
        let (tokens, lex_diags) = lexer.tokenize();
        assert!(!lex_diags.has_errors(), "lexer errors: {lex_diags:?}");
        let (module, parse_diags) = parse(tokens);
        assert!(!parse_diags.has_errors(), "parse errors: {parse_diags:?}");
        resolve(&module)
    }

    #[test]
    fn collect_function() {
        let (result, diags) = resolve_source("function greet() -> void");
        assert!(!diags.has_errors());
        assert_eq!(result.symbols.len(), 1);

        let sym = result.symbols.get(SymbolId::new(0));
        assert_eq!(sym.name, "greet");
        assert_eq!(sym.kind, SymbolKind::Function);
        assert_eq!(sym.visibility, Visibility::Private);
    }

    #[test]
    fn collect_pub_function() {
        let (result, diags) = resolve_source("pub function hello() -> void");
        assert!(!diags.has_errors());

        let sym = result.symbols.get(SymbolId::new(0));
        assert_eq!(sym.name, "hello");
        assert_eq!(sym.visibility, Visibility::Public);
    }

    #[test]
    fn collect_define() {
        let (result, diags) = resolve_source("pub define Button() -> @html { void }");
        assert!(!diags.has_errors());
        assert_eq!(result.symbols.len(), 1);

        let sym = result.symbols.get(SymbolId::new(0));
        assert_eq!(sym.name, "Button");
        assert_eq!(sym.kind, SymbolKind::Define);
        assert_eq!(sym.visibility, Visibility::Public);
    }

    #[test]
    fn collect_struct() {
        let (result, diags) = resolve_source("struct Point { x: i32, y: i32 }");
        assert!(!diags.has_errors());

        let sym = result.symbols.get(SymbolId::new(0));
        assert_eq!(sym.name, "Point");
        assert_eq!(sym.kind, SymbolKind::Struct);
    }

    #[test]
    fn collect_enum() {
        let (result, diags) = resolve_source("enum Color { Red, Green, Blue }");
        assert!(!diags.has_errors());

        let sym = result.symbols.get(SymbolId::new(0));
        assert_eq!(sym.name, "Color");
        assert_eq!(sym.kind, SymbolKind::Enum);
    }

    #[test]
    fn collect_type_alias() {
        let (result, diags) = resolve_source("type Name = string");
        assert!(!diags.has_errors());

        let sym = result.symbols.get(SymbolId::new(0));
        assert_eq!(sym.name, "Name");
        assert_eq!(sym.kind, SymbolKind::TypeAlias);
    }

    #[test]
    fn collect_let_binding() {
        let (result, diags) = resolve_source("let count = 42");
        assert!(!diags.has_errors());

        let sym = result.symbols.get(SymbolId::new(0));
        assert_eq!(sym.name, "count");
        assert_eq!(sym.kind, SymbolKind::Variable);
    }

    #[test]
    fn collect_const_binding() {
        let (result, diags) = resolve_source("const MAX = 100");
        assert!(!diags.has_errors());

        let sym = result.symbols.get(SymbolId::new(0));
        assert_eq!(sym.name, "MAX");
        assert_eq!(sym.kind, SymbolKind::Variable);
    }

    #[test]
    fn collect_import_single() {
        let (result, diags) = resolve_source("import components.Button");
        assert!(!diags.has_errors());
        assert_eq!(result.symbols.len(), 1);

        let sym = result.symbols.get(SymbolId::new(0));
        assert_eq!(sym.name, "Button");
        assert_eq!(sym.kind, SymbolKind::Import);
    }

    #[test]
    fn collect_import_aliased() {
        let (result, diags) = resolve_source("import components.Button as Btn");
        assert!(!diags.has_errors());
        assert_eq!(result.symbols.len(), 1);

        let sym = result.symbols.get(SymbolId::new(0));
        assert_eq!(sym.name, "Btn");
        assert_eq!(sym.kind, SymbolKind::Import);
    }

    #[test]
    fn collect_import_destructured() {
        let (result, diags) = resolve_source("import components.{Button, Input}");
        assert!(!diags.has_errors());
        assert_eq!(result.symbols.len(), 2);

        let names: Vec<&str> = result.symbols.iter().map(|(_, s)| s.name.as_str()).collect();
        assert_eq!(names, vec!["Button", "Input"]);
    }

    #[test]
    fn collect_multiple_items() {
        let src = "\
function greet() -> void
pub define App() -> @html { void }
let x = 1
struct Point { x: i32, y: i32 }
";
        let (result, diags) = resolve_source(src);
        assert!(!diags.has_errors());
        assert_eq!(result.symbols.len(), 4);

        let names: Vec<&str> = result.symbols.iter().map(|(_, s)| s.name.as_str()).collect();
        assert_eq!(names, vec!["greet", "App", "x", "Point"]);
    }

    #[test]
    fn duplicate_top_level_detected() {
        let src = "\
function foo() -> void
function foo() -> void
";
        let (_, diags) = resolve_source(src);
        assert!(diags.has_errors());
        assert_eq!(diags.len(), 1);

        let d = diags.iter().next().unwrap();
        assert!(d.message.contains("duplicate declaration of `foo`"));
    }
}
```

- [ ] **Step 2: Update `crates/orv-resolve/src/lib.rs`**

```rust
//! Name resolution for the orv language.
//!
//! Takes a parsed `Module` AST and resolves every identifier to its
//! declaration site, producing a `SymbolTable` and `ScopeMap`.

pub mod resolver;
pub mod scope;
pub mod symbol;

pub use resolver::{resolve, ResolveResult};
pub use scope::{ScopeId, ScopeKind, ScopeMap};
pub use symbol::{Symbol, SymbolId, SymbolKind, SymbolTable, Visibility};
```

- [ ] **Step 3: Verify**

Run: `cargo test -p orv-resolve`
Expected: all symbol tests + scope tests + ~13 resolver Pass 1 tests pass.

Run: `cargo clippy -p orv-resolve`
Expected: clean.

- [ ] **Step 4: Commit**

```bash
git add crates/orv-resolve/src/resolver.rs crates/orv-resolve/src/lib.rs
git commit -m "feat(resolve): Pass 1 collects top-level declarations into module scope"
```

---

### Task 4: Resolver Pass 2 — resolve Ident references and walk expressions

**Files:**
- Modify: `crates/orv-resolve/src/resolver.rs`

- [ ] **Step 1: Add Pass 2 expression and statement walker**

Add these methods to the `Resolver` impl block in `resolver.rs`, **after** the existing `collect_import` method:

```rust
    // ── Pass 2: Resolve references ─────────────────────────────────────

    /// Walks all items in the module, resolving identifiers in bodies.
    fn resolve_items(&mut self, module: &Module) {
        for item in &module.items {
            match item.node() {
                Item::Function(func) => self.resolve_function(func),
                Item::Define(def) => self.resolve_define(def),
                Item::Binding(b) => self.resolve_binding_stmt(b),
                Item::Stmt(stmt) => self.resolve_stmt(stmt),
                Item::Import(_)
                | Item::Struct(_)
                | Item::Enum(_)
                | Item::TypeAlias(_)
                | Item::Error => {
                    // Struct/enum/type bodies don't contain resolvable
                    // expressions in the current language.
                }
            }
        }
    }

    /// Resolves a function: opens a body scope, binds params, walks the body.
    fn resolve_function(&mut self, func: &FunctionItem) {
        let scope = self.push_scope(ScopeKind::Function);
        for param in &func.params {
            self.declare(
                param.node().name.node(),
                param.node().name.span(),
                SymbolKind::Parameter,
                Visibility::Private,
            );
            // Resolve default value if present.
            if let Some(default) = &param.node().default {
                self.resolve_expr(default);
            }
        }
        self.resolve_expr(&func.body);
        self.pop_scope(scope);
    }

    /// Resolves a define: opens a body scope, binds params, walks the body.
    fn resolve_define(&mut self, def: &DefineItem) {
        let scope = self.push_scope(ScopeKind::Define);
        for param in &def.params {
            self.declare(
                param.node().name.node(),
                param.node().name.span(),
                SymbolKind::Parameter,
                Visibility::Private,
            );
            if let Some(default) = &param.node().default {
                self.resolve_expr(default);
            }
        }
        self.resolve_expr(&def.body);
        self.pop_scope(scope);
    }

    /// Resolves a binding statement's initializer expression.
    fn resolve_binding_stmt(&mut self, binding: &BindingStmt) {
        // The name was already declared in Pass 1 (for top-level) or by
        // the block walker (for local bindings). Resolve the initializer.
        if let Some(value) = &binding.value {
            self.resolve_expr(value);
        }
    }

    // ── Statement resolution ───────────────────────────────────────────

    fn resolve_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Binding(b) => {
                // Local binding: declare in current scope, then resolve init.
                self.declare(
                    b.name.node(),
                    b.name.span(),
                    SymbolKind::Variable,
                    Visibility::Private,
                );
                if let Some(value) = &b.value {
                    self.resolve_expr(value);
                }
            }
            Stmt::Return(maybe_expr) => {
                if let Some(expr) = maybe_expr {
                    self.resolve_expr(expr);
                }
            }
            Stmt::If(if_stmt) => self.resolve_if(if_stmt),
            Stmt::For(for_stmt) => self.resolve_for(for_stmt),
            Stmt::While(while_stmt) => self.resolve_while(while_stmt),
            Stmt::Expr(expr) => self.resolve_expr(expr),
            Stmt::Error => {}
        }
    }

    fn resolve_if(&mut self, if_stmt: &IfStmt) {
        self.resolve_expr(&if_stmt.condition);

        let then_scope = self.push_scope(ScopeKind::IfBranch);
        self.resolve_expr(&if_stmt.then_body);
        self.pop_scope(then_scope);

        if let Some(else_body) = &if_stmt.else_body {
            let else_scope = self.push_scope(ScopeKind::IfBranch);
            self.resolve_expr(else_body);
            self.pop_scope(else_scope);
        }
    }

    fn resolve_for(&mut self, for_stmt: &ForStmt) {
        // The iterable is evaluated in the outer scope.
        self.resolve_expr(&for_stmt.iterable);

        let scope = self.push_scope(ScopeKind::ForLoop);
        self.declare(
            for_stmt.binding.node(),
            for_stmt.binding.span(),
            SymbolKind::LoopVariable,
            Visibility::Private,
        );
        self.resolve_expr(&for_stmt.body);
        self.pop_scope(scope);
    }

    fn resolve_while(&mut self, while_stmt: &WhileStmt) {
        self.resolve_expr(&while_stmt.condition);

        let scope = self.push_scope(ScopeKind::WhileLoop);
        self.resolve_expr(&while_stmt.body);
        self.pop_scope(scope);
    }

    // ── Expression resolution ──────────────────────────────────────────

    fn resolve_expr(&mut self, expr: &Spanned<Expr>) {
        match expr.node() {
            Expr::Ident(name) => {
                if self.scopes.lookup(self.current_scope, name).is_none() {
                    self.diagnostics.push(
                        Diagnostic::error(format!("unresolved name `{name}`"))
                            .with_label(Label::primary(expr.span(), "not found in this scope")),
                    );
                }
            }
            Expr::Binary { left, right, .. } => {
                self.resolve_expr(left);
                self.resolve_expr(right);
            }
            Expr::Unary { operand, .. } => {
                self.resolve_expr(operand);
            }
            Expr::Assign { target, value, .. } => {
                self.resolve_expr(target);
                self.resolve_expr(value);
            }
            Expr::Call { callee, args } => {
                self.resolve_expr(callee);
                for arg in args {
                    self.resolve_expr(&arg.node().value);
                }
            }
            Expr::Field { object, .. } => {
                self.resolve_expr(object);
            }
            Expr::Index { object, index } => {
                self.resolve_expr(object);
                self.resolve_expr(index);
            }
            Expr::Block(stmts) => {
                let scope = self.push_scope(ScopeKind::Block);
                for stmt in stmts {
                    self.resolve_stmt(stmt.node());
                }
                self.pop_scope(scope);
            }
            Expr::Object(fields) => {
                for field in fields {
                    self.resolve_expr(&field.node().value);
                }
            }
            Expr::Array(elems) => {
                for elem in elems {
                    self.resolve_expr(elem);
                }
            }
            Expr::Node(node_expr) => {
                for pos in &node_expr.positional {
                    self.resolve_expr(pos);
                }
                for prop in &node_expr.properties {
                    self.resolve_expr(&prop.node().value);
                }
                if let Some(body) = &node_expr.body {
                    self.resolve_expr(body);
                }
            }
            Expr::Paren(inner) => {
                self.resolve_expr(inner);
            }
            Expr::Await(inner) => {
                self.resolve_expr(inner);
            }
            Expr::StringInterp(parts) => {
                for part in parts {
                    if let orv_syntax::ast::StringPart::Expr(e) = part {
                        self.resolve_expr(e);
                    }
                }
            }
            // Literals and error have nothing to resolve.
            Expr::IntLiteral(_)
            | Expr::FloatLiteral(_)
            | Expr::StringLiteral(_)
            | Expr::BoolLiteral(_)
            | Expr::Void
            | Expr::Error => {}
        }
    }
```

- [ ] **Step 2: Update the `resolve` function to call Pass 2**

Replace the existing `resolve` function body at the bottom of `resolver.rs`:

```rust
/// Resolves all names in a parsed module.
///
/// Returns the resolution result and any diagnostics emitted.
pub fn resolve(module: &Module) -> (ResolveResult, DiagnosticBag) {
    let mut resolver = Resolver::new();

    // Pass 1: collect top-level declarations.
    resolver.collect_items(module);

    // Pass 2: resolve references in all bodies.
    resolver.resolve_items(module);

    let root = resolver.root_scope();
    let result = ResolveResult {
        symbols: resolver.symbols,
        scopes: resolver.scopes,
        root_scope: root,
    };
    (result, resolver.diagnostics)
}
```

- [ ] **Step 3: Add Pass 2 tests**

Append these tests to the existing `#[cfg(test)] mod tests` block in `resolver.rs`:

```rust
    #[test]
    fn resolve_ident_in_function_body() {
        let src = "\
let x = 1
function foo() -> x
";
        let (_, diags) = resolve_source(src);
        assert!(!diags.has_errors());
    }

    #[test]
    fn unresolved_ident_reported() {
        let src = "function foo() -> bar";
        let (_, diags) = resolve_source(src);
        assert!(diags.has_errors());
        assert_eq!(diags.len(), 1);

        let d = diags.iter().next().unwrap();
        assert!(d.message.contains("unresolved name `bar`"));
    }

    #[test]
    fn function_param_resolves_in_body() {
        let src = "function greet(name: string) -> name";
        let (result, diags) = resolve_source(src);
        assert!(!diags.has_errors());

        // greet + name (param) = 2 symbols
        assert_eq!(result.symbols.len(), 2);
    }

    #[test]
    fn define_param_resolves_in_body() {
        let src = "define Card(title: string) -> @html { title }";
        let (_, diags) = resolve_source(src);
        assert!(!diags.has_errors());
    }

    #[test]
    fn forward_reference_works() {
        // `foo` calls `bar` which is defined later — Pass 1 collected both first.
        let src = "\
function foo() -> bar()
function bar() -> 42
";
        let (_, diags) = resolve_source(src);
        assert!(!diags.has_errors());
    }

    #[test]
    fn import_name_resolves() {
        let src = "\
import components.Button
function render() -> Button()
";
        let (_, diags) = resolve_source(src);
        assert!(!diags.has_errors());
    }

    #[test]
    fn binary_expr_resolves_both_sides() {
        let src = "\
let a = 1
let b = 2
let c = a + b
";
        let (_, diags) = resolve_source(src);
        assert!(!diags.has_errors());
    }

    #[test]
    fn call_expr_resolves_callee_and_args() {
        let src = "\
let x = 10
function foo(n: i32) -> n
let result = foo(x)
";
        let (_, diags) = resolve_source(src);
        assert!(!diags.has_errors());
    }

    #[test]
    fn node_positional_resolves() {
        let src = "\
let msg = \"hello\"
@io.out msg
";
        let (_, diags) = resolve_source(src);
        assert!(!diags.has_errors());
    }
```

- [ ] **Step 4: Verify**

Run: `cargo test -p orv-resolve`
Expected: all existing tests + ~9 new Pass 2 tests pass.

Run: `cargo clippy -p orv-resolve`
Expected: clean.

- [ ] **Step 5: Commit**

```bash
git add crates/orv-resolve/src/resolver.rs
git commit -m "feat(resolve): Pass 2 walks expressions and resolves Ident references"
```

---

### Task 5: Function and define param scoping with default values

**Files:**
- Modify: `crates/orv-resolve/src/resolver.rs` (tests only — logic is already in Task 4)

This task verifies that the param-binding logic from Task 4 handles edge cases: multiple params, default values referencing earlier params or outer scope, and params shadowing outer names.

- [ ] **Step 1: Add param edge-case tests**

Append to the `tests` module in `resolver.rs`:

```rust
    #[test]
    fn multiple_params_all_resolve() {
        let src = "function add(a: i32, b: i32) -> a + b";
        let (result, diags) = resolve_source(src);
        assert!(!diags.has_errors());
        // add + a + b = 3 symbols
        assert_eq!(result.symbols.len(), 3);
    }

    #[test]
    fn param_shadows_outer() {
        let src = "\
let x = 10
function foo(x: i32) -> x
";
        let (_, diags) = resolve_source(src);
        // No error — `x` in body resolves to the parameter, not the outer `let x`.
        assert!(!diags.has_errors());
    }

    #[test]
    fn param_default_resolves_outer() {
        let src = "\
let default_size = 16
function make(size: i32 = default_size) -> size
";
        let (_, diags) = resolve_source(src);
        // Default value `default_size` resolves to the top-level let.
        assert!(!diags.has_errors());
    }

    #[test]
    fn define_multiple_params() {
        let src = "define Layout(width: i32, height: i32) -> @html { width + height }";
        let (result, diags) = resolve_source(src);
        assert!(!diags.has_errors());
        // Layout + width + height = 3 symbols
        assert_eq!(result.symbols.len(), 3);
    }

    #[test]
    fn unresolved_in_default_value() {
        let src = "function foo(x: i32 = unknown) -> x";
        let (_, diags) = resolve_source(src);
        assert!(diags.has_errors());
        let d = diags.iter().next().unwrap();
        assert!(d.message.contains("unresolved name `unknown`"));
    }
```

- [ ] **Step 2: Verify**

Run: `cargo test -p orv-resolve`
Expected: all tests pass including the 5 new param edge-case tests.

- [ ] **Step 3: Commit**

```bash
git add crates/orv-resolve/src/resolver.rs
git commit -m "test(resolve): add param scoping edge-case tests (shadowing, defaults)"
```

---

### Task 6: Block scoping — if/for/while bodies create child scopes with local shadowing

**Files:**
- Modify: `crates/orv-resolve/src/resolver.rs` (tests only — logic is already in Task 4)

This task verifies that block-level scoping is correct: variables declared inside `if`/`for`/`while` bodies are not visible outside, and inner bindings can shadow outer names.

- [ ] **Step 1: Add block scoping tests**

Append to the `tests` module in `resolver.rs`:

```rust
    #[test]
    fn block_scope_isolates_locals() {
        let src = "\
function foo() -> {
    let inner = 1
    inner
}
";
        let (_, diags) = resolve_source(src);
        assert!(!diags.has_errors());
    }

    #[test]
    fn for_loop_binding_scoped() {
        let src = "\
let items = 0
function foo() -> {
    for item of items {
        item
    }
}
";
        let (_, diags) = resolve_source(src);
        assert!(!diags.has_errors());
    }

    #[test]
    fn for_loop_var_not_visible_outside() {
        // `item` is declared in the for-loop scope and should not
        // leak out. This test depends on the exact scoping of block
        // vs for. In our implementation the for body is a child scope,
        // so `item` used after the loop is unresolved.
        let src = "\
function foo() -> {
    for item of [1, 2, 3] {
        item
    }
    item
}
";
        let (_, diags) = resolve_source(src);
        assert!(diags.has_errors());
        let d = diags.iter().next().unwrap();
        assert!(d.message.contains("unresolved name `item`"));
    }

    #[test]
    fn if_branch_scope_isolated() {
        let src = "\
function foo() -> {
    if true {
        let inside = 1
    }
    inside
}
";
        let (_, diags) = resolve_source(src);
        assert!(diags.has_errors());
        let d = diags.iter().next().unwrap();
        assert!(d.message.contains("unresolved name `inside`"));
    }

    #[test]
    fn while_body_scope_isolated() {
        let src = "\
function foo() -> {
    while true {
        let w = 1
    }
    w
}
";
        let (_, diags) = resolve_source(src);
        assert!(diags.has_errors());
        let d = diags.iter().next().unwrap();
        assert!(d.message.contains("unresolved name `w`"));
    }

    #[test]
    fn local_shadows_in_block() {
        let src = "\
let x = 1
function foo() -> {
    let x = 2
    x
}
";
        let (_, diags) = resolve_source(src);
        // No error — inner `x` shadows outer `x`.
        assert!(!diags.has_errors());
    }

    #[test]
    fn nested_block_scoping() {
        let src = "\
function foo() -> {
    let a = 1
    {
        let b = 2
        a + b
    }
}
";
        let (_, diags) = resolve_source(src);
        assert!(!diags.has_errors());
    }

    #[test]
    fn if_else_both_scoped() {
        let src = "\
let cond = true
function foo() -> {
    if cond {
        let t = 1
        t
    } else {
        let f = 2
        f
    }
}
";
        let (_, diags) = resolve_source(src);
        assert!(!diags.has_errors());
    }
```

- [ ] **Step 2: Verify**

Run: `cargo test -p orv-resolve`
Expected: all tests pass including the 8 new block scoping tests.

- [ ] **Step 3: Commit**

```bash
git add crates/orv-resolve/src/resolver.rs
git commit -m "test(resolve): add block scoping tests for if/for/while isolation and shadowing"
```

---

### Task 7: Duplicate symbol detection with rich diagnostics

**Files:**
- Modify: `crates/orv-resolve/src/resolver.rs` (tests only — logic is already in Task 3)

The duplicate detection logic was implemented in Task 3's `declare_in_scope`. This task adds thorough tests for all the duplicate scenarios: same-scope duplicates for various symbol kinds, and import collision cases.

- [ ] **Step 1: Add duplicate detection tests**

Append to the `tests` module in `resolver.rs`:

```rust
    #[test]
    fn duplicate_let_binding() {
        let src = "\
let x = 1
let x = 2
";
        let (_, diags) = resolve_source(src);
        assert!(diags.has_errors());
        assert_eq!(diags.len(), 1);
        let d = diags.iter().next().unwrap();
        assert!(d.message.contains("duplicate declaration of `x`"));
        // Should have both primary and secondary labels.
        assert_eq!(d.labels.len(), 2);
        assert!(d.labels[0].is_primary);
        assert!(!d.labels[1].is_primary);
    }

    #[test]
    fn duplicate_struct() {
        let src = "\
struct Foo { a: i32 }
struct Foo { b: i32 }
";
        let (_, diags) = resolve_source(src);
        assert!(diags.has_errors());
        let d = diags.iter().next().unwrap();
        assert!(d.message.contains("duplicate declaration of `Foo`"));
    }

    #[test]
    fn duplicate_function_and_let() {
        // A `let` and a `function` with the same name are also a conflict.
        let src = "\
function foo() -> void
let foo = 42
";
        let (_, diags) = resolve_source(src);
        assert!(diags.has_errors());
        let d = diags.iter().next().unwrap();
        assert!(d.message.contains("duplicate declaration of `foo`"));
    }

    #[test]
    fn duplicate_import_names() {
        let src = "\
import a.{Foo}
import b.{Foo}
";
        let (_, diags) = resolve_source(src);
        assert!(diags.has_errors());
        let d = diags.iter().next().unwrap();
        assert!(d.message.contains("duplicate declaration of `Foo`"));
    }

    #[test]
    fn import_alias_collision() {
        let src = "\
import a.Foo
import b.Bar as Foo
";
        let (_, diags) = resolve_source(src);
        assert!(diags.has_errors());
        let d = diags.iter().next().unwrap();
        assert!(d.message.contains("duplicate declaration of `Foo`"));
    }

    #[test]
    fn no_duplicate_in_different_scopes() {
        // Same name in different scopes is fine (shadowing, not duplication).
        let src = "\
let x = 1
function foo() -> {
    let x = 2
    x
}
";
        let (_, diags) = resolve_source(src);
        assert!(!diags.has_errors());
    }

    #[test]
    fn duplicate_in_same_block() {
        let src = "\
function foo() -> {
    let a = 1
    let a = 2
    a
}
";
        let (_, diags) = resolve_source(src);
        assert!(diags.has_errors());
        let d = diags.iter().next().unwrap();
        assert!(d.message.contains("duplicate declaration of `a`"));
    }
```

- [ ] **Step 2: Verify**

Run: `cargo test -p orv-resolve`
Expected: all tests pass including the 7 new duplicate detection tests.

- [ ] **Step 3: Commit**

```bash
git add crates/orv-resolve/src/resolver.rs
git commit -m "test(resolve): add duplicate symbol detection tests for all declaration kinds"
```

---

### Task 8: Integration tests and CLI `dump resolve` command

**Files:**
- Create: `crates/orv-resolve/src/tests.rs`
- Modify: `crates/orv-cli/Cargo.toml`
- Modify: `crates/orv-cli/src/main.rs`

- [ ] **Step 1: Create `crates/orv-resolve/src/tests.rs` with integration tests**

These tests exercise larger, more realistic orv programs that combine multiple features.

```rust
//! Integration tests for the name resolver using larger orv programs.

#[cfg(test)]
mod tests {
    use orv_diagnostics::DiagnosticBag;
    use orv_span::FileId;
    use orv_syntax::lexer::Lexer;
    use orv_syntax::parser::parse;
    use pretty_assertions::assert_eq;

    use crate::resolver::{resolve, ResolveResult};
    use crate::SymbolKind;

    fn resolve_source(src: &str) -> (ResolveResult, DiagnosticBag) {
        let file = FileId::new(0);
        let lexer = Lexer::new(src, file);
        let (tokens, lex_diags) = lexer.tokenize();
        assert!(!lex_diags.has_errors(), "lexer errors: {lex_diags:?}");
        let (module, parse_diags) = parse(tokens);
        assert!(!parse_diags.has_errors(), "parse errors: {parse_diags:?}");
        resolve(&module)
    }

    #[test]
    fn full_program_counter() {
        let src = "\
let count = 0

function increment() -> {
    count = count + 1
}

function get_count() -> count

pub define Counter() -> @html {
    @button \"Increment\" {
        increment()
    }
    @text count
}
";
        let (result, diags) = resolve_source(src);
        assert!(!diags.has_errors(), "unexpected errors: {diags:?}");

        let names: Vec<&str> = result.symbols.iter().map(|(_, s)| s.name.as_str()).collect();
        // count, increment, get_count, Counter
        assert!(names.contains(&"count"));
        assert!(names.contains(&"increment"));
        assert!(names.contains(&"get_count"));
        assert!(names.contains(&"Counter"));
    }

    #[test]
    fn program_with_imports_and_define() {
        let src = "\
import ui.{Button, Text}
import utils.format as fmt

pub define App() -> @html {
    Button()
    Text()
    fmt(\"hello\")
}
";
        let (_, diags) = resolve_source(src);
        assert!(!diags.has_errors(), "unexpected errors: {diags:?}");
    }

    #[test]
    fn program_for_loop_with_shadowing() {
        let src = "\
let items = [1, 2, 3]
let total = 0

function sum() -> {
    for item of items {
        total = total + item
    }
    total
}
";
        let (_, diags) = resolve_source(src);
        assert!(!diags.has_errors(), "unexpected errors: {diags:?}");
    }

    #[test]
    fn program_nested_if_else() {
        let src = "\
let x = 10

function classify() -> {
    if x > 0 {
        let label = \"positive\"
        label
    } else {
        if x == 0 {
            let label = \"zero\"
            label
        } else {
            let label = \"negative\"
            label
        }
    }
}
";
        let (_, diags) = resolve_source(src);
        assert!(!diags.has_errors(), "unexpected errors: {diags:?}");
    }

    #[test]
    fn program_multiple_errors() {
        let src = "\
function foo() -> {
    let a = unknown1
    unknown2 + a
}
";
        let (_, diags) = resolve_source(src);
        assert!(diags.has_errors());
        // Should have 2 unresolved errors: unknown1, unknown2
        let error_count = diags.iter().filter(|d| d.is_error()).count();
        assert_eq!(error_count, 2);
    }

    #[test]
    fn program_struct_and_enum_declared() {
        let src = "\
struct Point { x: i32, y: i32 }
enum Direction { North, South, East, West }
type Pos = Point

function origin() -> Point
";
        let (result, diags) = resolve_source(src);
        assert!(!diags.has_errors());

        let kinds: Vec<_> = result.symbols.iter().map(|(_, s)| s.kind).collect();
        assert!(kinds.contains(&SymbolKind::Struct));
        assert!(kinds.contains(&SymbolKind::Enum));
        assert!(kinds.contains(&SymbolKind::TypeAlias));
        assert!(kinds.contains(&SymbolKind::Function));
    }

    #[test]
    fn program_define_with_node_children() {
        let src = "\
import components.Icon

pub define NavItem(label: string, href: string) -> @html {
    @a href {
        Icon()
        @span label
    }
}
";
        let (_, diags) = resolve_source(src);
        assert!(!diags.has_errors(), "unexpected errors: {diags:?}");
    }
}
```

- [ ] **Step 2: Register the test module in `lib.rs`**

Add to the end of `crates/orv-resolve/src/lib.rs`:

```rust
#[cfg(test)]
mod tests;
```

- [ ] **Step 3: Add `orv-resolve` dependency to CLI**

In `crates/orv-cli/Cargo.toml`, add to `[dependencies]`:

```toml
orv-resolve = { workspace = true }
```

- [ ] **Step 4: Add `DumpTarget::Resolve` variant and handler in CLI**

In `crates/orv-cli/src/main.rs`, add a new `DumpTarget` variant:

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
    /// Dump AST for a source file
    Ast {
        /// Path to the .orv source file
        file: PathBuf,
    },
    /// Dump resolved symbols and scope tree for a source file
    Resolve {
        /// Path to the .orv source file
        file: PathBuf,
    },
}
```

Update the `match` arm in `main()`:

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
            DumpTarget::Resolve { file } => {
                run_dump_resolve(&file);
            }
        },
```

Add the handler function:

```rust
fn run_dump_resolve(path: &PathBuf) {
    let (loader, file_id) = load_source(path);

    if let Some(id) = file_id {
        let source_map = loader.source_map();
        let source = source_map.source(id);
        let lexer = orv_syntax::lexer::Lexer::new(source, id);
        let (tokens, lex_diags) = lexer.tokenize();

        if lex_diags.has_errors() {
            render_diagnostics(source_map, &lex_diags.into_vec());
            process::exit(1);
        }

        let (module, parse_diags) = orv_syntax::parser::parse(tokens);

        if parse_diags.has_errors() {
            render_diagnostics(source_map, &parse_diags.into_vec());
            process::exit(1);
        }

        let (result, resolve_diags) = orv_resolve::resolve(&module);

        if resolve_diags.has_errors() {
            render_diagnostics(source_map, &resolve_diags.into_vec());
        }

        // Dump symbol table.
        println!("=== Symbols ({}) ===", result.symbols.len());
        for (id, sym) in result.symbols.iter() {
            let (file, line, col) = source_map.resolve(sym.def_span);
            println!(
                "  [{:>3}] {:?} {:?} `{}` at {}:{}:{}",
                id.raw(),
                sym.visibility,
                sym.kind,
                sym.name,
                file,
                line + 1,
                col,
            );
        }

        // Dump scope tree.
        println!("\n=== Scopes ({}) ===", result.scopes.len());
        for i in 0..result.scopes.len() {
            let scope_id = orv_resolve::ScopeId::new(i as u32);
            let scope = result.scopes.get(scope_id);
            let parent = scope
                .parent
                .map_or("none".to_string(), |p| p.raw().to_string());
            let bindings: Vec<String> = scope
                .bindings()
                .map(|(name, sym_id)| format!("{name}=>{}", sym_id.raw()))
                .collect();
            println!(
                "  [{:>3}] {:?} parent={} bindings=[{}]",
                i,
                scope.kind,
                parent,
                bindings.join(", ")
            );
        }
    } else {
        let (source_map, diagnostics) = loader.into_parts();
        render_diagnostics(&source_map, &diagnostics.into_vec());
        process::exit(1);
    }
}
```

- [ ] **Step 5: Verify**

Run: `cargo test -p orv-resolve`
Expected: all existing + 7 new integration tests pass.

Run: `cargo build -p orv-cli`
Expected: builds cleanly.

Run: `cargo clippy --workspace --all-targets`
Expected: clean.

Test CLI manually:

```bash
echo 'let x = 1
function foo(n: i32) -> n + x
pub define App() -> @html { foo(42) }' > /tmp/test-resolve.orv

cargo run -p orv-cli -- dump resolve /tmp/test-resolve.orv
```

Expected output should show symbols (x, foo, n, App) and scopes (Module, Function, Define).

- [ ] **Step 6: Commit**

```bash
git add crates/orv-resolve/src/tests.rs crates/orv-resolve/src/lib.rs crates/orv-cli/
git commit -m "feat(resolve): add integration tests and CLI dump resolve command"
```

---

### Task 9: Full workspace validation

**Files:** None (validation only)

- [ ] **Step 1: Run all workspace tests**

Run: `cargo test --workspace`
Expected: all tests pass across orv-span, orv-diagnostics, orv-syntax, orv-core, orv-macros, orv-resolve.

- [ ] **Step 2: Run clippy**

Run: `cargo clippy --workspace --all-targets`
Expected: no errors.

- [ ] **Step 3: Run fmt check**

Run: `cargo fmt --all -- --check`
Expected: no formatting issues.

- [ ] **Step 4: Test CLI end-to-end**

```bash
# Create a test file
cat > /tmp/resolve-e2e.orv << 'EOF'
import ui.{Button, Text}

let counter = 0

pub function increment() -> {
    counter = counter + 1
}

pub define App() -> @html {
    Button()
    @text counter
    increment()
}
EOF

cargo run -p orv-cli -- dump resolve /tmp/resolve-e2e.orv
cargo run -p orv-cli -- check /tmp/resolve-e2e.orv
```

Expected: `dump resolve` shows all symbols and scopes. `check` reports success.

```bash
# Test error reporting
cat > /tmp/resolve-err.orv << 'EOF'
function foo() -> unknown_name
let x = 1
let x = 2
EOF

cargo run -p orv-cli -- dump resolve /tmp/resolve-err.orv
```

Expected: diagnostics for `unresolved name` and `duplicate declaration`.

- [ ] **Step 5: Commit if any fixes were needed**

```bash
git add -A
git commit -m "chore: fix lint and format issues from Phase 3 validation"
```

---

## Phase 3 Exit Criteria Checklist

Per the roadmap:

- [ ] Every name in test fixtures is either bound to a `SymbolId` or diagnosed as unresolved — verified by Pass 2 tests and integration tests
- [ ] Forward references work (function calling a later-declared function) — verified by `forward_reference_works` test
- [ ] Top-level declarations: function, define, struct, enum, type alias, let/const all register — verified by individual `collect_*` tests
- [ ] Import names (single, aliased, destructured) register in module scope — verified by `collect_import_*` tests
- [ ] Function and define parameters create bindings in body scope — verified by `function_param_resolves_in_body` and param edge-case tests
- [ ] Block scoping: if/for/while bodies create isolated child scopes — verified by `*_scope_isolated` tests
- [ ] Local shadowing works: inner scope can re-bind an outer name without error — verified by `local_shadows_in_block` and `param_shadows_outer` tests
- [ ] Duplicate declarations in the same scope produce diagnostic with both spans — verified by `duplicate_*` tests
- [ ] `dump resolve` CLI command works for manual inspection — verified by Task 8 Step 5
- [ ] All workspace tests, clippy, and fmt pass — verified by Task 9

## What This Phase Does NOT Cover (Deferred to Later Phases)

These items are explicitly out of scope for this phase and will be addressed later:

- **Multi-file module graph**: Resolving imports across actual files on disk. The current implementation registers import names as `SymbolKind::Import` but does not verify the imported module exists or that the exported name is real. This requires a module graph loader (Phase 3b or Phase 4 prerequisite).
- **Route reference resolution**: `@route` nodes referencing sibling routes need route-aware scope rules beyond basic lexical scoping.
- **Cyclic import detection**: Requires the module graph.
- **Wildcard imports**: `import foo.*` semantics are not yet defined in the parser.
- **`@children` magic binding**: The `@children` implicit variable inside define bodies needs special-case handling.
- **Type resolution**: Resolving type annotations (`TypeExpr::Named`) to their declarations is a Phase 4 concern.
- **Use-before-declaration policy**: Currently all top-level declarations are visible everywhere in the module (hoisted). A stricter policy for local `let` bindings (no use before the `let` statement) could be added later.
