use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;

const EDITOR_SNAPSHOT_GOLDEN: &str =
    include_str!("../../../docs/samples/editor-snapshot-v1.golden.json");
const EDITOR_EXPORT_OUTPUT_GOLDEN: &str =
    include_str!("../../../docs/samples/editor-export-output-v1.golden.json");

const SNAPSHOT_ROOT_KEYS: &[&str] = &[
    "diagnostics",
    "entry",
    "live_refresh",
    "panels",
    "project_graph",
    "schema_version",
];
const SNAPSHOT_ENTRY_KEYS: &[&str] = &["path", "uri"];
const LIVE_REFRESH_KEYS: &[&str] = &["project_graph_hash", "strategy", "watch"];
const WATCH_KEYS: &[&str] = &["sources"];
const WATCH_SOURCE_KEYS: &[&str] = &["content_hash", "file", "path", "uri"];
const SNAPSHOT_PANEL_KEYS: &[&str] = &["domains", "files", "routes", "schema"];
const FILE_PANEL_KEYS: &[&str] = &["file", "name", "node_id", "path", "uri"];
const ROUTE_PANEL_KEYS: &[&str] = &["location", "method", "name", "origin_id", "path"];
const NODE_PANEL_KEYS: &[&str] = &["kind", "location", "name", "node_id"];
const EXPORT_OUTPUT_KEYS: &[&str] = &["entry", "files", "kind", "out", "schema_version"];
const STATE_ROOT_KEYS: &[&str] = &[
    "debug",
    "kind",
    "production",
    "runtime",
    "schema_version",
    "snapshot",
];
const RUNTIME_ROOT_KEYS: &[&str] = &["entry", "frames", "panels", "runtime", "schema_version"];
const DEBUG_ROOT_KEYS: &[&str] = &[
    "adapter",
    "breakpoint_sources",
    "capabilities",
    "configurations",
    "controls",
    "data_breakpoints",
    "exception_filters",
    "function_breakpoints",
    "production_context",
    "result_artifact",
    "schema_version",
    "session_runner",
    "source_inventory",
];
const DEBUG_SESSION_RUNNER_KEYS: &[&str] = &[
    "command",
    "controls",
    "kind",
    "production_context",
    "program",
    "result",
    "schema_version",
    "session",
    "source_bundle",
    "transport",
];
const PRODUCTION_KEYS: &[&str] = &[
    "build_dir",
    "client",
    "commerce_adapters",
    "db_adapters",
    "graph_contract",
    "kind",
    "native_server",
    "preflight",
    "schema_version",
    "static",
    "summary",
];
const NATIVE_HOST_ROOT_KEYS: &[&str] = &[
    "artifacts",
    "capabilities",
    "debug",
    "entry",
    "host",
    "kind",
    "panels",
    "production",
    "runtime",
    "schema_version",
    "trace",
];
const NATIVE_HOST_ARTIFACT_KEYS: &[&str] = &[
    "debug_session_result",
    "debug_session_result_html",
    "debug_session_runner",
    "native_host_bridge_js",
    "native_host_desktop_app_entitlements",
    "native_host_desktop_app_info_plist",
    "native_host_desktop_app_main",
    "native_host_desktop_app_package",
    "native_host_desktop_launcher",
    "native_host_desktop_package",
    "native_host_desktop_package_script",
    "native_host_desktop_packaging",
    "production_panel_html",
    "runtime_panel_html",
    "shell",
    "state",
];
const NATIVE_HOST_CAPABILITY_KEYS: &[&str] = &[
    "client_bundles",
    "dap_controls",
    "dap_production_context",
    "dap_sources",
    "native_host_bridge",
    "native_host_desktop_app",
    "native_host_desktop_package",
    "native_host_desktop_packaging",
    "native_host_desktop_platform_matrix",
    "native_host_local_bridge",
    "production_adapters",
    "production_graph_contract",
    "production_preflight",
    "production_route_policies",
    "project_graph",
    "runtime_inspection",
    "trace_navigation",
    "trace_reveal_actions",
];
const NATIVE_HOST_HOST_KEYS: &[&str] = &[
    "action_endpoint",
    "bridge_script",
    "command_format",
    "desktop_app",
    "desktop_launcher",
    "desktop_package",
    "desktop_packaging",
    "desktop_platform_matrix",
    "kind",
    "schema_version",
    "shell",
];
const PANEL_ENTRY_KEYS: &[&str] = &["artifact", "name", "panel_contract", "root", "title"];
const PANEL_ARTIFACT_KEYS: &[&str] = &["kind", "media_type", "path"];
const REQUIRED_EXPORT_FILES: &[&str] = &[
    "index.html",
    "state.json",
    "debug/session-runner.json",
    "native-host.json",
    "native-host/bridge.js",
    "native-host/desktop-package.json",
    "native-host/run-desktop-host.sh",
    "native-host/desktop-packaging.json",
    "native-host/package-desktop-app.sh",
    "native-host/desktop-app/Package.swift",
    "native-host/desktop-app/Info.plist",
    "native-host/desktop-app/OrvEditorDesktop.entitlements",
    "native-host/desktop-app/Sources/OrvEditorDesktop/main.swift",
    "runtime/panel.html",
    "production/panel.html",
];
const WRITTEN_ARTIFACT_KEYS: &[&str] = &[
    "shell",
    "state",
    "debug_session_runner",
    "native_host_bridge_js",
    "native_host_desktop_package",
    "native_host_desktop_launcher",
    "native_host_desktop_packaging",
    "native_host_desktop_package_script",
    "native_host_desktop_app_package",
    "native_host_desktop_app_info_plist",
    "native_host_desktop_app_entitlements",
    "native_host_desktop_app_main",
    "runtime_panel_html",
    "production_panel_html",
];

