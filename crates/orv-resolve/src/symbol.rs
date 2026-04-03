//! Symbol table: a flat registry of all named declarations.

use orv_span::Span;

/// A unique identifier for a symbol within a compilation session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SymbolId(u32);

impl SymbolId {
    /// Creates a new `SymbolId` from a raw index.
    pub(crate) const fn new(raw: u32) -> Self {
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
        self.symbols.iter().enumerate().map(|(i, s)| {
            let id = u32::try_from(i).expect("too many symbols");
            (SymbolId::new(id), s)
        })
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
