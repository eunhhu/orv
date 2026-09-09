use super::*;

#[test]
fn dap_set_instruction_breakpoints_requires_launch_for_verification() {
    let response = dap_protocol_response(&serde_json::json!({
        "seq": 77,
        "type": "request",
        "command": "setInstructionBreakpoints",
        "arguments": {
            "breakpoints": [
                {
                    "instructionReference": "orv:entry:0",
                    "offset": 4,
                }
            ],
        },
    }));

    assert_eq!(response["type"], "response");
    assert_eq!(response["request_seq"], 77);
    assert_eq!(response["command"], "setInstructionBreakpoints");
    assert_eq!(response["success"], true);
    let breakpoint = &response["body"]["breakpoints"][0];
    assert_eq!(breakpoint["verified"], false);
    assert_eq!(breakpoint["instructionReference"], "orv:entry:0");
    assert_eq!(breakpoint["offset"], 4);
    assert_eq!(
        breakpoint["message"],
        "launch is required before verifying ORV instruction breakpoints"
    );
}

#[test]
fn dap_instruction_breakpoint_stops_continue_at_frame() {
    let dir = temp_output_dir("dap-instruction-breakpoint");
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
            "seq": 82,
            "type": "request",
            "command": "launch",
            "arguments": {
                "program": format!("file://{}", source.display()),
            },
        }))
        .expect("launch response");
    let set_instruction_breakpoints = session
        .message_response(&serde_json::json!({
            "seq": 83,
            "type": "request",
            "command": "setInstructionBreakpoints",
            "arguments": {
                "breakpoints": [
                    {
                        "instructionReference": "orv:frame:2",
                        "offset": 0,
                    }
                ],
            },
        }))
        .expect("setInstructionBreakpoints response");
    let continue_response = session
        .message_response(&serde_json::json!({
            "seq": 84,
            "type": "request",
            "command": "continue",
            "arguments": {
                "threadId": 1,
            },
        }))
        .expect("continue response");
    let events = session.drain_pending_events();
    let stack = session
        .message_response(&serde_json::json!({
            "seq": 85,
            "type": "request",
            "command": "stackTrace",
            "arguments": {
                "threadId": 1,
            },
        }))
        .expect("stack response");

    assert_eq!(
        set_instruction_breakpoints["body"]["breakpoints"][0]["verified"],
        true
    );
    assert_eq!(
        set_instruction_breakpoints["body"]["breakpoints"][0]["instructionReference"],
        "orv:frame:2"
    );
    assert_eq!(continue_response["success"], true, "{continue_response}");
    assert_eq!(stack["body"]["stackFrames"][0]["line"], 2);
    assert!(events.iter().any(|event| {
        event["type"] == "event"
            && event["event"] == "stopped"
            && event["body"]["reason"] == "instruction breakpoint"
    }));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn dap_set_exception_breakpoints_accepts_orv_filters() {
    let mut session = DapSession::default();

    let response = session
        .message_response(&serde_json::json!({
            "seq": 67,
            "type": "request",
            "command": "setExceptionBreakpoints",
            "arguments": {
                "filters": ["orv.diagnostics", "orv.runtime"],
            },
        }))
        .expect("setExceptionBreakpoints response");

    assert_eq!(response["success"], true, "{response}");
    assert_eq!(response["command"], "setExceptionBreakpoints");
    assert_eq!(
        response["body"]["breakpoints"]
            .as_array()
            .expect("breakpoints")
            .len(),
        2
    );
    assert_eq!(response["body"]["breakpoints"][0]["verified"], true);
    assert_eq!(
        response["body"]["breakpoints"][0]["filter"],
        "orv.diagnostics"
    );
    assert_eq!(response["body"]["breakpoints"][1]["verified"], true);
    assert_eq!(response["body"]["breakpoints"][1]["filter"], "orv.runtime");
}

