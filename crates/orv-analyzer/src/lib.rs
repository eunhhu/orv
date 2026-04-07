//! Semantic analysis for the orv language.

mod analysis;
mod lower;
mod purity;
mod types;
mod validate;

pub use analysis::{Analysis, analyze};
pub use orv_hir::{
    AssignOp, BinaryOp, Expr, Module as HirModule, ScopeRef, SymbolRef, Type, dump_hir,
};
pub use purity::{PurityMap, analyze_workspace_purity, is_server_expr};

#[cfg(test)]
mod tests;
