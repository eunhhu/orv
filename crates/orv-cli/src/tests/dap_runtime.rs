use super::*;

#[test]
fn dap_live_step_in_rejects_target_id() {
    let dir = temp_output_dir("dap-live-step-in-target");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let source = dir.join("app.orv");
    std::fs::write(&source, "let first: int = 1\nlet second: int = 2\n").expect("write source");
    let mut session = DapSession::default();

    session
        .message_response(&serde_json::json!({
            "seq": 218,
            "type": "request",
            "command": "launch",
            "arguments": {
                "program": format!("file://{}", source.display()),
                "live": true,
            },
        }))
        .expect("launch response");
    let step_in = session
        .message_response(&serde_json::json!({
            "seq": 219,
            "type": "request",
            "command": "stepIn",
            "arguments": {
                "threadId": 1,
                "targetId": 1_000_000,
            },
        }))
        .expect("stepIn response");
    let stack = session
        .message_response(&serde_json::json!({
            "seq": 220,
            "type": "request",
            "command": "stackTrace",
            "arguments": {
                "threadId": 1,
            },
        }))
        .expect("stack response");

    assert_eq!(step_in["success"], false, "{step_in}");
    assert!(step_in["message"]
        .as_str()
        .is_some_and(|message| message.contains("targetId is unavailable in live debug mode")));
    assert_eq!(stack["body"]["stackFrames"][0]["line"], 1);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn dap_long_running_server_state_uses_server_frame_without_runtime() {
    let dir = temp_output_dir("dap-long-running-server-state");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let source = dir.join("app.orv");
    std::fs::write(
        &source,
        r"@server {
  @listen 0
  @route GET /ping { @respond 200 { ok: true } }
}
",
    )
    .expect("write source");
    let loaded = orv_project::load_project(&source).expect("load project");
    let resolved = orv_resolve::resolve(&loaded.program);
    let lowered = orv_analyzer::lower_with_diagnostics(&loaded.program, &resolved);
    let sources = loaded
        .files
        .iter()
        .enumerate()
        .map(|(index, file)| dap_source_info(file, u64::try_from(index + 1).unwrap_or(u64::MAX)))
        .collect::<Vec<_>>();

    let (runtime, frames) =
        dap_long_running_runtime_state(&lowered.program, &loaded.files, &sources);

    assert!(dap_program_has_long_running_runtime(&lowered.program));
    assert_eq!(runtime.status, "paused");
    assert_eq!(frames.len(), 1);
    assert_eq!(frames[0].line, 1);
    assert_eq!(frames[0].stack[0].name, "server runtime");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn dap_attach_request_enables_runtime_transport() {
    let dir = temp_output_dir("dap-attach-runtime");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let source = dir.join("app.orv");
    std::fs::write(
        &source,
        "@server { @listen 0 @route GET /ping { @respond 200 { ok: true } } }\n",
    )
    .expect("write source");
    let mut session = DapSession::default();

    let attach = session
        .message_response(&serde_json::json!({
            "seq": 236,
            "type": "request",
            "command": "attach",
            "arguments": {
                "program": format!("file://{}", source.display()),
                "attachRuntimeMode": "inProcess",
            },
        }))
        .expect("attach response");
    assert_eq!(attach["type"], "response");
    assert_eq!(attach["command"], "attach");
    assert_eq!(attach["success"], true, "{attach}");
    assert_eq!(
        attach["body"]["runtime"]["async"]["transport"]["kind"],
        "in-process"
    );
    assert_eq!(
        attach["body"]["runtime"]["async"]["transport"]["state"],
        "detached"
    );
    session
        .message_response(&serde_json::json!({
            "seq": 237,
            "type": "request",
            "command": "continue",
            "arguments": {
                "threadId": 1,
            },
        }))
        .expect("continue response");
    let address = session
        .launched
        .as_ref()
        .and_then(|launched| launched.async_runtime.as_ref())
        .and_then(|runtime| runtime.transport.as_ref())
        .and_then(|transport| transport.address.clone())
        .expect("in-process runtime address");

    let response = send_raw_http(&address, "/ping");

    assert!(response.starts_with("HTTP/1.1 200"), "{response}");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn dap_in_process_runtime_exposes_request_trace_json() {
    let dir = temp_output_dir("dap-runtime-request-trace");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let source = dir.join("app.orv");
    std::fs::write(
        &source,
        "@server { @listen 0 @route GET /ping { @respond 200 { ok: true } } }\n",
    )
    .expect("write source");
    let mut session = DapSession::default();

    session
        .message_response(&serde_json::json!({
            "seq": 236,
            "type": "request",
            "command": "launch",
            "arguments": {
                "program": format!("file://{}", source.display()),
                "attachRuntime": true,
                "attachRuntimeMode": "inProcess",
            },
        }))
        .expect("launch response");
    session
        .message_response(&serde_json::json!({
            "seq": 237,
            "type": "request",
            "command": "continue",
            "arguments": {
                "threadId": 1,
            },
        }))
        .expect("continue response");
    let address = session
        .launched
        .as_ref()
        .and_then(|launched| launched.async_runtime.as_ref())
        .and_then(|runtime| runtime.transport.as_ref())
        .and_then(|transport| transport.address.clone())
        .expect("in-process runtime address");

    let response = send_raw_http(&address, "/ping");
    assert!(response.starts_with("HTTP/1.1 200"), "{response}");

    let variables = session
        .message_response(&serde_json::json!({
            "seq": 238,
            "type": "request",
            "command": "variables",
            "arguments": {
                "variablesReference": 1,
            },
        }))
        .expect("variables response");
    let trace = session
        .message_response(&serde_json::json!({
            "seq": 239,
            "type": "request",
            "command": "evaluate",
            "arguments": {
                "expression": "runtimeRequestTrace",
            },
        }))
        .expect("trace evaluate response");
    let completions = session
        .message_response(&serde_json::json!({
            "seq": 240,
            "type": "request",
            "command": "completions",
            "arguments": {
                "text": "runtimeRequestT",
                "column": 16,
                "line": 1,
            },
        }))
        .expect("completions response");

    assert!(variables["body"]["variables"]
        .as_array()
        .expect("variables")
        .iter()
        .any(|variable| variable["name"] == "runtimeRequestTrace" && variable["type"] == "json"));
    assert_eq!(trace["success"], true, "{trace}");
    let trace_json: serde_json::Value =
        serde_json::from_str(trace["body"]["result"].as_str().expect("trace json string"))
            .expect("trace json");
    assert_eq!(trace_json["schema_version"], 1);
    assert_eq!(trace_json["kind"], "orv.production.trace");
    assert_eq!(trace_json["frames"][0]["method"], "GET");
    assert_eq!(trace_json["frames"][0]["path"], "/ping");
    assert_eq!(trace_json["frames"][0]["status"], 200);
    assert!(trace_json["frames"][0]["route_origin_id"]
        .as_str()
        .is_some_and(|origin| origin.starts_with("ori_")));
    assert!(completions["body"]["targets"]
        .as_array()
        .expect("completion targets")
        .iter()
        .any(|target| target["label"] == "runtimeRequestTrace"));
    drop(session);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn dap_in_process_runtime_flushes_request_trace_path_on_pause() {
    let dir = temp_output_dir("dap-runtime-request-trace-path");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let source = dir.join("app.orv");
    let trace_path = dir.join("trace").join("requests.json");
    std::fs::write(
        &source,
        "@server { @listen 0 @route GET /ping { @respond 200 { ok: true } } }\n",
    )
    .expect("write source");
    let mut session = DapSession::default();

    let launch = dap_test_request(
        &mut session,
        241,
        "launch",
        serde_json::json!({
            "program": format!("file://{}", source.display()),
            "attachRuntime": true,
            "attachRuntimeMode": "inProcess",
            "runtimeRequestTracePath": trace_path.display().to_string(),
        }),
    );
    dap_test_request(
        &mut session,
        242,
        "continue",
        serde_json::json!({ "threadId": 1 }),
    );
    let address = session
        .launched
        .as_ref()
        .and_then(|launched| launched.async_runtime.as_ref())
        .and_then(|runtime| runtime.transport.as_ref())
        .and_then(|transport| transport.address.clone())
        .expect("in-process runtime address");

    let response = send_raw_http(&address, "/ping");
    assert!(response.starts_with("HTTP/1.1 200"), "{response}");
    let pause = dap_test_request(
        &mut session,
        243,
        "pause",
        serde_json::json!({ "threadId": 1 }),
    );

    assert_eq!(launch["success"], true, "{launch}");
    assert_eq!(pause["success"], true, "{pause}");
    let trace = read_json_value(&trace_path).expect("trace file");
    assert_eq!(trace["schema_version"], 1);
    assert_eq!(trace["kind"], "orv.production.trace");
    assert_eq!(trace["frames"][0]["method"], "GET");
    assert_eq!(trace["frames"][0]["path"], "/ping");
    assert_eq!(trace["frames"][0]["status"], 200);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn dap_in_process_runtime_exposes_request_trace_path_expression() {
    let dir = temp_output_dir("dap-runtime-request-trace-path-expression");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let source = dir.join("app.orv");
    let trace_path = dir.join("trace").join("requests.json");
    std::fs::write(
        &source,
        "@server { @listen 0 @route GET /ping { @respond 200 { ok: true } } }\n",
    )
    .expect("write source");
    let mut session = DapSession::default();

    let launch = dap_test_request(
        &mut session,
        247,
        "launch",
        serde_json::json!({
            "program": format!("file://{}", source.display()),
            "attachRuntime": true,
            "attachRuntimeMode": "inProcess",
            "runtimeRequestTracePath": trace_path.display().to_string(),
        }),
    );
    let variables = dap_test_request(
        &mut session,
        248,
        "variables",
        serde_json::json!({ "variablesReference": 1 }),
    );
    let trace_path_value = dap_test_request(
        &mut session,
        249,
        "evaluate",
        serde_json::json!({ "expression": "runtimeRequestTracePath" }),
    );
    let completions = dap_test_request(
        &mut session,
        250,
        "completions",
        serde_json::json!({
            "text": "runtimeRequestTraceP",
            "column": 21,
            "line": 1,
        }),
    );

    assert_eq!(launch["success"], true, "{launch}");
    assert!(variables["body"]["variables"]
        .as_array()
        .expect("variables")
        .iter()
        .any(|variable| variable["name"] == "runtimeRequestTracePath"
            && variable["value"] == trace_path.display().to_string()));
    assert_eq!(trace_path_value["success"], true, "{trace_path_value}");
    assert_eq!(
        trace_path_value["body"]["result"],
        trace_path.display().to_string()
    );
    assert!(completions["body"]["targets"]
        .as_array()
        .expect("completion targets")
        .iter()
        .any(|target| target["label"] == "runtimeRequestTracePath"));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn dap_step_back_moves_to_previous_runtime_frame() {
    let dir = temp_output_dir("dap-step-back");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let source = dir.join("app.orv");
    std::fs::write(&source, "let first: int = 1\nlet second: int = 2\n").expect("write source");
    let mut session = DapSession::default();

    session
        .message_response(&serde_json::json!({
            "seq": 186,
            "type": "request",
            "command": "launch",
            "arguments": {
                "program": format!("file://{}", source.display()),
            },
        }))
        .expect("launch response");
    session
        .message_response(&serde_json::json!({
            "seq": 187,
            "type": "request",
            "command": "next",
            "arguments": {
                "threadId": 1,
            },
        }))
        .expect("next response");
    let _ = session.drain_pending_events();
    let step_back = session
        .message_response(&serde_json::json!({
            "seq": 188,
            "type": "request",
            "command": "stepBack",
            "arguments": {
                "threadId": 1,
            },
        }))
        .expect("stepBack response");
    let events = session.drain_pending_events();
    let stack = session
        .message_response(&serde_json::json!({
            "seq": 189,
            "type": "request",
            "command": "stackTrace",
            "arguments": {
                "threadId": 1,
            },
        }))
        .expect("stack response");

    assert_eq!(step_back["success"], true, "{step_back}");
    assert_eq!(stack["body"]["stackFrames"][0]["line"], 1);
    assert!(events.iter().any(|event| {
        event["type"] == "event" && event["event"] == "stopped" && event["body"]["reason"] == "step"
    }));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn dap_next_queues_output_for_reached_runtime_frame() {
    let dir = temp_output_dir("dap-next-output-frame");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let source = dir.join("app.orv");
    std::fs::write(&source, "let first: int = 1\n@out \"second\"\n").expect("write source");
    let mut session = DapSession::default();

    session
        .message_response(&serde_json::json!({
            "seq": 166,
            "type": "request",
            "command": "launch",
            "arguments": {
                "program": format!("file://{}", source.display()),
            },
        }))
        .expect("launch response");
    assert!(session.drain_pending_events().is_empty());
    session
        .message_response(&serde_json::json!({
            "seq": 167,
            "type": "request",
            "command": "next",
            "arguments": {
                "threadId": 1,
            },
        }))
        .expect("next response");
    let events = session.drain_pending_events();

    assert!(events.iter().any(|event| {
        event["type"] == "event"
            && event["event"] == "output"
            && event["body"]["category"] == "stdout"
            && event["body"]["output"] == "second\n"
    }));
    assert!(events
        .iter()
        .any(|event| event["type"] == "event" && event["event"] == "stopped"));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn dap_stack_trace_names_runtime_function_frame() {
    let dir = temp_output_dir("dap-function-stack-frame");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let source = dir.join("app.orv");
    std::fs::write(
        &source,
        r"function add(a: int, b: int): int -> {
  let result: int = a + b
  result
}
let total: int = add(2, 3)
",
    )
    .expect("write source");
    let mut session = DapSession::default();

    session
        .message_response(&serde_json::json!({
            "seq": 163,
            "type": "request",
            "command": "launch",
            "arguments": {
                "program": format!("file://{}", source.display()),
            },
        }))
        .expect("launch response");
    session
        .message_response(&serde_json::json!({
            "seq": 164,
            "type": "request",
            "command": "stepIn",
            "arguments": {
                "threadId": 1,
            },
        }))
        .expect("stepIn response");
    let stack = session
        .message_response(&serde_json::json!({
            "seq": 165,
            "type": "request",
            "command": "stackTrace",
            "arguments": {
                "threadId": 1,
            },
        }))
        .expect("stack response");

    assert_eq!(stack["success"], true, "{stack}");
    assert_eq!(stack["body"]["stackFrames"][0]["name"], "add");
    assert_eq!(stack["body"]["stackFrames"][0]["line"], 2);
    assert_eq!(stack["body"]["stackFrames"][1]["name"], "orv entry");
    assert_eq!(stack["body"]["totalFrames"], 2);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn dap_locals_reflect_runtime_reassignment_after_step() {
    let dir = temp_output_dir("dap-runtime-assign-locals");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let source = dir.join("app.orv");
    std::fs::write(&source, "let mut total: int = 1\ntotal = total + 4\n").expect("write source");
    let mut session = DapSession::default();

    session
        .message_response(&serde_json::json!({
            "seq": 155,
            "type": "request",
            "command": "launch",
            "arguments": {
                "program": format!("file://{}", source.display()),
            },
        }))
        .expect("launch response");
    session
        .message_response(&serde_json::json!({
            "seq": 156,
            "type": "request",
            "command": "next",
            "arguments": {
                "threadId": 1,
            },
        }))
        .expect("next response");
    let locals = session
        .message_response(&serde_json::json!({
            "seq": 157,
            "type": "request",
            "command": "variables",
            "arguments": {
                "variablesReference": 2,
            },
        }))
        .expect("locals response");

    let vars = locals["body"]["variables"].as_array().expect("locals");
    assert!(vars
        .iter()
        .any(|var| { var["name"] == "total" && var["value"] == "5" && var["type"] == "int" }));
    let _ = std::fs::remove_dir_all(dir);
}
