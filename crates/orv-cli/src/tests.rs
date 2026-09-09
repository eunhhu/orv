use super::*;

mod artifact_cases;
use artifact_cases::{artifact_case, json_case, source_fixture, verify_artifact_cases};

mod unicode;

fn workspace_path(parts: &[&str]) -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("../..");
    for part in parts {
        path.push(part);
    }
    path
}

fn orv_files_under(parts: &[&str]) -> Vec<PathBuf> {
    let root = workspace_path(parts);
    let mut files = Vec::new();
    collect_orv_files(&root, &mut files);
    files.sort();
    files
}

fn collect_orv_files(root: &Path, out: &mut Vec<PathBuf>) {
    for entry in
        std::fs::read_dir(root).unwrap_or_else(|e| panic!("failed to read {}: {e}", root.display()))
    {
        let path = entry.expect("fixture dir entry").path();
        if path.is_dir() {
            collect_orv_files(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "orv") {
            out.push(path);
        }
    }
}

fn temp_output_dir(name: &str) -> PathBuf {
    static SEQUENCE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock after unix epoch")
        .as_nanos();
    let mut path = std::env::temp_dir();
    path.push(format!(
        "orv-cli-{name}-{}-{unique}-{sequence}",
        std::process::id()
    ));
    path
}

fn adapter_values_without_source_origin_ids(value: &serde_json::Value) -> serde_json::Value {
    let mut value = value.clone();
    for adapter in value.as_array_mut().expect("adapter array") {
        adapter
            .as_object_mut()
            .expect("adapter object")
            .remove("source_origin_id");
        adapter
            .as_object_mut()
            .expect("adapter object")
            .remove("source_origin_ids");
    }
    value
}

fn refresh_origin_map_entry_identity(origin_map: &mut serde_json::Value, index: usize) {
    let (old_id, new_id) = {
        let entry = &mut origin_map["entries"][index];
        let old_id = entry["id"].as_str().expect("origin id").to_string();
        let kind = entry["kind"].as_str().expect("origin kind").to_string();
        let name = entry["name"].as_str().expect("origin name").to_string();
        let span = Span::new(
            FileId(
                entry["span"]["file"]
                    .as_u64()
                    .expect("span file")
                    .try_into()
                    .expect("span file fits u32"),
            ),
            ByteRange::new(
                entry["span"]["start"]
                    .as_u64()
                    .expect("span start")
                    .try_into()
                    .expect("span start fits u32"),
                entry["span"]["end"]
                    .as_u64()
                    .expect("span end")
                    .try_into()
                    .expect("span end fits u32"),
            ),
        );
        let fingerprint = orv_hir::origin_fingerprint(&kind, &name, span);
        let new_id = orv_hir::origin_id(&kind, &name, span);
        entry["fingerprint"] = serde_json::json!(fingerprint);
        entry["id"] = serde_json::json!(new_id);
        (old_id, entry["id"].clone())
    };
    for edge in origin_map["edges"].as_array_mut().expect("origin edges") {
        if edge["from"] == old_id {
            edge["from"] = new_id.clone();
        }
        if edge["to"] == old_id {
            edge["to"] = new_id.clone();
        }
    }
}

fn corrupt_origin_entry_kind_and_graph(build_dir: &Path, origin_id: &str, kind: &str, name: &str) {
    let origin_map_path = build_dir.join("origin-map.json");
    let mut origin_map = read_json_value(&origin_map_path).expect("origin map");
    let entry = origin_map["entries"]
        .as_array_mut()
        .expect("origin entries")
        .iter_mut()
        .find(|entry| entry["id"] == origin_id)
        .expect("origin entry");
    entry["kind"] = serde_json::json!(kind);
    entry["name"] = serde_json::json!(name);
    write_json(&origin_map_path, &origin_map).expect("write corrupt origin map");

    let graph_path = build_dir.join("project-graph.json");
    let mut graph = read_json_value(&graph_path).expect("project graph");
    graph["semantic"]["origin_map"] = origin_map;
    write_json(&graph_path, &graph).expect("write corrupt graph origin map");
}

fn workspace_build_fixture(name: &str) -> PathBuf {
    let root = temp_output_dir(name);
    std::fs::create_dir_all(root.join("apps/web/src")).expect("create web src");
    std::fs::create_dir_all(root.join("shared/models/src")).expect("create models src");
    std::fs::write(
        root.join("orv.toml"),
        r#"[workspace]
resolver = "2"
members = ["apps/web", "shared/models"]
"#,
    )
    .expect("write root manifest");
    std::fs::write(
        root.join("apps/web/orv.toml"),
        r#"[project]
name = "web"
version = "0.1.0"
entry = "src/main.orv"

[dependencies]
models = { path = "../../shared/models", version = "0.1.0" }
"#,
    )
    .expect("write web manifest");
    std::fs::write(
        root.join("shared/models/orv.toml"),
        r#"[project]
name = "models"
version = "0.1.0"
entry = "src/main.orv"
"#,
    )
    .expect("write models manifest");
    std::fs::write(
        root.join("apps/web/src/main.orv"),
        r#"@out @html { @body { @h1 "Web" } }"#,
    )
    .expect("write web source");
    std::fs::write(
        root.join("shared/models/src/main.orv"),
        r#"@out @html { @body { @h1 "Models" } }"#,
    )
    .expect("write models source");
    root
}

fn send_raw_http(address: &str, path: &str) -> String {
    let mut last_error = None;
    for _ in 0..20 {
        match send_raw_http_once(address, path) {
            Ok(response) if !response.is_empty() => return response,
            Ok(_) => last_error = Some("empty response".to_string()),
            Err(err) => last_error = Some(err.to_string()),
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    panic!(
        "read http response: {}",
        last_error.unwrap_or_else(|| "no response".to_string())
    );
}

fn send_raw_http_once(address: &str, path: &str) -> std::io::Result<String> {
    let mut stream = std::net::TcpStream::connect(address)?;
    std::io::Write::write_all(
        &mut stream,
        format!("GET {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n").as_bytes(),
    )?;
    let mut response = String::new();
    std::io::Read::read_to_string(&mut stream, &mut response)?;
    Ok(response)
}

fn send_raw_http_json_post(address: &str, path: &str, body: &str) -> String {
    let mut last_error = None;
    for _ in 0..20 {
        match send_raw_http_json_post_once(address, path, body) {
            Ok(response) if !response.is_empty() => return response,
            Ok(_) => last_error = Some("empty response".to_string()),
            Err(err) => last_error = Some(err.to_string()),
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    panic!(
        "read http response: {}",
        last_error.unwrap_or_else(|| "no response".to_string())
    );
}

fn send_raw_http_json_post_once(address: &str, path: &str, body: &str) -> std::io::Result<String> {
    let mut stream = std::net::TcpStream::connect(address)?;
    std::io::Write::write_all(
            &mut stream,
            format!(
                "POST {path} HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            )
            .as_bytes(),
        )?;
    let mut response = String::new();
    std::io::Read::read_to_string(&mut stream, &mut response)?;
    Ok(response)
}

struct ChildGuard(std::process::Child);

impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn spawn_one_shot_http_json(path: &'static str, body: Vec<u8>) -> (String, JoinHandle<()>) {
    spawn_one_shot_http_json_with_optional_auth(path, body, None)
}

fn spawn_one_shot_http_json_with_auth(
    path: &'static str,
    body: Vec<u8>,
    expected_authorization: &'static str,
) -> (String, JoinHandle<()>) {
    spawn_one_shot_http_json_with_optional_auth(path, body, Some(expected_authorization))
}

fn spawn_one_shot_http_json_with_optional_auth(
    path: &'static str,
    body: Vec<u8>,
    expected_authorization: Option<&'static str>,
) -> (String, JoinHandle<()>) {
    let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).expect("bind registry");
    let addr = listener.local_addr().expect("registry address");
    let handle = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept registry request");
        let mut request = Vec::new();
        let mut buffer = [0_u8; 512];
        while !request.windows(4).any(|window| window == b"\r\n\r\n") && request.len() < 4096 {
            let read =
                std::io::Read::read(&mut stream, &mut buffer).expect("read registry request");
            if read == 0 {
                break;
            }
            request.extend_from_slice(&buffer[..read]);
        }
        let request = String::from_utf8_lossy(&request);
        assert!(
            request.starts_with(&format!("GET {path} HTTP/1.1")),
            "{request}"
        );
        if let Some(expected_authorization) = expected_authorization {
            assert!(
                request
                    .lines()
                    .any(|line| line == format!("Authorization: {expected_authorization}")),
                "{request}"
            );
        }
        let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
        std::io::Write::write_all(&mut stream, response.as_bytes())
            .expect("write registry response head");
        std::io::Write::write_all(&mut stream, &body).expect("write registry response body");
    });
    (format!("http://{addr}"), handle)
}

fn dap_test_request(
    session: &mut DapSession,
    seq: u64,
    command: &str,
    arguments: serde_json::Value,
) -> serde_json::Value {
    let mut request = serde_json::json!({
        "seq": seq,
        "type": "request",
        "command": command,
    });
    request["arguments"] = arguments;
    session
        .message_response(&request)
        .unwrap_or_else(|| panic!("{command} response"))
}

fn prod_server_source(name: &str) -> (PathBuf, PathBuf) {
    let dir = temp_output_dir(name);
    std::fs::create_dir_all(&dir).expect("create prod source dir");
    let path = dir.join("app.orv");
    std::fs::write(
        &path,
        "@server { @listen 8080 @route GET /ping { @respond 200 { ok: true } } }\n",
    )
    .expect("write prod source");
    (dir, path)
}

fn multi_route_prod_server_source(name: &str) -> (PathBuf, PathBuf) {
    let dir = temp_output_dir(name);
    std::fs::create_dir_all(&dir).expect("create multi-route prod source dir");
    let path = dir.join("app.orv");
    std::fs::write(
        &path,
        r#"@server {
  @listen 8080
  @route GET /ping { @respond 200 { ok: true } }
  @route GET /status { @respond 200 { status: "ok" } }
}
"#,
    )
    .expect("write multi-route prod source");
    (dir, path)
}

fn imported_prod_server_source(name: &str) -> (PathBuf, PathBuf) {
    let dir = temp_output_dir(name);
    let models = dir.join("models");
    std::fs::create_dir_all(&models).expect("create imported prod source dir");
    std::fs::write(
        models.join("status.orv"),
        r#"pub function status(): string -> "ok"
"#,
    )
    .expect("write imported helper source");
    let path = dir.join("app.orv");
    std::fs::write(
        &path,
        r#"import models.status.status

@server {
  @listen 8080
  @route GET /ping { @respond 200 { status: status() } }
}
"#,
    )
    .expect("write imported prod source");
    (dir, path)
}

fn env_prod_server_source(name: &str) -> (PathBuf, PathBuf) {
    let dir = temp_output_dir(name);
    std::fs::create_dir_all(&dir).expect("create env prod source dir");
    let path = dir.join("app.orv");
    std::fs::write(
        &path,
        r#"@server {
  @listen int.from(@env.PORT ?? "8080")
  @route GET /ping { @respond 200 { ok: true } }
}
"#,
    )
    .expect("write env prod source");
    (dir, path)
}

fn json_routes_include(routes: &serde_json::Value, method: &str, path: &str) -> bool {
    routes.as_array().is_some_and(|routes| {
        routes
            .iter()
            .any(|route| route["method"] == method && route["path"] == path)
    })
}

fn json_route<'a>(
    routes: &'a serde_json::Value,
    method: &str,
    path: &str,
) -> Option<&'a serde_json::Value> {
    routes.as_array()?.iter().find(|route| {
        route["method"] == serde_json::json!(method) && route["path"] == serde_json::json!(path)
    })
}

