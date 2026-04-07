use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process;

use clap::{Parser, ValueEnum};
use orv_analyzer::dump_hir;
use orv_compiler::{
    FrontendFailure, dump_workspace_graph, load_path, load_project_graph, load_workspace_hir,
};
use orv_diagnostics::render_diagnostics;
use orv_runtime::{
    AdapterKind, Request, RouteAction, compile_program, emit_build, execute_request,
    render_response,
    runner::{run_server, run_workspace},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum OutputFormat {
    Human,
    Json,
}

#[derive(Parser)]
#[command(name = "orv", version, about = "Integrated Platform Development DSL")]
struct Cli {
    /// Path to a .orv file to run directly (like `node app.js`)
    file: Option<PathBuf>,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(clap::Subcommand)]
enum Commands {
    /// Display version information
    Version,
    /// Initialize a new orv project
    Init {
        /// Project name (defaults to current directory name)
        name: Option<String>,
    },
    /// Check a source file for errors
    Check {
        /// Path to the .orv source file
        file: PathBuf,
        /// Output format
        #[arg(long, value_enum, default_value_t = OutputFormat::Human)]
        format: OutputFormat,
    },
    /// Execute a request through the reference runtime
    Run {
        /// Path to the .orv source file
        file: PathBuf,
        /// HTTP method to execute
        #[arg(long, default_value = "GET")]
        method: String,
        /// Request path to execute
        #[arg(long, default_value = "/")]
        path: String,
        /// Start a live HTTP server instead of executing a single request
        #[arg(long, default_value_t = false)]
        serve: bool,
    },
    /// Build a direct adapter binary and manifest
    Build {
        /// Path to the .orv source file
        file: PathBuf,
        /// Output directory for build artifacts
        #[arg(long, default_value = "dist")]
        output_dir: PathBuf,
        /// Build output kind
        #[arg(long, value_enum, default_value_t = BuildEmit::Dist)]
        emit: BuildEmit,
        /// Output format
        #[arg(long, value_enum, default_value_t = OutputFormat::Human)]
        format: OutputFormat,
    },
    /// Start a development server with live reloading
    Dev {
        /// Path to the .orv source file
        file: PathBuf,
        /// Port for the dev server (default: 3000)
        #[arg(long, default_value_t = 3000)]
        port: u16,
    },
    /// Format orv source files (placeholder)
    Fmt,
    /// Dump internal representations
    Dump {
        #[command(subcommand)]
        target: DumpTarget,
    },
}

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
    /// Dump lowered HIR for a source file
    Hir {
        /// Path to the .orv source file
        file: PathBuf,
    },
    /// Dump project graph for a source file
    ProjectGraph {
        /// Path to the .orv source file
        file: PathBuf,
    },
    /// Dump a stage-by-stage compile pipeline view
    Pipeline {
        /// Path to the .orv source file
        file: PathBuf,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum BuildEmit {
    NativeAdapter,
    ProjectGraph,
    Dist,
}

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();

    // `orv <file>` — run the file directly (like `node app.js`)
    if let Some(file) = &cli.file
        && cli.command.is_none()
    {
        run_file(file);
        return;
    }

    match cli.command {
        Some(Commands::Version) | None => {
            println!("orv {}", orv_core::version());
        }
        Some(Commands::Init { name }) => {
            run_init(name.as_deref());
        }
        Some(Commands::Check { file, format }) => {
            run_check(&file, format);
        }
        Some(Commands::Run {
            file,
            method,
            path,
            serve,
        }) => {
            if serve {
                run_live_server(&file);
            } else {
                run_runtime(&file, &method, &path);
            }
        }
        Some(Commands::Build {
            file,
            output_dir,
            emit,
            format,
        }) => {
            run_build(&file, &output_dir, emit, format);
        }
        Some(Commands::Dev { file, port }) => {
            run_dev(&file, port);
        }
        Some(Commands::Fmt) => {
            println!("orv fmt is not yet implemented. It will be available in a future release.");
        }
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
            DumpTarget::Hir { file } => {
                run_dump_hir(&file);
            }
            DumpTarget::ProjectGraph { file } => {
                run_dump_project_graph(&file);
            }
            DumpTarget::Pipeline { file } => {
                run_dump_pipeline(&file);
            }
        },
    }
}

