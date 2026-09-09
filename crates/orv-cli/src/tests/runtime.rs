use super::*;

#[test]
fn check_accepts_all_e2e_fixtures() {
    let files = orv_files_under(&["fixtures", "e2e"]);
    assert!(!files.is_empty(), "expected e2e fixtures");
    for file in files {
        cmd_check(&file).unwrap_or_else(|e| panic!("{}: {e}", file.display()));
    }
}

#[test]
fn check_accepts_plan_and_default_fixtures() {
    let mut files = orv_files_under(&["fixtures", "plan"]);
    files.push(workspace_path(&["fixtures", "default-syntax.orv"]));
    assert!(!files.is_empty(), "expected plan fixtures");
    for file in files {
        cmd_check(&file).unwrap_or_else(|e| panic!("{}: {e}", file.display()));
    }
}

#[test]
fn test_subcommand_is_accepted() {
    let parsed = Cli::try_parse_from(["orv", "test", "src/models", "--filter", "user"]);
    if let Err(err) = parsed {
        panic!("{}", err.render());
    }
}

#[test]
fn test_list_flag_is_accepted() {
    let parsed = Cli::try_parse_from(["orv", "test", "--list", "src/models"]);
    let cli = match parsed {
        Ok(cli) => cli,
        Err(err) => panic!("{}", err.render()),
    };
    match cli.command {
        Command::Test { path, filter, list } => {
            assert_eq!(path, PathBuf::from("src/models"));
            assert_eq!(filter, None);
            assert!(list);
        }
        _ => panic!("unexpected command"),
    }
}

#[test]
fn test_list_json_discovers_filtered_tests_without_running_them() {
    let dir = temp_output_dir("test-runner-list");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let source = dir.join("checkout_test.orv");
    std::fs::write(
        &source,
        r#"test "checkout shows cart" {
  assert true
}

test "checkout failing runtime body" {
  assert false
}
"#,
    )
    .expect("write test source");

    let value = orv_test_list_json(&dir, Some("shows")).expect("test list");
    let tests = value["tests"].as_array().expect("tests array");

    assert_eq!(value["schema_version"], 1);
    assert_eq!(tests.len(), 1);
    assert_eq!(tests[0]["name"], "checkout shows cart");
    assert_eq!(tests[0]["path"], source.display().to_string());
    assert_eq!(tests[0]["line"], 1);
    assert_eq!(tests[0]["column"], 1);
    assert_eq!(tests[0]["span"]["start"], 0);
    assert!(tests[0]["span"]["end"].as_u64().is_some_and(|end| end > 0));
    assert_eq!(tests[0]["range"]["start"]["line"], 0);
    assert_eq!(tests[0]["range"]["start"]["character"], 0);
    assert_eq!(tests[0]["range"]["end"]["line"], 2);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn test_summary_discovers_and_runs_matching_tests() {
    let dir = temp_output_dir("test-runner-pass");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let source = dir.join("math_test.orv");
    std::fs::write(
        &source,
        r#"test "math adds" {
  assert 1 + 2 == 3
}
"#,
    )
    .expect("write test source");

    let summary = orv_test_summary(&dir, Some("math")).expect("test summary");

    assert_eq!(summary.selected, 1);
    assert_eq!(summary.passed, 1);
    assert_eq!(summary.failed, 0);
    assert!(summary.files.iter().any(|file| file == &source));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn test_summary_reports_runtime_failures() {
    let dir = temp_output_dir("test-runner-fail");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let source = dir.join("math_test.orv");
    std::fs::write(
        &source,
        r#"test "math fails" {
  assert 1 + 2 == 4
}
"#,
    )
    .expect("write test source");

    let err = orv_test_summary(&dir, None).expect_err("failing test should fail");

    assert!(err.to_string().contains("math_test.orv"));
    assert!(err.to_string().contains("assertion failed"));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn check_artifact_subcommand_is_accepted() {
    let parsed = Cli::try_parse_from([
        "orv",
        "check-artifact",
        "target/orv-build-test/server/app.orv-runtime.json",
    ]);
    if let Err(err) = parsed {
        panic!("{}", err.render());
    }
}

#[test]
fn check_build_subcommand_is_accepted() {
    let parsed = Cli::try_parse_from(["orv", "check-build", "target/orv-build-test"]);
    if let Err(err) = parsed {
        panic!("{}", err.render());
    }
}

#[test]
fn run_artifact_subcommand_is_accepted() {
    let parsed = Cli::try_parse_from([
        "orv",
        "run-artifact",
        "target/orv-build-test/server/app.orv-runtime.json",
    ]);
    if let Err(err) = parsed {
        panic!("{}", err.render());
    }
}

