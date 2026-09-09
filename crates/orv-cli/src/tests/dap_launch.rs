use super::*;

#[test]
fn dap_live_launch_defers_output_until_next_step() {
    let dir = temp_output_dir("dap-live-launch");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let source = dir.join("app.orv");
    std::fs::write(&source, "let first: int = 1\n@out \"second\"\n").expect("write source");
    let mut session = DapSession::default();

    let launch = session
        .message_response(&serde_json::json!({
            "seq": 208,
            "type": "request",
            "command": "launch",
            "arguments": {
                "program": format!("file://{}", source.display()),
                "live": true,
            },
        }))
        .expect("launch response");
    let launch_events = session.drain_pending_events();
    let first_stack = session
        .message_response(&serde_json::json!({
            "seq": 209,
            "type": "request",
            "command": "stackTrace",
            "arguments": {
                "threadId": 1,
            },
        }))
        .expect("first stack response");
    let next = session
        .message_response(&serde_json::json!({
            "seq": 210,
            "type": "request",
            "command": "next",
            "arguments": {
                "threadId": 1,
            },
        }))
        .expect("next response");
    let next_events = session.drain_pending_events();

    assert_eq!(launch["success"], true, "{launch}");
    assert_eq!(launch["body"]["runtime"]["status"], "running");
    assert_eq!(launch["body"]["runtime"]["stdout"], "");
    assert!(launch_events
        .iter()
        .all(|event| { event["event"] != "output" || event["body"]["output"] != "second\n" }));
    assert_eq!(first_stack["body"]["stackFrames"][0]["line"], 1);
    assert_eq!(next["success"], true, "{next}");
    assert!(next_events.iter().any(|event| {
        event["type"] == "event"
            && event["event"] == "output"
            && event["body"]["category"] == "stdout"
            && event["body"]["output"] == "second\n"
    }));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn dap_launch_server_program_reports_paused_long_running_runtime() {
    let dir = temp_output_dir("dap-server-long-running-launch");
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

    let launch = session
        .message_response(&serde_json::json!({
            "seq": 221,
            "type": "request",
            "command": "launch",
            "arguments": {
                "program": format!("file://{}", source.display()),
            },
        }))
        .expect("launch response");
    let stack = session
        .message_response(&serde_json::json!({
            "seq": 222,
            "type": "request",
            "command": "stackTrace",
            "arguments": {
                "threadId": 1,
            },
        }))
        .expect("stack response");

    assert_eq!(launch["success"], true, "{launch}");
    assert_eq!(launch["body"]["runtime"]["status"], "paused");
    assert_eq!(launch["body"]["runtime"]["async"]["route_count"], 1);
    assert_eq!(
        launch["body"]["runtime"]["async"]["routes"][0]["method"],
        "GET"
    );
    assert_eq!(
        launch["body"]["runtime"]["async"]["routes"][0]["path"],
        "/ping"
    );
    assert_eq!(stack["success"], true, "{stack}");
    assert_eq!(stack["body"]["stackFrames"][0]["line"], 1);
    assert_eq!(stack["body"]["stackFrames"][0]["name"], "server runtime");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn dap_terminate_threads_clears_launch_and_queues_terminated_event() {
    let dir = temp_output_dir("dap-terminate-threads");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let source = dir.join("app.orv");
    std::fs::write(&source, "let answer: int = 42\n").expect("write source");
    let mut session = DapSession::default();

    session
        .message_response(&serde_json::json!({
            "seq": 183,
            "type": "request",
            "command": "launch",
            "arguments": {
                "program": format!("file://{}", source.display()),
            },
        }))
        .expect("launch response");
    let terminate_threads = session
        .message_response(&serde_json::json!({
            "seq": 184,
            "type": "request",
            "command": "terminateThreads",
            "arguments": {
                "threadIds": [1],
            },
        }))
        .expect("terminateThreads response");
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

    assert_eq!(terminate_threads["success"], true, "{terminate_threads}");
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
fn dap_restart_preserves_live_launch_mode() {
    let dir = temp_output_dir("dap-restart-live");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let source = dir.join("app.orv");
    std::fs::write(&source, "let first: int = 1\n@out \"after\"\n").expect("write source");
    let mut session = DapSession::default();

    session
        .message_response(&serde_json::json!({
            "seq": 215,
            "type": "request",
            "command": "launch",
            "arguments": {
                "program": format!("file://{}", source.display()),
                "live": true,
            },
        }))
        .expect("launch response");
    let _ = session.drain_pending_events();
    let restart = session
        .message_response(&serde_json::json!({
            "seq": 216,
            "type": "request",
            "command": "restart",
            "arguments": {},
        }))
        .expect("restart response");
    let restart_events = session.drain_pending_events();
    let restarted_stack = session
        .message_response(&serde_json::json!({
            "seq": 217,
            "type": "request",
            "command": "stackTrace",
            "arguments": {
                "threadId": 1,
            },
        }))
        .expect("restarted stack response");

    assert_eq!(restart["success"], true, "{restart}");
    assert_eq!(restart["body"]["runtime"]["status"], "running");
    assert_eq!(restart["body"]["runtime"]["stdout"], "");
    assert_eq!(restarted_stack["body"]["stackFrames"][0]["line"], 1);
    assert!(restart_events
        .iter()
        .all(|event| { event["event"] != "output" || event["body"]["output"] != "after\n" }));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn dap_launch_threads_and_stacktrace_use_entry_source() {
    let dir = temp_output_dir("dap-launch");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let source = dir.join("app.orv");
    std::fs::write(&source, "let answer: int = 42\n").expect("write source");
    let canonical_source = std::fs::canonicalize(&source).expect("canonical source");
    let mut session = DapSession::default();

    let launch = session
        .message_response(&serde_json::json!({
            "seq": 2,
            "type": "request",
            "command": "launch",
            "arguments": {
                "program": format!("file://{}", source.display()),
            },
        }))
        .expect("launch response");
    let threads = session
        .message_response(&serde_json::json!({
            "seq": 3,
            "type": "request",
            "command": "threads",
        }))
        .expect("threads response");
    let stack = session
        .message_response(&serde_json::json!({
            "seq": 4,
            "type": "request",
            "command": "stackTrace",
            "arguments": {
                "threadId": 1,
            },
        }))
        .expect("stack response");

    assert_eq!(launch["success"], true, "{launch}");
    assert_eq!(launch["body"]["projectGraphNodes"], 1);
    assert_eq!(threads["body"]["threads"][0]["id"], 1);
    assert_eq!(stack["success"], true, "{stack}");
    assert_eq!(stack["body"]["totalFrames"], 1);
    let frame = &stack["body"]["stackFrames"][0];
    assert_eq!(frame["id"], 1);
    assert_eq!(frame["line"], 1);
    assert_eq!(frame["column"], 1);
    assert_eq!(
        frame["source"]["path"],
        canonical_source.display().to_string()
    );
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn dap_loaded_sources_returns_project_files_after_launch() {
    let dir = temp_output_dir("dap-loaded-sources");
    let models = dir.join("models");
    std::fs::create_dir_all(&models).expect("create models dir");
    let source = dir.join("app.orv");
    let imported = models.join("user.orv");
    let source_text = "import models.user.User\nlet u: User = { id: 1 }\n";
    let imported_source = "pub struct User { id: int }\n";
    std::fs::write(&source, source_text).expect("write source");
    std::fs::write(&imported, imported_source).expect("write imported");
    let mut session = DapSession::default();

    let launch = session
        .message_response(&serde_json::json!({
            "seq": 30,
            "type": "request",
            "command": "launch",
            "arguments": {
                "program": format!("file://{}", source.display()),
            },
        }))
        .expect("launch response");
    let loaded = session
        .message_response(&serde_json::json!({
            "seq": 31,
            "type": "request",
            "command": "loadedSources",
            "arguments": {},
        }))
        .expect("loadedSources response");

    assert_eq!(launch["success"], true, "{launch}");
    assert_eq!(loaded["success"], true, "{loaded}");
    let sources = loaded["body"]["sources"].as_array().expect("sources");
    assert!(sources
        .iter()
        .any(|item| item["name"] == "app.orv" && item["path"].as_str().is_some()));
    let imported_item = sources
        .iter()
        .find(|item| item["name"] == "user.orv" && item["path"].as_str().is_some())
        .expect("imported source");
    assert_eq!(
        imported_item["checksums"][0]["algorithm"],
        serde_json::json!("SHA256")
    );
    assert_eq!(
        imported_item["checksums"][0]["checksum"],
        serde_json::json!(sha256_hex(imported_source.as_bytes()))
    );
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn dap_modules_returns_project_sources_after_launch() {
    let dir = temp_output_dir("dap-modules");
    let models = dir.join("models");
    std::fs::create_dir_all(&models).expect("create models dir");
    let source = dir.join("app.orv");
    let imported = models.join("user.orv");
    std::fs::write(
        &source,
        "import models.user.User\nlet u: User = { id: 1 }\n",
    )
    .expect("write source");
    std::fs::write(&imported, "pub struct User { id: int }\n").expect("write imported");
    let mut session = DapSession::default();

    session
        .message_response(&serde_json::json!({
            "seq": 175,
            "type": "request",
            "command": "launch",
            "arguments": {
                "program": format!("file://{}", source.display()),
            },
        }))
        .expect("launch response");
    let modules = session
        .message_response(&serde_json::json!({
            "seq": 176,
            "type": "request",
            "command": "modules",
            "arguments": {
                "startModule": 0,
                "moduleCount": 1,
            },
        }))
        .expect("modules response");

    assert_eq!(modules["success"], true, "{modules}");
    assert_eq!(modules["body"]["totalModules"], 2);
    let items = modules["body"]["modules"].as_array().expect("modules");
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["name"], "app.orv");
    assert_eq!(items[0]["id"], 1);
    assert_eq!(items[0]["isUserCode"], true);
    assert!(items[0]["path"].as_str().is_some());
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn dap_source_returns_loaded_file_content_after_launch() {
    let dir = temp_output_dir("dap-source");
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
    let canonical_imported = std::fs::canonicalize(&imported).expect("canonical imported");
    let mut session = DapSession::default();

    let launch = session
        .message_response(&serde_json::json!({
            "seq": 32,
            "type": "request",
            "command": "launch",
            "arguments": {
                "program": format!("file://{}", source.display()),
            },
        }))
        .expect("launch response");
    let source_response = session
        .message_response(&serde_json::json!({
            "seq": 33,
            "type": "request",
            "command": "source",
            "arguments": {
                "source": {
                    "path": canonical_imported.display().to_string(),
                },
            },
        }))
        .expect("source response");

    assert_eq!(launch["success"], true, "{launch}");
    assert_eq!(source_response["success"], true, "{source_response}");
    assert_eq!(source_response["body"]["content"], imported_source);
    assert_eq!(source_response["body"]["mimeType"], "text/x-orv");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn dap_launch_source_bundle_rehydrates_source_when_original_file_is_missing() {
    let dir = temp_output_dir("dap-source-bundle-launch");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let source = dir.join("app.orv");
    let source_text = "let answer: int = 42\n@out answer\n";
    std::fs::write(&source, source_text).expect("write source");
    let build_out = dir.join("dist");
    cmd_build_with_profile(&source, &build_out, BuildProfile::Production).expect("prod build");
    std::fs::remove_file(&source).expect("remove original source");
    let source_bundle_path = build_out.join(SOURCE_BUNDLE_PATH);
    assert_eq!(
        dap_launch_source_bundle_path(&serde_json::json!({
            "arguments": {
                "sourceBundle": source_bundle_path.display().to_string(),
            },
        }))
        .expect("camel sourceBundle path"),
        Some(source_bundle_path.clone())
    );
    assert_eq!(
        dap_launch_source_bundle_path(&serde_json::json!({
            "arguments": {
                "source_bundle": source_bundle_path.display().to_string(),
            },
        }))
        .expect("snake source_bundle path"),
        Some(source_bundle_path.clone())
    );
    let mut session = DapSession::default();
    let source_bundle_value =
        read_json_value(&source_bundle_path).expect("source bundle json value");
    let expected_source_bundle_hash =
        stable_json_hash(&source_bundle_value).expect("source bundle hash");

    let launch = session
        .message_response(&serde_json::json!({
            "seq": 37,
            "type": "request",
            "command": "launch",
            "arguments": {
                "program": format!("file://{}", source.display()),
                "sourceBundle": source_bundle_path.display().to_string(),
            },
        }))
        .expect("launch response");
    let loaded = session
        .message_response(&serde_json::json!({
            "seq": 38,
            "type": "request",
            "command": "loadedSources",
            "arguments": {},
        }))
        .expect("loadedSources response");
    let source_reference = loaded["body"]["sources"]
        .as_array()
        .expect("loaded sources")
        .iter()
        .find(|item| item["name"] == "app.orv")
        .and_then(|item| item["sourceReference"].as_u64())
        .expect("source reference");
    let source_response = session
        .message_response(&serde_json::json!({
            "seq": 39,
            "type": "request",
            "command": "source",
            "arguments": {
                "sourceReference": source_reference,
            },
        }))
        .expect("source response");
    let restart = session
        .message_response(&serde_json::json!({
            "seq": 40,
            "type": "request",
            "command": "restart",
            "arguments": {},
        }))
        .expect("restart response");

    assert_eq!(launch["success"], true, "{launch}");
    assert!(
        launch["body"]["projectGraphNodes"]
            .as_u64()
            .is_some_and(|count| count > 0),
        "{launch}"
    );
    assert_eq!(
        launch["body"]["sourceBundle"]["path"],
        source_bundle_path.display().to_string()
    );
    assert_eq!(
        launch["body"]["sourceBundle"]["entry"],
        source_bundle_value["entry"]
    );
    assert_eq!(launch["body"]["sourceBundle"]["fileCount"], 1);
    assert_eq!(
        launch["body"]["sourceBundle"]["hash"],
        expected_source_bundle_hash
    );
    assert_eq!(source_response["success"], true, "{source_response}");
    assert_eq!(source_response["body"]["content"], source_text);
    assert_eq!(restart["success"], true, "{restart}");
    assert_eq!(
        restart["body"]["sourceBundle"]["path"],
        source_bundle_path.display().to_string()
    );
    assert_eq!(
        restart["body"]["sourceBundle"]["hash"],
        expected_source_bundle_hash
    );
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn verify_build_rejects_deploy_smoke_dap_source_bundle_panel_missing() {
    let (src_dir, path) = prod_server_source("deploy-smoke-dap-source-bundle-source");
    let out = temp_output_dir("deploy-smoke-dap-source-bundle-missing");

    cmd_build_with_profile(&path, &out, BuildProfile::Production).expect("prod build");
    let smoke_path = out.join("deploy").join("smoke-test.sh");
    let smoke = std::fs::read_to_string(&smoke_path).expect("smoke test");
    write_text(
            &smoke_path,
            &smoke.replace(
                r#"orv_smoke_dap_summary_contains "dap source bundle panel path" '"path": "./source-bundle.json"'
"#,
                "",
            ),
        )
        .expect("write corrupt smoke test");

    let err = cmd_verify_build(&out).expect_err("smoke DAP source bundle panel mismatch");

    assert!(err
        .to_string()
        .contains("deploy smoke test must verify the build graph contract"));
    let _ = std::fs::remove_dir_all(src_dir);
    let _ = std::fs::remove_dir_all(out);
}

#[test]
fn build_prod_smoke_dap_source_bundle_count_uses_actual_file_count() {
    let (src_dir, path) = imported_prod_server_source("deploy-smoke-dap-source-count-source");
    let out = temp_output_dir("deploy-smoke-dap-source-count");

    cmd_build_with_profile(&path, &out, BuildProfile::Production).expect("prod build");
    let smoke =
        std::fs::read_to_string(out.join("deploy").join("smoke-test.sh")).expect("smoke test");
    let graph_contract_count = deploy_graph_contract_count(&out).expect("graph contract count");
    let project_graph = read_json_value(&out.join("project-graph.json")).expect("project graph");
    let project_graph_node_count = json_array_count(project_graph.get("nodes"));
    let origin_map = read_json_value(&out.join("origin-map.json")).expect("origin map");
    let origin_entry_count = json_array_count(origin_map.get("entries"));

    assert!(smoke.contains(&format!(
        r#"orv_smoke_dap_summary_contains "dap graph summary" '"graph_contract_count": {graph_contract_count}'"#
    )));
    assert!(smoke.contains(
        r#"orv_smoke_dap_summary_contains "dap source bundle summary" '"source_bundle_file_count": 2'"#
    ));
    assert!(smoke.contains(&format!(
        r#"orv_smoke_dap_summary_contains "dap project graph summary" '"project_graph_node_count": {project_graph_node_count}'"#
    )));
    assert!(smoke.contains(&format!(
        r#"orv_smoke_dap_summary_contains "dap origin summary" '"origin_entry_count": {origin_entry_count}'"#
    )));
    assert!(smoke.contains(
        r#"orv_smoke_dap_summary_contains "dap source bundle panel file count" '"fileCount": 2'"#
    ));
    assert!(!smoke.contains(
        r#"orv_smoke_dap_summary_contains "dap source bundle summary" '"source_bundle_file_count": 1'"#
    ));
    assert!(!smoke.contains(
        r#"orv_smoke_dap_summary_contains "dap source bundle panel file count" '"fileCount": 1'"#
    ));
    cmd_verify_build(&out).expect("verify prod build");
    let _ = std::fs::remove_dir_all(src_dir);
    let _ = std::fs::remove_dir_all(out);
}

#[test]
fn verify_build_rejects_deploy_smoke_dap_source_bundle_count_mismatch() {
    let (src_dir, path) =
        imported_prod_server_source("deploy-smoke-dap-source-count-mismatch-source");
    let out = temp_output_dir("deploy-smoke-dap-source-count-mismatch");

    cmd_build_with_profile(&path, &out, BuildProfile::Production).expect("prod build");
    let smoke_path = out.join("deploy").join("smoke-test.sh");
    let smoke = std::fs::read_to_string(&smoke_path).expect("smoke test");
    let graph_contract_count = deploy_graph_contract_count(&out).expect("graph contract count");
    let project_graph = read_json_value(&out.join("project-graph.json")).expect("project graph");
    let project_graph_node_count = json_array_count(project_graph.get("nodes"));
    let origin_map = read_json_value(&out.join("origin-map.json")).expect("origin map");
    let origin_entry_count = json_array_count(origin_map.get("entries"));
    write_text(
        &smoke_path,
        &smoke
            .replace(
                &format!(
                    r#"orv_smoke_dap_summary_contains "dap graph summary" '"graph_contract_count": {graph_contract_count}'"#
                ),
                r#"orv_smoke_dap_summary_contains "dap graph summary" '"graph_contract_count": 1'"#,
            )
            .replace(
                r#"orv_smoke_dap_summary_contains "dap source bundle summary" '"source_bundle_file_count": 2'"#,
                r#"orv_smoke_dap_summary_contains "dap source bundle summary" '"source_bundle_file_count": 1'"#,
            )
            .replace(
                r#"orv_smoke_dap_summary_contains "dap source bundle panel file count" '"fileCount": 2'"#,
                r#"orv_smoke_dap_summary_contains "dap source bundle panel file count" '"fileCount": 1'"#,
            )
            .replace(
                &format!(
                    r#"orv_smoke_dap_summary_contains "dap project graph summary" '"project_graph_node_count": {project_graph_node_count}'"#
                ),
                r#"orv_smoke_dap_summary_contains "dap project graph summary" '"project_graph_node_count": 1'"#,
            )
            .replace(
                &format!(
                    r#"orv_smoke_dap_summary_contains "dap origin summary" '"origin_entry_count": {origin_entry_count}'"#
                ),
                r#"orv_smoke_dap_summary_contains "dap origin summary" '"origin_entry_count": 1'"#,
            ),
    )
    .expect("write corrupt smoke test");

    let err = cmd_verify_build(&out).expect_err("smoke DAP source bundle count mismatch");

    assert!(
        err.to_string()
            .contains("deploy smoke test must verify the build graph contract"),
        "{err:?}"
    );
    let _ = std::fs::remove_dir_all(src_dir);
    let _ = std::fs::remove_dir_all(out);
}

#[test]
fn verify_build_rejects_deploy_smoke_output_dap_source_bundle_marker_missing() {
    let (src_dir, path) = prod_server_source("deploy-smoke-output-dap-bundle-source");
    let out = temp_output_dir("deploy-smoke-output-dap-bundle-missing");

    cmd_build_with_profile(&path, &out, BuildProfile::Production).expect("prod build");
    let smoke_path = out.join("deploy").join("smoke-test.sh");
    let smoke = std::fs::read_to_string(&smoke_path).expect("smoke test");
    write_text(
        &smoke_path,
        &smoke.replace("dap_source_bundle=verified", "dap_source_bundle=missing"),
    )
    .expect("write corrupt smoke test");

    let err = cmd_verify_build(&out).expect_err("smoke output DAP source bundle marker mismatch");

    assert!(
        err.to_string()
            .contains("deploy smoke test must write deploy smoke output artifact"),
        "{err:?}"
    );
    let _ = std::fs::remove_dir_all(src_dir);
    let _ = std::fs::remove_dir_all(out);
}