fn run_init(name: Option<&str>) {
    let project_name = name
        .map(String::from)
        .or_else(|| {
            std::env::current_dir()
                .ok()
                .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
        })
        .unwrap_or_else(|| "my-app".to_owned());

    let src_dir = Path::new("src");
    if src_dir.exists() {
        eprintln!("error: src/ directory already exists");
        process::exit(1);
    }

    if let Err(e) = fs::create_dir_all(src_dir) {
        eprintln!("error: failed to create src/: {e}");
        process::exit(1);
    }

    let main_content = format!(
        r#"import components.Layout

@server {{
    @listen 3000

    @route GET / {{
        @serve @html {{
            Layout(title="{project_name}") {{
                @div {{
                    @h1 "Welcome to {project_name}"
                    @p "Edit src/main.orv to get started."
                }}
            }}
        }}
    }}
}}
"#
    );

    let layout_dir = Path::new("src/components");
    if let Err(e) = fs::create_dir_all(layout_dir) {
        eprintln!("error: failed to create src/components/: {e}");
        process::exit(1);
    }

    let layout_content = r#"pub define Layout(title: string) -> @html {
    @head {
        @title "{title}"
        @meta %charset="utf-8"
    }
    @body {
        @children
    }
}
"#;

    if let Err(e) = fs::write("src/main.orv", main_content) {
        eprintln!("error: failed to write src/main.orv: {e}");
        process::exit(1);
    }

    if let Err(e) = fs::write("src/components/Layout.orv", layout_content) {
        eprintln!("error: failed to write src/components/Layout.orv: {e}");
        process::exit(1);
    }

    println!("initialized orv project: {project_name}");
    println!("  src/main.orv");
    println!("  src/components/Layout.orv");
    println!();
    println!("run `orv check src/main.orv` to validate");
    println!("run `orv run src/main.orv` to execute");
}

fn failure_to_json_diagnostics(
    path: &Path,
    source_map: &orv_span::SourceMap,
    diag_list: &[orv_diagnostics::Diagnostic],
) -> Vec<serde_json::Value> {
    diag_list
        .iter()
        .map(|d| {
            let (file, line, col) = d
                .labels
                .iter()
                .find(|l| l.is_primary)
                .map(|l| {
                    let (name, ln, col) = source_map.resolve(l.span);
                    (name.to_owned(), (ln + 1) as u64, col as u64)
                })
                .unwrap_or_else(|| (path.display().to_string(), 0, 0));
            let severity = match d.severity {
                orv_diagnostics::Severity::Error => "error",
                orv_diagnostics::Severity::Warning => "warning",
                orv_diagnostics::Severity::Help => "help",
            };
            serde_json::json!({
                "severity": severity,
                "message": d.message,
                "file": file,
                "line": line,
                "column": col,
            })
        })
        .collect()
}

fn exit_with_json_diagnostics(
    path: &Path,
    source_map: orv_span::SourceMap,
    diagnostics: orv_diagnostics::DiagnosticBag,
) -> ! {
    let diag_list = diagnostics.into_vec();
    let json_diags = failure_to_json_diagnostics(path, &source_map, &diag_list);
    println!(
        "{}",
        serde_json::to_string(&serde_json::json!({
            "status": "error",
            "diagnostics": json_diags,
        }))
        .unwrap()
    );
    process::exit(1);
}

