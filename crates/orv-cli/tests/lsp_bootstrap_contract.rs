use crate::support::{orv_bin, run_orv_json, temp_dir};
use std::collections::{BTreeMap, BTreeSet};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde_json::Value;

const LSP_SNAPSHOT_GOLDEN: &str = include_str!("../../../docs/samples/lsp-snapshot-v1.golden.json");
const LSP_INITIALIZE_CAPABILITIES_GOLDEN: &str =
    include_str!("../../../docs/samples/lsp-initialize-capabilities-v1.golden.json");
const LSP_METHOD_INVENTORY_GOLDEN: &str =
    include_str!("../../../docs/samples/lsp-method-inventory-v1.golden.json");
const LSP_EDITOR_ACTION_INVENTORY_GOLDEN: &str =
    include_str!("../../../docs/samples/lsp-editor-action-inventory-v1.golden.json");

const SNAPSHOT_ROOT_KEYS: &[&str] = &[
    "diagnostics",
    "document_symbols",
    "project_graph",
    "schema_version",
    "uri",
];
const DOCUMENT_SYMBOL_KEYS: &[&str] = &["kind", "name", "range", "selectionRange", "source_node"];
const RANGE_KEYS: &[&str] = &["end", "start"];
const POSITION_KEYS: &[&str] = &["character", "line"];
const JSONRPC_ROOT_KEYS: &[&str] = &["id", "jsonrpc", "result"];
const INITIALIZE_RESULT_KEYS: &[&str] = &["capabilities", "serverInfo"];
const SERVER_INFO_KEYS: &[&str] = &["name", "version"];
const CAPABILITY_KEYS: &[&str] = &[
    "callHierarchyProvider",
    "codeActionProvider",
    "codeLensProvider",
    "colorProvider",
    "completionProvider",
    "declarationProvider",
    "definitionProvider",
    "diagnosticProvider",
    "documentFormattingProvider",
    "documentHighlightProvider",
    "documentLinkProvider",
    "documentOnTypeFormattingProvider",
    "documentRangeFormattingProvider",
    "documentSymbolProvider",
    "executeCommandProvider",
    "foldingRangeProvider",
    "hoverProvider",
    "implementationProvider",
    "inlayHintProvider",
    "linkedEditingRangeProvider",
    "monikerProvider",
    "referencesProvider",
    "renameProvider",
    "selectionRangeProvider",
    "semanticTokensProvider",
    "signatureHelpProvider",
    "textDocumentSync",
    "typeDefinitionProvider",
    "typeHierarchyProvider",
    "workspaceSymbolProvider",
];

