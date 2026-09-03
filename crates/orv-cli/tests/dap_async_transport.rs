#![allow(clippy::too_many_lines)]

use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::{Duration, Instant};

mod support;

use support::{DapServer, PortReservation, TestDir};

fn wait_for_http_ok(address: &str) -> String {
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut last_error = String::new();
    while Instant::now() < deadline {
        match http_get(address) {
            Ok(response) if response.contains("200 OK") && response.contains(r#"{"ok":true}"#) => {
                return response;
            }
            Ok(response) => last_error = response,
            Err(err) => last_error = err,
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    panic!("server did not answer /ping: {last_error}");
}

fn wait_for_http_response(address: &str, path: &str, expected_body: &[&str]) -> String {
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut last_error = String::new();
    while Instant::now() < deadline {
        match http_get_path(address, path) {
            Ok(response)
                if response.contains("200 OK")
                    && expected_body
                        .iter()
                        .all(|expected| response.contains(expected)) =>
            {
                return response;
            }
            Ok(response) => last_error = response,
            Err(err) => last_error = err,
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    panic!("server did not answer {path}: {last_error}");
}

fn http_get(address: &str) -> Result<String, String> {
    http_get_path(address, "/ping")
}

fn http_get_path(address: &str, path: &str) -> Result<String, String> {
    let socket = address
        .to_socket_addrs()
        .map_err(|error| error.to_string())?
        .next()
        .ok_or_else(|| format!("no socket address for {address}"))?;
    let mut stream = TcpStream::connect_timeout(&socket, Duration::from_millis(500))
        .map_err(|error| error.to_string())?;
    stream
        .set_read_timeout(Some(Duration::from_secs(1)))
        .map_err(|e| e.to_string())?;
    write!(
        stream,
        "GET {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n"
    )
    .map_err(|e| e.to_string())?;
    let mut response = String::new();
    stream
        .read_to_string(&mut response)
        .map_err(|e| e.to_string())?;
    Ok(response)
}

#[test]
fn dap_attach_runtime_continue_serves_http_and_pause_resumes_transport() {
    let dir = TestDir::new("dap-async-transport");
    let port_reservation = PortReservation::localhost();
    let port = port_reservation.port();
    let source = dir.join("app.orv");
    std::fs::write(
        &source,
        format!(
            r"@server {{
  @listen {port}
  @route GET /ping {{ @respond 200 {{ ok: true }} }}
}}
"
        ),
    )
    .expect("write source");
    let mut dap = DapServer::start();

    let launch = dap.request(&serde_json::json!({
        "seq": 1,
        "type": "request",
        "command": "launch",
        "arguments": {
            "program": format!("file://{}", source.display()),
            "attachRuntime": true,
        },
    }));
    assert_eq!(launch["success"], true, "{launch}");
    assert_eq!(
        launch["body"]["runtime"]["async"]["transport"]["state"],
        "detached"
    );

    drop(port_reservation);
    let continued = dap.request(&serde_json::json!({
        "seq": 2,
        "type": "request",
        "command": "continue",
        "arguments": { "threadId": 1 },
    }));
    assert_eq!(continued["success"], true, "{continued}");
    let address = format!("127.0.0.1:{port}");
    wait_for_http_ok(&address);

    let pause = dap.request(&serde_json::json!({
        "seq": 3,
        "type": "request",
        "command": "pause",
        "arguments": { "threadId": 1 },
    }));
    assert_eq!(pause["success"], true, "{pause}");
    let suspended = dap.request(&serde_json::json!({
        "seq": 4,
        "type": "request",
        "command": "evaluate",
        "arguments": { "expression": "runtimeTransport" },
    }));
    assert_eq!(suspended["success"], true, "{suspended}");
    assert!(
        suspended["body"]["result"]
            .as_str()
            .expect("transport result")
            .starts_with("process suspended pid "),
        "{suspended}"
    );

    let resumed = dap.request(&serde_json::json!({
        "seq": 5,
        "type": "request",
        "command": "continue",
        "arguments": { "threadId": 1 },
    }));
    assert_eq!(resumed["success"], true, "{resumed}");
    wait_for_http_ok(&address);

    let terminated = dap.request(&serde_json::json!({
        "seq": 6,
        "type": "request",
        "command": "terminate",
        "arguments": {},
    }));
    assert_eq!(terminated["success"], true, "{terminated}");
}

#[test]
fn dap_attach_runtime_in_process_reports_request_frames() {
    let dir = TestDir::new("dap-in-process-request-frames");
    let source = dir.join("app.orv");
    std::fs::write(
        &source,
        r"@server {
  @listen 0
  @route GET /users/:id { @respond 200 { id: @param.id, debug: @query.debug } }
}
",
    )
    .expect("write source");
    let mut dap = DapServer::start();

    let launch = dap.request(&serde_json::json!({
        "seq": 21,
        "type": "request",
        "command": "launch",
        "arguments": {
            "program": format!("file://{}", source.display()),
            "attachRuntime": true,
            "attachRuntimeMode": "inProcess",
        },
    }));
    assert_eq!(launch["success"], true, "{launch}");

    let continued = dap.request(&serde_json::json!({
        "seq": 22,
        "type": "request",
        "command": "continue",
        "arguments": { "threadId": 1 },
    }));
    assert_eq!(continued["success"], true, "{continued}");

    let transport = dap.request(&serde_json::json!({
        "seq": 23,
        "type": "request",
        "command": "evaluate",
        "arguments": { "expression": "runtimeTransport" },
    }));
    let address = transport["body"]["result"]
        .as_str()
        .expect("runtime transport result")
        .strip_prefix("in-process running ")
        .expect("running in-process address")
        .to_string();
    let response = wait_for_http_response(
        &address,
        "/users/42?debug=true",
        &["\"id\":\"42\"", "\"debug\":\"true\""],
    );
    let response_origin_id = response
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case("x-orv-response-origin-id")
                .then(|| value.trim().to_string())
        })
        .expect("response origin header");
    let expected_request_summary = format!(
        "GET /users/42 -> 200 route GET /users/:id response {response_origin_id} params id=42 query debug=true"
    );

    let request_count = dap.request(&serde_json::json!({
        "seq": 24,
        "type": "request",
        "command": "evaluate",
        "arguments": { "expression": "runtimeRequestCount" },
    }));
    assert_eq!(request_count["success"], true, "{request_count}");
    assert_eq!(request_count["body"]["result"], "1");

    let last_request = dap.request(&serde_json::json!({
        "seq": 25,
        "type": "request",
        "command": "evaluate",
        "arguments": { "expression": "runtimeLastRequest" },
    }));
    assert_eq!(last_request["success"], true, "{last_request}");
    assert_eq!(last_request["body"]["result"], expected_request_summary);

    let request_frames = dap.request(&serde_json::json!({
        "seq": 26,
        "type": "request",
        "command": "evaluate",
        "arguments": { "expression": "runtimeRequestFrames" },
    }));
    assert_eq!(request_frames["success"], true, "{request_frames}");
    assert_eq!(
        request_frames["body"]["result"],
        format!("#1 {expected_request_summary}")
    );

    let terminated = dap.request(&serde_json::json!({
        "seq": 27,
        "type": "request",
        "command": "terminate",
        "arguments": {},
    }));
    assert_eq!(terminated["success"], true, "{terminated}");
}