fn native_routes_source_includes(source: &str, method: &str, path: &str) -> bool {
    source.contains(&format!(
        "OrvNativeRoute {{ method: {method:?}, path: {path:?},"
    ))
}

fn protocol_frames(output: &str) -> Vec<serde_json::Value> {
    let mut offset = 0;
    let mut frames = Vec::new();
    while offset < output.len() {
        let tail = &output[offset..];
        let (headers, _) = tail
            .split_once("\r\n\r\n")
            .expect("content-length response frame");
        let content_length = headers
            .lines()
            .find_map(|line| {
                line.strip_prefix("Content-Length: ")
                    .and_then(|value| value.parse::<usize>().ok())
            })
            .expect("content length header");
        let body_start = offset + headers.len() + "\r\n\r\n".len();
        let body_end = body_start + content_length;
        let body = output.get(body_start..body_end).expect("complete body");
        frames.push(serde_json::from_str(body).expect("response json"));
        offset = body_end;
    }
    frames
}

fn protocol_request_frame(body: &serde_json::Value) -> String {
    let body = body.to_string();
    format!("Content-Length: {}\r\n\r\n{}", body.len(), body)
}

const MIXED_DYNAMIC_RESPONSE_SOURCE: &str = r#"@server {
  @listen 8080
  @route POST /orders {
    @respond 201 { sku: @body.sku, coupon: @query.coupon }
  }
  @route POST /sessions {
    @respond 201 { matches: @body.token == @query.token }
  }
  @route POST /labels {
    @respond 201 { label: @body.first + @query.suffix }
  }
  @route POST /sku-labels {
    @respond 201 { label: "sku-{@body.sku}-v1" }
  }
  @route POST /joined-labels {
    @respond 201 { label: "{@body.first}-{@query.suffix}" }
  }
  @route POST /quantities {
    @respond 201 { next: 1 + (@body.quantity as int) }
  }
}
"#;

const MIXED_DYNAMIC_SERVER_SOURCE: &str = r#"@server {
  @listen 8080
  @route POST /orders {
    @respond 201 { sku: @body.sku, coupon: @query.coupon }
  }
  @route POST /sessions {
    @respond 201 { matches: @body.token == @query.token }
  }
  @route POST /labels {
    @respond 201 { label: @body.first + @query.suffix }
  }
  @route POST /sku-labels {
    @respond 201 { label: "sku-{@body.sku}-v1" }
  }
  @route POST /joined-labels {
    @respond 201 { label: "{@body.first}-{@query.suffix}" }
  }
  @route POST /quantities {
    @respond 201 { next: 1 + (@body.quantity as int) }
  }
  @route POST /quantity-doubles {
    @respond 201 { doubled: 2 * (@body.quantity as int) }
  }
  @route POST /quantity-limits {
    @respond 201 { below_limit: 10 > (@body.quantity as int) }
  }
}
"#;

fn assert_manifest_artifact(path: &Path, kind: &str, artifact_path: &str) {
    let manifest = read_json_value(path).expect("build manifest");
    assert!(
        manifest["artifacts"]
            .as_array()
            .expect("manifest artifacts")
            .iter()
            .any(|artifact| artifact["kind"] == kind && artifact["path"] == artifact_path),
        "missing manifest artifact {kind}"
    );
}

fn assert_bundle_target(path: &Path, kind: &str, target_path: &str) {
    let plan = read_json_value(path).expect("bundle plan");
    assert!(
        plan["bundles"]
            .as_array()
            .expect("bundle targets")
            .iter()
            .any(|bundle| bundle["kind"] == kind && bundle["path"] == target_path),
        "missing bundle target {kind}"
    );
}

fn assert_client_loader_contract(loader: &str) {
    for expected in [
        "ORV_CLIENT_BOOTSTRAP",
        "sourceBundleUrl",
        "../source-bundle.json",
        "sourceBundleHash",
        "sourceFileCount",
        "manifestUrl",
        "loadClientManifest",
        "client manifest hash mismatch",
        "validateWasmBundle",
        "client wasm hash mismatch",
        "reactivePlanUrl",
        "loadReactivePlan",
        "embeddedReactivePlan",
        "embeddedReactivePlanHash",
        "loadEmbeddedReactivePlan",
        "validateReactivePlan",
        "client embedded reactive plan hash mismatch",
        "validateReactiveBindings",
        "client reactive plan hash mismatch",
        "client reactive plan initial_render binding mismatch",
        "client reactive plan signal_state binding mismatch",
        "client reactive plan signal_text binding mismatch",
        "client reactive plan signal_attr binding mismatch",
        "client reactive plan signal_event binding mismatch",
        "renderSignalTextBinding",
        "text_template",
        "renderSignalTextCondition",
        "text_condition",
        "signalTextBindingStateKeys",
        "signalTextBindingCursorKey",
        "state_keys",
        "renderSignalAttrBinding",
        "attr_template",
        "signalAttrBindingStateKeys",
        "signalAttrBindingCursorKey",
        "renderSignalAttrCondition",
        "attr_condition",
        "compareSignalAttrCondition",
        "decodeSignalConditionOperand",
        "createReactiveState",
        "bindReactiveDom",
        "bindReactiveAttrs",
        "bindReactiveEvents",
        "applySignalAction",
        "assign_add",
        "assign_sub",
        "assign_toggle",
        "assign_event_target_checked",
        "assign_event_target_value",
        "assign_event_target_value_float",
        "assign_event_target_value_int",
        "setSignal",
        "loadSourceBundle",
        "sourceFileCount",
        "fnv1a64",
        "source bundle hash mismatch",
        "runtimeFeatures",
        "WebAssembly.instantiate",
        "validateInitialRender",
        "initial_render",
        "client initial render hash mismatch",
        "orv_start",
        "orv_render_ptr",
        "orv_render_len",
        "TextDecoder",
        "#orv-root",
        "initialRenderMountHtml",
        "DOMParser",
        "root.innerHTML",
        "app.wasm",
        "orvReactiveSignals",
        "orvReactiveBindings",
        "orvReactiveDomBindings",
        "orvReactiveAttrBindings",
        "orvReactiveEventBindings",
        "__ORV_CLIENT_REACTIVE_STATE__",
        "__ORV_SET_SIGNAL__",
    ] {
        assert!(
            loader.contains(expected),
            "missing loader snippet {expected}"
        );
    }
}

