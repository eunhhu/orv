use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;

const TRACE_ROOT_KEYS: &[&str] = &[
    "action_count",
    "actions",
    "build_dir",
    "frames",
    "kind",
    "live_refresh",
    "schema_version",
    "stream_runner",
    "trace",
];
const TRACE_META_KEYS: &[&str] = &["frame_count", "kind", "path", "status_counts"];
const STATUS_COUNTS_KEYS: &[&str] = &[
    "client_error",
    "ok",
    "other",
    "redirect",
    "server_error",
    "total",
];
const LIVE_REFRESH_KEYS: &[&str] = &["strategy", "transport", "watch"];
const TRACE_WATCH_KEYS: &[&str] = &["trace"];
const STREAM_WATCH_KEYS: &[&str] = &["event_stream"];
const WATCH_TARGET_KEYS: &[&str] = &["content_hash", "path"];
const TRANSPORT_KEYS: &[&str] = &["event", "kind", "url"];
const STREAM_RUNNER_KEYS: &[&str] = &[
    "command",
    "event_stream",
    "kind",
    "schema_version",
    "transport",
];
const TRACE_FRAME_KEYS: &[&str] = &[
    "actions",
    "commerce_adapter_origin_id",
    "commerce_navigation",
    "commerce_reveal_command",
    "db_navigation",
    "db_operation_origin_id",
    "db_reveal_command",
    "index",
    "navigation",
    "origin_id",
    "request",
    "response_navigation",
    "response_origin_id",
    "response_reveal_command",
    "reveal_command",
    "summary",
];
const TRACE_SUMMARY_KEYS: &[&str] = &[
    "commerce_adapter_origin_id",
    "db_operation_origin_id",
    "label",
    "origin_id",
    "response_origin_id",
    "route",
    "status",
    "status_class",
];
const TRACE_ACTION_KEYS: &[&str] = &[
    "action",
    "command",
    "focus",
    "frame_index",
    "kind",
    "label",
    "navigation",
    "origin_id",
    "production",
    "runner_command",
    "schema_version",
    "slot",
    "source",
    "source_line",
    "source_path",
    "target_panel",
];
const TRACE_STREAM_ROOT_KEYS: &[&str] = &[
    "build_dir",
    "event_stream",
    "events",
    "kind",
    "latest",
    "schema_version",
];
const EVENT_STREAM_KEYS: &[&str] = &[
    "content_hash",
    "content_type",
    "event_count",
    "path",
    "trace_event_count",
    "trace_frame_event_count",
];
const TRACE_FRAME_EVENT_KEYS: &[&str] = &["data_bytes", "event", "frame", "index"];
const NATIVE_TRACE_KEYS: &[&str] = &[
    "action_count",
    "action_result_artifact",
    "action_runner",
    "actions",
    "build_dir",
    "frame_count",
    "frames",
    "kind",
    "live_refresh",
    "panel_artifact",
    "panel_contract",
    "panel_html_path",
    "schema_version",
    "status_counts",
    "status_filters",
    "stream_runner",
    "summary",
    "trace_path",
    "transport",
];
const PANEL_ENTRY_KEYS: &[&str] = &["artifact", "name", "panel_contract", "root", "title"];
const ACTION_RESULT_ROOT_KEYS: &[&str] = &[
    "action",
    "command",
    "execution",
    "input",
    "kind",
    "navigation",
    "panels",
    "result_artifact",
    "schema_version",
];
const ACTION_EXECUTION_KEYS: &[&str] = &["allowlist", "kind", "status"];
const TRACE_ACTION_PANEL_KEYS: &[&str] = &[
    "action",
    "command",
    "navigation",
    "production",
    "result_artifact",
    "schema_version",
    "source",
    "summary",
];

struct TraceFixture {
    root: PathBuf,
    source_arg: String,
    build_arg: String,
    trace_arg: String,
    route_id: String,
    response_id: String,
    db_operation_id: String,
    payment_id: String,
}