fn run_check(path: &Path, format: OutputFormat) {
    let loaded = match load_path(path) {
        Ok(loaded) => loaded,
        Err(failure) => {
            if format == OutputFormat::Json {
                let (source_map, diagnostics) = failure.into_parts();
                exit_with_json_diagnostics(path, source_map, diagnostics);
            } else {
                render_failure_and_exit(failure);
            }
        }
    };
    let parsed = match loaded.parse() {
        Ok(parsed) => parsed,
        Err(failure) => {
            if format == OutputFormat::Json {
                let (source_map, diagnostics) = failure.into_parts();
                exit_with_json_diagnostics(path, source_map, diagnostics);
            } else {
                render_failure_and_exit(failure);
            }
        }
    };
    let analysis = match parsed.analyze() {
        Ok(analysis) => analysis,
        Err(failure) => {
            if format == OutputFormat::Json {
                let (source_map, diagnostics) = failure.into_parts();
                exit_with_json_diagnostics(path, source_map, diagnostics);
            } else {
                render_failure_and_exit(failure);
            }
        }
    };
    let name = analysis.source_map().name(analysis.file_id());
    let items = analysis.module().items.len();
    let symbols = analysis.analysis().symbols.len();
    let scopes = analysis.analysis().scopes.len();
    match format {
        OutputFormat::Human => {
            println!(
                "check: {name} \u{2014} {items} items, {symbols} symbols, {scopes} scopes, ok"
            );
        }
        OutputFormat::Json => {
            println!(
                "{}",
                serde_json::to_string(&serde_json::json!({
                    "status": "ok",
                    "file": name,
                    "items": items,
                    "symbols": symbols,
                    "scopes": scopes,
                }))
                .unwrap()
            );
        }
    }
}

fn run_file(path: &Path) {
    let workspace = match load_workspace_hir(path) {
        Ok(ws) => ws,
        Err(failure) => render_failure_and_exit(failure),
    };
    let modules = workspace.modules;
    if let Err(error) = run_workspace(&modules, None) {
        eprintln!("error: {error}");
        process::exit(1);
    }
}

fn run_live_server(path: &Path) {
    let analysis = analyze_or_exit(path);
    if let Err(error) = run_server(&analysis.analysis().hir) {
        eprintln!("server error: {error}");
        process::exit(1);
    }
}

fn run_runtime(path: &Path, method: &str, request_path: &str) {
    let analysis = analyze_or_exit(path);
    let program = match compile_program(&analysis.analysis().hir) {
        Ok(program) => program,
        Err(error) => {
            eprintln!("runtime compile error: {error}");
            process::exit(1);
        }
    };
    let response = match execute_request(
        &program,
        &Request {
            method,
            path: request_path,
        },
    ) {
        Ok(response) => response,
        Err(error) => {
            eprintln!("runtime execution error: {error}");
            process::exit(1);
        }
    };
    print!("{}", render_response(&response));
}

fn run_dev(path: &Path, port: u16) {
    eprintln!("  orv dev server");
    eprintln!("  ─────────────────────────");
    eprintln!("  entry:  {}", path.display());
    eprintln!("  url:    http://localhost:{port}");
    eprintln!("  mode:   development (file watching enabled)");
    eprintln!("  ─────────────────────────");
    eprintln!();

    // Track source file modification time for restart-on-change
    let source_path = path.to_owned();
    let initial_modified = file_modified_time(&source_path);

    // Spawn a watcher thread that checks for file changes
    let watch_path = source_path.clone();
    std::thread::spawn(move || {
        let mut last_modified = initial_modified;
        loop {
            std::thread::sleep(std::time::Duration::from_secs(1));
            let current = file_modified_time(&watch_path);
            if current != last_modified {
                last_modified = current;
                eprintln!(
                    "  [orv dev] file changed: {}, restart the server to apply",
                    watch_path.display()
                );
            }
        }
    });

    // Also scan for .orv files in the same directory
    if let Some(parent) = source_path.parent() {
        let scan_dir = parent.to_owned();
        std::thread::spawn(move || {
            let mut file_times: HashMap<PathBuf, Option<std::time::SystemTime>> = HashMap::new();
            // Initial scan
            if let Ok(entries) = fs::read_dir(&scan_dir) {
                for entry in entries.flatten() {
                    let p = entry.path();
                    if p.extension().and_then(|e| e.to_str()) == Some("orv") {
                        file_times.insert(p.clone(), file_modified_time(&p));
                    }
                }
            }
            loop {
                std::thread::sleep(std::time::Duration::from_secs(2));
                if let Ok(entries) = fs::read_dir(&scan_dir) {
                    for entry in entries.flatten() {
                        let p = entry.path();
                        if p.extension().and_then(|e| e.to_str()) == Some("orv") {
                            let current = file_modified_time(&p);
                            let prev = file_times.get(&p).copied().flatten();
                            let curr = current;
                            if curr != prev {
                                file_times.insert(p.clone(), current);
                                eprintln!("  [orv dev] changed: {}", p.display());
                            }
                        }
                    }
                }
            }
        });
    }

    // Load the full workspace (entry + all transitively imported modules) so
    // that defines like @Home, @DS, @User, @NotFound are registered. The
    // legacy single-file path (analyze_or_exit + run_server_on_port) only saw
    // the entry module, which left every imported define unresolved and
    // produced empty/broken responses.
    let workspace = match load_workspace_hir(&source_path) {
        Ok(ws) => ws,
        Err(failure) => render_failure_and_exit(failure),
    };
    if let Err(error) = run_workspace(&workspace.modules, Some(port)) {
        eprintln!("dev server error: {error}");
        process::exit(1);
    }
}