#[test]
fn editor_snapshot_export_v1_freezes_public_artifact_envelope() {
    let root = temp_dir("editor-snapshot-export-contract");
    std::fs::create_dir_all(&root).expect("create temp dir");
    let snapshot_source = root.join("snapshot.orv");
    write_snapshot_source(&snapshot_source);
    let snapshot = run_orv_json(&["editor", "snapshot", &path_arg(&snapshot_source)]);
    assert_snapshot_contract_with_route(&snapshot, &snapshot_source);
    assert_editor_snapshot_golden(&snapshot, &snapshot_source);

    let export_source = root.join("app.orv");
    write_export_source(&export_source);
    let build_dir = root.join("dist");
    let export_dir = root.join("editor");
    run_orv(&[
        "build",
        &path_arg(&export_source),
        "--prod",
        "--out",
        &path_arg(&build_dir),
    ]);
    let export = run_orv_json(&[
        "editor",
        "export",
        &path_arg(&export_source),
        "--out",
        &path_arg(&export_dir),
        "--build",
        &path_arg(&build_dir),
    ]);
    assert_export_output_contract(&export, &export_source, &export_dir);
    assert_editor_export_output_golden(&export, &export_source, &export_dir);

    let state = read_json(&export_dir.join("state.json"));
    assert_state_contract(&state, &export_source, &build_dir);

    let native_host = read_json(&export_dir.join("native-host.json"));
    assert_native_host_contract(&native_host);
    assert_static_artifacts(&export_dir);

    let _ = std::fs::remove_dir_all(root);
}

fn write_snapshot_source(path: &Path) {
    std::fs::write(
        path,
        r#"struct User { id: int }
define Auth() -> { @out "auth" }
@server {
  @listen 8080
  @route GET /users/:id { @respond 200 { ok: true } }
}
"#,
    )
    .expect("write snapshot source");
}

fn write_export_source(path: &Path) {
    std::fs::write(
        path,
        r#"struct User { id: int }
@out @html {
  @body { @h1 "Editor Export" }
}
"#,
    )
    .expect("write export source");
}

fn assert_snapshot_contract_with_route(snapshot: &Value, source: &Path) {
    assert_snapshot_contract(snapshot, source);
    let route = snapshot["panels"]["routes"]
        .as_array()
        .expect("routes")
        .iter()
        .find(|route| route["path"] == "/users/:id")
        .expect("route panel item");
    assert_object_keys(route, ROUTE_PANEL_KEYS);
    let domain = snapshot["panels"]["domains"]
        .as_array()
        .expect("domains")
        .iter()
        .find(|item| item["name"] == "Auth")
        .expect("domain panel item");
    assert_object_keys(domain, NODE_PANEL_KEYS);
}

fn assert_editor_snapshot_golden(snapshot: &Value, source: &Path) {
    let expected: Value =
        serde_json::from_str(EDITOR_SNAPSHOT_GOLDEN).expect("editor snapshot golden");
    assert_eq!(
        normalize_editor_snapshot_for_golden(snapshot.clone(), source),
        expected,
        "editor snapshot golden drift"
    );
}

