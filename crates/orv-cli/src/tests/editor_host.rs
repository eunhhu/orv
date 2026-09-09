use super::*;

#[test]
fn editor_native_host_bridge_post_runs_trace_action() {
    let (src_dir, path) = prod_server_source("editor-host-bridge-trace-action-source");
    let build_out = temp_output_dir("editor-host-bridge-trace-action-build");

    cmd_build_with_profile(&path, &build_out, BuildProfile::Production).expect("prod build");
    let origin_map: orv_compiler::OriginMap = serde_json::from_str(
        &std::fs::read_to_string(build_out.join("origin-map.json")).expect("origin map"),
    )
    .expect("origin map json");
    let route = origin_map
        .entries
        .iter()
        .find(|entry| entry.kind == "route" && entry.name == "GET /ping")
        .expect("route origin");
    let trace_path = src_dir.join("production-trace.json");
    write_json(
        &trace_path,
        &serde_json::json!({
            "schema_version": 1,
            "kind": "orv.production.trace",
            "frame_count": 1,
            "frames": [editor_trace_test_route_frame(&route.id)],
        }),
    )
    .expect("write trace");
    let editor_out = src_dir.join("editor");
    cmd_editor_export_with_options(&path, &editor_out, Some(&build_out), Some(&trace_path))
        .expect("editor export with trace action");
    let native_host =
        read_json_value(&editor_out.join(EDITOR_NATIVE_HOST_MANIFEST_PATH)).expect("native host");
    let action = native_host["trace"]["actions"][0].clone();
    let payload = serde_json::json!({
        "kind": "orv.editor.native_host.command",
        "action": action,
        "command": [
            "orv",
            "editor",
            "run-action",
            "native-host.json",
            "--action",
            "trace.route.reveal",
            "--frame-index",
            "0",
            "--slot",
            "route",
        ],
        "refresh": {
            "event": "orv:trace-action-result",
            "panel": "trace_action_result",
            "json": EDITOR_TRACE_ACTION_RESULT_PATH,
            "html": EDITOR_TRACE_ACTION_RESULT_HTML_PATH,
        },
    });
    let payload_bytes = serde_json::to_vec(&payload).expect("payload json");

    let response = editor_native_host_bridge_http_response(
        &editor_out,
        "POST",
        "/__orv/native-host/action",
        &payload_bytes,
    );

    assert_eq!(response.status, 200);
    assert_eq!(response.content_type, "application/json; charset=utf-8");
    let body: serde_json::Value =
        serde_json::from_slice(&response.body).expect("bridge response json");
    assert_eq!(
        body["kind"],
        "orv.editor.native_host.bridge.action.response"
    );
    assert_eq!(body["status"], "passed");
    assert_eq!(body["refresh"]["event"], "orv:trace-action-result");
    assert_eq!(
        body["result"]["kind"],
        "orv.editor.native_host.action.result"
    );
    assert_eq!(body["result"]["action"]["slot"], "route");
    assert_eq!(body["result"]["action"]["origin_id"], route.id);
    assert_eq!(body["result"]["navigation"]["focus"]["panel"], "routes");
    assert!(editor_out.join(EDITOR_TRACE_ACTION_RESULT_PATH).is_file());
    assert!(editor_out
        .join(EDITOR_TRACE_ACTION_RESULT_HTML_PATH)
        .is_file());
    let bridge =
        editor_native_host_bridge_http_response(&editor_out, "GET", "/native-host/bridge.js", &[]);
    assert_eq!(bridge.status, 200);
    assert_eq!(bridge.content_type, "text/javascript; charset=utf-8");
    let bridge_js = String::from_utf8(bridge.body).expect("bridge utf-8");
    assert!(bridge_js.contains("/__orv/native-host/action"));
    assert!(bridge_js.contains("orv:trace-action-result"));
    let mut drifted_payload = payload.clone();
    drifted_payload["unexpected"] = serde_json::json!("drift");
    let drifted_payload = serde_json::to_vec(&drifted_payload).expect("drifted payload json");
    let drifted_response = editor_native_host_bridge_http_response(
        &editor_out,
        "POST",
        "/__orv/native-host/action",
        &drifted_payload,
    );
    assert_eq!(drifted_response.status, 500);
    let drifted_body: serde_json::Value =
        serde_json::from_slice(&drifted_response.body).expect("bridge error json");
    assert!(
        drifted_body["error"].as_str().is_some_and(
            |error| error.contains("native-host bridge command keys must match contract")
        )
    );
    let mut drifted_action_payload = payload.clone();
    drifted_action_payload["action"]["unexpected"] = serde_json::json!("drift");
    let drifted_action_payload =
        serde_json::to_vec(&drifted_action_payload).expect("drifted action payload json");
    let drifted_action_response = editor_native_host_bridge_http_response(
        &editor_out,
        "POST",
        "/__orv/native-host/action",
        &drifted_action_payload,
    );
    assert_eq!(drifted_action_response.status, 500);
    let drifted_action_body: serde_json::Value =
        serde_json::from_slice(&drifted_action_response.body).expect("bridge error json");
    assert!(drifted_action_body["error"]
        .as_str()
        .is_some_and(|error| error.contains("native-host reveal action keys must match contract")));
    let traversal =
        editor_native_host_bridge_http_response(&editor_out, "GET", "/../native-host.json", &[]);
    assert_eq!(traversal.status, 400);
    let _ = std::fs::remove_dir_all(src_dir);
    let _ = std::fs::remove_dir_all(build_out);
}

