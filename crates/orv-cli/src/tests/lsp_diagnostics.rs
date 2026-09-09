use super::*;

#[test]
fn lsp_snapshot_includes_diagnostics_graph_and_document_symbols() {
    let dir = temp_output_dir("lsp-snapshot");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let source = dir.join("app.orv");
    std::fs::write(
        &source,
        r#"struct User {
  id: int
}

function greet(user: User): string -> "hello"
"#,
    )
    .expect("write source");

    let snapshot = lsp_snapshot_json(&source).expect("lsp snapshot");

    assert_eq!(snapshot["schema_version"], 1);
    assert_eq!(
        snapshot["diagnostics"]
            .as_array()
            .expect("diagnostics")
            .len(),
        0
    );
    assert!(snapshot["project_graph"]["nodes"]
        .as_array()
        .expect("nodes")
        .iter()
        .any(|node| node["kind"] == "struct" && node["name"] == "User"));
    let symbols = snapshot["document_symbols"]
        .as_array()
        .expect("document symbols");
    let user = symbols
        .iter()
        .find(|symbol| symbol["name"] == "User")
        .expect("User symbol");
    assert_eq!(user["kind"], "Struct");
    assert_eq!(user["range"]["start"]["line"], 0);
    assert!(symbols
        .iter()
        .any(|symbol| symbol["name"] == "greet" && symbol["kind"] == "Function"));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn lsp_text_document_diagnostic_returns_full_report_for_file_uri() {
    let dir = temp_output_dir("lsp-diagnostic");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let source = dir.join("app.orv");
    std::fs::write(&source, "let bad: int = \"wrong\"\n").expect("write source");
    let response = lsp_jsonrpc_response(&serde_json::json!({
        "jsonrpc": "2.0",
        "id": 13,
        "method": "textDocument/diagnostic",
        "params": {
            "textDocument": {
                "uri": format!("file://{}", source.display()),
            },
        },
    }));

    assert_eq!(response["id"], 13);
    assert!(response.get("error").is_none(), "{response}");
    assert_eq!(response["result"]["kind"], "full");
    let items = response["result"]["items"]
        .as_array()
        .expect("diagnostic items");
    assert!(items.iter().any(|item| {
        item["severity"] == 1
            && item["message"]
                .as_str()
                .is_some_and(|message| message.contains("type mismatch"))
    }));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn lsp_text_document_diagnostic_filters_imported_file_diagnostics_by_uri() {
    let dir = temp_output_dir("lsp-imported-text-document-diagnostic");
    let src = dir.join("src");
    let models = src.join("models");
    std::fs::create_dir_all(&models).expect("create models dir");
    let entry = src.join("main.orv");
    let imported = models.join("user.orv");
    std::fs::write(
        dir.join("orv.toml"),
        r#"[project]
name = "lsp-imported-text-document-diagnostic"
entry = "src/main.orv"
"#,
    )
    .expect("write manifest");
    std::fs::write(&entry, "import models.user.User\nlet ok: int = 1\n").expect("write entry");
    std::fs::write(
        &imported,
        "pub struct User { id: int }\nlet bad: int = \"wrong\"\n",
    )
    .expect("write imported");
    let canonical_entry = std::fs::canonicalize(&entry).expect("canonical entry");
    let canonical_imported = std::fs::canonicalize(&imported).expect("canonical imported");
    let mut session = LspSession::default();

    let initialize = session.jsonrpc_response(&serde_json::json!({
        "jsonrpc": "2.0",
        "id": 130,
        "method": "initialize",
        "params": {
            "rootUri": format!("file://{}", dir.display()),
        },
    }));
    let entry_response = session.jsonrpc_response(&serde_json::json!({
        "jsonrpc": "2.0",
        "id": 131,
        "method": "textDocument/diagnostic",
        "params": {
            "textDocument": {
                "uri": format!("file://{}", canonical_entry.display()),
            },
        },
    }));
    let imported_response = session.jsonrpc_response(&serde_json::json!({
        "jsonrpc": "2.0",
        "id": 132,
        "method": "textDocument/diagnostic",
        "params": {
            "textDocument": {
                "uri": format!("file://{}", canonical_imported.display()),
            },
        },
    }));

    assert!(initialize.get("error").is_none(), "{initialize}");
    assert!(entry_response.get("error").is_none(), "{entry_response}");
    assert_eq!(entry_response["result"]["kind"], "full");
    assert!(
        entry_response["result"]["items"]
            .as_array()
            .expect("entry diagnostics")
            .is_empty(),
        "{entry_response}"
    );
    assert!(
        imported_response.get("error").is_none(),
        "{imported_response}"
    );
    let imported_items = imported_response["result"]["items"]
        .as_array()
        .expect("imported diagnostics");
    assert!(imported_items.iter().any(|item| {
        item["message"]
            .as_str()
            .is_some_and(|message| message.contains("type mismatch"))
    }));

    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn lsp_text_document_diagnostic_uses_open_imported_source_overlay() {
    let dir = temp_output_dir("lsp-open-imported-diagnostic");
    let src = dir.join("src");
    let models = src.join("models");
    std::fs::create_dir_all(&models).expect("create models dir");
    let entry = src.join("main.orv");
    let imported = models.join("user.orv");
    std::fs::write(
        dir.join("orv.toml"),
        r#"[project]
name = "lsp-open-imported-diagnostic"
entry = "src/main.orv"
"#,
    )
    .expect("write manifest");
    std::fs::write(&entry, "import models.user.User\nlet ok: int = 1\n").expect("write entry");
    std::fs::write(&imported, "pub struct User { id: int }\nlet ok: int = 1\n")
        .expect("write imported");
    let canonical_imported = std::fs::canonicalize(&imported).expect("canonical imported");
    let mut session = LspSession::default();
    let imported_uri = format!("file://{}", canonical_imported.display());

    let initialize = session.jsonrpc_response(&serde_json::json!({
        "jsonrpc": "2.0",
        "id": 133,
        "method": "initialize",
        "params": {
            "rootUri": format!("file://{}", dir.display()),
        },
    }));
    session.handle_notification(&serde_json::json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didOpen",
        "params": {
            "textDocument": {
                "uri": imported_uri,
                "text": "pub struct User { id: int }\nlet bad: int = \"wrong\"\n",
            },
        },
    }));
    let response = session.jsonrpc_response(&serde_json::json!({
        "jsonrpc": "2.0",
        "id": 134,
        "method": "textDocument/diagnostic",
        "params": {
            "textDocument": {
                "uri": imported_uri,
            },
        },
    }));

    assert!(initialize.get("error").is_none(), "{initialize}");
    assert!(response.get("error").is_none(), "{response}");
    let items = response["result"]["items"].as_array().expect("diagnostics");
    assert!(items.iter().any(|item| {
        item["message"]
            .as_str()
            .is_some_and(|message| message.contains("type mismatch"))
    }));

    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn lsp_code_action_returns_reveal_action_for_diagnostic_range() {
    let dir = temp_output_dir("lsp-code-action");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let source = dir.join("app.orv");
    std::fs::write(&source, "let bad: int = \"wrong\"\n").expect("write source");
    let canonical_source = std::fs::canonicalize(&source).expect("canonical source");

    let response = lsp_jsonrpc_response(&serde_json::json!({
        "jsonrpc": "2.0",
        "id": 32,
        "method": "textDocument/codeAction",
        "params": {
            "textDocument": {
                "uri": format!("file://{}", source.display()),
            },
            "range": {
                "start": { "line": 0, "character": 0 },
                "end": { "line": 0, "character": 25 },
            },
            "context": {
                "diagnostics": [],
            },
        },
    }));

    assert_eq!(response["id"], 32);
    assert!(response.get("error").is_none(), "{response}");
    let actions = response["result"].as_array().expect("code actions");
    let action = actions
        .iter()
        .find(|action| {
            action["title"]
                .as_str()
                .is_some_and(|title| title.contains("type mismatch"))
        })
        .expect("diagnostic reveal action");
    assert_eq!(action["kind"], "quickfix");
    assert_eq!(action["command"]["command"], "orv.revealDiagnostic");
    assert_eq!(action["diagnostics"][0]["source"], "orv");
    assert_eq!(
        action["command"]["arguments"][0],
        format!("file://{}", canonical_source.display())
    );
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn lsp_code_action_inserts_default_route_method_and_path() {
    let dir = temp_output_dir("lsp-code-action-route-method");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let source = dir.join("app.orv");
    std::fs::write(&source, "@server {\n  @route {\n}\n").expect("write source");
    let uri = format!("file://{}", source.display());
    let canonical_uri = format!(
        "file://{}",
        std::fs::canonicalize(&source)
            .expect("canonical source")
            .display()
    );

    let response = lsp_jsonrpc_response(&serde_json::json!({
        "jsonrpc": "2.0",
        "id": 33,
        "method": "textDocument/codeAction",
        "params": {
            "textDocument": {
                "uri": uri,
            },
            "range": {
                "start": { "line": 1, "character": 2 },
                "end": { "line": 1, "character": 10 },
            },
            "context": {
                "diagnostics": [],
            },
        },
    }));

    assert_eq!(response["id"], 33);
    assert!(response.get("error").is_none(), "{response}");
    let actions = response["result"].as_array().expect("code actions");
    let action = actions
        .iter()
        .find(|action| action["title"] == "Insert default GET route head")
        .expect("route method quickfix");
    assert_eq!(action["kind"], "quickfix");
    assert_eq!(action["diagnostics"][0]["code"], "syntax/route-method");
    let change = &action["edit"]["changes"][canonical_uri.as_str()][0];
    assert_eq!(change["newText"], "GET /path ");
    assert_eq!(change["range"]["start"]["line"], 1);
    assert_eq!(change["range"]["start"]["character"], 9);
    assert_eq!(change["range"]["end"], change["range"]["start"]);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn lsp_code_action_inserts_default_route_path() {
    let dir = temp_output_dir("lsp-code-action-route-path");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let source = dir.join("app.orv");
    std::fs::write(&source, "@server {\n  @route GET {\n}\n").expect("write source");
    let uri = format!("file://{}", source.display());
    let canonical_uri = format!(
        "file://{}",
        std::fs::canonicalize(&source)
            .expect("canonical source")
            .display()
    );

    let response = lsp_jsonrpc_response(&serde_json::json!({
        "jsonrpc": "2.0",
        "id": 34,
        "method": "textDocument/codeAction",
        "params": {
            "textDocument": {
                "uri": uri,
            },
            "range": {
                "start": { "line": 1, "character": 2 },
                "end": { "line": 1, "character": 14 },
            },
            "context": {
                "diagnostics": [],
            },
        },
    }));

    assert_eq!(response["id"], 34);
    assert!(response.get("error").is_none(), "{response}");
    let actions = response["result"].as_array().expect("code actions");
    let action = actions
        .iter()
        .find(|action| action["title"] == "Insert default route path")
        .expect("route path quickfix");
    assert_eq!(action["kind"], "quickfix");
    assert_eq!(action["diagnostics"][0]["code"], "syntax/route-path");
    let change = &action["edit"]["changes"][canonical_uri.as_str()][0];
    assert_eq!(change["newText"], "/path ");
    assert_eq!(change["range"]["start"]["line"], 1);
    assert_eq!(change["range"]["start"]["character"], 13);
    assert_eq!(change["range"]["end"], change["range"]["start"]);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn lsp_code_lens_returns_project_graph_reveal_commands() {
    let dir = temp_output_dir("lsp-code-lens");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let source = dir.join("app.orv");
    std::fs::write(
        &source,
        r#"struct User { id: int }

function greet(user: User): string -> "hello"
"#,
    )
    .expect("write source");

    let response = lsp_jsonrpc_response(&serde_json::json!({
        "jsonrpc": "2.0",
        "id": 31,
        "method": "textDocument/codeLens",
        "params": {
            "textDocument": {
                "uri": format!("file://{}", source.display()),
            },
        },
    }));

    assert_eq!(response["id"], 31);
    assert!(response.get("error").is_none(), "{response}");
    let lenses = response["result"].as_array().expect("code lenses");
    let user_lens = lenses
        .iter()
        .find(|lens| lens["command"]["arguments"][1] == "User")
        .expect("User code lens");
    assert_eq!(user_lens["range"]["start"]["line"], 0);
    assert_eq!(user_lens["command"]["command"], "orv.revealSourceNode");
    assert_eq!(user_lens["command"]["title"], "Reveal Struct User");
    assert!(lenses
        .iter()
        .any(|lens| lens["command"]["arguments"][1] == "greet"));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn lsp_execute_command_reveals_project_graph_source_node() {
    let dir = temp_output_dir("lsp-execute-command");
    let src = dir.join("src");
    std::fs::create_dir_all(&src).expect("create src dir");
    let source = src.join("main.orv");
    std::fs::write(
        dir.join("orv.toml"),
        r#"[project]
name = "execute-command"
entry = "src/main.orv"
"#,
    )
    .expect("write manifest");
    std::fs::write(&source, "struct User { id: int }\n").expect("write source");
    let canonical_source = std::fs::canonicalize(&source).expect("canonical source");
    let mut session = LspSession::default();

    let initialize = session.jsonrpc_response(&serde_json::json!({
        "jsonrpc": "2.0",
        "id": 33,
        "method": "initialize",
        "params": {
            "rootUri": format!("file://{}", dir.display()),
        },
    }));
    let lenses = session.jsonrpc_response(&serde_json::json!({
        "jsonrpc": "2.0",
        "id": 34,
        "method": "textDocument/codeLens",
        "params": {
            "textDocument": {
                "uri": format!("file://{}", source.display()),
            },
        },
    }));
    let user_lens = lenses["result"]
        .as_array()
        .expect("code lenses")
        .iter()
        .find(|lens| lens["command"]["arguments"][1] == "User")
        .expect("User code lens")
        .clone();
    let execute = session.jsonrpc_response(&serde_json::json!({
        "jsonrpc": "2.0",
        "id": 35,
        "method": "workspace/executeCommand",
        "params": {
            "command": user_lens["command"]["command"],
            "arguments": user_lens["command"]["arguments"],
        },
    }));

    assert!(initialize.get("error").is_none(), "{initialize}");
    assert!(lenses.get("error").is_none(), "{lenses}");
    assert_eq!(execute["id"], 35);
    assert!(execute.get("error").is_none(), "{execute}");
    assert_eq!(execute["result"]["name"], "User");
    assert_eq!(execute["result"]["kind"], "Struct");
    assert_eq!(
        execute["result"]["source_node"],
        user_lens["command"]["arguments"][0]
    );
    assert_eq!(
        execute["result"]["location"]["uri"],
        format!("file://{}", canonical_source.display())
    );
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn lsp_workspace_diagnostic_returns_imported_file_diagnostics() {
    let dir = temp_output_dir("lsp-workspace-diagnostic");
    let src = dir.join("src");
    let models = src.join("models");
    std::fs::create_dir_all(&models).expect("create models dir");
    let entry = src.join("main.orv");
    let imported = models.join("user.orv");
    std::fs::write(
        dir.join("orv.toml"),
        r#"[project]
name = "workspace-diagnostic"
entry = "src/main.orv"
"#,
    )
    .expect("write manifest");
    std::fs::write(&entry, "import models.user.User\nlet ok: int = 1\n").expect("write entry");
    std::fs::write(
        &imported,
        "pub struct User { id: int }\nlet bad: int = \"wrong\"\n",
    )
    .expect("write imported");
    let canonical_imported = std::fs::canonicalize(&imported).expect("canonical imported");
    let mut session = LspSession::default();

    let initialize = session.jsonrpc_response(&serde_json::json!({
        "jsonrpc": "2.0",
        "id": 22,
        "method": "initialize",
        "params": {
            "rootUri": format!("file://{}", dir.display()),
        },
    }));
    let response = session.jsonrpc_response(&serde_json::json!({
        "jsonrpc": "2.0",
        "id": 23,
        "method": "workspace/diagnostic",
        "params": {
            "previousResultIds": [],
        },
    }));

    assert!(initialize.get("error").is_none(), "{initialize}");
    assert_eq!(response["id"], 23);
    assert!(response.get("error").is_none(), "{response}");
    let items = response["result"]["items"]
        .as_array()
        .expect("workspace diagnostic items");
    let imported_report = items
        .iter()
        .find(|item| item["uri"] == format!("file://{}", canonical_imported.display()))
        .expect("imported diagnostic report");
    let diagnostics = imported_report["items"]
        .as_array()
        .expect("imported diagnostics");
    assert!(diagnostics.iter().any(|item| {
        item["message"]
            .as_str()
            .is_some_and(|message| message.contains("type mismatch"))
    }));
    let _ = std::fs::remove_dir_all(dir);
}
