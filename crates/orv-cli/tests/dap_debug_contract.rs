use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

const DAP_STDIO_INITIALIZE_GOLDEN: &str =
    include_str!("../../../docs/samples/dap-stdio-initialize-v1.golden.json");
const DAP_RUNNER_RESULT_INVENTORY_GOLDEN: &str =
    include_str!("../../../docs/samples/dap-runner-result-inventory-v1.golden.json");
const DAP_STDIO_LAUNCH_STEP_GOLDEN: &str =
    include_str!("../../../docs/samples/dap-stdio-launch-step-v1.golden.json");
const DAP_STDIO_SOURCE_BUNDLE_LAUNCH_GOLDEN: &str =
    include_str!("../../../docs/samples/dap-stdio-source-bundle-launch-v1.golden.json");

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
    assert_dap_stdio_initialize_golden(&frames);

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

fn assert_dap_stdio_initialize_golden(frames: &[serde_json::Value]) {
    let expected: Vec<serde_json::Value> =
        serde_json::from_str(DAP_STDIO_INITIALIZE_GOLDEN).expect("DAP initialize golden");
    assert_eq!(frames, expected, "DAP stdio initialize golden drift");
}

#[test]
fn dap_debug_session_v1_freezes_stdio_launch_step_contract() {
    let root = temp_output_dir("dap-stdio-launch-step-contract");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("temp root");
    let source = build_debug_fixture(&root);
    let source_arg = source.display().to_string();

    let frames = run_dap_stdio_frames(&[
        serde_json::json!({
            "seq": 1,
            "type": "request",
            "command": "initialize",
            "arguments": {},
        }),
        serde_json::json!({
            "seq": 2,
            "type": "request",
            "command": "launch",
            "arguments": {
                "program": source_arg,
            },
        }),
        serde_json::json!({
            "seq": 3,
            "type": "request",
            "command": "loadedSources",
            "arguments": {},
        }),
        serde_json::json!({
            "seq": 4,
            "type": "request",
            "command": "source",
            "arguments": {
                "sourceReference": 1,
            },
        }),
        serde_json::json!({
            "seq": 5,
            "type": "request",
            "command": "next",
            "arguments": {
                "threadId": 1,
            },
        }),
        serde_json::json!({
            "seq": 6,
            "type": "request",
            "command": "stackTrace",
            "arguments": {
                "threadId": 1,
            },
        }),
        serde_json::json!({
            "seq": 7,
            "type": "request",
            "command": "scopes",
            "arguments": {
                "frameId": 1,
            },
        }),
        serde_json::json!({
            "seq": 8,
            "type": "request",
            "command": "variables",
            "arguments": {
                "variablesReference": 1,
            },
        }),
        serde_json::json!({
            "seq": 9,
            "type": "request",
            "command": "variables",
            "arguments": {
                "variablesReference": 2,
            },
        }),
        serde_json::json!({
            "seq": 10,
            "type": "request",
            "command": "evaluate",
            "arguments": {
                "expression": "total",
                "frameId": 1,
                "context": "watch",
            },
        }),
    ]);
    assert_dap_stdio_launch_step_golden(&frames);

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn dap_debug_session_v1_freezes_stdio_source_bundle_launch_contract() {
    let root = temp_output_dir("dap-stdio-source-bundle-launch-contract");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("temp root");
    let source = build_debug_fixture(&root);
    let build_out = root.join("dist");
    let source_arg = source.display().to_string();
    let build_arg = build_out.display().to_string();

    run_orv(&["build", &source_arg, "--out", &build_arg, "--prod"]);
    std::fs::remove_file(&source).expect("remove original source");
    let source_bundle_arg = build_out.join("source-bundle.json").display().to_string();

    let frames = run_dap_stdio_frames(&[
        serde_json::json!({
            "seq": 1,
            "type": "request",
            "command": "initialize",
            "arguments": {},
        }),
        serde_json::json!({
            "seq": 2,
            "type": "request",
            "command": "launch",
            "arguments": {
                "program": source_arg,
                "sourceBundle": source_bundle_arg,
            },
        }),
        serde_json::json!({
            "seq": 3,
            "type": "request",
            "command": "loadedSources",
            "arguments": {},
        }),
        serde_json::json!({
            "seq": 4,
            "type": "request",
            "command": "source",
            "arguments": {
                "sourceReference": 1,
            },
        }),
    ]);
    assert_dap_stdio_source_bundle_launch_golden(&frames);

    let _ = std::fs::remove_dir_all(&root);
}

fn assert_dap_stdio_launch_step_golden(frames: &[serde_json::Value]) {
    let expected: serde_json::Value =
        serde_json::from_str(DAP_STDIO_LAUNCH_STEP_GOLDEN).expect("DAP launch/step golden");
    assert_eq!(
        dap_stdio_launch_step_inventory(frames),
        expected,
        "DAP stdio launch/step golden drift"
    );
}

fn dap_stdio_launch_step_inventory(frames: &[serde_json::Value]) -> serde_json::Value {
    assert_eq!(frames.len(), 13, "DAP stdio launch/step frame count drift");
    let launch = &frames[2];
    let loaded_sources = &frames[3];
    let source = &frames[4];
    let stack = &frames[8];
    let scopes = &frames[9];
    let project_variables = &frames[10];
    let locals = &frames[11];
    let evaluate = &frames[12];

    serde_json::json!({
        "frame_count": frames.len(),
        "frame_sequence": frames.iter().map(protocol_frame_inventory).collect::<Vec<_>>(),
        "launch": {
            "command": launch["command"],
            "success": launch["success"],
            "type": launch["type"],
            "diagnostics": launch["body"]["diagnostics"],
            "entry": launch_entry_inventory(&launch["body"]["entry"]),
            "projectGraphNodes": launch["body"]["projectGraphNodes"],
            "runtime": launch["body"]["runtime"],
            "sourceBundle": launch["body"]["sourceBundle"],
        },
        "loaded_sources": map_array(
            &loaded_sources["body"]["sources"],
            "stdio loaded sources",
            source_inventory,
        ),
        "source": {
            "command": source["command"],
            "success": source["success"],
            "type": source["type"],
            "body": source["body"],
        },
        "events": frames
            .iter()
            .filter(|frame| frame["type"] == serde_json::json!("event"))
            .map(event_inventory)
            .collect::<Vec<_>>(),
        "stack": {
            "stackFrames": map_array(
                &stack["body"]["stackFrames"],
                "stdio stack frames",
                stack_frame_inventory,
            ),
            "totalFrames": stack["body"]["totalFrames"],
        },
        "scope_names": scope_names(&scopes["body"]["scopes"]),
        "project_variables": map_array(
            &project_variables["body"]["variables"],
            "stdio project variables",
            variable_inventory,
        ),
        "locals": map_array(&locals["body"]["variables"], "stdio locals", variable_inventory),
        "evaluate": {
            "command": evaluate["command"],
            "success": evaluate["success"],
            "type": evaluate["type"],
            "body": evaluate["body"],
        },
    })
}

fn assert_dap_stdio_source_bundle_launch_golden(frames: &[serde_json::Value]) {
    let expected: serde_json::Value = serde_json::from_str(DAP_STDIO_SOURCE_BUNDLE_LAUNCH_GOLDEN)
        .expect("DAP sourceBundle launch golden");
    assert_eq!(
        dap_stdio_source_bundle_launch_inventory(frames),
        expected,
        "DAP stdio sourceBundle launch golden drift"
    );
}

fn dap_stdio_source_bundle_launch_inventory(frames: &[serde_json::Value]) -> serde_json::Value {
    assert_eq!(
        frames.len(),
        5,
        "DAP stdio sourceBundle launch frame count drift"
    );
    let launch = &frames[2];
    let loaded_sources = &frames[3];
    let source = &frames[4];

    serde_json::json!({
        "frame_count": frames.len(),
        "frame_sequence": frames.iter().map(protocol_frame_inventory).collect::<Vec<_>>(),
        "launch": {
            "command": launch["command"],
            "success": launch["success"],
            "type": launch["type"],
            "diagnostics": launch["body"]["diagnostics"],
            "entry": launch_entry_inventory(&launch["body"]["entry"]),
            "projectGraphNodes": launch["body"]["projectGraphNodes"],
            "runtime": launch["body"]["runtime"],
            "sourceBundle": source_bundle_inventory(&launch["body"]["sourceBundle"]),
        },
        "loaded_sources": map_array(
            &loaded_sources["body"]["sources"],
            "stdio sourceBundle loaded sources",
            source_inventory_with_checksums,
        ),
        "source": {
            "command": source["command"],
            "success": source["success"],
            "type": source["type"],
            "body": source["body"],
        },
    })
}

fn assert_dap_runner_result_inventory_golden(run: &serde_json::Value) {
    let expected: serde_json::Value =
        serde_json::from_str(DAP_RUNNER_RESULT_INVENTORY_GOLDEN).expect("DAP result golden");
    assert_eq!(
        dap_runner_result_inventory(run),
        expected,
        "DAP runner result inventory golden drift"
    );
}

fn dap_runner_result_inventory(run: &serde_json::Value) -> serde_json::Value {
    let runner = &run["runner"];
    let production_context = &run["production_context"];
    let debug = &run["debug"];

    serde_json::json!({
        "schema_version": run["schema_version"],
        "kind": run["kind"],
        "state": "<build-dir>",
        "runner": {
            "schema_version": runner["schema_version"],
            "kind": runner["kind"],
            "program": "<entry>",
            "source_bundle": "<source-bundle>",
            "result": result_artifact_inventory(&runner["result"]),
        },
        "production_context": production_context_inventory(production_context),
        "debug": {
            "schema_version": debug["schema_version"],
            "kind": debug["kind"],
            "program": "<entry>",
            "adapter": debug["adapter"],
            "transport": debug["transport"],
            "launch": launch_inventory(&debug["launch"]),
            "loaded_sources": map_array(
                &debug["loaded_sources"]["sources"],
                "debug loaded sources",
                source_inventory,
            ),
            "source_snapshots": map_array(
                &debug["source_snapshots"],
                "debug source snapshots",
                source_snapshot_inventory,
            ),
            "stack": {
                "stackFrames": map_array(
                    &debug["stack"]["stackFrames"],
                    "debug stack frames",
                    stack_frame_inventory,
                ),
                "totalFrames": debug["stack"]["totalFrames"],
            },
            "scope_names": scope_names(&debug["scopes"]["scopes"]),
            "project_variables": map_array(
                &debug["project_variables"],
                "debug project variables",
                variable_inventory,
            ),
            "locals": map_array(&debug["locals"], "debug locals", variable_inventory),
            "control": control_inventory(&debug["control"]),
            "controls": map_array(&debug["controls"], "debug controls", control_inventory),
            "watch_expressions": map_array(
                &debug["watch_expressions"],
                "debug watch expressions",
                watch_expression_inventory,
            ),
            "frame_sequence": map_array(
                &debug["frames"],
                "debug protocol frames",
                protocol_frame_inventory,
            ),
        },
        "panel": debug_panel_inventory(&run["panels"]["debug"]),
    })
}

fn debug_panel_inventory(panel: &serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "schema_version": panel["schema_version"],
        "production_context": production_context_inventory(&panel["production_context"]),
        "production_summary": production_summary_inventory(&panel["production_summary"]),
        "session_summary": session_summary_inventory(&panel["session_summary"]),
        "source_bundle": source_bundle_inventory(&panel["source_bundle"]),
        "result_artifact": result_artifact_inventory(&panel["result_artifact"]),
        "selected_frame": stack_frame_inventory(&panel["selected_frame"]),
        "source_navigation": source_navigation_inventory(&panel["source_navigation"]),
        "scope_names": scope_names(&panel["scopes"]["scopes"]),
        "project_variables": map_array(
            &panel["project_variables"],
            "debug panel project variables",
            variable_inventory,
        ),
        "locals": map_array(&panel["locals"], "debug panel locals", variable_inventory),
        "counts": {
            "control_count": panel["control_count"],
            "breakpoint_count": panel["breakpoint_count"],
            "function_breakpoint_count": panel["function_breakpoint_count"],
            "data_breakpoint_count": panel["data_breakpoint_count"],
            "exception_filter_count": panel["exception_filter_count"],
            "watch_expression_count": panel["watch_expression_count"],
            "loaded_source_count": panel["loaded_source_count"],
            "source_snapshot_count": panel["source_snapshot_count"],
            "event_count": panel["event_count"],
            "stopped_event_count": panel["stopped_event_count"],
            "output_event_count": panel["output_event_count"],
        },
        "controls": map_array(&panel["controls"], "debug panel controls", control_inventory),
        "watch_expressions": map_array(
            &panel["watch_expressions"],
            "debug panel watch expressions",
            watch_expression_inventory,
        ),
        "loaded_sources": map_array(
            &panel["loaded_sources"]["sources"],
            "debug panel loaded sources",
            source_inventory,
        ),
        "source_snapshots": map_array(
            &panel["source_snapshots"],
            "debug panel source snapshots",
            source_snapshot_inventory,
        ),
        "events": map_array(&panel["events"], "debug panel events", event_inventory),
        "stopped_events": map_array(
            &panel["stopped_events"],
            "debug panel stopped events",
            event_inventory,
        ),
        "output_events": map_array(
            &panel["output_events"],
            "debug panel output events",
            event_inventory,
        ),
    })
}

