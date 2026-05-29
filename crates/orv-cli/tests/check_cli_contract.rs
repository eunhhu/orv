use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;

const CHECK_CLI_GOLDEN: &str = include_str!("../../../docs/samples/check-cli-v1.golden.json");

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
fn check_cli_v1_freezes_success_envelope() {
    let root = temp_dir("check-cli-success");
    let entry = root.join("main.orv");
    write_source(&entry, "let ok: int = 1\n@out ok\n");
    let entry_arg = entry.display().to_string();

    let output = run_orv(&["check", &entry_arg]);

    assert!(
        output.status.success(),
        "orv check failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        format!("check: {} passed\n", entry.display())
    );
    assert!(output.stderr.is_empty());
    assert_eq!(
        check_cli_success_inventory(&output, &entry),
        check_cli_golden()["success"],
        "Check CLI v1 success golden drift"
    );

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn check_cli_v1_routes_imported_file_diagnostics_to_imported_source() {
    let root = temp_dir("check-cli-imported-diagnostic");
    let entry = root.join("main.orv");
    let imported = root.join("models").join("user.orv");
    write_source(&entry, "import models.user.User\nlet ok: int = 1\n");
    write_source(
        &imported,
        "pub struct User { id: int }\nlet bad: int = \"wrong\"\n",
    );
    let entry_arg = entry.display().to_string();

    let output = run_orv(&["check", &entry_arg]);

    assert!(
        !output.status.success(),
        "invalid imported source must fail check"
    );
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains(&imported.display().to_string()), "{stderr}");
    assert!(stderr.contains("let bad: int = \"wrong\""), "{stderr}");
    assert!(stderr.contains("value has type `string`"), "{stderr}");
    assert!(
        stderr.contains("error: aborting due to previous errors"),
        "{stderr}"
    );
    assert!(!stderr.contains("let ok: int = 1"), "{stderr}");
    assert_eq!(
        check_cli_imported_diagnostic_inventory(&output, &entry, &imported),
        check_cli_golden()["imported_diagnostic"],
        "Check CLI v1 imported diagnostic golden drift"
    );

    let _ = std::fs::remove_dir_all(root);
}

fn check_cli_golden() -> Value {
    serde_json::from_str(CHECK_CLI_GOLDEN).expect("check CLI golden")
}

fn check_cli_success_inventory(output: &Output, entry: &Path) -> Value {
    serde_json::json!({
        "exit_success": output.status.success(),
        "stdout": normalize_path(
            &String::from_utf8_lossy(&output.stdout),
            entry,
            "<entry>"
        ),
        "stderr_empty": output.stderr.is_empty(),
    })
}

fn check_cli_imported_diagnostic_inventory(
    output: &Output,
    entry: &Path,
    imported: &Path,
) -> Value {
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    serde_json::json!({
        "exit_success": output.status.success(),
        "stdout_empty": stdout.is_empty(),
        "stderr": {
            "contains_imported_path": stderr.contains(&imported.display().to_string()),
            "contains_entry_path": stderr.contains(&entry.display().to_string()),
            "contains_primary_line_column": stderr.contains(":2:16"),
            "contains_imported_source_line": stderr.contains("let bad: int = \"wrong\""),
            "contains_entry_source_line": stderr.contains("let ok: int = 1"),
            "contains_type_mismatch": stderr.contains("type mismatch"),
            "contains_value_label": stderr.contains("value has type `string`"),
            "contains_abort_line": stderr.contains("error: aborting due to previous errors"),
        }
    })
}

fn normalize_path(text: &str, path: &Path, replacement: &str) -> String {
    text.replace(&path.display().to_string(), replacement)
}