#[test]
fn editor_trace_v1_freezes_trace_stream_and_action_envelopes() {
    let fixture = build_trace_fixture();

    let trace = run_orv_json(&[
        "editor",
        "trace",
        &fixture.build_arg,
        "--trace",
        &fixture.trace_arg,
    ]);
    assert_editor_trace_contract(&trace, &fixture);

    let events = fixture.root.join("trace-events.sse");
    write_trace_frame_events(&events, &fixture);
    let stream = run_orv_json(&[
        "editor",
        "trace-stream",
        &fixture.build_arg,
        "--events",
        &path_arg(&events),
    ]);
    assert_trace_stream_contract(&stream);

    let export_dir = fixture.root.join("editor");
    run_orv(&[
        "editor",
        "export",
        &fixture.source_arg,
        "--out",
        &path_arg(&export_dir),
        "--build",
        &fixture.build_arg,
        "--trace",
        &fixture.trace_arg,
    ]);
    let native_host = read_json(&export_dir.join("native-host.json"));
    assert_native_host_trace_contract(&native_host, &export_dir);

    let action = run_orv_json(&[
        "editor",
        "run-action",
        &path_arg(&export_dir),
        "--action",
        "trace.response.reveal",
        "--frame-index",
        "0",
        "--slot",
        "response",
    ]);
    assert_action_result_contract(&action, &export_dir, &fixture);

    let _ = std::fs::remove_dir_all(fixture.root);
}

#[test]
fn editor_run_action_rejects_stale_direct_reveal_action_schema_version() {
    let fixture = build_trace_fixture();
    let action_path = fixture.root.join("stale-action.json");
    std::fs::write(
        &action_path,
        serde_json::to_vec_pretty(&serde_json::json!({
            "schema_version": 0,
            "kind": "orv.editor.native_host.reveal_action",
            "action": "trace.response.reveal",
            "slot": "response",
            "frame_index": 0,
            "origin_id": fixture.response_id,
            "command": ["orv", "editor", "reveal", fixture.build_arg, fixture.response_id],
        }))
        .expect("action json"),
    )
    .expect("write stale action");

    let output = Command::new(orv_bin())
        .args([
            "editor",
            "run-action",
            &path_arg(&action_path),
            "--action",
            "trace.response.reveal",
        ])
        .output()
        .expect("run orv editor run-action");

    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("native-host reveal action schema_version must be 1"),
        "{stderr}"
    );

    let _ = std::fs::remove_dir_all(fixture.root);
}

fn build_trace_fixture() -> TraceFixture {
    let root = temp_dir("editor-trace-contract");
    std::fs::create_dir_all(&root).expect("create temp dir");
    let source = root.join("app.orv");
    let build = root.join("dist");
    let trace = root.join("trace.json");
    std::fs::write(
        &source,
        r#"@server {
  @listen 8080
  let shopdb = @db.connect(@env.SHOP_DATABASE_URL ?? "postgres://db.internal/shop")
  let payments = @payment.connect(@env.PAYMENT_ADAPTER_URL ?? "http://payments.internal/capture")
  @route POST /checkout {
    let order = await shopdb.create("Order", { id: "o_1", total: 42 })
    let captured = payments.capture({ orderId: order.id, amount: 42, method: "card" })
    @respond 200 { payment: captured.status }
  }
}
"#,
    )
    .expect("write source");
    let source_arg = path_arg(&source);
    let build_arg = path_arg(&build);
    run_orv(&["build", &source_arg, "--prod", "--out", &build_arg]);

    let origin_map = read_json(&build.join("origin-map.json"));
    let route_id = origin_id(&origin_map, "route", "POST /checkout");
    let response_id = origin_id(&origin_map, "domain", "respond");
    let db_operation_id = origin_id(&origin_map, "call", "shopdb.create");
    let payment_id = origin_id(&origin_map, "call", "@payment.connect");
    write_trace_json(
        &trace,
        &route_id,
        &response_id,
        &db_operation_id,
        &payment_id,
    );

    TraceFixture {
        root,
        source_arg,
        build_arg,
        trace_arg: path_arg(&trace),
        route_id,
        response_id,
        db_operation_id,
        payment_id,
    }
}

