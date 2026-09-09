use super::*;

#[test]
fn lsp_signature_help_returns_function_parameters() {
    let dir = temp_output_dir("lsp-signature-help");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let source = dir.join("app.orv");
    let source_text =
        "function add(left: int, right: int): int -> left + right\nlet total: int = add(1, 2)\n";
    std::fs::write(&source, source_text).expect("write source");
    let call_line = source_text.lines().nth(1).expect("call line");
    let character = call_line.find('2').expect("second argument");
    let response = lsp_jsonrpc_response(&serde_json::json!({
        "jsonrpc": "2.0",
        "id": 18,
        "method": "textDocument/signatureHelp",
        "params": {
            "textDocument": {
                "uri": format!("file://{}", source.display()),
            },
            "position": {
                "line": 1,
                "character": character,
            },
        },
    }));

    assert_eq!(response["id"], 18);
    assert!(response.get("error").is_none(), "{response}");
    assert_eq!(response["result"]["activeSignature"], 0);
    assert_eq!(response["result"]["activeParameter"], 1);
    let signature = &response["result"]["signatures"][0];
    assert_eq!(signature["label"], "add(left: int, right: int): int");
    assert_eq!(signature["parameters"][0]["label"], "left: int");
    assert_eq!(signature["parameters"][1]["label"], "right: int");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn lsp_inlay_hint_returns_function_parameter_labels() {
    let dir = temp_output_dir("lsp-inlay-hint");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let source = dir.join("app.orv");
    let source_text =
        "function add(left: int, right: int): int -> left + right\nlet total: int = add(1, 2)\n";
    std::fs::write(&source, source_text).expect("write source");
    let call_line = source_text.lines().nth(1).expect("call line");
    let first_arg = call_line.find('1').expect("first argument");
    let second_arg = call_line.find('2').expect("second argument");
    let response = lsp_jsonrpc_response(&serde_json::json!({
        "jsonrpc": "2.0",
        "id": 19,
        "method": "textDocument/inlayHint",
        "params": {
            "textDocument": {
                "uri": format!("file://{}", source.display()),
            },
            "range": {
                "start": { "line": 0, "character": 0 },
                "end": { "line": 1, "character": call_line.len() },
            },
        },
    }));

    assert_eq!(response["id"], 19);
    assert!(response.get("error").is_none(), "{response}");
    let hints = response["result"].as_array().expect("inlay hints");
    assert!(hints.iter().any(|hint| {
        hint["label"] == "left:"
            && hint["kind"] == 2
            && hint["position"]["line"] == 1
            && hint["position"]["character"] == first_arg
    }));
    assert!(hints.iter().any(|hint| {
        hint["label"] == "right:"
            && hint["kind"] == 2
            && hint["position"]["line"] == 1
            && hint["position"]["character"] == second_arg
    }));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn lsp_completion_returns_project_symbols() {
    let dir = temp_output_dir("lsp-completion");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let source = dir.join("app.orv");
    std::fs::write(
        &source,
        r#"struct User {
  id: int
}

function greet(user: User): string -> "hello"

@server {
  @route GET /ping {
    @respond 200 "ok"
  }
}
"#,
    )
    .expect("write source");
    let response = lsp_jsonrpc_response(&serde_json::json!({
        "jsonrpc": "2.0",
        "id": 18,
        "method": "textDocument/completion",
        "params": {
            "textDocument": {
                "uri": format!("file://{}", source.display()),
            },
            "position": {
                "line": 5,
                "character": 0,
            },
        },
    }));

    assert_eq!(response["id"], 18);
    assert!(response.get("error").is_none(), "{response}");
    assert_eq!(response["result"]["isIncomplete"], false);
    let items = response["result"]["items"]
        .as_array()
        .expect("completion items");
    assert!(items
        .iter()
        .any(|item| item["label"] == "User" && item["kind"] == 22));
    assert!(items
        .iter()
        .any(|item| item["label"] == "greet" && item["kind"] == 3));
    assert!(items
        .iter()
        .any(|item| item["label"] == "route" && item["kind"] == 23));
    assert!(items
        .iter()
        .any(|item| item["label"] == "function" && item["kind"] == 15));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn lsp_completion_returns_directive_snippets_at_at_prefix() {
    let dir = temp_output_dir("lsp-completion-directives");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let source = dir.join("app.orv");
    std::fs::write(
        &source,
        r#"@server {
  @
}
"#,
    )
    .expect("write source");
    let response = lsp_jsonrpc_response(&serde_json::json!({
        "jsonrpc": "2.0",
        "id": 19,
        "method": "textDocument/completion",
        "params": {
            "textDocument": {
                "uri": format!("file://{}", source.display()),
            },
            "position": {
                "line": 1,
                "character": 3,
            },
        },
    }));

    assert_eq!(response["id"], 19);
    assert!(response.get("error").is_none(), "{response}");
    let items = response["result"]["items"]
        .as_array()
        .expect("completion items");
    let route = items
        .iter()
        .find(|item| item["label"] == "@route")
        .expect("@route completion");
    assert_eq!(route["kind"], 15);
    assert_eq!(route["insertTextFormat"], 2);
    assert_eq!(route["insertText"], "@route ${1:GET} ${2:/path} {\n  $0\n}");
    assert!(items
        .iter()
        .any(|item| item["label"] == "@payment.connect" && item["kind"] == 15));
    assert!(items
        .iter()
        .any(|item| item["label"] == "@shipping.connect" && item["kind"] == 15));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn lsp_completion_returns_route_methods_inside_route_head() {
    let dir = temp_output_dir("lsp-completion-route-methods");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let source = dir.join("app.orv");
    std::fs::write(&source, "@server {\n  @route \n}\n").expect("write source");
    let response = lsp_jsonrpc_response(&serde_json::json!({
        "jsonrpc": "2.0",
        "id": 20,
        "method": "textDocument/completion",
        "params": {
            "textDocument": {
                "uri": format!("file://{}", source.display()),
            },
            "position": {
                "line": 1,
                "character": 9,
            },
        },
    }));

    assert_eq!(response["id"], 20);
    assert!(response.get("error").is_none(), "{response}");
    let items = response["result"]["items"]
        .as_array()
        .expect("completion items");
    assert!(items
        .iter()
        .any(|item| item["label"] == "GET" && item["kind"] == 14));
    assert!(items
        .iter()
        .any(|item| item["label"] == "POST" && item["kind"] == 14));
    assert!(!items.iter().any(|item| item["label"] == "@route"));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn lsp_completion_returns_route_path_params_after_param_dot() {
    let dir = temp_output_dir("lsp-completion-route-param-fields");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let source = dir.join("app.orv");
    std::fs::write(
        &source,
        r#"@server {
  @route GET /orders/:orderId/items/:itemId {
    let current = @param.
  }
}
"#,
    )
    .expect("write source");
    let response = lsp_jsonrpc_response(&serde_json::json!({
        "jsonrpc": "2.0",
        "id": 22,
        "method": "textDocument/completion",
        "params": {
            "textDocument": {
                "uri": format!("file://{}", source.display()),
            },
            "position": {
                "line": 2,
                "character": 25,
            },
        },
    }));

    assert_eq!(response["id"], 22);
    assert!(response.get("error").is_none(), "{response}");
    let items = response["result"]["items"]
        .as_array()
        .expect("completion items");
    assert!(items
        .iter()
        .any(|item| item["label"] == "orderId" && item["kind"] == 10));
    assert!(items
        .iter()
        .any(|item| item["label"] == "itemId" && item["kind"] == 10));
    assert!(!items.iter().any(|item| item["label"] == "@param"));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn lsp_completion_returns_env_names_after_dot() {
    let dir = temp_output_dir("lsp-completion-env-fields");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let source = dir.join("app.orv");
    std::fs::write(
        &source,
        r#"@server {
  let db = @db.connect(@env.SHOP_DATABASE_URL ?? "sqlite://data/shop.sqlite")
  let payments = @payment.connect(@env.PAYMENT_ADAPTER_URL ?? "file://data/payments.jsonl")
  let current = @env.
}
"#,
    )
    .expect("write source");
    let response = lsp_jsonrpc_response(&serde_json::json!({
        "jsonrpc": "2.0",
        "id": 22,
        "method": "textDocument/completion",
        "params": {
            "textDocument": {
                "uri": format!("file://{}", source.display()),
            },
            "position": {
                "line": 3,
                "character": 21,
            },
        },
    }));

    assert_eq!(response["id"], 22);
    assert!(response.get("error").is_none(), "{response}");
    let items = response["result"]["items"]
        .as_array()
        .expect("completion items");
    assert!(items
        .iter()
        .any(|item| item["label"] == "SHOP_DATABASE_URL" && item["kind"] == 21));
    assert!(items
        .iter()
        .any(|item| item["label"] == "PAYMENT_ADAPTER_URL" && item["kind"] == 21));
    assert!(!items.iter().any(|item| item["label"] == "@env"));
    let _ = std::fs::remove_dir_all(dir);
}