fn client_loader_bootstrap_json(loader: &str) -> serde_json::Value {
    let start_marker = "Object.freeze(";
    let start = loader.find(start_marker).expect("bootstrap start") + start_marker.len();
    let end = loader[start..]
        .find(");\n\nconst manifestUrl")
        .expect("bootstrap end")
        + start;
    serde_json::from_str(&loader[start..end]).expect("bootstrap json")
}

fn fill_benchmark_participant_runs(evidence: &mut serde_json::Value) {
    evidence["data"]["participant_runs"] = serde_json::json!([
        {
            "run_id": "run-1",
            "participant_id": "participant-1",
            "participant_profile": deploy_benchmark::PARTICIPANT_PROFILE_NON_DEVELOPER,
            "status": "passed",
            "started_at": "2026-05-18T09:00:00Z",
            "completed_at": "2026-05-18T10:30:00Z",
            "raw_notes_artifact": "evidence/participant-1.md",
            "raw_notes_sha256": null,
        },
        {
            "run_id": "run-2",
            "participant_id": "participant-2",
            "participant_profile": deploy_benchmark::PARTICIPANT_PROFILE_NON_DEVELOPER,
            "status": "passed",
            "started_at": "2026-05-18T11:00:00Z",
            "completed_at": "2026-05-18T12:20:00Z",
            "raw_notes_artifact": "evidence/participant-2.md",
            "raw_notes_sha256": null,
        },
    ]);
}

fn fill_benchmark_task_entries(evidence: &mut serde_json::Value) {
    for (index, entry) in evidence["task_entries"]
        .as_array_mut()
        .expect("task entries")
        .iter_mut()
        .enumerate()
    {
        entry["elapsed_minutes"] = serde_json::json!(10.0);
        entry["status"] = serde_json::json!("passed");
        entry["notes"] = serde_json::json!(format!("task {} completed", index + 1));
    }
}

fn write_benchmark_participant_note_artifacts(out: &Path, evidence: &mut serde_json::Value) {
    let evidence_dir = out.join("evidence");
    std::fs::create_dir_all(&evidence_dir).expect("create participant evidence dir");
    let participant_1_path = evidence_dir.join("participant-1.md");
    std::fs::write(
        &participant_1_path,
        "# Shop Benchmark Participant Notes\n\n## Participant\n\n- participant_id: participant-1\n- run_id: run-1\n- started_at: 2026-05-18T09:00:00Z\n- completed_at: 2026-05-18T10:30:00Z\n\n## Task Notes\n\nParticipant 1 completed the shop flow and retained real observations.\n\n## Evidence Review\n\n- failure_classification.primary: none\n- failure_classification.notes: no blockers\n",
    )
    .expect("write participant 1 notes");
    let participant_2_path = evidence_dir.join("participant-2.md");
    std::fs::write(
        &participant_2_path,
        "# Shop Benchmark Participant Notes\n\n## Participant\n\n- participant_id: participant-2\n- run_id: run-2\n- started_at: 2026-05-18T11:00:00Z\n- completed_at: 2026-05-18T12:20:00Z\n\n## Task Notes\n\nParticipant 2 completed the shop flow and retained real observations.\n\n## Evidence Review\n\n- failure_classification.primary: none\n- failure_classification.notes: no blockers\n",
    )
    .expect("write participant 2 notes");
    let participant_1_bytes = std::fs::read(participant_1_path).expect("read participant 1 notes");
    let participant_2_bytes = std::fs::read(participant_2_path).expect("read participant 2 notes");
    evidence["data"]["participant_runs"][0]["raw_notes_sha256"] =
        serde_json::json!(format!("sha256:{}", sha256_hex(&participant_1_bytes)));
    evidence["data"]["participant_runs"][1]["raw_notes_sha256"] =
        serde_json::json!(format!("sha256:{}", sha256_hex(&participant_2_bytes)));
}

fn canonical_build_dir_string(out: &Path) -> String {
    std::fs::canonicalize(out)
        .expect("canonical build dir")
        .display()
        .to_string()
}

fn benchmark_smoke_output_for(out: &Path, server_routes: u64) -> String {
    format!(
        "orv deploy smoke test passed\nbuild_dir={}\nbase_url=http://127.0.0.1:8080\ngraph_contract=verified\ndap_summary=verified\ndap_source_bundle=verified\nserver_routes={server_routes}\ntrace_stream_requested=1\n",
        canonical_build_dir_string(out)
    )
}

fn fill_benchmark_report_observation_data(evidence: &mut serde_json::Value) {
    evidence["data"]["docs_help_lookups"] = serde_json::json!(1);
    evidence["data"]["compiler_runtime_errors"] = serde_json::json!(0);
    evidence["data"]["ai_assistance_used"] = serde_json::json!(false);
    evidence["data"]["generated_artifact_edits"] = serde_json::json!(false);
    evidence["data"]["manual_undocumented_security_steps"] = serde_json::json!(false);
    evidence["data"]["manual_config_edits"] = serde_json::json!([]);
    evidence["data"]["participant_notes"] = serde_json::json!("required observation data");
    fill_benchmark_human_evidence_review(evidence);
    evidence["data"]["smoke_test_output"] = serde_json::json!(
        "orv deploy smoke test passed\nbuild_dir=/tmp/orv-build\nbase_url=http://127.0.0.1:8080\ngraph_contract=verified\ndap_summary=verified\ndap_source_bundle=verified\nserver_routes=1\ntrace_stream_requested=1\n"
    );
    fill_benchmark_participant_runs(evidence);
}

fn fill_benchmark_human_evidence_review(evidence: &mut serde_json::Value) {
    evidence["data"]["human_evidence_review"] = serde_json::json!({
        "reviewer": "benchmark-reviewer",
        "reviewed_at": "2026-05-18T17:00:00Z",
        "raw_notes_reviewed": true,
        "smoke_output_reviewed": true,
        "participant_identity_reviewed": true,
        "no_ai_assistance_confirmed": true,
        "notes": "reviewed retained participant notes, smoke output, participant identities, and no-AI evidence",
    });
}

fn shop_benchmark_report_passed_golden() -> serde_json::Value {
    serde_json::from_str(include_str!(
        "../../../docs/samples/shop-benchmark-report-passed-v1.golden.json"
    ))
    .expect("shop benchmark passed report golden")
}

