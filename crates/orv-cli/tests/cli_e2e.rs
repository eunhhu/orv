use std::path::PathBuf;
use std::process::Command;
use std::{
    fs,
    time::{SystemTime, UNIX_EPOCH},
};

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .expect("workspace root")
        .to_path_buf()
}

fn fixture_path(relative: &str) -> PathBuf {
    workspace_root().join(relative)
}

fn run_orv(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_orv"))
        .args(args)
        .current_dir(workspace_root())
        .output()
        .expect("orv CLI should run")
}

fn temp_dir(prefix: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time should move forward")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("{prefix}-{unique}"));
    fs::create_dir_all(&path).expect("temp dir should be created");
    path
}

#[test]
fn check_hello_fixture_succeeds() {
    let fixture = fixture_path("fixtures/ok/hello.orv");
    let output = run_orv(&["check", fixture.to_str().expect("utf-8 path")]);
    assert!(output.status.success(), "{output:?}");

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("check:"));
    assert!(stdout.contains("items"));
    assert!(stdout.contains("scopes"));
}

#[test]
fn dump_hir_counter_fixture_contains_lowered_scopes() {
    let fixture = fixture_path("fixtures/ok/counter.orv");
    let output = run_orv(&["dump", "hir", fixture.to_str().expect("utf-8 path")]);
    assert!(output.status.success(), "{output:?}");

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("Define CounterPage scope#1"));
    assert!(stdout.contains("block scope#2"));
    assert!(stdout.contains("count@symbol#1"));
}

#[test]
fn check_unresolved_program_fails_with_diagnostic() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time should move forward")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("orv-cli-e2e-{unique}.orv"));
    fs::write(&path, "function fail() -> missing\n").expect("temp source should be written");

    let output = run_orv(&["check", path.to_str().expect("utf-8 path")]);
    let _ = fs::remove_file(&path);
    assert!(!output.status.success(), "{output:?}");

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf-8");
    assert!(stderr.contains("error"));
    assert!(stderr.contains("unresolved name `missing`"));
}

#[test]
fn check_hash_map_program_succeeds() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time should move forward")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("orv-cli-map-ok-{unique}.orv"));
    fs::write(
        &path,
        "let scores: HashMap<string, i32> = #{ alice: 1, bob: 2 }\nlet count: i32 = scores.len()\nlet keys: Vec<string> = scores.keys()\nlet values: Vec<i32> = scores.values()\n",
    )
    .expect("temp source should be written");

    let output = run_orv(&["check", path.to_str().expect("utf-8 path")]);
    let _ = fs::remove_file(&path);
    assert!(output.status.success(), "{output:?}");
}

#[test]
fn check_empty_map_without_context_reports_type_error() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time should move forward")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("orv-cli-map-err-{unique}.orv"));
    fs::write(&path, "let scores = #{}\n").expect("temp source should be written");

    let output = run_orv(&["check", path.to_str().expect("utf-8 path")]);
    let _ = fs::remove_file(&path);
    assert!(!output.status.success(), "{output:?}");

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf-8");
    assert!(stderr.contains("cannot infer the value type of an empty map literal"));
}

#[test]
fn check_server_fixture_succeeds() {
    let fixture = fixture_path("fixtures/ok/server-basic.orv");
    let output = run_orv(&["check", fixture.to_str().expect("utf-8 path")]);
    assert!(output.status.success(), "{output:?}");

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("check:"));
    assert!(stdout.contains("ok"));
}

#[test]
fn invalid_html_node_in_server_reports_domain_error() {
    let fixture = fixture_path("fixtures/err/domain-html-in-server.orv");
    let output = run_orv(&["check", fixture.to_str().expect("utf-8 path")]);
    assert!(!output.status.success(), "{output:?}");

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf-8");
    assert!(stderr.contains("node `@div` is not valid in @server context"));
}

#[test]
fn invalid_route_node_in_html_reports_domain_error() {
    let fixture = fixture_path("fixtures/err/domain-route-in-ui.orv");
    let output = run_orv(&["check", fixture.to_str().expect("utf-8 path")]);
    assert!(!output.status.success(), "{output:?}");

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf-8");
    assert!(stderr.contains("node `@route` is not valid in @html context"));
}

