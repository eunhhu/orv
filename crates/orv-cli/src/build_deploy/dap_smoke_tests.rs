use super::*;

#[test]
fn build_prod_smoke_dap_source_bundle_hash_uses_artifact_hash() {
    // Given: a production build with a source bundle and generated smoke script.
    let (src_dir, path) = prod_server_source("deploy-smoke-dap-source-hash-source");
    let out = temp_output_dir("deploy-smoke-dap-source-hash");

    // When: reading the generated deploy smoke script.
    cmd_build_with_profile(&path, &out, BuildProfile::Production).expect("prod build");
    let smoke =
        std::fs::read_to_string(out.join("deploy").join("smoke-test.sh")).expect("smoke test");
    let source_bundle = read_json_value(&out.join(SOURCE_BUNDLE_PATH)).expect("source bundle");
    let source_bundle_hash = stable_json_hash(&source_bundle).expect("source bundle hash");

    // Then: the DAP panel hash check is pinned to the actual source-bundle artifact hash.
    assert!(smoke.contains(&dap_source_bundle_panel_hash_smoke_check(
        &source_bundle_hash
    )));

    let _ = std::fs::remove_dir_all(src_dir);
    let _ = std::fs::remove_dir_all(out);
}

#[test]
fn verify_build_rejects_deploy_smoke_dap_source_bundle_hash_mismatch() {
    // Given: a generated smoke script whose DAP source-bundle panel hash check drifted.
    let (src_dir, path) = prod_server_source("deploy-smoke-dap-source-hash-mismatch-source");
    let out = temp_output_dir("deploy-smoke-dap-source-hash-mismatch");

    cmd_build_with_profile(&path, &out, BuildProfile::Production).expect("prod build");
    let smoke_path = out.join("deploy").join("smoke-test.sh");
    let smoke = std::fs::read_to_string(&smoke_path).expect("smoke test");
    let source_bundle = read_json_value(&out.join(SOURCE_BUNDLE_PATH)).expect("source bundle");
    let source_bundle_hash = stable_json_hash(&source_bundle).expect("source bundle hash");
    let expected_check = dap_source_bundle_panel_hash_smoke_check(&source_bundle_hash);
    let loose_check = r#"orv_smoke_dap_summary_contains "dap source bundle panel hash" '"hash":"#;
    let wrong_check = r#"orv_smoke_dap_summary_contains "dap source bundle panel hash" '"hash": "0000000000000000"'"#;
    let drifted_smoke = if smoke.contains(&expected_check) {
        smoke.replace(&expected_check, wrong_check)
    } else {
        smoke.replace(loose_check, wrong_check)
    };
    std::fs::write(&smoke_path, drifted_smoke).expect("write corrupt smoke test");

    // When: verifying the build artifacts.
    let err = cmd_verify_build(&out).expect_err("smoke DAP source bundle hash mismatch");

    // Then: verify-build rejects the hash drift instead of accepting a generic hash field check.
    assert!(err
        .to_string()
        .contains("deploy smoke test must verify the build graph contract"));
    let _ = std::fs::remove_dir_all(src_dir);
    let _ = std::fs::remove_dir_all(out);
}

fn dap_source_bundle_panel_hash_smoke_check(hash: &str) -> String {
    format!(r#"orv_smoke_dap_summary_contains "dap source bundle panel hash" '"hash": "{hash}"'"#)
}

fn prod_server_source(name: &str) -> (PathBuf, PathBuf) {
    let dir = temp_output_dir(name);
    std::fs::create_dir_all(&dir).expect("create prod source dir");
    let path = dir.join("app.orv");
    std::fs::write(
        &path,
        "@server { @listen 8080 @route GET /ping { @respond 200 { ok: true } } }\n",
    )
    .expect("write prod source");
    (dir, path)
}

fn temp_output_dir(name: &str) -> PathBuf {
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    std::env::temp_dir().join(format!("orv-{name}-{unique}"))
}