#[test]
fn dap_set_exception_breakpoints_empty_filters_disable_diagnostic_stop_reason() {
    let dir = temp_output_dir("dap-exception-filters-empty");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let source = dir.join("app.orv");
    std::fs::write(&source, "let bad: int = \"wrong\"\n").expect("write source");
    let mut session = DapSession::default();

    session
        .message_response(&serde_json::json!({
            "seq": 68,
            "type": "request",
            "command": "setExceptionBreakpoints",
            "arguments": {
                "filters": [],
            },
        }))
        .expect("setExceptionBreakpoints response");
    session
        .message_response(&serde_json::json!({
            "seq": 69,
            "type": "request",
            "command": "launch",
            "arguments": {
                "program": format!("file://{}", source.display()),
            },
        }))
        .expect("launch response");
    session
        .message_response(&serde_json::json!({
            "seq": 70,
            "type": "request",
            "command": "configurationDone",
            "arguments": {},
        }))
        .expect("configurationDone response");
    let events = session.drain_pending_events();

    assert!(events
        .iter()
        .any(|event| { event["event"] == "stopped" && event["body"]["reason"] == "entry" }));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn dap_set_breakpoints_accepts_loaded_source_reference() {
    let dir = temp_output_dir("dap-set-breakpoints-source-ref");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let source = dir.join("app.orv");
    std::fs::write(&source, "let answer: int = 42\n").expect("write source");
    let mut session = DapSession::default();

    session
        .message_response(&serde_json::json!({
            "seq": 7,
            "type": "request",
            "command": "launch",
            "arguments": {
                "program": format!("file://{}", source.display()),
            },
        }))
        .expect("launch response");
    let response = session
        .message_response(&serde_json::json!({
            "seq": 8,
            "type": "request",
            "command": "setBreakpoints",
            "arguments": {
                "source": {
                    "sourceReference": 1,
                },
                "breakpoints": [
                    {
                        "line": 1,
                    },
                ],
            },
        }))
        .expect("setBreakpoints response");

    assert_eq!(response["success"], true, "{response}");
    assert_eq!(response["body"]["breakpoints"][0]["verified"], true);
    assert_eq!(response["body"]["breakpoints"][0]["line"], 1);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn dap_continue_stops_at_next_verified_breakpoint_frame() {
    let dir = temp_output_dir("dap-continue-breakpoint-frame");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let source = dir.join("app.orv");
    std::fs::write(
        &source,
        "let first: int = 1\nlet middle: int = 2\nlet last: int = 3\n",
    )
    .expect("write source");
    let mut session = DapSession::default();

    session
        .message_response(&serde_json::json!({
            "seq": 158,
            "type": "request",
            "command": "setBreakpoints",
            "arguments": {
                "source": {
                    "path": source.display().to_string(),
                },
                "breakpoints": [
                    { "line": 1 },
                    { "line": 3 },
                ],
            },
        }))
        .expect("breakpoints response");
    session
        .message_response(&serde_json::json!({
            "seq": 159,
            "type": "request",
            "command": "launch",
            "arguments": {
                "program": format!("file://{}", source.display()),
            },
        }))
        .expect("launch response");
    let first_stack = session
        .message_response(&serde_json::json!({
            "seq": 160,
            "type": "request",
            "command": "stackTrace",
            "arguments": {
                "threadId": 1,
            },
        }))
        .expect("first stack response");
    session
        .message_response(&serde_json::json!({
            "seq": 161,
            "type": "request",
            "command": "continue",
            "arguments": {
                "threadId": 1,
            },
        }))
        .expect("continue response");
    let events = session.drain_pending_events();
    let second_stack = session
        .message_response(&serde_json::json!({
            "seq": 162,
            "type": "request",
            "command": "stackTrace",
            "arguments": {
                "threadId": 1,
            },
        }))
        .expect("second stack response");

    assert_eq!(first_stack["body"]["stackFrames"][0]["line"], 1);
    assert_eq!(second_stack["body"]["stackFrames"][0]["line"], 3);
    assert!(events.iter().any(|event| {
        event["type"] == "event" && event["event"] == "continued" && event["body"]["threadId"] == 1
    }));
    assert!(events.iter().any(|event| {
        event["type"] == "event"
            && event["event"] == "stopped"
            && event["body"]["reason"] == "breakpoint"
            && event["body"]["threadId"] == 1
    }));
    assert!(!events
        .iter()
        .any(|event| event["type"] == "event" && event["event"] == "terminated"));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn dap_conditional_breakpoint_skips_false_condition_frame() {
    let dir = temp_output_dir("dap-conditional-breakpoint");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let source = dir.join("app.orv");
    std::fs::write(
        &source,
        "let mut total: int = 1\ntotal = total + 4\ntotal = total + 4\n",
    )
    .expect("write source");
    let mut session = DapSession::default();

    session
        .message_response(&serde_json::json!({
            "seq": 204,
            "type": "request",
            "command": "setBreakpoints",
            "arguments": {
                "source": {
                    "path": source.display().to_string(),
                },
                "breakpoints": [
                    {
                        "line": 2,
                        "condition": "total == 9",
                    },
                    {
                        "line": 3,
                        "condition": "total == 9",
                    },
                ],
            },
        }))
        .expect("setBreakpoints response");
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
            },
        }))
        .expect("stack response");

    assert_eq!(stack["success"], true, "{stack}");
    assert_eq!(stack["body"]["stackFrames"][0]["line"], 3);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn dap_hit_condition_breakpoint_stops_on_requested_hit() {
    let dir = temp_output_dir("dap-hit-condition-breakpoint");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let source = dir.join("app.orv");
    std::fs::write(
        &source,
        r"function bump(value: int): int -> {
  let result: int = value + 1
  result
}
let first: int = bump(0)
let second: int = bump(1)
",
    )
    .expect("write source");
    let mut session = DapSession::default();

    session
        .message_response(&serde_json::json!({
            "seq": 207,
            "type": "request",
            "command": "setBreakpoints",
            "arguments": {
                "source": {
                    "path": source.display().to_string(),
                },
                "breakpoints": [
                    {
                        "line": 2,
                        "hitCondition": "2",
                    },
                ],
            },
        }))
        .expect("setBreakpoints response");
    session
        .message_response(&serde_json::json!({
            "seq": 208,
            "type": "request",
            "command": "launch",
            "arguments": {
                "program": format!("file://{}", source.display()),
            },
        }))
        .expect("launch response");
    let locals = session
        .message_response(&serde_json::json!({
            "seq": 209,
            "type": "request",
            "command": "variables",
            "arguments": {
                "variablesReference": 2,
            },
        }))
        .expect("locals response");

    let vars = locals["body"]["variables"].as_array().expect("locals");
    assert!(
        vars.iter()
            .any(|var| var["name"] == "result" && var["value"] == "2"),
        "{locals}"
    );
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn dap_reverse_continue_stops_at_previous_verified_breakpoint_frame() {
    let dir = temp_output_dir("dap-reverse-continue");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let source = dir.join("app.orv");
    std::fs::write(
        &source,
        "let first: int = 1\nlet middle: int = 2\nlet last: int = 3\n",
    )
    .expect("write source");
    let mut session = DapSession::default();

    session
        .message_response(&serde_json::json!({
            "seq": 181,
            "type": "request",
            "command": "setBreakpoints",
            "arguments": {
                "source": {
                    "path": source.display().to_string(),
                },
                "breakpoints": [
                    { "line": 1 },
                    { "line": 3 },
                ],
            },
        }))
        .expect("breakpoints response");
    session
        .message_response(&serde_json::json!({
            "seq": 182,
            "type": "request",
            "command": "launch",
            "arguments": {
                "program": format!("file://{}", source.display()),
            },
        }))
        .expect("launch response");
    session
        .message_response(&serde_json::json!({
            "seq": 183,
            "type": "request",
            "command": "continue",
            "arguments": {
                "threadId": 1,
            },
        }))
        .expect("continue response");
    let _ = session.drain_pending_events();
    let reverse = session
        .message_response(&serde_json::json!({
            "seq": 184,
            "type": "request",
            "command": "reverseContinue",
            "arguments": {
                "threadId": 1,
            },
        }))
        .expect("reverseContinue response");
    let events = session.drain_pending_events();
    let stack = session
        .message_response(&serde_json::json!({
            "seq": 185,
            "type": "request",
            "command": "stackTrace",
            "arguments": {
                "threadId": 1,
            },
        }))
        .expect("stack response");

    assert_eq!(reverse["success"], true, "{reverse}");
    assert_eq!(stack["body"]["stackFrames"][0]["line"], 1);
    assert!(events.iter().any(|event| {
        event["type"] == "event"
            && event["event"] == "stopped"
            && event["body"]["reason"] == "breakpoint"
    }));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn dap_function_breakpoint_stops_inside_named_function() {
    let dir = temp_output_dir("dap-function-breakpoint");
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

    let breakpoints = session
        .message_response(&serde_json::json!({
            "seq": 190,
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
            "seq": 191,
            "type": "request",
            "command": "launch",
            "arguments": {
                "program": format!("file://{}", source.display()),
            },
        }))
        .expect("launch response");
    session
        .message_response(&serde_json::json!({
            "seq": 192,
            "type": "request",
            "command": "configurationDone",
            "arguments": {},
        }))
        .expect("configurationDone response");
    let events = session.drain_pending_events();
    let stack = session
        .message_response(&serde_json::json!({
            "seq": 193,
            "type": "request",
            "command": "stackTrace",
            "arguments": {
                "threadId": 1,
            },
        }))
        .expect("stack response");

    assert_eq!(breakpoints["success"], true, "{breakpoints}");
    assert_eq!(breakpoints["body"]["breakpoints"][0]["verified"], true);
    assert_eq!(stack["success"], true, "{stack}");
    assert_eq!(stack["body"]["stackFrames"][0]["name"], "add");
    assert_eq!(stack["body"]["stackFrames"][0]["line"], 2);
    assert!(events.iter().any(|event| {
        event["type"] == "event"
            && event["event"] == "stopped"
            && event["body"]["reason"] == "function breakpoint"
    }));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn dap_continue_stops_at_next_function_breakpoint_frame() {
    let dir = temp_output_dir("dap-continue-function-breakpoint");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let source = dir.join("app.orv");
    std::fs::write(
        &source,
        r"let first: int = 1
function add(a: int, b: int): int -> {
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
            "seq": 194,
            "type": "request",
            "command": "setBreakpoints",
            "arguments": {
                "source": {
                    "path": source.display().to_string(),
                },
                "breakpoints": [
                    { "line": 1 },
                ],
            },
        }))
        .expect("setBreakpoints response");
    session
        .message_response(&serde_json::json!({
            "seq": 195,
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
            "seq": 196,
            "type": "request",
            "command": "launch",
            "arguments": {
                "program": format!("file://{}", source.display()),
            },
        }))
        .expect("launch response");
    session
        .message_response(&serde_json::json!({
            "seq": 197,
            "type": "request",
            "command": "continue",
            "arguments": {
                "threadId": 1,
            },
        }))
        .expect("continue response");
    let events = session.drain_pending_events();
    let stack = session
        .message_response(&serde_json::json!({
            "seq": 198,
            "type": "request",
            "command": "stackTrace",
            "arguments": {
                "threadId": 1,
            },
        }))
        .expect("stack response");

    assert_eq!(stack["success"], true, "{stack}");
    assert_eq!(stack["body"]["stackFrames"][0]["name"], "add");
    assert_eq!(stack["body"]["stackFrames"][0]["line"], 3);
    assert!(events.iter().any(|event| {
        event["type"] == "event"
            && event["event"] == "stopped"
            && event["body"]["reason"] == "function breakpoint"
    }));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn dap_data_breakpoint_stops_when_local_changes() {
    let dir = temp_output_dir("dap-data-breakpoint");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let source = dir.join("app.orv");
    std::fs::write(&source, "let mut total: int = 1\ntotal = total + 4\n").expect("write source");
    let mut session = DapSession::default();

    session
        .message_response(&serde_json::json!({
            "seq": 199,
            "type": "request",
            "command": "launch",
            "arguments": {
                "program": format!("file://{}", source.display()),
            },
        }))
        .expect("launch response");
    let info = session
        .message_response(&serde_json::json!({
            "seq": 200,
            "type": "request",
            "command": "dataBreakpointInfo",
            "arguments": {
                "variablesReference": 2,
                "name": "total",
            },
        }))
        .expect("dataBreakpointInfo response");
    let data_id = info["body"]["dataId"].as_str().expect("data id");
    let set_data = session
        .message_response(&serde_json::json!({
            "seq": 201,
            "type": "request",
            "command": "setDataBreakpoints",
            "arguments": {
                "breakpoints": [
                    {
                        "dataId": data_id,
                        "accessType": "write",
                    },
                ],
            },
        }))
        .expect("setDataBreakpoints response");
    session
        .message_response(&serde_json::json!({
            "seq": 202,
            "type": "request",
            "command": "continue",
            "arguments": {
                "threadId": 1,
            },
        }))
        .expect("continue response");
    let events = session.drain_pending_events();
    let stack = session
        .message_response(&serde_json::json!({
            "seq": 203,
            "type": "request",
            "command": "stackTrace",
            "arguments": {
                "threadId": 1,
            },
        }))
        .expect("stack response");

    assert_eq!(info["success"], true, "{info}");
    assert_eq!(info["body"]["dataId"], "local:total");
    assert_eq!(set_data["success"], true, "{set_data}");
    assert_eq!(set_data["body"]["breakpoints"][0]["verified"], true);
    assert_eq!(stack["success"], true, "{stack}");
    assert_eq!(stack["body"]["stackFrames"][0]["line"], 2);
    assert!(events.iter().any(|event| {
        event["type"] == "event"
            && event["event"] == "stopped"
            && event["body"]["reason"] == "data breakpoint"
    }));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn dap_live_continue_stops_at_breakpoint_before_program_end() {
    let dir = temp_output_dir("dap-live-continue-breakpoint");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let source = dir.join("app.orv");
    std::fs::write(
        &source,
        "let first: int = 1\n@out \"middle\"\nlet third: int = 3\nlet done: int = 4\n",
    )
    .expect("write source");
    let mut session = DapSession::default();

    session
        .message_response(&serde_json::json!({
            "seq": 211,
            "type": "request",
            "command": "setBreakpoints",
            "arguments": {
                "source": {
                    "path": source.display().to_string(),
                },
                "breakpoints": [
                    {
                        "line": 3,
                    },
                ],
            },
        }))
        .expect("setBreakpoints response");
    let launch = session
        .message_response(&serde_json::json!({
            "seq": 212,
            "type": "request",
            "command": "launch",
            "arguments": {
                "program": format!("file://{}", source.display()),
                "live": true,
            },
        }))
        .expect("launch response");
    let _ = session.drain_pending_events();
    let continue_response = session
        .message_response(&serde_json::json!({
            "seq": 213,
            "type": "request",
            "command": "continue",
            "arguments": {
                "threadId": 1,
            },
        }))
        .expect("continue response");
    let events = session.drain_pending_events();
    let stack = session
        .message_response(&serde_json::json!({
            "seq": 214,
            "type": "request",
            "command": "stackTrace",
            "arguments": {
                "threadId": 1,
            },
        }))
        .expect("stack response");

    assert_eq!(launch["body"]["runtime"]["status"], "running");
    assert_eq!(continue_response["success"], true, "{continue_response}");
    assert_eq!(stack["success"], true, "{stack}");
    assert_eq!(stack["body"]["stackFrames"][0]["line"], 3);
    assert!(events.iter().any(|event| {
        event["type"] == "event"
            && event["event"] == "stopped"
            && event["body"]["reason"] == "breakpoint"
    }));
    assert!(events.iter().all(|event| event["event"] != "terminated"));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn dap_breakpoint_locations_return_project_graph_lines() {
    let dir = temp_output_dir("dap-breakpoint-locations");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let source = dir.join("app.orv");
    std::fs::write(
        &source,
        r#"struct User { id: int }

function greet(user: User): string -> "hello"
"#,
    )
    .expect("write source");
    let mut session = DapSession::default();

    let response = session
        .message_response(&serde_json::json!({
            "seq": 51,
            "type": "request",
            "command": "breakpointLocations",
            "arguments": {
                "source": {
                    "path": format!("file://{}", source.display()),
                },
                "line": 1,
                "endLine": 3,
            },
        }))
        .expect("breakpointLocations response");

    assert_eq!(response["success"], true, "{response}");
    let breakpoints = response["body"]["breakpoints"]
        .as_array()
        .expect("breakpoint locations");
    assert!(breakpoints
        .iter()
        .any(|breakpoint| breakpoint["line"] == 1 && breakpoint["column"] == 1));
    assert!(breakpoints
        .iter()
        .any(|breakpoint| breakpoint["line"] == 3 && breakpoint["column"] == 1));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn dap_exception_info_returns_launch_runtime_status() {
    let dir = temp_output_dir("dap-exception-info");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let source = dir.join("app.orv");
    std::fs::write(&source, "let bad: int = \"wrong\"\n").expect("write source");
    let mut session = DapSession::default();

    let launch = session
        .message_response(&serde_json::json!({
            "seq": 52,
            "type": "request",
            "command": "launch",
            "arguments": {
                "program": format!("file://{}", source.display()),
            },
        }))
        .expect("launch response");
    let exception = session
        .message_response(&serde_json::json!({
            "seq": 53,
            "type": "request",
            "command": "exceptionInfo",
            "arguments": {
                "threadId": 1,
            },
        }))
        .expect("exceptionInfo response");

    assert_eq!(launch["success"], true, "{launch}");
    assert_eq!(launch["body"]["runtime"]["status"], "diagnostics");
    assert_eq!(exception["success"], true, "{exception}");
    assert_eq!(exception["body"]["exceptionId"], "orv.diagnostics");
    assert_eq!(exception["body"]["description"], "diagnostics present");
    assert_eq!(exception["body"]["breakMode"], "always");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn dap_set_breakpoints_and_stacktrace_use_verified_breakpoint_line() {
    let dir = temp_output_dir("dap-breakpoints");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let source = dir.join("app.orv");
    std::fs::write(&source, "let first: int = 1\nlet second: int = 2\n").expect("write source");
    let mut session = DapSession::default();

    let breakpoints = session
        .message_response(&serde_json::json!({
            "seq": 5,
            "type": "request",
            "command": "setBreakpoints",
            "arguments": {
                "source": {
                    "path": source.display().to_string(),
                },
                "breakpoints": [
                    { "line": 2 }
                ],
            },
        }))
        .expect("breakpoints response");
    let launch = session
        .message_response(&serde_json::json!({
            "seq": 6,
            "type": "request",
            "command": "launch",
            "arguments": {
                "program": format!("file://{}", source.display()),
            },
        }))
        .expect("launch response");
    let stack = session
        .message_response(&serde_json::json!({
            "seq": 7,
            "type": "request",
            "command": "stackTrace",
            "arguments": {
                "threadId": 1,
            },
        }))
        .expect("stack response");

    assert_eq!(breakpoints["success"], true, "{breakpoints}");
    assert_eq!(breakpoints["body"]["breakpoints"][0]["verified"], true);
    assert_eq!(breakpoints["body"]["breakpoints"][0]["line"], 2);
    assert_eq!(launch["success"], true, "{launch}");
    assert_eq!(stack["body"]["stackFrames"][0]["line"], 2);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn dap_set_breakpoints_rejects_non_executable_lines() {
    let dir = temp_output_dir("dap-breakpoint-verify");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let source = dir.join("app.orv");
    std::fs::write(&source, "let first: int = 1\n\nlet second: int = 2\n").expect("write source");
    let mut session = DapSession::default();

    let breakpoints = session
        .message_response(&serde_json::json!({
            "seq": 47,
            "type": "request",
            "command": "setBreakpoints",
            "arguments": {
                "source": {
                    "path": source.display().to_string(),
                },
                "breakpoints": [
                    { "line": 2 },
                    { "line": 3 }
                ],
            },
        }))
        .expect("breakpoints response");

    assert_eq!(breakpoints["success"], true, "{breakpoints}");
    assert_eq!(breakpoints["body"]["breakpoints"][0]["verified"], false);
    assert_eq!(
        breakpoints["body"]["breakpoints"][0]["message"],
        "no executable ORV node on this line"
    );
    assert_eq!(breakpoints["body"]["breakpoints"][1]["verified"], true);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn editor_debug_exception_filter_argument_configures_dap_session() {
    let dir = temp_output_dir("editor-debug-exception-filter");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let path = dir.join("app.orv");
    std::fs::write(&path, "let answer: int = 42\n").expect("write source");
    let exception_filters = vec!["orv.runtime".to_string()];

    let debug = editor_debug_session_json(
        &path,
        &[EditorDebugControl::Next],
        &[],
        &[],
        &[],
        &exception_filters,
        &[],
    )
    .expect("editor debug session");

    assert_eq!(
        debug["exception_filters"][0]["request"]["command"],
        "setExceptionBreakpoints"
    );
    assert_eq!(
        debug["exception_filters"][0]["filters"],
        serde_json::json!(["orv.runtime"])
    );
    assert_eq!(debug["exception_filters"][0]["response"]["success"], true);
    assert_eq!(debug["control"]["response"]["success"], true);
    let _ = std::fs::remove_dir_all(dir);
}
