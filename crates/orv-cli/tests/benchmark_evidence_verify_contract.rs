use crate::support::{orv_bin, read_json, run_orv, temp_dir as temp_output_dir};
use std::path::{Path, PathBuf};
use std::process::Command;

fn orv_output(args: &[&str]) -> std::process::Output {
    Command::new(orv_bin())
        .args(args)
        .output()
        .expect("run orv")
}

fn write_json(path: &Path, value: &serde_json::Value) {
    let content = serde_json::to_string_pretty(value).expect("serialize json");
    std::fs::write(path, content).expect("write json");
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
fn verify_build_rejects_blank_recorded_benchmark_task_notes() {
    let out = temp_output_dir("benchmark-evidence-blank-task-notes");
    let _ = std::fs::remove_dir_all(&out);
    std::fs::create_dir_all(&out).expect("temp output dir");
    let fixture = write_prod_server_fixture(&out);
    let fixture_arg = fixture.display().to_string();
    let out_arg = out.display().to_string();

    run_orv(&["build", &fixture_arg, "--out", &out_arg, "--prod"]);
    let evidence_path = out.join("deploy").join("benchmark-evidence.json");
    let mut evidence = read_json(&evidence_path);
    evidence["task_entries"][0]["elapsed_minutes"] = serde_json::json!(12.5);
    evidence["task_entries"][0]["status"] = serde_json::json!("recorded");
    evidence["task_entries"][0]["notes"] = serde_json::json!("   ");
    write_json(&evidence_path, &evidence);

    let output = orv_output(&["verify-build", &out_arg]);

    assert!(!output.status.success(), "blank recorded notes must fail");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("deploy benchmark evidence task_entries[0] notes must not be blank"),
        "{stderr}"
    );
    let _ = std::fs::remove_dir_all(&out);
}
