//! Name resolution for the orv language.
//!
//! Takes a parsed `Module` AST and resolves every identifier to its
//! declaration site, producing a `SymbolTable` and `ScopeMap`.

pub mod resolver;
pub mod scope;
pub mod symbol;

pub use resolver::{ResolveResult, resolve};
pub use scope::{ScopeId, ScopeKind, ScopeMap};
pub use symbol::{Symbol, SymbolId, SymbolKind, SymbolTable, Visibility};

#[cfg(test)]
mod tests;