fn assert_snapshot_contract(snapshot: &Value, source: &Path) {
    assert_object_keys(snapshot, SNAPSHOT_ROOT_KEYS);
    assert_eq!(snapshot["schema_version"], 1);
    assert_object_keys(&snapshot["entry"], SNAPSHOT_ENTRY_KEYS);
    assert_eq!(snapshot["entry"]["path"], source.display().to_string());
    assert_eq!(snapshot["project_graph"]["schema_version"], 1);

    assert_object_keys(&snapshot["live_refresh"], LIVE_REFRESH_KEYS);
    assert_eq!(snapshot["live_refresh"]["strategy"], "source-hash");
    assert_object_keys(&snapshot["live_refresh"]["watch"], WATCH_KEYS);
    let watch_source = snapshot["live_refresh"]["watch"]["sources"]
        .as_array()
        .expect("watch sources")
        .first()
        .expect("watch source");
    assert_object_keys(watch_source, WATCH_SOURCE_KEYS);
    assert!(watch_source["content_hash"]
        .as_str()
        .is_some_and(|hash| hash.starts_with("fnv1a64:")));

    assert_object_keys(&snapshot["panels"], SNAPSHOT_PANEL_KEYS);
    let file = snapshot["panels"]["files"]
        .as_array()
        .expect("files")
        .first()
        .expect("file panel item");
    assert_object_keys(file, FILE_PANEL_KEYS);
    let schema = snapshot["panels"]["schema"]
        .as_array()
        .expect("schema")
        .iter()
        .find(|item| item["name"] == "User")
        .expect("schema panel item");
    assert_object_keys(schema, NODE_PANEL_KEYS);
}

fn normalize_editor_snapshot_for_golden(mut snapshot: Value, source: &Path) -> Value {
    if let Some(hash) = snapshot.pointer_mut("/live_refresh/project_graph_hash") {
        *hash = Value::String("<project-graph-hash>".to_string());
    }
    normalize_source_paths(&mut snapshot, source);
    snapshot
}

fn normalize_source_paths(value: &mut Value, source: &Path) {
    let source_path = source.display().to_string();
    let canonical_path = std::fs::canonicalize(source)
        .expect("canonical source path")
        .display()
        .to_string();
    let source_uri = format!("file://{source_path}");
    let canonical_uri = format!("file://{canonical_path}");
    normalize_path_strings(
        value,
        &[
            (source_path.as_str(), "<entry>"),
            (canonical_path.as_str(), "<entry>"),
            (source_uri.as_str(), "<entry-uri>"),
            (canonical_uri.as_str(), "<entry-uri>"),
        ],
    );
}

fn normalize_path_strings(value: &mut Value, replacements: &[(&str, &str)]) {
    match value {
        Value::String(text) => {
            if let Some((_, replacement)) = replacements
                .iter()
                .find(|(needle, _)| text.as_str() == *needle)
            {
                *text = (*replacement).to_string();
            }
        }
        Value::Array(items) => {
            for item in items {
                normalize_path_strings(item, replacements);
            }
        }
        Value::Object(object) => {
            for item in object.values_mut() {
                normalize_path_strings(item, replacements);
            }
        }
        Value::Null | Value::Bool(_) | Value::Number(_) => {}
    }
}

fn assert_export_output_contract(export: &Value, source: &Path, out: &Path) {
    assert_object_keys(export, EXPORT_OUTPUT_KEYS);
    assert_eq!(export["schema_version"], 1);
    assert_eq!(export["kind"], "orv.editor.export");
    assert_eq!(export["entry"], source.display().to_string());
    assert_eq!(export["out"], out.display().to_string());
    let files = export["files"].as_array().expect("export files");
    for required in REQUIRED_EXPORT_FILES {
        assert!(
            files.iter().any(|file| file == required),
            "missing export file {required}"
        );
    }
}

fn assert_editor_export_output_golden(export: &Value, source: &Path, out: &Path) {
    let expected: Value =
        serde_json::from_str(EDITOR_EXPORT_OUTPUT_GOLDEN).expect("editor export output golden");
    assert_eq!(
        normalize_editor_export_output_for_golden(export.clone(), source, out),
        expected,
        "editor export output golden drift"
    );
}

fn normalize_editor_export_output_for_golden(
    mut export: Value,
    source: &Path,
    out: &Path,
) -> Value {
    let source_path = source.display().to_string();
    let out_path = out.display().to_string();
    normalize_path_strings(
        &mut export,
        &[
            (source_path.as_str(), "<entry>"),
            (out_path.as_str(), "<out>"),
        ],
    );
    export
}

fn assert_state_contract(state: &Value, source: &Path, build: &Path) {
    assert_object_keys(state, STATE_ROOT_KEYS);
    assert_eq!(state["schema_version"], 1);
    assert_eq!(state["kind"], "orv.editor.export");
    assert_snapshot_contract(&state["snapshot"], source);
    assert_object_keys(&state["runtime"], RUNTIME_ROOT_KEYS);
    assert_eq!(state["runtime"]["schema_version"], 1);
    assert_object_keys(&state["debug"], DEBUG_ROOT_KEYS);
    assert_eq!(state["debug"]["schema_version"], 1);
    assert_eq!(state["debug"]["adapter"]["protocol"], "dap");
    assert_eq!(
        state["debug"]["session_runner"]["kind"],
        "orv.editor.debug.runner"
    );
    assert_object_keys(&state["debug"]["session_runner"], DEBUG_SESSION_RUNNER_KEYS);
    assert_eq!(state["debug"]["session_runner"]["schema_version"], 1);
    assert_eq!(
        state["debug"]["production_context"]["build_dir"],
        build.display().to_string()
    );

    assert_object_keys(&state["production"], PRODUCTION_KEYS);
    assert_eq!(state["production"]["kind"], "orv.editor.production");
    assert_eq!(
        state["production"]["build_dir"],
        build.display().to_string()
    );
    assert_eq!(state["production"]["summary"]["graph_contract_count"], 3);
}

