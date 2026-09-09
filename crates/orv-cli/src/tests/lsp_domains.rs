use super::*;

#[test]
fn lsp_prepare_rename_rejects_domain_field_names() {
    let dir = temp_output_dir("lsp-prepare-rename-domain-field");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let source = dir.join("app.orv");
    let source_text = r#"@server {
  @route POST /checkout {
    let sku = @body.sku
  }
}
"#;
    std::fs::write(&source, source_text).expect("write source");
    let body_line = source_text.lines().nth(2).expect("body line");
    let character = body_line.rfind("sku").expect("body field");
    let response = lsp_jsonrpc_response(&serde_json::json!({
        "jsonrpc": "2.0",
        "id": 32,
        "method": "textDocument/prepareRename",
        "params": {
            "textDocument": {
                "uri": format!("file://{}", source.display()),
            },
            "position": {
                "line": 2,
                "character": character,
            },
        },
    }));

    assert_eq!(response["id"], 32);
    assert!(response.get("error").is_none(), "{response}");
    assert!(response["result"].is_null());
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn lsp_document_highlight_returns_domain_field_occurrences() {
    let dir = temp_output_dir("lsp-document-highlight-domain-field");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let source = dir.join("app.orv");
    let source_text = r#"@server {
  @route POST /checkout {
    let sku = @body.sku
    let label = sku
    let again = @body.sku
  }
}
"#;
    std::fs::write(&source, source_text).expect("write source");
    let first_body_line = source_text.lines().nth(2).expect("first body line");
    let second_body_line = source_text.lines().nth(4).expect("second body line");
    let first_character = first_body_line.rfind("sku").expect("first body field");
    let second_character = second_body_line.rfind("sku").expect("second body field");

    let response = lsp_jsonrpc_response(&serde_json::json!({
        "jsonrpc": "2.0",
        "id": 31,
        "method": "textDocument/documentHighlight",
        "params": {
            "textDocument": {
                "uri": format!("file://{}", source.display()),
            },
            "position": {
                "line": 2,
                "character": first_character,
            },
        },
    }));

    assert_eq!(response["id"], 31);
    assert!(response.get("error").is_none(), "{response}");
    let highlights = response["result"].as_array().expect("highlights");
    assert_eq!(highlights.len(), 2);
    assert!(highlights.iter().any(|highlight| {
        highlight["range"]["start"]["line"] == 2
            && highlight["range"]["start"]["character"] == first_character
    }));
    assert!(highlights.iter().any(|highlight| {
        highlight["range"]["start"]["line"] == 4
            && highlight["range"]["start"]["character"] == second_character
    }));
    assert!(highlights.iter().all(|highlight| highlight["kind"] == 1));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn lsp_references_returns_domain_field_locations() {
    let dir = temp_output_dir("lsp-references-domain-field");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let source = dir.join("app.orv");
    let source_text = r#"@server {
  @route POST /checkout {
    let sku = @body.sku
    let label = sku
    let again = @body.sku
  }
}
"#;
    std::fs::write(&source, source_text).expect("write source");
    let first_body_line = source_text.lines().nth(2).expect("first body line");
    let second_body_line = source_text.lines().nth(4).expect("second body line");
    let first_character = first_body_line.rfind("sku").expect("first body field");
    let second_character = second_body_line.rfind("sku").expect("second body field");
    let response = lsp_jsonrpc_response(&serde_json::json!({
        "jsonrpc": "2.0",
        "id": 21,
        "method": "textDocument/references",
        "params": {
            "textDocument": {
                "uri": format!("file://{}", source.display()),
            },
            "position": {
                "line": 2,
                "character": first_character,
            },
        },
    }));

    assert_eq!(response["id"], 21);
    assert!(response.get("error").is_none(), "{response}");
    let locations = response["result"].as_array().expect("reference locations");
    assert_eq!(locations.len(), 2);
    assert!(locations.iter().any(|location| {
        location["range"]["start"]["line"] == 2
            && location["range"]["start"]["character"] == first_character
    }));
    assert!(locations.iter().any(|location| {
        location["range"]["start"]["line"] == 4
            && location["range"]["start"]["character"] == second_character
    }));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn lsp_completion_returns_domain_field_names_after_dot() {
    let dir = temp_output_dir("lsp-completion-domain-fields");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let source = dir.join("app.orv");
    std::fs::write(
        &source,
        r#"@server {
  let db = @db.connect(@env.SHOP_DATABASE_URL ?? "sqlite://data/shop.sqlite")
  @route POST /checkout {
    let sku = @body.sku
    let quantity = @body.quantity
    let id = @param.orderId
    let page = @query.page
    let next = @body.
  }
}
"#,
    )
    .expect("write source");
    let response = lsp_jsonrpc_response(&serde_json::json!({
        "jsonrpc": "2.0",
        "id": 21,
        "method": "textDocument/completion",
        "params": {
            "textDocument": {
                "uri": format!("file://{}", source.display()),
            },
            "position": {
                "line": 7,
                "character": 21,
            },
        },
    }));

    assert_eq!(response["id"], 21);
    assert!(response.get("error").is_none(), "{response}");
    let items = response["result"]["items"]
        .as_array()
        .expect("completion items");
    assert!(items
        .iter()
        .any(|item| item["label"] == "sku" && item["kind"] == 10));
    assert!(items
        .iter()
        .any(|item| item["label"] == "quantity" && item["kind"] == 10));
    assert!(!items.iter().any(|item| item["label"] == "@route"));
    let _ = std::fs::remove_dir_all(dir);
}
