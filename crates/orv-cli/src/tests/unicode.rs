use super::*;

#[test]
fn lsp_unicode_stdio_references_and_rename_use_utf16() {
    let dir = temp_output_dir("lsp-utf16");
    std::fs::create_dir_all(&dir).expect("create test directory");
    let path = dir.join("app.orv");
    let source = "function greet(): string -> \"ok\"\r\n@out \"😀😀😀😀😀😀\"; @out greet()\r\n";
    std::fs::write(&path, source).expect("write source");
    let uri = lsp_file_uri_for_path(&path.canonicalize().expect("canonical path"));
    let requests = [
        serde_json::json!({
            "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {},
        }),
        serde_json::json!({
            "jsonrpc": "2.0", "id": 2, "method": "textDocument/references",
            "params": {
                "textDocument": { "uri": uri },
                "position": { "line": 0, "character": 9 },
                "context": { "includeDeclaration": true },
            },
        }),
        serde_json::json!({
            "jsonrpc": "2.0", "id": 3, "method": "textDocument/rename",
            "params": {
                "textDocument": { "uri": uri },
                "position": { "line": 1, "character": 26 },
                "newName": "welcome",
            },
        }),
    ];
    let input = requests
        .iter()
        .map(|request| {
            let body = request.to_string();
            format!("Content-Length: {}\r\n\r\n{body}", body.len())
        })
        .collect::<String>();
    let output = lsp_stdio_response(&input).expect("serve LSP frames");
    std::fs::remove_dir_all(dir).expect("remove test directory");
    let mut reader = std::io::Cursor::new(output.as_bytes());
    let mut frames = Vec::new();
    while let Some(length) = read_lsp_content_length(&mut reader).expect("frame header") {
        let mut bytes = vec![0; length];
        std::io::Read::read_exact(&mut reader, &mut bytes).expect("frame body");
        frames.push(serde_json::from_slice::<serde_json::Value>(&bytes).expect("JSON frame"));
    }
    assert_eq!(frames.len(), 3);
    let references = frames[1]["result"].as_array().expect("references");
    let usage = references
        .iter()
        .find(|location| location["range"]["start"]["line"] == 1)
        .expect("usage after emoji");
    assert_eq!(usage["range"]["start"]["character"], 26);
    assert_eq!(usage["range"]["end"]["character"], 31);

    let rename = &frames[2];
    assert!(rename.get("error").is_none(), "{rename}");
    let edits = rename["result"]["changes"][&uri]
        .as_array()
        .expect("rename edits");
    assert_eq!(edits.len(), 2);

    // Apply returned edits to UTF-16 buffers, as an editor does, independently
    // of the server's position conversion helpers.
    let mut lines: Vec<Vec<u16>> = source
        .split("\r\n")
        .map(|line| line.encode_utf16().collect())
        .collect();
    for edit in edits {
        let start = &edit["range"]["start"];
        let end = &edit["range"]["end"];
        assert_eq!(start["line"], end["line"]);
        let line = usize::try_from(start["line"].as_u64().expect("line")).unwrap();
        let start = usize::try_from(start["character"].as_u64().expect("start")).unwrap();
        let end = usize::try_from(end["character"].as_u64().expect("end")).unwrap();
        lines[line].splice(start..end, edit["newText"].as_str().unwrap().encode_utf16());
    }
    let renamed = lines
        .iter()
        .map(|line| String::from_utf16(line).expect("valid UTF-16 after rename"))
        .collect::<Vec<_>>()
        .join("\r\n");
    assert_eq!(renamed, source.replace("greet", "welcome"));
}
