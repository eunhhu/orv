use std::fmt::Write as _;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;
use sha2::{Digest, Sha256};

pub const APP_SOURCE: &str = r#"import models.user.user_id

let total: int = user_id()
@out total
"#;

pub const IMPORTED_SOURCE: &str = r#"pub function user_id(): int -> 7
"#;

pub fn expected_sha256(source: &str) -> String {
    format!("{:x}", Sha256::digest(source.as_bytes()))
}

pub fn temp_dir(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time")
        .as_nanos();
    std::env::temp_dir().join(format!("orv-cli-{name}-{}-{nanos}", std::process::id()))
}

const fn orv_bin() -> &'static str {
    env!("CARGO_BIN_EXE_orv")
}

fn run_orv(args: &[&str]) {
    let output = Command::new(orv_bin())
        .args(args)
        .output()
        .expect("run orv");
    assert_success(&output, &format!("orv {args:?}"));
}

pub fn run_orv_json(args: &[&str]) -> Value {
    let output = Command::new(orv_bin())
        .args(args)
        .output()
        .expect("run orv");
    assert_success(&output, &format!("orv {args:?}"));
    serde_json::from_slice(&output.stdout).expect("json stdout")
}

pub fn read_json(path: &Path) -> Value {
    serde_json::from_str(&std::fs::read_to_string(path).expect("read json")).expect("json")
}

pub fn write_json(path: &Path, value: &Value) {
    std::fs::write(
        path,
        serde_json::to_string_pretty(value).expect("serialize json"),
    )
    .expect("write json");
}

pub fn run_orv_failure(args: &[&str]) -> String {
    let output = Command::new(orv_bin())
        .args(args)
        .output()
        .expect("run orv");
    assert!(
        !output.status.success(),
        "orv {args:?} must fail\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn write_fixture(root: &Path) -> (PathBuf, PathBuf) {
    let models = root.join("models");
    std::fs::create_dir_all(&models).expect("create models dir");
    let app = root.join("app.orv");
    let imported = models.join("user.orv");
    std::fs::write(&app, APP_SOURCE).expect("write app source");
    std::fs::write(&imported, IMPORTED_SOURCE).expect("write imported source");
    (app, imported)
}

pub fn build_fixture(root: &Path) -> (PathBuf, PathBuf, PathBuf) {
    let (app, imported) = write_fixture(root);
    let out = root.join("dist");
    let app_arg = app.display().to_string();
    let out_arg = out.display().to_string();
    run_orv(&["build", &app_arg, "--out", &out_arg, "--prod"]);
    (app, imported, out)
}

pub fn assert_source_bundle_files(bundle: &Value) {
    let files = bundle["files"].as_array().expect("source-bundle files");
    assert_eq!(files.len(), 2, "source-bundle file count");
    assert!(files.iter().any(|file| file["path"]
        .as_str()
        .is_some_and(|path| path.ends_with("app.orv"))
        && file["source"] == APP_SOURCE));
    assert!(files.iter().any(|file| {
        file["path"]
            .as_str()
            .is_some_and(|path| path.ends_with("models/user.orv"))
            && file["source"] == IMPORTED_SOURCE
    }));
}

pub fn run_dap_stdio_frames(requests: &[Value]) -> Vec<Value> {
    let mut input = String::new();
    for request in requests {
        let body = serde_json::to_string(request).expect("serialize dap request");
        write!(&mut input, "Content-Length: {}\r\n\r\n{body}", body.len())
            .expect("append dap frame");
    }

    let mut child = Command::new(orv_bin())
        .args(["dap", "serve", "--stdio"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn dap server");
    child
        .stdin
        .take()
        .expect("dap stdin")
        .write_all(input.as_bytes())
        .expect("write dap input");
    let output = child.wait_with_output().expect("wait dap server");
    assert_success(&output, "orv dap serve --stdio");
    protocol_frames(&String::from_utf8(output.stdout).expect("dap stdout utf8"))
}

fn protocol_frames(output: &str) -> Vec<Value> {
    let mut rest = output;
    let mut frames = Vec::new();
    while !rest.is_empty() {
        let (header, body_start) = rest.split_once("\r\n\r\n").expect("dap frame header");
        let content_length = header
            .lines()
            .find_map(|line| {
                let (name, value) = line.split_once(':')?;
                name.eq_ignore_ascii_case("Content-Length")
                    .then(|| value.trim().parse::<usize>().expect("content length"))
            })
            .expect("content length header");
        let body = body_start
            .get(..content_length)
            .unwrap_or_else(|| panic!("truncated dap frame body: {body_start}"));
        frames.push(serde_json::from_str(body).expect("dap frame json"));
        rest = &body_start[content_length..];
    }
    frames
}

pub fn response<'a>(frames: &'a [Value], command: &str, request_seq: u64) -> &'a Value {
    frames
        .iter()
        .find(|frame| {
            frame["type"] == "response"
                && frame["command"] == command
                && frame["request_seq"] == request_seq
        })
        .unwrap_or_else(|| panic!("missing {command} response for request {request_seq}"))
}

pub fn assert_loaded_source(loaded: &Value, name: &str, expected_source: &str) {
    let sources = loaded["body"]["sources"]
        .as_array()
        .expect("loaded sources");
    assert_loaded_source_inventory(sources, name, expected_source);
}

pub fn assert_loaded_source_inventory(sources: &[Value], name: &str, expected_source: &str) {
    let source = sources
        .iter()
        .find(|source| source["name"] == name)
        .unwrap_or_else(|| panic!("missing loaded source {name}"));
    assert!(source["sourceReference"]
        .as_u64()
        .is_some_and(|source_reference| source_reference > 0));
    let checksums = source["checksums"].as_array().expect("source checksums");
    assert!(!checksums.is_empty(), "source checksums must not be empty");

    let expected_checksum = expected_sha256(expected_source);
    assert!(
        checksums.iter().any(|checksum| {
            checksum["algorithm"] == "SHA256" && checksum["checksum"] == expected_checksum
        }),
        "missing SHA256 checksum for {name}; expected {expected_checksum}, got {checksums:?}"
    );
}

pub fn assert_source_responses(responses: [&Value; 2]) {
    for response in responses {
        assert_eq!(response["success"], true, "{response}");
        assert_eq!(response["body"]["mimeType"], "text/x-orv");
    }
    let contents = responses
        .iter()
        .map(|response| response["body"]["content"].clone())
        .collect::<Vec<_>>();
    assert!(contents.contains(&Value::String(APP_SOURCE.to_string())));
    assert!(contents.contains(&Value::String(IMPORTED_SOURCE.to_string())));
}

fn assert_success(output: &Output, label: &str) {
    assert!(
        output.status.success(),
        "{label} failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