#[test]
fn run_artifact_trace_option_is_accepted() {
    let parsed = Cli::try_parse_from([
        "orv",
        "run-artifact",
        "target/orv-build-test/server/app.orv-runtime.json",
        "--trace",
        "target/orv-request-trace.json",
    ])
    .unwrap_or_else(|err| panic!("{}", err.render()));
    let Command::RunArtifact { trace, .. } = parsed.command else {
        panic!("expected run-artifact command");
    };
    assert_eq!(trace, Some(PathBuf::from("target/orv-request-trace.json")));
}

#[test]
fn run_build_subcommand_is_accepted() {
    let parsed = Cli::try_parse_from(["orv", "run-build", "target/orv-build-test"]);
    if let Err(err) = parsed {
        panic!("{}", err.render());
    }
}

#[test]
fn run_build_trace_option_is_accepted() {
    let parsed = Cli::try_parse_from([
        "orv",
        "run-build",
        "target/orv-build-test",
        "--trace",
        "target/orv-request-trace.json",
    ])
    .unwrap_or_else(|err| panic!("{}", err.render()));
    let Command::RunBuild { trace, .. } = parsed.command else {
        panic!("expected run-build command");
    };
    assert_eq!(trace, Some(PathBuf::from("target/orv-request-trace.json")));
}

