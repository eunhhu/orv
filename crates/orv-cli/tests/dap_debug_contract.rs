use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

fn temp_output_dir(name: &str) -> PathBuf {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    std::env::temp_dir().join(format!("orv-{name}-{}-{nonce}", std::process::id()))
}

const fn orv_bin() -> &'static str {
    env!("CARGO_BIN_EXE_orv")
}

fn run_orv(args: &[&str]) {
    let output = Command::new(orv_bin())
        .args(args)
        .output()
        .expect("run orv");
    assert!(
        output.status.success(),
        "orv {args:?} failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn run_orv_json(args: &[&str]) -> serde_json::Value {
    let output = Command::new(orv_bin())
        .args(args)
        .output()
        .expect("run orv");
    assert!(
        output.status.success(),
        "orv {args:?} failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("json stdout")
}

fn read_json(path: &Path) -> serde_json::Value {
    serde_json::from_str(&std::fs::read_to_string(path).expect("read json")).expect("json")
}

fn assert_keys(value: &serde_json::Value, expected: &[&str], context: &str) {
    let object = value
        .as_object()
        .unwrap_or_else(|| panic!("{context} must be an object"));
    let actual = object.keys().map(String::as_str).collect::<BTreeSet<_>>();
    let expected = expected.iter().copied().collect::<BTreeSet<_>>();
    assert_eq!(actual, expected, "{context} keys drifted");
}

fn build_debug_fixture(root: &Path) -> PathBuf {
    let source = root.join("app.orv");
    std::fs::write(&source, "let total: int = 41\n@out total\n").expect("write source");
    source
}

#[test]
fn dap_debug_session_v1_freezes_stdio_initialize_contract() {
    let frames = run_dap_stdio_frames(&[serde_json::json!({
        "seq": 1,
        "type": "request",
        "command": "initialize",
        "arguments": {},
    })]);

    assert_eq!(frames.len(), 2, "{frames:?}");
    let response = &frames[0];
    assert_keys(
        response,
        &["seq", "type", "request_seq", "success", "command", "body"],
        "dap initialize response",
    );
    assert_eq!(response["seq"], serde_json::json!(1));
    assert_eq!(response["type"], serde_json::json!("response"));
    assert_eq!(response["request_seq"], serde_json::json!(1));
    assert_eq!(response["success"], serde_json::json!(true));
    assert_eq!(response["command"], serde_json::json!("initialize"));
    assert_initialize_capabilities(&response["body"]);

    let initialized = &frames[1];
    assert_keys(
        initialized,
        &["seq", "type", "event", "body"],
        "dap initialized event",
    );
    assert_eq!(initialized["type"], serde_json::json!("event"));
    assert_eq!(initialized["event"], serde_json::json!("initialized"));
    assert!(initialized["body"]
        .as_object()
        .is_some_and(serde_json::Map::is_empty));
}

#[test]
fn dap_debug_runner_result_contract_freezes_public_shape() {
    let root = temp_output_dir("dap-debug-contract");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("temp root");
    let source = build_debug_fixture(&root);
    let build_out = root.join("dist");
    let source_arg = source.display().to_string();
    let build_arg = build_out.display().to_string();

    run_orv(&["build", &source_arg, "--out", &build_arg, "--prod"]);
    let run = run_orv_json(&[
        "editor",
        "run-debug",
        &build_arg,
        "--control",
        "next",
        "--watch-expression",
        "total",
    ]);

    assert_result_root(&run);
    assert_runner_contract(&run["runner"]);
    assert_production_context_contract(&run["production_context"]);
    assert_debug_session_contract(&run["debug"]);
    assert_debug_panel_contract(&run["panels"]["debug"]);
    assert_written_result_artifacts(&build_out, &run);

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn dap_debug_runner_rejects_stale_runner_schema_version() {
    let root = temp_output_dir("dap-debug-runner-schema-version");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("temp root");
    let runner = root.join("session-runner.json");
    std::fs::write(
        &runner,
        serde_json::to_string_pretty(&serde_json::json!({
            "kind": "orv.editor.debug.runner",
            "program": root.join("app.orv").display().to_string(),
            "result": {
                "path": "debug/session-result.json"
            }
        }))
        .expect("runner json"),
    )
    .expect("write stale runner");

    let output = Command::new(orv_bin())
        .args(["editor", "run-debug", &runner.display().to_string()])
        .output()
        .expect("run orv editor run-debug");

    assert!(!output.status.success(), "stale runner schema must fail");
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("editor debug runner schema_version must be 1"),
        "{stderr}"
    );

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn dap_debug_runner_rejects_stale_export_state_schema_version() {
    let root = temp_output_dir("dap-debug-export-state-schema-version");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("temp root");
    let state = root.join("state.json");
    std::fs::write(
        &state,
        serde_json::to_string_pretty(&serde_json::json!({
            "kind": "orv.editor.export",
            "debug": {
                "session_runner": {
                    "schema_version": 1,
                    "kind": "orv.editor.debug.runner",
                    "program": root.join("app.orv").display().to_string(),
                    "result": {
                        "path": "debug/session-result.json"
                    }
                }
            }
        }))
        .expect("state json"),
    )
    .expect("write stale state");

    let output = Command::new(orv_bin())
        .args(["editor", "run-debug", &state.display().to_string()])
        .output()
        .expect("run orv editor run-debug");

    assert!(!output.status.success(), "stale state schema must fail");
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("editor export state schema_version must be 1"),
        "{stderr}"
    );

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn dap_debug_runner_rejects_extra_runner_root_key() {
    let root = temp_output_dir("dap-debug-runner-extra-root");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("temp root");
    let source = build_debug_fixture(&root);
    let out = root.join("editor");
    let source_arg = source.display().to_string();
    let out_arg = out.display().to_string();
    run_orv(&["editor", "export", &source_arg, "--out", &out_arg]);
    let runner = out.join("debug").join("session-runner.json");
    let mut value = read_json(&runner);
    value["unexpected"] = serde_json::json!(true);
    std::fs::write(
        &runner,
        serde_json::to_string_pretty(&value).expect("runner json"),
    )
    .expect("write corrupt runner");

    let output = Command::new(orv_bin())
        .args(["editor", "run-debug", &runner.display().to_string()])
        .output()
        .expect("run orv editor run-debug");

    assert!(!output.status.success(), "extra runner key must fail");
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("editor debug runner keys must match contract"),
        "{stderr}"
    );

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn dap_debug_runner_rejects_extra_result_artifact_key() {
    let root = temp_output_dir("dap-debug-runner-extra-result");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("temp root");
    let source = build_debug_fixture(&root);
    let out = root.join("editor");
    let source_arg = source.display().to_string();
    let out_arg = out.display().to_string();
    run_orv(&["editor", "export", &source_arg, "--out", &out_arg]);
    let runner = out.join("debug").join("session-runner.json");
    let mut value = read_json(&runner);
    value["result"]["unexpected"] = serde_json::json!("drift");
    std::fs::write(
        &runner,
        serde_json::to_string_pretty(&value).expect("runner json"),
    )
    .expect("write corrupt runner");

    let output = Command::new(orv_bin())
        .args(["editor", "run-debug", &runner.display().to_string()])
        .output()
        .expect("run orv editor run-debug");

    assert!(
        !output.status.success(),
        "extra result artifact key must fail"
    );
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("editor debug runner result artifact keys must match contract"),
        "{stderr}"
    );

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn dap_debug_runner_rejects_extra_result_panel_contract_key() {
    let root = temp_output_dir("dap-debug-runner-extra-result-panel-contract");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("temp root");
    let source = build_debug_fixture(&root);
    let out = root.join("editor");
    let source_arg = source.display().to_string();
    let out_arg = out.display().to_string();
    run_orv(&["editor", "export", &source_arg, "--out", &out_arg]);
    let runner = out.join("debug").join("session-runner.json");
    let mut value = read_json(&runner);
    value["result"]["panel_contract"]["unexpected"] = serde_json::json!("drift");
    std::fs::write(
        &runner,
        serde_json::to_string_pretty(&value).expect("runner json"),
    )
    .expect("write corrupt runner");

    let output = Command::new(orv_bin())
        .args(["editor", "run-debug", &runner.display().to_string()])
        .output()
        .expect("run orv editor run-debug");

    assert!(
        !output.status.success(),
        "extra result panel_contract key must fail"
    );
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("editor debug runner panel_contract keys must match contract"),
        "{stderr}"
    );

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn dap_debug_runner_rejects_result_artifact_value_drift() {
    let root = temp_output_dir("dap-debug-runner-result-value-drift");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("temp root");
    let source = build_debug_fixture(&root);
    let out = root.join("editor");
    let source_arg = source.display().to_string();
    let out_arg = out.display().to_string();
    run_orv(&["editor", "export", &source_arg, "--out", &out_arg]);
    let runner = out.join("debug").join("session-runner.json");
    let mut value = read_json(&runner);
    value["result"]["path"] = serde_json::json!("debug/drifted-result.json");
    std::fs::write(
        &runner,
        serde_json::to_string_pretty(&value).expect("runner json"),
    )
    .expect("write corrupt runner");

    let output = Command::new(orv_bin())
        .args(["editor", "run-debug", &runner.display().to_string()])
        .output()
        .expect("run orv editor run-debug");

    assert!(
        !output.status.success(),
        "result artifact value drift must fail"
    );
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("editor debug runner result artifact must match generated contract"),
        "{stderr}"
    );

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn dap_debug_runner_rejects_command_value_drift() {
    let root = temp_output_dir("dap-debug-runner-command-value-drift");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("temp root");
    let source = build_debug_fixture(&root);
    let out = root.join("editor");
    let source_arg = source.display().to_string();
    let out_arg = out.display().to_string();
    run_orv(&["editor", "export", &source_arg, "--out", &out_arg]);
    let runner = out.join("debug").join("session-runner.json");
    let mut value = read_json(&runner);
    value["command"][5] = serde_json::json!("continue");
    std::fs::write(
        &runner,
        serde_json::to_string_pretty(&value).expect("runner json"),
    )
    .expect("write corrupt runner");

    let output = Command::new(orv_bin())
        .args(["editor", "run-debug", &runner.display().to_string()])
        .output()
        .expect("run orv editor run-debug");

    assert!(!output.status.success(), "command value drift must fail");
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("editor debug runner command must match generated contract"),
        "{stderr}"
    );

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn dap_debug_runner_rejects_extra_export_state_root_key() {
    let root = temp_output_dir("dap-debug-export-state-extra-root");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("temp root");
    let source = build_debug_fixture(&root);
    let out = root.join("editor");
    let source_arg = source.display().to_string();
    let out_arg = out.display().to_string();
    run_orv(&["editor", "export", &source_arg, "--out", &out_arg]);
    let state = out.join("state.json");
    let mut value = read_json(&state);
    value["unexpected"] = serde_json::json!("drift");
    std::fs::write(
        &state,
        serde_json::to_string_pretty(&value).expect("state json"),
    )
    .expect("write corrupt state");

    let output = Command::new(orv_bin())
        .args(["editor", "run-debug", &state.display().to_string()])
        .output()
        .expect("run orv editor run-debug");

    assert!(!output.status.success(), "extra export state key must fail");
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("editor export state keys must match contract"),
        "{stderr}"
    );

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn dap_debug_runner_rejects_extra_export_debug_key() {
    let root = temp_output_dir("dap-debug-export-debug-extra-key");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("temp root");
    let source = build_debug_fixture(&root);
    let out = root.join("editor");
    let source_arg = source.display().to_string();
    let out_arg = out.display().to_string();
    run_orv(&["editor", "export", &source_arg, "--out", &out_arg]);
    let state = out.join("state.json");
    let mut value = read_json(&state);
    value["debug"]["unexpected"] = serde_json::json!("drift");
    std::fs::write(
        &state,
        serde_json::to_string_pretty(&value).expect("state json"),
    )
    .expect("write corrupt state");

    let output = Command::new(orv_bin())
        .args(["editor", "run-debug", &state.display().to_string()])
        .output()
        .expect("run orv editor run-debug");

    assert!(!output.status.success(), "extra debug key must fail");
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("editor export debug keys must match contract"),
        "{stderr}"
    );

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn dap_debug_runner_rejects_export_debug_adapter_value_drift() {
    let root = temp_output_dir("dap-debug-export-debug-adapter-drift");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("temp root");
    let source = build_debug_fixture(&root);
    let out = root.join("editor");
    let source_arg = source.display().to_string();
    let out_arg = out.display().to_string();
    run_orv(&["editor", "export", &source_arg, "--out", &out_arg]);
    let state = out.join("state.json");
    let mut value = read_json(&state);
    value["debug"]["adapter"]["command"] = serde_json::json!(["orv", "dap", "serve"]);
    std::fs::write(
        &state,
        serde_json::to_string_pretty(&value).expect("state json"),
    )
    .expect("write corrupt state");

    let output = Command::new(orv_bin())
        .args(["editor", "run-debug", &state.display().to_string()])
        .output()
        .expect("run orv editor run-debug");

    assert!(
        !output.status.success(),
        "debug adapter value drift must fail"
    );
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("editor export debug adapter must match generated contract"),
        "{stderr}"
    );

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn dap_debug_runner_rejects_export_debug_capabilities_value_drift() {
    let root = temp_output_dir("dap-debug-export-debug-capabilities-drift");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("temp root");
    let source = build_debug_fixture(&root);
    let out = root.join("editor");
    let source_arg = source.display().to_string();
    let out_arg = out.display().to_string();
    run_orv(&["editor", "export", &source_arg, "--out", &out_arg]);
    let state = out.join("state.json");
    let mut value = read_json(&state);
    value["debug"]["capabilities"]["supportsStepBack"] = serde_json::json!(false);
    std::fs::write(
        &state,
        serde_json::to_string_pretty(&value).expect("state json"),
    )
    .expect("write corrupt state");

    let output = Command::new(orv_bin())
        .args(["editor", "run-debug", &state.display().to_string()])
        .output()
        .expect("run orv editor run-debug");

    assert!(
        !output.status.success(),
        "debug capabilities value drift must fail"
    );
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("editor export debug capabilities must match generated contract"),
        "{stderr}"
    );

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn dap_debug_runner_rejects_export_debug_controls_value_drift() {
    let root = temp_output_dir("dap-debug-export-debug-controls-drift");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("temp root");
    let source = build_debug_fixture(&root);
    let out = root.join("editor");
    let source_arg = source.display().to_string();
    let out_arg = out.display().to_string();
    run_orv(&["editor", "export", &source_arg, "--out", &out_arg]);
    let state = out.join("state.json");
    let mut value = read_json(&state);
    value["debug"]["controls"][0]["runner_command"][5] = serde_json::json!("continue-drift");
    std::fs::write(
        &state,
        serde_json::to_string_pretty(&value).expect("state json"),
    )
    .expect("write corrupt state");

    let output = Command::new(orv_bin())
        .args(["editor", "run-debug", &state.display().to_string()])
        .output()
        .expect("run orv editor run-debug");

    assert!(
        !output.status.success(),
        "debug controls value drift must fail"
    );
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("editor export debug controls must match generated contract"),
        "{stderr}"
    );

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn dap_debug_runner_rejects_export_debug_source_inventory_extra_key() {
    let root = temp_output_dir("dap-debug-export-debug-source-inventory-extra");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("temp root");
    let source = build_debug_fixture(&root);
    let out = root.join("editor");
    let source_arg = source.display().to_string();
    let out_arg = out.display().to_string();
    run_orv(&["editor", "export", &source_arg, "--out", &out_arg]);
    let state = out.join("state.json");
    let mut value = read_json(&state);
    value["debug"]["source_inventory"]["sources"][0]["unexpected"] = serde_json::json!("drift");
    std::fs::write(
        &state,
        serde_json::to_string_pretty(&value).expect("state json"),
    )
    .expect("write corrupt state");

    let output = Command::new(orv_bin())
        .args(["editor", "run-debug", &state.display().to_string()])
        .output()
        .expect("run orv editor run-debug");

    assert!(
        !output.status.success(),
        "source inventory key drift must fail"
    );
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("editor export debug source_inventory.sources[0] keys must match contract"),
        "{stderr}"
    );

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn dap_debug_runner_rejects_export_debug_breakpoint_source_extra_key() {
    let root = temp_output_dir("dap-debug-export-debug-breakpoint-source-extra");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("temp root");
    let source = build_debug_fixture(&root);
    let out = root.join("editor");
    let source_arg = source.display().to_string();
    let out_arg = out.display().to_string();
    run_orv(&["editor", "export", &source_arg, "--out", &out_arg]);
    let state = out.join("state.json");
    let mut value = read_json(&state);
    value["debug"]["breakpoint_sources"][0]["unexpected"] = serde_json::json!("drift");
    std::fs::write(
        &state,
        serde_json::to_string_pretty(&value).expect("state json"),
    )
    .expect("write corrupt state");

    let output = Command::new(orv_bin())
        .args(["editor", "run-debug", &state.display().to_string()])
        .output()
        .expect("run orv editor run-debug");

    assert!(
        !output.status.success(),
        "breakpoint source key drift must fail"
    );
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("editor export debug breakpoint_sources[0] keys must match contract"),
        "{stderr}"
    );

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn dap_debug_runner_rejects_export_debug_exception_filter_extra_key() {
    let root = temp_output_dir("dap-debug-export-debug-exception-filter-extra");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("temp root");
    let source = build_debug_fixture(&root);
    let out = root.join("editor");
    let source_arg = source.display().to_string();
    let out_arg = out.display().to_string();
    run_orv(&["editor", "export", &source_arg, "--out", &out_arg]);
    let state = out.join("state.json");
    let mut value = read_json(&state);
    value["debug"]["exception_filters"][0]["unexpected"] = serde_json::json!("drift");
    std::fs::write(
        &state,
        serde_json::to_string_pretty(&value).expect("state json"),
    )
    .expect("write corrupt state");

    let output = Command::new(orv_bin())
        .args(["editor", "run-debug", &state.display().to_string()])
        .output()
        .expect("run orv editor run-debug");

    assert!(
        !output.status.success(),
        "exception filter key drift must fail"
    );
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("editor export debug exception_filters[0] keys must match contract"),
        "{stderr}"
    );

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn dap_debug_runner_rejects_export_debug_result_artifact_value_drift() {
    let root = temp_output_dir("dap-debug-export-debug-result-drift");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("temp root");
    let source = build_debug_fixture(&root);
    let out = root.join("editor");
    let source_arg = source.display().to_string();
    let out_arg = out.display().to_string();
    run_orv(&["editor", "export", &source_arg, "--out", &out_arg]);
    let state = out.join("state.json");
    let mut value = read_json(&state);
    value["debug"]["result_artifact"]["path"] = serde_json::json!("debug/drifted-result.json");
    std::fs::write(
        &state,
        serde_json::to_string_pretty(&value).expect("state json"),
    )
    .expect("write corrupt state");

    let output = Command::new(orv_bin())
        .args(["editor", "run-debug", &state.display().to_string()])
        .output()
        .expect("run orv editor run-debug");

    assert!(
        !output.status.success(),
        "debug result artifact value drift must fail"
    );
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("editor debug runner result artifact must match generated contract"),
        "{stderr}"
    );

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn dap_debug_runner_rejects_extra_transport_key() {
    let root = temp_output_dir("dap-debug-runner-extra-transport");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("temp root");
    let source = build_debug_fixture(&root);
    let out = root.join("editor");
    let source_arg = source.display().to_string();
    let out_arg = out.display().to_string();
    run_orv(&["editor", "export", &source_arg, "--out", &out_arg]);
    let runner = out.join("debug").join("session-runner.json");
    let mut value = read_json(&runner);
    value["transport"]["unexpected"] = serde_json::json!("drift");
    std::fs::write(
        &runner,
        serde_json::to_string_pretty(&value).expect("runner json"),
    )
    .expect("write corrupt runner");

    let output = Command::new(orv_bin())
        .args(["editor", "run-debug", &runner.display().to_string()])
        .output()
        .expect("run orv editor run-debug");

    assert!(!output.status.success(), "extra transport key must fail");
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("editor debug runner transport keys must match contract"),
        "{stderr}"
    );

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn dap_debug_runner_rejects_transport_value_drift() {
    let root = temp_output_dir("dap-debug-runner-transport-value-drift");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("temp root");
    let source = build_debug_fixture(&root);
    let out = root.join("editor");
    let source_arg = source.display().to_string();
    let out_arg = out.display().to_string();
    run_orv(&["editor", "export", &source_arg, "--out", &out_arg]);
    let runner = out.join("debug").join("session-runner.json");
    let mut value = read_json(&runner);
    value["transport"]["framing"] = serde_json::json!("line-delimited");
    std::fs::write(
        &runner,
        serde_json::to_string_pretty(&value).expect("runner json"),
    )
    .expect("write corrupt runner");

    let output = Command::new(orv_bin())
        .args(["editor", "run-debug", &runner.display().to_string()])
        .output()
        .expect("run orv editor run-debug");

    assert!(!output.status.success(), "transport value drift must fail");
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("editor debug runner transport must match generated contract"),
        "{stderr}"
    );

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn dap_debug_runner_rejects_extra_session_key() {
    let root = temp_output_dir("dap-debug-runner-extra-session");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("temp root");
    let source = build_debug_fixture(&root);
    let out = root.join("editor");
    let source_arg = source.display().to_string();
    let out_arg = out.display().to_string();
    run_orv(&["editor", "export", &source_arg, "--out", &out_arg]);
    let runner = out.join("debug").join("session-runner.json");
    let mut value = read_json(&runner);
    value["session"]["unexpected"] = serde_json::json!("drift");
    std::fs::write(
        &runner,
        serde_json::to_string_pretty(&value).expect("runner json"),
    )
    .expect("write corrupt runner");

    let output = Command::new(orv_bin())
        .args(["editor", "run-debug", &runner.display().to_string()])
        .output()
        .expect("run orv editor run-debug");

    assert!(!output.status.success(), "extra session key must fail");
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("editor debug runner session keys must match contract"),
        "{stderr}"
    );

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn dap_debug_runner_rejects_session_value_drift() {
    let root = temp_output_dir("dap-debug-runner-session-value-drift");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("temp root");
    let source = build_debug_fixture(&root);
    let out = root.join("editor");
    let source_arg = source.display().to_string();
    let out_arg = out.display().to_string();
    run_orv(&["editor", "export", &source_arg, "--out", &out_arg]);
    let runner = out.join("debug").join("session-runner.json");
    let mut value = read_json(&runner);
    value["session"]["thread_id"] = serde_json::json!(2);
    std::fs::write(
        &runner,
        serde_json::to_string_pretty(&value).expect("runner json"),
    )
    .expect("write corrupt runner");

    let output = Command::new(orv_bin())
        .args(["editor", "run-debug", &runner.display().to_string()])
        .output()
        .expect("run orv editor run-debug");

    assert!(!output.status.success(), "session value drift must fail");
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("editor debug runner session must match generated contract"),
        "{stderr}"
    );

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn dap_debug_runner_rejects_extra_control_key() {
    let root = temp_output_dir("dap-debug-runner-extra-control");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("temp root");
    let source = build_debug_fixture(&root);
    let out = root.join("editor");
    let source_arg = source.display().to_string();
    let out_arg = out.display().to_string();
    run_orv(&["editor", "export", &source_arg, "--out", &out_arg]);
    let runner = out.join("debug").join("session-runner.json");
    let mut value = read_json(&runner);
    value["controls"][0]["unexpected"] = serde_json::json!("drift");
    std::fs::write(
        &runner,
        serde_json::to_string_pretty(&value).expect("runner json"),
    )
    .expect("write corrupt runner");

    let output = Command::new(orv_bin())
        .args(["editor", "run-debug", &runner.display().to_string()])
        .output()
        .expect("run orv editor run-debug");

    assert!(!output.status.success(), "extra control key must fail");
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("editor debug runner controls[0] keys must match contract"),
        "{stderr}"
    );

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn dap_debug_runner_rejects_controls_value_drift() {
    let root = temp_output_dir("dap-debug-runner-controls-value-drift");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("temp root");
    let source = build_debug_fixture(&root);
    let out = root.join("editor");
    let source_arg = source.display().to_string();
    let out_arg = out.display().to_string();
    run_orv(&["editor", "export", &source_arg, "--out", &out_arg]);
    let runner = out.join("debug").join("session-runner.json");
    let mut value = read_json(&runner);
    value["controls"][0]["value"] = serde_json::json!("continue-drift");
    std::fs::write(
        &runner,
        serde_json::to_string_pretty(&value).expect("runner json"),
    )
    .expect("write corrupt runner");

    let output = Command::new(orv_bin())
        .args(["editor", "run-debug", &runner.display().to_string()])
        .output()
        .expect("run orv editor run-debug");

    assert!(!output.status.success(), "controls value drift must fail");
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("editor debug runner controls must match generated contract"),
        "{stderr}"
    );

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn dap_debug_runner_rejects_extra_production_context_key() {
    let root = temp_output_dir("dap-debug-runner-extra-production-context");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("temp root");
    let source = build_debug_fixture(&root);
    let build_out = root.join("dist");
    let out = root.join("editor");
    let source_arg = source.display().to_string();
    let build_arg = build_out.display().to_string();
    let out_arg = out.display().to_string();
    run_orv(&["build", &source_arg, "--prod", "--out", &build_arg]);
    run_orv(&[
        "editor",
        "export",
        &source_arg,
        "--out",
        &out_arg,
        "--build",
        &build_arg,
    ]);
    let runner = out.join("debug").join("session-runner.json");
    let mut value = read_json(&runner);
    value["production_context"]["unexpected"] = serde_json::json!("drift");
    std::fs::write(
        &runner,
        serde_json::to_string_pretty(&value).expect("runner json"),
    )
    .expect("write corrupt runner");

    let output = Command::new(orv_bin())
        .args(["editor", "run-debug", &runner.display().to_string()])
        .output()
        .expect("run orv editor run-debug");

    assert!(
        !output.status.success(),
        "extra production_context key must fail"
    );
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("editor debug production_context keys must match contract"),
        "{stderr}"
    );

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn dap_debug_runner_rejects_extra_production_summary_key() {
    let root = temp_output_dir("dap-debug-runner-extra-production-summary");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("temp root");
    let source = build_debug_fixture(&root);
    let build_out = root.join("dist");
    let out = root.join("editor");
    let source_arg = source.display().to_string();
    let build_arg = build_out.display().to_string();
    let out_arg = out.display().to_string();
    run_orv(&["build", &source_arg, "--prod", "--out", &build_arg]);
    run_orv(&[
        "editor",
        "export",
        &source_arg,
        "--out",
        &out_arg,
        "--build",
        &build_arg,
    ]);
    let runner = out.join("debug").join("session-runner.json");
    let mut value = read_json(&runner);
    value["production_context"]["summary"]["unexpected"] = serde_json::json!("drift");
    std::fs::write(
        &runner,
        serde_json::to_string_pretty(&value).expect("runner json"),
    )
    .expect("write corrupt runner");

    let output = Command::new(orv_bin())
        .args(["editor", "run-debug", &runner.display().to_string()])
        .output()
        .expect("run orv editor run-debug");

    assert!(
        !output.status.success(),
        "extra production summary key must fail"
    );
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("editor debug production_context.summary keys must match contract"),
        "{stderr}"
    );

    let _ = std::fs::remove_dir_all(&root);
}

fn run_dap_stdio_frames(requests: &[serde_json::Value]) -> Vec<serde_json::Value> {
    let mut input = String::new();
    for request in requests {
        let body = serde_json::to_string(request).expect("serialize dap request");
        write!(&mut input, "Content-Length: {}\r\n\r\n{body}", body.len())
            .expect("append dap input frame");
    }

    let mut child = Command::new(orv_bin())
        .args(["dap", "serve", "--stdio"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn dap server");
    child
        .stdin
        .take()
        .expect("dap stdin")
        .write_all(input.as_bytes())
        .expect("write dap input");
    let output = child.wait_with_output().expect("wait dap server");
    assert_success(&output, "orv dap serve --stdio");
    protocol_frames(&String::from_utf8(output.stdout).expect("dap stdout utf8"))
}

fn assert_success(output: &Output, label: &str) {
    assert!(
        output.status.success(),
        "{label} failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn protocol_frames(output: &str) -> Vec<serde_json::Value> {
    let mut rest = output;
    let mut frames = Vec::new();
    while !rest.is_empty() {
        let Some((header, body_start)) = rest.split_once("\r\n\r\n") else {
            panic!("missing DAP frame body: {rest}");
        };
        let content_length = header
            .lines()
            .find_map(|line| {
                let (name, value) = line.split_once(':')?;
                name.eq_ignore_ascii_case("Content-Length")
                    .then(|| value.trim().parse::<usize>().expect("content length"))
            })
            .expect("content length header");
        let body = body_start
            .get(..content_length)
            .unwrap_or_else(|| panic!("truncated DAP frame body: {body_start}"));
        frames.push(serde_json::from_str(body).expect("dap frame json"));
        rest = &body_start[content_length..];
    }
    frames
}

fn assert_initialize_capabilities(body: &serde_json::Value) {
    let capability_keys = [
        "supportsConfigurationDoneRequest",
        "supportsTerminateRequest",
        "supportsTerminateThreadsRequest",
        "supportsLoadedSourcesRequest",
        "supportsEvaluateForHovers",
        "supportsCompletionsRequest",
        "supportsBreakpointLocationsRequest",
        "supportsConditionalBreakpoints",
        "supportsHitConditionalBreakpoints",
        "supportsFunctionBreakpoints",
        "supportsDataBreakpoints",
        "supportsExceptionInfoRequest",
        "supportsRestartRequest",
        "supportsSetVariable",
        "supportsSetExpression",
        "supportsModulesRequest",
        "supportsGotoTargetsRequest",
        "supportsStepBack",
        "supportsStepInTargetsRequest",
        "supportsRestartFrame",
        "supportsPauseRequest",
        "supportsCancelRequest",
        "supportsInstructionBreakpoints",
        "supportsDisassembleRequest",
        "supportsReadMemoryRequest",
        "supportsOrvRuntimeAttach",
        "supportsOrvRuntimeTracePath",
        "supportsOrvSourceBundleLaunch",
    ];
    let mut expected = capability_keys.to_vec();
    expected.push("exceptionBreakpointFilters");
    assert_keys(body, &expected, "dap initialize capabilities");
    for key in capability_keys {
        assert_eq!(body[key], serde_json::json!(true), "{key}");
    }

    let filters = body["exceptionBreakpointFilters"]
        .as_array()
        .expect("exception breakpoint filters");
    assert_eq!(filters.len(), 2);
    for filter in filters {
        assert_keys(
            filter,
            &["filter", "label", "default"],
            "exception breakpoint filter",
        );
        assert_eq!(filter["default"], serde_json::json!(true));
    }
    assert_eq!(filters[0]["filter"], serde_json::json!("orv.diagnostics"));
    assert_eq!(filters[0]["label"], serde_json::json!("ORV diagnostics"));
    assert_eq!(filters[1]["filter"], serde_json::json!("orv.runtime"));
    assert_eq!(filters[1]["label"], serde_json::json!("ORV runtime errors"));
}

fn assert_result_root(run: &serde_json::Value) {
    assert_keys(
        run,
        &[
            "schema_version",
            "kind",
            "state",
            "runner",
            "production_context",
            "debug",
            "panels",
        ],
        "debug runner result",
    );
    assert_eq!(run["schema_version"], serde_json::json!(1));
    assert_eq!(
        run["kind"],
        serde_json::json!("orv.editor.debug.runner.result")
    );
    assert!(run["state"]
        .as_str()
        .is_some_and(|state| state.ends_with("dist")));
    assert_keys(&run["panels"], &["debug"], "debug result panels root");
}

fn assert_runner_contract(runner: &serde_json::Value) {
    assert_keys(
        runner,
        &[
            "schema_version",
            "kind",
            "program",
            "source_bundle",
            "production_context",
            "result",
        ],
        "debug runner",
    );
    assert_eq!(runner["schema_version"], serde_json::json!(1));
    assert_eq!(runner["kind"], serde_json::json!("orv.editor.debug.runner"));
    assert!(runner["program"]
        .as_str()
        .is_some_and(|program| program.ends_with("app.orv")));
    assert!(runner["source_bundle"]
        .as_str()
        .is_some_and(|path| path.ends_with("source-bundle.json")));
    assert_result_artifact_contract(&runner["result"]);
}

fn assert_result_artifact_contract(result: &serde_json::Value) {
    assert_keys(
        result,
        &[
            "path",
            "html_path",
            "kind",
            "media_type",
            "panels",
            "panel_contract",
        ],
        "debug runner result artifact",
    );
    assert_eq!(
        result["path"],
        serde_json::json!("debug/session-result.json")
    );
    assert_eq!(
        result["html_path"],
        serde_json::json!("debug/session-result.html")
    );
    assert_eq!(
        result["kind"],
        serde_json::json!("orv.editor.debug.runner.result")
    );
    assert_eq!(result["media_type"], serde_json::json!("application/json"));
    assert_eq!(result["panels"], serde_json::json!(["debug"]));
    assert_panel_contract(&result["panel_contract"]);
}

fn assert_panel_contract(panel_contract: &serde_json::Value) {
    assert_keys(
        panel_contract,
        &["schema_version", "root", "sections"],
        "debug result artifact panel contract",
    );
    assert_eq!(panel_contract["schema_version"], serde_json::json!(1));
    assert_eq!(panel_contract["root"], serde_json::json!("panels.debug"));
    let sections = panel_contract["sections"]
        .as_array()
        .expect("debug panel contract sections");
    let mut names = BTreeSet::new();
    for section in sections {
        assert_keys(section, &["kind", "name", "path"], "debug panel section");
        let name = section["name"].as_str().expect("debug panel section name");
        let path = section["path"].as_str().expect("debug panel section path");
        assert!(section["kind"].as_str().is_some());
        assert_eq!(path, format!("panels.debug.{name}"));
        names.insert(name);
    }
    let expected = [
        "production_context",
        "production_summary",
        "session_summary",
        "source_bundle",
        "selected_frame",
        "stack_frames",
        "source_navigation",
        "scopes",
        "locals",
        "project_variables",
        "controls",
        "breakpoints",
        "function_breakpoints",
        "data_breakpoints",
        "exception_filters",
        "watch_expressions",
        "loaded_sources",
        "source_snapshots",
        "stopped_events",
        "events",
        "output_events",
    ]
    .into_iter()
    .collect::<BTreeSet<_>>();
    assert_eq!(names, expected, "debug panel section names drifted");
}

fn assert_production_context_contract(context: &serde_json::Value) {
    assert_keys(
        context,
        &[
            "schema_version",
            "kind",
            "build_dir",
            "source_bundle",
            "graph_contract",
            "preflight",
            "summary",
        ],
        "debug production context",
    );
    assert_eq!(context["schema_version"], serde_json::json!(1));
    assert_eq!(
        context["kind"],
        serde_json::json!("orv.editor.debug.production_context")
    );
    assert!(context["source_bundle"]
        .as_str()
        .is_some_and(|path| path.ends_with("source-bundle.json")));
    assert!(context["graph_contract"]
        .as_array()
        .is_some_and(|items| items.len() == 3));
    assert!(context["preflight"].as_array().is_some_and(Vec::is_empty));
    assert_production_summary_contract(&context["summary"]);
}

fn assert_production_summary_contract(summary: &serde_json::Value) {
    assert_keys(
        summary,
        &[
            "schema_version",
            "build_dir",
            "graph_contract_count",
            "source_bundle_file_count",
            "project_graph_node_count",
            "origin_entry_count",
            "client_target_count",
            "client_manifest_count",
            "client_capability_surface_count",
            "route_target_count",
            "native_server_target_count",
            "native_server_route_count",
            "native_server_blocker_count",
            "static_target_count",
            "static_verified_count",
            "preflight_target_count",
            "preflight_command_count",
            "preflight_route_count",
            "preflight_required_env_count",
            "preflight_optional_env_count",
            "preflight_smoke_summary_present_count",
            "preflight_smoke_summary_missing_count",
            "preflight_smoke_summary_missing_marker_count",
            "route_policy_count",
            "route_policy_kind_counts",
            "db_target_count",
            "commerce_target_count",
            "db_adapter_count",
            "commerce_adapter_count",
            "adapter_count",
            "missing_artifact_count",
        ],
        "debug production summary",
    );
    assert_eq!(summary["schema_version"], serde_json::json!(1));
    assert_eq!(summary["graph_contract_count"], serde_json::json!(3));
    assert_eq!(summary["source_bundle_file_count"], serde_json::json!(1));
    assert_eq!(summary["native_server_target_count"], serde_json::json!(0));
    assert_eq!(summary["native_server_route_count"], serde_json::json!(0));
    assert_eq!(summary["preflight_target_count"], serde_json::json!(0));
}

fn assert_debug_session_contract(debug: &serde_json::Value) {
    assert_keys(
        debug,
        &[
            "schema_version",
            "kind",
            "program",
            "adapter",
            "transport",
            "breakpoints",
            "function_breakpoints",
            "data_breakpoints",
            "exception_filters",
            "launch",
            "loaded_sources",
            "source_snapshots",
            "control",
            "controls",
            "watch_expressions",
            "stack",
            "scopes",
            "project_variables",
            "locals",
            "frames",
        ],
        "debug session",
    );
    assert_eq!(debug["schema_version"], serde_json::json!(1));
    assert_eq!(debug["kind"], serde_json::json!("orv.editor.debug"));
    assert_eq!(debug["transport"]["protocol"], serde_json::json!("dap"));
    assert_eq!(
        debug["transport"]["framing"],
        serde_json::json!("content-length")
    );
    assert!(debug["transport"]["request_count"]
        .as_u64()
        .is_some_and(|count| count > 0));
    assert_launch_source_bundle_contract(&debug["launch"]["body"]["sourceBundle"]);
    assert!(debug["loaded_sources"]["sources"]
        .as_array()
        .is_some_and(|sources| !sources.is_empty()));
    assert!(debug["source_snapshots"]
        .as_array()
        .is_some_and(|sources| !sources.is_empty()));
    assert!(debug["stack"]["stackFrames"]
        .as_array()
        .is_some_and(|frames| !frames.is_empty()));
    assert!(debug["locals"].as_array().is_some_and(|locals| {
        locals
            .iter()
            .any(|local| local["name"] == "total" && local["value"] == "41")
    }));
    assert!(debug["watch_expressions"].as_array().is_some_and(|items| {
        items.iter().any(|item| {
            item["expression"] == "total"
                && item["response"]["success"] == true
                && item["response"]["body"]["result"] == "41"
        })
    }));
}

fn assert_debug_panel_contract(panel: &serde_json::Value) {
    assert_keys(
        panel,
        &[
            "schema_version",
            "production_context",
            "production_summary",
            "session_summary",
            "source_bundle",
            "result_artifact",
            "selected_frame",
            "stack_frames",
            "source_navigation",
            "scopes",
            "project_variables",
            "locals",
            "control_count",
            "breakpoint_count",
            "function_breakpoint_count",
            "data_breakpoint_count",
            "exception_filter_count",
            "watch_expression_count",
            "loaded_source_count",
            "source_snapshot_count",
            "controls",
            "breakpoints",
            "function_breakpoints",
            "data_breakpoints",
            "exception_filters",
            "watch_expressions",
            "loaded_sources",
            "source_snapshots",
            "event_count",
            "stopped_event_count",
            "output_event_count",
            "events",
            "stopped_events",
            "output_events",
        ],
        "debug result panel",
    );
    assert_eq!(panel["schema_version"], serde_json::json!(1));
    assert_eq!(
        panel["production_summary"]["native_server_target_count"],
        serde_json::json!(0)
    );
    assert_eq!(
        panel["session_summary"]["source_bundle_file_count"],
        serde_json::json!(1)
    );
    assert_eq!(panel["source_bundle"]["fileCount"], serde_json::json!(1));
    assert_result_artifact_contract(&panel["result_artifact"]);
    assert_eq!(panel["control_count"], serde_json::json!(1));
    assert_eq!(panel["watch_expression_count"], serde_json::json!(1));
    assert!(panel["events"]
        .as_array()
        .is_some_and(|events| { events.iter().any(|event| event["event"] == "stopped") }));
}

fn assert_launch_source_bundle_contract(source_bundle: &serde_json::Value) {
    assert_keys(
        source_bundle,
        &["entry", "fileCount", "hash", "path"],
        "debug launch sourceBundle",
    );
    assert!(source_bundle["entry"]
        .as_str()
        .is_some_and(|entry| entry.ends_with("app.orv")));
    assert_eq!(source_bundle["fileCount"], serde_json::json!(1));
    assert!(source_bundle["hash"]
        .as_str()
        .is_some_and(|hash| hash.len() == 16));
    assert!(source_bundle["path"]
        .as_str()
        .is_some_and(|path| path.ends_with("source-bundle.json")));
}

fn assert_written_result_artifacts(build_out: &Path, run: &serde_json::Value) {
    let result_path = build_out.join("debug").join("session-result.json");
    let html_path = build_out.join("debug").join("session-result.html");
    assert_eq!(read_json(&result_path), *run);
    let html = std::fs::read_to_string(html_path).expect("result html");
    assert!(html.contains("id=\"orv-debug-result\""));
    assert!(html.contains("Selected Frame"));
    assert!(html.contains("Production Summary"));
    assert!(html.contains("Source Bundle"));
    assert!(html.contains("source_bundle"));
    assert!(html.contains("Watch Expressions"));
}
