use crate::support::{run_orv, temp_dir as temp_output_dir};
use std::path::{Path, PathBuf};

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
fn deploy_runbook_documents_participant_raw_notes_identity_gate() {
    let out = temp_output_dir("deploy-runbook-benchmark-contract");
    let _ = std::fs::remove_dir_all(&out);
    std::fs::create_dir_all(&out).expect("temp output dir");
    let fixture = write_prod_server_fixture(&out);
    let fixture_arg = fixture.display().to_string();
    let out_arg = out.display().to_string();

    run_orv(&["build", &fixture_arg, "--out", &out_arg, "--prod"]);
    run_orv(&["verify-build", &out_arg]);

    let runbook =
        std::fs::read_to_string(out.join("deploy").join("README.md")).expect("read deploy runbook");
    assert!(runbook.contains("the copied value must match the retained"));
    assert!(runbook.contains("benchmark report fails"));
    assert!(runbook.contains("empty Task Notes"));
    assert!(runbook.contains("generated template instruction prose"));
    assert!(runbook.contains("duplicate identity fields"));
    assert!(runbook.contains("non-exact-once participant_id/run_id identity"));

    let _ = std::fs::remove_dir_all(&out);
}
