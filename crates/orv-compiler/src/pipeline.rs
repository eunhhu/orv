use std::collections::{BTreeSet, VecDeque};
use std::path::{Path, PathBuf};

use orv_analyzer::{Analysis, HirModule, analyze};
use orv_core::source::SourceLoader;
use orv_diagnostics::DiagnosticBag;
use orv_project::{ModuleDependency, ProjectGraph, WorkspaceGraph, build_project_graph};
use orv_span::{FileId, SourceMap, Spanned};
use orv_syntax::{ast, lexer::Lexer, parser, token::TokenKind};

#[derive(Debug)]
pub struct FrontendFailure {
    source_map: SourceMap,
    diagnostics: DiagnosticBag,
}

impl FrontendFailure {
    #[must_use]
    pub fn into_parts(self) -> (SourceMap, DiagnosticBag) {
        (self.source_map, self.diagnostics)
    }
}

pub struct LoadedUnit {
    loader: SourceLoader,
    file_id: FileId,
}

impl LoadedUnit {
    #[must_use]
    pub const fn file_id(&self) -> FileId {
        self.file_id
    }

    #[must_use]
    pub const fn source_map(&self) -> &SourceMap {
        self.loader.source_map()
    }

    #[must_use]
    pub fn source_name(&self) -> &str {
        self.source_map().name(self.file_id)
    }

    #[must_use]
    pub fn source(&self) -> &str {
        self.source_map().source(self.file_id)
    }

    #[must_use]
    pub fn tokenize(&self) -> (Vec<Spanned<TokenKind>>, DiagnosticBag) {
        Lexer::new(self.source(), self.file_id).tokenize()
    }

    pub fn parse(self) -> Result<ParsedUnit, FrontendFailure> {
        let (tokens, diagnostics) = self.tokenize();
        if diagnostics.has_errors() {
            return Err(self.into_failure(diagnostics));
        }

        let (module, diagnostics) = parser::parse(tokens);
        if diagnostics.has_errors() {
            return Err(self.into_failure(diagnostics));
        }

        Ok(ParsedUnit {
            loaded: self,
            module,
        })
    }

    fn into_failure(self, diagnostics: DiagnosticBag) -> FrontendFailure {
        let (source_map, loader_diagnostics) = self.loader.into_parts();
        let diagnostics = merge_diagnostics(loader_diagnostics, diagnostics);
        FrontendFailure {
            source_map,
            diagnostics,
        }
    }
}

pub struct ParsedUnit {
    loaded: LoadedUnit,
    module: ast::Module,
}

impl ParsedUnit {
    #[must_use]
    pub const fn file_id(&self) -> FileId {
        self.loaded.file_id()
    }

    #[must_use]
    pub const fn source_map(&self) -> &SourceMap {
        self.loaded.source_map()
    }

    #[must_use]
    pub const fn module(&self) -> &ast::Module {
        &self.module
    }

    #[must_use]
    pub fn dump_ast(&self) -> String {
        parser::dump_ast(&self.module)
    }

    pub fn analyze(self) -> Result<AnalyzedUnit, FrontendFailure> {
        let (analysis, diagnostics) = analyze(&self.module);
        if diagnostics.has_errors() {
            return Err(self.into_failure(diagnostics));
        }

        Ok(AnalyzedUnit {
            parsed: self,
            analysis,
        })
    }

    fn into_failure(self, diagnostics: DiagnosticBag) -> FrontendFailure {
        let (source_map, loader_diagnostics) = self.loaded.loader.into_parts();
        let diagnostics = merge_diagnostics(loader_diagnostics, diagnostics);
        FrontendFailure {
            source_map,
            diagnostics,
        }
    }
}

pub struct AnalyzedUnit {
    parsed: ParsedUnit,
    analysis: Analysis,
}

impl AnalyzedUnit {
    #[must_use]
    pub const fn file_id(&self) -> FileId {
        self.parsed.file_id()
    }

    #[must_use]
    pub const fn source_map(&self) -> &SourceMap {
        self.parsed.source_map()
    }

    #[must_use]
    pub const fn module(&self) -> &ast::Module {
        self.parsed.module()
    }

    #[must_use]
    pub const fn analysis(&self) -> &Analysis {
        &self.analysis
    }

