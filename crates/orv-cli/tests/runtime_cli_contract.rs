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

fn write_source(path: &Path, source: &str) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("create source parent");
    }
    std::fs::write(path, source).expect("write source");
}

#[test]
fn runtime_cli_v1_freezes_foreground_success_output() {
    let root = temp_dir("runtime-cli-success");
    let source = root.join("app.orv");
    write_source(
        &source,
        r#"let name: string = "Ada"
@out "hello {name}"
@out 1 + 2
@out true
"#,
    );
    let source_arg = source.display().to_string();

    let output = run_orv(&["run", &source_arg]);

    assert!(
        output.status.success(),
        "orv run failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "hello Ada\n3\ntrue\n"
    );
    assert!(output.stderr.is_empty());

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn runtime_cli_v1_freezes_runtime_failure_envelope() {
    let root = temp_dir("runtime-cli-failure");
    let source = root.join("app.orv");
    write_source(
        &source,
        r#"@out "before"
assert false
@out "after"
"#,
    );
    let source_arg = source.display().to_string();

    let output = run_orv(&["run", &source_arg]);

    assert!(
        !output.status.success(),
        "runtime failure must exit non-zero"
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "before\n");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.starts_with("error: "));
    assert!(stderr.contains("assertion failed"));
    assert!(!stderr.contains("after"));

    let _ = std::fs::remove_dir_all(root);
}
