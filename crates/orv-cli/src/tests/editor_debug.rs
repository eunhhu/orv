use super::*;

#[test]
fn editor_debug_subcommand_is_accepted() {
    let parsed = Cli::try_parse_from([
        "orv",
        "editor",
        "debug",
        "fixtures/e2e/hello.orv",
        "--control",
        "next",
    ]);
    if let Err(err) = parsed {
        panic!("{}", err.render());
    }
}

#[test]
fn editor_debug_subcommand_accepts_control_sequence() {
    let parsed = Cli::try_parse_from([
        "orv",
        "editor",
        "debug",
        "fixtures/e2e/hello.orv",
        "--control",
        "next",
        "--control",
        "next",
    ]);
    if let Err(err) = parsed {
        panic!("{}", err.render());
    }
}

#[test]
fn editor_debug_subcommand_accepts_watch_expression() {
    let parsed = Cli::try_parse_from([
        "orv",
        "editor",
        "debug",
        "fixtures/e2e/hello.orv",
        "--watch-expression",
        "runtimeStatus",
    ])
    .unwrap_or_else(|err| panic!("{}", err.render()));
    let Command::Editor {
        command: EditorCommand::Debug {
            watch_expressions, ..
        },
    } = parsed.command
    else {
        panic!("expected editor debug command");
    };
    assert_eq!(watch_expressions, vec!["runtimeStatus".to_string()]);
}

#[test]
fn editor_debug_subcommand_accepts_function_breakpoint() {
    let parsed = Cli::try_parse_from([
        "orv",
        "editor",
        "debug",
        "fixtures/e2e/hello.orv",
        "--function-breakpoint",
        "add",
    ])
    .unwrap_or_else(|err| panic!("{}", err.render()));
    let Command::Editor {
        command: EditorCommand::Debug {
            function_breakpoints,
            ..
        },
    } = parsed.command
    else {
        panic!("expected editor debug command");
    };
    assert_eq!(function_breakpoints, vec!["add".to_string()]);
}

#[test]
fn editor_debug_subcommand_accepts_data_breakpoint() {
    let parsed = Cli::try_parse_from([
        "orv",
        "editor",
        "debug",
        "fixtures/e2e/hello.orv",
        "--data-breakpoint",
        "total",
    ])
    .unwrap_or_else(|err| panic!("{}", err.render()));
    let Command::Editor {
        command: EditorCommand::Debug {
            data_breakpoints, ..
        },
    } = parsed.command
    else {
        panic!("expected editor debug command");
    };
    assert_eq!(data_breakpoints, vec!["total".to_string()]);
}

#[test]
fn editor_debug_subcommand_accepts_exception_filter() {
    let parsed = Cli::try_parse_from([
        "orv",
        "editor",
        "debug",
        "fixtures/e2e/hello.orv",
        "--exception-filter",
        "orv.runtime",
    ])
    .unwrap_or_else(|err| panic!("{}", err.render()));
    let Command::Editor {
        command: EditorCommand::Debug {
            exception_filters, ..
        },
    } = parsed.command
    else {
        panic!("expected editor debug command");
    };
    assert_eq!(exception_filters, vec!["orv.runtime".to_string()]);
}

#[test]
fn editor_run_debug_subcommand_accepts_exported_state() {
    let parsed = Cli::try_parse_from([
        "orv",
        "editor",
        "run-debug",
        "target/orv-editor/state.json",
        "--control",
        "next",
        "--control",
        "step-in",
        "--watch-expression",
        "stdout",
    ]);
    if let Err(err) = parsed {
        panic!("{}", err.render());
    }
}

