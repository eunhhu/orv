use super::*;

#[test]
fn editor_desktop_run_probe_spawns_host_and_reads_ready_json() {
    let dir = temp_output_dir("editor-desktop-run-probe");
    std::fs::create_dir_all(dir.join("native-host")).expect("create native-host dir");
    let session_path = dir.join(EDITOR_NATIVE_HOST_DESKTOP_SESSION_PATH);
    write_json(
        &session_path,
        &serde_json::json!({
            "schema_version": 1,
            "kind": "orv.editor.native_host.desktop_shell",
            "status": "ready",
            "root": dir.display().to_string(),
            "package": {
                "path": "native-host/desktop-package.json",
                "hash": "fnv1a64:test",
            },
            "lifecycle": {
                "spawn": {
                    "command": [
                        "/bin/sh",
                        "-c",
                        "printf '{\"schema_version\":1,\"kind\":\"orv.editor.native_host.server\",\"url\":\"http://127.0.0.1:4321/\"}\\n'; sleep 5",
                    ],
                    "stdout_kind": "orv.editor.native_host.server",
                    "url_field": "url",
                },
            },
            "process_supervision": {
                "mode": "local-child-process",
                "deny_unknown_commands": true,
                "allowed_commands": [],
            },
            "webview": {
                "initial_url_template": "{url}index.html",
                "reload_policy": "reload-panel-artifacts-after-refresh-event",
            },
            "refresh": {
                "events": [{
                    "event": "orv:trace-action-result",
                    "panel": "trace_action_result",
                }],
            },
            "platform_matrix": editor_native_host_desktop_platform_matrix_json(),
            "source_permission_prompt": {
                "mode": "prompt-before-source-reveal",
                "default": "prompt-before-open",
                "denied_mode": "open-read-only",
                "reveal_requires_origin_id": true,
                "webview_injection": "orvNativeHostSourcePermissions",
                "decision_event": "orv:source-permission",
                "blocked_event": "orv:source-permission-blocked",
                "root_count": 1,
                "source_count": 0,
                "allowed_roots": [dir.display().to_string()],
                "source_hashes": [],
                "prompt": {
                    "title": "Allow orv source reveal access?",
                    "allow_label": "Allow Source Reveal",
                    "read_only_label": "Open Read-Only",
                    "quit_label": "Quit",
                },
            },
            "artifact_checks": [],
            "session_artifact": {
                "path": EDITOR_NATIVE_HOST_DESKTOP_SESSION_PATH,
                "kind": "orv.editor.native_host.desktop_shell",
            },
        }),
    )
    .expect("write desktop session");

    let run = editor_native_host_desktop_run_probe_json(&session_path, "127.0.0.1:4322")
        .expect("desktop run probe");

    assert_eq!(run["kind"], "orv.editor.native_host.desktop_run");
    assert_eq!(run["status"], "probe_ready");
    assert_eq!(run["host"]["kind"], "orv.editor.native_host.server");
    assert_eq!(run["host"]["url"], "http://127.0.0.1:4321/");
    assert_eq!(run["webview"]["url"], "http://127.0.0.1:4321/index.html");
    assert_eq!(run["process"]["supervision"]["deny_unknown_commands"], true);
    assert!(run["process"]["pid"].as_u64().is_some_and(|pid| pid > 0));
    assert_eq!(
        run["source_permission_prompt"]["default"],
        "prompt-before-open"
    );
    let _ = std::fs::remove_dir_all(dir);
}