fn write_trace_json(
    path: &Path,
    route_id: &str,
    response_id: &str,
    db_operation_id: &str,
    payment_id: &str,
) {
    let frame = trace_frame_json(route_id, response_id, db_operation_id, payment_id);
    std::fs::write(
        path,
        serde_json::to_vec_pretty(&serde_json::json!({
            "schema_version": 1,
            "kind": "orv.production.trace",
            "frames": [frame],
        }))
        .expect("trace json"),
    )
    .expect("write trace");
}

fn write_trace_frame_events(path: &Path, fixture: &TraceFixture) {
    let frame = trace_frame_json(
        &fixture.route_id,
        &fixture.response_id,
        &fixture.db_operation_id,
        &fixture.payment_id,
    );
    let event = serde_json::json!({
        "schema_version": 1,
        "kind": "orv.production.trace.frame",
        "index": 0,
        "frame": frame,
    });
    std::fs::write(
        path,
        format!(
            "event: orv:trace.frame\ndata: {}\n\n",
            serde_json::to_string(&event).expect("event json")
        ),
    )
    .expect("write trace frame events");
}

fn trace_frame_json(
    route_id: &str,
    response_id: &str,
    db_operation_id: &str,
    payment_id: &str,
) -> Value {
    serde_json::json!({
        "method": "POST",
        "path": "/checkout",
        "status": 200,
        "route_origin_id": route_id,
        "response_origin_id": response_id,
        "db_operation_origin_id": db_operation_id,
        "commerce_adapter_origin_id": payment_id,
    })
}

fn assert_editor_trace_contract(trace: &Value, fixture: &TraceFixture) {
    assert_object_keys(trace, TRACE_ROOT_KEYS);
    assert_eq!(trace["schema_version"], 1);
    assert_eq!(trace["kind"], "orv.editor.trace");
    assert_eq!(trace["build_dir"], fixture.build_arg);
    assert_object_keys(&trace["trace"], TRACE_META_KEYS);
    assert_eq!(trace["trace"]["frame_count"], 1);
    assert_object_keys(&trace["trace"]["status_counts"], STATUS_COUNTS_KEYS);
    assert_eq!(trace["trace"]["status_counts"]["total"], 1);
    assert_eq!(trace["trace"]["status_counts"]["ok"], 1);

    assert_trace_live_refresh(&trace["live_refresh"], "trace-file-hash");
    assert_object_keys(&trace["stream_runner"], STREAM_RUNNER_KEYS);
    assert_eq!(
        trace["stream_runner"]["command"],
        serde_json::json!([
            "orv",
            "editor",
            "trace-stream",
            fixture.build_arg,
            "--events",
            "trace/events.sse"
        ])
    );

    let frame = trace["frames"]
        .as_array()
        .expect("trace frames")
        .first()
        .expect("trace frame");
    assert_trace_frame_contract(frame, fixture);
    assert_eq!(trace["action_count"], 4);
    let actions = trace["actions"].as_array().expect("trace actions");
    assert_eq!(actions.len(), 4);
    assert_trace_action_contract(
        find_action(actions, "trace.response.reveal"),
        "response",
        &fixture.response_id,
    );
}