fn benchmark_report_passed_inventory(report: &serde_json::Value) -> serde_json::Value {
    let raw_notes = report["data"]["participant_raw_notes_artifacts"]
        .as_array()
        .expect("participant raw notes artifacts");
    serde_json::json!({
        "schema_version": 1,
        "kind": "orv.shop_benchmark_report.passed_inventory",
        "status": report["status"].clone(),
        "contract_verified": report["contract_verified"].clone(),
        "time_over_limit": report["time_over_limit"].clone(),
        "max_elapsed_minutes": report["max_elapsed_minutes"].clone(),
        "total_elapsed_minutes": report["total_elapsed_minutes"].clone(),
        "smoke_output_contract": report["smoke_output_contract"].clone(),
        "tasks": {
            "task_count": report["tasks"]["task_count"].clone(),
            "recorded_task_count": report["tasks"]["recorded_task_count"].clone(),
            "missing_task_count": report["tasks"]["missing_task_count"].clone(),
            "failed_task_count": report["tasks"]["failed_task_count"].clone(),
        },
        "data": {
            "missing_data_count": report["data"]["missing_data"].as_array().map_or(0, Vec::len),
            "failed_data_count": report["data"]["failed_data"].as_array().map_or(0, Vec::len),
            "smoke_test_required_markers": report["data"]["smoke_test_required_markers"].clone(),
            "human_evidence_review": report["data"]["human_evidence_review"].clone(),
            "smoke_summary": {
                "passed_marker": report["data"]["smoke_test_summary"]["passed_marker"].clone(),
                "graph_contract_verified": report["data"]["smoke_test_summary"]["graph_contract_verified"].clone(),
                "dap_summary_verified": report["data"]["smoke_test_summary"]["dap_summary_verified"].clone(),
                "dap_source_bundle_verified": report["data"]["smoke_test_summary"]["dap_source_bundle_verified"].clone(),
                "server_routes": report["data"]["smoke_test_summary"]["server_routes"].clone(),
                "trace_stream_requested": report["data"]["smoke_test_summary"]["trace_stream_requested"].clone(),
                "missing_marker_count": report["data"]["smoke_test_summary"]["missing_markers"].as_array().map_or(0, Vec::len),
                "duplicate_field_count": report["data"]["smoke_test_summary"]["duplicate_fields"].as_array().map_or(0, Vec::len),
            },
            "participant_summary": report["data"]["participant_summary"].clone(),
            "raw_notes_artifacts": raw_notes.iter().map(|artifact| {
                serde_json::json!({
                    "path": artifact["path"].clone(),
                    "path_safe": artifact["path_safe"].clone(),
                    "checked": artifact["checked"].clone(),
                    "retained": artifact["retained"].clone(),
                    "non_empty": artifact["non_empty"].clone(),
                    "template_filled": artifact["template_filled"].clone(),
                    "identity_match": artifact["identity_match"].clone(),
                    "expected_sha256": artifact["expected_sha256"].clone(),
                    "actual_sha256": artifact["actual_sha256"].clone(),
                    "sha256_match": artifact["sha256_match"].clone(),
                    "size_positive": artifact["size_bytes"].as_u64().is_some_and(|size| size > 0),
                })
            }).collect::<Vec<_>>(),
        },
    })
}

fn corrupt_generated_render_len_const(wasm: &mut [u8], replacement: u8) {
    let mut offset = WASM_MODULE_HEADER.len();
    while offset < wasm.len() {
        let section_id = wasm[offset];
        offset += 1;
        let section_len =
            read_wasm_u32_leb(wasm, &mut offset, wasm.len()).expect("section length") as usize;
        let section_end = offset + section_len;
        if section_id == 10 {
            let mut body_offset = offset;
            let function_count =
                read_wasm_u32_leb(wasm, &mut body_offset, section_end).expect("function count");
            assert_eq!(function_count, 3);
            for ordinal in 0..function_count {
                let body_len = read_wasm_u32_leb(wasm, &mut body_offset, section_end)
                    .expect("body len") as usize;
                let body_start = body_offset;
                let body_end = body_start + body_len;
                if ordinal == 2 {
                    assert_eq!(wasm[body_start], 0x00);
                    assert_eq!(wasm[body_start + 1], 0x41);
                    assert_eq!(wasm[body_end - 1], 0x0b);
                    wasm[body_start + 2] = replacement;
                    return;
                }
                body_offset = body_end;
            }
        }
        offset = section_end;
    }
    panic!("render_len function body not found");
}

fn corrupt_generated_memory_export_kind(wasm: &mut [u8], replacement: u8) {
    let Some(position) = wasm
        .windows(CLIENT_WASM_MEMORY_EXPORT.len())
        .rposition(|window| window == CLIENT_WASM_MEMORY_EXPORT.as_bytes())
    else {
        panic!("memory export name not found");
    };
    let kind_offset = position + CLIENT_WASM_MEMORY_EXPORT.len();
    assert_eq!(wasm[kind_offset], 2);
    wasm[kind_offset] = replacement;
}

fn corrupt_generated_start_export_index(wasm: &mut [u8], replacement: u8) {
    let Some(position) = wasm
        .windows(CLIENT_WASM_START_EXPORT.len())
        .rposition(|window| window == CLIENT_WASM_START_EXPORT.as_bytes())
    else {
        panic!("start export name not found");
    };
    let index_offset = position + CLIENT_WASM_START_EXPORT.len() + 1;
    assert_eq!(wasm[index_offset], 0);
    wasm[index_offset] = replacement;
}

fn corrupt_generated_memory_export_index(wasm: &mut [u8], replacement: u8) {
    let Some(position) = wasm
        .windows(CLIENT_WASM_MEMORY_EXPORT.len())
        .rposition(|window| window == CLIENT_WASM_MEMORY_EXPORT.as_bytes())
    else {
        panic!("memory export name not found");
    };
    let index_offset = position + CLIENT_WASM_MEMORY_EXPORT.len() + 1;
    assert_eq!(wasm[index_offset], 0);
    wasm[index_offset] = replacement;
}

fn refresh_client_manifest_wasm_hash(build_out: &Path) {
    let manifest_path = build_out.join(CLIENT_MANIFEST_PATH);
    let mut manifest = read_json_value(&manifest_path).expect("client manifest");
    let wasm_hash = file_content_hash(&build_out.join(CLIENT_WASM_PATH)).expect("client wasm hash");
    manifest["wasm_hash"] = serde_json::json!(wasm_hash);
    write_json(&manifest_path, &manifest).expect("rewrite client manifest wasm hash");
}

fn refresh_client_manifest_loader_hash(build_out: &Path) {
    let manifest_path = build_out.join(CLIENT_MANIFEST_PATH);
    let mut manifest = read_json_value(&manifest_path).expect("client manifest");
    let loader_hash =
        file_content_hash(&build_out.join(CLIENT_JS_PATH)).expect("client loader hash");
    manifest["loader_hash"] = serde_json::json!(loader_hash);
    write_json(&manifest_path, &manifest).expect("rewrite client manifest loader hash");
}

fn refresh_client_manifest_reactive_plan_hash(build_out: &Path) {
    let manifest_path = build_out.join(CLIENT_MANIFEST_PATH);
    let mut manifest = read_json_value(&manifest_path).expect("client manifest");
    let reactive_plan =
        read_json_value(&build_out.join(CLIENT_REACTIVE_PLAN_PATH)).expect("reactive plan");
    let reactive_plan_hash = stable_json_hash(&reactive_plan).expect("reactive plan hash");
    manifest["reactive_plan_hash"] = serde_json::json!(reactive_plan_hash);
    write_json(&manifest_path, &manifest).expect("rewrite client manifest reactive plan hash");
}

fn editor_trace_test_frame_for(method: &str, path: &str, status: u16) -> serde_json::Value {
    serde_json::json!({
        "method": method,
        "path": path,
        "status": status,
        "route_method": null,
        "route_path": null,
        "route_origin_id": null,
        "response_origin_id": null,
        "params": {},
        "query": {},
        "body": "",
    })
}

fn editor_trace_test_frame() -> serde_json::Value {
    editor_trace_test_frame_for("GET", "/ping", 200)
}

fn editor_trace_test_route_frame(route_origin_id: &str) -> serde_json::Value {
    let mut frame = editor_trace_test_frame();
    frame["route_method"] = serde_json::json!("GET");
    frame["route_path"] = serde_json::json!("/ping");
    frame["route_origin_id"] = serde_json::json!(route_origin_id);
    frame
}

fn assert_editor_debug_runner_artifact(out: &Path, state: &serde_json::Value) {
    let runner =
        read_json_value(&out.join(EDITOR_DEBUG_SESSION_RUNNER_PATH)).expect("debug runner");
    assert_eq!(runner, state["debug"]["session_runner"]);
    assert_eq!(runner["result"]["path"], EDITOR_DEBUG_SESSION_RESULT_PATH);
    let run = editor_debug_runner_session_json(
        &out.join(EDITOR_DEBUG_SESSION_RUNNER_PATH),
        &[EditorDebugControl::Next],
        &[],
        &[],
        &[],
        &[],
        &[],
    )
    .expect("run standalone debug runner");
    assert_eq!(run["kind"], "orv.editor.debug.runner.result");
    assert_eq!(run["runner"], runner);
}

