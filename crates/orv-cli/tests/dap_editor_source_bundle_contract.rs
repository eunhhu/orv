#[path = "dap_editor_source_bundle_contract/support.rs"]
mod support;

use support::{
    assert_loaded_source, assert_loaded_source_inventory, assert_source_bundle_files,
    assert_source_responses, build_fixture, expected_sha256, read_json, response,
    run_dap_stdio_frames, run_orv_failure, run_orv_json, write_json, APP_SOURCE, IMPORTED_SOURCE,
};

#[test]
fn dap_stdio_source_bundle_rehydrates_entry_and_imports_when_originals_are_missing() {
    let root = support::temp_dir("dap-source-bundle-imports");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("temp root");
    let (app, imported, out) = build_fixture(&root);
    let bundle_path = out.join("source-bundle.json");
    assert_source_bundle_files(&read_json(&bundle_path));
    std::fs::remove_file(&app).expect("remove app source");
    std::fs::remove_file(&imported).expect("remove imported source");

    let frames = run_dap_stdio_frames(&[
        serde_json::json!({"seq": 1, "type": "request", "command": "initialize", "arguments": {}}),
        serde_json::json!({
            "seq": 2,
            "type": "request",
            "command": "launch",
            "arguments": {
                "program": app.display().to_string(),
                "sourceBundle": bundle_path.display().to_string(),
            },
        }),
        serde_json::json!({"seq": 3, "type": "request", "command": "loadedSources", "arguments": {}}),
        serde_json::json!({"seq": 4, "type": "request", "command": "source", "arguments": {"sourceReference": 1}}),
        serde_json::json!({"seq": 5, "type": "request", "command": "source", "arguments": {"sourceReference": 2}}),
    ]);

    let launch = response(&frames, "launch", 2);
    assert_eq!(launch["success"], true, "{launch}");
    assert_eq!(launch["body"]["sourceBundle"]["fileCount"], 2);
    assert_eq!(
        launch["body"]["sourceBundle"]["path"],
        bundle_path.display().to_string()
    );
    assert!(launch["body"]["sourceBundle"]["hash"]
        .as_str()
        .is_some_and(|hash| hash.len() == 16));

    let loaded = response(&frames, "loadedSources", 3);
    assert_loaded_source(loaded, "app.orv", APP_SOURCE);
    assert_loaded_source(loaded, "user.orv", IMPORTED_SOURCE);
    assert_source_responses([
        response(&frames, "source", 4),
        response(&frames, "source", 5),
    ]);

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn editor_run_debug_preserves_imported_source_bundle_summary_after_sources_are_missing() {
    let root = support::temp_dir("editor-run-debug-source-bundle-imports");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("temp root");
    let (app, imported, out) = build_fixture(&root);
    let bundle_path = out.join("source-bundle.json");
    assert_source_bundle_files(&read_json(&bundle_path));
    std::fs::remove_file(&app).expect("remove app source");
    std::fs::remove_file(&imported).expect("remove imported source");

    let out_arg = out.display().to_string();
    let run = run_orv_json(&[
        "editor",
        "run-debug",
        &out_arg,
        "--control",
        "next",
        "--watch-expression",
        "total",
    ]);

    let launch_bundle = &run["debug"]["launch"]["body"]["sourceBundle"];
    assert_eq!(
        run["runner"]["source_bundle"],
        bundle_path.display().to_string()
    );
    assert_eq!(launch_bundle["fileCount"], 2);
    assert_eq!(launch_bundle["path"], bundle_path.display().to_string());
    assert_eq!(run["panels"]["debug"]["source_bundle"], *launch_bundle);
    assert_eq!(
        run["panels"]["debug"]["session_summary"]["source_bundle"],
        *launch_bundle
    );
    assert_eq!(
        run["panels"]["debug"]["session_summary"]["source_bundle_file_count"],
        2
    );
    assert_eq!(
        run["production_context"]["summary"]["source_bundle_file_count"],
        2
    );
    assert_eq!(
        run["panels"]["debug"]["production_summary"]["source_bundle_file_count"],
        2
    );
    assert!(
        run["production_context"]["summary"]["project_graph_node_count"]
            .as_u64()
            .is_some_and(|count| count > 0)
    );
    assert!(run["production_context"]["summary"]["origin_entry_count"]
        .as_u64()
        .is_some_and(|count| count > 0));

    let loaded_sources = run["debug"]["loaded_sources"]["sources"]
        .as_array()
        .expect("loaded sources");
    assert!(loaded_sources
        .iter()
        .any(|source| source["name"] == "app.orv"));
    assert!(loaded_sources
        .iter()
        .any(|source| source["name"] == "user.orv"));
    assert_loaded_source_inventory(loaded_sources, "app.orv", APP_SOURCE);
    assert_loaded_source_inventory(loaded_sources, "user.orv", IMPORTED_SOURCE);
    assert_eq!(
        run["panels"]["debug"]["loaded_source_count"],
        loaded_sources.len()
    );
    assert_eq!(
        run["panels"]["debug"]["source_snapshot_count"],
        run["debug"]["source_snapshots"]
            .as_array()
            .expect("source snapshots")
            .len()
    );
    assert!(run["debug"]["source_snapshots"]
        .as_array()
        .expect("source snapshots")
        .iter()
        .any(|snapshot| snapshot["response"]["body"]["content"] == APP_SOURCE));
    let imported_snapshot = run["debug"]["source_snapshots"]
        .as_array()
        .expect("source snapshots")
        .iter()
        .find(|snapshot| snapshot["source"]["name"] == "user.orv")
        .expect("imported source snapshot");
    assert_eq!(
        imported_snapshot["response"]["body"]["content"],
        IMPORTED_SOURCE
    );
    assert_eq!(imported_snapshot["checksum"]["algorithm"], "SHA256");
    assert_eq!(
        imported_snapshot["checksum"]["value"],
        expected_sha256(IMPORTED_SOURCE)
    );

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn exported_runner_rehydrates_imported_sources_after_originals_are_missing() {
    let root = support::temp_dir("editor-exported-runner-source-bundle-imports");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("temp root");
    let (app, imported, build_out) = build_fixture(&root);
    let bundle_path = build_out.join("source-bundle.json");
    assert_source_bundle_files(&read_json(&bundle_path));

    let editor_out = root.join("editor");
    let app_arg = app.display().to_string();
    let editor_out_arg = editor_out.display().to_string();
    let build_out_arg = build_out.display().to_string();
    let _export = run_orv_json(&[
        "editor",
        "export",
        &app_arg,
        "--out",
        &editor_out_arg,
        "--build",
        &build_out_arg,
    ]);
    let runner = editor_out.join("debug").join("session-runner.json");
    let state = editor_out.join("state.json");

    std::fs::remove_file(&app).expect("remove app source");
    std::fs::remove_file(&imported).expect("remove imported source");

    let runner_arg = runner.display().to_string();
    let state_arg = state.display().to_string();
    let runner_run = run_orv_json(&[
        "editor",
        "run-debug",
        &runner_arg,
        "--control",
        "next",
        "--watch-expression",
        "total",
    ]);
    let state_run = run_orv_json(&[
        "editor",
        "run-debug",
        &state_arg,
        "--control",
        "next",
        "--watch-expression",
        "total",
    ]);

    for run in [&runner_run, &state_run] {
        let launch_bundle = &run["debug"]["launch"]["body"]["sourceBundle"];
        assert_eq!(
            run["runner"]["source_bundle"],
            bundle_path.display().to_string()
        );
        assert_eq!(launch_bundle["fileCount"], 2);
        assert_eq!(launch_bundle["path"], bundle_path.display().to_string());
        assert!(launch_bundle["hash"]
            .as_str()
            .is_some_and(|hash| !hash.is_empty()));

        let loaded_sources = run["debug"]["loaded_sources"]["sources"]
            .as_array()
            .expect("loaded sources");
        assert!(loaded_sources
            .iter()
            .any(|source| source["name"] == "app.orv"));
        assert!(loaded_sources
            .iter()
            .any(|source| source["name"] == "user.orv"));
        assert_loaded_source_inventory(loaded_sources, "app.orv", APP_SOURCE);
        assert_loaded_source_inventory(loaded_sources, "user.orv", IMPORTED_SOURCE);

        let source_snapshots = run["debug"]["source_snapshots"]
            .as_array()
            .expect("source snapshots");
        let app_snapshot = source_snapshots
            .iter()
            .find(|snapshot| snapshot["source"]["name"] == "app.orv")
            .expect("app source snapshot");
        assert_eq!(app_snapshot["response"]["body"]["content"], APP_SOURCE);
        assert_eq!(app_snapshot["checksum"]["algorithm"], "SHA256");
        assert_eq!(
            app_snapshot["checksum"]["value"],
            expected_sha256(APP_SOURCE)
        );
        let imported_snapshot = source_snapshots
            .iter()
            .find(|snapshot| snapshot["source"]["name"] == "user.orv")
            .expect("imported source snapshot");
        assert_eq!(
            imported_snapshot["response"]["body"]["content"],
            IMPORTED_SOURCE
        );
        assert_eq!(imported_snapshot["checksum"]["algorithm"], "SHA256");
        assert_eq!(
            imported_snapshot["checksum"]["value"],
            expected_sha256(IMPORTED_SOURCE)
        );
    }

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn production_summary_parity_matches_dap_editor_and_reveal_for_same_fixture() {
    let root = support::temp_dir("source-bundle-summary-parity");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("temp root");
    let (app, imported, out) = build_fixture(&root);
    let bundle_path = out.join("source-bundle.json");
    assert_source_bundle_files(&read_json(&bundle_path));

    let origin_map = read_json(&out.join("origin-map.json"));
    let reveal_origin_id = origin_map["entries"]
        .as_array()
        .expect("origin entries")
        .first()
        .and_then(|entry| entry["id"].as_str())
        .expect("origin id")
        .to_string();

    std::fs::remove_file(&app).expect("remove app source");
    std::fs::remove_file(&imported).expect("remove imported source");

    let dap_frames = run_dap_stdio_frames(&[
        serde_json::json!({"seq": 1, "type": "request", "command": "initialize", "arguments": {}}),
        serde_json::json!({
            "seq": 2,
            "type": "request",
            "command": "launch",
            "arguments": {
                "program": app.display().to_string(),
                "sourceBundle": bundle_path.display().to_string(),
            },
        }),
        serde_json::json!({"seq": 3, "type": "request", "command": "loadedSources", "arguments": {}}),
    ]);
    let dap_launch_bundle = &response(&dap_frames, "launch", 2)["body"]["sourceBundle"];
    let dap_loaded_sources = response(&dap_frames, "loadedSources", 3)["body"]["sources"]
        .as_array()
        .expect("dap loaded sources");

    let out_arg = out.display().to_string();
    let run = run_orv_json(&[
        "editor",
        "run-debug",
        &out_arg,
        "--control",
        "next",
        "--watch-expression",
        "total",
    ]);
    let reveal = run_orv_json(&["reveal", &out_arg, &reveal_origin_id]);

    let run_launch_bundle = &run["debug"]["launch"]["body"]["sourceBundle"];
    let run_panel_bundle = &run["panels"]["debug"]["source_bundle"];
    for key in ["path", "hash", "fileCount"] {
        assert_eq!(dap_launch_bundle[key], run_launch_bundle[key], "{key}");
        assert_eq!(dap_launch_bundle[key], run_panel_bundle[key], "{key}");
    }
    let reveal_source_bundle = reveal["production"]["graph_contract"]
        .as_array()
        .expect("reveal graph contract")
        .iter()
        .find(|target| target["kind"] == "source_bundle")
        .expect("reveal source bundle target");
    assert!(dap_launch_bundle["path"]
        .as_str()
        .is_some_and(|path| path.ends_with(reveal_source_bundle["path"].as_str().expect("path"))));
    assert_eq!(
        dap_launch_bundle["hash"],
        reveal_source_bundle["artifact_hash"]
    );
    assert_eq!(
        dap_launch_bundle["fileCount"],
        reveal_source_bundle["file_count"]
    );

    let run_summary = &run["production_context"]["summary"];
    let run_panel_summary = &run["panels"]["debug"]["production_summary"];
    let reveal_summary = &reveal["production"]["summary"];
    for key in [
        "source_bundle_file_count",
        "project_graph_node_count",
        "origin_entry_count",
    ] {
        assert_eq!(run_summary[key], run_panel_summary[key], "{key}");
        assert_eq!(run_summary[key], reveal_summary[key], "{key}");
    }

    let dap_loaded_source_count = dap_loaded_sources.len() as u64;
    let run_loaded_source_count = run["panels"]["debug"]["loaded_source_count"]
        .as_u64()
        .expect("run loaded source count");
    let run_snapshot_count = run["panels"]["debug"]["source_snapshot_count"]
        .as_u64()
        .expect("run source snapshot count");
    assert_eq!(run_loaded_source_count, dap_loaded_source_count);
    assert_eq!(
        run_loaded_source_count,
        run["debug"]["loaded_sources"]["sources"]
            .as_array()
            .expect("run loaded sources")
            .len() as u64
    );
    assert_eq!(
        run_snapshot_count,
        run["debug"]["source_snapshots"]
            .as_array()
            .expect("run source snapshots")
            .len() as u64
    );

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn verify_build_rejects_imported_source_bundle_checksum_drift() {
    let root = support::temp_dir("dap-source-bundle-checksum-drift");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("temp root");
    let (_app, _imported, out) = build_fixture(&root);
    let bundle_path = out.join("source-bundle.json");
    let mut bundle = read_json(&bundle_path);
    bundle["files"][1]["content_hash"] = serde_json::json!("fnv1a64:0000000000000000");
    write_json(&bundle_path, &bundle);

    let err = run_orv_failure(&["verify-build", &out.display().to_string()]);

    assert!(err.contains("content hash mismatch for"), "{err}");
    assert!(err.contains("models/user.orv"), "{err}");
    let _ = std::fs::remove_dir_all(root);
}
