use std::path::PathBuf;

use crate::common::{run_orv, temp_output_dir, write_prod_server_fixture};

pub(crate) fn build_prod_contract_fixture() -> PathBuf {
    let out = temp_output_dir("deploy-schema-contract");
    let _ = std::fs::remove_dir_all(&out);
    std::fs::create_dir_all(&out).expect("temp output dir");
    let fixture = write_prod_server_fixture(&out);
    let fixture_arg = fixture.display().to_string();
    let out_arg = out.display().to_string();

    run_orv(&["build", &fixture_arg, "--out", &out_arg, "--prod"]);
    run_orv(&["verify-build", &out_arg]);
    run_orv(&["deploy-env-check", &out_arg]);

    out
}