#[test]
fn run_build_executes_server_launch_artifact_relative_to_build_dir() {
    let out = temp_output_dir("run-build");
    let artifact = out.join("server").join("app.orv-runtime.json");
    write_reference_artifact(&artifact, "artifact.orv", r#"@out "build ok""#);
    let launch = orv_compiler::ServerLaunchArtifact {
        schema_version: orv_compiler::SERVER_LAUNCH_ARTIFACT_VERSION,
        runtime: "reference-interpreter".to_string(),
        artifact: "server/app.orv-runtime.json".to_string(),
        command: vec![
            "orv".to_string(),
            "run-artifact".to_string(),
            "server/app.orv-runtime.json".to_string(),
        ],
        protocol: "http1".to_string(),
        routes: Vec::new(),
        listen: None,
    };
    write_json(
        &out.join("server").join("launch.json"),
        &serde_json::to_value(launch).expect("launch value"),
    )
    .expect("write launch");
    let mut stdout = Vec::new();

    run_build_with_writer(&out, &mut stdout).expect("run build");

    assert_eq!(
        String::from_utf8(stdout).expect("stdout utf-8"),
        "build ok\n"
    );
    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn run_build_resolves_relative_persistence_under_build_dir() {
    let out = temp_output_dir("run-build-persistence-cwd");
    let unique = std::process::id();
    let sqlite_name = format!("orv-run-build-cwd-{unique}.sqlite");
    let record_name = format!("orv-run-build-cwd-{unique}.jsonl");
    let cwd_data = std::env::current_dir().expect("cwd").join("data");
    let cwd_sqlite = cwd_data.join(&sqlite_name);
    let cwd_record = cwd_data.join(&record_name);
    let _ = std::fs::remove_file(&cwd_sqlite);
    let _ = std::fs::remove_file(&cwd_record);
    let source = format!(
        r#"let db = @db.connect "sqlite://data/{sqlite_name}"
await db.create("Item", {{ name: "ok" }})
let payments = @payment.connect("file://data/{record_name}")
payments.capture({{ orderId: 1, amount: 100, method: "card" }})
@out "ok""#
    );
    let artifact = out.join("server").join("app.orv-runtime.json");
    write_reference_artifact(&artifact, "artifact.orv", &source);
    let launch = orv_compiler::ServerLaunchArtifact {
        schema_version: orv_compiler::SERVER_LAUNCH_ARTIFACT_VERSION,
        runtime: "reference-interpreter".to_string(),
        artifact: "server/app.orv-runtime.json".to_string(),
        command: vec![
            "orv".to_string(),
            "run-artifact".to_string(),
            "server/app.orv-runtime.json".to_string(),
        ],
        protocol: "http1".to_string(),
        routes: Vec::new(),
        listen: None,
    };
    write_json(
        &out.join("server").join("launch.json"),
        &serde_json::to_value(launch).expect("launch value"),
    )
    .expect("write launch");
    let mut stdout = Vec::new();

    run_build_with_writer(&out, &mut stdout).expect("run build");

    assert_eq!(String::from_utf8(stdout).expect("stdout utf8"), "ok\n");
    assert!(out.join("data").join(&sqlite_name).is_file());
    assert!(out.join("data").join(&record_name).is_file());
    assert!(!cwd_sqlite.exists());
    assert!(!cwd_record.exists());
    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn run_build_prints_zero_runtime_static_page() {
    let out = temp_output_dir("run-build-static");
    std::fs::create_dir_all(&out).expect("create temp root");
    let entry = out.join("page.orv");
    std::fs::write(&entry, r#"@out @html { @body { @h1 "Static" } }"#).expect("write entry");
    let build_out = out.join("dist");
    cmd_build(&entry, &build_out).expect("build artifacts");
    let mut stdout = Vec::new();

    run_build_with_writer(&build_out, &mut stdout).expect("run build");

    assert_eq!(
        String::from_utf8(stdout).expect("stdout utf-8"),
        "<html><body><h1>Static</h1></body></html>"
    );
    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn run_build_prints_client_page_shell() {
    let out = temp_output_dir("run-build-client-page");
    std::fs::create_dir_all(&out).expect("create temp root");
    let entry = out.join("page.orv");
    std::fs::write(
        &entry,
        "let sig count: int = 0\n@out @html { @body { @p count } }",
    )
    .expect("write entry");
    let build_out = out.join("dist");
    cmd_build(&entry, &build_out).expect("build artifacts");
    let mut stdout = Vec::new();

    run_build_with_writer(&build_out, &mut stdout).expect("run build");

    let html = String::from_utf8(stdout).expect("stdout utf-8");
    assert!(html.contains("data-orv-client=\"wasm\""));
    assert!(html.contains("../client/app.js"));
    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn run_build_uses_bundle_plan_instead_of_stale_server_launcher() {
    let out = temp_output_dir("run-build-static-stale-server");
    std::fs::create_dir_all(&out).expect("create temp root");
    let entry = out.join("page.orv");
    std::fs::write(&entry, r#"@out @html { @body { @h1 "Fresh" } }"#).expect("write entry");
    let build_out = out.join("dist");
    cmd_build(&entry, &build_out).expect("build artifacts");
    let stale_launch = build_out.join("server").join("launch.json");
    if let Some(parent) = stale_launch.parent() {
        std::fs::create_dir_all(parent).expect("create stale server dir");
    }
    std::fs::write(&stale_launch, "{ stale").expect("write stale launch");
    let mut stdout = Vec::new();

    run_build_with_writer(&build_out, &mut stdout).expect("run build");

    assert_eq!(
        String::from_utf8(stdout).expect("stdout utf-8"),
        "<html><body><h1>Fresh</h1></body></html>"
    );
    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn check_artifact_rehydrates_generated_server_runtime_artifact() {
    let path = workspace_path(&["fixtures", "e2e", "hello.orv"]);
    let out = temp_output_dir("check-artifact");

    cmd_build(&path, &out).expect("build artifacts");
    let artifact = out.join("server").join("app.orv-runtime.json");

    cmd_check_artifact(&artifact).expect("check artifact");

    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn run_artifact_rehydrates_and_runs_source_bundle() {
    let out = temp_output_dir("run-artifact");
    let artifact = out.join("app.orv-runtime.json");
    write_reference_artifact(&artifact, "artifact.orv", r#"@out "artifact ok""#);
    let mut stdout = Vec::new();

    run_artifact_with_writer(&artifact, &mut stdout).expect("run artifact");

    assert_eq!(
        String::from_utf8(stdout).expect("stdout utf-8"),
        "artifact ok\n"
    );
    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn run_artifact_rehydrates_imported_source_bundle() {
    let out = temp_output_dir("run-artifact-import");
    let artifact = out.join("app.orv-runtime.json");
    write_reference_artifact_with_sources(
        &artifact,
        "main.orv",
        [
            (
                "main.orv",
                "import models.user.User\nlet u: User = { name: \"Ada\" }\n@out u.name",
            ),
            ("models/user.orv", "pub struct User { name: string }"),
        ],
    );
    let mut stdout = Vec::new();

    run_artifact_with_writer(&artifact, &mut stdout).expect("run artifact");

    assert_eq!(String::from_utf8(stdout).expect("stdout utf-8"), "Ada\n");
    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn run_artifact_rejects_corrupt_source_bundle() {
    let out = temp_output_dir("run-artifact-corrupt");
    let artifact_path = out.join("app.orv-runtime.json");
    write_reference_artifact(&artifact_path, "artifact.orv", r#"@out "artifact ok""#);
    let mut artifact: orv_compiler::ServerRuntimeArtifact =
        serde_json::from_str(&std::fs::read_to_string(&artifact_path).expect("artifact json"))
            .expect("artifact");
    artifact.source_bundle.files[0].source = r#"@out "tampered""#.to_string();
    write_json(
        &artifact_path,
        &serde_json::to_value(artifact).expect("artifact value"),
    )
    .expect("write artifact");
    let mut stdout = Vec::new();

    let err = run_artifact_with_writer(&artifact_path, &mut stdout).expect_err("hash mismatch");

    assert!(err.to_string().contains("content hash mismatch"));
    assert!(stdout.is_empty());
    let _ = std::fs::remove_dir_all(&out);
}
