use super::*;

#[test]
fn verify_build_rejects_client_wasm_without_orv_custom_section() {
    let out = temp_output_dir("verify-build-client-wasm-section");
    std::fs::create_dir_all(&out).expect("create temp root");
    let entry = out.join("page.orv");
    std::fs::write(
        &entry,
        r"let sig count: int = 0
@out @html { @body { @p count } }",
    )
    .expect("write entry");
    let build_out = out.join("dist");

    cmd_build(&entry, &build_out).expect("build artifacts");
    let mut wasm = WASM_MODULE_HEADER.to_vec();
    let mut custom_section = Vec::new();
    push_wasm_len(&mut custom_section, "not.orv".len());
    custom_section.extend_from_slice(b"not.orv");
    custom_section.extend_from_slice(br#"{"note":"orv.client source_bundle"}"#);
    wasm.push(0);
    push_wasm_len(&mut wasm, custom_section.len());
    wasm.extend(custom_section);
    std::fs::write(build_out.join("client").join("app.wasm"), wasm).expect("rewrite wasm");
    refresh_client_manifest_wasm_hash(&build_out);

    let err = cmd_verify_build(&build_out).expect_err("invalid client wasm");

    assert!(
        err.to_string().contains("ORV metadata"),
        "unexpected error: {err}"
    );
    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn verify_build_rejects_client_js_without_event_arithmetic_actions() {
    let out = temp_output_dir("verify-build-client-js-event-arithmetic");
    std::fs::create_dir_all(&out).expect("create temp root");
    let entry = out.join("page.orv");
    std::fs::write(
            &entry,
            "let sig count: int = 0\n@out @html { @body { @button onClick={count += 1} \"+\" @button onClick={count -= 1} \"-\" } }",
        )
        .expect("write entry");
    let build_out = out.join("dist");

    cmd_build(&entry, &build_out).expect("build artifacts");
    let loader_path = build_out.join("client").join("app.js");
    let loader = std::fs::read_to_string(&loader_path)
        .expect("client loader")
        .replace("assign_add", "assign_plus")
        .replace("assign_sub", "assign_minus");
    std::fs::write(&loader_path, loader).expect("rewrite loader");
    refresh_client_manifest_loader_hash(&build_out);

    let err = cmd_verify_build(&build_out).expect_err("invalid client loader");

    assert!(
        err.to_string().contains("client reactive plan"),
        "unexpected error: {err}"
    );
    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn verify_build_rejects_client_reactive_plan_invalid_text_condition() {
    let out = temp_output_dir("verify-build-client-reactive-text-condition");
    std::fs::create_dir_all(&out).expect("create temp root");
    let entry = out.join("page.orv");
    std::fs::write(
        &entry,
        r#"let sig count: int = 0
@out @html { @body { @p { count > 0 ? "has items" : "empty" } } }"#,
    )
    .expect("write entry");
    let build_out = out.join("dist");

    cmd_build(&entry, &build_out).expect("build artifacts");
    let plan_path = build_out.join(CLIENT_REACTIVE_PLAN_PATH);
    let mut plan = read_json_value(&plan_path).expect("reactive plan");
    let binding = plan["bindings"]
        .as_array_mut()
        .expect("bindings")
        .iter_mut()
        .find(|binding| binding["kind"] == "signal_text")
        .expect("signal text binding");
    binding["text_condition"]["truthy"] = serde_json::json!(true);
    write_json(&plan_path, &plan).expect("write corrupt reactive plan");
    refresh_client_manifest_reactive_plan_hash(&build_out);

    let err = cmd_verify_build(&build_out).expect_err("invalid reactive plan");

    assert!(
        err.to_string().contains("signal_text binding"),
        "unexpected error: {err}"
    );
    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn verify_build_rejects_client_manifest_without_blocker_detail() {
    let out = temp_output_dir("verify-build-client-manifest-blocker-detail");
    std::fs::create_dir_all(&out).expect("create temp root");
    let entry = out.join("page.orv");
    std::fs::write(
        &entry,
        "let sig count: int = 0\n@out @html { @body { @p count } }",
    )
    .expect("write entry");
    let build_out = out.join("dist");

    cmd_build(&entry, &build_out).expect("build artifacts");
    let manifest_path = build_out.join(CLIENT_MANIFEST_PATH);
    let mut manifest = read_json_value(&manifest_path).expect("client manifest");
    manifest["blockers"] = serde_json::json!([]);
    write_json(&manifest_path, &manifest).expect("write corrupt client manifest");

    let err = cmd_verify_build(&build_out).expect_err("invalid client manifest");

    assert!(
        err.to_string().contains(
            "client_manifest blockers must describe blocked_by entry dynamic-client-codegen"
        ),
        "unexpected error: {err}"
    );
    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn verify_build_rejects_client_reactive_plan_without_blocker_detail() {
    let out = temp_output_dir("verify-build-client-reactive-plan-blocker-detail");
    std::fs::create_dir_all(&out).expect("create temp root");
    let entry = out.join("page.orv");
    std::fs::write(
        &entry,
        "let sig count: int = 0\n@out @html { @body { @p count } }",
    )
    .expect("write entry");
    let build_out = out.join("dist");

    cmd_build(&entry, &build_out).expect("build artifacts");
    let plan_path = build_out.join(CLIENT_REACTIVE_PLAN_PATH);
    let mut plan = read_json_value(&plan_path).expect("reactive plan");
    plan["blockers"] = serde_json::json!([]);
    write_json(&plan_path, &plan).expect("write corrupt reactive plan");
    refresh_client_manifest_reactive_plan_hash(&build_out);

    let err = cmd_verify_build(&build_out).expect_err("invalid reactive plan");

    assert!(
        err.to_string().contains(
            "client_reactive_plan blockers must describe blocked_by entry reactive-dom-diff"
        ),
        "unexpected error: {err}"
    );
    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn verify_client_production_artifact_cases() {
    verify_artifact_cases(
        "verify_client_production_artifact_cases",
        |name| {
            source_fixture(
                name,
                "let sig count: int = 0\n@out @html { @body { @p count } }",
            )
        },
        BuildProfile::Production,
        &[
            json_case(
                "deploy_client_capability_drift",
                "deploy/manifest.json",
                "deploy client capabilities do not match client manifest",
                |deploy| {
                    deploy["client"]["capabilities"]["bindings"]["signal_text"] =
                        serde_json::json!(0);
                },
            ),
            json_case(
                "deploy_client_extra_root_key",
                "deploy/manifest.json",
                "deploy client keys must match contract",
                |deploy| {
                    deploy["client"]["unexpected"] = serde_json::json!(true);
                },
            ),
            json_case(
                "deploy_client_reactive_plan_drift",
                "deploy/manifest.json",
                "deploy client reactive_plan does not match client manifest",
                |deploy| {
                    deploy["client"]["reactive_plan"] = serde_json::json!("client/other-plan.json");
                },
            ),
            json_case(
                "deploy_client_blocker_drift",
                "deploy/manifest.json",
                "deploy client blockers do not match client manifest",
                |deploy| {
                    deploy["client"]["blockers"] = serde_json::json!([]);
                },
            ),
        ],
    );
}

#[test]
fn verify_client_development_artifact_cases() {
    verify_artifact_cases(
        "verify_client_development_artifact_cases",
        |name| {
            source_fixture(
                name,
                "let sig count: int = 0\n@out @html { @body { @p count } }",
            )
        },
        BuildProfile::Development,
        &[
            artifact_case("client_wasm_without_start_export", |out| {
                let original_wasm =
                    std::fs::read(out.join("client").join("app.wasm")).expect("client wasm");
                let original_metadata = client_wasm_custom_section_payload(&original_wasm)
                    .expect("read wasm metadata")
                    .expect("orv metadata section")
                    .to_vec();
                let mut wasm = WASM_MODULE_HEADER.to_vec();
                let mut custom_section = Vec::new();
                push_wasm_len(&mut custom_section, CLIENT_WASM_CUSTOM_SECTION_NAME.len());
                custom_section.extend_from_slice(CLIENT_WASM_CUSTOM_SECTION_NAME.as_bytes());
                custom_section.extend_from_slice(&original_metadata);
                push_wasm_section(&mut wasm, 0, &custom_section);
                std::fs::write(out.join("client").join("app.wasm"), wasm).expect("rewrite wasm");
                refresh_client_manifest_wasm_hash(out);

                let err = cmd_verify_build(out).expect_err("invalid client wasm");

                assert!(
                    err.to_string().contains("orv_start"),
                    "unexpected error: {err}"
                );
            }),
            artifact_case("client_wasm_start_export_wrong_index", |out| {
                let wasm_path = out.join("client").join("app.wasm");
                let mut wasm = std::fs::read(&wasm_path).expect("client wasm");
                corrupt_generated_start_export_index(&mut wasm, 1);
                std::fs::write(&wasm_path, wasm).expect("rewrite wasm");
                refresh_client_manifest_wasm_hash(out);

                let err = cmd_verify_build(out).expect_err("invalid client wasm start index");

                assert!(
                    err.to_string().contains("orv_start"),
                    "unexpected error: {err}"
                );
            }),
            artifact_case("client_wasm_without_memory_export", |out| {
                let wasm_path = out.join("client").join("app.wasm");
                let mut wasm = std::fs::read(&wasm_path).expect("client wasm");
                corrupt_generated_memory_export_kind(&mut wasm, 0);
                std::fs::write(&wasm_path, wasm).expect("rewrite wasm");
                refresh_client_manifest_wasm_hash(out);

                let err = cmd_verify_build(out).expect_err("invalid client wasm memory export");

                assert!(
                    err.to_string().contains("memory"),
                    "unexpected error: {err}"
                );
            }),
            artifact_case("client_wasm_memory_export_wrong_index", |out| {
                let wasm_path = out.join("client").join("app.wasm");
                let mut wasm = std::fs::read(&wasm_path).expect("client wasm");
                corrupt_generated_memory_export_index(&mut wasm, 1);
                std::fs::write(&wasm_path, wasm).expect("rewrite wasm");
                refresh_client_manifest_wasm_hash(out);

                let err = cmd_verify_build(out).expect_err("invalid client wasm memory index");

                assert!(
                    err.to_string().contains("memory 0"),
                    "unexpected error: {err}"
                );
            }),
            artifact_case("client_wasm_initial_render_data_mismatch", |out| {
                let wasm_path = out.join("client").join("app.wasm");
                let mut wasm = std::fs::read(&wasm_path).expect("client wasm");
                let initial_html = b"<html><body><p>0</p></body></html>";
                let html_offset = wasm
                    .windows(initial_html.len())
                    .position(|window| window == initial_html)
                    .expect("initial render data segment");
                let count_offset = html_offset + b"<html><body><p>".len();
                assert_eq!(wasm[count_offset], b'0');
                wasm[count_offset] = b'1';
                std::fs::write(&wasm_path, wasm).expect("rewrite wasm");
                refresh_client_manifest_wasm_hash(out);

                let err = cmd_verify_build(out).expect_err("invalid client wasm render data");

                assert!(
                    err.to_string().contains("initial_render html_hash"),
                    "unexpected error: {err}"
                );
            }),
            artifact_case("client_wasm_render_len_export_mismatch", |out| {
                let wasm_path = out.join("client").join("app.wasm");
                let mut wasm = std::fs::read(&wasm_path).expect("client wasm");
                corrupt_generated_render_len_const(&mut wasm, 0);
                std::fs::write(&wasm_path, wasm).expect("rewrite wasm");
                refresh_client_manifest_wasm_hash(out);

                let err = cmd_verify_build(out).expect_err("invalid client wasm render len");

                assert!(
                    err.to_string().contains("orv_render_len"),
                    "unexpected error: {err}"
                );
            }),
            artifact_case("client_manifest_wasm_hash_mismatch", |out| {
                let manifest_path = out.join(CLIENT_MANIFEST_PATH);
                let mut manifest = read_json_value(&manifest_path).expect("client manifest");
                manifest["wasm_hash"] = serde_json::json!("fnv1a64:bad");
                write_json(&manifest_path, &manifest).expect("rewrite client manifest");

                let err = cmd_verify_build(out).expect_err("invalid client manifest wasm hash");

                assert!(
                    err.to_string().contains("wasm_hash"),
                    "unexpected error: {err}"
                );
            }),
            artifact_case("client_manifest_loader_hash_mismatch", |out| {
                let manifest_path = out.join(CLIENT_MANIFEST_PATH);
                let mut manifest = read_json_value(&manifest_path).expect("client manifest");
                manifest["loader_hash"] = serde_json::json!("fnv1a64:bad");
                write_json(&manifest_path, &manifest).expect("rewrite client manifest");

                let err = cmd_verify_build(out).expect_err("invalid client manifest loader hash");

                assert!(
                    err.to_string().contains("loader_hash"),
                    "unexpected error: {err}"
                );
            }),
            artifact_case("client_manifest_reactive_plan_hash_mismatch", |out| {
                let manifest_path = out.join(CLIENT_MANIFEST_PATH);
                let mut manifest = read_json_value(&manifest_path).expect("client manifest");
                manifest["reactive_plan_hash"] = serde_json::json!("fnv1a64:bad");
                write_json(&manifest_path, &manifest).expect("rewrite client manifest");

                let err =
                    cmd_verify_build(out).expect_err("invalid client manifest reactive plan hash");

                assert!(
                    err.to_string().contains("reactive_plan_hash"),
                    "unexpected error: {err}"
                );
            }),
            artifact_case("client_manifest_initial_render_mismatch", |out| {
                let manifest_path = out.join(CLIENT_MANIFEST_PATH);
                let mut manifest = read_json_value(&manifest_path).expect("client manifest");
                manifest["initial_render"]["byte_length"] = serde_json::json!(0);
                write_json(&manifest_path, &manifest).expect("rewrite client manifest");

                let err = cmd_verify_build(out).expect_err("invalid client manifest render");

                assert!(
                    err.to_string().contains("initial_render"),
                    "unexpected error: {err}"
                );
            }),
            artifact_case("client_manifest_extra_root_key", |out| {
                let manifest_path = out.join(CLIENT_MANIFEST_PATH);
                let mut manifest = read_json_value(&manifest_path).expect("client manifest");
                manifest["unexpected"] = serde_json::json!("drift");
                write_json(&manifest_path, &manifest).expect("write drifted client manifest");

                let err =
                    cmd_verify_build(out).expect_err("extra client manifest root key must fail");

                assert!(
                    err.to_string()
                        .contains("client_manifest keys must match contract"),
                    "unexpected error: {err}"
                );
            }),
            artifact_case("client_manifest_extra_export_key", |out| {
                let manifest_path = out.join(CLIENT_MANIFEST_PATH);
                let mut manifest = read_json_value(&manifest_path).expect("client manifest");
                manifest["exports"]["unexpected"] = serde_json::json!("drift");
                write_json(&manifest_path, &manifest).expect("write drifted client manifest");

                let err =
                    cmd_verify_build(out).expect_err("extra client manifest export key must fail");

                assert!(
                    err.to_string()
                        .contains("client_manifest exports keys must match contract"),
                    "unexpected error: {err}"
                );
            }),
            artifact_case("client_reactive_plan_extra_signal_key", |out| {
                let plan_path = out.join(CLIENT_REACTIVE_PLAN_PATH);
                let mut plan = read_json_value(&plan_path).expect("reactive plan");
                plan["signals"][0]["unexpected"] = serde_json::json!("drift");
                write_json(&plan_path, &plan).expect("write drifted reactive plan");
                refresh_client_manifest_reactive_plan_hash(out);

                let err = cmd_verify_build(out).expect_err("extra client signal key must fail");

                assert!(
                    err.to_string()
                        .contains("client_reactive_plan signals[0] keys must match contract"),
                    "unexpected error: {err}"
                );
            }),
            artifact_case("client_reactive_plan_extra_binding_key", |out| {
                let plan_path = out.join(CLIENT_REACTIVE_PLAN_PATH);
                let mut plan = read_json_value(&plan_path).expect("reactive plan");
                let binding = plan["bindings"]
                    .as_array_mut()
                    .expect("bindings")
                    .iter_mut()
                    .find(|binding| binding["kind"] == "signal_text")
                    .expect("signal text binding");
                binding["unexpected"] = serde_json::json!("drift");
                write_json(&plan_path, &plan).expect("write drifted reactive plan");
                refresh_client_manifest_reactive_plan_hash(out);

                let err = cmd_verify_build(out).expect_err("extra client binding key must fail");

                assert!(
                    err.to_string().contains("client_reactive_plan bindings"),
                    "unexpected error: {err}"
                );
                assert!(
                    err.to_string().contains("keys must match contract"),
                    "unexpected error: {err}"
                );
            }),
            artifact_case("client_js_generated_loader_drift", |out| {
                let loader_path = out.join("client").join("app.js");
                let mut loader = std::fs::read_to_string(&loader_path).expect("client loader");
                loader.push_str("\nconsole.log('drift');\n");
                std::fs::write(&loader_path, loader).expect("rewrite loader");
                refresh_client_manifest_loader_hash(out);

                let err =
                    cmd_verify_build(out).expect_err("generated client loader drift must fail");

                assert!(
                    err.to_string()
                        .contains("client_js bundle must match generated loader"),
                    "unexpected error: {err}"
                );
            }),
            artifact_case("client_js_without_start_call", |out| {
                let loader_path = out.join("client").join("app.js");
                let loader = std::fs::read_to_string(&loader_path)
                    .expect("client loader")
                    .replace(
                        r#"  if (typeof instance.exports.orv_start === "function") {
    instance.exports.orv_start();
  }
"#,
                        "",
                    );
                std::fs::write(&loader_path, loader).expect("rewrite loader");
                refresh_client_manifest_loader_hash(out);

                let err = cmd_verify_build(out).expect_err("invalid client loader");

                assert!(
                    err.to_string().contains("orv_start"),
                    "unexpected error: {err}"
                );
            }),
            artifact_case("client_js_without_source_bundle_hash_check", |out| {
                let loader_path = out.join("client").join("app.js");
                let loader = std::fs::read_to_string(&loader_path)
                    .expect("client loader")
                    .replace("source bundle hash mismatch", "source bundle hash skipped");
                std::fs::write(&loader_path, loader).expect("rewrite loader");
                refresh_client_manifest_loader_hash(out);

                let err = cmd_verify_build(out).expect_err("invalid client loader");

                assert!(
                    err.to_string().contains("source bundle hash"),
                    "unexpected error: {err}"
                );
            }),
            artifact_case("client_js_without_manifest_contract_check", |out| {
                let loader_path = out.join("client").join("app.js");
                let loader = std::fs::read_to_string(&loader_path)
                    .expect("client loader")
                    .replace("loadClientManifest", "loadClientContract");
                std::fs::write(&loader_path, loader).expect("rewrite loader");
                refresh_client_manifest_loader_hash(out);

                let err = cmd_verify_build(out).expect_err("invalid client loader");

                assert!(
                    err.to_string().contains("client manifest"),
                    "unexpected error: {err}"
                );
            }),
            artifact_case("client_js_without_reactive_plan_check", |out| {
                let loader_path = out.join("client").join("app.js");
                let loader = std::fs::read_to_string(&loader_path)
                    .expect("client loader")
                    .replace("loadReactivePlan", "loadReactiveContract");
                std::fs::write(&loader_path, loader).expect("rewrite loader");
                refresh_client_manifest_loader_hash(out);

                let err = cmd_verify_build(out).expect_err("invalid client loader");

                assert!(
                    err.to_string().contains("client reactive plan"),
                    "unexpected error: {err}"
                );
            }),
            artifact_case("client_js_without_initial_render_hash_check", |out| {
                let loader_path = out.join("client").join("app.js");
                let loader = std::fs::read_to_string(&loader_path)
                    .expect("client loader")
                    .replace("validateInitialRender", "skipInitialRenderValidation");
                std::fs::write(&loader_path, loader).expect("rewrite loader");
                refresh_client_manifest_loader_hash(out);

                let err = cmd_verify_build(out).expect_err("invalid client loader");

                assert!(
                    err.to_string().contains("initial render"),
                    "unexpected error: {err}"
                );
            }),
            artifact_case(
                "client_reactive_plan_without_initial_render_binding",
                |out| {
                    let plan_path = out.join(CLIENT_REACTIVE_PLAN_PATH);
                    let mut plan = read_json_value(&plan_path).expect("reactive plan");
                    plan["bindings"] = serde_json::json!([]);
                    write_json(&plan_path, &plan).expect("write corrupt reactive plan");
                    refresh_client_manifest_reactive_plan_hash(out);

                    let err = cmd_verify_build(out).expect_err("invalid reactive plan");

                    assert!(
                        err.to_string().contains("initial_render binding"),
                        "unexpected error: {err}"
                    );
                },
            ),
            artifact_case("client_reactive_plan_initial_render_mismatch", |out| {
                let plan_path = out.join(CLIENT_REACTIVE_PLAN_PATH);
                let mut plan = read_json_value(&plan_path).expect("reactive plan");
                let binding = plan["bindings"]
                    .as_array_mut()
                    .expect("bindings")
                    .iter_mut()
                    .find(|binding| binding["kind"] == "initial_render")
                    .expect("initial render binding");
                binding["byte_length"] = serde_json::json!(0);
                write_json(&plan_path, &plan).expect("write corrupt reactive plan");
                refresh_client_manifest_reactive_plan_hash(out);

                let err = cmd_verify_build(out).expect_err("invalid reactive plan");

                assert!(
                    err.to_string().contains("initial_render binding"),
                    "unexpected error: {err}"
                );
            }),
            artifact_case("client_reactive_plan_without_signal_state_binding", |out| {
                let plan_path = out.join(CLIENT_REACTIVE_PLAN_PATH);
                let mut plan = read_json_value(&plan_path).expect("reactive plan");
                let bindings = plan["bindings"].as_array_mut().expect("bindings");
                bindings.retain(|binding| binding["kind"] != "signal_state");
                write_json(&plan_path, &plan).expect("write corrupt reactive plan");
                refresh_client_manifest_reactive_plan_hash(out);

                let err = cmd_verify_build(out).expect_err("invalid reactive plan");

                assert!(
                    err.to_string().contains("signal_state binding"),
                    "unexpected error: {err}"
                );
            }),
            artifact_case("client_manifest_capability_drift", |out| {
                let manifest_path = out.join(CLIENT_MANIFEST_PATH);
                let mut manifest = read_json_value(&manifest_path).expect("client manifest");
                manifest["capabilities"]["bindings"]["signal_text"] = serde_json::json!(0);
                write_json(&manifest_path, &manifest).expect("write corrupt client manifest");

                let err = cmd_verify_build(out).expect_err("invalid client manifest capabilities");

                assert!(
                    err.to_string()
                        .contains("client_manifest capabilities do not match reactive plan"),
                    "unexpected error: {err}"
                );
            }),
        ],
    );
}