fn assert_editor_native_host_manifest(out: &Path, state: &serde_json::Value) {
    let native_host =
        read_json_value(&out.join(EDITOR_NATIVE_HOST_MANIFEST_PATH)).expect("native host");
    assert_eq!(native_host["kind"], "orv.editor.native_host");
    assert_eq!(native_host["artifacts"]["shell"], "index.html");
    assert_eq!(native_host["artifacts"]["state"], "state.json");
    assert_eq!(
        native_host["artifacts"]["debug_session_runner"],
        EDITOR_DEBUG_SESSION_RUNNER_PATH
    );
    assert_eq!(
        native_host["artifacts"]["debug_session_result"],
        EDITOR_DEBUG_SESSION_RESULT_PATH
    );
    assert_eq!(
        native_host["artifacts"]["debug_session_result_html"],
        EDITOR_DEBUG_SESSION_RESULT_HTML_PATH
    );
    assert_eq!(
        native_host["artifacts"]["native_host_bridge_js"],
        EDITOR_NATIVE_HOST_BRIDGE_JS_PATH
    );
    assert_eq!(
        native_host["artifacts"]["native_host_desktop_package"],
        EDITOR_NATIVE_HOST_DESKTOP_PACKAGE_PATH
    );
    assert_eq!(
        native_host["artifacts"]["native_host_desktop_launcher"],
        EDITOR_NATIVE_HOST_DESKTOP_LAUNCHER_PATH
    );
    assert_eq!(
        native_host["artifacts"]["native_host_desktop_packaging"],
        EDITOR_NATIVE_HOST_DESKTOP_PACKAGING_PATH
    );
    assert_eq!(
        native_host["artifacts"]["native_host_desktop_package_script"],
        EDITOR_NATIVE_HOST_DESKTOP_PACKAGE_SCRIPT_PATH
    );
    assert_eq!(
        native_host["artifacts"]["native_host_desktop_app_package"],
        EDITOR_NATIVE_HOST_DESKTOP_APP_PACKAGE_PATH
    );
    assert_eq!(
        native_host["artifacts"]["native_host_desktop_app_info_plist"],
        EDITOR_NATIVE_HOST_DESKTOP_APP_INFO_PLIST_PATH
    );
    assert_eq!(
        native_host["artifacts"]["native_host_desktop_app_entitlements"],
        EDITOR_NATIVE_HOST_DESKTOP_APP_ENTITLEMENTS_PATH
    );
    assert_eq!(
        native_host["artifacts"]["native_host_desktop_app_main"],
        EDITOR_NATIVE_HOST_DESKTOP_APP_MAIN_PATH
    );
    assert_eq!(native_host["capabilities"]["native_host_bridge"], true);
    let bridge =
        std::fs::read_to_string(out.join(EDITOR_NATIVE_HOST_BRIDGE_JS_PATH)).expect("bridge js");
    assert!(bridge.contains("window.orvNativeHost"));
    assert!(bridge.contains("webkit.messageHandlers.orvNativeHost"));
    assert!(bridge.contains("chrome.webview.postMessage"));
    assert!(bridge.contains("/__orv/native-host/action"));
    assert!(bridge.contains("fetch(endpoint"));
    assert!(bridge.contains("orv:trace-action-result"));
    assert!(bridge.contains("orv:native-host-command"));
    assert!(bridge.contains("orv:source-permission-blocked"));
    assert!(bridge.contains("orvNativeHostSourcePermissions"));
    assert!(bridge.contains("sourceRevealAllowed"));
    assert!(bridge.contains("trace/action-result.html"));
    assert_eq!(
        native_host["host"]["action_endpoint"],
        "/__orv/native-host/action"
    );
    assert_eq!(native_host["host"]["command_format"][2], "host");
    assert_eq!(
        native_host["host"]["desktop_package"],
        EDITOR_NATIVE_HOST_DESKTOP_PACKAGE_PATH
    );
    assert_eq!(
        native_host["host"]["desktop_launcher"],
        EDITOR_NATIVE_HOST_DESKTOP_LAUNCHER_PATH
    );
    assert_eq!(
        native_host["capabilities"]["native_host_local_bridge"],
        true
    );
    assert_eq!(
        native_host["capabilities"]["native_host_desktop_package"],
        true
    );
    assert_eq!(native_host["capabilities"]["native_host_desktop_app"], true);
    assert_eq!(
        native_host["capabilities"]["native_host_desktop_packaging"],
        true
    );
    assert_eq!(
        native_host["capabilities"]["native_host_desktop_platform_matrix"],
        true
    );
    let desktop_package = read_json_value(&out.join(EDITOR_NATIVE_HOST_DESKTOP_PACKAGE_PATH))
        .expect("desktop package");
    assert_eq!(
        desktop_package["kind"],
        "orv.editor.native_host.desktop_package"
    );
    assert_eq!(desktop_package["runtime"], "local-http-bridge");
    assert_eq!(
        desktop_package["artifacts"]["manifest"],
        EDITOR_NATIVE_HOST_MANIFEST_PATH
    );
    assert_eq!(
        desktop_package["artifacts"]["desktop_app_package"],
        EDITOR_NATIVE_HOST_DESKTOP_APP_PACKAGE_PATH
    );
    assert_eq!(
        desktop_package["artifacts"]["desktop_packaging"],
        EDITOR_NATIVE_HOST_DESKTOP_PACKAGING_PATH
    );
    assert_eq!(
        desktop_package["platform_matrix"]["kind"],
        "orv.editor.native_host.desktop_platform_matrix"
    );
    assert_eq!(desktop_package["platform_matrix"]["implemented_count"], 1);
    assert_eq!(desktop_package["platform_matrix"]["planned_count"], 2);
    assert!(desktop_package["platform_matrix"]["targets"]
        .as_array()
        .expect("desktop platform targets")
        .iter()
        .any(|target| target["platform"] == "macos"
            && target["status"] == "implemented"
            && target["packaging"]["script"] == EDITOR_NATIVE_HOST_DESKTOP_PACKAGE_SCRIPT_PATH));
    assert!(desktop_package["platform_matrix"]["targets"]
        .as_array()
        .expect("desktop platform targets")
        .iter()
        .any(|target| target["platform"] == "windows"
            && target["status"] == "planned"
            && target["container"] == "WebView2"
            && target["blocked_by"]
                .as_array()
                .expect("windows blockers")
                .iter()
                .any(|blocker| blocker["id"] == "windows-webview2-container")));
    assert!(desktop_package["platform_matrix"]["targets"]
        .as_array()
        .expect("desktop platform targets")
        .iter()
        .any(|target| target["platform"] == "linux"
            && target["status"] == "planned"
            && target["container"] == "WebKitGTK or Tauri/WebView runtime"
            && target["blocked_by"]
                .as_array()
                .expect("linux blockers")
                .iter()
                .any(|blocker| blocker["id"] == "linux-webview-container")));
    assert_eq!(
        desktop_package["packaging"]["bundle"]["info_plist"],
        EDITOR_NATIVE_HOST_DESKTOP_APP_INFO_PLIST_PATH
    );
    assert_eq!(
        desktop_package["packaging"]["codesign"]["identity_env"],
        "ORV_EDITOR_CODESIGN_IDENTITY"
    );
    assert_eq!(
        desktop_package["packaging"]["notarization"]["status"],
        "optional"
    );
    assert_eq!(
        desktop_package["packaging"]["notarization"]["enable_env"],
        "ORV_EDITOR_NOTARIZE"
    );
    assert_eq!(
        desktop_package["packaging"]["notarization"]["profile_env"],
        "ORV_EDITOR_NOTARY_PROFILE"
    );
    assert_eq!(
        desktop_package["packaging"]["notarization"]["apple_id_env"],
        "ORV_EDITOR_NOTARY_APPLE_ID"
    );
    assert_eq!(
        desktop_package["packaging"]["notarization"]["password_env"],
        "ORV_EDITOR_NOTARY_PASSWORD"
    );
    assert_eq!(
        desktop_package["packaging"]["notarization"]["team_id_env"],
        "ORV_EDITOR_NOTARY_TEAM_ID"
    );
    assert_eq!(
        desktop_package["desktop_app"]["run_command"],
        serde_json::json!([
            "swift",
            "run",
            "--package-path",
            "native-host/desktop-app",
            "OrvEditorDesktop",
            EDITOR_NATIVE_HOST_DESKTOP_SESSION_PATH,
        ])
    );
    assert_eq!(
        native_host["host"]["desktop_app"]["product"],
        "OrvEditorDesktop"
    );
    assert_eq!(
        native_host["host"]["desktop_platform_matrix"],
        desktop_package["platform_matrix"]
    );
    assert_eq!(
        native_host["host"]["desktop_app"]["capabilities"]["source_permission_denied_mode"],
        "open-read-only"
    );
    assert_eq!(
        desktop_package["desktop_app"]["packaging"]["script"],
        EDITOR_NATIVE_HOST_DESKTOP_PACKAGE_SCRIPT_PATH
    );
    assert_eq!(
        desktop_package["lifecycle"]["spawn"]["command"],
        serde_json::json!(["orv", "editor", "host", ".", "--listen", "127.0.0.1:0"])
    );
    assert_eq!(
        desktop_package["lifecycle"]["webview"]["initial_url_template"],
        "{url}index.html"
    );
    assert_eq!(
        desktop_package["process_policy"]["deny_unknown_commands"],
        true
    );
    assert!(desktop_package["process_policy"]["allowed_commands"]
        .as_array()
        .expect("allowed commands")
        .iter()
        .any(
            |command| command["name"] == "debug_runner" && command["argv_prefix"][2] == "run-debug"
        ));
    assert!(desktop_package["source_permissions"]["allowed_roots"]
        .as_array()
        .expect("allowed roots")
        .iter()
        .any(|root| root.as_str().is_some_and(|root| !root.is_empty())));
    assert_eq!(
        desktop_package["source_permissions"]["mode"],
        "prompt-before-source-reveal"
    );
    assert_eq!(
        desktop_package["source_permissions"]["denied_mode"],
        "open-read-only"
    );
    assert_eq!(
        desktop_package["source_permissions"]["webview_injection"],
        "orvNativeHostSourcePermissions"
    );
    assert_eq!(
        desktop_package["source_permissions"]["blocked_event"],
        "orv:source-permission-blocked"
    );
    assert!(desktop_package["source_permissions"]["root_count"]
        .as_u64()
        .is_some_and(|count| count > 0));
    assert!(desktop_package["source_permissions"]["source_hashes"]
        .as_array()
        .expect("source hashes")
        .iter()
        .any(|source| source["path"]
            .as_str()
            .is_some_and(|path| path.ends_with("app.orv"))));
    assert!(desktop_package["source_permissions"]["source_count"]
        .as_u64()
        .is_some_and(|count| count > 0));
    let launcher = std::fs::read_to_string(out.join(EDITOR_NATIVE_HOST_DESKTOP_LAUNCHER_PATH))
        .expect("desktop launcher");
    assert!(launcher.contains("orv editor host"));
    assert!(launcher.contains("ORV_EDITOR_HOST_LISTEN"));
    let desktop_app_package =
        std::fs::read_to_string(out.join(EDITOR_NATIVE_HOST_DESKTOP_APP_PACKAGE_PATH))
            .expect("desktop app package");
    let desktop_app_info =
        std::fs::read_to_string(out.join(EDITOR_NATIVE_HOST_DESKTOP_APP_INFO_PLIST_PATH))
            .expect("desktop app info");
    let desktop_app_entitlements =
        std::fs::read_to_string(out.join(EDITOR_NATIVE_HOST_DESKTOP_APP_ENTITLEMENTS_PATH))
            .expect("desktop app entitlements");
    let desktop_app_main =
        std::fs::read_to_string(out.join(EDITOR_NATIVE_HOST_DESKTOP_APP_MAIN_PATH))
            .expect("desktop app main");
    let package_script =
        std::fs::read_to_string(out.join(EDITOR_NATIVE_HOST_DESKTOP_PACKAGE_SCRIPT_PATH))
            .expect("desktop package script");
    let packaging = read_json_value(&out.join(EDITOR_NATIVE_HOST_DESKTOP_PACKAGING_PATH))
        .expect("desktop packaging");
    assert!(desktop_app_package.contains("executableTarget(name: \"OrvEditorDesktop\")"));
    assert!(desktop_app_info.contains("<key>CFBundleExecutable</key>"));
    assert!(desktop_app_info.contains("<string>OrvEditorDesktop</string>"));
    assert!(desktop_app_entitlements.contains("<dict/>"));
    assert!(desktop_app_main.contains("WKWebView"));
    assert!(desktop_app_main.contains("NSAlert"));
    assert!(desktop_app_main.contains("Open Read-Only"));
    assert!(desktop_app_main.contains("WKUserScript"));
    assert!(desktop_app_main.contains("Process()"));
    assert!(desktop_app_main.contains("readReadyJSON"));
    assert!(desktop_app_main.contains("sourcePermissionDecision"));
    assert!(desktop_app_main.contains("orvNativeHostSourcePermissions"));
    assert!(package_script.contains("swift build --package-path"));
    assert!(package_script.contains("codesign --force --options runtime"));
    assert!(package_script.contains("ORV_EDITOR_NOTARIZE"));
    assert!(package_script.contains("ORV_EDITOR_NOTARY_PROFILE"));
    assert!(package_script.contains("ditto -c -k --keepParent"));
    assert!(package_script.contains("xcrun notarytool submit"));
    assert!(package_script.contains("xcrun stapler staple"));
    assert!(package_script.contains("\"notarized\""));
    assert_eq!(
        packaging["bundle"]["path"],
        "native-host/dist/OrvEditorDesktop.app"
    );
    assert_eq!(packaging["notarization"]["status"], "optional");
    assert_eq!(
        packaging["notarization"]["zip_path"],
        "native-host/dist/OrvEditorDesktop.zip"
    );
    let desktop_shell = editor_native_host_desktop_shell_json(out, "127.0.0.1:38123")
        .expect("desktop shell session");
    let canonical_out = out.canonicalize().expect("canonical editor out");
    assert_eq!(
        desktop_shell["kind"],
        "orv.editor.native_host.desktop_shell"
    );
    assert_eq!(desktop_shell["status"], "ready");
    assert_eq!(
        desktop_shell["lifecycle"]["spawn"]["command"],
        serde_json::json!([
            "orv",
            "editor",
            "host",
            canonical_out.display().to_string(),
            "--listen",
            "127.0.0.1:38123",
        ])
    );
    assert_eq!(
        desktop_shell["webview"]["initial_url_template"],
        "{url}index.html"
    );
    assert_eq!(
        desktop_shell["process_supervision"]["deny_unknown_commands"],
        true
    );
    assert_eq!(
        desktop_shell["platform_matrix"],
        desktop_package["platform_matrix"]
    );
    assert_eq!(
        desktop_shell["source_permission_prompt"]["denied_mode"],
        "open-read-only"
    );
    assert_eq!(
        desktop_shell["source_permission_prompt"]["blocked_event"],
        "orv:source-permission-blocked"
    );
    assert!(desktop_shell["source_permission_prompt"]["source_count"]
        .as_u64()
        .is_some_and(|count| count > 0));
    assert!(desktop_shell["artifact_checks"]
        .as_array()
        .expect("artifact checks")
        .iter()
        .any(|check| check["name"] == "launcher"
            && check["path"] == EDITOR_NATIVE_HOST_DESKTOP_LAUNCHER_PATH
            && check["exists"] == true));
    cmd_editor_desktop_shell(out, "127.0.0.1:38124", true).expect("write desktop shell session");
    let desktop_session = read_json_value(&out.join(EDITOR_NATIVE_HOST_DESKTOP_SESSION_PATH))
        .expect("desktop session");
    assert_eq!(
        desktop_session["session_artifact"]["path"],
        EDITOR_NATIVE_HOST_DESKTOP_SESSION_PATH
    );
    assert_eq!(
        desktop_session["lifecycle"]["spawn"]["command"][5],
        "127.0.0.1:38124"
    );
    assert_eq!(
        native_host["debug"]["adapter_command"],
        serde_json::json!(["orv", "dap", "serve", "--stdio"])
    );
    assert_eq!(
        native_host["debug"]["capabilities"],
        state["debug"]["capabilities"]
    );
    assert_eq!(
        native_host["debug"]["source_inventory"],
        state["debug"]["source_inventory"]
    );
    assert_eq!(native_host["debug"]["source_count"], 1);
    assert_eq!(native_host["capabilities"]["dap_sources"], true);
    assert_eq!(
        native_host["debug"]["runner_command"],
        state["debug"]["session_runner"]["command"]
    );
    assert_eq!(native_host["debug"]["breakpoint_argument"], "--breakpoint");
    assert_eq!(native_host["debug"]["breakpoint_format"], "<path>:<line>");
    assert_eq!(
        native_host["debug"]["function_breakpoint_argument"],
        "--function-breakpoint"
    );
    assert_eq!(
        native_host["debug"]["function_breakpoint_format"],
        "<function-name>"
    );
    assert_eq!(
        native_host["debug"]["data_breakpoint_argument"],
        "--data-breakpoint"
    );
    assert_eq!(
        native_host["debug"]["data_breakpoint_format"],
        "<local-name>"
    );
    assert_eq!(
        native_host["debug"]["exception_filter_argument"],
        "--exception-filter"
    );
    assert_eq!(
        native_host["debug"]["exception_filter_format"],
        "<orv.diagnostics|orv.runtime>"
    );
    assert_eq!(
        native_host["debug"]["watch_expression_argument"],
        "--watch-expression"
    );
    assert_eq!(
        native_host["debug"]["watch_expression_format"],
        "<expression>"
    );
    assert_eq!(
        native_host["debug"]["result_path"],
        EDITOR_DEBUG_SESSION_RESULT_PATH
    );
    assert_eq!(
        native_host["debug"]["result_kind"],
        "orv.editor.debug.runner.result"
    );
    assert_eq!(
        native_host["debug"]["result_artifact"],
        state["debug"]["result_artifact"]
    );
    assert_eq!(native_host["debug"]["panel_contract"]["root"], "debug");
    let debug_sections = native_host["debug"]["panel_contract"]["sections"]
        .as_array()
        .expect("native host debug panel sections");
    assert!(debug_sections
        .iter()
        .any(|section| section["name"] == "configurations"
            && section["path"] == "debug.configurations"));
    assert!(debug_sections.iter().any(|section| {
        section["name"] == "source_inventory" && section["path"] == "debug.source_inventory"
    }));
    assert!(debug_sections.iter().any(|section| {
        section["name"] == "control_commands" && section["path"] == "debug.control_commands"
    }));
    assert!(debug_sections.iter().any(|section| {
        section["name"] == "breakpoint_commands" && section["path"] == "debug.breakpoint_commands"
    }));
    assert!(debug_sections.iter().any(|section| {
        section["name"] == "function_breakpoint_commands"
            && section["path"] == "debug.function_breakpoint_commands"
    }));
    assert!(debug_sections.iter().any(|section| {
        section["name"] == "data_breakpoint_commands"
            && section["path"] == "debug.data_breakpoint_commands"
    }));
    assert!(debug_sections.iter().any(|section| {
        section["name"] == "exception_filter_commands"
            && section["path"] == "debug.exception_filter_commands"
    }));
    assert!(debug_sections.iter().any(|section| {
        section["name"] == "function_breakpoint_argument"
            && section["path"] == "debug.function_breakpoint_argument"
    }));
    assert!(debug_sections.iter().any(|section| {
        section["name"] == "data_breakpoint_argument"
            && section["path"] == "debug.data_breakpoint_argument"
    }));
    assert!(debug_sections.iter().any(|section| {
        section["name"] == "exception_filter_argument"
            && section["path"] == "debug.exception_filter_argument"
    }));
    assert!(debug_sections.iter().any(|section| {
        section["name"] == "watch_expression_argument"
            && section["path"] == "debug.watch_expression_argument"
    }));
    assert!(debug_sections.iter().any(|section| {
        section["name"] == "result_artifact" && section["path"] == "debug.result_artifact"
    }));
    assert!(
        native_host["debug"]["result_artifact"]["panel_contract"]["sections"]
            .as_array()
            .expect("native host result panel sections")
            .iter()
            .any(|section| section["name"] == "events" && section["path"] == "panels.debug.events")
    );
    assert!(
        native_host["debug"]["result_artifact"]["panel_contract"]["sections"]
            .as_array()
            .expect("native host result panel sections")
            .iter()
            .any(|section| section["name"] == "function_breakpoints"
                && section["path"] == "panels.debug.function_breakpoints")
    );
    assert!(
        native_host["debug"]["result_artifact"]["panel_contract"]["sections"]
            .as_array()
            .expect("native host result panel sections")
            .iter()
            .any(|section| section["name"] == "data_breakpoints"
                && section["path"] == "panels.debug.data_breakpoints")
    );
    assert!(
        native_host["debug"]["result_artifact"]["panel_contract"]["sections"]
            .as_array()
            .expect("native host result panel sections")
            .iter()
            .any(|section| section["name"] == "exception_filters"
                && section["path"] == "panels.debug.exception_filters")
    );
    assert!(
        native_host["debug"]["result_artifact"]["panel_contract"]["sections"]
            .as_array()
            .expect("native host result panel sections")
            .iter()
            .any(|section| section["name"] == "watch_expressions"
                && section["path"] == "panels.debug.watch_expressions")
    );
    assert!(
        native_host["debug"]["result_artifact"]["panel_contract"]["sections"]
            .as_array()
            .expect("native host result panel sections")
            .iter()
            .any(|section| section["name"] == "source_snapshots"
                && section["path"] == "panels.debug.source_snapshots")
    );
    assert_eq!(native_host["debug"]["configuration_count"], 3);
    let configurations = native_host["debug"]["configurations"]
        .as_array()
        .expect("native host debug configurations");
    assert!(configurations
        .iter()
        .any(|config| config["name"] == "Live Launch ORV" && config["live"] == true));
    assert!(configurations.iter().any(|config| {
        config["name"] == "Attach ORV Runtime"
            && config["request"] == "attach"
            && config["attachRuntimeMode"] == "inProcess"
    }));
    assert!(native_host["debug"]["breakpoint_count"]
        .as_u64()
        .is_some_and(|count| count > 0));
    assert!(native_host["debug"]["function_breakpoint_count"]
        .as_u64()
        .is_some_and(|count| count > 0));
    assert!(native_host["debug"]["data_breakpoint_count"]
        .as_u64()
        .is_some_and(|count| count > 0));
    assert!(native_host["debug"]["exception_filter_count"]
        .as_u64()
        .is_some_and(|count| count > 0));
    let control_commands = native_host["debug"]["control_commands"]
        .as_array()
        .expect("native host control commands");
    assert!(control_commands.iter().any(|command| {
        command["name"] == "Next"
            && command["command"]
                == serde_json::json!([
                    "orv",
                    "editor",
                    "run-debug",
                    "debug/session-runner.json",
                    "--control",
                    "next"
                ])
    }));
    assert!(control_commands.iter().any(|command| {
        command["name"] == "Step Back"
            && command["request"]
                == serde_json::json!({"command": "stepBack", "arguments": {"threadId": 1}})
    }));
    assert!(control_commands.iter().any(|command| {
        command["name"] == "Reverse Continue"
            && command["request"]
                == serde_json::json!({"command": "reverseContinue", "arguments": {"threadId": 1}})
    }));
    assert!(control_commands.iter().any(|command| {
        command["name"] == "Restart Frame"
            && command["request"]
                == serde_json::json!({"command": "restartFrame", "arguments": {"frameId": 1}})
    }));
    assert!(control_commands.iter().any(|command| {
        command["name"] == "Terminate"
            && command["request"] == serde_json::json!({"command": "terminate", "arguments": {}})
            && command["command"]
                .as_array()
                .is_some_and(|command| command.iter().any(|part| part == "terminate"))
    }));
    assert!(control_commands.iter().any(|command| {
            command["name"] == "Terminate Threads"
                && command["request"]
                    == serde_json::json!({"command": "terminateThreads", "arguments": {"threadIds": [1]}})
                && command["command"].as_array().is_some_and(|command| {
                    command.iter().any(|part| part == "terminate-threads")
                })
        }));
    assert!(control_commands.iter().any(|command| {
        command["name"] == "Step In Targets"
            && command["request"]
                == serde_json::json!({"command": "stepInTargets", "arguments": {"frameId": 1}})
    }));
    let breakpoint_commands = native_host["debug"]["breakpoint_commands"]
        .as_array()
        .expect("native host breakpoint commands");
    assert!(breakpoint_commands.iter().any(|breakpoint| {
        breakpoint["line"] == 1
            && breakpoint["source"]["path"]
                .as_str()
                .is_some_and(|path| path.ends_with("app.orv"))
            && breakpoint["request"]["command"] == "setBreakpoints"
            && breakpoint["command"].as_array().is_some_and(|command| {
                command.iter().any(|part| part == "--breakpoint")
                    && command.iter().any(|part| part == "continue")
            })
    }));
    let function_breakpoint_commands = native_host["debug"]["function_breakpoint_commands"]
        .as_array()
        .expect("native host function breakpoint commands");
    assert!(function_breakpoint_commands.iter().any(|breakpoint| {
        breakpoint["name"] == "helper"
            && breakpoint["request"]["command"] == "setFunctionBreakpoints"
            && breakpoint["command"].as_array().is_some_and(|command| {
                command.iter().any(|part| part == "--function-breakpoint")
                    && command.iter().any(|part| part == "helper")
            })
    }));
    let data_breakpoint_commands = native_host["debug"]["data_breakpoint_commands"]
        .as_array()
        .expect("native host data breakpoint commands");
    assert!(data_breakpoint_commands.iter().any(|breakpoint| {
        breakpoint["name"] == "total"
            && breakpoint["info_request"]["command"] == "dataBreakpointInfo"
            && breakpoint["request"]["command"] == "setDataBreakpoints"
            && breakpoint["command"].as_array().is_some_and(|command| {
                command.iter().any(|part| part == "--data-breakpoint")
                    && command.iter().any(|part| part == "total")
            })
    }));
    let exception_filter_commands = native_host["debug"]["exception_filter_commands"]
        .as_array()
        .expect("native host exception filter commands");
    assert!(exception_filter_commands.iter().any(|filter| {
        filter["filter"] == "orv.runtime"
            && filter["request"]["command"] == "setExceptionBreakpoints"
            && filter["command"].as_array().is_some_and(|command| {
                command.iter().any(|part| part == "--exception-filter")
                    && command.iter().any(|part| part == "orv.runtime")
            })
    }));
}