fn map_array(
    value: &serde_json::Value,
    context: &str,
    mapper: fn(&serde_json::Value) -> serde_json::Value,
) -> Vec<serde_json::Value> {
    value
        .as_array()
        .unwrap_or_else(|| panic!("{context} must be an array"))
        .iter()
        .map(mapper)
        .collect()
}

fn array_len(value: &serde_json::Value, context: &str) -> usize {
    value
        .as_array()
        .unwrap_or_else(|| panic!("{context} must be an array"))
        .len()
}

fn production_context_inventory(context: &serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "schema_version": context["schema_version"],
        "kind": context["kind"],
        "build_dir": "<build-dir>",
        "source_bundle": "<source-bundle>",
        "graph_contract_count": array_len(&context["graph_contract"], "graph contract"),
        "preflight_count": array_len(&context["preflight"], "production preflight"),
        "summary": production_summary_inventory(&context["summary"]),
    })
}

fn production_summary_inventory(summary: &serde_json::Value) -> serde_json::Value {
    let mut normalized = summary.clone();
    normalized["build_dir"] = serde_json::json!("<build-dir>");
    normalized
}

fn result_artifact_inventory(result: &serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "path": result["path"],
        "html_path": result["html_path"],
        "kind": result["kind"],
        "media_type": result["media_type"],
        "panels": result["panels"],
        "panel_contract": result["panel_contract"],
    })
}

