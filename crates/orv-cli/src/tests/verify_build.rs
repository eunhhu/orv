use super::*;

#[test]
fn verify_build_subcommand_is_accepted() {
    let parsed = Cli::try_parse_from(["orv", "verify-build", "target/orv-build-test"]);
    if let Err(err) = parsed {
        panic!("{}", err.render());
    }
}

#[test]
fn verify_build_rejects_bundle_plan_and_manifest_paired_drift() {
    let (src_dir, path) = prod_server_source("bundle-plan-paired-drift-source");
    let out = temp_output_dir("bundle-plan-paired-drift");

    cmd_build(&path, &out).expect("build artifacts");
    let plan_path = out.join("bundle-plan.json");
    let mut plan = read_json_value(&plan_path).expect("bundle plan");
    plan["bundles"]
        .as_array_mut()
        .expect("bundle targets")
        .retain(|target| target["kind"] != "server_runtime");
    write_json(&plan_path, &plan).expect("write drifted bundle plan");
    let manifest_path = out.join("build-manifest.json");
    let mut manifest = read_json_value(&manifest_path).expect("build manifest");
    manifest["artifacts"]
        .as_array_mut()
        .expect("manifest artifacts")
        .retain(|artifact| artifact["kind"] != "server_runtime");
    write_json(&manifest_path, &manifest).expect("write drifted build manifest");

    let err = cmd_verify_build(&out).expect_err("paired bundle/manifest drift must fail");

    assert!(err
        .to_string()
        .contains("bundle plan does not match origin-map contract"));
    let _ = std::fs::remove_dir_all(src_dir);
    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn verify_build_rejects_invalid_dev_hmr_session_manifest() {
    let out = temp_output_dir("verify-build-dev-hmr-session");
    std::fs::create_dir_all(&out).expect("create temp root");
    let entry = out.join("page.orv");
    std::fs::write(
        &entry,
        "let sig count: int = 0\n@out @html { @body { @p count } }",
    )
    .expect("write entry");
    let build_out = out.join("dist");
    let mut stdout = Vec::new();

    dev_with_writer_with_options(&entry, &build_out, true, false, &mut stdout).expect("dev hmr");
    let session_path = build_out.join("dev").join("session.json");
    let mut session = read_json_value(&session_path).expect("dev session");
    session["watch"]["targets"] = serde_json::Value::Array(
        session["watch"]["targets"]
            .as_array()
            .expect("targets")
            .iter()
            .filter(|target| target["kind"] != "client_wasm")
            .cloned()
            .collect(),
    );
    write_json(&session_path, &session).expect("write corrupt dev session");

    let err = cmd_verify_build(&build_out).expect_err("invalid dev hmr session");

    assert!(err
        .to_string()
        .contains("dev session missing bundle target client_wasm:client/app.wasm"));
    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn verify_build_rejects_invalid_dev_hmr_transport_manifest() {
    let out = temp_output_dir("verify-build-dev-hmr-transport");
    std::fs::create_dir_all(&out).expect("create temp root");
    let entry = out.join("page.orv");
    std::fs::write(
        &entry,
        "let sig count: int = 0\n@out @html { @body { @p count } }",
    )
    .expect("write entry");
    let build_out = out.join("dist");
    let mut stdout = Vec::new();

    dev_with_writer_with_options(&entry, &build_out, true, false, &mut stdout).expect("dev hmr");
    let transport_path = build_out.join("dev").join("transport.json");
    let mut transport = read_json_value(&transport_path).expect("dev hmr transport");
    transport["browser"]["client"] = serde_json::json!("tmp/hmr-client.js");
    write_json(&transport_path, &transport).expect("write corrupt dev hmr transport");

    let err = cmd_verify_build(&build_out).expect_err("invalid dev hmr transport");

    assert!(err
        .to_string()
        .contains("dev hmr transport browser client must be dev/hmr-client.js"));
    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn verify_build_rejects_invalid_dev_watch_session_manifest() {
    let out = temp_output_dir("verify-build-dev-watch-session");
    std::fs::create_dir_all(&out).expect("create temp root");
    let entry = out.join("page.orv");
    std::fs::write(&entry, "@out @html { @body { @h1 \"Watch\" } }").expect("write entry");
    let build_out = out.join("dist");
    let mut stdout = Vec::new();

    dev_with_writer_with_options(&entry, &build_out, false, true, &mut stdout).expect("dev watch");
    let session_path = build_out.join("dev").join("watch.json");
    let mut session = read_json_value(&session_path).expect("dev watch session");
    session["loop"]["interval_ms"] = serde_json::json!(0);
    write_json(&session_path, &session).expect("write corrupt dev watch session");

    let err = cmd_verify_build(&build_out).expect_err("invalid dev watch session");

    assert!(err
        .to_string()
        .contains("dev watch session loop interval_ms must be positive"));
    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn verify_build_rejects_invalid_dev_watch_transport_path() {
    let out = temp_output_dir("verify-build-dev-watch-transport");
    std::fs::create_dir_all(&out).expect("create temp root");
    let entry = out.join("page.orv");
    std::fs::write(&entry, "@out @html { @body { @h1 \"Watch\" } }").expect("write entry");
    let build_out = out.join("dist");
    let mut stdout = Vec::new();

    dev_with_writer_with_options(&entry, &build_out, false, true, &mut stdout).expect("dev watch");
    let session_path = build_out.join("dev").join("watch.json");
    let mut session = read_json_value(&session_path).expect("dev watch session");
    session["transport"]["path"] = serde_json::json!("tmp/watch.json");
    write_json(&session_path, &session).expect("write corrupt dev watch session");

    let err = cmd_verify_build(&build_out).expect_err("invalid dev watch transport");

    assert!(err
        .to_string()
        .contains("dev watch session transport path must be dev/watch.json"));
    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn verify_build_artifact_cases() {
    verify_artifact_cases(
        "verify_build_artifact_cases",
        prod_server_source,
        BuildProfile::Production,
        &[
            json_case(
                "build_manifest_extra_root_key",
                "build-manifest.json",
                "build manifest keys must match contract",
                |manifest| {
                    manifest["unexpected"] = serde_json::json!(true);
                },
            ),
            json_case(
                "build_manifest_extra_capability_key",
                "build-manifest.json",
                "build manifest capabilities keys must match contract",
                |manifest| {
                    manifest["capabilities"]["unexpected"] = serde_json::json!("drift");
                },
            ),
            json_case(
                "build_manifest_capability_value_drift",
                "build-manifest.json",
                "build manifest capabilities do not match origin-map contract",
                |manifest| {
                    manifest["capabilities"]["server_routes"] = serde_json::json!(0);
                },
            ),
            json_case(
                "bundle_plan_extra_root_key",
                "bundle-plan.json",
                "bundle plan keys must match contract",
                |plan| {
                    plan["unexpected"] = serde_json::json!("drift");
                },
            ),
            json_case(
                "bundle_target_extra_key",
                "bundle-plan.json",
                "bundle target keys must match contract",
                |plan| {
                    plan["bundles"][0]["unexpected"] = serde_json::json!("drift");
                },
            ),
            json_case(
                "bundle_target_runtime_features_drift",
                "bundle-plan.json",
                "bundle target server_runtime runtime_features do not match target contract",
                |plan| {
                    plan["bundles"][0]["runtime_features"] = serde_json::json!([]);
                },
            ),
        ],
    );
}

#[test]
fn verify_build_static_artifact_cases() {
    verify_artifact_cases(
        "verify_build_static_artifact_cases",
        |name| source_fixture(name, r#"@out @html { @body { @h1 "Home" } }"#),
        BuildProfile::Development,
        &[
            artifact_case("verify_build_accepts_static_page_output", |out| {
                cmd_verify_build(out).expect("verify build");
            }),
            artifact_case("missing_static_page_output", |out| {
                std::fs::remove_file(out.join("pages").join("index.html")).expect("remove page");

                let err = cmd_verify_build(out).expect_err("missing static page");

                let message = err.to_string();
                assert!(
                    message.contains("missing bundle target static_page"),
                    "unexpected error: {message}"
                );
            }),
        ],
    );
}
