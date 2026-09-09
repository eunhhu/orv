use super::*;

#[test]
fn dap_initialize_returns_debug_capabilities() {
    let response = dap_protocol_response(&serde_json::json!({
        "seq": 1,
        "type": "request",
        "command": "initialize",
        "arguments": {},
    }));

    assert_eq!(response["type"], "response");
    assert_eq!(response["request_seq"], 1);
    assert_eq!(response["command"], "initialize");
    assert_eq!(response["success"], true);
    assert_eq!(response["body"]["supportsConfigurationDoneRequest"], true);
    assert_eq!(response["body"]["supportsTerminateRequest"], true);
    assert_eq!(response["body"]["supportsTerminateThreadsRequest"], true);
    assert_eq!(response["body"]["supportsLoadedSourcesRequest"], true);
    assert_eq!(response["body"]["supportsEvaluateForHovers"], true);
    assert_eq!(response["body"]["supportsCompletionsRequest"], true);
    assert_eq!(response["body"]["supportsBreakpointLocationsRequest"], true);
    assert_eq!(response["body"]["supportsConditionalBreakpoints"], true);
    assert_eq!(response["body"]["supportsHitConditionalBreakpoints"], true);
    assert_eq!(response["body"]["supportsFunctionBreakpoints"], true);
    assert_eq!(response["body"]["supportsDataBreakpoints"], true);
    assert_eq!(response["body"]["supportsExceptionInfoRequest"], true);
    assert_eq!(response["body"]["supportsRestartRequest"], true);
    assert_eq!(response["body"]["supportsSetVariable"], true);
    assert_eq!(response["body"]["supportsSetExpression"], true);
    assert_eq!(response["body"]["supportsModulesRequest"], true);
    assert_eq!(response["body"]["supportsGotoTargetsRequest"], true);
    assert_eq!(response["body"]["supportsStepBack"], true);
    assert_eq!(response["body"]["supportsStepInTargetsRequest"], true);
    assert_eq!(response["body"]["supportsRestartFrame"], true);
    assert_eq!(response["body"]["supportsPauseRequest"], true);
    assert_eq!(response["body"]["supportsCancelRequest"], true);
    assert_eq!(response["body"]["supportsInstructionBreakpoints"], true);
    assert_eq!(response["body"]["supportsDisassembleRequest"], true);
    assert_eq!(response["body"]["supportsReadMemoryRequest"], true);
    assert_eq!(response["body"]["supportsOrvRuntimeAttach"], true);
    assert_eq!(response["body"]["supportsOrvRuntimeTracePath"], true);
    assert_eq!(response["body"]["supportsOrvSourceBundleLaunch"], true);
}

#[test]
fn dap_cancel_request_is_accepted() {
    let response = dap_protocol_response(&serde_json::json!({
        "seq": 66,
        "type": "request",
        "command": "cancel",
        "arguments": {
            "requestId": 1,
            "progressId": "orv-progress",
        },
    }));

    assert_eq!(response["type"], "response");
    assert_eq!(response["request_seq"], 66);
    assert_eq!(response["command"], "cancel");
    assert_eq!(response["success"], true);
}

