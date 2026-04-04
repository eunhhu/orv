//! Frontend pipeline orchestration for the orv language.

mod pipeline;

pub use orv_project::{ProjectGraph, dump_project_graph};
pub use pipeline::{AnalyzedUnit, FrontendFailure, LoadedUnit, ParsedUnit, load_path, load_string};
