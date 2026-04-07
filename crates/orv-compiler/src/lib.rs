//! Frontend pipeline orchestration for the orv language.

pub mod emit;
mod pipeline;

pub use orv_project::{ProjectGraph, WorkspaceGraph, dump_project_graph, dump_workspace_graph};
pub use pipeline::{
    AnalyzedUnit, FrontendFailure, LoadedUnit, ParsedUnit, WorkspaceHir, load_path,
    load_project_graph, load_string, load_workspace_hir,
};