    #[must_use]
    pub fn project_graph(&self) -> ProjectGraph {
        build_project_graph(self.source_map().name(self.file_id()), &self.analysis.hir)
    }
}

pub fn load_path(path: &Path) -> Result<LoadedUnit, FrontendFailure> {
    let absolute = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let root = absolute.parent().unwrap_or_else(|| absolute.as_ref());
    let display_name = path.display().to_string();

    load_path_with_root(&absolute, root, &display_name)
}

pub fn load_project_graph(path: &Path) -> Result<WorkspaceGraph, FrontendFailure> {
    let entry = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let root = entry
        .parent()
        .unwrap_or_else(|| entry.as_ref())
        .to_path_buf();
    let entry_display = display_name_for(&root, &entry);
    let mut queue = VecDeque::from([entry.clone()]);
    let mut visited = BTreeSet::new();
    let mut modules = Vec::new();
    let mut dependencies = Vec::new();

    while let Some(module_path) = queue.pop_front() {
        let canonical = std::fs::canonicalize(&module_path).unwrap_or_else(|_| module_path.clone());
        if !visited.insert(canonical.clone()) {
            continue;
        }

        let analyzed =
            load_path_with_root(&canonical, &root, &display_name_for(&root, &canonical))?
                .parse()?
                .analyze()?;
        let module_name = analyzed.source_map().name(analyzed.file_id()).to_owned();
        let module_dir = canonical
            .parent()
            .unwrap_or_else(|| canonical.as_ref())
            .to_path_buf();
        let imports = discover_import_modules_with_base(analyzed.module(), &root, &module_dir);
        for import in imports {
            dependencies.push(ModuleDependency {
                from: module_name.clone(),
                to: import.display_name.clone(),
            });
            queue.push_back(import.absolute_path);
        }
        modules.push(analyzed.project_graph());
    }

    Ok(WorkspaceGraph {
        entry: entry_display,
        modules,
        dependencies,
    })
}

fn load_path_with_root(
    absolute: &Path,
    root: &Path,
    display_name: &str,
) -> Result<LoadedUnit, FrontendFailure> {
    let mut loader = SourceLoader::new(root);

    if let Some(file_id) = loader.load_absolute(absolute, display_name) {
        Ok(LoadedUnit { loader, file_id })
    } else {
        let (source_map, diagnostics) = loader.into_parts();
        Err(FrontendFailure {
            source_map,
            diagnostics,
        })
    }
}

#[must_use]
pub fn load_string(name: &str, source: &str) -> LoadedUnit {
    let mut loader = SourceLoader::new(PathBuf::from("."));
    let file_id = loader.load_string(name, source);
    LoadedUnit { loader, file_id }
}

fn merge_diagnostics(mut initial: DiagnosticBag, next: DiagnosticBag) -> DiagnosticBag {
    for diagnostic in next.into_vec() {
        initial.push(diagnostic);
    }
    initial
}

#[derive(Debug, Clone)]
struct ImportModuleCandidate {
    absolute_path: PathBuf,
    display_name: String,
}