#[test]
fn editor_export_native_host_includes_trace_transport_contract() {
    let (src_dir, path) = prod_server_source("editor-export-trace-transport-source");
    let build_out = temp_output_dir("editor-export-trace-transport-build");

    cmd_build_with_profile(&path, &build_out, BuildProfile::Production).expect("prod build");
    let trace_path = src_dir.join("production-trace.json");
    write_json(
        &trace_path,
        &serde_json::json!({
            "schema_version": 1,
            "kind": "orv.production.trace",
            "frame_count": 0,
            "frames": [],
        }),
    )
    .expect("write trace");
    let editor_out = src_dir.join("editor");

    cmd_editor_export_with_options(&path, &editor_out, Some(&build_out), Some(&trace_path))
        .expect("editor export with trace transport");

    let html = std::fs::read_to_string(editor_out.join("index.html")).expect("editor html");
    let native_host =
        read_json_value(&editor_out.join(EDITOR_NATIVE_HOST_MANIFEST_PATH)).expect("native host");
    assert_eq!(native_host["trace"]["kind"], "orv.editor.native_host.trace");
    assert_eq!(
        native_host["trace"]["panel_html_path"],
        EDITOR_TRACE_PANEL_HTML_PATH
    );
    assert_eq!(
        native_host["trace"]["transport"]["url"],
        "http://127.0.0.1:8080/__orv/trace/events"
    );
    assert_eq!(
        native_host["trace"]["stream_runner"]["kind"],
        "orv.editor.native_host.trace_stream_runner"
    );
    assert_eq!(
        native_host["trace"]["stream_runner"]["event_stream"],
        "trace/events.sse"
    );
    assert_eq!(
        native_host["trace"]["stream_runner"]["command"],
        serde_json::json!([
            "orv",
            "editor",
            "trace-stream",
            build_out.display().to_string(),
            "--events",
            "trace/events.sse"
        ])
    );
    assert_eq!(native_host["trace"]["frame_count"], 0);
    assert!(html.contains("Trace Transport"));
    assert!(html.contains("id=\"trace-transport-detail\""));
    assert!(html.contains("Trace Stream Runner"));
    assert!(html.contains("id=\"trace-stream-runner-detail\""));
    assert!(html.contains("renderTraceTransport"));
    let _ = std::fs::remove_dir_all(src_dir);
    let _ = std::fs::remove_dir_all(build_out);
}

