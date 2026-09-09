use super::*;

#[test]
fn editor_export_subcommand_is_accepted() {
    let parsed = Cli::try_parse_from([
        "orv",
        "editor",
        "export",
        "src/main.orv",
        "--out",
        "target/orv-editor",
    ]);
    if let Err(err) = parsed {
        panic!("{}", err.render());
    }
}

#[test]
fn editor_export_writes_static_editor_shell_and_state() {
    let dir = temp_output_dir("editor-export");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let path = dir.join("app.orv");
    std::fs::write(
        &path,
        "struct User { id: int }\n@out \"editor-export-ready\"\n",
    )
    .expect("write source");
    let out = dir.join("editor");

    cmd_editor_export(&path, &out).expect("editor export");

    let html = std::fs::read_to_string(out.join("index.html")).expect("editor html");
    let runtime_panel =
        std::fs::read_to_string(out.join(EDITOR_RUNTIME_PANEL_HTML_PATH)).expect("runtime panel");
    let state = read_json_value(&out.join("state.json")).expect("editor state");
    let native_host =
        read_json_value(&out.join(EDITOR_NATIVE_HOST_MANIFEST_PATH)).expect("native host");
    assert!(html.contains("id=\"orv-editor\""));
    assert!(html.contains("id=\"routes-list\""));
    assert!(html.contains("renderEditorState"));
    assert!(html.contains("Routes"));
    assert!(html.contains("Runtime"));
    assert!(html.contains("Project Graph"));
    assert!(html.contains("id=\"editor-graph-view\""));
    assert_eq!(state["schema_version"], 1);
    assert_eq!(state["snapshot"]["schema_version"], 1);
    assert_eq!(state["snapshot"]["project_graph"]["schema_version"], 1);
    assert_eq!(state["runtime"]["runtime"]["status"], "ok");
    assert_eq!(
        state["runtime"]["runtime"]["stdout"],
        "editor-export-ready\n"
    );
    assert_eq!(native_host["runtime"]["status"], "ok");
    assert!(native_host["runtime"]["frame_count"]
        .as_u64()
        .is_some_and(|count| count > 0));
    assert!(runtime_panel.contains("Runtime Panel"));
    assert!(runtime_panel.contains("editor-export-ready"));
    assert_eq!(
        native_host["artifacts"]["runtime_panel_html"],
        EDITOR_RUNTIME_PANEL_HTML_PATH
    );
    assert_eq!(
        native_host["runtime"]["panel_html_path"],
        EDITOR_RUNTIME_PANEL_HTML_PATH
    );
    assert_eq!(
        native_host["runtime"]["panel_artifact"]["path"],
        EDITOR_RUNTIME_PANEL_HTML_PATH
    );
    assert_eq!(
        native_host["runtime"]["panel_artifact"]["kind"],
        "orv.editor.runtime.panel"
    );
    let panels = native_host["panels"]
        .as_array()
        .expect("native host panel inventory");
    assert!(panels.iter().any(|panel| {
        panel["name"] == "debug_result"
            && panel["artifact"]["path"] == EDITOR_DEBUG_SESSION_RESULT_PATH
    }));
    assert!(panels.iter().any(|panel| {
        panel["name"] == "runtime" && panel["artifact"]["path"] == EDITOR_RUNTIME_PANEL_HTML_PATH
    }));
    assert!(!panels.iter().any(|panel| panel["name"] == "production"));
    assert!(!panels.iter().any(|panel| panel["name"] == "trace"));
    assert_eq!(native_host["runtime"]["panel_contract"]["root"], "runtime");
    let runtime_sections = native_host["runtime"]["panel_contract"]["sections"]
        .as_array()
        .expect("runtime panel sections");
    assert!(runtime_sections
        .iter()
        .any(|section| section["name"] == "panel" && section["path"] == "runtime.panel"));
    assert!(runtime_sections
        .iter()
        .any(|section| section["name"] == "frames" && section["path"] == "runtime.frames"));
    assert!(runtime_sections
        .iter()
        .any(|section| section["name"] == "panel_artifact"
            && section["path"] == "runtime.panel_artifact"));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn editor_export_renders_runtime_frame_inspector() {
    let dir = temp_output_dir("editor-export-runtime-frames");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let path = dir.join("app.orv");
    std::fs::write(
        &path,
        "let total: int = 41\nlet next: int = total + 1\n@out next\n",
    )
    .expect("write source");
    let out = dir.join("editor");

    cmd_editor_export(&path, &out).expect("editor export");

    let html = std::fs::read_to_string(out.join("index.html")).expect("editor html");
    let state = read_json_value(&out.join("state.json")).expect("editor state");
    assert!(html.contains("id=\"runtime-frame-list\""));
    assert!(html.contains("id=\"runtime-frame-detail\""));
    assert!(html.contains("renderRuntimeDetail"));
    assert!(html.contains("Runtime Frames"));
    let frames = state["runtime"]["frames"]
        .as_array()
        .expect("runtime frames");
    assert!(!frames.is_empty());
    assert!(frames.iter().any(|frame| {
        frame["locals"]
            .as_array()
            .is_some_and(|locals| locals.iter().any(|local| local["name"] == "next"))
    }));
    let _ = std::fs::remove_dir_all(dir);
}
