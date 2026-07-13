//! Shared fixture/build/json helpers used by both the source-bundle contract
//! target and the summary-parity contract target. DAP stdio helpers that only
//! the source-bundle contract target consumes live in `dap_support.rs`.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;

pub const APP_SOURCE: &str = r"import models.user.user_id

let total: int = user_id()
@out total
";

pub const IMPORTED_SOURCE: &str = r"pub function user_id(): int -> 7
";

pub fn temp_dir(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time")
        .as_nanos();
    std::env::temp_dir().join(format!("orv-cli-{name}-{}-{nanos}", std::process::id()))
}

pub const fn orv_bin() -> &'static str {
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

pub fn assert_success(output: &Output, label: &str) {
    assert!(
        output.status.success(),
        "{label} failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