/// Discover import modules, using `base` as the directory of the importing file
/// for relative/glob imports, and `root` as the project root for absolute imports.
fn discover_import_modules_with_base(
    module: &ast::Module,
    root: &Path,
    base: &Path,
) -> Vec<ImportModuleCandidate> {
    let mut seen = BTreeSet::new();
    let mut imports = Vec::new();

    for item in &module.items {
        let ast::Item::Import(import) = item.node() else {
            continue;
        };

        let path_segments = import
            .path
            .iter()
            .map(|segment| segment.node().as_str())
            .collect::<Vec<_>>();
        if path_segments.is_empty() || path_segments[0].starts_with('@') {
            continue;
        }

        let names = import
            .names
            .iter()
            .map(|name| name.node().as_str())
            .collect::<Vec<_>>();

        if names.is_empty() {
            let candidates = if path_segments.len() > 1 {
                vec![
                    path_segments.clone(),
                    path_segments[..path_segments.len() - 1].to_vec(),
                ]
            } else {
                vec![path_segments.clone()]
            };

            for segments in candidates {
                if let Some(candidate) = resolve_module_file(root, &segments)
                    && seen.insert(candidate.display_name.clone())
                {
                    imports.push(candidate);
                    break;
                }
            }
            continue;
        }

        // Glob import: `import *.*` or `import *.{*}` — scan sibling .orv files
        if path_segments.len() == 1 && path_segments[0] == "*" && names.len() == 1 && names[0] == "*" {
            if let Ok(entries) = std::fs::read_dir(base) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().and_then(|e| e.to_str()) == Some("orv")
                        && path.is_file()
                    {
                        let display = display_name_for(root, &path);
                        if seen.insert(display.clone()) {
                            imports.push(ImportModuleCandidate {
                                display_name: display,
                                absolute_path: path,
                            });
                        }
                    }
                }
            }
            continue;
        }

        if names.len() == 1
            && names[0] == "*"
            && let Some(candidate) = resolve_module_file(root, &path_segments)
            && seen.insert(candidate.display_name.clone())
        {
            imports.push(candidate);
            continue;
        }

        if let Some(candidate) = resolve_module_file(root, &path_segments)
            && seen.insert(candidate.display_name.clone())
        {
            imports.push(candidate);
        }

        for name in names {
            let mut segments = path_segments.clone();
            segments.push(name);
            if let Some(candidate) = resolve_module_file(root, &segments)
                && seen.insert(candidate.display_name.clone())
            {
                imports.push(candidate);
            }
        }
    }

    imports
}

fn resolve_module_file(root: &Path, segments: &[&str]) -> Option<ImportModuleCandidate> {
    if segments.is_empty() {
        return None;
    }

    let mut absolute = root.to_path_buf();
    for segment in segments {
        absolute.push(segment);
    }

    // Try <path>.orv first
    let mut with_ext = absolute.clone();
    with_ext.set_extension("orv");
    if with_ext.is_file() {
        return Some(ImportModuleCandidate {
            display_name: display_name_for(root, &with_ext),
            absolute_path: with_ext,
        });
    }

    // Try <path>/index.orv (directory module)
    let index_path = absolute.join("index.orv");
    if index_path.is_file() {
        return Some(ImportModuleCandidate {
            display_name: display_name_for(root, &index_path),
            absolute_path: index_path,
        });
    }

    None
}

/// A workspace with all modules' HIR for runtime execution.
#[derive(Debug, Clone)]
pub struct WorkspaceHir {
    pub entry: String,
    pub modules: Vec<(String, HirModule)>,
}

/// Load the entry module and all its transitive imports, returning every
/// module's HIR.  The entry module is always the first element.
pub fn load_workspace_hir(path: &Path) -> Result<WorkspaceHir, FrontendFailure> {
    let entry = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let root = entry
        .parent()
        .unwrap_or_else(|| entry.as_ref())
        .to_path_buf();
    let entry_display = display_name_for(&root, &entry);
    let mut queue = VecDeque::from([entry.clone()]);
    let mut visited = BTreeSet::new();
    let mut modules = Vec::new();

    while let Some(module_path) = queue.pop_front() {
        let canonical = std::fs::canonicalize(&module_path).unwrap_or_else(|_| module_path.clone());
        if !visited.insert(canonical.clone()) {
            continue;
        }

        let parsed = load_path_with_root(
            &canonical,
            &root,
            &display_name_for(&root, &canonical),
        )?
        .parse()?;

        let module_dir = canonical
            .parent()
            .unwrap_or_else(|| canonical.as_ref())
            .to_path_buf();

        // Discover imports from the AST (before analysis) so we can traverse
        // even when a module has unresolved names.
        let ast_imports =
            discover_import_modules_with_base(parsed.module(), &root, &module_dir);
        for import in ast_imports {
            queue.push_back(import.absolute_path);
        }

        // Analysis may fail for modules with unresolved domain nodes (e.g.
        // @db.find).  When that happens we still need the HIR for any
        // defines the module exports, so we perform a best-effort lowering.
        match parsed.analyze() {
            Ok(analyzed) => {
                let module_name =
                    analyzed.source_map().name(analyzed.file_id()).to_owned();
                modules.push((module_name, analyzed.analysis.hir.clone()));
            }
            Err(failure) => {
                // Best-effort: the analysis produced diagnostics but also
                // a usable HIR.  Re-analyze ignoring errors to get it.
                let (source_map, _diagnostics) = failure.into_parts();
                let module_name =
                    source_map.name(orv_span::FileId::new(0)).to_owned();
                if let Ok(re) = load_path_with_root(
                    &canonical,
                    &root,
                    &display_name_for(&root, &canonical),
                )
                    && let Ok(re) = re.parse()
                {
                    let (analysis, _) = analyze(re.module());
                    modules.push((module_name, analysis.hir.clone()));
                }
            }
        }
    }

    Ok(WorkspaceHir {
        entry: entry_display,
        modules,
    })
}

