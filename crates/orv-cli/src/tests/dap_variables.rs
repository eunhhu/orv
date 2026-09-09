use super::*;

#[test]
fn dap_scopes_and_variables_expose_project_launch_state() {
    let dir = temp_output_dir("dap-variables");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let source = dir.join("app.orv");
    let source_text = "let answer: int = 42\n";
    std::fs::write(&source, source_text).expect("write source");
    let canonical_source = std::fs::canonicalize(&source).expect("canonical source");
    let mut session = DapSession::default();

    session
        .message_response(&serde_json::json!({
            "seq": 8,
            "type": "request",
            "command": "launch",
            "arguments": {
                "program": format!("file://{}", source.display()),
            },
        }))
        .expect("launch response");
    let scopes = session
        .message_response(&serde_json::json!({
            "seq": 9,
            "type": "request",
            "command": "scopes",
            "arguments": {
                "frameId": 1,
            },
        }))
        .expect("scopes response");
    let variables = session
        .message_response(&serde_json::json!({
            "seq": 10,
            "type": "request",
            "command": "variables",
            "arguments": {
                "variablesReference": 1,
            },
        }))
        .expect("variables response");

    assert_eq!(scopes["success"], true, "{scopes}");
    assert_eq!(scopes["body"]["scopes"][0]["name"], "Project");
    assert_eq!(scopes["body"]["scopes"][0]["variablesReference"], 1);
    assert_eq!(scopes["body"]["scopes"][0]["namedVariables"], 6);
    assert_eq!(scopes["body"]["scopes"][1]["name"], "Locals");
    assert_eq!(
        scopes["body"]["scopes"][0]["source"]["checksums"][0]["algorithm"],
        serde_json::json!("SHA256")
    );
    assert_eq!(
        scopes["body"]["scopes"][0]["source"]["checksums"][0]["checksum"],
        serde_json::json!(sha256_hex(source_text.as_bytes()))
    );
    assert!(scopes["body"]["scopes"][1]["namedVariables"]
        .as_u64()
        .is_some_and(|count| count >= 1));
    let vars = variables["body"]["variables"]
        .as_array()
        .expect("variables");
    assert_eq!(
        scopes["body"]["scopes"][0]["namedVariables"],
        serde_json::json!(vars.len())
    );
    assert!(vars.iter().any(|var| {
        var["name"] == "entry" && var["value"] == canonical_source.display().to_string()
    }));
    assert!(vars
        .iter()
        .any(|var| var["name"] == "projectGraphNodes" && var["value"] == "1"));
    assert!(vars
        .iter()
        .any(|var| var["name"] == "diagnostics" && var["value"] == "0"));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn dap_scopes_rejects_unknown_frame_id() {
    let dir = temp_output_dir("dap-scopes-frame-id");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let source = dir.join("app.orv");
    std::fs::write(&source, "let answer: int = 42\n").expect("write source");
    let mut session = DapSession::default();

    session
        .message_response(&serde_json::json!({
            "seq": 214,
            "type": "request",
            "command": "launch",
            "arguments": {
                "program": format!("file://{}", source.display()),
            },
        }))
        .expect("launch response");
    let response = session
        .message_response(&serde_json::json!({
            "seq": 215,
            "type": "request",
            "command": "scopes",
            "arguments": {
                "frameId": 99,
            },
        }))
        .expect("scopes response");

    assert_eq!(response["success"], false, "{response}");
    assert!(response["message"]
        .as_str()
        .is_some_and(|message| message.contains("unknown ORV frameId 99")));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn dap_variables_expose_top_level_locals() {
    let dir = temp_output_dir("dap-locals");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let source = dir.join("app.orv");
    std::fs::write(
        &source,
        "let answer: int = 42\nconst greeting = \"hello\"\nlet ready = true\n",
    )
    .expect("write source");
    let mut session = DapSession::default();

    session
        .message_response(&serde_json::json!({
            "seq": 41,
            "type": "request",
            "command": "launch",
            "arguments": {
                "program": format!("file://{}", source.display()),
            },
        }))
        .expect("launch response");
    let scopes = session
        .message_response(&serde_json::json!({
            "seq": 42,
            "type": "request",
            "command": "scopes",
            "arguments": {
                "frameId": 1,
            },
        }))
        .expect("scopes response");
    let locals_ref = scopes["body"]["scopes"]
        .as_array()
        .expect("scopes")
        .iter()
        .find(|scope| scope["name"] == "Locals")
        .and_then(|scope| scope["variablesReference"].as_u64())
        .expect("locals scope");
    session
        .message_response(&serde_json::json!({
            "seq": 43,
            "type": "request",
            "command": "next",
            "arguments": {
                "threadId": 1,
            },
        }))
        .expect("first next response");
    session
        .message_response(&serde_json::json!({
            "seq": 44,
            "type": "request",
            "command": "next",
            "arguments": {
                "threadId": 1,
            },
        }))
        .expect("second next response");
    let locals = session
        .message_response(&serde_json::json!({
            "seq": 45,
            "type": "request",
            "command": "variables",
            "arguments": {
                "variablesReference": locals_ref,
            },
        }))
        .expect("locals response");

    assert_eq!(locals_ref, 2);
    assert_eq!(locals["success"], true, "{locals}");
    let vars = locals["body"]["variables"].as_array().expect("locals");
    assert!(vars
        .iter()
        .any(|var| var["name"] == "answer" && var["value"] == "42" && var["type"] == "int"));
    assert!(vars.iter().any(|var| {
        var["name"] == "greeting" && var["value"] == "\"hello\"" && var["type"] == "string"
    }));
    assert!(vars
        .iter()
        .any(|var| var["name"] == "ready" && var["value"] == "true" && var["type"] == "bool"));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn dap_variables_honor_start_and_count() {
    let dir = temp_output_dir("dap-variables-paging");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let source = dir.join("app.orv");
    std::fs::write(
        &source,
        "let answer: int = 42\nconst greeting = \"hello\"\nlet ready = true\n",
    )
    .expect("write source");
    let mut session = DapSession::default();

    session
        .message_response(&serde_json::json!({
            "seq": 207,
            "type": "request",
            "command": "launch",
            "arguments": {
                "program": format!("file://{}", source.display()),
            },
        }))
        .expect("launch response");
    session
        .message_response(&serde_json::json!({
            "seq": 208,
            "type": "request",
            "command": "next",
            "arguments": {
                "threadId": 1,
            },
        }))
        .expect("first next response");
    session
        .message_response(&serde_json::json!({
            "seq": 209,
            "type": "request",
            "command": "next",
            "arguments": {
                "threadId": 1,
            },
        }))
        .expect("second next response");
    let locals = session
        .message_response(&serde_json::json!({
            "seq": 210,
            "type": "request",
            "command": "variables",
            "arguments": {
                "variablesReference": 2,
                "start": 1,
                "count": 1,
            },
        }))
        .expect("locals response");

    assert_eq!(locals["success"], true, "{locals}");
    let vars = locals["body"]["variables"].as_array().expect("locals");
    assert_eq!(vars.len(), 1);
    assert_eq!(vars[0]["name"], "greeting");
    assert_eq!(vars[0]["value"], "\"hello\"");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn dap_variables_honor_named_and_indexed_filters() {
    let dir = temp_output_dir("dap-variables-filter");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let source = dir.join("app.orv");
    std::fs::write(&source, "let answer: int = 42\n").expect("write source");
    let mut session = DapSession::default();

    session
        .message_response(&serde_json::json!({
            "seq": 211,
            "type": "request",
            "command": "launch",
            "arguments": {
                "program": format!("file://{}", source.display()),
            },
        }))
        .expect("launch response");
    let named = session
        .message_response(&serde_json::json!({
            "seq": 212,
            "type": "request",
            "command": "variables",
            "arguments": {
                "variablesReference": 2,
                "filter": "named",
            },
        }))
        .expect("named locals response");
    let indexed = session
        .message_response(&serde_json::json!({
            "seq": 213,
            "type": "request",
            "command": "variables",
            "arguments": {
                "variablesReference": 2,
                "filter": "indexed",
            },
        }))
        .expect("indexed locals response");

    assert_eq!(named["success"], true, "{named}");
    assert_eq!(indexed["success"], true, "{indexed}");
    assert!(named["body"]["variables"]
        .as_array()
        .expect("named locals")
        .iter()
        .any(|var| var["name"] == "answer"));
    assert!(indexed["body"]["variables"]
        .as_array()
        .expect("indexed locals")
        .is_empty());
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn dap_set_variable_updates_current_local_and_evaluate() {
    let dir = temp_output_dir("dap-set-variable");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let source = dir.join("app.orv");
    std::fs::write(&source, "let answer: int = 42\n").expect("write source");
    let mut session = DapSession::default();

    session
        .message_response(&serde_json::json!({
            "seq": 168,
            "type": "request",
            "command": "launch",
            "arguments": {
                "program": format!("file://{}", source.display()),
            },
        }))
        .expect("launch response");
    let set_variable = session
        .message_response(&serde_json::json!({
            "seq": 169,
            "type": "request",
            "command": "setVariable",
            "arguments": {
                "variablesReference": 2,
                "name": "answer",
                "value": "99",
            },
        }))
        .expect("setVariable response");
    let locals = session
        .message_response(&serde_json::json!({
            "seq": 170,
            "type": "request",
            "command": "variables",
            "arguments": {
                "variablesReference": 2,
            },
        }))
        .expect("locals response");
    let evaluate = session
        .message_response(&serde_json::json!({
            "seq": 171,
            "type": "request",
            "command": "evaluate",
            "arguments": {
                "expression": "answer",
                "context": "repl",
            },
        }))
        .expect("evaluate response");

    assert_eq!(set_variable["success"], true, "{set_variable}");
    assert_eq!(set_variable["body"]["value"], "99");
    assert_eq!(set_variable["body"]["type"], "int");
    let vars = locals["body"]["variables"].as_array().expect("locals");
    assert!(vars
        .iter()
        .any(|var| var["name"] == "answer" && var["value"] == "99" && var["type"] == "int"));
    assert_eq!(evaluate["body"]["result"], "99");
    assert_eq!(evaluate["body"]["type"], "int");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn dap_evaluate_and_completions_include_top_level_locals() {
    let dir = temp_output_dir("dap-local-evaluate");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let source = dir.join("app.orv");
    std::fs::write(&source, "let answer: int = 42\n").expect("write source");
    let mut session = DapSession::default();

    session
        .message_response(&serde_json::json!({
            "seq": 44,
            "type": "request",
            "command": "launch",
            "arguments": {
                "program": format!("file://{}", source.display()),
            },
        }))
        .expect("launch response");
    let evaluate = session
        .message_response(&serde_json::json!({
            "seq": 45,
            "type": "request",
            "command": "evaluate",
            "arguments": {
                "expression": "answer",
                "context": "repl",
            },
        }))
        .expect("evaluate response");
    let completions = session
        .message_response(&serde_json::json!({
            "seq": 46,
            "type": "request",
            "command": "completions",
            "arguments": {
                "text": "ans",
                "column": 4,
                "line": 1,
            },
        }))
        .expect("completions response");

    assert_eq!(evaluate["success"], true, "{evaluate}");
    assert_eq!(evaluate["body"]["result"], "42");
    assert_eq!(evaluate["body"]["type"], "int");
    let targets = completions["body"]["targets"]
        .as_array()
        .expect("completion targets");
    assert!(targets
        .iter()
        .any(|target| target["label"] == "answer" && target["type"] == "variable"));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn dap_locals_use_runtime_values_from_function_calls() {
    let dir = temp_output_dir("dap-runtime-call-locals");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let source = dir.join("app.orv");
    std::fs::write(
        &source,
        "function add(a: int, b: int): int -> a + b\nlet total: int = add(2, 3)\n",
    )
    .expect("write source");
    let mut session = DapSession::default();

    session
        .message_response(&serde_json::json!({
            "seq": 151,
            "type": "request",
            "command": "launch",
            "arguments": {
                "program": format!("file://{}", source.display()),
            },
        }))
        .expect("launch response");
    session
        .message_response(&serde_json::json!({
            "seq": 152,
            "type": "request",
            "command": "next",
            "arguments": {
                "threadId": 1,
            },
        }))
        .expect("next response");
    let locals = session
        .message_response(&serde_json::json!({
            "seq": 153,
            "type": "request",
            "command": "variables",
            "arguments": {
                "variablesReference": 2,
            },
        }))
        .expect("locals response");
    let evaluate = session
        .message_response(&serde_json::json!({
            "seq": 154,
            "type": "request",
            "command": "evaluate",
            "arguments": {
                "expression": "total",
                "context": "repl",
            },
        }))
        .expect("evaluate response");

    let vars = locals["body"]["variables"].as_array().expect("locals");
    assert!(vars
        .iter()
        .any(|var| var["name"] == "total" && var["value"] == "5" && var["type"] == "int"));
    assert_eq!(evaluate["success"], true, "{evaluate}");
    assert_eq!(evaluate["body"]["result"], "5");
    assert_eq!(evaluate["body"]["type"], "int");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn dap_variables_include_reference_runtime_output() {
    let dir = temp_output_dir("dap-runtime-output");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let source = dir.join("app.orv");
    std::fs::write(&source, "@out \"debug-ready\"\n").expect("write source");
    let mut session = DapSession::default();

    let launch = session
        .message_response(&serde_json::json!({
            "seq": 11,
            "type": "request",
            "command": "launch",
            "arguments": {
                "program": format!("file://{}", source.display()),
            },
        }))
        .expect("launch response");
    let variables = session
        .message_response(&serde_json::json!({
            "seq": 12,
            "type": "request",
            "command": "variables",
            "arguments": {
                "variablesReference": 1,
            },
        }))
        .expect("variables response");

    assert_eq!(launch["success"], true, "{launch}");
    assert_eq!(launch["body"]["runtime"]["status"], "ok");
    assert_eq!(launch["body"]["runtime"]["stdout"], "debug-ready\n");
    let vars = variables["body"]["variables"]
        .as_array()
        .expect("variables");
    assert!(vars
        .iter()
        .any(|var| var["name"] == "runtimeStatus" && var["value"] == "ok"));
    assert!(vars
        .iter()
        .any(|var| var["name"] == "stdout" && var["value"] == "debug-ready\n"));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn dap_evaluate_returns_project_runtime_values() {
    let dir = temp_output_dir("dap-evaluate");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let source = dir.join("app.orv");
    std::fs::write(&source, "@out \"eval-ready\"\n").expect("write source");
    let mut session = DapSession::default();

    let launch = session
        .message_response(&serde_json::json!({
            "seq": 37,
            "type": "request",
            "command": "launch",
            "arguments": {
                "program": format!("file://{}", source.display()),
            },
        }))
        .expect("launch response");
    let evaluate = session
        .message_response(&serde_json::json!({
            "seq": 38,
            "type": "request",
            "command": "evaluate",
            "arguments": {
                "expression": "stdout",
                "context": "repl",
            },
        }))
        .expect("evaluate response");

    assert_eq!(launch["success"], true, "{launch}");
    assert_eq!(evaluate["success"], true, "{evaluate}");
    assert_eq!(evaluate["body"]["result"], "eval-ready\n");
    assert_eq!(evaluate["body"]["type"], "string");
    assert_eq!(evaluate["body"]["variablesReference"], 0);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn dap_completions_returns_evaluable_project_values() {
    let dir = temp_output_dir("dap-completions");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let source = dir.join("app.orv");
    std::fs::write(&source, "@out \"complete-ready\"\n").expect("write source");
    let mut session = DapSession::default();

    let launch = session
        .message_response(&serde_json::json!({
            "seq": 39,
            "type": "request",
            "command": "launch",
            "arguments": {
                "program": format!("file://{}", source.display()),
            },
        }))
        .expect("launch response");
    let completions = session
        .message_response(&serde_json::json!({
            "seq": 40,
            "type": "request",
            "command": "completions",
            "arguments": {
                "text": "std",
                "column": 4,
                "line": 1,
            },
        }))
        .expect("completions response");

    assert_eq!(launch["success"], true, "{launch}");
    assert_eq!(completions["success"], true, "{completions}");
    let targets = completions["body"]["targets"]
        .as_array()
        .expect("completion targets");
    assert!(targets
        .iter()
        .any(|target| target["label"] == "stdout" && target["type"] == "property"));
    assert!(targets.iter().all(|target| target["label"]
        .as_str()
        .is_some_and(|label| label.starts_with("std"))));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn dap_locals_evaluate_pure_top_level_expressions() {
    let dir = temp_output_dir("dap-expression-locals");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let source = dir.join("app.orv");
    std::fs::write(
        &source,
        "let base: int = 2\nlet doubled: int = base * 2 + 1\n",
    )
    .expect("write source");
    let mut session = DapSession::default();

    session
        .message_response(&serde_json::json!({
            "seq": 62,
            "type": "request",
            "command": "launch",
            "arguments": {
                "program": format!("file://{}", source.display()),
            },
        }))
        .expect("launch response");
    session
        .message_response(&serde_json::json!({
            "seq": 63,
            "type": "request",
            "command": "next",
            "arguments": {
                "threadId": 1,
            },
        }))
        .expect("next response");
    let scopes = session
        .message_response(&serde_json::json!({
            "seq": 64,
            "type": "request",
            "command": "scopes",
            "arguments": {
                "frameId": 1,
            },
        }))
        .expect("scopes response");
    let locals_ref = scopes["body"]["scopes"]
        .as_array()
        .expect("scopes")
        .iter()
        .find(|scope| scope["name"] == "Locals")
        .and_then(|scope| scope["variablesReference"].as_u64())
        .expect("locals scope");
    let locals = session
        .message_response(&serde_json::json!({
            "seq": 65,
            "type": "request",
            "command": "variables",
            "arguments": {
                "variablesReference": locals_ref,
            },
        }))
        .expect("locals response");
    let evaluate = session
        .message_response(&serde_json::json!({
            "seq": 66,
            "type": "request",
            "command": "evaluate",
            "arguments": {
                "expression": "doubled",
                "context": "repl",
            },
        }))
        .expect("evaluate response");

    let vars = locals["body"]["variables"].as_array().expect("locals");
    assert!(vars
        .iter()
        .any(|var| var["name"] == "doubled" && var["value"] == "5" && var["type"] == "int"));
    assert_eq!(evaluate["success"], true, "{evaluate}");
    assert_eq!(evaluate["body"]["result"], "5");
    assert_eq!(evaluate["body"]["type"], "int");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn dap_locals_evaluate_array_and_object_initializers() {
    let dir = temp_output_dir("dap-compound-locals");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let source = dir.join("app.orv");
    std::fs::write(
        &source,
        "let xs = [1, 2, 3]\nlet user = { id: 1, name: \"Ada\" }\n",
    )
    .expect("write source");
    let mut session = DapSession::default();

    session
        .message_response(&serde_json::json!({
            "seq": 74,
            "type": "request",
            "command": "launch",
            "arguments": {
                "program": format!("file://{}", source.display()),
            },
        }))
        .expect("launch response");
    session
        .message_response(&serde_json::json!({
            "seq": 75,
            "type": "request",
            "command": "next",
            "arguments": {
                "threadId": 1,
            },
        }))
        .expect("next response");
    let scopes = session
        .message_response(&serde_json::json!({
            "seq": 76,
            "type": "request",
            "command": "scopes",
            "arguments": {
                "frameId": 1,
            },
        }))
        .expect("scopes response");
    let locals_ref = scopes["body"]["scopes"]
        .as_array()
        .expect("scopes")
        .iter()
        .find(|scope| scope["name"] == "Locals")
        .and_then(|scope| scope["variablesReference"].as_u64())
        .expect("locals scope");
    let locals = session
        .message_response(&serde_json::json!({
            "seq": 77,
            "type": "request",
            "command": "variables",
            "arguments": {
                "variablesReference": locals_ref,
            },
        }))
        .expect("locals response");

    let vars = locals["body"]["variables"].as_array().expect("locals");
    assert!(vars
        .iter()
        .any(|var| var["name"] == "xs" && var["value"] == "[1, 2, 3]" && var["type"] == "array"));
    assert!(vars.iter().any(|var| {
        var["name"] == "user"
            && var["value"] == "{ id: 1, name: \"Ada\" }"
            && var["type"] == "object"
    }));
    let _ = std::fs::remove_dir_all(dir);
}