fn launch_inventory(launch: &serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "command": launch["command"],
        "success": launch["success"],
        "type": launch["type"],
        "diagnostics": launch["body"]["diagnostics"],
        "projectGraphNodes": launch["body"]["projectGraphNodes"],
        "runtime": launch["body"]["runtime"],
        "sourceBundle": source_bundle_inventory(&launch["body"]["sourceBundle"]),
    })
}

fn launch_entry_inventory(entry: &serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "name": entry["name"],
        "path": "<entry>",
        "uri": "file://<entry>",
    })
}

fn source_bundle_inventory(source_bundle: &serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "entry": "<entry>",
        "fileCount": source_bundle["fileCount"],
        "hash": "<source-bundle-hash>",
        "path": "<source-bundle>",
    })
}

fn source_inventory(source: &serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "checksum_count": array_len(&source["checksums"], "source checksums"),
        "name": source["name"],
        "path": "<entry>",
        "sourceReference": source["sourceReference"],
        "uri": "file://<entry>",
    })
}

fn source_inventory_with_checksums(source: &serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "checksums": map_array(
            &source["checksums"],
            "source checksums",
            source_checksum_inventory,
        ),
        "name": source["name"],
        "path": "<entry>",
        "sourceReference": source["sourceReference"],
        "uri": "file://<entry>",
    })
}