fn assert_trace_frame_contract(frame: &Value, fixture: &TraceFixture) {
    assert_object_keys(frame, TRACE_FRAME_KEYS);
    assert_eq!(frame["index"], 0);
    assert_eq!(frame["origin_id"], fixture.route_id);
    assert_eq!(frame["response_origin_id"], fixture.response_id);
    assert_eq!(frame["db_operation_origin_id"], fixture.db_operation_id);
    assert_eq!(frame["commerce_adapter_origin_id"], fixture.payment_id);
    assert_object_keys(&frame["summary"], TRACE_SUMMARY_KEYS);
    assert_eq!(frame["summary"]["label"], "POST /checkout -> 200");
    assert_eq!(frame["summary"]["status_class"], "ok");
    assert_eq!(
        frame["response_reveal_command"],
        serde_json::json!([
            "orv",
            "editor",
            "reveal",
            fixture.build_arg,
            fixture.response_id
        ])
    );
    assert!(frame["response_navigation"]["source"]["snippet"]
        .as_str()
        .is_some_and(|snippet| snippet.contains("@respond 200")));
    let actions = frame["actions"].as_array().expect("frame actions");
    assert_eq!(actions.len(), 4);
    assert_trace_action_contract(
        find_action(actions, "trace.db.reveal"),
        "db",
        &fixture.db_operation_id,
    );
}

fn assert_trace_action_contract(action: &Value, slot: &str, origin_id: &str) {
    assert_object_keys(action, TRACE_ACTION_KEYS);
    assert_eq!(action["schema_version"], 1);
    assert_eq!(action["kind"], "orv.editor.native_host.reveal_action");
    assert_eq!(action["slot"], slot);
    assert_eq!(action["origin_id"], origin_id);
    assert_eq!(action["frame_index"], 0);
    assert!(action["command"].as_array().is_some());
    assert!(action["runner_command"].as_array().is_some());
    assert!(action["source_path"].as_str().is_some());
}

fn assert_trace_stream_contract(stream: &Value) {
    assert_object_keys(stream, TRACE_STREAM_ROOT_KEYS);
    assert_eq!(stream["schema_version"], 1);
    assert_eq!(stream["kind"], "orv.editor.trace.stream");
    assert_object_keys(&stream["event_stream"], EVENT_STREAM_KEYS);
    assert_eq!(stream["event_stream"]["content_type"], "text/event-stream");
    assert_eq!(stream["event_stream"]["event_count"], 1);
    assert_eq!(stream["event_stream"]["trace_frame_event_count"], 1);
    let event = stream["events"]
        .as_array()
        .expect("stream events")
        .first()
        .expect("stream event");
    assert_object_keys(event, TRACE_FRAME_EVENT_KEYS);
    assert_eq!(event["event"], "orv:trace.frame");
    assert_object_keys(&stream["latest"], TRACE_ROOT_KEYS);
    assert_trace_live_refresh(&stream["latest"]["live_refresh"], "event-source-snapshot");
}

fn assert_native_host_trace_contract(native_host: &Value, export_dir: &Path) {
    let trace = &native_host["trace"];
    assert_object_keys(trace, NATIVE_TRACE_KEYS);
    assert_eq!(trace["schema_version"], 1);
    assert_eq!(trace["kind"], "orv.editor.native_host.trace");
    assert_eq!(trace["frame_count"], 1);
    assert_eq!(trace["action_count"], 4);
    assert_eq!(trace["panel_html_path"], "trace/panel.html");
    assert_eq!(
        trace["action_result_artifact"]["path"],
        "trace/action-result.json"
    );
    assert_eq!(
        trace["action_result_artifact"]["html_path"],
        "trace/action-result.html"
    );
    assert_eq!(trace["panel_contract"]["root"], "trace");

    let panels = native_host["panels"].as_array().expect("native panels");
    assert_panel_entry(find_panel(panels, "trace"));
    assert_panel_entry(find_panel(panels, "trace_action_result"));
    assert_eq!(native_host["capabilities"]["trace_navigation"], true);
    assert_eq!(native_host["capabilities"]["trace_reveal_actions"], true);
    assert!(export_dir.join("trace/panel.html").is_file());
    let trace_panel =
        std::fs::read_to_string(export_dir.join("trace/panel.html")).expect("trace panel");
    assert!(trace_panel.contains("Trace Panel"));
    assert!(trace_panel.contains("Panel Contract"));
}