fn file_modified_time(path: &Path) -> Option<std::time::SystemTime> {
    fs::metadata(path).ok().and_then(|m| m.modified().ok())
}

fn run_build(path: &Path, output_dir: &Path, emit: BuildEmit, format: OutputFormat) {
    let workspace_graph = match load_project_graph(path) {
        Ok(graph) => graph,
        Err(failure) => render_failure_and_exit(failure),
    };

    if emit == BuildEmit::ProjectGraph {
        if let Err(error) = fs::create_dir_all(output_dir) {
            eprintln!("build error: {error}");
            process::exit(1);
        }
        let output_path = output_dir.join("project-graph.json");
        if let Err(error) = write_project_graph(&workspace_graph, &output_path) {
            eprintln!("build error: {error}");
            process::exit(1);
        }
        match format {
            OutputFormat::Human => {
                println!("build: {}", path.display());
                println!("emit: project-graph");
                println!("output: {}", output_path.display());
            }
            OutputFormat::Json => {
                println!(
                    "{}",
                    serde_json::to_string(&serde_json::json!({
                        "status": "ok",
                        "artifacts": { "graph": output_path.display().to_string() }
                    }))
                    .unwrap()
                );
            }
        }
        return;
    }

    // Dist emit — workspace-graph based build (default)
    if emit == BuildEmit::Dist {
        let entry_name = path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "app".to_owned());
        // Load WorkspaceHir for HTML/CSS pre-rendering (best-effort)
        let workspace_hir = load_workspace_hir(path).ok();
        let result = match orv_compiler::emit::emit_build_output(
            &workspace_graph,
            &entry_name,
            output_dir,
            workspace_hir.as_ref(),
        ) {
            Ok(result) => result,
            Err(error) => {
                eprintln!("build error: {error}");
                process::exit(1);
            }
        };
        let graph_path = output_dir.join("project-graph.json");
        let _ = write_project_graph(&workspace_graph, &graph_path);
        match format {
            OutputFormat::Human => {
                println!("build: {}", path.display());
                println!("emit: dist");
                println!("manifest: {}", result.manifest_path.display());
                for output in &result.outputs {
                    println!("  {}: {}", output.kind, output.path);
                }
                println!("graph: {}", graph_path.display());
            }
            OutputFormat::Json => {
                println!(
                    "{}",
                    serde_json::to_string(&serde_json::json!({
                        "status": "ok",
                        "artifacts": {
                            "manifest": result.manifest_path.display().to_string(),
                            "output_dir": result.output_dir.display().to_string(),
                            "graph": graph_path.display().to_string(),
                        }
                    }))
                    .unwrap()
                );
            }
        }
        return;
    }

    // BuildEmit::NativeAdapter — single-file static compile path.
    // This path requires a simple @server with literal values.
    let analysis = analyze_or_exit(path);
    let program = match compile_program(&analysis.analysis().hir) {
        Ok(program) => program,
        Err(error) => {
            eprintln!("runtime compile error: {error}");
            eprintln!("hint: use `orv build --emit dist` for workspace-level builds");
            process::exit(1);
        }
    };
    let artifacts = match emit_build(&program, output_dir) {
        Ok(artifacts) => artifacts,
        Err(error) => {
            eprintln!("build error: {error}");
            process::exit(1);
        }
    };
    let graph_path = output_dir.join("project-graph.json");
    let _ = write_project_graph(&workspace_graph, &graph_path);
    match format {
        OutputFormat::Human => {
            println!("build: {}", path.display());
            println!("adapter: direct-match");
            println!("manifest: {}", artifacts.manifest_path.display());
            println!("source: {}", artifacts.adapter_source_path.display());
            println!("binary: {}", artifacts.binary_path.display());
            println!("graph: {}", graph_path.display());
        }
        OutputFormat::Json => {
            println!(
                "{}",
                serde_json::to_string(&serde_json::json!({
                    "status": "ok",
                    "artifacts": {
                        "manifest": artifacts.manifest_path.display().to_string(),
                        "source": artifacts.adapter_source_path.display().to_string(),
                        "binary": artifacts.binary_path.display().to_string(),
                        "graph": graph_path.display().to_string(),
                    }
                }))
                .unwrap()
            );
        }
    }
}