#[test]
fn run_server_fixture_executes_direct_adapter_path() {
    let fixture = fixture_path("fixtures/ok/server-basic.orv");
    let output = run_orv(&[
        "run",
        fixture.to_str().expect("utf-8 path"),
        "--method",
        "GET",
        "--path",
        "/api/health",
    ]);
    assert!(output.status.success(), "{output:?}");

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("adapter: direct-match"));
    assert!(stdout.contains("status: 200"));
    assert!(stdout.contains("content-type: application/json"));
    assert!(stdout.contains(r#"body: {"status":"ok"}"#));
}

#[test]
fn build_server_fixture_emits_native_binary_that_runs() {
    let fixture = fixture_path("fixtures/ok/server-basic.orv");
    let output_dir = temp_dir("orv-build-e2e");
    let output = run_orv(&[
        "build",
        fixture.to_str().expect("utf-8 path"),
        "--output-dir",
        output_dir.to_str().expect("utf-8 path"),
    ]);
    assert!(output.status.success(), "{output:?}");

    let binary = output_dir.join(format!("orv-app{}", std::env::consts::EXE_SUFFIX));
    assert!(
        binary.exists(),
        "binary should exist at {}",
        binary.display()
    );
    assert!(output_dir.join("program.json").exists());
    assert!(output_dir.join("direct_adapter.rs").exists());
    assert!(output_dir.join("project-graph.json").exists());

    let built = Command::new(&binary)
        .args(["GET", "/api/health"])
        .output()
        .expect("built adapter should run");
    assert!(built.status.success(), "{built:?}");

    let stdout = String::from_utf8(built.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("adapter: direct-match"));
    assert!(stdout.contains("status: 200"));
    assert!(stdout.contains(r#"body: {"status":"ok"}"#));

    let _ = fs::remove_dir_all(&output_dir);
}

#[test]
fn build_project_graph_emits_json_summary() {
    let fixture = fixture_path("fixtures/ok/counter.orv");
    let output_dir = temp_dir("orv-project-graph-e2e");
    let output = run_orv(&[
        "build",
        fixture.to_str().expect("utf-8 path"),
        "--emit",
        "project-graph",
        "--output-dir",
        output_dir.to_str().expect("utf-8 path"),
    ]);
    assert!(output.status.success(), "{output:?}");

    let graph_path = output_dir.join("project-graph.json");
    assert!(graph_path.exists(), "project graph should exist");

    let json = fs::read_to_string(&graph_path).expect("project graph should be readable");
    assert!(json.contains("\"module\""));
    assert!(json.contains("\"pages\""));
    assert!(json.contains("\"signals\""));
    assert!(json.contains("\"CounterPage\""));

    let _ = fs::remove_dir_all(&output_dir);
}

#[test]
fn dump_pipeline_server_fixture_shows_stage_graph() {
    let fixture = fixture_path("fixtures/ok/server-basic.orv");
    let output = run_orv(&["dump", "pipeline", fixture.to_str().expect("utf-8 path")]);
    assert!(output.status.success(), "{output:?}");

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("Compile Pipeline"));
    assert!(stdout.contains("1. Load     OK"));
    assert!(stdout.contains("2. Lex      OK"));
    assert!(stdout.contains("3. Parse    OK"));
    assert!(stdout.contains("4. Analyze  OK"));
    assert!(stdout.contains("5. Graph    OK"));
    assert!(stdout.contains("6. Runtime  OK"));
    assert!(stdout.contains("7. Build    READY"));
    assert!(stdout.contains("- GET /api/health -> @response json"));
}

#[test]
fn dump_pipeline_ui_fixture_marks_runtime_as_skipped() {
    let fixture = fixture_path("fixtures/ok/counter.orv");
    let output = run_orv(&["dump", "pipeline", fixture.to_str().expect("utf-8 path")]);
    assert!(output.status.success(), "{output:?}");

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("Compile Pipeline"));
    assert!(stdout.contains("4. Analyze  OK"));
    assert!(stdout.contains("5. Graph    OK"));
    assert!(stdout.contains("6. Runtime  SKIPPED"));
    assert!(stdout.contains("7. Build    SKIPPED"));
}

#[test]
fn dump_project_graph_counter_fixture_shows_pages_and_signals() {
    let fixture = fixture_path("fixtures/ok/counter.orv");
    let output = run_orv(&[
        "dump",
        "project-graph",
        fixture.to_str().expect("utf-8 path"),
    ]);
    assert!(output.status.success(), "{output:?}");

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("Project Graph"));
    assert!(stdout.contains("pages: 1"));
    assert!(stdout.contains("signals: 1"));
    assert!(stdout.contains("page CounterPage (html)"));
    assert!(stdout.contains("signal CounterPage.count deps: none"));
}