#[test]
fn lsp_bootstrap_v1_freezes_snapshot_and_initialize_contracts() {
    let root = temp_dir("lsp-bootstrap-contract");
    std::fs::create_dir_all(&root).expect("create temp dir");
    let source = root.join("app.orv");
    std::fs::write(
        &source,
        r#"struct User { id: int }
enum Role { Admin = "admin", User = "user" }
type UserId = int
define Auth() -> { @out "auth" }
function greet(user: User): string -> "hello"
@server {
  @listen 8080
  @route GET /users/:id { @respond 200 { ok: true } }
}
"#,
    )
    .expect("write source");

    let source_arg = source.display().to_string();
    let snapshot = run_orv_json(&["lsp", "snapshot", &source_arg]);
    assert_snapshot_contract(&snapshot, &source);
    assert_lsp_snapshot_golden(&snapshot, &source);

    let initialize = lsp_stdio_initialize_response();
    assert_initialize_contract(&initialize);
    assert_initialize_capabilities_golden(&initialize);

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn lsp_bootstrap_v1_freezes_common_method_inventory() {
    let root = temp_dir("lsp-method-inventory-contract");
    std::fs::create_dir_all(&root).expect("create temp dir");
    let source = root.join("app.orv");
    std::fs::write(
        &source,
        r#"struct User { id: int }
function greet(user: User): string -> "hello"
@server{
@listen 8080
@route GET /users/:id { @respond 200 { ok: true } }
}
@
"#,
    )
    .expect("write source");

    let frames = lsp_common_method_frames(&source);
    assert_lsp_method_inventory_golden(&frames);

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn lsp_bootstrap_v1_freezes_editor_action_inventory() {
    let root = temp_dir("lsp-editor-action-inventory-contract");
    let src = root.join("src");
    let models = src.join("models");
    std::fs::create_dir_all(&models).expect("create temp models dir");
    let main = src.join("main.orv");
    let imported = models.join("user.orv");
    std::fs::write(
        root.join("orv.toml"),
        r#"[project]
name = "lsp-editor-action-inventory"
entry = "src/main.orv"
"#,
    )
    .expect("write manifest");
    std::fs::write(
        &main,
        r#"import models.user.User
function greet(user: User): string -> "hello"

let u: User = { id: 1 }
let v: User = u
"#,
    )
    .expect("write main source");
    std::fs::write(
        &imported,
        "pub struct User { id: int }\nlet bad: int = \"wrong\"\n",
    )
    .expect("write imported source");

    let paths = LspEditorActionPaths {
        root: root.clone(),
        main,
        imported,
    };
    let frames = lsp_editor_action_frames(&paths);
    let user_lens = user_code_lens(&frames).clone();
    let execute_frames = lsp_execute_command_frames(&paths, &user_lens);
    assert_lsp_editor_action_inventory_golden(&frames, &execute_frames, &paths);

    let _ = std::fs::remove_dir_all(root);
}

fn assert_snapshot_contract(snapshot: &Value, source: &Path) {
    assert_object_keys(snapshot, SNAPSHOT_ROOT_KEYS);
    assert_eq!(snapshot["schema_version"], 1);
    assert_eq!(snapshot["uri"], source.display().to_string());
    assert_eq!(
        snapshot["diagnostics"]
            .as_array()
            .expect("diagnostics")
            .len(),
        0
    );
    assert_eq!(snapshot["project_graph"]["schema_version"], 1);

    let symbols = snapshot["document_symbols"]
        .as_array()
        .expect("document symbols");
    for symbol in symbols {
        assert_document_symbol_shape(symbol);
    }
    assert_named_document_symbol(symbols, "User", "Struct");
    assert_named_document_symbol(symbols, "Role", "Enum");
    assert_named_document_symbol(symbols, "UserId", "TypeAlias");
    assert_named_document_symbol(symbols, "Auth", "Function");
    assert_named_document_symbol(symbols, "greet", "Function");
    assert_named_document_symbol(symbols, "server", "Event");
    assert_named_document_symbol(symbols, "route", "Event");
}

fn assert_lsp_snapshot_golden(snapshot: &Value, source: &Path) {
    let expected: Value = serde_json::from_str(LSP_SNAPSHOT_GOLDEN).expect("LSP snapshot golden");
    assert_eq!(
        normalize_lsp_snapshot_for_golden(snapshot.clone(), source),
        expected,
        "LSP snapshot golden drift"
    );
}

fn normalize_lsp_snapshot_for_golden(mut snapshot: Value, source: &Path) -> Value {
    let diagnostics_count = snapshot["diagnostics"]
        .as_array()
        .expect("diagnostics")
        .len();
    let object = snapshot.as_object_mut().expect("snapshot object");
    object.remove("diagnostics");
    object.insert(
        "diagnostics_count".to_string(),
        serde_json::json!(diagnostics_count),
    );
    normalize_source_paths(&mut snapshot, source);
    snapshot
}

fn normalize_source_paths(value: &mut Value, source: &Path) {
    let source_path = source.display().to_string();
    let canonical_path = std::fs::canonicalize(source)
        .expect("canonical source path")
        .display()
        .to_string();
    normalize_path_strings(
        value,
        &[
            (source_path.as_str(), "<entry>"),
            (canonical_path.as_str(), "<entry>"),
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
            let entries = std::mem::take(object);
            for (key, mut item) in entries {
                normalize_path_strings(&mut item, replacements);
                let normalized_key = replacements
                    .iter()
                    .find_map(|(needle, replacement)| {
                        (key.as_str() == *needle).then(|| (*replacement).to_string())
                    })
                    .unwrap_or(key);
                object.insert(normalized_key, item);
            }
        }
        Value::Null | Value::Bool(_) | Value::Number(_) => {}
    }
}

fn assert_named_document_symbol(symbols: &[Value], name: &str, kind: &str) {
    let symbol = symbols
        .iter()
        .find(|symbol| symbol["name"] == name && symbol["kind"] == kind)
        .unwrap_or_else(|| panic!("{name} {kind} symbol"));
    assert_document_symbol_shape(symbol);
}

fn assert_document_symbol_shape(symbol: &Value) {
    assert_object_keys(symbol, DOCUMENT_SYMBOL_KEYS);
    assert!(symbol["name"].as_str().is_some());
    assert!(symbol["kind"].as_str().is_some());
    assert_range_contract(&symbol["range"]);
    assert_range_contract(&symbol["selectionRange"]);
    assert!(symbol["source_node"].as_u64().is_some());
}

fn assert_initialize_contract(response: &Value) {
    assert_object_keys(response, JSONRPC_ROOT_KEYS);
    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 1);
    assert_object_keys(&response["result"], INITIALIZE_RESULT_KEYS);
    assert_object_keys(&response["result"]["serverInfo"], SERVER_INFO_KEYS);
    assert_eq!(response["result"]["serverInfo"]["name"], "orv-lsp");
    assert!(response["result"]["serverInfo"]["version"]
        .as_str()
        .is_some());

    let capabilities = &response["result"]["capabilities"];
    assert_object_keys(capabilities, CAPABILITY_KEYS);
    assert_text_document_sync_contract(capabilities);
    assert_command_and_action_contract(capabilities);
    assert_semantic_tokens_contract(capabilities);
    assert_editing_capability_contract(capabilities);
}

fn assert_initialize_capabilities_golden(response: &Value) {
    let expected: Value = serde_json::from_str(LSP_INITIALIZE_CAPABILITIES_GOLDEN)
        .expect("LSP initialize capabilities golden");
    assert_eq!(
        response["result"]["capabilities"], expected,
        "LSP initialize capabilities golden drift"
    );
}

fn assert_lsp_method_inventory_golden(frames: &[Value]) {
    let expected: Value =
        serde_json::from_str(LSP_METHOD_INVENTORY_GOLDEN).expect("LSP method inventory golden");
    assert_eq!(
        lsp_method_inventory(frames),
        expected,
        "LSP common method inventory golden drift"
    );
}

fn assert_lsp_editor_action_inventory_golden(
    frames: &[Value],
    execute_frames: &[Value],
    paths: &LspEditorActionPaths,
) {
    let expected: Value = serde_json::from_str(LSP_EDITOR_ACTION_INVENTORY_GOLDEN)
        .expect("LSP editor action inventory golden");
    assert_eq!(
        lsp_editor_action_inventory(frames, execute_frames, paths),
        expected,
        "LSP editor action inventory golden drift"
    );
}

struct LspEditorActionPaths {
    root: PathBuf,
    main: PathBuf,
    imported: PathBuf,
}

fn lsp_editor_action_frames(paths: &LspEditorActionPaths) -> Vec<Value> {
    let main_uri = file_uri(&paths.main);
    let imported_uri = file_uri(&paths.imported);
    lsp_stdio_responses(&[
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "rootUri": file_uri(&paths.root),
            },
        }),
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "textDocument/diagnostic",
            "params": {
                "textDocument": { "uri": imported_uri },
            },
        }),
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "textDocument/documentLink",
            "params": {
                "textDocument": { "uri": main_uri },
            },
        }),
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 4,
            "method": "textDocument/prepareRename",
            "params": {
                "textDocument": { "uri": main_uri },
                "position": { "line": 1, "character": 22 },
            },
        }),
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 5,
            "method": "textDocument/rename",
            "params": {
                "textDocument": { "uri": main_uri },
                "position": { "line": 3, "character": 8 },
                "newName": "Account",
            },
        }),
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 6,
            "method": "textDocument/documentHighlight",
            "params": {
                "textDocument": { "uri": main_uri },
                "position": { "line": 3, "character": 8 },
            },
        }),
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 7,
            "method": "textDocument/references",
            "params": {
                "textDocument": { "uri": main_uri },
                "position": { "line": 3, "character": 8 },
            },
        }),
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 8,
            "method": "workspace/symbol",
            "params": {
                "query": "User",
            },
        }),
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 9,
            "method": "workspace/diagnostic",
            "params": {
                "previousResultIds": [],
            },
        }),
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 10,
            "method": "textDocument/codeLens",
            "params": {
                "textDocument": { "uri": imported_uri },
            },
        }),
    ])
}