fn run_dump_tokens(path: &Path) {
    let loaded = load_or_exit(path);
    let source_map = loaded.source_map();
    let (tokens, diags) = loaded.tokenize();

    if diags.has_errors() {
        render_diagnostics(source_map, &diags.into_vec());
        process::exit(1);
    }

    for token in &tokens {
        let span = token.span();
        let (_, line, col) = source_map.resolve(span);
        println!("{:>4}:{:<3} {:?}", line + 1, col, token.node());
    }
}

fn run_dump_ast(path: &Path) {
    let parsed = match load_or_exit(path).parse() {
        Ok(parsed) => parsed,
        Err(failure) => render_failure_and_exit(failure),
    };
    println!("{}", parsed.dump_ast());
}

fn run_dump_source(path: &Path) {
    let loaded = load_or_exit(path);
    let source_map = loaded.source_map();
    let name = source_map.name(loaded.file_id());
    let source = source_map.source(loaded.file_id());
    let line_count = source_map.line_index(loaded.file_id()).line_count();
    let byte_count = source.len();
    println!("file: {name}");
    println!("file_id: {}", loaded.file_id().raw());
    println!("bytes: {byte_count}");
    println!("lines: {line_count}");
}

fn run_dump_hir(path: &Path) {
    let analysis = analyze_or_exit(path);
    println!("{}", dump_hir(&analysis.analysis().hir));
}

fn run_dump_project_graph(path: &Path) {
    let graph = match load_project_graph(path) {
        Ok(graph) => graph,
        Err(failure) => render_failure_and_exit(failure),
    };
    println!("{}", dump_workspace_graph(&graph));
}