fn assert_editor_debug_configurations(state: &serde_json::Value) {
    assert!(state["debug"]["configurations"]
        .as_array()
        .expect("debug configurations")
        .iter()
        .any(|config| config["name"] == "Live Launch ORV" && config["live"] == true));
}

fn assert_editor_debug_breakpoint_sources(state: &serde_json::Value) {
    let breakpoint_sources = state["debug"]["breakpoint_sources"]
        .as_array()
        .expect("breakpoint sources");
    assert!(breakpoint_sources.iter().any(|source| {
        source["source"]["path"]
            .as_str()
            .is_some_and(|path| path.ends_with("app.orv"))
            && source["lines"]
                .as_array()
                .is_some_and(|lines| lines.iter().any(|line| line == 1))
    }));
    assert!(breakpoint_sources.iter().any(|source| {
        source["source"]["path"]
            .as_str()
            .is_some_and(|path| path.ends_with("app.orv"))
            && source["breakpoints"].as_array().is_some_and(|breakpoints| {
                breakpoints.iter().any(|breakpoint| {
                    breakpoint["line"] == 1
                        && breakpoint["request"]["command"] == "setBreakpoints"
                        && breakpoint["runner_command"]
                            .as_array()
                            .is_some_and(|command| {
                                command.iter().any(|part| part == "--breakpoint")
                                    && command.iter().any(|part| part == "continue")
                            })
                })
            })
    }));
}