#[test]
fn verify_build_rejects_deploy_preflight_editor_run_debug_command_mismatch() {
    let (src_dir, path) = prod_server_source("deploy-preflight-run-debug-source");
    let out = temp_output_dir("deploy-preflight-run-debug-mismatch");

    cmd_build_with_profile(&path, &out, BuildProfile::Production).expect("prod build");
    let preflight_path = out.join("deploy").join("preflight.json");
    let mut preflight = read_json_value(&preflight_path).expect("preflight");
    preflight["commands"]["editor_run_debug"] =
        serde_json::json!("orv editor run-debug other --control next");
    write_json(&preflight_path, &preflight).expect("write corrupt preflight");

    let err = cmd_verify_build(&out).expect_err("preflight editor run-debug mismatch");

    assert!(err
        .to_string()
        .contains("deploy preflight editor_run_debug command"));
    let _ = std::fs::remove_dir_all(src_dir);
    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn editor_export_debug_source_inventory_tracks_imports() {
    let dir = temp_output_dir("editor-export-debug-sources");
    let models = dir.join("models");
    std::fs::create_dir_all(&models).expect("create models dir");
    let path = dir.join("app.orv");
    let imported = models.join("user.orv");
    let imported_source = "pub struct User { id: int }\n";
    std::fs::write(
        &path,
        "import models.user.User\nlet user: User = { id: 1 }\n@out \"ok\"\n",
    )
    .expect("write source");
    std::fs::write(&imported, imported_source).expect("write imported source");
    let out = dir.join("editor");

    cmd_editor_export(&path, &out).expect("editor export");
    let state = read_json_value(&out.join("state.json")).expect("editor state");
    let native_host =
        read_json_value(&out.join(EDITOR_NATIVE_HOST_MANIFEST_PATH)).expect("native host");
    let run = editor_debug_runner_session_json(
        &out.join(EDITOR_DEBUG_SESSION_RUNNER_PATH),
        &[EditorDebugControl::Next],
        &[],
        &[],
        &[],
        &[],
        &[],
    )
    .expect("run debug source inventory");

    assert_eq!(state["debug"]["source_inventory"]["source_count"], 2);
    assert_eq!(
        native_host["debug"]["source_inventory"],
        state["debug"]["source_inventory"]
    );
    assert!(state["debug"]["source_inventory"]["sources"]
        .as_array()
        .expect("source inventory")
        .iter()
        .any(|source| {
            source["source"]["name"] == "user.orv"
                && source["checksum"]["value"]
                    == serde_json::json!(sha256_hex(imported_source.as_bytes()))
                && source["request"]["command"] == "source"
        }));
    assert!(run["debug"]["loaded_sources"]["sources"]
        .as_array()
        .expect("loaded sources")
        .iter()
        .any(|source| {
            source["name"] == "user.orv"
                && source["checksums"][0]["checksum"]
                    == serde_json::json!(sha256_hex(imported_source.as_bytes()))
        }));
    assert!(run["debug"]["source_snapshots"]
        .as_array()
        .expect("source snapshots")
        .iter()
        .any(|snapshot| {
            snapshot["source"]["name"] == "user.orv"
                && snapshot["response"]["success"] == true
                && snapshot["response"]["body"]["content"] == imported_source
        }));
    assert_eq!(run["panels"]["debug"]["loaded_source_count"], 2);
    assert_eq!(run["panels"]["debug"]["source_snapshot_count"], 2);
    assert!(run["panels"]["debug"]["source_snapshots"]
        .as_array()
        .expect("panel source snapshots")
        .iter()
        .any(|snapshot| snapshot["source"]["name"] == "user.orv"));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn editor_run_debug_build_dir_rehydrates_source_bundle_when_original_source_is_missing() {
    let dir = temp_output_dir("editor-run-debug-build-dir-source-bundle");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let path = dir.join("page.orv");
    std::fs::write(&path, r#"@out @html { @body { @h1 "Home" } }"#).expect("write source");
    let build_out = dir.join("dist");

    cmd_build_with_profile(&path, &build_out, BuildProfile::Production).expect("prod build");
    let source_bundle_path = build_out.join(SOURCE_BUNDLE_PATH);
    let source_bundle_value = read_json_value(&source_bundle_path).expect("source bundle");
    let expected_source_bundle_hash =
        stable_json_hash(&source_bundle_value).expect("source bundle hash");
    std::fs::remove_file(&path).expect("remove original source");

    let run = editor_debug_runner_session_json(
        &build_out,
        &[EditorDebugControl::Next],
        &[],
        &[],
        &[],
        &[],
        &[],
    )
    .expect("run debug runner from build dir source bundle");
    assert_eq!(run["runner"]["kind"], "orv.editor.debug.runner");
    assert_eq!(
        run["runner"]["source_bundle"],
        source_bundle_path.display().to_string()
    );
    assert_eq!(
        run["debug"]["launch"]["body"]["sourceBundle"]["path"],
        source_bundle_path.display().to_string()
    );
    assert_eq!(
        run["debug"]["launch"]["body"]["sourceBundle"]["entry"],
        source_bundle_value["entry"]
    );
    assert_eq!(
        run["debug"]["launch"]["body"]["sourceBundle"]["fileCount"],
        1
    );
    assert_eq!(
        run["debug"]["launch"]["body"]["sourceBundle"]["hash"],
        expected_source_bundle_hash
    );
    assert_eq!(
        run["panels"]["debug"]["source_bundle"],
        run["debug"]["launch"]["body"]["sourceBundle"]
    );
    assert_eq!(
        run["panels"]["debug"]["session_summary"]["source_bundle"],
        run["debug"]["launch"]["body"]["sourceBundle"]
    );
    assert_eq!(
        run["panels"]["debug"]["production_summary"]["static_target_count"],
        1
    );
    assert_eq!(
        run["panels"]["debug"]["production_summary"]["static_verified_count"],
        1
    );
    assert!(run["debug"]["source_snapshots"]
        .as_array()
        .expect("source snapshots")
        .iter()
        .any(|snapshot| snapshot["response"]["body"]["content"]
            .as_str()
            .is_some_and(|content| content.contains("@html"))));

    cmd_editor_run_debug(
        &build_out,
        &[EditorDebugControl::Next],
        &[],
        &[],
        &[],
        &[],
        &[],
    )
    .expect("write build-dir debug result");
    let result =
        read_json_value(&build_out.join(EDITOR_DEBUG_SESSION_RESULT_PATH)).expect("result");
    assert_eq!(
        result["panels"]["debug"]["production_summary"]["static_target_count"],
        1
    );
    assert_eq!(
        result["panels"]["debug"]["source_bundle"]["hash"],
        expected_source_bundle_hash
    );
    assert!(build_out
        .join(EDITOR_DEBUG_SESSION_RESULT_HTML_PATH)
        .is_file());
    let result_html =
        std::fs::read_to_string(build_out.join(EDITOR_DEBUG_SESSION_RESULT_HTML_PATH))
            .expect("debug result html");
    assert!(result_html.contains("Source Bundle"));
    assert!(result_html.contains("source_bundle"));
    assert!(result_html.contains("source-bundle.json"));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn editor_debug_breakpoint_argument_stops_continue_at_line() {
    let dir = temp_output_dir("editor-debug-breakpoint");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let path = dir.join("app.orv");
    std::fs::write(
        &path,
        "let first: int = 1\nlet second: int = 2\nlet third: int = 3\n",
    )
    .expect("write source");
    let breakpoint = EditorDebugBreakpoint {
        path: path.clone(),
        line: 3,
    };

    let debug = editor_debug_session_json(
        &path,
        &[EditorDebugControl::Continue],
        &[breakpoint],
        &[],
        &[],
        &[],
        &[],
    )
    .expect("editor debug session");

    assert_eq!(debug["transport"]["request_count"], 10);
    assert_eq!(
        debug["breakpoints"][0]["source"]["path"],
        path.display().to_string()
    );
    assert_eq!(debug["breakpoints"][0]["lines"], serde_json::json!([3]));
    assert_eq!(debug["breakpoints"][0]["response"]["success"], true);
    assert!(debug["breakpoints"][0]["response"]["body"]["breakpoints"]
        .as_array()
        .expect("breakpoints")
        .iter()
        .any(|breakpoint| breakpoint["verified"] == true && breakpoint["line"] == 3));
    assert_eq!(debug["control"]["request"]["command"], "continue");
    assert_eq!(debug["control"]["response"]["success"], true);
    assert_eq!(debug["stack"]["stackFrames"][0]["line"], 3);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn editor_debug_function_breakpoint_argument_stops_inside_function() {
    let dir = temp_output_dir("editor-debug-function-breakpoint");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let path = dir.join("app.orv");
    std::fs::write(
            &path,
            "function add(a: int, b: int): int -> {\n  let result: int = a + b\n  result\n}\nlet total: int = add(2, 3)\n",
        )
        .expect("write source");
    let function_breakpoints = vec!["add".to_string()];

    let debug = editor_debug_session_json(
        &path,
        &[EditorDebugControl::Continue],
        &[],
        &function_breakpoints,
        &[],
        &[],
        &[],
    )
    .expect("editor debug session");

    assert_eq!(
        debug["function_breakpoints"][0]["request"]["command"],
        "setFunctionBreakpoints"
    );
    assert_eq!(
        debug["function_breakpoints"][0]["names"],
        serde_json::json!(["add"])
    );
    assert_eq!(
        debug["function_breakpoints"][0]["response"]["body"]["breakpoints"][0]["verified"],
        true
    );
    assert_eq!(debug["stack"]["stackFrames"][0]["name"], "add");
    assert!(debug["frames"]
        .as_array()
        .expect("frames")
        .iter()
        .any(|frame| {
            frame["type"] == "event"
                && frame["event"] == "stopped"
                && frame["body"]["reason"] == "function breakpoint"
        }));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn editor_debug_data_breakpoint_argument_stops_when_local_changes() {
    let dir = temp_output_dir("editor-debug-data-breakpoint");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let path = dir.join("app.orv");
    std::fs::write(&path, "let mut total: int = 1\ntotal = total + 4\n").expect("write source");
    let data_breakpoints = vec!["total".to_string()];

    let debug = editor_debug_session_json(
        &path,
        &[EditorDebugControl::Continue],
        &[],
        &[],
        &data_breakpoints,
        &[],
        &[],
    )
    .expect("editor debug session");

    assert_eq!(
        debug["data_breakpoints"][0]["infos"][0]["request"]["command"],
        "dataBreakpointInfo"
    );
    assert_eq!(
        debug["data_breakpoints"][0]["request"]["command"],
        "setDataBreakpoints"
    );
    assert_eq!(
        debug["data_breakpoints"][0]["names"],
        serde_json::json!(["total"])
    );
    assert_eq!(
        debug["data_breakpoints"][0]["response"]["body"]["breakpoints"][0]["verified"],
        true
    );
    assert_eq!(debug["stack"]["stackFrames"][0]["line"], 2);
    assert!(debug["frames"]
        .as_array()
        .expect("frames")
        .iter()
        .any(|frame| {
            frame["type"] == "event"
                && frame["event"] == "stopped"
                && frame["body"]["reason"] == "data breakpoint"
        }));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn editor_run_debug_executes_exported_session_runner() {
    let dir = temp_output_dir("editor-run-debug");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let path = dir.join("app.orv");
    std::fs::write(
        &path,
        "let first: int = 1\nlet second: int = 2\nlet third: int = 3\n",
    )
    .expect("write source");
    let out = dir.join("editor");
    cmd_editor_export(&path, &out).expect("editor export");

    let run = editor_debug_runner_session_json(
        &out.join("state.json"),
        &[EditorDebugControl::Next, EditorDebugControl::Next],
        &[],
        &[],
        &[],
        &[],
        &[],
    )
    .expect("run exported debug runner");

    assert_eq!(run["kind"], "orv.editor.debug.runner.result");
    assert_eq!(run["runner"]["kind"], "orv.editor.debug.runner");
    assert_eq!(run["debug"]["transport"]["framing"], "content-length");
    assert_eq!(run["debug"]["transport"]["request_count"], 10);
    assert_eq!(run["debug"]["stack"]["stackFrames"][0]["line"], 3);
    assert!(run["debug"]["locals"]
        .as_array()
        .expect("locals")
        .iter()
        .any(|local| local["name"] == "third" && local["value"] == "3"));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn editor_run_debug_executes_exported_runner_with_breakpoint() {
    let dir = temp_output_dir("editor-run-debug-breakpoint");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let path = dir.join("app.orv");
    std::fs::write(
        &path,
        "let first: int = 1\nlet second: int = 2\nlet third: int = 3\n",
    )
    .expect("write source");
    let out = dir.join("editor");
    cmd_editor_export(&path, &out).expect("editor export");
    let breakpoint = EditorDebugBreakpoint { path, line: 3 };

    let run = editor_debug_runner_session_json(
        &out.join("debug").join("session-runner.json"),
        &[EditorDebugControl::Continue],
        &[breakpoint],
        &[],
        &[],
        &[],
        &[],
    )
    .expect("run exported debug runner");

    assert_eq!(run["kind"], "orv.editor.debug.runner.result");
    assert_eq!(run["debug"]["breakpoints"][0]["response"]["success"], true);
    assert_eq!(run["debug"]["stack"]["stackFrames"][0]["line"], 3);
    assert!(run["panels"]["debug"]["controls"]
        .as_array()
        .expect("panel controls")
        .iter()
        .any(|control| control["name"] == "Continue"));
    assert!(run["panels"]["debug"]["breakpoints"]
        .as_array()
        .expect("panel breakpoints")
        .iter()
        .any(|breakpoint| {
            breakpoint["source"]["path"]
                .as_str()
                .is_some_and(|source| source.ends_with("app.orv"))
                && breakpoint["lines"]
                    .as_array()
                    .is_some_and(|lines| lines.iter().any(|line| line == 3))
                && breakpoint["response"]["success"] == true
        }));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn editor_run_debug_executes_exported_runner_with_data_breakpoint() {
    let dir = temp_output_dir("editor-run-debug-data-breakpoint");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let path = dir.join("app.orv");
    std::fs::write(&path, "let mut total: int = 1\ntotal = total + 4\n").expect("write source");
    let out = dir.join("editor");
    cmd_editor_export(&path, &out).expect("editor export");
    let data_breakpoints = vec!["total".to_string()];

    let run = editor_debug_runner_session_json(
        &out.join("debug").join("session-runner.json"),
        &[EditorDebugControl::Continue],
        &[],
        &[],
        &data_breakpoints,
        &[],
        &[],
    )
    .expect("run exported debug runner");

    assert_eq!(run["kind"], "orv.editor.debug.runner.result");
    assert_eq!(
        run["debug"]["data_breakpoints"][0]["response"]["success"],
        true
    );
    assert_eq!(
        run["debug"]["data_breakpoints"][0]["response"]["body"]["breakpoints"][0]["verified"],
        true
    );
    assert_eq!(run["debug"]["stack"]["stackFrames"][0]["line"], 2);
    assert_eq!(run["panels"]["debug"]["data_breakpoint_count"], 1);
    assert!(run["panels"]["debug"]["data_breakpoints"]
        .as_array()
        .expect("panel data breakpoints")
        .iter()
        .any(|breakpoint| {
            breakpoint["names"]
                .as_array()
                .is_some_and(|names| names.iter().any(|name| name == "total"))
                && breakpoint["response"]["success"] == true
        }));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn editor_run_debug_writes_exported_runner_result_artifact() {
    let dir = temp_output_dir("editor-run-debug-result-artifact");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let path = dir.join("app.orv");
    std::fs::write(
        &path,
        "let first: int = 1\nlet second: int = 2\nlet third: int = 3\n",
    )
    .expect("write source");
    let out = dir.join("editor");
    cmd_editor_export(&path, &out).expect("editor export");
    let result_path = out.join(EDITOR_DEBUG_SESSION_RESULT_PATH);
    let result_html_path = out.join(EDITOR_DEBUG_SESSION_RESULT_HTML_PATH);
    assert!(!result_path.exists());
    assert!(!result_html_path.exists());

    cmd_editor_run_debug(
        &out.join(EDITOR_DEBUG_SESSION_RUNNER_PATH),
        &[EditorDebugControl::Next, EditorDebugControl::Next],
        &[],
        &[],
        &[],
        &[],
        &[],
    )
    .expect("run exported debug runner");

    let result = read_json_value(&result_path).expect("debug runner result artifact");
    assert_eq!(result["kind"], "orv.editor.debug.runner.result");
    assert_eq!(result["debug"]["stack"]["stackFrames"][0]["line"], 3);
    assert_eq!(
        result["runner"]["result"]["path"],
        EDITOR_DEBUG_SESSION_RESULT_PATH
    );
    let result_html =
        std::fs::read_to_string(result_html_path).expect("debug result html artifact");
    assert!(result_html.contains("id=\"orv-debug-result\""));
    assert!(result_html.contains("Selected Frame"));
    assert!(result_html.contains("Session Summary"));
    assert!(result_html.contains("Source Bundle"));
    assert!(result_html.contains("Source Navigation"));
    assert!(result_html.contains("Stack Frames"));
    assert!(result_html.contains("Scopes"));
    assert!(result_html.contains("Locals"));
    assert!(result_html.contains("Project Variables"));
    assert!(result_html.contains("Executed Controls"));
    assert!(result_html.contains("Requested Breakpoints"));
    assert!(result_html.contains("Function Breakpoints"));
    assert!(result_html.contains("Data Breakpoints"));
    assert!(result_html.contains("Exception Filters"));
    assert!(result_html.contains("Watch Expressions"));
    assert!(result_html.contains("Stopped Events"));
    assert!(result_html.contains("All Events"));
    assert!(result_html.contains("initialized"));
    assert!(result_html.contains("line 3"));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn editor_run_debug_writes_native_debug_result_panel_contract() {
    let dir = temp_output_dir("editor-run-debug-result-panel");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let path = dir.join("app.orv");
    std::fs::write(
        &path,
        "let first: int = 1\nlet second: int = first + 1\nlet third: int = second + 1\n",
    )
    .expect("write source");
    let out = dir.join("editor");
    cmd_editor_export(&path, &out).expect("editor export");
    let watch_expressions = vec!["third".to_string(), "runtimeStatus".to_string()];

    cmd_editor_run_debug(
        &out.join(EDITOR_DEBUG_SESSION_RUNNER_PATH),
        &[EditorDebugControl::Next, EditorDebugControl::Next],
        &[],
        &[],
        &[],
        &[],
        &watch_expressions,
    )
    .expect("run exported debug runner");

    let result =
        read_json_value(&out.join(EDITOR_DEBUG_SESSION_RESULT_PATH)).expect("debug result");
    assert_eq!(result["panels"]["debug"]["schema_version"], 1);
    assert_eq!(result["panels"]["debug"]["control_count"], 2);
    assert_eq!(result["panels"]["debug"]["breakpoint_count"], 0);
    assert_eq!(result["panels"]["debug"]["function_breakpoint_count"], 0);
    assert_eq!(result["panels"]["debug"]["data_breakpoint_count"], 0);
    assert_eq!(result["panels"]["debug"]["exception_filter_count"], 0);
    assert_eq!(result["panels"]["debug"]["watch_expression_count"], 2);
    let panel_controls = result["panels"]["debug"]["controls"]
        .as_array()
        .expect("panel controls");
    assert_eq!(panel_controls.len(), 2);
    assert_eq!(panel_controls[0]["name"], "Next");
    assert_eq!(panel_controls[1]["name"], "Next");
    assert!(result["runner"]["result"]["panel_contract"]["sections"]
        .as_array()
        .expect("panel sections")
        .iter()
        .any(|section| {
            section["name"] == "production_summary"
                && section["path"] == "panels.debug.production_summary"
        }));
    assert!(result["runner"]["result"]["panel_contract"]["sections"]
        .as_array()
        .expect("panel sections")
        .iter()
        .any(|section| {
            section["name"] == "source_navigation"
                && section["path"] == "panels.debug.source_navigation"
        }));
    assert!(result["runner"]["result"]["panel_contract"]["sections"]
        .as_array()
        .expect("panel sections")
        .iter()
        .any(|section| {
            section["name"] == "scopes" && section["path"] == "panels.debug.scopes"
        }));
    assert!(result["runner"]["result"]["panel_contract"]["sections"]
        .as_array()
        .expect("panel sections")
        .iter()
        .any(|section| {
            section["name"] == "controls" && section["path"] == "panels.debug.controls"
        }));
    assert!(result["runner"]["result"]["panel_contract"]["sections"]
        .as_array()
        .expect("panel sections")
        .iter()
        .any(|section| {
            section["name"] == "breakpoints" && section["path"] == "panels.debug.breakpoints"
        }));
    assert!(result["runner"]["result"]["panel_contract"]["sections"]
        .as_array()
        .expect("panel sections")
        .iter()
        .any(|section| {
            section["name"] == "function_breakpoints"
                && section["path"] == "panels.debug.function_breakpoints"
        }));
    assert!(result["runner"]["result"]["panel_contract"]["sections"]
        .as_array()
        .expect("panel sections")
        .iter()
        .any(|section| {
            section["name"] == "data_breakpoints"
                && section["path"] == "panels.debug.data_breakpoints"
        }));
    assert!(result["runner"]["result"]["panel_contract"]["sections"]
        .as_array()
        .expect("panel sections")
        .iter()
        .any(|section| {
            section["name"] == "exception_filters"
                && section["path"] == "panels.debug.exception_filters"
        }));
    assert!(result["runner"]["result"]["panel_contract"]["sections"]
        .as_array()
        .expect("panel sections")
        .iter()
        .any(|section| {
            section["name"] == "watch_expressions"
                && section["path"] == "panels.debug.watch_expressions"
        }));
    assert_eq!(
        result["panels"]["debug"]["session_summary"]["schema_version"],
        1
    );
    assert_eq!(
        result["panels"]["debug"]["session_summary"]["program"],
        path.display().to_string()
    );
    assert_eq!(
        result["panels"]["debug"]["session_summary"]["selected_line"],
        3
    );
    assert_eq!(
        result["panels"]["debug"]["session_summary"]["control_count"],
        2
    );
    assert_eq!(
        result["panels"]["debug"]["session_summary"]["function_breakpoint_count"],
        0
    );
    assert_eq!(
        result["panels"]["debug"]["session_summary"]["data_breakpoint_count"],
        0
    );
    assert_eq!(
        result["panels"]["debug"]["session_summary"]["exception_filter_count"],
        0
    );
    assert_eq!(
        result["panels"]["debug"]["session_summary"]["watch_expression_count"],
        2
    );
    assert!(result["panels"]["debug"]["session_summary"]["last_event"]
        .as_str()
        .is_some_and(|event| !event.is_empty()));
    assert_eq!(result["panels"]["debug"]["selected_frame"]["line"], 3);
    assert!(result["panels"]["debug"]["stack_frames"]
        .as_array()
        .expect("stack frames")
        .iter()
        .any(|frame| frame["line"] == 3));
    assert_eq!(
        result["panels"]["debug"]["source_navigation"]["selected"]["line"],
        3
    );
    assert!(
        result["panels"]["debug"]["source_navigation"]["selected"]["source"]["path"]
            .as_str()
            .is_some_and(|source| source.ends_with("app.orv"))
    );
    assert!(result["panels"]["debug"]["source_navigation"]["frames"]
        .as_array()
        .expect("source navigation frames")
        .iter()
        .any(|frame| frame["line"] == 3));
    assert!(result["panels"]["debug"]["scopes"]["scopes"]
        .as_array()
        .expect("scopes")
        .iter()
        .any(|scope| scope["name"] == "Locals" || scope["name"] == "Project"));
    assert!(result["panels"]["debug"]["locals"]
        .as_array()
        .expect("locals")
        .iter()
        .any(|local| local["name"] == "third" && local["value"] == "3"));
    let watch_panel = result["panels"]["debug"]["watch_expressions"]
        .as_array()
        .expect("watch expressions");
    assert_eq!(watch_panel.len(), 2);
    assert!(watch_panel.iter().any(|expression| {
        expression["expression"] == "third"
            && expression["response"]["success"] == true
            && expression["response"]["body"]["result"] == "3"
            && expression["response"]["body"]["type"] == "int"
    }));
    assert!(watch_panel.iter().any(|expression| {
        expression["expression"] == "runtimeStatus"
            && expression["response"]["success"] == true
            && expression["response"]["body"]["type"] == "string"
    }));
    assert!(result["panels"]["debug"]["project_variables"]
        .as_array()
        .expect("project variables")
        .iter()
        .any(|variable| variable["name"] == "stdout"));
    assert!(
        result["panels"]["debug"]["stopped_events"]
            .as_array()
            .expect("stopped events")
            .len()
            >= 2
    );
    assert!(
        result["panels"]["debug"]["event_count"]
            .as_u64()
            .is_some_and(|count| count >= 2),
        "{result}"
    );
    assert!(
        result["panels"]["debug"]["events"]
            .as_array()
            .expect("events")
            .iter()
            .any(|event| event["event"] == "stopped"),
        "{result}"
    );
    assert!(result["panels"]["debug"]["result_artifact"]["path"]
        .as_str()
        .is_some_and(|path| path.ends_with(EDITOR_DEBUG_SESSION_RESULT_PATH)));
    let _ = std::fs::remove_dir_all(dir);
}
