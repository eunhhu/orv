use std::collections::BTreeSet;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;

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

    let initialize = lsp_stdio_initialize_response();
    assert_initialize_contract(&initialize);

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
    let body = request.to_string();
    let input = format!("Content-Length: {}\r\n\r\n{}", body.len(), body);
    let output = run_lsp_stdio(input.as_bytes());
    let frames = parse_lsp_frames(&output);
    assert_eq!(frames.len(), 1);
    frames.into_iter().next().expect("initialize frame")
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

fn temp_dir(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time")
        .as_nanos();
    std::env::temp_dir().join(format!("orv-cli-{name}-{}-{nanos}", std::process::id()))
}

const fn orv_bin() -> &'static str {
    env!("CARGO_BIN_EXE_orv")
}

fn run_orv_json(args: &[&str]) -> Value {
    let output = Command::new(orv_bin())
        .args(args)
        .output()
        .expect("run orv");
    assert!(
        output.status.success(),
        "orv {args:?} failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("orv json")
}
