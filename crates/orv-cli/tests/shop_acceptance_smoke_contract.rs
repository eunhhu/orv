use std::path::{Path, PathBuf};
use std::process::Command;

fn temp_output_dir(name: &str) -> PathBuf {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    std::env::temp_dir().join(format!("orv-{name}-{}-{nonce}", std::process::id()))
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates dir")
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

fn shop_acceptance_script() -> PathBuf {
    workspace_root()
        .join("scripts")
        .join("shop_acceptance_smoke.sh")
}

fn run_smoke(workdir: &Path) -> (String, String) {
    let output = Command::new("sh")
        .arg(shop_acceptance_script())
        .env("ORV_BIN", env!("CARGO_BIN_EXE_orv"))
        .env("ORV_SHOP_ACCEPTANCE_DIR", workdir)
        .output()
        .expect("run shop acceptance smoke");
    assert!(
        output.status.success(),
        "shop acceptance smoke failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    (
        String::from_utf8(output.stdout).expect("utf8 stdout"),
        String::from_utf8(output.stderr).expect("utf8 stderr"),
    )
}

fn parse_stdout_path(stdout: &str, key: &str) -> PathBuf {
    stdout
        .lines()
        .find_map(|line| line.strip_prefix(&format!("{key}=")))
        .map(PathBuf::from)
        .unwrap_or_else(|| panic!("missing {key}= line in stdout:\n{stdout}"))
}

fn read_json(path: &Path) -> serde_json::Value {
    serde_json::from_str(&std::fs::read_to_string(path).expect("read json")).expect("json")
}

#[test]
fn shop_acceptance_smoke_writes_expected_artifacts() {
    // Given a fresh acceptance workdir.
    let workdir = temp_output_dir("shop-acceptance-smoke-contract");
    let _ = std::fs::remove_dir_all(&workdir);
    std::fs::create_dir_all(&workdir).expect("create workdir");

    let result = std::panic::catch_unwind(|| {
        // When the smoke runner is executed with the local orv binary.
        let (stdout, stderr) = run_smoke(&workdir);

        // Then the runner reports success and exposes the artifact paths.
        assert!(stderr.is_empty(), "expected empty stderr, got:\n{stderr}");
        assert!(
            stdout.contains("shop acceptance smoke passed"),
            "missing success marker in stdout:\n{stdout}"
        );

        let smoke_output = parse_stdout_path(&stdout, "smoke_output");
        let benchmark_prepare = parse_stdout_path(&stdout, "benchmark_prepare");
        let benchmark_report = parse_stdout_path(&stdout, "benchmark_report");

        assert!(smoke_output.is_file(), "missing smoke output artifact");
        assert!(
            benchmark_prepare.is_file(),
            "missing benchmark prepare artifact"
        );
        assert!(
            benchmark_report.is_file(),
            "missing benchmark report artifact"
        );

        let report = read_json(&benchmark_report);
        assert_eq!(
            report["status"],
            serde_json::json!("incomplete"),
            "benchmark report should be incomplete before human evidence"
        );
    });

    let _ = std::fs::remove_dir_all(&workdir);
    result.expect("shop acceptance smoke regression");
}