#[test]
fn dap_attach_runtime_in_process_serves_http_and_reports_transport() {
    let dir = TestDir::new("dap-in-process-transport");
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
    let mut dap = DapServer::start();

    let launch = dap.request(&serde_json::json!({
        "seq": 11,
        "type": "request",
        "command": "launch",
        "arguments": {
            "program": format!("file://{}", source.display()),
            "attachRuntime": true,
            "attachRuntimeMode": "inProcess",
        },
    }));
    assert_eq!(launch["success"], true, "{launch}");
    assert_eq!(
        launch["body"]["runtime"]["async"]["transport"]["kind"],
        "in-process"
    );
    assert_eq!(
        launch["body"]["runtime"]["async"]["transport"]["state"],
        "detached"
    );

    let continued = dap.request(&serde_json::json!({
        "seq": 12,
        "type": "request",
        "command": "continue",
        "arguments": { "threadId": 1 },
    }));
    assert_eq!(continued["success"], true, "{continued}");

    let transport = dap.request(&serde_json::json!({
        "seq": 13,
        "type": "request",
        "command": "evaluate",
        "arguments": { "expression": "runtimeTransport" },
    }));
    assert_eq!(transport["success"], true, "{transport}");
    let address = transport["body"]["result"]
        .as_str()
        .expect("runtime transport result")
        .strip_prefix("in-process running ")
        .expect("running in-process address");
    assert!(address.starts_with("127.0.0.1:"), "{transport}");
    wait_for_http_ok(address);

    let terminated = dap.request(&serde_json::json!({
        "seq": 14,
        "type": "request",
        "command": "terminate",
        "arguments": {},
    }));
    assert_eq!(terminated["success"], true, "{terminated}");
}
