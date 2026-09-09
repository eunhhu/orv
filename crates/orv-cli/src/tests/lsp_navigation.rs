use super::*;

#[test]
fn lsp_snapshot_subcommand_is_accepted() {
    let parsed = Cli::try_parse_from(["orv", "lsp", "snapshot", "fixtures/e2e/hello.orv"]);
    if let Err(err) = parsed {
        panic!("{}", err.render());
    }
}

#[test]
fn lsp_reveal_subcommand_is_accepted() {
    let parsed = Cli::try_parse_from([
        "orv",
        "lsp",
        "reveal",
        "target/orv-build-test",
        "route:GET_/ping:abc123",
    ]);
    if let Err(err) = parsed {
        panic!("{}", err.render());
    }
}

#[test]
fn lsp_initialize_returns_server_capabilities() {
    let response = lsp_jsonrpc_response(&serde_json::json!({
        "jsonrpc": "2.0",
        "id": 7,
        "method": "initialize",
        "params": {},
    }));

    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 7);
    assert_eq!(response["result"]["serverInfo"]["name"], "orv-lsp");
    let capabilities = &response["result"]["capabilities"];
    assert_eq!(capabilities["textDocumentSync"]["openClose"], true);
    assert_eq!(capabilities["textDocumentSync"]["change"], 1);
    assert_eq!(
        capabilities["textDocumentSync"]["save"]["includeText"],
        true
    );
    for provider in [
        "documentSymbolProvider",
        "foldingRangeProvider",
        "selectionRangeProvider",
        "definitionProvider",
        "declarationProvider",
        "typeDefinitionProvider",
        "implementationProvider",
        "typeHierarchyProvider",
        "callHierarchyProvider",
        "monikerProvider",
        "colorProvider",
        "linkedEditingRangeProvider",
        "referencesProvider",
        "documentHighlightProvider",
        "workspaceSymbolProvider",
        "hoverProvider",
        "inlayHintProvider",
        "documentFormattingProvider",
        "documentRangeFormattingProvider",
    ] {
        assert_eq!(capabilities[provider], true, "{provider}");
    }
    assert_eq!(
        capabilities["documentOnTypeFormattingProvider"]["firstTriggerCharacter"],
        "}"
    );
    assert!(
        capabilities["documentOnTypeFormattingProvider"]["moreTriggerCharacter"]
            .as_array()
            .expect("on type trigger characters")
            .iter()
            .any(|trigger| trigger == "\n")
    );
    assert_eq!(
        capabilities["documentLinkProvider"]["resolveProvider"],
        false
    );
    assert_eq!(capabilities["semanticTokensProvider"]["full"], true);
    assert_eq!(
        capabilities["semanticTokensProvider"]["legend"]["tokenTypes"][1],
        "type"
    );
    assert_eq!(capabilities["codeLensProvider"]["resolveProvider"], false);
    assert_eq!(
        capabilities["codeActionProvider"]["codeActionKinds"][0],
        "quickfix"
    );
    assert_eq!(
        capabilities["executeCommandProvider"]["commands"][0],
        "orv.revealSourceNode"
    );
    assert_eq!(capabilities["renameProvider"]["prepareProvider"], true);
    assert_eq!(
        capabilities["completionProvider"]["triggerCharacters"][0],
        "@"
    );
    assert_eq!(
        capabilities["signatureHelpProvider"]["triggerCharacters"][0],
        "("
    );
    assert_eq!(
        capabilities["diagnosticProvider"]["workspaceDiagnostics"],
        true
    );
}

#[test]
fn lsp_shutdown_returns_null_result() {
    let response = lsp_jsonrpc_response(&serde_json::json!({
        "jsonrpc": "2.0",
        "id": 8,
        "method": "shutdown",
    }));

    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 8);
    assert!(response.get("error").is_none());
    assert!(response
        .get("result")
        .is_some_and(serde_json::Value::is_null));
}

#[test]
fn lsp_unknown_method_returns_method_not_found_with_method_name() {
    let response = lsp_jsonrpc_response(&serde_json::json!({
        "jsonrpc": "2.0",
        "id": "request-9",
        "method": "workspace/configuration",
    }));

    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], "request-9");
    assert_eq!(response["error"]["code"], -32601);
    assert_eq!(
        response["error"]["data"]["method"],
        "workspace/configuration"
    );
}

