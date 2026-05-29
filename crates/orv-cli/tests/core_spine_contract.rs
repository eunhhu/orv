use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const CORE_SPINE_GOLDEN: &str = include_str!("../../../docs/samples/core-spine-v1.golden.json");
const CORE_SPINE_SOURCE: &str =
    "@server { @listen 0 @route GET /ping { @respond 200 { ok: true } } }\n";

struct DapServer {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

impl Drop for DapServer {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

struct RuntimeTraceEvidence {
    http_response: String,
    trace: serde_json::Value,
}

fn temp_dir(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time")
        .as_nanos();
    std::env::temp_dir().join(format!("orv-cli-{name}-{}-{nanos}", std::process::id()))
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

#[test]
fn core_spine_v1_freezes_route_origin_through_runtime_trace() {
    let root = temp_dir("core-spine-contract");
    std::fs::create_dir_all(&root).expect("create temp dir");
    let source = root.join("app.orv");
    let build = root.join("dist");
    std::fs::write(&source, CORE_SPINE_SOURCE).expect("write source");
    let source_arg = source.display().to_string();
    let build_arg = build.display().to_string();

    let origins = run_orv_json(&["origins", &source_arg]);
    let graph = run_orv_json(&["graph", &source_arg]);
    run_orv(&["build", &source_arg, "--out", &build_arg]);
    let build_origin_map = read_json(&build.join("origin-map.json"));
    let build_graph = read_json(&build.join("project-graph.json"));

    let runtime = live_runtime_trace(&source);
    let trace_path = root.join("runtime-trace.json");
    std::fs::write(
        &trace_path,
        serde_json::to_vec_pretty(&runtime.trace).expect("trace json"),
    )
    .expect("write runtime trace");
    let trace_arg = trace_path.display().to_string();
    let editor_trace = run_orv_json(&["editor", "trace", &build_arg, "--trace", &trace_arg]);

    let actual = core_spine_inventory(
        &origins,
        &graph,
        &build_origin_map,
        &build_graph,
        &runtime.http_response,
        &runtime.trace,
        &editor_trace,
    );
    let expected: serde_json::Value =
        serde_json::from_str(CORE_SPINE_GOLDEN).expect("core spine golden");
    assert_eq!(actual, expected, "Core Spine v1 golden drift");

    let _ = std::fs::remove_dir_all(root);
}

fn live_runtime_trace(source: &Path) -> RuntimeTraceEvidence {
    let mut dap = start_dap();
    let program = format!("file://{}", source.display());
    let launch = dap_response(
        &mut dap,
        &serde_json::json!({
            "seq": 1,
            "type": "request",
            "command": "launch",
            "arguments": {
                "program": program,
                "attachRuntime": true,
                "attachRuntimeMode": "inProcess",
            },
        }),
    );
    assert_eq!(launch["success"], true, "{launch}");

    let continued = dap_response(
        &mut dap,
        &serde_json::json!({
            "seq": 2,
            "type": "request",
            "command": "continue",
            "arguments": { "threadId": 1 },
        }),
    );
    assert_eq!(continued["success"], true, "{continued}");

    let transport = dap_response(
        &mut dap,
        &serde_json::json!({
            "seq": 3,
            "type": "request",
            "command": "evaluate",
            "arguments": { "expression": "runtimeTransport" },
        }),
    );
    assert_eq!(transport["success"], true, "{transport}");
    let address = transport["body"]["result"]
        .as_str()
        .expect("runtime transport")
        .strip_prefix("in-process running ")
        .expect("in-process runtime address")
        .to_string();

    let http_response = wait_for_http_ok(&address);
    let trace = dap_response(
        &mut dap,
        &serde_json::json!({
            "seq": 4,
            "type": "request",
            "command": "evaluate",
            "arguments": { "expression": "runtimeRequestTrace" },
        }),
    );
    assert_eq!(trace["success"], true, "{trace}");
    let trace = serde_json::from_str(trace["body"]["result"].as_str().expect("trace result"))
        .expect("runtime trace json");

    let terminated = dap_response(
        &mut dap,
        &serde_json::json!({
            "seq": 5,
            "type": "request",
            "command": "terminate",
            "arguments": {},
        }),
    );
    assert_eq!(terminated["success"], true, "{terminated}");

    RuntimeTraceEvidence {
        http_response,
        trace,
    }
}

fn start_dap() -> DapServer {
    let mut child = Command::new(orv_bin())
        .args(["dap", "serve", "--stdio"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn dap server");
    let stdin = child.stdin.take().expect("dap stdin");
    let stdout = BufReader::new(child.stdout.take().expect("dap stdout"));
    DapServer {
        child,
        stdin,
        stdout,
    }
}

fn dap_response(server: &mut DapServer, request: &serde_json::Value) -> serde_json::Value {
    let request_seq = request["seq"].as_u64().expect("request seq");
    let body = serde_json::to_vec(request).expect("serialize request");
    write!(server.stdin, "Content-Length: {}\r\n\r\n", body.len()).expect("write header");
    server.stdin.write_all(&body).expect("write body");
    server.stdin.flush().expect("flush request");

    loop {
        let frame = read_dap_frame(&mut server.stdout);
        if frame["type"] == "response" && frame["request_seq"] == request_seq {
            return frame;
        }
    }
}

fn read_dap_frame(stdout: &mut BufReader<ChildStdout>) -> serde_json::Value {
    let mut content_length = None;
    loop {
        let mut line = String::new();
        stdout.read_line(&mut line).expect("read DAP header");
        let header = line.trim_end_matches('\n').trim_end_matches('\r');
        if header.is_empty() {
            break;
        }
        if let Some((name, value)) = header.split_once(':') {
            if name.eq_ignore_ascii_case("Content-Length") {
                content_length = Some(value.trim().parse::<usize>().expect("content length"));
            }
        }
    }
    let length = content_length.expect("content length header");
    let mut body = vec![0_u8; length];
    stdout.read_exact(&mut body).expect("read DAP body");
    serde_json::from_slice(&body).expect("DAP frame json")
}

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

fn http_get(address: &str) -> Result<String, String> {
    let addr = address
        .to_socket_addrs()
        .map_err(|err| err.to_string())?
        .next()
        .ok_or_else(|| format!("no socket address for {address}"))?;
    let mut stream = TcpStream::connect_timeout(&addr, Duration::from_millis(500))
        .map_err(|err| err.to_string())?;
    stream
        .set_read_timeout(Some(Duration::from_secs(1)))
        .map_err(|err| err.to_string())?;
    write!(
        stream,
        "GET /ping HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n"
    )
    .map_err(|err| err.to_string())?;
    let mut response = String::new();
    stream
        .read_to_string(&mut response)
        .map_err(|err| err.to_string())?;
    Ok(response)
}

fn core_spine_inventory(
    origins: &serde_json::Value,
    graph: &serde_json::Value,
    build_origin_map: &serde_json::Value,
    build_graph: &serde_json::Value,
    http_response: &str,
    runtime_trace: &serde_json::Value,
    editor_trace: &serde_json::Value,
) -> serde_json::Value {
    assert_eq!(
        build_origin_map, origins,
        "build origin-map must match CLI origins"
    );
    assert_eq!(
        build_graph["semantic"]["origin_map"], *origins,
        "build ProjectGraph must embed the same OriginMap"
    );
    assert_eq!(
        graph["semantic"]["origin_map"], *origins,
        "CLI ProjectGraph must embed the same OriginMap"
    );

    let route = origin_entry(origins, "route", "GET /ping");
    let respond = origin_entry(origins, "domain", "respond");
    let route_id = route["id"].as_str().expect("route id");
    let respond_id = respond["id"].as_str().expect("respond id");
    let route_link = origin_link(graph, route_id);
    let respond_link = origin_link(graph, respond_id);
    let route_node = graph_node(
        graph,
        route_link["node_id"].as_u64().expect("route node id"),
    );
    let respond_node = graph_node(
        graph,
        respond_link["node_id"].as_u64().expect("respond node id"),
    );
    let frame = &runtime_trace["frames"].as_array().expect("trace frames")[0];

    assert_eq!(response_header(http_response, "x-orv-origin-id"), route_id);
    assert_eq!(
        response_header(http_response, "x-orv-response-origin-id"),
        respond_id
    );
    assert_eq!(frame["route_origin_id"], route_id);
    assert_eq!(frame["response_origin_id"], respond_id);
    assert_eq!(editor_trace["frames"][0]["origin_id"], route_id);
    assert_eq!(editor_trace["frames"][0]["response_origin_id"], respond_id);
    assert!(editor_trace["frames"][0]["navigation"]["source"]["snippet"]
        .as_str()
        .is_some_and(|snippet| snippet.contains("@route GET /ping")));
    assert!(
        editor_trace["frames"][0]["response_navigation"]["source"]["snippet"]
            .as_str()
            .is_some_and(|snippet| snippet.contains("@respond 200"))
    );

    serde_json::json!({
        "schema_version": 1,
        "kind": "orv.core_spine",
        "source": {
            "fixture": "<fixture>/app.orv",
            "byte_len": CORE_SPINE_SOURCE.len(),
        },
        "origin_map": {
            "version": origins["version"],
            "entry_count": origins["entries"].as_array().expect("origin entries").len(),
            "edge_count": origins["edges"].as_array().expect("origin edges").len(),
            "route_to_response_contains": origin_edge(origins, route_id, respond_id, "contains"),
        },
        "project_graph": {
            "schema_version": graph["schema_version"],
            "node_count": graph["stats"]["node_count"],
            "semantic_origin_count": graph["stats"]["semantic_origin_count"],
            "semantic_edge_count": graph["stats"]["semantic_edge_count"],
            "route_origin_link_kind": route_link["kind"],
            "response_origin_link_kind": respond_link["kind"],
        },
        "route": {
            "name": route["name"],
            "origin_id": route_id,
            "origin_span": route["span"],
            "project_node_id": route_node["id"],
            "project_node_kind": route_node["kind"],
            "project_node_name": route_node["name"],
            "project_node_span": route_node["span"],
        },
        "response": {
            "name": respond["name"],
            "origin_id": respond_id,
            "origin_span": respond["span"],
            "project_node_id": respond_node["id"],
            "project_node_kind": respond_node["kind"],
            "project_node_name": respond_node["name"],
            "project_node_span": respond_node["span"],
        },
        "runtime_event": {
            "http_status": 200,
            "header_origin_id": route_id,
            "header_response_origin_id": respond_id,
            "trace_kind": runtime_trace["kind"],
            "trace_frame_count": runtime_trace["frame_count"],
            "trace_method": frame["method"],
            "trace_path": frame["path"],
            "trace_route_method": frame["route_method"],
            "trace_route_path": frame["route_path"],
            "trace_status": frame["status"],
            "trace_origin_id": frame["route_origin_id"],
            "trace_response_origin_id": frame["response_origin_id"],
        },
        "editor_trace": {
            "kind": editor_trace["kind"],
            "frame_count": editor_trace["trace"]["frame_count"],
            "frame_origin_id": editor_trace["frames"][0]["origin_id"],
            "frame_response_origin_id": editor_trace["frames"][0]["response_origin_id"],
            "navigation_panel": editor_trace["frames"][0]["navigation"]["focus"]["panel"],
            "response_navigation_panel": editor_trace["frames"][0]["response_navigation"]["focus"]["panel"],
        },
        "artifact_alignment": {
            "cli_graph_embeds_origin_map": graph["semantic"]["origin_map"] == *origins,
            "build_origin_map_matches_cli": build_origin_map == origins,
            "build_graph_embeds_origin_map": build_graph["semantic"]["origin_map"] == *origins,
        },
    })
}

fn origin_entry<'a>(
    origin_map: &'a serde_json::Value,
    kind: &str,
    name: &str,
) -> &'a serde_json::Value {
    origin_map["entries"]
        .as_array()
        .expect("origin entries")
        .iter()
        .find(|entry| entry["kind"] == kind && entry["name"] == name)
        .unwrap_or_else(|| panic!("missing origin {kind}:{name}"))
}

fn origin_edge(origin_map: &serde_json::Value, from: &str, to: &str, kind: &str) -> bool {
    origin_map["edges"]
        .as_array()
        .expect("origin edges")
        .iter()
        .any(|edge| edge["from"] == from && edge["to"] == to && edge["kind"] == kind)
}

fn origin_link<'a>(graph: &'a serde_json::Value, origin_id: &str) -> &'a serde_json::Value {
    graph["semantic"]["origin_links"]
        .as_array()
        .expect("origin links")
        .iter()
        .find(|link| link["kind"] == "source_node" && link["origin_id"] == origin_id)
        .unwrap_or_else(|| panic!("missing origin link for {origin_id}"))
}

fn graph_node(graph: &serde_json::Value, node_id: u64) -> &serde_json::Value {
    graph["nodes"]
        .as_array()
        .expect("graph nodes")
        .iter()
        .find(|node| node["id"] == node_id)
        .unwrap_or_else(|| panic!("missing graph node {node_id}"))
}

fn response_header(response: &str, name: &str) -> String {
    response
        .lines()
        .find_map(|line| {
            let (header_name, value) = line.split_once(':')?;
            header_name
                .eq_ignore_ascii_case(name)
                .then(|| value.trim().to_string())
        })
        .unwrap_or_else(|| panic!("missing response header {name}"))
}