#[test]
fn dap_disassemble_returns_source_frame_pseudo_instructions() {
    let dir = temp_output_dir("dap-disassemble");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let source = dir.join("app.orv");
    let source_text = "let first: int = 1\nlet second: int = 2\n";
    std::fs::write(&source, source_text).expect("write source");
    let canonical_source = std::fs::canonicalize(&source).expect("canonical source");
    let mut session = DapSession::default();

    session
        .message_response(&serde_json::json!({
            "seq": 78,
            "type": "request",
            "command": "launch",
            "arguments": {
                "program": format!("file://{}", source.display()),
            },
        }))
        .expect("launch response");
    let response = session
        .message_response(&serde_json::json!({
            "seq": 79,
            "type": "request",
            "command": "disassemble",
            "arguments": {
                "memoryReference": "orv:frame:1",
                "instructionOffset": 0,
                "instructionCount": 2,
            },
        }))
        .expect("disassemble response");

    assert_eq!(response["type"], "response");
    assert_eq!(response["request_seq"], 79);
    assert_eq!(response["command"], "disassemble");
    assert_eq!(response["success"], true, "{response}");
    let instructions = response["body"]["instructions"]
        .as_array()
        .expect("instructions");
    assert_eq!(instructions.len(), 2);
    assert_eq!(instructions[0]["address"], "orv:frame:1");
    assert_eq!(instructions[0]["instruction"], "orv entry line 1");
    assert_eq!(
        instructions[0]["location"]["path"],
        canonical_source.display().to_string()
    );
    assert_eq!(
        instructions[0]["location"]["checksums"][0]["algorithm"],
        serde_json::json!("SHA256")
    );
    assert_eq!(
        instructions[0]["location"]["checksums"][0]["checksum"],
        serde_json::json!(sha256_hex(source_text.as_bytes()))
    );
    assert_eq!(instructions[0]["line"], 1);
    assert_eq!(instructions[1]["address"], "orv:frame:2");
    assert_eq!(instructions[1]["instruction"], "orv entry line 2");
    assert_eq!(instructions[1]["line"], 2);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn dap_read_memory_returns_base64_source_frame_bytes() {
    let dir = temp_output_dir("dap-read-memory");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let source = dir.join("app.orv");
    std::fs::write(&source, "let first: int = 1\nlet second: int = 2\n").expect("write source");
    let mut session = DapSession::default();

    session
        .message_response(&serde_json::json!({
            "seq": 80,
            "type": "request",
            "command": "launch",
            "arguments": {
                "program": format!("file://{}", source.display()),
            },
        }))
        .expect("launch response");
    let response = session
        .message_response(&serde_json::json!({
            "seq": 81,
            "type": "request",
            "command": "readMemory",
            "arguments": {
                "memoryReference": "orv:frame:1",
                "offset": 4,
                "count": 5,
            },
        }))
        .expect("readMemory response");

    assert_eq!(response["type"], "response");
    assert_eq!(response["request_seq"], 81);
    assert_eq!(response["command"], "readMemory");
    assert_eq!(response["success"], true, "{response}");
    assert_eq!(response["body"]["address"], "orv:frame:1");
    assert_eq!(response["body"]["data"], "Zmlyc3Q=");
    assert_eq!(response["body"]["unreadableBytes"], 0);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn dap_continue_terminates_session_state() {
    let dir = temp_output_dir("dap-continue-terminates-state");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let source = dir.join("app.orv");
    std::fs::write(&source, "let answer: int = 42\n").expect("write source");
    let mut session = DapSession::default();

    session
        .message_response(&serde_json::json!({
            "seq": 71,
            "type": "request",
            "command": "launch",
            "arguments": {
                "program": format!("file://{}", source.display()),
            },
        }))
        .expect("launch response");
    let continue_response = session
        .message_response(&serde_json::json!({
            "seq": 72,
            "type": "request",
            "command": "continue",
            "arguments": {
                "threadId": 1,
            },
        }))
        .expect("continue response");
    let stack = session
        .message_response(&serde_json::json!({
            "seq": 73,
            "type": "request",
            "command": "stackTrace",
            "arguments": {
                "threadId": 1,
            },
        }))
        .expect("stack response");

    assert_eq!(continue_response["success"], true, "{continue_response}");
    assert_eq!(stack["success"], false, "{stack}");
    assert!(stack["message"]
        .as_str()
        .is_some_and(|message| message.contains("launch is required")));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn dap_debug_control_rejects_unknown_thread_id() {
    let dir = temp_output_dir("dap-debug-control-thread");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let source = dir.join("app.orv");
    std::fs::write(&source, "let first: int = 1\nlet second: int = 2\n").expect("write source");

    for command in ["continue", "next", "stepIn", "pause"] {
        let mut session = DapSession::default();
        session
            .message_response(&serde_json::json!({
                "seq": 57,
                "type": "request",
                "command": "launch",
                "arguments": {
                    "program": format!("file://{}", source.display()),
                },
            }))
            .expect("launch response");
        let response = session
            .message_response(&serde_json::json!({
                "seq": 58,
                "type": "request",
                "command": command,
                "arguments": {
                    "threadId": 99,
                },
            }))
            .expect("debug control response");

        assert_eq!(response["success"], false, "{command}: {response}");
        assert!(response["message"]
            .as_str()
            .is_some_and(|message| { message.contains("unknown ORV thread id 99") }));
    }
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn dap_logpoint_outputs_without_stopping() {
    let dir = temp_output_dir("dap-logpoint");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let source = dir.join("app.orv");
    std::fs::write(
        &source,
        "let first: int = 1\nlet second: int = 2\nlet third: int = 3\n",
    )
    .expect("write source");
    let mut session = DapSession::default();

    session
        .message_response(&serde_json::json!({
            "seq": 164,
            "type": "request",
            "command": "setBreakpoints",
            "arguments": {
                "source": {
                    "path": source.display().to_string(),
                },
                "breakpoints": [
                    {
                        "line": 2,
                        "logMessage": "middle reached",
                    },
                ],
            },
        }))
        .expect("breakpoints response");
    session
        .message_response(&serde_json::json!({
            "seq": 165,
            "type": "request",
            "command": "launch",
            "arguments": {
                "program": format!("file://{}", source.display()),
            },
        }))
        .expect("launch response");
    let first_stack = session
        .message_response(&serde_json::json!({
            "seq": 166,
            "type": "request",
            "command": "stackTrace",
            "arguments": {
                "threadId": 1,
            },
        }))
        .expect("first stack response");
    session.drain_pending_events();
    let continue_response = session
        .message_response(&serde_json::json!({
            "seq": 167,
            "type": "request",
            "command": "continue",
            "arguments": {
                "threadId": 1,
            },
        }))
        .expect("continue response");
    let events = session.drain_pending_events();

    assert_eq!(first_stack["body"]["stackFrames"][0]["line"], 1);
    assert_eq!(continue_response["success"], true, "{continue_response}");
    assert!(events.iter().any(|event| {
        event["type"] == "event"
            && event["event"] == "output"
            && event["body"]["category"] == "console"
            && event["body"]["output"] == "middle reached\n"
    }));
    assert!(!events.iter().any(|event| {
        event["type"] == "event"
            && event["event"] == "stopped"
            && event["body"]["reason"] == "breakpoint"
    }));
    assert!(events
        .iter()
        .any(|event| event["type"] == "event" && event["event"] == "terminated"));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn dap_hit_condition_modulo_stays_msrv_compatible() {
    assert!(dap_hit_condition_matches("%=2", 2));
    assert!(dap_hit_condition_matches("%2", 4));
    assert!(!dap_hit_condition_matches("%=2", 3));
    assert!(!dap_hit_condition_matches("%=0", 4));
}

#[test]
fn dap_stack_trace_honors_start_frame_and_levels() {
    let dir = temp_output_dir("dap-stack-trace-paging");
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
            "seq": 204,
            "type": "request",
            "command": "setFunctionBreakpoints",
            "arguments": {
                "breakpoints": [
                    { "name": "add" },
                ],
            },
        }))
        .expect("setFunctionBreakpoints response");
    session
        .message_response(&serde_json::json!({
            "seq": 205,
            "type": "request",
            "command": "launch",
            "arguments": {
                "program": format!("file://{}", source.display()),
            },
        }))
        .expect("launch response");
    let stack = session
        .message_response(&serde_json::json!({
            "seq": 206,
            "type": "request",
            "command": "stackTrace",
            "arguments": {
                "threadId": 1,
                "startFrame": 1,
                "levels": 1,
            },
        }))
        .expect("stack response");

    assert_eq!(stack["success"], true, "{stack}");
    assert_eq!(stack["body"]["totalFrames"], 2);
    let frames = stack["body"]["stackFrames"]
        .as_array()
        .expect("stack frames");
    assert_eq!(frames.len(), 1);
    assert_eq!(frames[0]["name"], "orv entry");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn dap_next_advances_to_next_executable_line_and_queues_stopped_event() {
    let dir = temp_output_dir("dap-next-line");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let source = dir.join("app.orv");
    std::fs::write(&source, "let first: int = 1\n\nlet second: int = 2\n").expect("write source");
    let mut session = DapSession::default();

    session
        .message_response(&serde_json::json!({
            "seq": 48,
            "type": "request",
            "command": "launch",
            "arguments": {
                "program": format!("file://{}", source.display()),
            },
        }))
        .expect("launch response");
    let first_stack = session
        .message_response(&serde_json::json!({
            "seq": 49,
            "type": "request",
            "command": "stackTrace",
            "arguments": {
                "threadId": 1,
            },
        }))
        .expect("first stack response");
    let next = session
        .message_response(&serde_json::json!({
            "seq": 50,
            "type": "request",
            "command": "next",
            "arguments": {
                "threadId": 1,
            },
        }))
        .expect("next response");
    let events = session.drain_pending_events();
    let second_stack = session
        .message_response(&serde_json::json!({
            "seq": 51,
            "type": "request",
            "command": "stackTrace",
            "arguments": {
                "threadId": 1,
            },
        }))
        .expect("second stack response");

    assert_eq!(first_stack["body"]["stackFrames"][0]["line"], 1);
    assert_eq!(next["success"], true, "{next}");
    assert_eq!(next["body"], serde_json::json!({}));
    assert_eq!(second_stack["body"]["stackFrames"][0]["line"], 3);
    assert!(events.iter().any(|event| {
        event["type"] == "event"
            && event["event"] == "stopped"
            && event["body"]["reason"] == "step"
            && event["body"]["threadId"] == 1
    }));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn dap_long_running_continue_and_pause_queue_events() {
    let dir = temp_output_dir("dap-server-long-running-pause");
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
    let mut session = DapSession::default();

    session
        .message_response(&serde_json::json!({
            "seq": 223,
            "type": "request",
            "command": "launch",
            "arguments": {
                "program": format!("file://{}", source.display()),
            },
        }))
        .expect("launch response");
    let _ = session.drain_pending_events();
    let continue_response = session
        .message_response(&serde_json::json!({
            "seq": 224,
            "type": "request",
            "command": "continue",
            "arguments": {
                "threadId": 1,
            },
        }))
        .expect("continue response");
    let continue_events = session.drain_pending_events();
    let pause = session
        .message_response(&serde_json::json!({
            "seq": 225,
            "type": "request",
            "command": "pause",
            "arguments": {
                "threadId": 1,
            },
        }))
        .expect("pause response");
    let pause_events = session.drain_pending_events();

    assert_eq!(continue_response["success"], true, "{continue_response}");
    assert!(continue_events.iter().any(|event| {
        event["type"] == "event" && event["event"] == "continued" && event["body"]["threadId"] == 1
    }));
    assert_eq!(pause["success"], true, "{pause}");
    assert!(pause_events.iter().any(|event| {
        event["type"] == "event"
            && event["event"] == "stopped"
            && event["body"]["reason"] == "pause"
            && event["body"]["threadId"] == 1
    }));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn dap_long_running_exposes_env_listen_endpoint() {
    let dir = temp_output_dir("dap-server-env-listen");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let source = dir.join("app.orv");
    std::fs::write(
        &source,
        r#"@server {
  @listen int.from(@env.PORT ?? "8080")
  @route GET /ping { @respond 200 { ok: true } }
}
"#,
    )
    .expect("write source");
    let mut session = DapSession::default();

    let launch = session
        .message_response(&serde_json::json!({
            "seq": 240,
            "type": "request",
            "command": "launch",
            "arguments": {
                "program": format!("file://{}", source.display()),
            },
        }))
        .expect("launch response");
    let listen = session
        .message_response(&serde_json::json!({
            "seq": 241,
            "type": "request",
            "command": "evaluate",
            "arguments": {
                "expression": "runtimeListen",
            },
        }))
        .expect("listen evaluate response");

    assert_eq!(launch["body"]["runtime"]["async"]["listen"]["kind"], "env");
    assert_eq!(
        launch["body"]["runtime"]["async"]["listen"]["variable"],
        "PORT"
    );
    assert_eq!(
        launch["body"]["runtime"]["async"]["listen"]["default_port"],
        8080
    );
    assert_eq!(listen["success"], true, "{listen}");
    assert_eq!(listen["body"]["result"], "PORT default 8080");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn dap_step_out_leaves_current_function_frame() {
    let dir = temp_output_dir("dap-step-out");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let source = dir.join("app.orv");
    std::fs::write(
        &source,
        r"function add(a: int, b: int): int -> {
  let result: int = a + b
  result
}
let total: int = add(2, 3)
let done: int = total
",
    )
    .expect("write source");
    let mut session = DapSession::default();

    session
        .message_response(&serde_json::json!({
            "seq": 190,
            "type": "request",
            "command": "launch",
            "arguments": {
                "program": format!("file://{}", source.display()),
            },
        }))
        .expect("launch response");
    session
        .message_response(&serde_json::json!({
            "seq": 191,
            "type": "request",
            "command": "stepIn",
            "arguments": {
                "threadId": 1,
            },
        }))
        .expect("stepIn response");
    let inside_stack = session
        .message_response(&serde_json::json!({
            "seq": 192,
            "type": "request",
            "command": "stackTrace",
            "arguments": {
                "threadId": 1,
            },
        }))
        .expect("inside stack response");
    let step_out = session
        .message_response(&serde_json::json!({
            "seq": 193,
            "type": "request",
            "command": "stepOut",
            "arguments": {
                "threadId": 1,
            },
        }))
        .expect("stepOut response");
    let events = session.drain_pending_events();
    let outside_stack = session
        .message_response(&serde_json::json!({
            "seq": 194,
            "type": "request",
            "command": "stackTrace",
            "arguments": {
                "threadId": 1,
            },
        }))
        .expect("outside stack response");

    assert_eq!(inside_stack["body"]["stackFrames"][0]["name"], "add");
    assert_eq!(inside_stack["body"]["stackFrames"][0]["line"], 2);
    assert_eq!(step_out["success"], true, "{step_out}");
    assert_eq!(outside_stack["body"]["stackFrames"][0]["name"], "orv entry");
    assert_eq!(outside_stack["body"]["stackFrames"][0]["line"], 5);
    assert!(events.iter().any(|event| {
        event["type"] == "event" && event["event"] == "stopped" && event["body"]["reason"] == "step"
    }));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn dap_next_steps_over_function_call_frames() {
    let dir = temp_output_dir("dap-next-step-over");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let source = dir.join("app.orv");
    std::fs::write(
        &source,
        r"function add(a: int, b: int): int -> {
  let result: int = a + b
  result
}
let total: int = add(2, 3)
let done: int = total
",
    )
    .expect("write source");
    let mut session = DapSession::default();

    session
        .message_response(&serde_json::json!({
            "seq": 195,
            "type": "request",
            "command": "launch",
            "arguments": {
                "program": format!("file://{}", source.display()),
            },
        }))
        .expect("launch response");
    let next = session
        .message_response(&serde_json::json!({
            "seq": 196,
            "type": "request",
            "command": "next",
            "arguments": {
                "threadId": 1,
            },
        }))
        .expect("next response");
    let stack = session
        .message_response(&serde_json::json!({
            "seq": 197,
            "type": "request",
            "command": "stackTrace",
            "arguments": {
                "threadId": 1,
            },
        }))
        .expect("stack response");

    assert_eq!(next["success"], true, "{next}");
    assert_eq!(stack["body"]["stackFrames"][0]["name"], "orv entry");
    assert_eq!(stack["body"]["stackFrames"][0]["line"], 5);
    assert_eq!(stack["body"]["totalFrames"], 1);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn dap_step_in_targets_enter_selected_function_frame() {
    let dir = temp_output_dir("dap-step-in-targets");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let source = dir.join("app.orv");
    let source_text = r"function add(a: int, b: int): int -> {
  let result: int = a + b
  result
}
let total: int = add(2, 3)
";
    std::fs::write(&source, source_text).expect("write source");
    let mut session = DapSession::default();

    session
        .message_response(&serde_json::json!({
            "seq": 198,
            "type": "request",
            "command": "launch",
            "arguments": {
                "program": format!("file://{}", source.display()),
            },
        }))
        .expect("launch response");
    let targets = session
        .message_response(&serde_json::json!({
            "seq": 199,
            "type": "request",
            "command": "stepInTargets",
            "arguments": {
                "frameId": 1,
            },
        }))
        .expect("stepInTargets response");
    let add_target = targets["body"]["targets"]
        .as_array()
        .expect("targets")
        .iter()
        .find(|target| target["label"] == "add")
        .expect("add target");
    let target_id = add_target["id"].as_u64().expect("add target id");
    let step_in = session
        .message_response(&serde_json::json!({
            "seq": 200,
            "type": "request",
            "command": "stepIn",
            "arguments": {
                "threadId": 1,
                "targetId": target_id,
            },
        }))
        .expect("stepIn response");
    let events = session.drain_pending_events();
    let stack = session
        .message_response(&serde_json::json!({
            "seq": 201,
            "type": "request",
            "command": "stackTrace",
            "arguments": {
                "threadId": 1,
            },
        }))
        .expect("stack response");
    let caller_frame_id = stack["body"]["stackFrames"]
        .as_array()
        .expect("stack frames")
        .get(1)
        .and_then(|frame| frame["id"].as_u64())
        .expect("caller frame id");
    let caller_scopes = session
        .message_response(&serde_json::json!({
            "seq": 202,
            "type": "request",
            "command": "scopes",
            "arguments": {
                "frameId": caller_frame_id,
            },
        }))
        .expect("caller scopes response");
    let caller_targets = session
        .message_response(&serde_json::json!({
            "seq": 203,
            "type": "request",
            "command": "stepInTargets",
            "arguments": {
                "frameId": caller_frame_id,
            },
        }))
        .expect("caller stepInTargets response");

    assert_eq!(targets["success"], true, "{targets}");
    assert_eq!(
        add_target["source"]["checksums"][0]["checksum"],
        serde_json::json!(sha256_hex(source_text.as_bytes()))
    );
    assert_eq!(step_in["success"], true, "{step_in}");
    assert_eq!(stack["body"]["stackFrames"][0]["name"], "add");
    assert_eq!(stack["body"]["stackFrames"][0]["line"], 2);
    assert_eq!(
        stack["body"]["stackFrames"][0]["source"]["checksums"][0]["checksum"],
        serde_json::json!(sha256_hex(source_text.as_bytes()))
    );
    assert_eq!(caller_scopes["success"], true, "{caller_scopes}");
    assert_eq!(
        caller_scopes["body"]["scopes"][0]["variablesReference"],
        serde_json::json!(0)
    );
    assert_eq!(
        caller_scopes["body"]["scopes"][0]["source"]["checksums"][0]["checksum"],
        serde_json::json!(sha256_hex(source_text.as_bytes()))
    );
    assert_eq!(caller_targets["success"], true, "{caller_targets}");
    assert_eq!(caller_targets["body"]["targets"], serde_json::json!([]));
    assert!(events.iter().any(|event| {
        event["type"] == "event" && event["event"] == "stopped" && event["body"]["reason"] == "step"
    }));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn dap_restart_frame_rewinds_current_function_frame() {
    let dir = temp_output_dir("dap-restart-frame");
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
            "seq": 202,
            "type": "request",
            "command": "launch",
            "arguments": {
                "program": format!("file://{}", source.display()),
            },
        }))
        .expect("launch response");
    session
        .message_response(&serde_json::json!({
            "seq": 203,
            "type": "request",
            "command": "stepIn",
            "arguments": {
                "threadId": 1,
            },
        }))
        .expect("first stepIn response");
    session
        .message_response(&serde_json::json!({
            "seq": 204,
            "type": "request",
            "command": "stepIn",
            "arguments": {
                "threadId": 1,
            },
        }))
        .expect("second stepIn response");
    let before = session
        .message_response(&serde_json::json!({
            "seq": 205,
            "type": "request",
            "command": "stackTrace",
            "arguments": {
                "threadId": 1,
            },
        }))
        .expect("before stack response");
    let restart_frame = session
        .message_response(&serde_json::json!({
            "seq": 206,
            "type": "request",
            "command": "restartFrame",
            "arguments": {
                "frameId": 1,
            },
        }))
        .expect("restartFrame response");
    let events = session.drain_pending_events();
    let after = session
        .message_response(&serde_json::json!({
            "seq": 207,
            "type": "request",
            "command": "stackTrace",
            "arguments": {
                "threadId": 1,
            },
        }))
        .expect("after stack response");

    assert_eq!(before["body"]["stackFrames"][0]["name"], "add");
    assert_eq!(before["body"]["stackFrames"][0]["line"], 3);
    assert_eq!(restart_frame["success"], true, "{restart_frame}");
    assert_eq!(after["body"]["stackFrames"][0]["name"], "add");
    assert_eq!(after["body"]["stackFrames"][0]["line"], 2);
    assert!(events.iter().any(|event| {
        event["type"] == "event"
            && event["event"] == "stopped"
            && event["body"]["reason"] == "restart"
    }));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn dap_restart_frame_accepts_reported_entry_frame_id() {
    let dir = temp_output_dir("dap-restart-entry-frame");
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
            "seq": 216,
            "type": "request",
            "command": "launch",
            "arguments": {
                "program": format!("file://{}", source.display()),
            },
        }))
        .expect("launch response");
    session
        .message_response(&serde_json::json!({
            "seq": 217,
            "type": "request",
            "command": "stepIn",
            "arguments": {
                "threadId": 1,
            },
        }))
        .expect("first stepIn response");
    session
        .message_response(&serde_json::json!({
            "seq": 218,
            "type": "request",
            "command": "stepIn",
            "arguments": {
                "threadId": 1,
            },
        }))
        .expect("second stepIn response");
    let before = session
        .message_response(&serde_json::json!({
            "seq": 219,
            "type": "request",
            "command": "stackTrace",
            "arguments": {
                "threadId": 1,
            },
        }))
        .expect("before stack response");
    let entry_frame_id = before["body"]["stackFrames"]
        .as_array()
        .expect("stack frames")
        .iter()
        .find(|frame| frame["name"] == "orv entry")
        .and_then(|frame| frame["id"].as_u64())
        .expect("entry frame id");
    let restart_frame = session
        .message_response(&serde_json::json!({
            "seq": 220,
            "type": "request",
            "command": "restartFrame",
            "arguments": {
                "frameId": entry_frame_id,
            },
        }))
        .expect("restartFrame response");
    let after = session
        .message_response(&serde_json::json!({
            "seq": 221,
            "type": "request",
            "command": "stackTrace",
            "arguments": {
                "threadId": 1,
            },
        }))
        .expect("after stack response");

    assert_eq!(restart_frame["success"], true, "{restart_frame}");
    assert_eq!(after["body"]["stackFrames"][0]["name"], "orv entry");
    assert_eq!(after["body"]["stackFrames"][0]["line"], 1);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn dap_next_after_last_executable_line_terminates_session() {
    let dir = temp_output_dir("dap-next-terminate");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let source = dir.join("app.orv");
    std::fs::write(&source, "let only: int = 1\n").expect("write source");
    let mut session = DapSession::default();

    session
        .message_response(&serde_json::json!({
            "seq": 68,
            "type": "request",
            "command": "launch",
            "arguments": {
                "program": format!("file://{}", source.display()),
            },
        }))
        .expect("launch response");
    let next = session
        .message_response(&serde_json::json!({
            "seq": 69,
            "type": "request",
            "command": "next",
            "arguments": {
                "threadId": 1,
            },
        }))
        .expect("next response");
    let events = session.drain_pending_events();
    let stack = session
        .message_response(&serde_json::json!({
            "seq": 70,
            "type": "request",
            "command": "stackTrace",
            "arguments": {
                "threadId": 1,
            },
        }))
        .expect("stack response");

    assert_eq!(next["success"], true, "{next}");
    assert!(events
        .iter()
        .any(|event| { event["type"] == "event" && event["event"] == "terminated" }));
    assert_eq!(stack["success"], false, "{stack}");
    assert!(stack["message"]
        .as_str()
        .is_some_and(|message| message.contains("launch is required")));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn dap_pause_keeps_current_line_and_queues_pause_stopped_event() {
    let dir = temp_output_dir("dap-pause-event");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let source = dir.join("app.orv");
    std::fs::write(&source, "let answer: int = 42\n").expect("write source");
    let mut session = DapSession::default();

    session
        .message_response(&serde_json::json!({
            "seq": 52,
            "type": "request",
            "command": "launch",
            "arguments": {
                "program": format!("file://{}", source.display()),
            },
        }))
        .expect("launch response");
    let pause = session
        .message_response(&serde_json::json!({
            "seq": 53,
            "type": "request",
            "command": "pause",
            "arguments": {
                "threadId": 1,
            },
        }))
        .expect("pause response");
    let events = session.drain_pending_events();
    let stack = session
        .message_response(&serde_json::json!({
            "seq": 54,
            "type": "request",
            "command": "stackTrace",
            "arguments": {
                "threadId": 1,
            },
        }))
        .expect("stack response");

    assert_eq!(pause["success"], true, "{pause}");
    assert_eq!(stack["body"]["stackFrames"][0]["line"], 1);
    assert!(events.iter().any(|event| {
        event["type"] == "event"
            && event["event"] == "stopped"
            && event["body"]["reason"] == "pause"
            && event["body"]["threadId"] == 1
    }));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn dap_restart_reloads_current_program_and_resets_stopped_line() {
    let dir = temp_output_dir("dap-restart");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let source = dir.join("app.orv");
    std::fs::write(&source, "let first: int = 1\nlet second: int = 2\n").expect("write source");
    let mut session = DapSession::default();

    session
        .message_response(&serde_json::json!({
            "seq": 78,
            "type": "request",
            "command": "launch",
            "arguments": {
                "program": format!("file://{}", source.display()),
            },
        }))
        .expect("launch response");
    session
        .message_response(&serde_json::json!({
            "seq": 79,
            "type": "request",
            "command": "next",
            "arguments": {
                "threadId": 1,
            },
        }))
        .expect("next response");
    let moved_stack = session
        .message_response(&serde_json::json!({
            "seq": 80,
            "type": "request",
            "command": "stackTrace",
            "arguments": {
                "threadId": 1,
            },
        }))
        .expect("moved stack response");
    let restart = session
        .message_response(&serde_json::json!({
            "seq": 81,
            "type": "request",
            "command": "restart",
            "arguments": {},
        }))
        .expect("restart response");
    let restarted_stack = session
        .message_response(&serde_json::json!({
            "seq": 82,
            "type": "request",
            "command": "stackTrace",
            "arguments": {
                "threadId": 1,
            },
        }))
        .expect("restarted stack response");

    assert_eq!(moved_stack["body"]["stackFrames"][0]["line"], 2);
    assert_eq!(restart["success"], true, "{restart}");
    assert_eq!(restarted_stack["body"]["stackFrames"][0]["line"], 1);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn dap_goto_targets_and_goto_move_to_executable_frame() {
    let dir = temp_output_dir("dap-goto");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let source = dir.join("app.orv");
    std::fs::write(&source, "let first: int = 1\n\nlet third: int = 3\n").expect("write source");
    let mut session = DapSession::default();

    session
        .message_response(&serde_json::json!({
            "seq": 177,
            "type": "request",
            "command": "launch",
            "arguments": {
                "program": format!("file://{}", source.display()),
            },
        }))
        .expect("launch response");
    let targets = session
        .message_response(&serde_json::json!({
            "seq": 178,
            "type": "request",
            "command": "gotoTargets",
            "arguments": {
                "source": {
                    "path": source.display().to_string(),
                },
                "line": 1,
                "endLine": 3,
            },
        }))
        .expect("gotoTargets response");
    assert_eq!(targets["success"], true, "{targets}");
    let target_id = targets["body"]["targets"]
        .as_array()
        .expect("targets")
        .iter()
        .find(|target| target["line"] == 3)
        .and_then(|target| target["id"].as_u64())
        .expect("line 3 target");
    let goto = session
        .message_response(&serde_json::json!({
            "seq": 179,
            "type": "request",
            "command": "goto",
            "arguments": {
                "threadId": 1,
                "targetId": target_id,
            },
        }))
        .expect("goto response");
    let events = session.drain_pending_events();
    let stack = session
        .message_response(&serde_json::json!({
            "seq": 180,
            "type": "request",
            "command": "stackTrace",
            "arguments": {
                "threadId": 1,
            },
        }))
        .expect("stack response");

    let target_lines = targets["body"]["targets"]
        .as_array()
        .expect("targets")
        .iter()
        .map(|target| target["line"].as_u64().expect("line"))
        .collect::<Vec<_>>();
    assert_eq!(target_lines, vec![1, 3]);
    assert_eq!(goto["success"], true, "{goto}");
    assert_eq!(stack["body"]["stackFrames"][0]["line"], 3);
    assert!(events.iter().any(|event| {
        event["type"] == "event" && event["event"] == "stopped" && event["body"]["reason"] == "goto"
    }));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn dap_source_returns_content_by_loaded_source_reference() {
    let dir = temp_output_dir("dap-source-reference");
    let models = dir.join("models");
    std::fs::create_dir_all(&models).expect("create models dir");
    let source = dir.join("app.orv");
    let imported = models.join("user.orv");
    let imported_source = "pub struct User { id: int }\n";
    std::fs::write(
        &source,
        "import models.user.User\nlet u: User = { id: 1 }\n",
    )
    .expect("write source");
    std::fs::write(&imported, imported_source).expect("write imported");
    let mut session = DapSession::default();

    let launch = session
        .message_response(&serde_json::json!({
            "seq": 34,
            "type": "request",
            "command": "launch",
            "arguments": {
                "program": format!("file://{}", source.display()),
            },
        }))
        .expect("launch response");
    let loaded = session
        .message_response(&serde_json::json!({
            "seq": 35,
            "type": "request",
            "command": "loadedSources",
            "arguments": {},
        }))
        .expect("loadedSources response");
    let user_reference = loaded["body"]["sources"]
        .as_array()
        .expect("sources")
        .iter()
        .find(|item| item["name"] == "user.orv")
        .and_then(|item| item["sourceReference"].as_u64())
        .expect("user source reference");
    std::fs::remove_file(&imported).expect("remove imported after launch");
    let source_response = session
        .message_response(&serde_json::json!({
            "seq": 36,
            "type": "request",
            "command": "source",
            "arguments": {
                "sourceReference": user_reference,
            },
        }))
        .expect("source response");

    assert_eq!(launch["success"], true, "{launch}");
    assert!(user_reference > 0);
    assert_eq!(source_response["success"], true, "{source_response}");
    assert_eq!(source_response["body"]["content"], imported_source);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn dap_set_expression_updates_current_local() {
    let dir = temp_output_dir("dap-set-expression");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let source = dir.join("app.orv");
    std::fs::write(&source, "let name = \"Ada\"\n").expect("write source");
    let mut session = DapSession::default();

    session
        .message_response(&serde_json::json!({
            "seq": 172,
            "type": "request",
            "command": "launch",
            "arguments": {
                "program": format!("file://{}", source.display()),
            },
        }))
        .expect("launch response");
    let set_expression = session
        .message_response(&serde_json::json!({
            "seq": 173,
            "type": "request",
            "command": "setExpression",
            "arguments": {
                "expression": "name",
                "value": "\"Grace\"",
                "frameId": 1,
            },
        }))
        .expect("setExpression response");
    let evaluate = session
        .message_response(&serde_json::json!({
            "seq": 174,
            "type": "request",
            "command": "evaluate",
            "arguments": {
                "expression": "name",
                "context": "repl",
            },
        }))
        .expect("evaluate response");

    assert_eq!(set_expression["success"], true, "{set_expression}");
    assert_eq!(set_expression["body"]["value"], "\"Grace\"");
    assert_eq!(set_expression["body"]["type"], "string");
    assert_eq!(evaluate["body"]["result"], "\"Grace\"");
    assert_eq!(evaluate["body"]["type"], "string");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn dap_locals_follow_current_stopped_line() {
    let dir = temp_output_dir("dap-line-locals");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let source = dir.join("app.orv");
    std::fs::write(&source, "let first: int = 1\nlet second: int = 2\n").expect("write source");
    let mut session = DapSession::default();

    session
        .message_response(&serde_json::json!({
            "seq": 57,
            "type": "request",
            "command": "launch",
            "arguments": {
                "program": format!("file://{}", source.display()),
            },
        }))
        .expect("launch response");
    let scopes = session
        .message_response(&serde_json::json!({
            "seq": 58,
            "type": "request",
            "command": "scopes",
            "arguments": {
                "frameId": 1,
            },
        }))
        .expect("scopes response");
    let locals_ref = scopes["body"]["scopes"]
        .as_array()
        .expect("scopes")
        .iter()
        .find(|scope| scope["name"] == "Locals")
        .and_then(|scope| scope["variablesReference"].as_u64())
        .expect("locals scope");
    let first_locals = session
        .message_response(&serde_json::json!({
            "seq": 59,
            "type": "request",
            "command": "variables",
            "arguments": {
                "variablesReference": locals_ref,
            },
        }))
        .expect("first locals response");
    session
        .message_response(&serde_json::json!({
            "seq": 60,
            "type": "request",
            "command": "next",
            "arguments": {
                "threadId": 1,
            },
        }))
        .expect("next response");
    let second_locals = session
        .message_response(&serde_json::json!({
            "seq": 61,
            "type": "request",
            "command": "variables",
            "arguments": {
                "variablesReference": locals_ref,
            },
        }))
        .expect("second locals response");

    let first_vars = first_locals["body"]["variables"]
        .as_array()
        .expect("first locals");
    assert!(first_vars.iter().any(|var| var["name"] == "first"));
    assert!(!first_vars.iter().any(|var| var["name"] == "second"));
    let second_vars = second_locals["body"]["variables"]
        .as_array()
        .expect("second locals");
    assert!(second_vars.iter().any(|var| var["name"] == "first"));
    assert!(second_vars.iter().any(|var| var["name"] == "second"));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn build_prod_smoke_dap_native_route_summary_uses_actual_route_count() {
    let (src_dir, path) = multi_route_prod_server_source("deploy-smoke-dap-route-count-source");
    let out = temp_output_dir("deploy-smoke-dap-route-count");

    cmd_build_with_profile(&path, &out, BuildProfile::Production).expect("prod build");
    let smoke =
        std::fs::read_to_string(out.join("deploy").join("smoke-test.sh")).expect("smoke test");
    let native_summary = deploy_native_server_summary_counts(&out).expect("native summary counts");

    assert!(smoke.contains(&format!(
        r#"orv_smoke_dap_summary_contains "dap native target summary" '"native_server_target_count": {}'"#,
        native_summary.targets
    )));
    assert!(smoke.contains(&format!(
        r#"orv_smoke_dap_summary_contains "dap native route summary" '"native_server_route_count": {}'"#,
        native_summary.routes
    )));
    assert!(!smoke.contains(
        r#"orv_smoke_dap_summary_contains "dap native route summary" '"native_server_route_count": 1'"#
    ));
    cmd_verify_build(&out).expect("verify prod build");
    let _ = std::fs::remove_dir_all(src_dir);
    let _ = std::fs::remove_dir_all(out);
}

#[test]
fn verify_build_rejects_deploy_smoke_dap_native_route_count_mismatch() {
    let (src_dir, path) =
        multi_route_prod_server_source("deploy-smoke-dap-route-count-mismatch-source");
    let out = temp_output_dir("deploy-smoke-dap-route-count-mismatch");

    cmd_build_with_profile(&path, &out, BuildProfile::Production).expect("prod build");
    let smoke_path = out.join("deploy").join("smoke-test.sh");
    let smoke = std::fs::read_to_string(&smoke_path).expect("smoke test");
    let native_summary = deploy_native_server_summary_counts(&out).expect("native summary counts");
    let native_target_gate = format!(
        r#"orv_smoke_dap_summary_contains "dap native target summary" '"native_server_target_count": {}'"#,
        native_summary.targets
    );
    let wrong_native_target_gate = format!(
        r#"orv_smoke_dap_summary_contains "dap native target summary" '"native_server_target_count": {}'"#,
        native_summary.targets + 1
    );
    write_text(
        &smoke_path,
        &smoke.replace(&native_target_gate, &wrong_native_target_gate),
    )
    .expect("write corrupt smoke test");

    let err = cmd_verify_build(&out).expect_err("smoke DAP native target count mismatch");

    assert!(
        err.to_string()
            .contains("deploy smoke test must check DAP native production summary counters"),
        "{err:?}"
    );
    write_text(&smoke_path, &smoke).expect("restore smoke test");

    write_text(
        &smoke_path,
        &smoke.replace(
            r#"orv_smoke_dap_summary_contains "dap native route summary" '"native_server_route_count": 2'"#,
            r#"orv_smoke_dap_summary_contains "dap native route summary" '"native_server_route_count": 1'"#,
        ),
    )
    .expect("write corrupt smoke test");

    let err = cmd_verify_build(&out).expect_err("smoke DAP native route count mismatch");

    assert!(
        err.to_string()
            .contains("deploy smoke test must check DAP native production summary counters"),
        "{err:?}"
    );
    let _ = std::fs::remove_dir_all(src_dir);
    let _ = std::fs::remove_dir_all(out);
}

#[test]
fn verify_build_rejects_deploy_smoke_dap_marker_contract_missing() {
    let (src_dir, path) = prod_server_source("deploy-smoke-dap-marker-contract-source");
    let out = temp_output_dir("deploy-smoke-dap-marker-contract-missing");

    cmd_build_with_profile(&path, &out, BuildProfile::Production).expect("prod build");
    let smoke_path = out.join("deploy").join("smoke-test.sh");
    let smoke = std::fs::read_to_string(&smoke_path).expect("smoke test");
    write_text(
        &smoke_path,
        &smoke.replace(
            r#"orv_smoke_dap_summary_contains "dap smoke required markers" '"smoke_test_required_markers": ['
"#,
            "",
        ),
    )
    .expect("write corrupt smoke test");

    let err = cmd_verify_build(&out).expect_err("smoke DAP marker contract mismatch");

    assert!(
        err.to_string().contains(
            "deploy smoke test must verify smoke marker contract in DAP production context"
        ),
        "{err:?}"
    );
    let _ = std::fs::remove_dir_all(src_dir);
    let _ = std::fs::remove_dir_all(out);
}

#[test]
fn verify_build_rejects_deploy_smoke_output_dap_marker_missing() {
    let (src_dir, path) = prod_server_source("deploy-smoke-output-dap-source");
    let out = temp_output_dir("deploy-smoke-output-dap-missing");

    cmd_build_with_profile(&path, &out, BuildProfile::Production).expect("prod build");
    let smoke_path = out.join("deploy").join("smoke-test.sh");
    let smoke = std::fs::read_to_string(&smoke_path).expect("smoke test");
    write_text(
        &smoke_path,
        &smoke.replace("dap_summary=verified", "dap_summary=missing"),
    )
    .expect("write corrupt smoke test");

    let err = cmd_verify_build(&out).expect_err("smoke output DAP marker mismatch");

    assert!(
        err.to_string()
            .contains("deploy smoke test must write deploy smoke output artifact"),
        "{err:?}"
    );
    let _ = std::fs::remove_dir_all(src_dir);
    let _ = std::fs::remove_dir_all(out);
}

#[test]
fn editor_export_embeds_dap_debug_wiring() {
    let dir = temp_output_dir("editor-export-debug");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let path = dir.join("app.orv");
    std::fs::write(
            &path,
            "function helper(value: int): int -> {\n  value + 1\n}\nlet total: int = 41\nlet next: int = total + 1\n@out next\n",
        )
        .expect("write source");
    let out = dir.join("editor");

    cmd_editor_export(&path, &out).expect("editor export");

    let html = std::fs::read_to_string(out.join("index.html")).expect("editor html");
    let state = read_json_value(&out.join("state.json")).expect("editor state");
    assert!(html.contains("native-host/bridge.js"));
    assert_eq!(state["debug"]["schema_version"], 1);
    assert_eq!(state["debug"]["adapter"]["protocol"], "dap");
    assert_eq!(
        state["debug"]["adapter"]["command"],
        serde_json::json!(["orv", "dap", "serve", "--stdio"])
    );
    assert_eq!(
        state["debug"]["capabilities"]["supportsStepBack"],
        serde_json::json!(true)
    );
    assert_eq!(
        state["debug"]["capabilities"]["supportsLoadedSourcesRequest"],
        serde_json::json!(true)
    );
    assert_eq!(
        state["debug"]["capabilities"]["supportsStepInTargetsRequest"],
        serde_json::json!(true)
    );
    assert_eq!(
        state["debug"]["capabilities"]["supportsRestartFrame"],
        serde_json::json!(true)
    );
    assert_eq!(
        state["debug"]["session_runner"]["kind"],
        "orv.editor.debug.runner"
    );
    assert_eq!(
        state["debug"]["session_runner"]["command"],
        serde_json::json!([
            "orv",
            "editor",
            "run-debug",
            "debug/session-runner.json",
            "--control",
            "next"
        ])
    );
    assert_eq!(
        state["debug"]["session_runner"]["session"]["reuse_session"],
        true
    );
    assert_eq!(
        state["debug"]["session_runner"]["session"]["breakpoint_argument"],
        "--breakpoint"
    );
    assert_eq!(
        state["debug"]["session_runner"]["session"]["function_breakpoint_argument"],
        "--function-breakpoint"
    );
    assert_eq!(
        state["debug"]["session_runner"]["session"]["data_breakpoint_argument"],
        "--data-breakpoint"
    );
    assert_eq!(
        state["debug"]["session_runner"]["session"]["exception_filter_argument"],
        "--exception-filter"
    );
    assert_eq!(
        state["debug"]["session_runner"]["session"]["watch_expression_argument"],
        "--watch-expression"
    );
    assert!(state["debug"]["function_breakpoints"]
        .as_array()
        .expect("function breakpoints")
        .iter()
        .any(|breakpoint| {
            breakpoint["name"] == "helper"
                && breakpoint["request"]["command"] == "setFunctionBreakpoints"
                && breakpoint["runner_command"]
                    .as_array()
                    .is_some_and(|command| {
                        command.iter().any(|part| part == "--function-breakpoint")
                            && command.iter().any(|part| part == "helper")
                    })
        }));
    assert!(state["debug"]["data_breakpoints"]
        .as_array()
        .expect("data breakpoints")
        .iter()
        .any(|breakpoint| {
            breakpoint["name"] == "total"
                && breakpoint["info_request"]["command"] == "dataBreakpointInfo"
                && breakpoint["request"]["command"] == "setDataBreakpoints"
                && breakpoint["runner_command"]
                    .as_array()
                    .is_some_and(|command| {
                        command.iter().any(|part| part == "--data-breakpoint")
                            && command.iter().any(|part| part == "total")
                    })
        }));
    assert!(state["debug"]["exception_filters"]
        .as_array()
        .expect("exception filters")
        .iter()
        .any(|filter| {
            filter["filter"] == "orv.runtime"
                && filter["request"]["command"] == "setExceptionBreakpoints"
                && filter["runner_command"].as_array().is_some_and(|command| {
                    command.iter().any(|part| part == "--exception-filter")
                        && command.iter().any(|part| part == "orv.runtime")
                })
        }));
    assert_eq!(
        state["debug"]["session_runner"]["result"]["path"],
        EDITOR_DEBUG_SESSION_RESULT_PATH
    );
    assert_eq!(
        state["debug"]["result_artifact"]["path"],
        EDITOR_DEBUG_SESSION_RESULT_PATH
    );
    assert_eq!(
        state["debug"]["result_artifact"]["kind"],
        "orv.editor.debug.runner.result"
    );
    assert_eq!(
        state["debug"]["result_artifact"]["panel_contract"]["root"],
        "panels.debug"
    );
    assert_eq!(
        state["debug"]["source_inventory"]["kind"],
        "orv.editor.debug.source_inventory"
    );
    assert_eq!(state["debug"]["source_inventory"]["source_count"], 1);
    assert_eq!(
        state["debug"]["source_inventory"]["loaded_sources_request"]["command"],
        "loadedSources"
    );
    assert!(state["debug"]["source_inventory"]["sources"]
        .as_array()
        .expect("source inventory")
        .iter()
        .any(|source| {
            source["source"]["path"]
                .as_str()
                .is_some_and(|path| path.ends_with("app.orv"))
                && source["source"]["sourceReference"] == 1
                && source["request"]["command"] == "source"
                && source["request"]["arguments"]["sourceReference"] == 1
                && source["checksum"]["algorithm"] == "SHA256"
        }));
    assert!(
        state["debug"]["result_artifact"]["panel_contract"]["sections"]
            .as_array()
            .expect("result panel sections")
            .iter()
            .any(|section| {
                section["name"] == "session_summary"
                    && section["path"] == "panels.debug.session_summary"
            })
    );
    assert_editor_debug_runner_artifact(&out, &state);
    assert_editor_native_host_manifest(&out, &state);
    assert_editor_debug_configurations(&state);
    assert_editor_debug_breakpoint_sources(&state);
    assert_editor_debug_controls(&state);
    assert_editor_debug_html(&html);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn editor_debug_control_sequence_reuses_one_dap_session() {
    let dir = temp_output_dir("editor-debug-control-sequence");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let path = dir.join("app.orv");
    std::fs::write(
        &path,
        "let first: int = 1\nlet second: int = 2\nlet third: int = 3\n",
    )
    .expect("write source");

    let debug = editor_debug_session_json(
        &path,
        &[EditorDebugControl::Next, EditorDebugControl::Next],
        &[],
        &[],
        &[],
        &[],
        &[],
    )
    .expect("editor debug session");

    let controls = debug["controls"].as_array().expect("controls");
    assert_eq!(controls.len(), 2);
    assert!(controls
        .iter()
        .all(|control| control["response"]["success"] == true));
    assert_eq!(debug["transport"]["request_count"], 10);
    assert_eq!(debug["stack"]["stackFrames"][0]["line"], 3);
    assert!(debug["locals"]
        .as_array()
        .expect("locals")
        .iter()
        .any(|local| local["name"] == "third" && local["value"] == "3"));
    let step_stops = debug["frames"]
        .as_array()
        .expect("frames")
        .iter()
        .filter(|frame| {
            frame["type"] == "event"
                && frame["event"] == "stopped"
                && frame["body"]["reason"] == "step"
        })
        .count();
    assert!(step_stops >= 2, "{debug}");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn editor_debug_terminate_threads_control_uses_dap_session() {
    let dir = temp_output_dir("editor-debug-terminate-threads");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let path = dir.join("app.orv");
    std::fs::write(&path, "let answer: int = 42\n").expect("write source");

    let debug = editor_debug_session_json(
        &path,
        &[EditorDebugControl::TerminateThreads],
        &[],
        &[],
        &[],
        &[],
        &[],
    )
    .expect("editor debug session");

    assert_eq!(debug["control"]["request"]["command"], "terminateThreads");
    assert_eq!(debug["control"]["response"]["success"], true);
    assert!(debug["stack"]
        .as_object()
        .is_some_and(serde_json::Map::is_empty));
    assert!(debug["frames"]
        .as_array()
        .expect("frames")
        .iter()
        .any(|frame| frame["type"] == "event" && frame["event"] == "terminated"));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn editor_debug_terminate_control_uses_dap_session() {
    let dir = temp_output_dir("editor-debug-terminate");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let path = dir.join("app.orv");
    std::fs::write(&path, "let answer: int = 42\n").expect("write source");

    let debug = editor_debug_session_json(
        &path,
        &[EditorDebugControl::Terminate],
        &[],
        &[],
        &[],
        &[],
        &[],
    )
    .expect("editor debug session");

    assert_eq!(debug["control"]["request"]["command"], "terminate");
    assert_eq!(debug["control"]["response"]["success"], true);
    assert!(debug["stack"]
        .as_object()
        .is_some_and(serde_json::Map::is_empty));
    assert!(debug["frames"]
        .as_array()
        .expect("frames")
        .iter()
        .any(|frame| frame["type"] == "event" && frame["event"] == "terminated"));
    let _ = std::fs::remove_dir_all(dir);
}