fn assert_editor_debug_controls(state: &serde_json::Value) {
    let controls = state["debug"]["controls"]
        .as_array()
        .expect("debug controls");
    assert_editor_debug_control(
        controls,
        "Continue",
        &serde_json::json!({"command": "continue", "arguments": {"threadId": 1}}),
    );
    assert_editor_debug_control(
        controls,
        "Pause",
        &serde_json::json!({"command": "pause", "arguments": {"threadId": 1}}),
    );
    assert_editor_debug_control(
        controls,
        "Reverse Continue",
        &serde_json::json!({"command": "reverseContinue", "arguments": {"threadId": 1}}),
    );
    assert_editor_debug_control(
        controls,
        "Next",
        &serde_json::json!({"command": "next", "arguments": {"threadId": 1}}),
    );
    assert_editor_debug_control_runner_command(controls, "Next", "next");
    assert_editor_debug_control(
        controls,
        "Step Back",
        &serde_json::json!({"command": "stepBack", "arguments": {"threadId": 1}}),
    );
    assert_editor_debug_control(
        controls,
        "Step In",
        &serde_json::json!({"command": "stepIn", "arguments": {"threadId": 1}}),
    );
    assert_editor_debug_control(
        controls,
        "Step In Targets",
        &serde_json::json!({"command": "stepInTargets", "arguments": {"frameId": 1}}),
    );
    assert_editor_debug_control(
        controls,
        "Step Out",
        &serde_json::json!({"command": "stepOut", "arguments": {"threadId": 1}}),
    );
    assert_editor_debug_control(
        controls,
        "Restart Frame",
        &serde_json::json!({"command": "restartFrame", "arguments": {"frameId": 1}}),
    );
    assert_editor_debug_control(
        controls,
        "Restart",
        &serde_json::json!({"command": "restart", "arguments": {}}),
    );
    assert_editor_debug_control(
        controls,
        "Terminate",
        &serde_json::json!({"command": "terminate", "arguments": {}}),
    );
    assert_editor_debug_control_runner_command(controls, "Terminate", "terminate");
    assert_editor_debug_control(
        controls,
        "Terminate Threads",
        &serde_json::json!({"command": "terminateThreads", "arguments": {"threadIds": [1]}}),
    );
    assert_editor_debug_control_runner_command(controls, "Terminate Threads", "terminate-threads");
    assert_editor_debug_control(
        controls,
        "Disconnect",
        &serde_json::json!({"command": "disconnect", "arguments": {"terminateDebuggee": true}}),
    );
}