fn run_dump_pipeline(path: &Path) {
    let loaded = load_or_exit(path);
    let source_name = loaded.source_name().to_owned();
    let source_len = loaded.source().len();
    let line_count = loaded
        .source_map()
        .line_index(loaded.file_id())
        .line_count();
    let (tokens, diagnostics) = loaded.tokenize();
    if diagnostics.has_errors() {
        render_diagnostics(loaded.source_map(), &diagnostics.into_vec());
        process::exit(1);
    }

    let token_count = tokens.len();
    let parsed = match loaded.parse() {
        Ok(parsed) => parsed,
        Err(failure) => render_failure_and_exit(failure),
    };
    let item_count = parsed.module().items.len();
    let analysis = match parsed.analyze() {
        Ok(analysis) => analysis,
        Err(failure) => render_failure_and_exit(failure),
    };

    let mut out = String::new();
    out.push_str("Compile Pipeline\n");
    out.push_str("================\n");
    out.push_str("1. Load     OK\n");
    out.push_str(&format!("   file: {source_name}\n"));
    out.push_str(&format!("   bytes: {source_len}\n"));
    out.push_str(&format!("   lines: {line_count}\n"));
    out.push_str("2. Lex      OK\n");
    out.push_str(&format!("   tokens: {token_count}\n"));
    out.push_str("3. Parse    OK\n");
    out.push_str(&format!("   items: {item_count}\n"));
    out.push_str("4. Analyze  OK\n");
    out.push_str(&format!(
        "   symbols: {}\n",
        analysis.analysis().symbols.len()
    ));
    out.push_str(&format!(
        "   scopes: {}\n",
        analysis.analysis().scopes.len()
    ));
    let graph = match load_project_graph(path) {
        Ok(graph) => graph,
        Err(failure) => render_failure_and_exit(failure),
    };
    out.push_str("5. Graph    OK\n");
    out.push_str(&format!("   modules: {}\n", graph.modules.len()));
    out.push_str(&format!(
        "   pages: {}\n",
        graph
            .modules
            .iter()
            .map(|module| module.pages.len())
            .sum::<usize>()
    ));
    out.push_str(&format!(
        "   signals: {}\n",
        graph
            .modules
            .iter()
            .map(|module| module.signals.len())
            .sum::<usize>()
    ));
    out.push_str(&format!(
        "   routes: {}\n",
        graph
            .modules
            .iter()
            .map(|module| module.routes.len())
            .sum::<usize>()
    ));
    out.push_str(&format!(
        "   fetches: {}\n",
        graph
            .modules
            .iter()
            .map(|module| module.fetches.len())
            .sum::<usize>()
    ));

    match compile_program(&analysis.analysis().hir) {
        Ok(program) => {
            out.push_str("6. Runtime  OK\n");
            out.push_str(&format!(
                "   adapter: {}\n",
                match program.adapter {
                    AdapterKind::DirectMatch => "direct-match",
                }
            ));
            out.push_str(&format!("   listen: {}\n", program.server.listen));
            out.push_str(&format!("   routes: {}\n", program.server.routes.len()));
            for route in &program.server.routes {
                out.push_str(&format!(
                    "   - {} {} -> {}\n",
                    route.method,
                    route.path,
                    describe_route_action(&route.action)
                ));
            }
            out.push_str("7. Build    READY\n");
            out.push_str("   backend: direct native adapter via rustc -O\n");
            out.push_str(
                "   outputs: program.json, direct_adapter.rs, orv-app, project-graph.json\n",
            );
        }
        Err(error) => {
            out.push_str("6. Runtime  SKIPPED\n");
            out.push_str(&format!("   reason: {error}\n"));
            out.push_str("7. Build    SKIPPED\n");
            out.push_str("   reason: runtime program could not be lowered\n");
        }
    }

    print!("{out}");
}

fn load_or_exit(path: &Path) -> orv_compiler::LoadedUnit {
    match load_path(path) {
        Ok(loaded) => loaded,
        Err(failure) => render_failure_and_exit(failure),
    }
}

fn analyze_or_exit(path: &Path) -> orv_compiler::AnalyzedUnit {
    let parsed = match load_or_exit(path).parse() {
        Ok(parsed) => parsed,
        Err(failure) => render_failure_and_exit(failure),
    };
    match parsed.analyze() {
        Ok(analysis) => analysis,
        Err(failure) => render_failure_and_exit(failure),
    }
}

fn render_failure_and_exit(failure: FrontendFailure) -> ! {
    let (source_map, diagnostics) = failure.into_parts();
    render_diagnostics(&source_map, &diagnostics.into_vec());
    process::exit(1);
}

fn describe_route_action(action: &RouteAction) -> &'static str {
    match action {
        RouteAction::JsonResponse { .. } => "@respond json",
        RouteAction::StaticServe { .. } => "@serve static",
        RouteAction::HtmlServe { .. } => "@serve html",
    }
}

fn write_project_graph(
    graph: &orv_compiler::WorkspaceGraph,
    output_path: &Path,
) -> Result<(), std::io::Error> {
    let json = serde_json::to_vec_pretty(graph)
        .map_err(|error| std::io::Error::other(error.to_string()))?;
    fs::write(output_path, json)
}