fn source_checksum_inventory(checksum: &serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "algorithm": checksum["algorithm"],
        "checksum": checksum["checksum"],
    })
}

fn source_snapshot_inventory(snapshot: &serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "checksum_algorithm": snapshot["checksum"]["algorithm"],
        "checksum_value": snapshot["checksum"]["value"],
        "content_length": snapshot["content_length"],
        "line_count": snapshot["line_count"],
        "mimeType": snapshot["response"]["body"]["mimeType"],
        "source": source_inventory(&snapshot["source"]),
    })
}

fn stack_frame_inventory(frame: &serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "column": frame["column"],
        "id": frame["id"],
        "line": frame["line"],
        "name": frame["name"],
        "source": source_inventory(&frame["source"]),
    })
}

fn source_navigation_inventory(source_navigation: &serde_json::Value) -> serde_json::Value {
    let selected = &source_navigation["selected"];
    serde_json::json!({
        "schema_version": source_navigation["schema_version"],
        "frame_count": source_navigation["frame_count"],
        "selected": {
            "column": selected["column"],
            "frame_id": selected["frame_id"],
            "frame_name": selected["frame_name"],
            "line": selected["line"],
            "source": {
                "name": selected["source"]["name"],
                "path": "<entry>",
            },
        },
    })
}

fn scope_names(scopes: &serde_json::Value) -> Vec<serde_json::Value> {
    scopes
        .as_array()
        .expect("debug scopes must be an array")
        .iter()
        .map(|scope| scope["name"].clone())
        .collect()
}

