use crate::support::{orv_output as run_orv, run_orv_json, temp_dir};
use std::collections::BTreeSet;
use std::path::Path;
use std::process::Output;

use serde_json::Value;

const COMPILER_PIPELINE_GOLDEN: &str =
    include_str!("../../../docs/samples/compiler-pipeline-v1.golden.json");

fn write_source(path: &Path, source: &str) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("create source parent");
    }
    std::fs::write(path, source).expect("write source");
}

#[test]
fn compiler_pipeline_v1_resolves_hoisted_functions_and_lexical_shadowing() {
    let root = temp_dir("compiler-pipeline-success");
    let entry = root.join("app.orv");
    write_source(
        &entry,
        r"function twice(x: int): int -> add(x, x)
function add(a: int, b: int): int -> a + b
let x: int = 4
{ let x: int = 10
@out twice(x) }
@out twice(x)
",
    );
    let entry_arg = entry.display().to_string();

    let check = run_orv(&["check", &entry_arg]);
    assert!(
        check.status.success(),
        "orv check failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&check.stdout),
        String::from_utf8_lossy(&check.stderr)
    );
    assert!(check.stderr.is_empty());

    let run = run_orv(&["run", &entry_arg]);
    assert!(
        run.status.success(),
        "orv run failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&run.stdout), "20\n8\n");
    assert!(run.stderr.is_empty());

    let origins = run_orv_json(&["origins", &entry_arg]);
    assert_origin_calls(&origins, "add", "add");
    assert_origin_calls(&origins, "twice", "twice");
    assert_eq!(
        compiler_pipeline_success_inventory(&check, &run, &origins, &entry),
        compiler_pipeline_golden()["success"],
        "Compiler Pipeline v1 success golden drift"
    );

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn compiler_pipeline_v1_reports_resolve_and_hir_analysis_failures() {
    let root = temp_dir("compiler-pipeline-failures");
    let out_of_scope = root.join("out_of_scope.orv");
    write_source(&out_of_scope, "for i in 0..1 { @out i }\n@out i\n");
    let out_of_scope_arg = out_of_scope.display().to_string();

    let resolve = run_orv(&["check", &out_of_scope_arg]);
    assert!(
        !resolve.status.success(),
        "out-of-scope binding must fail check"
    );
    let resolve_stderr = String::from_utf8_lossy(&resolve.stderr);
    assert!(resolve.stdout.is_empty());
    assert!(
        resolve_stderr.contains("undefined variable `i`"),
        "{resolve_stderr}"
    );
    assert!(resolve_stderr.contains("@out i"), "{resolve_stderr}");
    assert!(
        resolve_stderr.contains("error: aborting due to previous errors"),
        "{resolve_stderr}"
    );

    let type_mismatch = root.join("type_mismatch.orv");
    write_source(
        &type_mismatch,
        "function add(a: int, b: int): int -> a + b\nlet bad: int = add(1, \"two\")\n",
    );
    let type_mismatch_arg = type_mismatch.display().to_string();

    let analyze = run_orv(&["check", &type_mismatch_arg]);
    assert!(
        !analyze.status.success(),
        "HIR analysis type mismatch must fail check"
    );
    let analyze_stderr = String::from_utf8_lossy(&analyze.stderr);
    assert!(analyze.stdout.is_empty());
    assert!(
        analyze_stderr.contains("type mismatch: `add` arg #2 expects `int` but got `string`"),
        "{analyze_stderr}"
    );
    assert!(analyze_stderr.contains("\"two\""), "{analyze_stderr}");
    assert!(
        analyze_stderr.contains("error: aborting due to previous errors"),
        "{analyze_stderr}"
    );
    assert_eq!(
        compiler_pipeline_failure_inventory(&resolve, &analyze),
        compiler_pipeline_golden()["failures"],
        "Compiler Pipeline v1 failure golden drift"
    );

    let _ = std::fs::remove_dir_all(root);
}

fn assert_origin_calls(origin_map: &serde_json::Value, call_name: &str, function_name: &str) {
    assert!(
        origin_call_edge_present(origin_map, call_name, function_name),
        "missing calls edge from {call_name} to {function_name}: {origin_map}"
    );
}

fn origin_call_edge_present(
    origin_map: &serde_json::Value,
    call_name: &str,
    function_name: &str,
) -> bool {
    let entries = origin_map["entries"].as_array().expect("origin entries");
    let call_ids = entries
        .iter()
        .filter(|entry| entry["kind"] == "call" && entry["name"] == call_name)
        .filter_map(|entry| entry["id"].as_str())
        .collect::<BTreeSet<_>>();
    let function_id = entries
        .iter()
        .find(|entry| entry["kind"] == "function" && entry["name"] == function_name)
        .and_then(|entry| entry["id"].as_str())
        .unwrap_or_else(|| panic!("missing function origin {function_name}"));
    assert!(
        !call_ids.is_empty(),
        "missing call origin for {call_name}: {origin_map}"
    );
    origin_map["edges"]
        .as_array()
        .expect("origin edges")
        .iter()
        .any(|edge| {
            edge["kind"] == "calls"
                && edge["to"] == function_id
                && edge["from"]
                    .as_str()
                    .is_some_and(|from| call_ids.contains(from))
        })
}

fn compiler_pipeline_golden() -> Value {
    serde_json::from_str(COMPILER_PIPELINE_GOLDEN).expect("compiler pipeline golden")
}

fn compiler_pipeline_success_inventory(
    check: &Output,
    run: &Output,
    origins: &Value,
    entry: &Path,
) -> Value {
    serde_json::json!({
        "check": {
            "exit_success": check.status.success(),
            "stdout": normalize_path(&String::from_utf8_lossy(&check.stdout), entry, "<entry>"),
            "stderr_empty": check.stderr.is_empty(),
        },
        "run": {
            "exit_success": run.status.success(),
            "stdout": String::from_utf8_lossy(&run.stdout),
            "stderr_empty": run.stderr.is_empty(),
        },
        "origin_calls": [
            {
                "call": "add",
                "function": "add",
                "edge_present": origin_call_edge_present(origins, "add", "add"),
            },
            {
                "call": "twice",
                "function": "twice",
                "edge_present": origin_call_edge_present(origins, "twice", "twice"),
            },
        ],
    })
}

fn compiler_pipeline_failure_inventory(resolve: &Output, analyze: &Output) -> Value {
    let resolve_stderr = String::from_utf8_lossy(&resolve.stderr);
    let analyze_stderr = String::from_utf8_lossy(&analyze.stderr);
    serde_json::json!({
        "resolve": {
            "exit_success": resolve.status.success(),
            "stdout_empty": resolve.stdout.is_empty(),
            "stderr": {
                "contains_undefined_i": resolve_stderr.contains("undefined variable `i`"),
                "contains_source_line": resolve_stderr.contains("@out i"),
                "contains_abort_line": resolve_stderr.contains("error: aborting due to previous errors"),
            }
        },
        "analyze": {
            "exit_success": analyze.status.success(),
            "stdout_empty": analyze.stdout.is_empty(),
            "stderr": {
                "contains_arg_type_mismatch": analyze_stderr.contains("type mismatch: `add` arg #2 expects `int` but got `string`"),
                "contains_source_literal": analyze_stderr.contains("\"two\""),
                "contains_abort_line": analyze_stderr.contains("error: aborting due to previous errors"),
            }
        },
    })
}

fn normalize_path(text: &str, path: &Path, replacement: &str) -> String {
    text.replace(&path.display().to_string(), replacement)
}