fn lsp_execute_command_frames(paths: &LspEditorActionPaths, user_lens: &Value) -> Vec<Value> {
    lsp_stdio_responses(&[
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 20,
            "method": "initialize",
            "params": {
                "rootUri": file_uri(&paths.root),
            },
        }),
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 21,
            "method": "workspace/executeCommand",
            "params": {
                "command": user_lens["command"]["command"],
                "arguments": user_lens["command"]["arguments"],
            },
        }),
    ])
}

fn lsp_editor_action_inventory(
    frames: &[Value],
    execute_frames: &[Value],
    paths: &LspEditorActionPaths,
) -> Value {
    assert_eq!(frames.len(), 10, "LSP editor action frame count drift");
    assert_eq!(
        execute_frames.len(),
        2,
        "LSP execute-command frame count drift"
    );
    for frame in frames.iter().chain(execute_frames) {
        assert!(
            frame.get("error").is_none(),
            "LSP editor action frame errored: {frame}"
        );
    }

    let mut inventory = serde_json::json!({
        "schema_version": 1,
        "kind": "orv.lsp.editor_action.inventory",
        "response_ids": frames
            .iter()
            .map(|frame| frame["id"].clone())
            .collect::<Vec<_>>(),
        "diagnostic": diagnostic_inventory(&frames[1]["result"]),
        "document_links": document_link_inventory(&frames[2]["result"]),
        "prepare_rename": frames[3]["result"],
        "rename": rename_inventory(&frames[4]["result"]),
        "document_highlights": range_kind_inventory(&frames[5]["result"]),
        "references": location_inventory(&frames[6]["result"]),
        "workspace_symbols": workspace_symbol_inventory(&frames[7]["result"]),
        "workspace_diagnostics": workspace_diagnostic_inventory(&frames[8]["result"]),
        "code_lenses": code_lens_inventory(&frames[9]["result"]),
        "execute_command": execute_command_inventory(&execute_frames[1]["result"]),
    });
    normalize_editor_action_paths(&mut inventory, paths);
    inventory
}