fn assert_native_host_contract(native_host: &Value) {
    assert_object_keys(native_host, NATIVE_HOST_ROOT_KEYS);
    assert_eq!(native_host["schema_version"], 1);
    assert_eq!(native_host["kind"], "orv.editor.native_host");
    assert_object_keys(&native_host["artifacts"], NATIVE_HOST_ARTIFACT_KEYS);
    assert_eq!(native_host["artifacts"]["shell"], "index.html");
    assert_eq!(
        native_host["artifacts"]["production_panel_html"],
        "production/panel.html"
    );
    assert_object_keys(&native_host["capabilities"], NATIVE_HOST_CAPABILITY_KEYS);
    assert_eq!(native_host["capabilities"]["project_graph"], true);
    assert_eq!(native_host["capabilities"]["runtime_inspection"], true);
    assert_eq!(native_host["capabilities"]["dap_production_context"], true);
    assert_eq!(
        native_host["capabilities"]["production_graph_contract"],
        true
    );
    assert_eq!(native_host["capabilities"]["trace_navigation"], false);

    assert_object_keys(&native_host["host"], NATIVE_HOST_HOST_KEYS);
    assert_eq!(
        native_host["host"]["action_endpoint"],
        "/__orv/native-host/action"
    );
    assert_eq!(
        native_host["runtime"]["panel_html_path"],
        "runtime/panel.html"
    );
    assert_eq!(
        native_host["production"]["panel_html_path"],
        "production/panel.html"
    );
    assert_eq!(native_host["trace"], Value::Null);
    let exported_files = native_host_exported_files(native_host);
    for key in WRITTEN_ARTIFACT_KEYS {
        let artifact = native_host["artifacts"][*key]
            .as_str()
            .unwrap_or_else(|| panic!("native host artifact {key}"));
        assert!(
            exported_files.contains(artifact),
            "artifact {key} path {artifact} must be listed in export files"
        );
    }

    let panels = native_host["panels"].as_array().expect("panels");
    assert_panel_contract(find_panel(panels, "debug_result"));
    assert_panel_contract(find_panel(panels, "runtime"));
    assert_panel_contract(find_panel(panels, "production"));
}

fn assert_panel_contract(panel: &Value) {
    assert_object_keys(panel, PANEL_ENTRY_KEYS);
    assert_object_keys(&panel["artifact"], PANEL_ARTIFACT_KEYS);
    assert_eq!(panel["panel_contract"]["schema_version"], 1);
}

fn assert_static_artifacts(out: &Path) {
    for relative in REQUIRED_EXPORT_FILES {
        assert!(
            out.join(relative).is_file(),
            "missing artifact {}",
            out.join(relative).display()
        );
    }
    let html = std::fs::read_to_string(out.join("index.html")).expect("editor html");
    assert!(html.contains("id=\"orv-editor\""));
    assert!(html.contains("native-host/bridge.js"));
    let production_panel =
        std::fs::read_to_string(out.join("production/panel.html")).expect("production panel");
    assert!(production_panel.contains("Production Panel"));
    assert!(production_panel.contains("Panel Contract"));
    let runner = read_json(&out.join("debug/session-runner.json"));
    assert_object_keys(&runner, DEBUG_SESSION_RUNNER_KEYS);
    assert_eq!(runner["schema_version"], 1);
    assert_eq!(runner["kind"], "orv.editor.debug.runner");
    assert_object_keys(&runner["transport"], &["framing", "protocol"]);
    assert_eq!(runner["transport"]["protocol"], "dap");
    assert_eq!(runner["transport"]["framing"], "content-length");
}

fn native_host_exported_files(native_host: &Value) -> BTreeSet<&str> {
    REQUIRED_EXPORT_FILES
        .iter()
        .copied()
        .chain(std::iter::once("native-host.json"))
        .filter(|path| {
            native_host["artifacts"]
                .as_object()
                .is_some_and(|artifacts| {
                    artifacts
                        .values()
                        .any(|value| value.as_str() == Some(*path))
                })
        })
        .collect()
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
