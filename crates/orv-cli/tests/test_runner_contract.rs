use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_dir(name: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    std::env::temp_dir().join(format!("orv-{name}-{}-{nonce}", std::process::id()))
}

const fn orv_bin() -> &'static str {
    env!("CARGO_BIN_EXE_orv")
}

fn run_orv(args: &[&str]) -> Output {
    Command::new(orv_bin())
        .args(args)
        .output()
        .expect("run orv")
}

fn run_orv_json(args: &[&str]) -> serde_json::Value {
    let output = run_orv(args);
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

fn write_source(path: &Path, source: &str) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("create source parent");
    }
    std::fs::write(path, source).expect("write source");
}

#[test]
fn test_runner_v1_freezes_discovery_json_and_filter_semantics() {
    let root = temp_dir("test-runner-contract-list");
    let first = root.join("a_checkout_test.orv");
    let second = root.join("nested").join("b_math_test.orv");
    write_source(
        &first,
        r#"test "checkout shows cart" {
  assert true
}

test "checkout excluded failure" {
  assert false
}
"#,
    );
    write_source(
        &second,
        r#"test "math adds" {
  assert 1 + 2 == 3
}
"#,
    );

    let root_arg = root.display().to_string();
    let all = run_orv_json(&["test", &root_arg, "--list"]);
    assert_keys(&all, &["schema_version", "tests"], "test list root");
    assert_eq!(all["schema_version"], serde_json::json!(1));
    let tests = all["tests"].as_array().expect("tests array");
    assert_eq!(tests.len(), 3);
    assert_eq!(tests[0]["name"], serde_json::json!("checkout shows cart"));
    assert_eq!(
        tests[1]["name"],
        serde_json::json!("checkout excluded failure")
    );
    assert_eq!(tests[2]["name"], serde_json::json!("math adds"));

    let filtered = run_orv_json(&["test", &root_arg, "--filter", "shows", "--list"]);
    assert_keys(
        &filtered,
        &["schema_version", "tests"],
        "filtered test list root",
    );
    assert_eq!(filtered["schema_version"], serde_json::json!(1));
    let filtered_tests = filtered["tests"].as_array().expect("filtered tests array");
    assert_eq!(filtered_tests.len(), 1);
    let test = &filtered_tests[0];
    assert_keys(
        test,
        &["column", "line", "name", "path", "range", "span"],
        "test entry",
    );
    assert_eq!(test["path"], serde_json::json!(first.display().to_string()));
    assert_eq!(test["name"], serde_json::json!("checkout shows cart"));
    assert_eq!(test["line"], serde_json::json!(1));
    assert_eq!(test["column"], serde_json::json!(1));
    assert_keys(&test["span"], &["end", "start"], "test span");
    assert_eq!(test["span"]["start"], serde_json::json!(0));
    assert!(test["span"]["end"].as_u64().is_some_and(|end| end > 0));
    assert_keys(&test["range"], &["end", "start"], "test range");
    assert_keys(
        &test["range"]["start"],
        &["character", "line"],
        "range start",
    );
    assert_keys(&test["range"]["end"], &["character", "line"], "range end");
    assert_eq!(test["range"]["start"]["line"], serde_json::json!(0));
    assert_eq!(test["range"]["start"]["character"], serde_json::json!(0));
    assert_eq!(test["range"]["end"]["line"], serde_json::json!(2));

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn test_runner_v1_freezes_execution_summary_and_failure_envelope() {
    let root = temp_dir("test-runner-contract-run");
    let source = root.join("runner_test.orv");
    write_source(
        &source,
        r#"test "passes selected" {
  assert true
}

test "fails selected" {
  assert false
}
"#,
    );
    let source_arg = source.display().to_string();

    let passed = run_orv(&["test", &source_arg, "--filter", "passes"]);
    assert!(
        passed.status.success(),
        "orv test success path failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&passed.stdout),
        String::from_utf8_lossy(&passed.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&passed.stdout), "test: 1 passed\n");
    assert!(passed.stderr.is_empty());

    let failed = run_orv(&["test", &source_arg, "--filter", "fails"]);
    assert!(!failed.status.success(), "failing test must exit non-zero");
    assert!(failed.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&failed.stderr);
    assert!(stderr.starts_with("error: test: "));
    assert!(stderr.contains("runner_test.orv"));
    assert!(stderr.contains("`fails selected` failed"));
    assert!(stderr.contains("assertion failed"));

    let _ = std::fs::remove_dir_all(root);
}