fn diagnostic_inventory(result: &Value) -> Value {
    serde_json::json!({
        "kind": result["kind"],
        "items": result["items"]
            .as_array()
            .expect("diagnostic items")
            .iter()
            .map(|item| {
                serde_json::json!({
                    "severity": item["severity"],
                    "source": item["source"],
                    "code": item["code"],
                    "message_class": diagnostic_message_class(&item["message"]),
                    "range": item["range"],
                })
            })
            .collect::<Vec<_>>(),
    })
}

fn document_link_inventory(result: &Value) -> Vec<Value> {
    result
        .as_array()
        .expect("document links")
        .iter()
        .map(|link| {
            serde_json::json!({
                "target": link["target"],
                "range": link["range"],
            })
        })
        .collect()
}

fn rename_inventory(result: &Value) -> Value {
    let changes = result["changes"].as_object().expect("rename changes");
    let mut files = BTreeMap::new();
    for (uri, edits) in changes {
        let edits = edits.as_array().expect("rename edits");
        files.insert(
            uri.clone(),
            serde_json::json!({
                "edit_count": edits.len(),
                "new_texts": edits
                    .iter()
                    .map(|edit| edit["newText"].clone())
                    .collect::<Vec<_>>(),
                "ranges": edits
                    .iter()
                    .map(|edit| edit["range"].clone())
                    .collect::<Vec<_>>(),
            }),
        );
    }
    serde_json::json!({ "changes": files })
}

fn range_kind_inventory(result: &Value) -> Vec<Value> {
    result
        .as_array()
        .expect("range-kind result")
        .iter()
        .map(|item| {
            serde_json::json!({
                "kind": item["kind"],
                "range": item["range"],
            })
        })
        .collect()
}

fn location_inventory(result: &Value) -> Vec<Value> {
    result
        .as_array()
        .expect("locations")
        .iter()
        .map(|location| {
            serde_json::json!({
                "uri": location["uri"],
                "range": location["range"],
            })
        })
        .collect()
}

fn workspace_symbol_inventory(result: &Value) -> Vec<Value> {
    let mut symbols = result
        .as_array()
        .expect("workspace symbols")
        .iter()
        .map(|symbol| {
            serde_json::json!({
                "name": symbol["name"],
                "kind": symbol["kind"],
                "uri": symbol["location"]["uri"],
                "range": symbol["location"]["range"],
            })
        })
        .collect::<Vec<_>>();
    symbols.sort_by_key(|symbol| {
        format!(
            "{}:{}",
            symbol["name"].as_str().unwrap_or_default(),
            symbol["uri"].as_str().unwrap_or_default()
        )
    });
    symbols
}