fn display_name_for(root: &Path, absolute: &Path) -> String {
    absolute.strip_prefix(root).map_or_else(
        |_| absolute.display().to_string(),
        |relative| relative.display().to_string(),
    )
}

#[cfg(test)]
mod tests {
    use std::fs;

    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn parse_pipeline_succeeds_for_valid_program() {
        let parsed =
            match load_string("hello.orv", "function greet(name: string) -> name\n").parse() {
                Ok(parsed) => parsed,
                Err(_) => panic!("parse should succeed"),
            };

        assert_eq!(parsed.module().items.len(), 1);
        assert_eq!(parsed.source_map().name(parsed.file_id()), "hello.orv");
    }

    #[test]
    fn analyze_pipeline_succeeds_for_valid_program() {
        let parsed = match load_string("counter.orv", "let x = 1\nfunction foo() -> x\n").parse() {
            Ok(parsed) => parsed,
            Err(_) => panic!("parse should succeed"),
        };
        let analyzed = match parsed.analyze() {
            Ok(analyzed) => analyzed,
            Err(_) => panic!("analysis should succeed"),
        };

        assert_eq!(analyzed.analysis().symbols.len(), 2);
        assert_eq!(analyzed.analysis().scopes.len(), 2);
    }

    #[test]
    fn parse_failure_preserves_diagnostics() {
        let failure = match load_string("broken.orv", "function foo( -> void\n").parse() {
            Ok(_) => panic!("parse should fail"),
            Err(failure) => failure,
        };

        let (source_map, diagnostics) = failure.into_parts();
        assert_eq!(source_map.name(FileId::new(0)), "broken.orv");
        assert!(diagnostics.has_errors());
    }

    #[test]
    fn analysis_failure_preserves_source_map() {
        let parsed = match load_string("missing.orv", "function foo() -> missing\n").parse() {
            Ok(parsed) => parsed,
            Err(_) => panic!("parse should succeed"),
        };
        let failure = match parsed.analyze() {
            Ok(_) => panic!("analysis should fail"),
            Err(failure) => failure,
        };

        let (source_map, diagnostics) = failure.into_parts();
        assert_eq!(source_map.name(FileId::new(0)), "missing.orv");
        assert!(diagnostics.has_errors());
    }

    #[test]
    fn project_graph_recursively_loads_local_imports() {
        let root = std::env::temp_dir().join("orv-project-graph-recursive");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("components")).expect("components dir");
        fs::create_dir_all(root.join("libs")).expect("libs dir");
        fs::write(
            root.join("main.orv"),
            "import components.Button\nimport libs.counter\npub define Home() -> @html {\n  @body {\n    let sig count: i32 = 0\n    @Button \"ok\"\n  }\n}\n",
        )
        .expect("main file");
        fs::write(
            root.join("components/Button.orv"),
            "pub define Button(label: string) -> @button label\n",
        )
        .expect("button file");
        fs::write(
            root.join("libs/counter.orv"),
            "pub function counter(): i32 -> 1\n",
        )
        .expect("counter file");

        let graph = load_project_graph(&root.join("main.orv")).expect("project graph should load");
        assert_eq!(graph.entry, "main.orv");
        assert_eq!(graph.modules.len(), 3);
        assert!(
            graph
                .dependencies
                .iter()
                .any(|dependency| dependency.from == "main.orv"
                    && dependency.to == "components/Button.orv")
        );
        assert!(
            graph
                .dependencies
                .iter()
                .any(|dependency| dependency.from == "main.orv"
                    && dependency.to == "libs/counter.orv")
        );

        let _ = fs::remove_dir_all(&root);
    }
}
