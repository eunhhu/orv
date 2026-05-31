use std::collections::BTreeSet;
use std::path::PathBuf;
use std::process::Command;

fn temp_output_dir(name: &str) -> PathBuf {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    std::env::temp_dir().join(format!("orv-{name}-{}-{nonce}", std::process::id()))
}

const fn orv_bin() -> &'static str {
    env!("CARGO_BIN_EXE_orv")
}

fn run_orv(args: &[&str]) {
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
}

fn run_orv_json(args: &[&str]) -> serde_json::Value {
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
    serde_json::from_slice(&output.stdout).expect("json stdout")
}

fn assert_keys(value: &serde_json::Value, expected: &[&str], context: &str) {
    let object = value
        .as_object()
        .unwrap_or_else(|| panic!("{context} must be an object"));
    let actual = object.keys().map(String::as_str).collect::<BTreeSet<_>>();
    let expected = expected.iter().copied().collect::<BTreeSet<_>>();
    assert_eq!(actual, expected, "{context} keys drifted");
}

#[test]
fn dap_debug_result_freezes_loaded_source_and_snapshot_nested_shapes() {
    let root = temp_output_dir("dap-debug-nested-contract");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("temp root");
    let source = root.join("app.orv");
    let build_out = root.join("dist");
    std::fs::write(&source, "let total: int = 41\n@out total\n").expect("write source");

    let source_arg = source.display().to_string();
    let build_arg = build_out.display().to_string();
    run_orv(&["build", &source_arg, "--out", &build_arg, "--prod"]);
    let run = run_orv_json(&[
        "editor",
        "run-debug",
        &build_arg,
        "--control",
        "next",
        "--watch-expression",
        "total",
    ]);

    assert_loaded_sources_contract(&run["debug"]["loaded_sources"], "debug.loaded_sources");
    assert_loaded_sources_contract(
        &run["panels"]["debug"]["loaded_sources"],
        "panels.debug.loaded_sources",
    );
    assert_source_snapshots_contract(&run["debug"]["source_snapshots"], "debug.source_snapshots");
    assert_source_snapshots_contract(
        &run["panels"]["debug"]["source_snapshots"],
        "panels.debug.source_snapshots",
    );
    assert_eq!(
        run["panels"]["debug"]["loaded_source_count"],
        serde_json::json!(source_array(&run["debug"]["loaded_sources"]).len())
    );
    assert_eq!(
        run["panels"]["debug"]["source_snapshot_count"],
        serde_json::json!(snapshot_array(&run["debug"]["source_snapshots"]).len())
    );

    let _ = std::fs::remove_dir_all(&root);
}

fn assert_loaded_sources_contract(value: &serde_json::Value, context: &str) {
    assert_keys(value, &["sources"], context);
    let sources = source_array(value);
    assert!(!sources.is_empty(), "{context}.sources must not be empty");
    for (index, source) in sources.iter().enumerate() {
        assert_dap_source_contract(source, &format!("{context}.sources[{index}]"));
    }
}

fn assert_source_snapshots_contract(value: &serde_json::Value, context: &str) {
    let snapshots = snapshot_array(value);
    assert!(!snapshots.is_empty(), "{context} must not be empty");
    for (index, snapshot) in snapshots.iter().enumerate() {
        let context = format!("{context}[{index}]");
        assert_keys(
            snapshot,
            &[
                "checksum",
                "content_length",
                "line_count",
                "request",
                "response",
                "source",
            ],
            &context,
        );
        assert_dap_source_contract(&snapshot["source"], &format!("{context}.source"));
        assert_keys(
            &snapshot["checksum"],
            &["algorithm", "value"],
            &format!("{context}.checksum"),
        );
        assert_keys(
            &snapshot["request"],
            &["arguments", "command", "seq", "type"],
            &format!("{context}.request"),
        );
        assert_keys(
            &snapshot["response"],
            &["body", "command", "request_seq", "seq", "success", "type"],
            &format!("{context}.response"),
        );
    }
}

fn assert_dap_source_contract(source: &serde_json::Value, context: &str) {
    assert_keys(
        source,
        &["checksums", "name", "path", "sourceReference", "uri"],
        context,
    );
    let checksums = source["checksums"]
        .as_array()
        .unwrap_or_else(|| panic!("{context}.checksums must be an array"));
    assert!(
        !checksums.is_empty(),
        "{context}.checksums must not be empty"
    );
    for (index, checksum) in checksums.iter().enumerate() {
        assert_keys(
            checksum,
            &["algorithm", "checksum"],
            &format!("{context}.checksums[{index}]"),
        );
    }
}

fn source_array(value: &serde_json::Value) -> &[serde_json::Value] {
    value["sources"].as_array().expect("sources array")
}

fn snapshot_array(value: &serde_json::Value) -> &[serde_json::Value] {
    value.as_array().expect("source snapshots array")
}