fn workspace_diagnostic_inventory(result: &Value) -> Value {
    let mut reports = result["items"]
        .as_array()
        .expect("workspace diagnostic items")
        .iter()
        .map(|report| {
            serde_json::json!({
                "uri": report["uri"],
                "version": report["version"],
                "kind": report["kind"],
                "items": report["items"]
                    .as_array()
                    .expect("workspace diagnostic report items")
                    .iter()
                    .map(|item| {
                        serde_json::json!({
                            "severity": item["severity"],
                            "source": item["source"],
                            "code": item["code"],
                            "message_class": diagnostic_message_class(&item["message"]),
                            "range": item["range"],
                        })
                    })
                    .collect::<Vec<_>>(),
            })
        })
        .collect::<Vec<_>>();
    reports.sort_by_key(|report| report["uri"].as_str().unwrap_or_default().to_string());
    serde_json::json!({ "items": reports })
}

fn code_lens_inventory(result: &Value) -> Vec<Value> {
    result
        .as_array()
        .expect("code lenses")
        .iter()
        .map(|lens| {
            serde_json::json!({
                "range": lens["range"],
                "command": {
                    "title": lens["command"]["title"],
                    "command": lens["command"]["command"],
                    "source_node": "<source-node>",
                    "name": lens["command"]["arguments"][1],
                },
            })
        })
        .collect()
}

fn execute_command_inventory(result: &Value) -> Value {
    serde_json::json!({
        "name": result["name"],
        "kind": result["kind"],
        "source_node": "<source-node>",
        "location": result["location"],
    })
}

fn diagnostic_message_class(message: &Value) -> Value {
    let message = message.as_str().unwrap_or_default();
    if message.contains("type mismatch") {
        serde_json::json!("type_mismatch")
    } else {
        serde_json::json!("other")
    }
}

fn user_code_lens(frames: &[Value]) -> &Value {
    frames[9]["result"]
        .as_array()
        .expect("code lenses")
        .iter()
        .find(|lens| lens["command"]["arguments"][1] == "User")
        .expect("User struct code lens")
}

fn normalize_editor_action_paths(value: &mut Value, paths: &LspEditorActionPaths) {
    let root_uri = file_uri(&paths.root);
    let main_uri = file_uri(&paths.main);
    let imported_uri = file_uri(&paths.imported);
    let root_path = paths.root.display().to_string();
    let main_path = paths.main.display().to_string();
    let imported_path = paths.imported.display().to_string();
    let canonical_root_uri = canonical_file_uri(&paths.root);
    let canonical_main_uri = canonical_file_uri(&paths.main);
    let canonical_imported_uri = canonical_file_uri(&paths.imported);
    let canonical_root_path = std::fs::canonicalize(&paths.root)
        .expect("canonical LSP inventory workspace")
        .display()
        .to_string();
    let canonical_main_path = std::fs::canonicalize(&paths.main)
        .expect("canonical LSP inventory main")
        .display()
        .to_string();
    let canonical_imported_path = std::fs::canonicalize(&paths.imported)
        .expect("canonical LSP inventory imported")
        .display()
        .to_string();
    normalize_path_strings(
        value,
        &[
            (root_uri.as_str(), "file://<workspace>"),
            (canonical_root_uri.as_str(), "file://<workspace>"),
            (main_uri.as_str(), "file://<entry>"),
            (canonical_main_uri.as_str(), "file://<entry>"),
            (imported_uri.as_str(), "file://<imported>"),
            (canonical_imported_uri.as_str(), "file://<imported>"),
            (root_path.as_str(), "<workspace>"),
            (canonical_root_path.as_str(), "<workspace>"),
            (main_path.as_str(), "<entry>"),
            (canonical_main_path.as_str(), "<entry>"),
            (imported_path.as_str(), "<imported>"),
            (canonical_imported_path.as_str(), "<imported>"),
        ],
    );
}

fn file_uri(path: &Path) -> String {
    format!("file://{}", path.display())
}

fn canonical_file_uri(path: &Path) -> String {
    format!(
        "file://{}",
        std::fs::canonicalize(path)
            .expect("canonical LSP inventory path")
            .display()
    )
}

