use std::path::{Path, PathBuf};
use std::process::{Command, Output};

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
    let status = Command::new(orv_bin())
        .args(args)
        .status()
        .expect("run orv");
    assert!(status.success(), "orv {args:?} failed with {status}");
}

fn orv_output(args: &[&str]) -> Output {
    Command::new(orv_bin())
        .args(args)
        .output()
        .expect("run orv")
}

fn write_prod_server_fixture(out: &Path) -> PathBuf {
    let fixture = out.join("app.orv");
    std::fs::write(
        &fixture,
        r"@server {
  @listen 8080

  @route GET /ping {
    @respond 200 { ok: true }
  }
}
",
    )
    .expect("write fixture");
    fixture
}

#[test]
fn benchmark_prepare_handoff_names_exact_once_raw_notes_identity_rule() {
    let out = temp_output_dir("benchmark-prepare-handoff-contract");
    let _ = std::fs::remove_dir_all(&out);
    std::fs::create_dir_all(&out).expect("temp output dir");
    let fixture = write_prod_server_fixture(&out);
    let fixture_arg = fixture.display().to_string();
    let out_arg = out.display().to_string();

    run_orv(&["build", &fixture_arg, "--out", &out_arg, "--prod"]);
    let output = orv_output(&["benchmark-prepare", &out_arg, "--participants", "2"]);

    assert!(
        output.status.success(),
        "benchmark-prepare failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let prepared: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("benchmark prepare json");
    let raw_notes_rule = prepared["recording_handoff"]["raw_notes_rule"]
        .as_str()
        .expect("raw notes rule");
    assert!(raw_notes_rule.contains("retained non-empty relative file"));
    assert!(raw_notes_rule.contains("Task Notes"));
    assert!(raw_notes_rule.contains("participant-specific observations"));
    assert!(raw_notes_rule.contains("deploy/smoke-output.txt"));
    assert!(raw_notes_rule.contains("benchmark report fails"));
    assert!(raw_notes_rule.contains("exactly once"));
    assert!(raw_notes_rule.contains("participant_id"));
    assert!(raw_notes_rule.contains("run_id"));

    let _ = std::fs::remove_dir_all(&out);
}
