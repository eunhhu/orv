use super::*;

#[test]
fn editor_export_trace_options_are_accepted() {
    let parsed = Cli::try_parse_from([
        "orv",
        "editor",
        "export",
        "src/main.orv",
        "--out",
        "target/orv-editor",
        "--build",
        "target/orv-build",
        "--trace",
        "target/orv-trace.json",
    ]);
    if let Err(err) = parsed {
        panic!("{}", err.render());
    }
}

#[test]
fn editor_trace_subcommand_is_accepted() {
    let parsed = Cli::try_parse_from([
        "orv",
        "editor",
        "trace",
        "target/orv-build",
        "--trace",
        "target/orv-trace.json",
    ]);
    if let Err(err) = parsed {
        panic!("{}", err.render());
    }
}

#[test]
fn editor_trace_stream_subcommand_accepts_event_stream_snapshot() {
    let parsed = Cli::try_parse_from([
        "orv",
        "editor",
        "trace-stream",
        "target/orv-build",
        "--events",
        "target/orv-build/trace-events.sse",
    ]);
    if let Err(err) = parsed {
        panic!("{}", err.render());
    }
}

#[test]
fn editor_trace_links_request_origin_to_source_navigation() {
    let dir = temp_output_dir("editor-trace");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let path = dir.join("app.orv");
    std::fs::write(
        &path,
        r#"@server {
  @listen 0
  @route GET /ping {
    @respond 200 { ok: true }
  }
}"#,
    )
    .expect("write source");
    let out = dir.join("dist");

    cmd_build(&path, &out).expect("build artifacts");
    let origin_map: orv_compiler::OriginMap = serde_json::from_str(
        &std::fs::read_to_string(out.join("origin-map.json")).expect("origin map"),
    )
    .expect("origin map json");
    let route = origin_map
        .entries
        .iter()
        .find(|entry| entry.kind == "route" && entry.name == "GET /ping")
        .expect("route origin");
    let trace_path = dir.join("production-trace.json");
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

    let trace = editor_trace_json(&out, &trace_path).expect("editor trace");

    assert_eq!(trace["schema_version"], 1);
    assert_eq!(trace["kind"], "orv.editor.trace");
    assert_eq!(trace["trace"]["frame_count"], 1);
    assert_eq!(trace["live_refresh"]["strategy"], "trace-file-hash");
    assert_eq!(
        trace["live_refresh"]["watch"]["trace"]["path"],
        trace_path.display().to_string()
    );
    assert!(trace["live_refresh"]["watch"]["trace"]["content_hash"]
        .as_str()
        .is_some_and(|hash| hash.starts_with("fnv1a64:")));
    assert_eq!(trace["frames"][0]["request"]["method"], "GET");
    assert_eq!(trace["frames"][0]["request"]["path"], "/ping");
    assert_eq!(trace["frames"][0]["origin_id"], route.id);
    assert_eq!(trace["frames"][0]["navigation"]["focus"]["panel"], "routes");
    assert!(trace["frames"][0]["navigation"]["source"]["snippet"]
        .as_str()
        .is_some_and(|snippet| snippet.contains("@route GET /ping")));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn editor_trace_links_response_origin_to_source_navigation() {
    let dir = temp_output_dir("editor-trace-response");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let path = dir.join("app.orv");
    std::fs::write(
        &path,
        r#"@server {
  @listen 0
  @route GET /ping {
    @respond 200 { ok: true }
  }
}"#,
    )
    .expect("write source");
    let out = dir.join("dist");

    cmd_build(&path, &out).expect("build artifacts");
    let origin_map: orv_compiler::OriginMap = serde_json::from_str(
        &std::fs::read_to_string(out.join("origin-map.json")).expect("origin map"),
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
    let trace_path = dir.join("production-trace.json");
    let mut frame = editor_trace_test_route_frame(&route.id);
    frame["response_origin_id"] = serde_json::json!(response.id);
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

    let trace = editor_trace_json(&out, &trace_path).expect("editor trace");

    assert_eq!(trace["frames"][0]["origin_id"], route.id);
    assert_eq!(trace["frames"][0]["response_origin_id"], response.id);
    assert_eq!(
        trace["frames"][0]["summary"]["response_origin_id"],
        response.id
    );
    assert!(trace["frames"][0]["navigation"]["source"]["snippet"]
        .as_str()
        .is_some_and(|snippet| snippet.contains("@route GET /ping")));
    assert!(
        trace["frames"][0]["response_navigation"]["source"]["snippet"]
            .as_str()
            .is_some_and(|snippet| snippet.contains("@respond 200"))
    );
    assert_eq!(trace["action_count"], 2);
    assert!(trace["frames"][0]["actions"]
        .as_array()
        .expect("trace actions")
        .iter()
        .any(|action| action["slot"] == "route"
            && action["command"] == trace["frames"][0]["reveal_command"]));
    assert!(trace["frames"][0]["actions"]
        .as_array()
        .expect("trace actions")
        .iter()
        .any(|action| action["slot"] == "response"
            && action["command"] == trace["frames"][0]["response_reveal_command"]));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn editor_trace_summarizes_request_statuses_for_panels() {
    let dir = temp_output_dir("editor-trace-status-summary");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let trace_path = dir.join("production-trace.json");
    write_json(
        &trace_path,
        &serde_json::json!({
            "schema_version": 1,
            "kind": "orv.production.trace",
            "frame_count": 3,
            "frames": [
                editor_trace_test_frame_for("GET", "/ok", 200),
                editor_trace_test_frame_for("GET", "/missing", 404),
                editor_trace_test_frame_for("POST", "/checkout", 503),
            ],
        }),
    )
    .expect("write trace");

    let trace = editor_trace_json(&dir, &trace_path).expect("editor trace");

    assert_eq!(trace["trace"]["status_counts"]["total"], 3);
    assert_eq!(trace["trace"]["status_counts"]["ok"], 1);
    assert_eq!(trace["trace"]["status_counts"]["client_error"], 1);
    assert_eq!(trace["trace"]["status_counts"]["server_error"], 1);
    assert_eq!(trace["frames"][0]["summary"]["label"], "GET /ok -> 200");
    assert_eq!(
        trace["frames"][1]["summary"]["status_class"],
        "client_error"
    );
    assert_eq!(
        trace["frames"][2]["summary"]["status_class"],
        "server_error"
    );
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn editor_trace_rejects_extra_trace_root_key() {
    let dir = temp_output_dir("editor-trace-extra-root");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let trace_path = dir.join("production-trace.json");
    write_json(
        &trace_path,
        &serde_json::json!({
            "schema_version": 1,
            "kind": "orv.production.trace",
            "frame_count": 0,
            "frames": [],
            "unexpected": true,
        }),
    )
    .expect("write trace");

    let err = editor_trace_json(&dir, &trace_path).expect_err("extra trace root key must fail");

    assert!(err
        .to_string()
        .contains("trace JSON keys must match contract"));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn editor_trace_rejects_missing_trace_frame_count() {
    let dir = temp_output_dir("editor-trace-missing-frame-count");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let trace_path = dir.join("production-trace.json");
    write_json(
        &trace_path,
        &serde_json::json!({
            "schema_version": 1,
            "kind": "orv.production.trace",
            "frames": [],
        }),
    )
    .expect("write trace");

    let err =
        editor_trace_json(&dir, &trace_path).expect_err("missing trace frame_count must fail");

    assert!(err
        .to_string()
        .contains("trace JSON keys must match contract"));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn editor_trace_rejects_trace_root_version_and_kind_drift() {
    for (case, field, value, expected) in [
        (
            "schema-version",
            "schema_version",
            serde_json::json!(2),
            "trace JSON schema_version must be 1",
        ),
        (
            "kind",
            "kind",
            serde_json::json!("orv.production.trace.v2"),
            "trace JSON kind must be orv.production.trace",
        ),
    ] {
        let dir = temp_output_dir(&format!("editor-trace-root-{case}-drift"));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        let trace_path = dir.join("production-trace.json");
        let mut trace = serde_json::json!({
            "schema_version": 1,
            "kind": "orv.production.trace",
            "frame_count": 0,
            "frames": [],
        });
        trace[field] = value;
        write_json(&trace_path, &trace).expect("write trace");

        let err = editor_trace_json(&dir, &trace_path).expect_err("trace root drift must fail");

        assert!(err.to_string().contains(expected), "{err}");
        let _ = std::fs::remove_dir_all(dir);
    }
}

#[test]
fn editor_trace_rejects_trace_frame_count_mismatch() {
    let dir = temp_output_dir("editor-trace-frame-count-mismatch");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let trace_path = dir.join("production-trace.json");
    write_json(
        &trace_path,
        &serde_json::json!({
            "schema_version": 1,
            "kind": "orv.production.trace",
            "frame_count": 2,
            "frames": [editor_trace_test_frame()],
        }),
    )
    .expect("write trace");

    let err = editor_trace_json(&dir, &trace_path).expect_err("frame_count drift must fail");

    assert!(err
        .to_string()
        .contains("trace JSON frame_count must match frames length"));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn editor_trace_rejects_missing_trace_frame_base_key() {
    let dir = temp_output_dir("editor-trace-missing-frame-base-key");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let trace_path = dir.join("production-trace.json");
    let mut frame = editor_trace_test_frame();
    frame
        .as_object_mut()
        .expect("trace frame object")
        .remove("body");
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

    let err =
        editor_trace_json(&dir, &trace_path).expect_err("missing trace frame base key must fail");

    assert!(err
        .to_string()
        .contains("trace JSON frames[0].body is required"));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn editor_trace_rejects_extra_trace_frame_key() {
    let dir = temp_output_dir("editor-trace-extra-frame");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let trace_path = dir.join("production-trace.json");
    let mut frame = editor_trace_test_frame();
    frame["unexpected"] = serde_json::json!("drift");
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

    let err = editor_trace_json(&dir, &trace_path).expect_err("extra trace frame key must fail");

    assert!(err
        .to_string()
        .contains("trace JSON frames[0] keys must match contract"));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn editor_trace_rejects_invalid_trace_frame_status_type() {
    let dir = temp_output_dir("editor-trace-invalid-status");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let trace_path = dir.join("production-trace.json");
    let mut frame = editor_trace_test_frame();
    frame["status"] = serde_json::json!("200");
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

    let err = editor_trace_json(&dir, &trace_path).expect_err("string trace status must fail");

    assert!(err
        .to_string()
        .contains("trace JSON frames[0].status must be an unsigned integer"));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn editor_trace_rejects_invalid_trace_frame_params_type() {
    let dir = temp_output_dir("editor-trace-invalid-params");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let trace_path = dir.join("production-trace.json");
    let mut frame = editor_trace_test_frame();
    frame["params"] = serde_json::json!({ "id": 42 });
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

    let err = editor_trace_json(&dir, &trace_path).expect_err("non-string trace params must fail");

    assert!(err
        .to_string()
        .contains("trace JSON frames[0].params values must be strings"));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn editor_trace_rejects_invalid_trace_frame_origin_id_types() {
    for key in [
        "route_origin_id",
        "response_origin_id",
        "db_operation_origin_id",
        "commerce_adapter_origin_id",
    ] {
        let dir = temp_output_dir(&format!("editor-trace-invalid-{key}"));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        let trace_path = dir.join("production-trace.json");
        let mut frame = editor_trace_test_frame();
        frame[key] = serde_json::json!(42);
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

        let Err(err) = editor_trace_json(&dir, &trace_path) else {
            panic!("{key} numeric origin id must fail")
        };

        assert!(
            err.to_string().contains(&format!(
                "trace JSON frames[0].{key} must be a string or null"
            )),
            "{err}"
        );
        let _ = std::fs::remove_dir_all(dir);
    }
}

#[test]
fn editor_trace_exposes_live_trace_event_stream_transport() {
    let (src_dir, path) = prod_server_source("editor-trace-live-transport-source");
    let out = temp_output_dir("editor-trace-live-transport");

    cmd_build_with_profile(&path, &out, BuildProfile::Production).expect("prod build");
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

    let trace = editor_trace_json(&out, &trace_path).expect("editor trace");

    assert_eq!(trace["live_refresh"]["transport"]["kind"], "event-source");
    assert_eq!(trace["live_refresh"]["transport"]["event"], "orv:trace");
    assert_eq!(
        trace["live_refresh"]["transport"]["url"],
        "http://127.0.0.1:8080/__orv/trace/events"
    );
    let _ = std::fs::remove_dir_all(src_dir);
    let _ = std::fs::remove_dir_all(out);
}

#[test]
fn editor_trace_stream_consumes_eventsource_trace_snapshot() {
    let (src_dir, path) = prod_server_source("editor-trace-stream-source");
    let out = temp_output_dir("editor-trace-stream");

    cmd_build_with_profile(&path, &out, BuildProfile::Production).expect("prod build");
    let origin_map: orv_compiler::OriginMap = serde_json::from_str(
        &std::fs::read_to_string(out.join("origin-map.json")).expect("origin map"),
    )
    .expect("origin map json");
    let route = origin_map
        .entries
        .iter()
        .find(|entry| entry.kind == "route" && entry.name == "GET /ping")
        .expect("route origin");
    let payload = serde_json::json!({
        "schema_version": 1,
        "kind": "orv.production.trace",
        "frame_count": 1,
        "frames": [editor_trace_test_route_frame(&route.id)],
    });
    let events_path = src_dir.join("trace-events.sse");
    std::fs::write(
        &events_path,
        format!(
            "event: message\ndata: {{\"kind\":\"heartbeat\"}}\n\nevent: orv:trace\ndata: {}\n\n",
            serde_json::to_string(&payload).expect("payload json")
        ),
    )
    .expect("write trace events");

    let stream = editor_trace_stream_json(&out, &events_path).expect("editor trace stream");

    assert_eq!(stream["kind"], "orv.editor.trace.stream");
    assert_eq!(stream["event_stream"]["content_type"], "text/event-stream");
    assert_eq!(stream["event_stream"]["event_count"], 2);
    assert_eq!(stream["event_stream"]["trace_event_count"], 1);
    assert_eq!(stream["events"][0]["event"], "orv:trace");
    assert_eq!(stream["latest"]["kind"], "orv.editor.trace");
    assert_eq!(
        stream["latest"]["live_refresh"]["strategy"],
        "event-source-snapshot"
    );
    assert_eq!(
        stream["latest"]["live_refresh"]["transport"]["url"],
        "http://127.0.0.1:8080/__orv/trace/events"
    );
    assert_eq!(stream["latest"]["frames"][0]["origin_id"], route.id);
    assert_eq!(
        stream["latest"]["frames"][0]["navigation"]["focus"]["panel"],
        "routes"
    );
    let _ = std::fs::remove_dir_all(src_dir);
    let _ = std::fs::remove_dir_all(out);
}

#[test]
fn editor_trace_stream_consumes_trace_frame_events() {
    let (src_dir, path) = prod_server_source("editor-trace-frame-stream-source");
    let out = temp_output_dir("editor-trace-frame-stream");

    cmd_build_with_profile(&path, &out, BuildProfile::Production).expect("prod build");
    let origin_map: orv_compiler::OriginMap = serde_json::from_str(
        &std::fs::read_to_string(out.join("origin-map.json")).expect("origin map"),
    )
    .expect("origin map json");
    let route = origin_map
        .entries
        .iter()
        .find(|entry| entry.kind == "route" && entry.name == "GET /ping")
        .expect("route origin");
    let frame = editor_trace_test_route_frame(&route.id);
    let events_path = src_dir.join("trace-frame-events.sse");
    std::fs::write(
        &events_path,
        format!(
            "event: orv:trace.frame\ndata: {}\n\nevent: orv:trace.frame\ndata: {}\n\n",
            serde_json::to_string(&serde_json::json!({
                "schema_version": 1,
                "kind": "orv.production.trace.frame",
                "index": 0,
                "frame": frame,
            }))
            .expect("frame event"),
            serde_json::to_string(&serde_json::json!({
                "schema_version": 1,
                "kind": "orv.production.trace.frame",
                "index": 1,
                "frame": frame,
            }))
            .expect("frame event"),
        ),
    )
    .expect("write trace frame events");

    let stream = editor_trace_stream_json(&out, &events_path).expect("editor trace stream");

    assert_eq!(stream["event_stream"]["trace_event_count"], 0);
    assert_eq!(stream["event_stream"]["trace_frame_event_count"], 2);
    assert_eq!(stream["events"][0]["event"], "orv:trace.frame");
    assert_eq!(stream["latest"]["kind"], "orv.editor.trace");
    assert_eq!(stream["latest"]["trace"]["frame_count"], 2);
    assert_eq!(stream["latest"]["frames"][0]["origin_id"], route.id);
    assert_eq!(
        stream["latest"]["frames"][0]["navigation"]["focus"]["panel"],
        "routes"
    );
    let _ = std::fs::remove_dir_all(src_dir);
    let _ = std::fs::remove_dir_all(out);
}

#[test]
fn editor_trace_stream_applies_frame_events_after_snapshot_to_latest() {
    let (src_dir, path) = prod_server_source("editor-trace-snapshot-plus-frame-source");
    let out = temp_output_dir("editor-trace-snapshot-plus-frame");

    cmd_build_with_profile(&path, &out, BuildProfile::Production).expect("prod build");
    let origin_map: orv_compiler::OriginMap = serde_json::from_str(
        &std::fs::read_to_string(out.join("origin-map.json")).expect("origin map"),
    )
    .expect("origin map json");
    let route = origin_map
        .entries
        .iter()
        .find(|entry| entry.kind == "route" && entry.name == "GET /ping")
        .expect("route origin");
    let frame = editor_trace_test_route_frame(&route.id);
    let events_path = src_dir.join("trace-snapshot-plus-frame.sse");
    std::fs::write(
        &events_path,
        format!(
            "event: orv:trace\ndata: {}\n\nevent: orv:trace.frame\ndata: {}\n\n",
            serde_json::to_string(&serde_json::json!({
                "schema_version": 1,
                "kind": "orv.production.trace",
                "frame_count": 0,
                "frames": [],
            }))
            .expect("snapshot event"),
            serde_json::to_string(&serde_json::json!({
                "schema_version": 1,
                "kind": "orv.production.trace.frame",
                "index": 0,
                "frame": frame,
            }))
            .expect("frame event"),
        ),
    )
    .expect("write trace events");

    let stream = editor_trace_stream_json(&out, &events_path).expect("editor trace stream");

    assert_eq!(stream["event_stream"]["trace_event_count"], 1);
    assert_eq!(stream["event_stream"]["trace_frame_event_count"], 1);
    assert_eq!(stream["latest"]["trace"]["frame_count"], 1);
    assert_eq!(stream["latest"]["frames"][0]["origin_id"], route.id);
    assert_eq!(
        stream["latest"]["frames"][0]["navigation"]["focus"]["panel"],
        "routes"
    );
    let _ = std::fs::remove_dir_all(src_dir);
    let _ = std::fs::remove_dir_all(out);
}

#[test]
fn editor_trace_stream_appends_live_frame_after_snapshot_replay() {
    let (src_dir, path) = prod_server_source("editor-trace-snapshot-replay-append-source");
    let out = temp_output_dir("editor-trace-snapshot-replay-append");

    cmd_build_with_profile(&path, &out, BuildProfile::Production).expect("prod build");
    let origin_map: orv_compiler::OriginMap = serde_json::from_str(
        &std::fs::read_to_string(out.join("origin-map.json")).expect("origin map"),
    )
    .expect("origin map json");
    let route = origin_map
        .entries
        .iter()
        .find(|entry| entry.kind == "route" && entry.name == "GET /ping")
        .expect("route origin");
    let snapshot_frame = editor_trace_test_route_frame(&route.id);
    let mut live_frame = editor_trace_test_route_frame(&route.id);
    live_frame["status"] = serde_json::json!(204);
    let events_path = src_dir.join("trace-snapshot-replay-append.sse");
    std::fs::write(
        &events_path,
        format!(
            "event: orv:trace\ndata: {}\n\nevent: orv:trace.frame\ndata: {}\n\n",
            serde_json::to_string(&serde_json::json!({
                "schema_version": 1,
                "kind": "orv.production.trace",
                "frame_count": 1,
                "frames": [snapshot_frame],
            }))
            .expect("snapshot event"),
            serde_json::to_string(&serde_json::json!({
                "schema_version": 1,
                "kind": "orv.production.trace.frame",
                "index": 1,
                "frame": live_frame,
            }))
            .expect("frame event"),
        ),
    )
    .expect("write trace events");

    let stream = editor_trace_stream_json(&out, &events_path).expect("editor trace stream");

    assert_eq!(stream["event_stream"]["trace_event_count"], 1);
    assert_eq!(stream["event_stream"]["trace_frame_event_count"], 1);
    assert_eq!(stream["latest"]["trace"]["frame_count"], 2);
    assert_eq!(stream["latest"]["frames"][0]["request"]["status"], 200);
    assert_eq!(stream["latest"]["frames"][1]["request"]["status"], 204);
    assert_eq!(stream["latest"]["frames"][0]["origin_id"], route.id);
    assert_eq!(stream["latest"]["frames"][1]["origin_id"], route.id);
    let _ = std::fs::remove_dir_all(src_dir);
    let _ = std::fs::remove_dir_all(out);
}

#[test]
fn editor_trace_stream_rejects_extra_trace_frame_event_key() {
    let dir = temp_output_dir("editor-trace-stream-extra-event");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let events_path = dir.join("trace-frame-events.sse");
    let event = serde_json::json!({
        "schema_version": 1,
        "kind": "orv.production.trace.frame",
        "index": 0,
        "frame": editor_trace_test_frame(),
        "unexpected": true,
    });
    std::fs::write(
        &events_path,
        format!(
            "event: orv:trace.frame\ndata: {}\n\n",
            serde_json::to_string(&event).expect("event json")
        ),
    )
    .expect("write trace frame events");

    let err = editor_trace_stream_json(&dir, &events_path)
        .expect_err("extra trace frame event key must fail");

    assert!(err
        .to_string()
        .contains("trace frame event 0 keys must match contract"));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn editor_trace_stream_rejects_trace_frame_event_version_and_kind_drift() {
    for (case, field, value, expected) in [
        (
            "schema-version",
            "schema_version",
            serde_json::json!(2),
            "trace frame event 0 schema_version must be 1",
        ),
        (
            "kind",
            "kind",
            serde_json::json!("orv.production.trace"),
            "trace frame event 0 kind must be orv.production.trace.frame",
        ),
    ] {
        let dir = temp_output_dir(&format!("editor-trace-stream-event-{case}-drift"));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        let events_path = dir.join("trace-frame-events.sse");
        let mut event = serde_json::json!({
            "schema_version": 1,
            "kind": "orv.production.trace.frame",
            "index": 0,
            "frame": editor_trace_test_frame(),
        });
        event[field] = value;
        std::fs::write(
            &events_path,
            format!(
                "event: orv:trace.frame\ndata: {}\n\n",
                serde_json::to_string(&event).expect("event json")
            ),
        )
        .expect("write trace frame events");

        let err = editor_trace_stream_json(&dir, &events_path)
            .expect_err("trace frame event drift must fail");

        assert!(err.to_string().contains(expected), "{err}");
        let _ = std::fs::remove_dir_all(dir);
    }
}

#[test]
fn editor_trace_stream_rejects_trace_frame_event_index_drift() {
    let dir = temp_output_dir("editor-trace-stream-index-drift");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let events_path = dir.join("trace-frame-events.sse");
    let event = serde_json::json!({
        "schema_version": 1,
        "kind": "orv.production.trace.frame",
        "index": 1,
        "frame": editor_trace_test_frame(),
    });
    std::fs::write(
        &events_path,
        format!(
            "event: orv:trace.frame\ndata: {}\n\n",
            serde_json::to_string(&event).expect("event json")
        ),
    )
    .expect("write trace frame events");

    let err = editor_trace_stream_json(&dir, &events_path)
        .expect_err("trace frame event index drift must fail");

    assert!(err
        .to_string()
        .contains("trace frame event 0 index must match frame event order"));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn editor_trace_stream_rejects_snapshot_replay_frame_drift() {
    let dir = temp_output_dir("editor-trace-stream-snapshot-replay-drift");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let events_path = dir.join("trace-frame-events.sse");
    let snapshot = serde_json::json!({
        "schema_version": 1,
        "kind": "orv.production.trace",
        "frame_count": 1,
        "frames": [editor_trace_test_frame()],
    });
    let mut drift_frame = editor_trace_test_frame();
    drift_frame["path"] = serde_json::json!("/pong");
    let event = serde_json::json!({
        "schema_version": 1,
        "kind": "orv.production.trace.frame",
        "index": 0,
        "frame": drift_frame,
    });
    std::fs::write(
        &events_path,
        format!(
            "event: orv:trace\ndata: {}\n\nevent: orv:trace.frame\ndata: {}\n\n",
            serde_json::to_string(&snapshot).expect("snapshot event"),
            serde_json::to_string(&event).expect("frame event")
        ),
    )
    .expect("write trace frame events");

    let err =
        editor_trace_stream_json(&dir, &events_path).expect_err("snapshot replay drift must fail");

    assert!(err
        .to_string()
        .contains("trace frame event 1 frame must match snapshot frame at index"));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn editor_trace_stream_rejects_live_frame_gap_after_snapshot_replay() {
    let dir = temp_output_dir("editor-trace-stream-snapshot-replay-gap");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let events_path = dir.join("trace-frame-events.sse");
    let snapshot = serde_json::json!({
        "schema_version": 1,
        "kind": "orv.production.trace",
        "frame_count": 1,
        "frames": [editor_trace_test_frame()],
    });
    let event = serde_json::json!({
        "schema_version": 1,
        "kind": "orv.production.trace.frame",
        "index": 2,
        "frame": editor_trace_test_frame(),
    });
    std::fs::write(
        &events_path,
        format!(
            "event: orv:trace\ndata: {}\n\nevent: orv:trace.frame\ndata: {}\n\n",
            serde_json::to_string(&snapshot).expect("snapshot event"),
            serde_json::to_string(&event).expect("frame event")
        ),
    )
    .expect("write trace frame events");

    let err =
        editor_trace_stream_json(&dir, &events_path).expect_err("snapshot replay gap must fail");

    assert!(err
        .to_string()
        .contains("trace frame event 1 index must match frame event order"));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn editor_trace_stream_rejects_unwrapped_trace_frame_event() {
    let dir = temp_output_dir("editor-trace-stream-unwrapped-event");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let events_path = dir.join("trace-frame-events.sse");
    std::fs::write(
        &events_path,
        "event: orv:trace.frame\ndata: {\"method\":\"GET\",\"path\":\"/ping\",\"status\":200}\n\n",
    )
    .expect("write trace frame events");

    let err = editor_trace_stream_json(&dir, &events_path)
        .expect_err("unwrapped trace frame event must fail");

    assert!(err
        .to_string()
        .contains("trace frame event 0 keys must match contract"));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn editor_export_embeds_trace_navigation_state() {
    let dir = temp_output_dir("editor-export-trace");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let path = dir.join("app.orv");
    std::fs::write(
        &path,
        r"@server {
  @listen 0
  @route GET /ping {
    @respond 200 { ok: true }
  }
}",
    )
    .expect("write source");
    let build_out = dir.join("dist");

    cmd_build(&path, &build_out).expect("build artifacts");
    let origin_map: orv_compiler::OriginMap = serde_json::from_str(
        &std::fs::read_to_string(build_out.join("origin-map.json")).expect("origin map"),
    )
    .expect("origin map json");
    let route = origin_map
        .entries
        .iter()
        .find(|entry| entry.kind == "route" && entry.name == "GET /ping")
        .expect("route origin");
    let trace_path = dir.join("production-trace.json");
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
    let out = dir.join("editor");

    cmd_editor_export_with_options(&path, &out, Some(&build_out), Some(&trace_path))
        .expect("editor export with trace");

    let html = std::fs::read_to_string(out.join("index.html")).expect("editor html");
    let trace_panel =
        std::fs::read_to_string(out.join(EDITOR_TRACE_PANEL_HTML_PATH)).expect("trace panel");
    let state = read_json_value(&out.join("state.json")).expect("editor state");
    let native_host =
        read_json_value(&out.join(EDITOR_NATIVE_HOST_MANIFEST_PATH)).expect("native host");
    let desktop_package = read_json_value(&out.join(EDITOR_NATIVE_HOST_DESKTOP_PACKAGE_PATH))
        .expect("desktop package");
    assert!(html.contains("Trace"));
    assert!(html.contains("id=\"trace-list\""));
    assert!(html.contains("id=\"trace-detail\""));
    assert!(html.contains("id=\"trace-action-list\""));
    assert!(html.contains("id=\"trace-action-detail\""));
    assert!(html.contains("renderEditorState"));
    assert!(html.contains("renderTraceDetail"));
    assert!(html.contains("runTraceRevealAction"));
    assert!(html.contains("orv:trace-reveal-action"));
    assert_eq!(state["trace"]["kind"], "orv.editor.trace");
    assert_eq!(state["trace"]["frames"][0]["origin_id"], route.id);
    assert_eq!(
        state["trace"]["frames"][0]["navigation"]["focus"]["panel"],
        "routes"
    );
    assert_eq!(state["trace"]["frames"][0]["actions"][0]["slot"], "route");
    assert_eq!(
        state["trace"]["frames"][0]["actions"][0]["command"],
        serde_json::json!([
            "orv",
            "editor",
            "reveal",
            build_out.display().to_string(),
            route.id
        ])
    );
    assert_eq!(state["trace"]["action_count"], 1);
    assert!(trace_panel.contains("Trace Panel"));
    assert!(trace_panel.contains("GET /ping -> 200"));
    assert!(trace_panel.contains(route.id.as_str()));
    assert_eq!(
        native_host["artifacts"]["trace_panel_html"],
        EDITOR_TRACE_PANEL_HTML_PATH
    );
    assert_eq!(
        native_host["trace"]["panel_html_path"],
        EDITOR_TRACE_PANEL_HTML_PATH
    );
    assert_eq!(
        native_host["trace"]["panel_artifact"]["path"],
        EDITOR_TRACE_PANEL_HTML_PATH
    );
    assert_eq!(
        native_host["trace"]["panel_artifact"]["kind"],
        "orv.editor.trace.panel"
    );
    assert_eq!(
        native_host["trace"]["action_runner"]["command_format"][2],
        "run-action"
    );
    assert_eq!(
        native_host["trace"]["action_result_artifact"]["path"],
        EDITOR_TRACE_ACTION_RESULT_PATH
    );
    assert_eq!(
        native_host["artifacts"]["trace_action_result"],
        EDITOR_TRACE_ACTION_RESULT_PATH
    );
    let panels = native_host["panels"]
        .as_array()
        .expect("native host panel inventory");
    assert!(panels.iter().any(|panel| {
        panel["name"] == "trace" && panel["artifact"]["path"] == EDITOR_TRACE_PANEL_HTML_PATH
    }));
    assert!(panels.iter().any(|panel| {
        panel["name"] == "trace_action_result"
            && panel["artifact"]["path"] == EDITOR_TRACE_ACTION_RESULT_PATH
    }));
    assert_eq!(native_host["capabilities"]["trace_navigation"], true);
    assert!(desktop_package["process_policy"]["allowed_commands"]
        .as_array()
        .expect("allowed commands")
        .iter()
        .any(|command| command["name"] == "trace_reveal_action"
            && command["endpoint"] == "/__orv/native-host/action"
            && command["result"]["json"] == EDITOR_TRACE_ACTION_RESULT_PATH));
    assert!(desktop_package["refresh"]["events"]
        .as_array()
        .expect("refresh events")
        .iter()
        .any(|event| event["event"] == "orv:trace-action-result"
            && event["html"] == EDITOR_TRACE_ACTION_RESULT_HTML_PATH));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn editor_run_action_executes_trace_reveal_and_writes_result_artifact() {
    let (src_dir, path) = prod_server_source("editor-run-action-trace-reveal-source");
    let build_out = temp_output_dir("editor-run-action-trace-reveal-build");

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

    cmd_editor_run_action(
        &editor_out.join(EDITOR_NATIVE_HOST_MANIFEST_PATH),
        "trace.route.reveal",
        Some(0),
        Some("route"),
    )
    .expect("run trace reveal action");

    let result =
        read_json_value(&editor_out.join(EDITOR_TRACE_ACTION_RESULT_PATH)).expect("action result");
    assert_eq!(result["kind"], "orv.editor.native_host.action.result");
    assert_eq!(result["execution"]["status"], "passed");
    assert_eq!(result["execution"]["allowlist"], "orv.editor.reveal");
    assert_eq!(result["action"]["slot"], "route");
    assert_eq!(result["action"]["origin_id"], route.id);
    assert_eq!(
        result["command"],
        serde_json::json!([
            "orv",
            "editor",
            "reveal",
            build_out.display().to_string(),
            route.id
        ])
    );
    assert_eq!(result["navigation"]["focus"]["panel"], "routes");
    assert!(result["panels"]["trace_action"]["source"]["snippet"]
        .as_str()
        .is_some_and(|snippet| snippet.contains("@route GET /ping")));
    assert_eq!(
        result["result_artifact"]["html_path"],
        EDITOR_TRACE_ACTION_RESULT_HTML_PATH
    );
    let html = std::fs::read_to_string(editor_out.join(EDITOR_TRACE_ACTION_RESULT_HTML_PATH))
        .expect("action result html");
    assert!(html.contains("Trace Action Result"));
    assert!(html.contains("orv editor reveal"));
    assert!(html.contains(route.id.as_str()));
    assert!(html.contains("@route GET /ping"));
    let _ = std::fs::remove_dir_all(src_dir);
    let _ = std::fs::remove_dir_all(build_out);
}

#[test]
fn editor_export_renders_trace_status_filters() {
    let state = serde_json::json!({
        "schema_version": 1,
        "snapshot": {
            "entry": { "path": "app.orv" },
            "panels": {
                "files": [],
                "routes": [],
                "schema": [],
                "domains": []
            },
            "diagnostics": []
        },
        "runtime": {
            "runtime": {
                "status": "ok",
                "stdout": ""
            }
        },
        "trace": {
            "trace": {
                "status_counts": {
                    "total": 3,
                    "ok": 1,
                    "redirect": 0,
                    "client_error": 1,
                    "server_error": 1,
                    "other": 0
                }
            },
            "frames": [
                { "summary": { "label": "GET /ok -> 200", "status_class": "ok" } },
                { "summary": { "label": "GET /missing -> 404", "status_class": "client_error" } },
                { "summary": { "label": "POST /checkout -> 503", "status_class": "server_error" } }
            ]
        }
    });

    let html = editor_export_html(&state).expect("editor html");

    assert!(html.contains("id=\"trace-status-summary\""));
    assert!(html.contains("data-trace-filter=\"client_error\""));
    assert!(html.contains("data-trace-filter=\"server_error\""));
    assert!(html.contains("filterTraceFrames"));
    assert!(html.contains("Client Err<b>1</b>"));
    assert!(html.contains("Server Err<b>1</b>"));
}