fn lsp_common_method_frames(source: &Path) -> Vec<Value> {
    let uri = format!("file://{}", source.display());
    lsp_stdio_responses(&[
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {},
        }),
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "textDocument/documentSymbol",
            "params": {
                "textDocument": { "uri": uri },
            },
        }),
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "textDocument/completion",
            "params": {
                "textDocument": { "uri": uri },
                "position": { "line": 6, "character": 1 },
            },
        }),
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 4,
            "method": "textDocument/hover",
            "params": {
                "textDocument": { "uri": uri },
                "position": { "line": 0, "character": 8 },
            },
        }),
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 5,
            "method": "textDocument/formatting",
            "params": {
                "textDocument": { "uri": uri },
                "options": { "tabSize": 2, "insertSpaces": true },
            },
        }),
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 6,
            "method": "textDocument/semanticTokens/full",
            "params": {
                "textDocument": { "uri": uri },
            },
        }),
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 7,
            "method": "textDocument/foldingRange",
            "params": {
                "textDocument": { "uri": uri },
            },
        }),
    ])
}

fn lsp_method_inventory(frames: &[Value]) -> Value {
    assert_eq!(frames.len(), 7, "LSP method inventory frame count drift");
    serde_json::json!({
        "response_ids": frames
            .iter()
            .map(|frame| frame["id"].clone())
            .collect::<Vec<_>>(),
        "initialize_capability_count": frames[0]["result"]["capabilities"]
            .as_object()
            .expect("initialize capabilities")
            .len(),
        "document_symbols": frames[1]["result"],
        "completion": {
            "isIncomplete": frames[2]["result"]["isIncomplete"],
            "items": completion_item_inventory(&frames[2]["result"]["items"]),
        },
        "hover": frames[3]["result"],
        "formatting": frames[4]["result"],
        "semantic_tokens": frames[5]["result"],
        "folding_ranges": frames[6]["result"],
    })
}

fn completion_item_inventory(items: &Value) -> Vec<Value> {
    items
        .as_array()
        .expect("completion items")
        .iter()
        .map(|item| {
            let mut object = serde_json::Map::new();
            object.insert("label".to_string(), item["label"].clone());
            object.insert("kind".to_string(), item["kind"].clone());
            if let Some(detail) = item.get("detail") {
                object.insert("detail".to_string(), detail.clone());
            }
            if let Some(insert_text_format) = item.get("insertTextFormat") {
                object.insert("insertTextFormat".to_string(), insert_text_format.clone());
            }
            Value::Object(object)
        })
        .collect()
}

fn assert_text_document_sync_contract(capabilities: &Value) {
    assert_object_keys(
        &capabilities["textDocumentSync"],
        &["change", "openClose", "save"],
    );
    assert_eq!(capabilities["textDocumentSync"]["openClose"], true);
    assert_eq!(capabilities["textDocumentSync"]["change"], 1);
    assert_object_keys(&capabilities["textDocumentSync"]["save"], &["includeText"]);
    assert_eq!(
        capabilities["textDocumentSync"]["save"]["includeText"],
        true
    );
}

fn assert_command_and_action_contract(capabilities: &Value) {
    assert_object_keys(&capabilities["codeLensProvider"], &["resolveProvider"]);
    assert_eq!(capabilities["codeLensProvider"]["resolveProvider"], false);
    assert_object_keys(&capabilities["codeActionProvider"], &["codeActionKinds"]);
    assert_eq!(
        capabilities["codeActionProvider"]["codeActionKinds"],
        serde_json::json!(["quickfix"])
    );
    assert_object_keys(&capabilities["executeCommandProvider"], &["commands"]);
    assert_eq!(
        capabilities["executeCommandProvider"]["commands"],
        serde_json::json!(["orv.revealSourceNode", "orv.revealDiagnostic"])
    );
    assert_object_keys(&capabilities["documentLinkProvider"], &["resolveProvider"]);
    assert_eq!(
        capabilities["documentLinkProvider"]["resolveProvider"],
        false
    );
}

fn assert_semantic_tokens_contract(capabilities: &Value) {
    assert_object_keys(
        &capabilities["semanticTokensProvider"],
        &["full", "legend", "range"],
    );
    assert_eq!(capabilities["semanticTokensProvider"]["full"], true);
    assert_eq!(capabilities["semanticTokensProvider"]["range"], false);
    assert_object_keys(
        &capabilities["semanticTokensProvider"]["legend"],
        &["tokenModifiers", "tokenTypes"],
    );
    assert_eq!(
        capabilities["semanticTokensProvider"]["legend"]["tokenTypes"],
        serde_json::json!(["namespace", "type", "function"])
    );
    assert_eq!(
        capabilities["semanticTokensProvider"]["legend"]["tokenModifiers"],
        serde_json::json!(["declaration"])
    );
}