fn assert_action_result_contract(action: &Value, export_dir: &Path, fixture: &TraceFixture) {
    assert_object_keys(action, ACTION_RESULT_ROOT_KEYS);
    assert_eq!(action["schema_version"], 1);
    assert_eq!(action["kind"], "orv.editor.native_host.action.result");
    assert_object_keys(&action["execution"], ACTION_EXECUTION_KEYS);
    assert_eq!(action["execution"]["allowlist"], "orv.editor.reveal");
    assert_eq!(action["execution"]["status"], "passed");
    assert_eq!(action["action"]["action"], "trace.response.reveal");
    assert_eq!(action["action"]["origin_id"], fixture.response_id);
    assert_eq!(
        action["command"],
        serde_json::json!([
            "orv",
            "editor",
            "reveal",
            fixture.build_arg,
            fixture.response_id
        ])
    );
    assert_eq!(action["navigation"]["origin"]["id"], fixture.response_id);
    assert_object_keys(&action["panels"]["trace_action"], TRACE_ACTION_PANEL_KEYS);
    assert_eq!(
        action["panels"]["trace_action"]["summary"]["status"],
        "passed"
    );
    assert!(export_dir.join("trace/action-result.json").is_file());
    assert!(export_dir.join("trace/action-result.html").is_file());
}

fn assert_trace_live_refresh(refresh: &Value, strategy: &str) {
    assert_object_keys(refresh, LIVE_REFRESH_KEYS);
    assert_eq!(refresh["strategy"], strategy);
    assert_object_keys(&refresh["transport"], TRANSPORT_KEYS);
    assert_eq!(refresh["transport"]["kind"], "event-source");
    assert_eq!(refresh["transport"]["event"], "orv:trace");
    let watch_key = if strategy == "trace-file-hash" {
        assert_object_keys(&refresh["watch"], TRACE_WATCH_KEYS);
        "trace"
    } else {
        assert_object_keys(&refresh["watch"], STREAM_WATCH_KEYS);
        "event_stream"
    };
    assert_object_keys(&refresh["watch"][watch_key], WATCH_TARGET_KEYS);
    assert!(refresh["watch"][watch_key]["content_hash"]
        .as_str()
        .is_some_and(|hash| hash.starts_with("fnv1a64:")));
}

fn assert_panel_entry(panel: &Value) {
    assert_object_keys(panel, PANEL_ENTRY_KEYS);
}

fn find_action<'a>(actions: &'a [Value], action_id: &str) -> &'a Value {
    actions
        .iter()
        .find(|action| action["action"] == action_id)
        .unwrap_or_else(|| panic!("missing action {action_id}"))
}

fn find_panel<'a>(panels: &'a [Value], name: &str) -> &'a Value {
    panels
        .iter()
        .find(|panel| panel["name"] == name)
        .unwrap_or_else(|| panic!("missing panel {name}"))
}

fn assert_object_keys(value: &Value, expected: &[&str]) {
    let object = value.as_object().expect("object");
    let actual = object.keys().map(String::as_str).collect::<BTreeSet<_>>();
    let expected = expected.iter().copied().collect::<BTreeSet<_>>();
    assert_eq!(actual, expected);
}

fn origin_id(origin_map: &Value, kind: &str, name: &str) -> String {
    origin_map["entries"]
        .as_array()
        .expect("origin entries")
        .iter()
        .find(|entry| entry["kind"] == kind && entry["name"] == name)
        .unwrap_or_else(|| panic!("missing origin {kind}:{name}"))
        .get("id")
        .and_then(Value::as_str)
        .expect("origin id")
        .to_string()
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

fn path_arg(path: &Path) -> String {
    path.display().to_string()
}

fn run_orv_json(args: &[&str]) -> Value {
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
    serde_json::from_slice(&output.stdout).expect("orv json")
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

fn read_json(path: &Path) -> Value {
    serde_json::from_str(&std::fs::read_to_string(path).expect("read json")).expect("json")
}
