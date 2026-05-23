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

    let _ = std::fs::remove_dir_all(root);
}

fn assert_origin_calls(origin_map: &serde_json::Value, call_name: &str, function_name: &str) {
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
    assert!(
        origin_map["edges"]
            .as_array()
            .expect("origin edges")
            .iter()
            .any(|edge| edge["kind"] == "calls"
                && edge["to"] == function_id
                && edge["from"]
                    .as_str()
                    .is_some_and(|from| call_ids.contains(from))),
        "missing calls edge from {call_name} to {function_name}: {origin_map}"
    );
}