fn assert_editing_capability_contract(capabilities: &Value) {
    assert_object_keys(&capabilities["renameProvider"], &["prepareProvider"]);
    assert_eq!(capabilities["renameProvider"]["prepareProvider"], true);
    assert_object_keys(
        &capabilities["signatureHelpProvider"],
        &["triggerCharacters"],
    );
    assert_eq!(
        capabilities["signatureHelpProvider"]["triggerCharacters"],
        serde_json::json!(["(", ","])
    );
    assert_object_keys(
        &capabilities["documentOnTypeFormattingProvider"],
        &["firstTriggerCharacter", "moreTriggerCharacter"],
    );
    assert_eq!(
        capabilities["documentOnTypeFormattingProvider"]["firstTriggerCharacter"],
        "}"
    );
    assert_eq!(
        capabilities["documentOnTypeFormattingProvider"]["moreTriggerCharacter"],
        serde_json::json!(["{", "\n"])
    );
    assert_object_keys(&capabilities["completionProvider"], &["triggerCharacters"]);
    assert_eq!(
        capabilities["completionProvider"]["triggerCharacters"],
        serde_json::json!(["@", ".", ":"])
    );
    assert_object_keys(
        &capabilities["diagnosticProvider"],
        &["interFileDependencies", "workspaceDiagnostics"],
    );
    assert_eq!(
        capabilities["diagnosticProvider"]["interFileDependencies"],
        true
    );
    assert_eq!(
        capabilities["diagnosticProvider"]["workspaceDiagnostics"],
        true
    );
}

fn assert_range_contract(range: &Value) {
    assert_object_keys(range, RANGE_KEYS);
    assert_object_keys(&range["start"], POSITION_KEYS);
    assert_object_keys(&range["end"], POSITION_KEYS);
}

fn lsp_stdio_initialize_response() -> Value {
    let request = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {},
    });
    let frames = lsp_stdio_responses(&[request]);
    assert_eq!(frames.len(), 1);
    frames.into_iter().next().expect("initialize frame")
}

fn lsp_stdio_responses(requests: &[Value]) -> Vec<Value> {
    use std::fmt::Write as _;
    let mut input = String::new();
    for request in requests {
        let body = request.to_string();
        let _ = write!(input, "Content-Length: {}\r\n\r\n{}", body.len(), body);
    }
    let output = run_lsp_stdio(input.as_bytes());
    parse_lsp_frames(&output)
}

fn run_lsp_stdio(input: &[u8]) -> Vec<u8> {
    let mut child = Command::new(orv_bin())
        .args(["lsp", "serve", "--stdio"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn orv lsp serve");
    {
        let stdin = child.stdin.as_mut().expect("stdin");
        stdin.write_all(input).expect("write lsp input");
    }
    drop(child.stdin.take());
    let output = child.wait_with_output().expect("wait lsp serve");
    assert!(
        output.status.success(),
        "orv lsp serve failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    output.stdout
}

fn parse_lsp_frames(mut output: &[u8]) -> Vec<Value> {
    let mut frames = Vec::new();
    while !output.is_empty() {
        let header_end = find_subslice(output, b"\r\n\r\n").expect("frame header");
        let header = std::str::from_utf8(&output[..header_end]).expect("utf8 header");
        let content_length = header
            .lines()
            .find_map(|line| line.strip_prefix("Content-Length: "))
            .expect("content length")
            .parse::<usize>()
            .expect("content length number");
        let body_start = header_end + 4;
        let body_end = body_start + content_length;
        assert!(body_end <= output.len(), "truncated lsp body");
        frames.push(serde_json::from_slice(&output[body_start..body_end]).expect("lsp json"));
        output = &output[body_end..];
    }
    frames
}

fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

fn assert_object_keys(value: &Value, expected: &[&str]) {
    let object = value.as_object().expect("object");
    let actual = object.keys().map(String::as_str).collect::<BTreeSet<_>>();
    let expected = expected.iter().copied().collect::<BTreeSet<_>>();
    assert_eq!(actual, expected);
}
