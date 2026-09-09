use super::*;

#[test]
fn lsp_document_link_returns_import_targets() {
    let dir = temp_output_dir("lsp-document-link");
    let models = dir.join("models");
    std::fs::create_dir_all(&models).expect("create models dir");
    let source = dir.join("app.orv");
    let imported = models.join("user.orv");
    std::fs::write(&source, "import models.user.User\nlet ok: int = 1\n").expect("write source");
    std::fs::write(&imported, "pub struct User { id: int }\n").expect("write imported");
    let canonical_imported = std::fs::canonicalize(&imported).expect("canonical imported");

    let response = lsp_jsonrpc_response(&serde_json::json!({
        "jsonrpc": "2.0",
        "id": 24,
        "method": "textDocument/documentLink",
        "params": {
            "textDocument": {
                "uri": format!("file://{}", source.display()),
            },
        },
    }));

    assert_eq!(response["id"], 24);
    assert!(response.get("error").is_none(), "{response}");
    let links = response["result"].as_array().expect("document links");
    let link = links
        .iter()
        .find(|link| link["target"] == format!("file://{}", canonical_imported.display()))
        .expect("import document link");
    assert_eq!(link["range"]["start"]["line"], 0);
    assert_eq!(link["range"]["start"]["character"], 0);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn lsp_folding_range_returns_multiline_declarations() {
    let dir = temp_output_dir("lsp-folding-range");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let source = dir.join("app.orv");
    std::fs::write(
        &source,
        r#"struct User {
  id: int
  email: string
}

function greet(user: User): string -> {
  "hello"
}
"#,
    )
    .expect("write source");

    let response = lsp_jsonrpc_response(&serde_json::json!({
        "jsonrpc": "2.0",
        "id": 25,
        "method": "textDocument/foldingRange",
        "params": {
            "textDocument": {
                "uri": format!("file://{}", source.display()),
            },
        },
    }));

    assert_eq!(response["id"], 25);
    assert!(response.get("error").is_none(), "{response}");
    let ranges = response["result"].as_array().expect("folding ranges");
    assert!(ranges.iter().any(|range| {
        range["startLine"] == 0 && range["endLine"].as_u64().is_some_and(|line| line >= 3)
    }));
    assert!(ranges.iter().any(|range| {
        range["startLine"] == 5 && range["endLine"].as_u64().is_some_and(|line| line >= 7)
    }));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn lsp_selection_range_returns_structural_parent_range() {
    let dir = temp_output_dir("lsp-selection-range");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let source = dir.join("app.orv");
    std::fs::write(
        &source,
        r#"struct User {
  id: int
  email: string
}

function greet(user: User): string -> {
  "hello"
}
"#,
    )
    .expect("write source");

    let response = lsp_jsonrpc_response(&serde_json::json!({
        "jsonrpc": "2.0",
        "id": 26,
        "method": "textDocument/selectionRange",
        "params": {
            "textDocument": {
                "uri": format!("file://{}", source.display()),
            },
            "positions": [
                {
                    "line": 1,
                    "character": 4,
                },
            ],
        },
    }));

    assert_eq!(response["id"], 26);
    assert!(response.get("error").is_none(), "{response}");
    let selections = response["result"].as_array().expect("selection ranges");
    assert_eq!(selections.len(), 1);
    let selection = &selections[0];
    assert_eq!(selection["range"]["start"]["line"], 0);
    assert_eq!(selection["range"]["start"]["character"], 0);
    assert!(selection["range"]["end"]["line"]
        .as_u64()
        .is_some_and(|line| line >= 3));
    assert!(selection
        .get("parent")
        .is_none_or(serde_json::Value::is_null));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn lsp_document_color_returns_hex_literal_ranges() {
    let dir = temp_output_dir("lsp-document-color");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let source = dir.join("app.orv");
    let source_text = "let accent = \"#336699\"\n";
    std::fs::write(&source, source_text).expect("write source");
    let color_character = source_text.find("#336699").expect("color literal");
    let response = lsp_jsonrpc_response(&serde_json::json!({
        "jsonrpc": "2.0",
        "id": 32,
        "method": "textDocument/documentColor",
        "params": {
            "textDocument": {
                "uri": format!("file://{}", source.display()),
            },
        },
    }));

    assert_eq!(response["id"], 32);
    assert!(response.get("error").is_none(), "{response}");
    let colors = response["result"].as_array().expect("document colors");
    assert_eq!(colors.len(), 1);
    assert_eq!(colors[0]["range"]["start"]["character"], color_character);
    assert_eq!(colors[0]["color"]["red"], 0.2);
    assert_eq!(colors[0]["color"]["green"], 0.4);
    assert_eq!(colors[0]["color"]["blue"], 0.6);
    assert_eq!(colors[0]["color"]["alpha"], 1.0);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn lsp_color_presentation_returns_hex_text_edit() {
    let response = lsp_jsonrpc_response(&serde_json::json!({
        "jsonrpc": "2.0",
        "id": 33,
        "method": "textDocument/colorPresentation",
        "params": {
            "textDocument": {
                "uri": "file:///tmp/app.orv",
            },
            "color": {
                "red": 0.2,
                "green": 0.4,
                "blue": 0.6,
                "alpha": 1.0,
            },
            "range": {
                "start": { "line": 0, "character": 14 },
                "end": { "line": 0, "character": 21 },
            },
        },
    }));

    assert_eq!(response["id"], 33);
    assert!(response.get("error").is_none(), "{response}");
    let presentations = response["result"].as_array().expect("color presentations");
    assert_eq!(presentations.len(), 1);
    assert_eq!(presentations[0]["label"], "#336699");
    assert_eq!(presentations[0]["textEdit"]["newText"], "#336699");
}

#[test]
fn lsp_linked_editing_range_returns_identifier_ranges() {
    let dir = temp_output_dir("lsp-linked-editing-range");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let source = dir.join("app.orv");
    let source_text = "let total = 1\nlet next = total + 1\n";
    std::fs::write(&source, source_text).expect("write source");
    let use_line = source_text.lines().nth(1).expect("use line");
    let use_character = use_line.find("total").expect("identifier use");
    let response = lsp_jsonrpc_response(&serde_json::json!({
        "jsonrpc": "2.0",
        "id": 34,
        "method": "textDocument/linkedEditingRange",
        "params": {
            "textDocument": {
                "uri": format!("file://{}", source.display()),
            },
            "position": {
                "line": 1,
                "character": use_character,
            },
        },
    }));

    assert_eq!(response["id"], 34);
    assert!(response.get("error").is_none(), "{response}");
    let result = response["result"]
        .as_object()
        .expect("linked editing result");
    let ranges = result["ranges"].as_array().expect("linked ranges");
    assert_eq!(ranges.len(), 2);
    assert_eq!(ranges[0]["start"]["line"], 0);
    assert_eq!(ranges[0]["start"]["character"], 4);
    assert_eq!(ranges[1]["start"]["line"], 1);
    assert_eq!(ranges[1]["start"]["character"], use_character);
    assert_eq!(result["wordPattern"], "[A-Za-z_][A-Za-z0-9_]*");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn lsp_linked_editing_range_ignores_builtin_directives() {
    let dir = temp_output_dir("lsp-linked-editing-range-directive");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let source = dir.join("app.orv");
    std::fs::write(&source, "@server {\n  @route GET /ping {\n  }\n}\n").expect("write source");
    let response = lsp_jsonrpc_response(&serde_json::json!({
        "jsonrpc": "2.0",
        "id": 35,
        "method": "textDocument/linkedEditingRange",
        "params": {
            "textDocument": {
                "uri": format!("file://{}", source.display()),
            },
            "position": {
                "line": 1,
                "character": 4,
            },
        },
    }));

    assert_eq!(response["id"], 35);
    assert!(response.get("error").is_none(), "{response}");
    assert!(response["result"].is_null());
    let _ = std::fs::remove_dir_all(dir);
}