fn variable_inventory(variable: &serde_json::Value) -> serde_json::Value {
    let value = if variable["name"] == serde_json::json!("entry") {
        serde_json::json!("<entry>")
    } else {
        variable["value"].clone()
    };
    serde_json::json!({
        "name": variable["name"],
        "type": variable["type"],
        "value": value,
        "variablesReference": variable["variablesReference"],
    })
}

fn control_inventory(control: &serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "name": control["name"],
        "request": control["request"],
        "response": {
            "command": control["response"]["command"],
            "success": control["response"]["success"],
            "type": control["response"]["type"],
        },
    })
}

fn watch_expression_inventory(watch_expression: &serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "expression": watch_expression["expression"],
        "request": {
            "arguments": watch_expression["request"]["arguments"],
            "command": watch_expression["request"]["command"],
            "type": watch_expression["request"]["type"],
        },
        "response": {
            "body": watch_expression["response"]["body"],
            "command": watch_expression["response"]["command"],
            "success": watch_expression["response"]["success"],
            "type": watch_expression["response"]["type"],
        },
    })
}

fn protocol_frame_inventory(frame: &serde_json::Value) -> serde_json::Value {
    let mut inventory = serde_json::Map::new();
    inventory.insert("type".to_string(), frame["type"].clone());
    if let Some(command) = frame.get("command") {
        inventory.insert("command".to_string(), command.clone());
    }
    if let Some(event) = frame.get("event") {
        inventory.insert("event".to_string(), event.clone());
    }
    if let Some(success) = frame.get("success") {
        inventory.insert("success".to_string(), success.clone());
    }
    serde_json::Value::Object(inventory)
}

fn event_inventory(event: &serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "body": event["body"],
        "event": event["event"],
        "type": event["type"],
    })
}

fn session_summary_inventory(summary: &serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "schema_version": summary["schema_version"],
        "breakpoint_count": summary["breakpoint_count"],
        "control_count": summary["control_count"],
        "data_breakpoint_count": summary["data_breakpoint_count"],
        "event_count": summary["event_count"],
        "exception_filter_count": summary["exception_filter_count"],
        "frame_count": summary["frame_count"],
        "function_breakpoint_count": summary["function_breakpoint_count"],
        "last_event": summary["last_event"],
        "last_stopped_reason": summary["last_stopped_reason"],
        "output_event_count": summary["output_event_count"],
        "program": "<entry>",
        "request_count": summary["request_count"],
        "selected_frame": summary["selected_frame"],
        "selected_frame_id": summary["selected_frame_id"],
        "selected_line": summary["selected_line"],
        "selected_source": "<entry>",
        "source_bundle": source_bundle_inventory(&summary["source_bundle"]),
        "source_bundle_file_count": summary["source_bundle_file_count"],
        "stopped_event_count": summary["stopped_event_count"],
        "watch_expression_count": summary["watch_expression_count"],
    })
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
    assert_dap_runner_result_inventory_golden(&run);
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
fn dap_debug_runner_rejects_export_debug_source_inventory_reference_drift() {
    let root = temp_output_dir("dap-debug-export-debug-source-inventory-reference");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("temp root");
    let source = build_debug_fixture(&root);
    let out = root.join("editor");
    let source_arg = source.display().to_string();
    let out_arg = out.display().to_string();
    run_orv(&["editor", "export", &source_arg, "--out", &out_arg]);
    let state = out.join("state.json");
    let mut value = read_json(&state);
    value["debug"]["source_inventory"]["sources"][0]["source_reference"] = serde_json::json!(99);
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
        "source inventory reference drift must fail"
    );
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(
            "editor export debug source_inventory.sources[0] source_reference must match DAP source"
        ),
        "{stderr}"
    );

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn dap_debug_runner_rejects_export_debug_source_inventory_checksum_drift() {
    let root = temp_output_dir("dap-debug-export-debug-source-inventory-checksum");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("temp root");
    let source = build_debug_fixture(&root);
    let out = root.join("editor");
    let source_arg = source.display().to_string();
    let out_arg = out.display().to_string();
    run_orv(&["editor", "export", &source_arg, "--out", &out_arg]);
    let state = out.join("state.json");
    let mut value = read_json(&state);
    value["debug"]["source_inventory"]["sources"][0]["checksum"]["value"] =
        serde_json::json!("drift");
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
        "source inventory checksum drift must fail"
    );
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(
            "editor export debug source_inventory.sources[0] checksum must match DAP source"
        ),
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
