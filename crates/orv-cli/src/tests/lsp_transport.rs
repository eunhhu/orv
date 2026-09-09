use super::*;

#[test]
fn lsp_serve_stdio_subcommand_is_accepted() {
    let parsed = Cli::try_parse_from(["orv", "lsp", "serve", "--stdio"]);
    if let Err(err) = parsed {
        panic!("{}", err.render());
    }
}

#[test]
fn lsp_stdio_serves_content_length_initialize_frame() {
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 10,
        "method": "initialize",
        "params": {},
    })
    .to_string();
    let input = format!("Content-Length: {}\r\n\r\n{}", body.len(), body);

    let output = lsp_stdio_response(&input).expect("stdio response");
    let (_, response_body) = output
        .split_once("\r\n\r\n")
        .expect("content-length response frame");
    let response: serde_json::Value = serde_json::from_str(response_body).expect("response json");

    assert!(output.starts_with("Content-Length: "));
    assert_eq!(response["id"], 10);
    assert_eq!(response["result"]["serverInfo"]["name"], "orv-lsp");
}

#[test]
fn lsp_stdio_ignores_notifications_without_id() {
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "initialized",
        "params": {},
    })
    .to_string();
    let input = format!("Content-Length: {}\r\n\r\n{}", body.len(), body);

    let output = lsp_stdio_response(&input).expect("stdio response");

    assert_eq!(output, "");
}

#[test]
fn lsp_stdio_document_symbol_returns_symbols_for_file_uri() {
    let dir = temp_output_dir("lsp-document-symbol");
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
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 11,
        "method": "textDocument/documentSymbol",
        "params": {
            "textDocument": {
                "uri": format!("file://{}", source.display()),
            },
        },
    })
    .to_string();
    let input = format!("Content-Length: {}\r\n\r\n{}", body.len(), body);

    let output = lsp_stdio_response(&input).expect("stdio response");
    let (_, response_body) = output
        .split_once("\r\n\r\n")
        .expect("content-length response frame");
    let response: serde_json::Value = serde_json::from_str(response_body).expect("response json");
    let symbols = response["result"].as_array().expect("document symbols");

    assert_eq!(response["id"], 11);
    assert!(response.get("error").is_none());
    assert!(symbols
        .iter()
        .any(|symbol| symbol["name"] == "User" && symbol["kind"] == 23));
    assert!(symbols
        .iter()
        .any(|symbol| symbol["name"] == "greet" && symbol["kind"] == 12));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn lsp_stdio_document_symbol_uses_did_open_unsaved_content() {
    let dir = temp_output_dir("lsp-did-open-symbol");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let source = dir.join("unsaved.orv");
    let uri = format!("file://{}", source.display());
    let did_open = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didOpen",
        "params": {
            "textDocument": {
                "uri": uri,
                "languageId": "orv",
                "version": 1,
                "text": "struct Draft { id: int }\n",
            },
        },
    })
    .to_string();
    let document_symbol = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 14,
        "method": "textDocument/documentSymbol",
        "params": {
            "textDocument": {
                "uri": format!("file://{}", source.display()),
            },
        },
    })
    .to_string();
    let input = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        did_open.len(),
        did_open,
        document_symbol.len(),
        document_symbol
    );

    let output = lsp_stdio_response(&input).expect("stdio response");
    let (_, response_body) = output
        .split_once("\r\n\r\n")
        .expect("content-length response frame");
    let response: serde_json::Value = serde_json::from_str(response_body).expect("response json");

    assert_eq!(response["id"], 14);
    assert!(response.get("error").is_none(), "{response}");
    assert!(response["result"]
        .as_array()
        .expect("document symbols")
        .iter()
        .any(|symbol| symbol["name"] == "Draft"));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn lsp_stdio_document_symbol_uses_did_change_unsaved_content() {
    let dir = temp_output_dir("lsp-did-change-symbol");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let source = dir.join("unsaved.orv");
    let uri = format!("file://{}", source.display());
    let did_open = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didOpen",
        "params": {
            "textDocument": {
                "uri": uri,
                "languageId": "orv",
                "version": 1,
                "text": "struct Draft { id: int }\n",
            },
        },
    })
    .to_string();
    let did_change = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didChange",
        "params": {
            "textDocument": {
                "uri": format!("file://{}", source.display()),
                "version": 2,
            },
            "contentChanges": [
                { "text": "struct Changed { id: int }\n" }
            ],
        },
    })
    .to_string();
    let document_symbol = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 15,
        "method": "textDocument/documentSymbol",
        "params": {
            "textDocument": {
                "uri": format!("file://{}", source.display()),
            },
        },
    })
    .to_string();
    let input = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        did_open.len(),
        did_open,
        did_change.len(),
        did_change,
        document_symbol.len(),
        document_symbol
    );

    let output = lsp_stdio_response(&input).expect("stdio response");
    let (_, response_body) = output
        .split_once("\r\n\r\n")
        .expect("content-length response frame");
    let response: serde_json::Value = serde_json::from_str(response_body).expect("response json");
    let symbols = response["result"].as_array().expect("document symbols");

    assert_eq!(response["id"], 15);
    assert!(response.get("error").is_none(), "{response}");
    assert!(symbols.iter().any(|symbol| symbol["name"] == "Changed"));
    assert!(!symbols.iter().any(|symbol| symbol["name"] == "Draft"));
    let _ = std::fs::remove_dir_all(dir);
}
