pub use crate::support::{orv_output as run_orv, temp_dir};
use std::path::Path;
use std::process::Output;

use serde_json::Value;

const CHECK_CLI_GOLDEN: &str = include_str!("../../../../docs/samples/check-cli-v1.golden.json");

pub fn write_source(path: &Path, source: &str) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("create source parent");
    }
    std::fs::write(path, source).expect("write source");
}

pub fn check_cli_golden() -> Value {
    serde_json::from_str(CHECK_CLI_GOLDEN).expect("check CLI golden")
}

pub fn check_cli_success_inventory(output: &Output, entry: &Path) -> Value {
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

pub fn check_cli_entry_diagnostic_inventory(
    output: &Output,
    entry: &Path,
    entry_bad_line: &str,
) -> Value {
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    serde_json::json!({
        "exit_success": output.status.success(),
        "stdout_empty": stdout.is_empty(),
        "stderr": {
            "contains_entry_path": stderr.contains(&entry.display().to_string()),
            "contains_primary_line_column": stderr.contains(":1:16"),
            "contains_entry_source_line": stderr.contains(entry_bad_line),
            "contains_type_mismatch": stderr.contains("type mismatch"),
            "contains_value_label": stderr.contains("value has type `string`"),
            "contains_abort_line": stderr.contains("error: aborting due to previous errors"),
        }
    })
}

pub fn check_cli_imported_diagnostic_inventory(
    output: &Output,
    entry: &Path,
    imported: &Path,
    entry_lookalike_line: &str,
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
            "contains_entry_source_line": stderr.contains(entry_lookalike_line),
            "contains_type_mismatch": stderr.contains("type mismatch"),
            "contains_value_label": stderr.contains("value has type `string`"),
            "contains_abort_line": stderr.contains("error: aborting due to previous errors"),
        }
    })
}

fn normalize_path(text: &str, path: &Path, replacement: &str) -> String {
    text.replace(&path.display().to_string(), replacement)
}

pub fn content_hash(source: &str) -> String {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in source.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("fnv1a64:{hash:016x}")
}
