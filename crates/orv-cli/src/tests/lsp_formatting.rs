use super::*;

#[test]
fn lsp_formatting_returns_full_document_text_edit_for_unsaved_content() {
    let dir = temp_output_dir("lsp-formatting");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let source = dir.join("app.orv");
    let uri = format!("file://{}", source.display());
    let mut session = LspSession::default();
    session.handle_notification(&serde_json::json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": uri,
                    "languageId": "orv",
                    "version": 1,
                    "text": "@server {\n@listen 8080  \n@route GET /ping {\n@respond 200 {\nok: true\n}\n}\n}\n",
                },
            },
        }));

    let response = session.jsonrpc_response(&serde_json::json!({
        "jsonrpc": "2.0",
        "id": 44,
        "method": "textDocument/formatting",
        "params": {
            "textDocument": {
                "uri": format!("file://{}", source.display()),
            },
            "options": {
                "tabSize": 2,
                "insertSpaces": true,
            },
        },
    }));

    assert_eq!(response["id"], 44);
    assert!(response.get("error").is_none(), "{response}");
    let edits = response["result"].as_array().expect("format edits");
    assert_eq!(edits.len(), 1);
    assert_eq!(edits[0]["range"]["start"]["line"], 0);
    assert_eq!(edits[0]["range"]["start"]["character"], 0);
    assert_eq!(edits[0]["range"]["end"]["line"], 8);
    assert_eq!(edits[0]["range"]["end"]["character"], 0);
    assert_eq!(
            edits[0]["newText"],
            "@server {\n  @listen 8080\n  @route GET /ping {\n    @respond 200 {\n      ok: true\n    }\n  }\n}\n"
        );
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn lsp_range_formatting_uses_surrounding_indent_context() {
    let dir = temp_output_dir("lsp-range-formatting");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let source = dir.join("app.orv");
    let uri = format!("file://{}", source.display());
    let mut session = LspSession::default();
    session.handle_notification(&serde_json::json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": uri,
                    "languageId": "orv",
                    "version": 1,
                    "text": "@server {\n@listen 8080\n@route GET /ping {\n@respond 200 {\nok: true\n}\n}\n}\n",
                },
            },
        }));

    let response = session.jsonrpc_response(&serde_json::json!({
        "jsonrpc": "2.0",
        "id": 45,
        "method": "textDocument/rangeFormatting",
        "params": {
            "textDocument": {
                "uri": format!("file://{}", source.display()),
            },
            "range": {
                "start": { "line": 2, "character": 0 },
                "end": { "line": 6, "character": 1 },
            },
            "options": {
                "tabSize": 2,
                "insertSpaces": true,
            },
        },
    }));

    assert_eq!(response["id"], 45);
    assert!(response.get("error").is_none(), "{response}");
    let edits = response["result"].as_array().expect("format edits");
    assert_eq!(edits.len(), 1);
    assert_eq!(edits[0]["range"]["start"]["line"], 2);
    assert_eq!(edits[0]["range"]["start"]["character"], 0);
    assert_eq!(edits[0]["range"]["end"]["line"], 7);
    assert_eq!(edits[0]["range"]["end"]["character"], 0);
    assert_eq!(
        edits[0]["newText"],
        "  @route GET /ping {\n    @respond 200 {\n      ok: true\n    }\n  }\n"
    );
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn lsp_on_type_formatting_indents_new_current_line() {
    let dir = temp_output_dir("lsp-on-type-formatting-newline");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let source = dir.join("app.orv");
    let uri = format!("file://{}", source.display());
    let mut session = LspSession::default();
    session.handle_notification(&serde_json::json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didOpen",
        "params": {
            "textDocument": {
                "uri": uri,
                "languageId": "orv",
                "version": 1,
                "text": "@server {\n",
            },
        },
    }));

    let response = session.jsonrpc_response(&serde_json::json!({
        "jsonrpc": "2.0",
        "id": 46,
        "method": "textDocument/onTypeFormatting",
        "params": {
            "textDocument": {
                "uri": format!("file://{}", source.display()),
            },
            "position": { "line": 1, "character": 0 },
            "ch": "\n",
            "options": {
                "tabSize": 2,
                "insertSpaces": true,
            },
        },
    }));

    assert_eq!(response["id"], 46);
    assert!(response.get("error").is_none(), "{response}");
    let edits = response["result"].as_array().expect("format edits");
    assert_eq!(edits.len(), 1);
    assert_eq!(edits[0]["range"]["start"]["line"], 1);
    assert_eq!(edits[0]["range"]["start"]["character"], 0);
    assert_eq!(edits[0]["newText"], "  ");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn lsp_on_type_formatting_aligns_closing_brace_line() {
    let dir = temp_output_dir("lsp-on-type-formatting-brace");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let source = dir.join("app.orv");
    let uri = format!("file://{}", source.display());
    let mut session = LspSession::default();
    session.handle_notification(&serde_json::json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": uri,
                    "languageId": "orv",
                    "version": 1,
                    "text": "@server {\n  @route GET /ping {\n    @respond 200 {\n      ok: true\n}\n  }\n}\n",
                },
            },
        }));

    let response = session.jsonrpc_response(&serde_json::json!({
        "jsonrpc": "2.0",
        "id": 47,
        "method": "textDocument/onTypeFormatting",
        "params": {
            "textDocument": {
                "uri": format!("file://{}", source.display()),
            },
            "position": { "line": 4, "character": 1 },
            "ch": "}",
            "options": {
                "tabSize": 2,
                "insertSpaces": true,
            },
        },
    }));

    assert_eq!(response["id"], 47);
    assert!(response.get("error").is_none(), "{response}");
    let edits = response["result"].as_array().expect("format edits");
    assert_eq!(edits.len(), 1);
    assert_eq!(edits[0]["range"]["start"]["line"], 4);
    assert_eq!(edits[0]["range"]["end"]["line"], 5);
    assert_eq!(edits[0]["newText"], "    }\n");
    let _ = std::fs::remove_dir_all(dir);
}