#[test]
fn editor_export_native_host_includes_trace_frame_navigation_inventory() {
    let (src_dir, path) = prod_server_source("editor-export-trace-frame-source");
    let build_out = temp_output_dir("editor-export-trace-frame-build");

    cmd_build_with_profile(&path, &build_out, BuildProfile::Production).expect("prod build");
    let origin_map: orv_compiler::OriginMap = serde_json::from_str(
        &std::fs::read_to_string(build_out.join("origin-map.json")).expect("origin map"),
    )
    .expect("origin map json");
    let route = origin_map
        .entries
        .iter()
        .find(|entry| entry.kind == "route" && entry.name == "GET /ping")
        .expect("route origin");
    let response = origin_map
        .entries
        .iter()
        .find(|entry| entry.kind == "domain" && entry.name == "respond")
        .expect("response origin");
    let trace_path = src_dir.join("production-trace.json");
    write_json(
        &trace_path,
        &serde_json::json!({
            "schema_version": 1,
            "kind": "orv.production.trace",
            "frame_count": 2,
            "frames": [
                {
                    "method": "GET",
                    "path": "/ping",
                    "status": 200,
                    "route_method": "GET",
                    "route_path": "/ping",
                    "route_origin_id": route.id,
                    "response_origin_id": response.id,
                    "params": {},
                    "query": {},
                    "body": "",
                },
                {
                    "method": "GET",
                    "path": "/missing",
                    "status": 404,
                    "route_method": serde_json::Value::Null,
                    "route_path": serde_json::Value::Null,
                    "route_origin_id": serde_json::Value::Null,
                    "response_origin_id": serde_json::Value::Null,
                    "params": {},
                    "query": {},
                    "body": "",
                },
            ],
        }),
    )
    .expect("write trace");
    let editor_out = src_dir.join("editor");

    cmd_editor_export_with_options(&path, &editor_out, Some(&build_out), Some(&trace_path))
        .expect("editor export with trace frame inventory");

    let native_host =
        read_json_value(&editor_out.join(EDITOR_NATIVE_HOST_MANIFEST_PATH)).expect("native host");
    let frames = native_host["trace"]["frames"]
        .as_array()
        .expect("native trace frames");
    assert_eq!(
        native_host["trace"]["summary"]["schema_version"],
        serde_json::json!(1)
    );
    assert_eq!(native_host["trace"]["summary"]["frame_count"], 2);
    assert_eq!(
        native_host["trace"]["summary"]["status_counts"]["client_error"],
        1
    );
    assert_eq!(
        native_host["trace"]["summary"]["first_request"]["label"],
        "GET /ping -> 200"
    );
    assert_eq!(
        native_host["trace"]["summary"]["last_request"]["label"],
        "GET /missing -> 404"
    );
    assert_eq!(frames.len(), 2);
    assert_eq!(frames[0]["index"], 0);
    assert_eq!(frames[0]["origin_id"], route.id);
    assert_eq!(frames[0]["response_origin_id"], response.id);
    assert_eq!(frames[0]["summary"]["status_class"], "ok");
    assert_eq!(frames[0]["request"]["path"], "/ping");
    assert_eq!(frames[0]["navigation"]["focus"]["panel"], "routes");
    assert_eq!(frames[0]["source"], frames[0]["navigation"]["source"]);
    assert_eq!(
        frames[0]["production"],
        frames[0]["navigation"]["production"]
    );
    assert_eq!(
        frames[0]["response_source"],
        frames[0]["response_navigation"]["source"]
    );
    assert_eq!(
        frames[0]["reveal_command"],
        serde_json::json!([
            "orv",
            "editor",
            "reveal",
            build_out.display().to_string(),
            route.id
        ])
    );
    assert_eq!(
        frames[0]["response_reveal_command"],
        serde_json::json!([
            "orv",
            "editor",
            "reveal",
            build_out.display().to_string(),
            response.id
        ])
    );
    let actions = frames[0]["actions"]
        .as_array()
        .expect("native trace frame actions");
    assert_eq!(actions.len(), 2);
    assert!(actions.iter().any(|action| action["slot"] == "route"
        && action["action"] == "trace.route.reveal"
        && action["origin_id"] == route.id
        && action["command"] == frames[0]["reveal_command"]
        && action["source"] == frames[0]["source"]
        && action["target_panel"] == "routes"));
    assert!(actions.iter().any(|action| action["slot"] == "response"
        && action["action"] == "trace.response.reveal"
        && action["origin_id"] == response.id
        && action["command"] == frames[0]["response_reveal_command"]
        && action["source"] == frames[0]["response_source"]));
    let trace_actions = native_host["trace"]["actions"]
        .as_array()
        .expect("native trace actions");
    assert_eq!(native_host["trace"]["action_count"], 2);
    assert_eq!(trace_actions.len(), 2);
    assert_eq!(native_host["capabilities"]["trace_reveal_actions"], true);
    assert!(frames[0]["navigation"]["source"]["snippet"]
        .as_str()
        .is_some_and(|snippet| snippet.contains("@route GET /ping")));
    assert!(frames[0]["response_source"]["snippet"]
        .as_str()
        .is_some_and(|snippet| snippet.contains("@respond 200")));
    assert_eq!(frames[1]["summary"]["status_class"], "client_error");
    assert_eq!(frames[1]["navigation"], serde_json::Value::Null);
    assert_eq!(frames[1]["source"], serde_json::Value::Null);
    assert_eq!(frames[1]["production"], serde_json::Value::Null);
    assert_eq!(frames[1]["reveal_command"], serde_json::Value::Null);
    assert_eq!(frames[1]["actions"], serde_json::json!([]));
    let filters = native_host["trace"]["status_filters"]
        .as_array()
        .expect("native trace status filters");
    assert!(filters
        .iter()
        .any(|filter| filter["name"] == "all" && filter["count"] == 2));
    assert!(filters
        .iter()
        .any(|filter| filter["name"] == "client_error" && filter["count"] == 1));
    assert_eq!(native_host["trace"]["panel_contract"]["root"], "trace");
    let sections = native_host["trace"]["panel_contract"]["sections"]
        .as_array()
        .expect("native trace panel sections");
    assert!(sections
        .iter()
        .any(|section| section["name"] == "summary" && section["path"] == "trace.summary"));
    assert!(sections
        .iter()
        .any(|section| section["name"] == "status_filters"
            && section["path"] == "trace.status_filters"));
    assert!(sections
        .iter()
        .any(|section| section["name"] == "frames" && section["path"] == "trace.frames"));
    assert!(sections
        .iter()
        .any(|section| section["name"] == "actions" && section["path"] == "trace.actions"));
    assert!(sections
        .iter()
        .any(|section| section["name"] == "panel_artifact"
            && section["path"] == "trace.panel_artifact"));
    let _ = std::fs::remove_dir_all(src_dir);
    let _ = std::fs::remove_dir_all(build_out);
}

