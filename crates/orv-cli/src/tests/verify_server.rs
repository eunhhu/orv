use super::*;

#[test]
fn verify_artifact_subcommand_is_accepted() {
    let parsed = Cli::try_parse_from([
        "orv",
        "verify-artifact",
        "target/orv-build-test/server/app.orv-runtime.json",
    ]);
    if let Err(err) = parsed {
        panic!("{}", err.render());
    }
}

#[test]
fn verify_build_rejects_server_source_bundle_drift() {
    let (src_dir, path) = prod_server_source("server-source-bundle-source");
    let out = temp_output_dir("server-source-bundle-mismatch");

    cmd_build_with_profile(&path, &out, BuildProfile::Production).expect("prod build");
    let artifact_path = out.join("server").join("app.orv-runtime.json");
    let mut artifact = read_json_value(&artifact_path).expect("server artifact");
    let source_path = artifact["source_bundle"]["files"][0]["path"]
        .as_str()
        .expect("source path")
        .to_string();
    let tampered_source =
        "@server { @listen 8080 @route GET /wrong { @respond 200 { ok: true } } }\n";
    let tampered_bundle = orv_compiler::source_bundle_artifact(
        artifact["entry"].as_str().expect("entry"),
        [(source_path.as_str(), tampered_source)],
    );
    artifact["source_bundle"]["files"][0]["source"] = serde_json::json!(tampered_source);
    artifact["source_bundle"]["files"][0]["content_hash"] =
        serde_json::json!(tampered_bundle.files[0].content_hash.clone());
    write_json(&artifact_path, &artifact).expect("write corrupt server artifact");

    let err = cmd_verify_build(&out).expect_err("server source bundle mismatch");

    assert!(err
        .to_string()
        .contains("does not match build source-bundle artifact"));
    let _ = std::fs::remove_dir_all(src_dir);
    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn verify_build_rejects_source_bundle_duplicate_file_path() {
    let (src_dir, path) = prod_server_source("source-bundle-duplicate-path-source");
    let out = temp_output_dir("source-bundle-duplicate-path");

    cmd_build_with_profile(&path, &out, BuildProfile::Production).expect("prod build");
    let source_bundle_path = out.join("source-bundle.json");
    let mut source_bundle = read_json_value(&source_bundle_path).expect("source bundle");
    let duplicate = source_bundle["files"][0].clone();
    source_bundle["files"]
        .as_array_mut()
        .expect("source bundle files array")
        .push(duplicate);
    write_json(&source_bundle_path, &source_bundle).expect("write drifted source bundle");

    let err = cmd_verify_build(&out).expect_err("duplicate source bundle path must fail");

    assert!(err
        .to_string()
        .contains("source bundle contains duplicate file path"));
    let _ = std::fs::remove_dir_all(src_dir);
    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn verify_build_rejects_server_launcher_listen_mismatch() {
    let path = workspace_path(&["fixtures", "e2e", "hello.orv"]);
    let out = temp_output_dir("server-launch-listen-mismatch");

    cmd_build(&path, &out).expect("build");
    let launch_path = out.join("server").join("launch.json");
    let mut launch = read_json_value(&launch_path).expect("launch");
    launch["listen"]["port"] = serde_json::json!(1234);
    write_json(&launch_path, &launch).expect("write corrupt launch");

    let err = cmd_verify_build(&out).expect_err("listen mismatch");

    assert!(err
        .to_string()
        .contains("server launcher listen does not match runtime artifact"));
    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn verify_build_rejects_invalid_dev_hmr_server_manifest() {
    let out = temp_output_dir("verify-build-dev-hmr-server");
    std::fs::create_dir_all(&out).expect("create temp root");
    let entry = out.join("page.orv");
    std::fs::write(
        &entry,
        "let sig count: int = 0\n@out @html { @body { @p count } }",
    )
    .expect("write entry");
    let build_out = out.join("dist");
    let mut stdout = Vec::new();

    dev_with_writer_with_options(&entry, &build_out, true, true, &mut stdout)
        .expect("dev hmr watch");
    write_dev_watch_events(
        &build_out,
        true,
        1,
        &[dev_watch_loop_event(
            1,
            "initial",
            "build-verify-run",
            "ok",
            Some("sig"),
        )],
    )
    .expect("write events");
    write_dev_hmr_server_manifest(&build_out, "127.0.0.1:1234".parse().expect("addr"))
        .expect("server manifest");
    let server_path = build_out.join("dev").join("server.json");
    let mut server = read_json_value(&server_path).expect("dev hmr server");
    server["endpoints"]["events"] = serde_json::json!("/wrong");
    write_json(&server_path, &server).expect("write corrupt dev hmr server");

    let err = cmd_verify_build(&build_out).expect_err("invalid dev hmr server");

    assert!(err
        .to_string()
        .contains("dev hmr server events endpoint must be /__orv/hmr/events"));
    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn verify_artifact_accepts_generated_server_runtime_artifact() {
    let path = workspace_path(&["fixtures", "e2e", "hello.orv"]);
    let out = temp_output_dir("verify-artifact");

    cmd_build(&path, &out).expect("build artifacts");
    let artifact = out.join("server").join("app.orv-runtime.json");

    cmd_verify_artifact(&artifact).expect("verify artifact");

    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn verify_server_artifact_cases() {
    verify_artifact_cases(
        "verify_server_artifact_cases",
        prod_server_source,
        BuildProfile::Production,
        &[
            json_case(
                "build_manifest_extra_artifact_key",
                "build-manifest.json",
                "build manifest artifact keys must match contract",
                |manifest| {
                    manifest["artifacts"][0]["unexpected"] = serde_json::json!("drift");
                },
            ),
            json_case(
                "build_manifest_artifact_list_drift",
                "build-manifest.json",
                "build manifest artifacts must match bundle plan contract",
                |manifest| {
                    let artifacts = manifest["artifacts"]
                        .as_array_mut()
                        .expect("manifest artifacts");
                    artifacts.retain(|artifact| artifact["kind"] != "source_bundle");
                },
            ),
            json_case(
                "source_bundle_extra_root_key",
                "source-bundle.json",
                "source-bundle.json keys must match contract",
                |source_bundle| {
                    source_bundle["unexpected"] = serde_json::json!(true);
                },
            ),
            json_case(
                "source_bundle_extra_file_key",
                "source-bundle.json",
                "source-bundle.json files[0] keys must match contract",
                |source_bundle| {
                    source_bundle["files"][0]["unexpected"] = serde_json::json!("drift");
                },
            ),
            json_case(
                "source_bundle_content_hash_drift",
                "source-bundle.json",
                "content hash mismatch for",
                |source_bundle| {
                    source_bundle["files"][0]["content_hash"] =
                        serde_json::json!("fnv1a64:0000000000000000");
                },
            ),
            json_case(
                "source_bundle_entry_drift",
                "source-bundle.json",
                "server runtime entry does not match source-bundle artifact",
                |source_bundle| {
                    source_bundle["entry"] = serde_json::json!("wrong.orv");
                },
            ),
        ],
    );
}
