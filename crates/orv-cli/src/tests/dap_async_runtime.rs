use super::*;

#[test]
fn dap_long_running_exposes_async_pause_resume_state() {
    let dir = temp_output_dir("dap-server-async-state");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let source = dir.join("app.orv");
    std::fs::write(
        &source,
        "@server { @listen 0 @route GET /ping { @respond 200 { ok: true } } }\n",
    )
    .expect("write source");
    let mut session = DapSession::default();

    let launch = session
        .message_response(&serde_json::json!({
            "seq": 226,
            "type": "request",
            "command": "launch",
            "arguments": {
                "program": format!("file://{}", source.display()),
            },
        }))
        .expect("launch response");
    session
        .message_response(&serde_json::json!({
            "seq": 227,
            "type": "request",
            "command": "continue",
            "arguments": {
                "threadId": 1,
            },
        }))
        .expect("continue response");
    session
        .message_response(&serde_json::json!({
            "seq": 228,
            "type": "request",
            "command": "pause",
            "arguments": {
                "threadId": 1,
            },
        }))
        .expect("pause response");
    let variables = session
        .message_response(&serde_json::json!({
            "seq": 229,
            "type": "request",
            "command": "variables",
            "arguments": {
                "variablesReference": 1,
            },
        }))
        .expect("variables response");
    let async_state = session
        .message_response(&serde_json::json!({
            "seq": 230,
            "type": "request",
            "command": "evaluate",
            "arguments": {
                "expression": "runtimeAsyncState",
            },
        }))
        .expect("evaluate response");
    let completions = session
        .message_response(&serde_json::json!({
            "seq": 231,
            "type": "request",
            "command": "completions",
            "arguments": {
                "text": "runtime",
                "column": 8,
                "line": 1,
            },
        }))
        .expect("completions response");

    assert_eq!(launch["success"], true, "{launch}");
    assert_eq!(launch["body"]["runtime"]["async"]["kind"], "server");
    assert_eq!(launch["body"]["runtime"]["async"]["state"], "paused");
    assert!(variables["body"]["variables"]
        .as_array()
        .expect("variables")
        .iter()
        .any(|variable| variable["name"] == "runtimeAsyncState" && variable["value"] == "paused"));
    assert!(variables["body"]["variables"]
        .as_array()
        .expect("variables")
        .iter()
        .any(|variable| variable["name"] == "runtimeResumeCount" && variable["value"] == "1"));
    assert!(variables["body"]["variables"]
        .as_array()
        .expect("variables")
        .iter()
        .any(|variable| variable["name"] == "runtimePauseCount" && variable["value"] == "1"));
    assert_eq!(async_state["success"], true, "{async_state}");
    assert_eq!(async_state["body"]["result"], "paused");
    assert!(completions["body"]["targets"]
        .as_array()
        .expect("completion targets")
        .iter()
        .any(|target| target["label"] == "runtimeAsyncState" && target["type"] == "property"));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn dap_long_running_exposes_async_route_inventory() {
    let dir = temp_output_dir("dap-server-async-routes");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let source = dir.join("app.orv");
    std::fs::write(
        &source,
        "@server { @listen 0 @route GET /ping { @respond 200 { ok: true } } }\n",
    )
    .expect("write source");
    let mut session = DapSession::default();

    let launch = session
        .message_response(&serde_json::json!({
            "seq": 232,
            "type": "request",
            "command": "launch",
            "arguments": {
                "program": format!("file://{}", source.display()),
            },
        }))
        .expect("launch response");
    let variables = session
        .message_response(&serde_json::json!({
            "seq": 233,
            "type": "request",
            "command": "variables",
            "arguments": {
                "variablesReference": 1,
            },
        }))
        .expect("variables response");
    let routes = session
        .message_response(&serde_json::json!({
            "seq": 234,
            "type": "request",
            "command": "evaluate",
            "arguments": {
                "expression": "runtimeRoutes",
            },
        }))
        .expect("route evaluate response");
    let completions = session
        .message_response(&serde_json::json!({
            "seq": 235,
            "type": "request",
            "command": "completions",
            "arguments": {
                "text": "runtimeR",
                "column": 9,
                "line": 1,
            },
        }))
        .expect("completions response");

    assert_eq!(launch["body"]["runtime"]["async"]["route_count"], 1);
    assert_eq!(
        launch["body"]["runtime"]["async"]["routes"][0]["path"],
        "/ping"
    );
    assert!(variables["body"]["variables"]
        .as_array()
        .expect("variables")
        .iter()
        .any(|variable| variable["name"] == "runtimeRouteCount" && variable["value"] == "1"));
    assert_eq!(routes["success"], true, "{routes}");
    assert_eq!(routes["body"]["result"], "GET /ping");
    assert!(completions["body"]["targets"]
        .as_array()
        .expect("completion targets")
        .iter()
        .any(|target| target["label"] == "runtimeRoutes" && target["type"] == "property"));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn dap_long_running_exposes_async_listen_endpoint() {
    let dir = temp_output_dir("dap-server-async-listen");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let source = dir.join("app.orv");
    std::fs::write(
        &source,
        "@server { @listen 8080 @route GET /ping { @respond 200 { ok: true } } }\n",
    )
    .expect("write source");
    let mut session = DapSession::default();

    let launch = session
        .message_response(&serde_json::json!({
            "seq": 236,
            "type": "request",
            "command": "launch",
            "arguments": {
                "program": format!("file://{}", source.display()),
            },
        }))
        .expect("launch response");
    let variables = session
        .message_response(&serde_json::json!({
            "seq": 237,
            "type": "request",
            "command": "variables",
            "arguments": {
                "variablesReference": 1,
            },
        }))
        .expect("variables response");
    let listen = session
        .message_response(&serde_json::json!({
            "seq": 238,
            "type": "request",
            "command": "evaluate",
            "arguments": {
                "expression": "runtimeListen",
            },
        }))
        .expect("listen evaluate response");
    let completions = session
        .message_response(&serde_json::json!({
            "seq": 239,
            "type": "request",
            "command": "completions",
            "arguments": {
                "text": "runtimeL",
                "column": 9,
                "line": 1,
            },
        }))
        .expect("completions response");

    assert_eq!(
        launch["body"]["runtime"]["async"]["listen"]["kind"],
        "static"
    );
    assert_eq!(launch["body"]["runtime"]["async"]["listen"]["port"], 8080);
    assert!(variables["body"]["variables"]
        .as_array()
        .expect("variables")
        .iter()
        .any(|variable| variable["name"] == "runtimeListen" && variable["value"] == "8080"));
    assert_eq!(listen["success"], true, "{listen}");
    assert_eq!(listen["body"]["result"], "8080");
    assert!(completions["body"]["targets"]
        .as_array()
        .expect("completion targets")
        .iter()
        .any(|target| target["label"] == "runtimeListen" && target["type"] == "property"));
    let _ = std::fs::remove_dir_all(dir);
}