#[test]
fn editor_export_native_host_includes_trace_adapter_reveal_navigation() {
    let dir = temp_output_dir("editor-export-trace-adapter-navigation-source");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let path = dir.join("app.orv");
    std::fs::write(
        &path,
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
    let build_out = dir.join("dist");

    cmd_build_with_profile(&path, &build_out, BuildProfile::Production).expect("prod build");
    let origin_map: orv_compiler::OriginMap = serde_json::from_str(
        &std::fs::read_to_string(build_out.join("origin-map.json")).expect("origin map"),
    )
    .expect("origin map json");
    let route = origin_map
        .entries
        .iter()
        .find(|entry| entry.kind == "route" && entry.name == "POST /checkout")
        .expect("checkout route origin");
    let db_operation = origin_map
        .entries
        .iter()
        .find(|entry| entry.kind == "call" && entry.name == "shopdb.create")
        .expect("db operation origin");
    let commerce_adapter = origin_map
        .entries
        .iter()
        .find(|entry| entry.kind == "call" && entry.name == "@payment.connect")
        .expect("commerce adapter origin");
    let trace_path = dir.join("production-trace.json");
    let mut frame = editor_trace_test_frame_for("POST", "/checkout", 200);
    frame["route_method"] = serde_json::json!("POST");
    frame["route_path"] = serde_json::json!("/checkout");
    frame["route_origin_id"] = serde_json::json!(route.id);
    frame["db_operation_origin_id"] = serde_json::json!(db_operation.id);
    frame["commerce_adapter_origin_id"] = serde_json::json!(commerce_adapter.id);
    write_json(
        &trace_path,
        &serde_json::json!({
            "schema_version": 1,
            "kind": "orv.production.trace",
            "frame_count": 1,
            "frames": [frame],
        }),
    )
    .expect("write trace");
    let editor_out = dir.join("editor");

    cmd_editor_export_with_options(&path, &editor_out, Some(&build_out), Some(&trace_path))
        .expect("editor export with trace adapter navigation");

    let native_host =
        read_json_value(&editor_out.join(EDITOR_NATIVE_HOST_MANIFEST_PATH)).expect("native host");
    let frame = &native_host["trace"]["frames"]
        .as_array()
        .expect("native trace frames")[0];
    assert_eq!(frame["origin_id"], route.id);
    assert_eq!(frame["db_operation_origin_id"], db_operation.id);
    assert_eq!(frame["commerce_adapter_origin_id"], commerce_adapter.id);
    assert_eq!(frame["db_source"], frame["db_navigation"]["source"]);
    assert_eq!(
        frame["commerce_source"],
        frame["commerce_navigation"]["source"]
    );
    assert_eq!(
        frame["db_reveal_command"],
        serde_json::json!([
            "orv",
            "editor",
            "reveal",
            build_out.display().to_string(),
            db_operation.id
        ])
    );
    assert_eq!(
        frame["commerce_reveal_command"],
        serde_json::json!([
            "orv",
            "editor",
            "reveal",
            build_out.display().to_string(),
            commerce_adapter.id
        ])
    );
    assert!(frame["db_source"]["snippet"]
        .as_str()
        .is_some_and(|snippet| snippet.contains("shopdb.create")));
    assert!(frame["commerce_source"]["snippet"]
        .as_str()
        .is_some_and(|snippet| snippet.contains("@payment.connect")));
    let actions = frame["actions"]
        .as_array()
        .expect("native trace adapter actions");
    assert!(actions.iter().any(|action| action["slot"] == "db"
        && action["action"] == "trace.db.reveal"
        && action["origin_id"] == db_operation.id
        && action["command"] == frame["db_reveal_command"]
        && action["source"] == frame["db_source"]));
    assert!(actions.iter().any(|action| action["slot"] == "commerce"
        && action["action"] == "trace.commerce.reveal"
        && action["origin_id"] == commerce_adapter.id
        && action["command"] == frame["commerce_reveal_command"]
        && action["source"] == frame["commerce_source"]));
    let _ = std::fs::remove_dir_all(dir);
}
