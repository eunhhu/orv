use super::*;

#[test]
fn dap_serve_stdio_subcommand_is_accepted() {
    let parsed = Cli::try_parse_from(["orv", "dap", "serve", "--stdio"]);
    if let Err(err) = parsed {
        panic!("{}", err.render());
    }
}

#[test]
fn dap_stdio_serves_content_length_initialize_frame() {
    let body = serde_json::json!({
        "seq": 1,
        "type": "request",
        "command": "initialize",
        "arguments": {},
    })
    .to_string();
    let input = format!("Content-Length: {}\r\n\r\n{}", body.len(), body);

    let output = dap_stdio_response(&input).expect("stdio response");
    let frames = protocol_frames(&output);
    let response = &frames[0];

    assert!(output.starts_with("Content-Length: "));
    assert_eq!(response["type"], "response");
    assert_eq!(response["command"], "initialize");
    assert_eq!(response["success"], true);
}

#[test]
fn dap_stdio_emits_initialized_event_after_initialize() {
    let body = serde_json::json!({
        "seq": 1,
        "type": "request",
        "command": "initialize",
        "arguments": {},
    })
    .to_string();
    let input = format!("Content-Length: {}\r\n\r\n{}", body.len(), body);

    let output = dap_stdio_response(&input).expect("stdio response");
    let frames = protocol_frames(&output);

    assert_eq!(frames.len(), 2, "{output}");
    assert_eq!(frames[0]["type"], "response");
    assert_eq!(frames[0]["command"], "initialize");
    assert_eq!(frames[1]["type"], "event");
    assert_eq!(frames[1]["event"], "initialized");
}

#[test]
fn dap_stdio_emits_stopped_event_after_configuration_done() {
    let dir = temp_output_dir("dap-stopped-event");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let source = dir.join("app.orv");
    std::fs::write(&source, "let answer: int = 42\n").expect("write source");
    let input = [
        protocol_request_frame(&serde_json::json!({
            "seq": 1,
            "type": "request",
            "command": "launch",
            "arguments": {
                "program": format!("file://{}", source.display()),
            },
        })),
        protocol_request_frame(&serde_json::json!({
            "seq": 2,
            "type": "request",
            "command": "configurationDone",
            "arguments": {},
        })),
    ]
    .join("");

    let output = dap_stdio_response(&input).expect("stdio response");
    let frames = protocol_frames(&output);
    let stopped = frames
        .iter()
        .find(|frame| frame["type"] == "event" && frame["event"] == "stopped")
        .expect("stopped event");

    assert_eq!(stopped["body"]["reason"], "entry");
    assert_eq!(stopped["body"]["threadId"], 1);
    assert_eq!(stopped["body"]["allThreadsStopped"], false);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn dap_stdio_emits_continued_and_terminated_events_after_continue() {
    let dir = temp_output_dir("dap-continue-events");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let source = dir.join("app.orv");
    std::fs::write(&source, "let answer: int = 42\n").expect("write source");
    let input = [
        protocol_request_frame(&serde_json::json!({
            "seq": 1,
            "type": "request",
            "command": "launch",
            "arguments": {
                "program": format!("file://{}", source.display()),
            },
        })),
        protocol_request_frame(&serde_json::json!({
            "seq": 2,
            "type": "request",
            "command": "continue",
            "arguments": {
                "threadId": 1,
            },
        })),
    ]
    .join("");

    let output = dap_stdio_response(&input).expect("stdio response");
    let frames = protocol_frames(&output);
    let continued = frames
        .iter()
        .find(|frame| frame["type"] == "event" && frame["event"] == "continued")
        .expect("continued event");
    let terminated = frames
        .iter()
        .find(|frame| frame["type"] == "event" && frame["event"] == "terminated")
        .expect("terminated event");

    assert_eq!(continued["body"]["threadId"], 1);
    assert_eq!(continued["body"]["allThreadsContinued"], false);
    assert_eq!(terminated["body"], serde_json::json!({}));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn dap_stdio_emits_output_event_for_reference_stdout_after_launch() {
    let dir = temp_output_dir("dap-output-event");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let source = dir.join("app.orv");
    std::fs::write(&source, "@out \"debug-ready\"\n").expect("write source");
    let input = protocol_request_frame(&serde_json::json!({
        "seq": 55,
        "type": "request",
        "command": "launch",
        "arguments": {
            "program": format!("file://{}", source.display()),
        },
    }));

    let output = dap_stdio_response(&input).expect("stdio response");
    let frames = protocol_frames(&output);
    let output_event = frames
        .iter()
        .find(|frame| frame["type"] == "event" && frame["event"] == "output")
        .expect("output event");

    assert_eq!(output_event["body"]["category"], "stdout");
    assert_eq!(output_event["body"]["output"], "debug-ready\n");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn dap_stdio_emits_stderr_output_event_for_runtime_error_after_launch() {
    let dir = temp_output_dir("dap-error-output-event");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let source = dir.join("app.orv");
    std::fs::write(&source, "throw \"panic!\"\n").expect("write source");
    let input = protocol_request_frame(&serde_json::json!({
        "seq": 56,
        "type": "request",
        "command": "launch",
        "arguments": {
            "program": format!("file://{}", source.display()),
        },
    }));

    let output = dap_stdio_response(&input).expect("stdio response");
    let frames = protocol_frames(&output);
    let output_event = frames
        .iter()
        .find(|frame| frame["type"] == "event" && frame["event"] == "output")
        .expect("output event");

    assert_eq!(frames[0]["body"]["runtime"]["status"], "error");
    assert_eq!(output_event["body"]["category"], "stderr");
    assert!(output_event["body"]["output"]
        .as_str()
        .is_some_and(|output| output.contains("panic!")));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn editor_debug_control_uses_dap_stdio_transport() {
    let dir = temp_output_dir("editor-debug-control");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let path = dir.join("app.orv");
    std::fs::write(&path, "let first: int = 1\nlet second: int = 2\n").expect("write source");

    let debug =
        editor_debug_session_json(&path, &[EditorDebugControl::Next], &[], &[], &[], &[], &[])
            .expect("editor debug session");

    assert_eq!(debug["kind"], "orv.editor.debug");
    assert_eq!(debug["adapter"]["protocol"], "dap");
    assert_eq!(debug["transport"]["framing"], "content-length");
    assert_eq!(debug["control"]["request"]["command"], "next");
    assert_eq!(debug["control"]["response"]["success"], true);
    assert_eq!(debug["stack"]["stackFrames"][0]["line"], 2);
    assert!(debug["frames"]
        .as_array()
        .expect("frames")
        .iter()
        .any(|frame| {
            frame["type"] == "event"
                && frame["event"] == "stopped"
                && frame["body"]["reason"] == "step"
        }));
    let _ = std::fs::remove_dir_all(dir);
}