fn assert_editor_debug_html(html: &str) {
    assert!(html.contains("id=\"debug-config-list\""));
    assert!(html.contains("id=\"debug-control-list\""));
    assert!(html.contains("Debug Controls"));
    assert!(html.contains("DAP Capabilities"));
    assert!(html.contains("id=\"debug-breakpoint-list\""));
    assert!(html.contains("id=\"debug-function-breakpoint-list\""));
    assert!(html.contains("id=\"debug-data-breakpoint-list\""));
    assert!(html.contains("id=\"debug-exception-filter-list\""));
    assert!(html.contains("id=\"debug-capability-list\""));
    assert!(html.contains("id=\"debug-runner-detail\""));
    assert!(html.contains("id=\"debug-result-detail\""));
    assert!(html.contains("Runner Command"));
    assert!(html.contains("renderDebugRunner"));
    assert!(html.contains("renderDebugCapabilities"));
    assert!(html.contains("renderDebugResultArtifact"));
    assert!(html.contains("renderDebugDetail"));
    assert!(html.contains("renderDebugControlCommand"));
    assert!(html.contains("renderFunctionBreakpoints"));
    assert!(html.contains("renderDataBreakpoints"));
    assert!(html.contains("renderExceptionFilters"));
}

fn assert_editor_debug_control(
    controls: &[serde_json::Value],
    name: &str,
    request: &serde_json::Value,
) {
    assert!(
        controls
            .iter()
            .any(|control| control["name"] == name && control["request"] == *request),
        "missing debug control {name}"
    );
}

fn assert_editor_debug_control_runner_command(
    controls: &[serde_json::Value],
    name: &str,
    value: &str,
) {
    assert!(
        controls.iter().any(|control| {
            control["name"] == name
                && control["runner_command"]
                    == serde_json::json!([
                        "orv",
                        "editor",
                        "run-debug",
                        "debug/session-runner.json",
                        "--control",
                        value
                    ])
        }),
        "missing debug control runner command {name}"
    );
}

fn write_reference_artifact(path: &Path, entry: &str, source: &str) {
    write_reference_artifact_with_sources(path, entry, [(entry, source)]);
}

fn write_reference_artifact_with_sources<'a>(
    path: &Path,
    entry: &str,
    sources: impl IntoIterator<Item = (&'a str, &'a str)>,
) {
    let manifest = orv_compiler::BuildManifest {
        schema_version: orv_compiler::BUILD_MANIFEST_VERSION,
        entry: entry.to_string(),
        runtime: "reference-interpreter".to_string(),
        artifacts: Vec::new(),
        capabilities: orv_compiler::BuildCapabilities {
            has_server: false,
            server_routes: 0,
            client_wasm: false,
            runtime_features: vec!["console_io".to_string()],
        },
    };
    let origin_map = orv_compiler::OriginMap {
        version: orv_compiler::ORIGIN_MAP_VERSION,
        entries: Vec::new(),
        edges: Vec::new(),
    };
    let artifact = orv_compiler::server_runtime_artifact(&manifest, &origin_map, sources);
    write_json(
        path,
        &serde_json::to_value(artifact).expect("artifact value"),
    )
    .expect("write artifact");
}

mod benchmark;

mod build;

mod client;

mod dap_async_runtime;

mod dap_breakpoints;

mod dap_control;

mod dap_launch;

mod dap_runtime;

mod dap_transport;

mod dap_variables;

mod database;

mod deploy;

mod dev;

mod editor_debug;

mod editor_desktop;

mod editor_export;

mod editor_host;

mod editor_production;

mod editor_snapshot;

mod editor_trace;

mod graph;

mod language;

mod lsp_completion;

mod lsp_diagnostics;

mod lsp_document;

mod lsp_domains;

mod lsp_formatting;

mod lsp_navigation;

mod lsp_symbols;

mod lsp_transport;

mod native;

mod project;

mod reveal;

mod runtime;

mod shop;

mod smoke;

mod verify_benchmark;

mod verify_build;

mod verify_client;

mod verify_deploy;

mod verify_graph;

mod verify_native;

mod verify_preflight;

mod verify_server;

mod verify_smoke;
