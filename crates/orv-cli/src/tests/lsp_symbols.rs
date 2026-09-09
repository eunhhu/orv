use super::*;

#[test]
fn lsp_document_symbol_accepts_percent_encoded_file_uri() {
    let dir = temp_output_dir("lsp-document-symbol-space");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let source = dir.join("app space.orv");
    std::fs::write(&source, "struct User { id: int }\n").expect("write source");
    let uri = format!("file://{}", source.display()).replace(' ', "%20");

    let response = lsp_jsonrpc_response(&serde_json::json!({
        "jsonrpc": "2.0",
        "id": 12,
        "method": "textDocument/documentSymbol",
        "params": {
            "textDocument": {
                "uri": uri,
            },
        },
    }));

    assert!(response.get("error").is_none(), "{response}");
    assert!(response["result"]
        .as_array()
        .expect("document symbols")
        .iter()
        .any(|symbol| symbol["name"] == "User"));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn lsp_semantic_tokens_returns_project_graph_declaration_tokens() {
    let dir = temp_output_dir("lsp-semantic-tokens");
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
        "id": 30,
        "method": "textDocument/semanticTokens/full",
        "params": {
            "textDocument": {
                "uri": format!("file://{}", source.display()),
            },
        },
    }));

    assert_eq!(response["id"], 30);
    assert!(response.get("error").is_none(), "{response}");
    let data = response["result"]["data"]
        .as_array()
        .expect("semantic token data");
    assert_eq!(data.len() % 5, 0);
    let tokens: Vec<Vec<u64>> = data
        .chunks(5)
        .map(|chunk| {
            chunk
                .iter()
                .map(|value| value.as_u64().expect("semantic token integer"))
                .collect()
        })
        .collect();
    assert!(tokens
        .iter()
        .any(|token| token.as_slice() == [0, 7, 4, 1, 1]));
    assert!(tokens
        .iter()
        .any(|token| token.as_slice() == [2, 9, 5, 2, 1]));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn lsp_definition_returns_symbol_declaration_location() {
    let dir = temp_output_dir("lsp-definition");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let source = dir.join("app.orv");
    std::fs::write(
        &source,
        r"struct User {
  id: int
}

let u: User = { id: 1 }
",
    )
    .expect("write source");
    let response = lsp_jsonrpc_response(&serde_json::json!({
        "jsonrpc": "2.0",
        "id": 16,
        "method": "textDocument/definition",
        "params": {
            "textDocument": {
                "uri": format!("file://{}", source.display()),
            },
            "position": {
                "line": 4,
                "character": 8,
            },
        },
    }));

    assert_eq!(response["id"], 16);
    assert!(response.get("error").is_none(), "{response}");
    let canonical_source = std::fs::canonicalize(&source).expect("canonical source");
    assert_eq!(
        response["result"]["uri"],
        format!("file://{}", canonical_source.display())
    );
    assert_eq!(response["result"]["range"]["start"]["line"], 0);
    assert_eq!(response["result"]["range"]["start"]["character"], 0);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn lsp_declaration_returns_symbol_declaration_location() {
    let dir = temp_output_dir("lsp-declaration");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let source = dir.join("app.orv");
    let source_text =
        "function greet(name: string): string -> name\nlet message: string = greet(\"Ada\")\n";
    std::fs::write(&source, source_text).expect("write source");
    let call_line = source_text.lines().nth(1).expect("call line");
    let call_character = call_line.find("greet").expect("call name");
    let response = lsp_jsonrpc_response(&serde_json::json!({
        "jsonrpc": "2.0",
        "id": 20,
        "method": "textDocument/declaration",
        "params": {
            "textDocument": {
                "uri": format!("file://{}", source.display()),
            },
            "position": {
                "line": 1,
                "character": call_character,
            },
        },
    }));

    assert_eq!(response["id"], 20);
    assert!(response.get("error").is_none(), "{response}");
    let canonical_source = std::fs::canonicalize(&source).expect("canonical source");
    assert_eq!(
        response["result"]["uri"],
        format!("file://{}", canonical_source.display())
    );
    assert_eq!(response["result"]["range"]["start"]["line"], 0);
    assert_eq!(response["result"]["range"]["start"]["character"], 0);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn lsp_implementation_returns_concrete_symbol_location() {
    let dir = temp_output_dir("lsp-implementation");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let source = dir.join("app.orv");
    let source_text =
        "function greet(name: string): string -> name\nlet message: string = greet(\"Ada\")\n";
    std::fs::write(&source, source_text).expect("write source");
    let call_line = source_text.lines().nth(1).expect("call line");
    let call_character = call_line.find("greet").expect("call name");
    let response = lsp_jsonrpc_response(&serde_json::json!({
        "jsonrpc": "2.0",
        "id": 27,
        "method": "textDocument/implementation",
        "params": {
            "textDocument": {
                "uri": format!("file://{}", source.display()),
            },
            "position": {
                "line": 1,
                "character": call_character,
            },
        },
    }));

    assert_eq!(response["id"], 27);
    assert!(response.get("error").is_none(), "{response}");
    let canonical_source = std::fs::canonicalize(&source).expect("canonical source");
    assert_eq!(
        response["result"]["uri"],
        format!("file://{}", canonical_source.display())
    );
    assert_eq!(response["result"]["range"]["start"]["line"], 0);
    assert_eq!(response["result"]["range"]["start"]["character"], 0);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn lsp_moniker_returns_project_symbol_identifier() {
    let dir = temp_output_dir("lsp-moniker");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let source = dir.join("app.orv");
    let source_text =
        "struct User {\n  id: int\n}\n\nfunction greet(user: User): string -> \"hello\"\n";
    std::fs::write(&source, source_text).expect("write source");
    let function_line = source_text.lines().nth(4).expect("function line");
    let function_character = function_line.find("greet").expect("function name");
    let response = lsp_jsonrpc_response(&serde_json::json!({
        "jsonrpc": "2.0",
        "id": 31,
        "method": "textDocument/moniker",
        "params": {
            "textDocument": {
                "uri": format!("file://{}", source.display()),
            },
            "position": {
                "line": 4,
                "character": function_character,
            },
        },
    }));

    assert_eq!(response["id"], 31);
    assert!(response.get("error").is_none(), "{response}");
    let monikers = response["result"].as_array().expect("monikers");
    assert_eq!(monikers.len(), 1);
    assert_eq!(monikers[0]["scheme"], "orv");
    assert_eq!(monikers[0]["identifier"], "function:greet");
    assert_eq!(monikers[0]["unique"], "project");
    assert_eq!(monikers[0]["kind"], "export");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn lsp_hover_returns_symbol_summary() {
    let dir = temp_output_dir("lsp-hover");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let source = dir.join("app.orv");
    std::fs::write(
        &source,
        r"struct User {
  id: int
}

let u: User = { id: 1 }
",
    )
    .expect("write source");
    let response = lsp_jsonrpc_response(&serde_json::json!({
        "jsonrpc": "2.0",
        "id": 17,
        "method": "textDocument/hover",
        "params": {
            "textDocument": {
                "uri": format!("file://{}", source.display()),
            },
            "position": {
                "line": 4,
                "character": 8,
            },
        },
    }));

    assert_eq!(response["id"], 17);
    assert!(response.get("error").is_none(), "{response}");
    assert_eq!(response["result"]["contents"]["kind"], "markdown");
    assert_eq!(response["result"]["contents"]["value"], "**Struct** `User`");
    assert_eq!(response["result"]["range"]["start"]["line"], 0);
    assert_eq!(response["result"]["range"]["start"]["character"], 0);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn lsp_workspace_symbol_returns_matching_project_symbols() {
    let dir = temp_output_dir("lsp-workspace-symbol");
    let src = dir.join("src");
    let models = src.join("models");
    std::fs::create_dir_all(&models).expect("create models dir");
    let entry = src.join("main.orv");
    let imported = models.join("user.orv");
    std::fs::write(
        dir.join("orv.toml"),
        r#"[project]
name = "workspace-symbol"
entry = "src/main.orv"
"#,
    )
    .expect("write manifest");
    std::fs::write(
        &entry,
        "import models.user.User\nfunction checkout(user: User): string -> \"ok\"\n",
    )
    .expect("write entry");
    std::fs::write(&imported, "pub struct User { id: int }\n").expect("write imported");
    let canonical_imported = std::fs::canonicalize(&imported).expect("canonical imported");
    let mut session = LspSession::default();

    let initialize = session.jsonrpc_response(&serde_json::json!({
        "jsonrpc": "2.0",
        "id": 20,
        "method": "initialize",
        "params": {
            "rootUri": format!("file://{}", dir.display()),
        },
    }));
    let response = session.jsonrpc_response(&serde_json::json!({
        "jsonrpc": "2.0",
        "id": 21,
        "method": "workspace/symbol",
        "params": {
            "query": "User",
        },
    }));

    assert!(initialize.get("error").is_none(), "{initialize}");
    assert_eq!(response["id"], 21);
    assert!(response.get("error").is_none(), "{response}");
    let symbols = response["result"].as_array().expect("workspace symbols");
    let user = symbols
        .iter()
        .find(|symbol| symbol["name"] == "User")
        .expect("User workspace symbol");
    assert_eq!(user["kind"], 23);
    assert_eq!(
        user["location"]["uri"],
        format!("file://{}", canonical_imported.display())
    );
    assert!(symbols.iter().all(|symbol| symbol["name"]
        .as_str()
        .is_some_and(|name| name.contains("User"))));
    let _ = std::fs::remove_dir_all(dir);
}