#[test]
fn lsp_prepare_rename_returns_identifier_range_and_placeholder() {
    let dir = temp_output_dir("lsp-prepare-rename");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let source = dir.join("app.orv");
    std::fs::write(&source, "struct User { id: int }\n").expect("write source");

    let response = lsp_jsonrpc_response(&serde_json::json!({
        "jsonrpc": "2.0",
        "id": 27,
        "method": "textDocument/prepareRename",
        "params": {
            "textDocument": {
                "uri": format!("file://{}", source.display()),
            },
            "position": {
                "line": 0,
                "character": 8,
            },
        },
    }));

    assert_eq!(response["id"], 27);
    assert!(response.get("error").is_none(), "{response}");
    assert_eq!(response["result"]["placeholder"], "User");
    assert_eq!(response["result"]["range"]["start"]["line"], 0);
    assert_eq!(response["result"]["range"]["start"]["character"], 7);
    assert_eq!(response["result"]["range"]["end"]["character"], 11);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn lsp_prepare_rename_rejects_language_tokens_and_builtin_directives() {
    let dir = temp_output_dir("lsp-prepare-rename-language-token");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let source = dir.join("app.orv");
    std::fs::write(
            &source,
            "struct User { id: int }\n@server {\n  @route GET /ping {\n    @respond 200 \"ok\"\n  }\n}\n",
        )
        .expect("write source");

    let keyword_response = lsp_jsonrpc_response(&serde_json::json!({
        "jsonrpc": "2.0",
        "id": 29,
        "method": "textDocument/prepareRename",
        "params": {
            "textDocument": {
                "uri": format!("file://{}", source.display()),
            },
            "position": {
                "line": 0,
                "character": 1,
            },
        },
    }));
    let route_response = lsp_jsonrpc_response(&serde_json::json!({
        "jsonrpc": "2.0",
        "id": 30,
        "method": "textDocument/prepareRename",
        "params": {
            "textDocument": {
                "uri": format!("file://{}", source.display()),
            },
            "position": {
                "line": 2,
                "character": 4,
            },
        },
    }));

    assert_eq!(keyword_response["id"], 29);
    assert!(
        keyword_response.get("error").is_none(),
        "{keyword_response}"
    );
    assert!(keyword_response["result"].is_null());
    assert_eq!(route_response["id"], 30);
    assert!(route_response.get("error").is_none(), "{route_response}");
    assert!(route_response["result"].is_null());
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn lsp_rename_returns_workspace_edit_for_project_references() {
    let dir = temp_output_dir("lsp-rename");
    let models = dir.join("models");
    std::fs::create_dir_all(&models).expect("create models dir");
    let source = dir.join("app.orv");
    let imported = models.join("user.orv");
    std::fs::write(
        &source,
        "import models.user.User\nlet u: User = { id: 1 }\n",
    )
    .expect("write source");
    std::fs::write(&imported, "pub struct User { id: int }\n").expect("write imported");
    let canonical_source = std::fs::canonicalize(&source).expect("canonical source");
    let canonical_imported = std::fs::canonicalize(&imported).expect("canonical imported");

    let response = lsp_jsonrpc_response(&serde_json::json!({
        "jsonrpc": "2.0",
        "id": 28,
        "method": "textDocument/rename",
        "params": {
            "textDocument": {
                "uri": format!("file://{}", source.display()),
            },
            "position": {
                "line": 1,
                "character": 8,
            },
            "newName": "Account",
        },
    }));

    assert_eq!(response["id"], 28);
    assert!(response.get("error").is_none(), "{response}");
    let changes = response["result"]["changes"].as_object().expect("changes");
    let source_uri = format!("file://{}", canonical_source.display());
    let imported_uri = format!("file://{}", canonical_imported.display());
    let source_edits = changes
        .get(&source_uri)
        .and_then(serde_json::Value::as_array)
        .expect("source edits");
    let imported_edits = changes
        .get(&imported_uri)
        .and_then(serde_json::Value::as_array)
        .expect("imported edits");
    assert!(
        source_edits
            .iter()
            .filter(|edit| edit["newText"] == "Account")
            .count()
            >= 2
    );
    assert!(imported_edits
        .iter()
        .any(|edit| edit["newText"] == "Account"));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn lsp_rename_rejects_keyword_new_name() {
    let dir = temp_output_dir("lsp-rename-keyword-new-name");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let source = dir.join("app.orv");
    std::fs::write(&source, "struct User { id: int }\n").expect("write source");

    let response = lsp_jsonrpc_response(&serde_json::json!({
        "jsonrpc": "2.0",
        "id": 31,
        "method": "textDocument/rename",
        "params": {
            "textDocument": {
                "uri": format!("file://{}", source.display()),
            },
            "position": {
                "line": 0,
                "character": 8,
            },
            "newName": "struct",
        },
    }));

    assert_eq!(response["id"], 31);
    assert_eq!(response["error"]["code"], -32602);
    assert!(response["error"]["message"]
        .as_str()
        .is_some_and(|message| message.contains("non-keyword identifier")));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn lsp_document_highlight_returns_current_file_identifier_occurrences() {
    let dir = temp_output_dir("lsp-document-highlight");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let source = dir.join("app.orv");
    std::fs::write(
        &source,
        r"struct User { id: int }

let u: User = { id: 1 }
let v: User = u
",
    )
    .expect("write source");

    let response = lsp_jsonrpc_response(&serde_json::json!({
        "jsonrpc": "2.0",
        "id": 29,
        "method": "textDocument/documentHighlight",
        "params": {
            "textDocument": {
                "uri": format!("file://{}", source.display()),
            },
            "position": {
                "line": 2,
                "character": 8,
            },
        },
    }));

    assert_eq!(response["id"], 29);
    assert!(response.get("error").is_none(), "{response}");
    let highlights = response["result"].as_array().expect("highlights");
    assert_eq!(highlights.len(), 3);
    assert!(highlights
        .iter()
        .any(|highlight| highlight["range"]["start"]["line"] == 0));
    assert!(highlights
        .iter()
        .any(|highlight| highlight["range"]["start"]["line"] == 2));
    assert!(highlights
        .iter()
        .any(|highlight| highlight["range"]["start"]["line"] == 3));
    assert!(highlights.iter().all(|highlight| highlight["kind"] == 1));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn lsp_document_highlight_ignores_language_keywords() {
    let dir = temp_output_dir("lsp-document-highlight-keyword");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let source = dir.join("app.orv");
    std::fs::write(&source, "let total = 1\nlet next = total + 1\n").expect("write source");

    let response = lsp_jsonrpc_response(&serde_json::json!({
        "jsonrpc": "2.0",
        "id": 30,
        "method": "textDocument/documentHighlight",
        "params": {
            "textDocument": {
                "uri": format!("file://{}", source.display()),
            },
            "position": {
                "line": 0,
                "character": 1,
            },
        },
    }));

    assert_eq!(response["id"], 30);
    assert!(response.get("error").is_none(), "{response}");
    assert!(response["result"]
        .as_array()
        .expect("highlight result")
        .is_empty());
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn lsp_type_definition_returns_type_declaration_location() {
    let dir = temp_output_dir("lsp-type-definition");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let source = dir.join("app.orv");
    let source_text = r"struct User {
  id: int
}

let u: User = { id: 1 }
";
    std::fs::write(&source, source_text).expect("write source");
    let binding_line = source_text.lines().nth(4).expect("binding line");
    let type_character = binding_line.find("User").expect("type name");
    let response = lsp_jsonrpc_response(&serde_json::json!({
        "jsonrpc": "2.0",
        "id": 21,
        "method": "textDocument/typeDefinition",
        "params": {
            "textDocument": {
                "uri": format!("file://{}", source.display()),
            },
            "position": {
                "line": 4,
                "character": type_character,
            },
        },
    }));

    assert_eq!(response["id"], 21);
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
fn lsp_prepare_type_hierarchy_returns_type_item() {
    let dir = temp_output_dir("lsp-type-hierarchy-prepare");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let source = dir.join("app.orv");
    let source_text = "struct User {\n  id: int\n}\n\nlet u: User = { id: 1 }\n";
    std::fs::write(&source, source_text).expect("write source");
    let binding_line = source_text.lines().nth(4).expect("binding line");
    let type_character = binding_line.find("User").expect("type name");
    let response = lsp_jsonrpc_response(&serde_json::json!({
        "jsonrpc": "2.0",
        "id": 28,
        "method": "textDocument/prepareTypeHierarchy",
        "params": {
            "textDocument": {
                "uri": format!("file://{}", source.display()),
            },
            "position": {
                "line": 4,
                "character": type_character,
            },
        },
    }));

    assert_eq!(response["id"], 28);
    assert!(response.get("error").is_none(), "{response}");
    let items = response["result"].as_array().expect("type hierarchy items");
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["name"], "User");
    assert_eq!(items[0]["kind"], 23);
    assert_eq!(items[0]["selectionRange"]["start"]["line"], 0);
    assert_eq!(items[0]["selectionRange"]["start"]["character"], 7);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn lsp_type_hierarchy_supertypes_and_subtypes_are_empty_without_inheritance() {
    let dir = temp_output_dir("lsp-type-hierarchy-empty");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let source = dir.join("app.orv");
    let source_text = "struct User {\n  id: int\n}\n";
    std::fs::write(&source, source_text).expect("write source");
    let canonical_source = std::fs::canonicalize(&source).expect("canonical source");
    let item = serde_json::json!({
        "name": "User",
        "kind": 23,
        "uri": format!("file://{}", canonical_source.display()),
        "range": {
            "start": { "line": 0, "character": 0 },
            "end": { "line": 2, "character": 1 },
        },
        "selectionRange": {
            "start": { "line": 0, "character": 7 },
            "end": { "line": 0, "character": 11 },
        },
    });
    let supertypes = lsp_jsonrpc_response(&serde_json::json!({
        "jsonrpc": "2.0",
        "id": 29,
        "method": "typeHierarchy/supertypes",
        "params": {
            "item": item,
        },
    }));
    let subtypes = lsp_jsonrpc_response(&serde_json::json!({
        "jsonrpc": "2.0",
        "id": 30,
        "method": "typeHierarchy/subtypes",
        "params": {
            "item": item,
        },
    }));

    assert_eq!(supertypes["id"], 29);
    assert!(supertypes.get("error").is_none(), "{supertypes}");
    assert_eq!(
        supertypes["result"].as_array().expect("supertypes").len(),
        0
    );
    assert_eq!(subtypes["id"], 30);
    assert!(subtypes.get("error").is_none(), "{subtypes}");
    assert_eq!(subtypes["result"].as_array().expect("subtypes").len(), 0);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn lsp_prepare_call_hierarchy_returns_function_item() {
    let dir = temp_output_dir("lsp-call-hierarchy-prepare");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let source = dir.join("app.orv");
    let source_text = "function discount(price: int): int -> price\nfunction total(price: int): int -> discount(price)\nlet value: int = total(10)\n";
    std::fs::write(&source, source_text).expect("write source");
    let total_line = source_text.lines().nth(1).expect("total line");
    let total_character = total_line.find("total").expect("total name");
    let response = lsp_jsonrpc_response(&serde_json::json!({
        "jsonrpc": "2.0",
        "id": 24,
        "method": "textDocument/prepareCallHierarchy",
        "params": {
            "textDocument": {
                "uri": format!("file://{}", source.display()),
            },
            "position": {
                "line": 1,
                "character": total_character,
            },
        },
    }));

    assert_eq!(response["id"], 24);
    assert!(response.get("error").is_none(), "{response}");
    let items = response["result"].as_array().expect("call hierarchy items");
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["name"], "total");
    assert_eq!(items[0]["kind"], 12);
    assert_eq!(
        items[0]["uri"],
        format!(
            "file://{}",
            std::fs::canonicalize(&source)
                .expect("canonical source")
                .display()
        )
    );
    assert_eq!(items[0]["selectionRange"]["start"]["line"], 1);
    assert_eq!(
        items[0]["selectionRange"]["start"]["character"],
        total_character
    );
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn lsp_call_hierarchy_outgoing_returns_direct_calls() {
    let dir = temp_output_dir("lsp-call-hierarchy-outgoing");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let source = dir.join("app.orv");
    let source_text = "function discount(price: int): int -> price\nfunction total(price: int): int -> discount(price)\nlet value: int = total(10)\n";
    std::fs::write(&source, source_text).expect("write source");
    let canonical_source = std::fs::canonicalize(&source).expect("canonical source");
    let total_line = source_text.lines().nth(1).expect("total line");
    let call_character = total_line.find("discount").expect("discount call");
    let response = lsp_jsonrpc_response(&serde_json::json!({
        "jsonrpc": "2.0",
        "id": 25,
        "method": "callHierarchy/outgoingCalls",
        "params": {
            "item": {
                "name": "total",
                "kind": 12,
                "uri": format!("file://{}", canonical_source.display()),
                "range": {
                    "start": { "line": 1, "character": 0 },
                    "end": { "line": 1, "character": total_line.len() },
                },
                "selectionRange": {
                    "start": { "line": 1, "character": total_line.find("total").expect("total name") },
                    "end": { "line": 1, "character": total_line.find("total").expect("total name") + "total".len() },
                },
            },
        },
    }));

    assert_eq!(response["id"], 25);
    assert!(response.get("error").is_none(), "{response}");
    let calls = response["result"].as_array().expect("outgoing calls");
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0]["to"]["name"], "discount");
    assert_eq!(calls[0]["to"]["kind"], 12);
    assert_eq!(calls[0]["fromRanges"][0]["start"]["line"], 1);
    assert_eq!(
        calls[0]["fromRanges"][0]["start"]["character"],
        call_character
    );
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn lsp_call_hierarchy_incoming_returns_direct_callers() {
    let dir = temp_output_dir("lsp-call-hierarchy-incoming");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let source = dir.join("app.orv");
    let source_text = "function discount(price: int): int -> price\nfunction total(price: int): int -> discount(price)\nlet value: int = total(10)\n";
    std::fs::write(&source, source_text).expect("write source");
    let canonical_source = std::fs::canonicalize(&source).expect("canonical source");
    let discount_line = source_text.lines().next().expect("discount line");
    let total_line = source_text.lines().nth(1).expect("total line");
    let call_character = total_line.find("discount").expect("discount call");
    let response = lsp_jsonrpc_response(&serde_json::json!({
        "jsonrpc": "2.0",
        "id": 26,
        "method": "callHierarchy/incomingCalls",
        "params": {
            "item": {
                "name": "discount",
                "kind": 12,
                "uri": format!("file://{}", canonical_source.display()),
                "range": {
                    "start": { "line": 0, "character": 0 },
                    "end": { "line": 0, "character": discount_line.len() },
                },
                "selectionRange": {
                    "start": { "line": 0, "character": discount_line.find("discount").expect("discount name") },
                    "end": { "line": 0, "character": discount_line.find("discount").expect("discount name") + "discount".len() },
                },
            },
        },
    }));

    assert_eq!(response["id"], 26);
    assert!(response.get("error").is_none(), "{response}");
    let calls = response["result"].as_array().expect("incoming calls");
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0]["from"]["name"], "total");
    assert_eq!(calls[0]["from"]["kind"], 12);
    assert_eq!(calls[0]["fromRanges"][0]["start"]["line"], 1);
    assert_eq!(
        calls[0]["fromRanges"][0]["start"]["character"],
        call_character
    );
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn lsp_hover_returns_request_body_field_summary() {
    let dir = temp_output_dir("lsp-hover-body-field");
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
        "id": 18,
        "method": "textDocument/hover",
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

    assert_eq!(response["id"], 18);
    assert!(response.get("error").is_none(), "{response}");
    assert_eq!(response["result"]["contents"]["kind"], "markdown");
    assert_eq!(
        response["result"]["contents"]["value"],
        "**Request body field** `sku`"
    );
    assert_eq!(response["result"]["range"]["start"]["line"], 2);
    assert_eq!(response["result"]["range"]["start"]["character"], character);
    assert_eq!(
        response["result"]["range"]["end"]["character"],
        character + "sku".len()
    );
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn lsp_hover_returns_env_value_summary() {
    let dir = temp_output_dir("lsp-hover-env-field");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let source = dir.join("app.orv");
    let source_text = r#"@server {
  let db = @db.connect(@env.SHOP_DATABASE_URL ?? "sqlite://data/shop.sqlite")
}
"#;
    std::fs::write(&source, source_text).expect("write source");
    let env_line = source_text.lines().nth(1).expect("env line");
    let character = env_line.find("SHOP_DATABASE_URL").expect("env field name");
    let response = lsp_jsonrpc_response(&serde_json::json!({
        "jsonrpc": "2.0",
        "id": 19,
        "method": "textDocument/hover",
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

    assert_eq!(response["id"], 19);
    assert!(response.get("error").is_none(), "{response}");
    assert_eq!(response["result"]["contents"]["kind"], "markdown");
    assert_eq!(
        response["result"]["contents"]["value"],
        "**Environment value** `SHOP_DATABASE_URL`"
    );
    assert_eq!(response["result"]["range"]["start"]["line"], 1);
    assert_eq!(response["result"]["range"]["start"]["character"], character);
    assert_eq!(
        response["result"]["range"]["end"]["character"],
        character + "SHOP_DATABASE_URL".len()
    );
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn lsp_references_returns_identifier_locations() {
    let dir = temp_output_dir("lsp-references");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let source = dir.join("app.orv");
    std::fs::write(
        &source,
        r#"struct User {
  id: int
}

function greet(user: User): string -> "hello"

let u: User = { id: 1 }
"#,
    )
    .expect("write source");
    let response = lsp_jsonrpc_response(&serde_json::json!({
        "jsonrpc": "2.0",
        "id": 19,
        "method": "textDocument/references",
        "params": {
            "textDocument": {
                "uri": format!("file://{}", source.display()),
            },
            "position": {
                "line": 6,
                "character": 8,
            },
        },
    }));

    assert_eq!(response["id"], 19);
    assert!(response.get("error").is_none(), "{response}");
    let locations = response["result"].as_array().expect("reference locations");
    assert!(locations.iter().any(|location| {
        location["range"]["start"]["line"] == 0 && location["range"]["start"]["character"] == 7
    }));
    assert!(locations.iter().any(|location| {
        location["range"]["start"]["line"] == 4 && location["range"]["start"]["character"] == 21
    }));
    assert!(locations.iter().any(|location| {
        location["range"]["start"]["line"] == 6 && location["range"]["start"]["character"] == 7
    }));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn lsp_references_ignore_language_keywords() {
    let dir = temp_output_dir("lsp-references-keyword");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let source = dir.join("app.orv");
    std::fs::write(
        &source,
        "struct User { id: int }\nstruct Post { id: int }\n",
    )
    .expect("write source");
    let response = lsp_jsonrpc_response(&serde_json::json!({
        "jsonrpc": "2.0",
        "id": 20,
        "method": "textDocument/references",
        "params": {
            "textDocument": {
                "uri": format!("file://{}", source.display()),
            },
            "position": {
                "line": 0,
                "character": 1,
            },
        },
    }));

    assert_eq!(response["id"], 20);
    assert!(response.get("error").is_none(), "{response}");
    assert!(response["result"]
        .as_array()
        .expect("reference result")
        .is_empty());
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn lsp_did_close_drops_unsaved_content() {
    let dir = temp_output_dir("lsp-did-close-symbol");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let source = dir.join("app.orv");
    std::fs::write(&source, "struct Disk { id: int }\n").expect("write source");
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
                "text": "struct Draft { id: int }\n",
            },
        },
    }));
    session.handle_notification(&serde_json::json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didClose",
        "params": {
            "textDocument": {
                "uri": format!("file://{}", source.display()),
            },
        },
    }));

    let response = session.jsonrpc_response(&serde_json::json!({
        "jsonrpc": "2.0",
        "id": 16,
        "method": "textDocument/documentSymbol",
        "params": {
            "textDocument": {
                "uri": format!("file://{}", source.display()),
            },
        },
    }));
    let symbols = response["result"].as_array().expect("document symbols");

    assert_eq!(response["id"], 16);
    assert!(response.get("error").is_none(), "{response}");
    assert!(symbols.iter().any(|symbol| symbol["name"] == "Disk"));
    assert!(!symbols.iter().any(|symbol| symbol["name"] == "Draft"));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn lsp_did_save_with_text_updates_unsaved_content() {
    let dir = temp_output_dir("lsp-did-save-text-symbol");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let source = dir.join("app.orv");
    std::fs::write(&source, "struct Disk { id: int }\n").expect("write source");
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
                "text": "struct Draft { id: int }\n",
            },
        },
    }));
    session.handle_notification(&serde_json::json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didSave",
        "params": {
            "textDocument": {
                "uri": format!("file://{}", source.display()),
            },
            "text": "struct Saved { id: int }\n",
        },
    }));

    let response = session.jsonrpc_response(&serde_json::json!({
        "jsonrpc": "2.0",
        "id": 17,
        "method": "textDocument/documentSymbol",
        "params": {
            "textDocument": {
                "uri": format!("file://{}", source.display()),
            },
        },
    }));
    let symbols = response["result"].as_array().expect("document symbols");

    assert_eq!(response["id"], 17);
    assert!(response.get("error").is_none(), "{response}");
    assert!(symbols.iter().any(|symbol| symbol["name"] == "Saved"));
    assert!(!symbols.iter().any(|symbol| symbol["name"] == "Draft"));
    assert!(!symbols.iter().any(|symbol| symbol["name"] == "Disk"));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn lsp_did_save_without_text_returns_to_disk_content() {
    let dir = temp_output_dir("lsp-did-save-no-text-symbol");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let source = dir.join("app.orv");
    std::fs::write(&source, "struct Disk { id: int }\n").expect("write source");
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
                "text": "struct Draft { id: int }\n",
            },
        },
    }));
    session.handle_notification(&serde_json::json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didSave",
        "params": {
            "textDocument": {
                "uri": format!("file://{}", source.display()),
            },
        },
    }));

    let response = session.jsonrpc_response(&serde_json::json!({
        "jsonrpc": "2.0",
        "id": 18,
        "method": "textDocument/documentSymbol",
        "params": {
            "textDocument": {
                "uri": format!("file://{}", source.display()),
            },
        },
    }));
    let symbols = response["result"].as_array().expect("document symbols");

    assert_eq!(response["id"], 18);
    assert!(response.get("error").is_none(), "{response}");
    assert!(symbols.iter().any(|symbol| symbol["name"] == "Disk"));
    assert!(!symbols.iter().any(|symbol| symbol["name"] == "Draft"));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn lsp_reveal_returns_location_for_build_origin() {
    let dir = temp_output_dir("lsp-reveal");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let path = dir.join("app.orv");
    std::fs::write(
        &path,
        r"@server {
  @listen 0
  @route GET /ping {
    @respond 200 { ok: true }
  }
}",
    )
    .expect("write source");
    let out = dir.join("dist");

    cmd_build(&path, &out).expect("build artifacts");
    let origin_map: orv_compiler::OriginMap = serde_json::from_str(
        &std::fs::read_to_string(out.join("origin-map.json")).expect("origin map"),
    )
    .expect("origin map json");
    let route = origin_map
        .entries
        .iter()
        .find(|entry| entry.kind == "route" && entry.name == "GET /ping")
        .expect("route origin");

    let reveal = lsp_reveal_json(&out, &route.id).expect("lsp reveal");

    assert_eq!(reveal["schema_version"], 1);
    assert_eq!(reveal["origin"]["id"], route.id);
    let canonical_path = std::fs::canonicalize(&path).expect("canonical source path");
    assert_eq!(
        reveal["location"]["uri"],
        format!("file://{}", canonical_path.display())
    );
    assert_eq!(reveal["location"]["range"]["start"]["line"], 2);
    assert_eq!(reveal["location"]["range"]["start"]["character"], 2);
    assert!(reveal["production"]["routes"]
        .as_array()
        .expect("routes")
        .iter()
        .any(|route| route["method"] == "GET" && route["path"] == "/ping"));
    assert_eq!(reveal["production"]["summary"]["route_target_count"], 1);
    assert_eq!(
        reveal["production"]["summary"]["native_server_target_count"],
        1
    );
    assert_eq!(
        reveal["production"]["summary"]["native_server_route_count"],
        1
    );
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn lsp_reveal_uses_build_source_bundle_when_original_source_is_missing() {
    let dir = temp_output_dir("lsp-reveal-source-bundle");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let path = dir.join("page.orv");
    std::fs::write(
        &path,
        r#"let sig count: int = 0
@out @html { @body { @p count } }"#,
    )
    .expect("write source");
    let out = dir.join("dist");

    cmd_build(&path, &out).expect("build artifacts");
    let origin_map: orv_compiler::OriginMap = serde_json::from_str(
        &std::fs::read_to_string(out.join("origin-map.json")).expect("origin map"),
    )
    .expect("origin map json");
    let signal = origin_map
        .entries
        .iter()
        .find(|entry| entry.kind == "signal" && entry.name == "count")
        .expect("signal origin");
    std::fs::remove_file(&path).expect("remove source");

    let reveal = lsp_reveal_json(&out, &signal.id).expect("lsp reveal");

    assert_eq!(reveal["origin"]["kind"], "signal");
    assert_eq!(reveal["location"]["range"]["start"]["line"], 0);
    assert!(reveal["production"]["client"]
        .as_array()
        .expect("client targets")
        .iter()
        .any(|target| target["kind"] == "client_wasm"));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn lsp_reveal_exposes_db_adapter_origin_match() {
    let dir = temp_output_dir("lsp-reveal-db-adapter");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let path = dir.join("app.orv");
    std::fs::write(
        &path,
        r#"@server {
  @listen 8080
  let shopdb = @db.connect(@env.SHOP_DATABASE_URL ?? "postgres://db.internal/shop")
  @route GET /ping { @respond 200 { ok: true } }
}
"#,
    )
    .expect("write source");
    let out = dir.join("dist");

    cmd_build_with_profile(&path, &out, BuildProfile::Production).expect("prod build");
    let origin_map: orv_compiler::OriginMap = serde_json::from_str(
        &std::fs::read_to_string(out.join("origin-map.json")).expect("origin map"),
    )
    .expect("origin map json");
    let db_connect = origin_map
        .entries
        .iter()
        .find(|entry| entry.kind == "call" && entry.name == "@db.connect")
        .expect("db connect origin");

    let reveal = lsp_reveal_json(&out, &db_connect.id).expect("lsp reveal");
    let db_adapters = reveal["production"]["db_adapters"]
        .as_array()
        .expect("db adapters");
    let target = db_adapters
        .iter()
        .find(|target| target["path"] == "deploy/db-adapters.json")
        .expect("db adapter target");
    let matched = target["matched_adapters"]
        .as_array()
        .expect("matched db adapters");

    assert_eq!(reveal["origin"]["id"], db_connect.id);
    assert_eq!(
        reveal["production"]["graph_contract"]
            .as_array()
            .expect("graph contract")
            .len(),
        3
    );
    assert_eq!(reveal["production"]["summary"]["graph_contract_count"], 3);
    assert_eq!(reveal["production"]["summary"]["preflight_target_count"], 1);
    assert_eq!(
        reveal["production"]["summary"]["preflight_smoke_summary_present_count"],
        0
    );
    assert_eq!(
        reveal["production"]["summary"]["preflight_smoke_summary_missing_count"],
        1
    );
    assert_eq!(
        reveal["production"]["summary"]["preflight_smoke_summary_missing_marker_count"],
        0
    );
    assert_eq!(reveal["production"]["summary"]["db_target_count"], 1);
    assert_eq!(target["matched"], true);
    assert_eq!(target["selected_origin_id"], db_connect.id);
    assert_eq!(target["matched_adapter_count"], 1);
    assert_eq!(matched.len(), 1);
    assert_eq!(matched[0]["source_origin_id"], db_connect.id);
    assert_eq!(matched[0]["matched_origin_id"], db_connect.id);
    assert_eq!(matched[0]["match"], "direct");
    assert_eq!(matched[0]["provider"], "postgres");
    assert_eq!(matched[0]["bridge"]["contract"], "http-json-v1");
    let _ = std::fs::remove_dir_all(dir);
}
