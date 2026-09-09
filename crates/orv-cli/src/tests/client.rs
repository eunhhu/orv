use super::*;

#[test]
fn build_prod_runbook_documents_client_bundle_contract() {
    let dir = temp_output_dir("build-prod-client-runbook-source");
    std::fs::create_dir_all(&dir).expect("create temp root");
    let path = dir.join("app.orv");
    std::fs::write(
        &path,
        r#"@server {
  @listen 8080
  @route GET /ping {
    @respond 200 { ok: true }
  }
}

let sig count: int = 0
@out @html { @body { @p count } }
"#,
    )
    .expect("write source");
    let out = temp_output_dir("build-prod-client-runbook");

    cmd_build_with_profile(&path, &out, BuildProfile::Production).expect("prod build");
    let runbook_path = out.join("deploy").join("README.md");
    let runbook = std::fs::read_to_string(&runbook_path).expect("deploy runbook");

    assert!(runbook.contains("## Client Bundle"));
    assert!(runbook.contains("- Client manifest: client/manifest.json"));
    assert!(runbook.contains("- Client reactive plan: client/reactive-plan.json"));
    assert!(runbook.contains("- Client page: pages/index.html"));
    assert!(runbook.contains("- Client loader: client/app.js"));
    assert!(runbook.contains("- Client WASM: client/app.wasm"));
    assert!(runbook.contains("- Client runtime: client_wasm"));
    assert!(runbook.contains("signal_text"));
    assert!(runbook.contains("dynamic-client-codegen"));
    cmd_verify_build(&out).expect("verify client runbook");

    write_text(
        &runbook_path,
        &runbook.replace("signal_text", "signal_slot"),
    )
    .expect("write corrupt runbook");
    let err = cmd_verify_build(&out).expect_err("client runbook mismatch");
    assert!(
        err.to_string()
            .contains("deploy runbook must document client capability surface signal_text"),
        "{err}"
    );
    let _ = std::fs::remove_dir_all(dir);
    let _ = std::fs::remove_dir_all(out);
}

#[test]
fn build_writes_client_wasm_for_signal_html_entry() {
    let out = temp_output_dir("build-client-wasm");
    std::fs::create_dir_all(&out).expect("create temp root");
    let entry = out.join("page.orv");
    std::fs::write(
        &entry,
        "let sig count: int = 0\n@out @html { @body { @p count } }",
    )
    .expect("write entry");
    let build_out = out.join("dist");

    cmd_build(&entry, &build_out).expect("build artifacts");

    let manifest = read_json_value(&build_out.join("build-manifest.json")).expect("manifest");
    assert_eq!(manifest["capabilities"]["client_wasm"], true);
    assert!(manifest["capabilities"]["runtime_features"]
        .as_array()
        .expect("runtime features")
        .iter()
        .any(|feature| feature == "client_wasm"));
    let plan = read_json_value(&build_out.join("bundle-plan.json")).expect("plan");
    assert!(plan["bundles"]
        .as_array()
        .expect("bundles")
        .iter()
        .any(|bundle| bundle["kind"] == "client_wasm" && bundle["path"] == "client/app.wasm"));
    assert!(plan["bundles"]
        .as_array()
        .expect("bundles")
        .iter()
        .any(|bundle| bundle["kind"] == "client_js" && bundle["path"] == "client/app.js"));
    assert!(plan["bundles"]
        .as_array()
        .expect("bundles")
        .iter()
        .any(|bundle| bundle["kind"] == "client_page" && bundle["path"] == "pages/index.html"));
    assert!(!plan["bundles"]
        .as_array()
        .expect("bundles")
        .iter()
        .any(|bundle| bundle["kind"] == "static_page"));
    let wasm = std::fs::read(build_out.join("client").join("app.wasm")).expect("client wasm");
    assert_eq!(&wasm[..4], b"\0asm");
    let wasm_text = String::from_utf8_lossy(&wasm);
    assert!(wasm_text.contains("orv.client"));
    assert!(wasm_text.contains("source_bundle"));
    assert!(wasm_text.contains("orv_start"));
    let source_bundle =
        read_json_value(&build_out.join("source-bundle.json")).expect("source bundle");
    let expected_source_bundle_hash = stable_json_hash(&source_bundle).expect("source bundle hash");
    let wasm_metadata = client_wasm_custom_section_payload(&wasm)
        .expect("read wasm metadata")
        .expect("orv metadata section");
    let wasm_metadata: serde_json::Value =
        serde_json::from_slice(wasm_metadata).expect("wasm metadata json");
    assert_eq!(wasm_metadata["entry"], source_bundle["entry"]);
    assert_eq!(
        wasm_metadata["source_bundle_hash"],
        expected_source_bundle_hash
    );
    assert_eq!(wasm_metadata["initial_render"]["content_type"], "text/html");
    assert_eq!(wasm_metadata["initial_render"]["encoding"], "utf-8");
    assert!(wasm_metadata["initial_render"]["html_hash"]
        .as_str()
        .is_some_and(|hash| !hash.is_empty()));
    assert!(
        client_wasm_exports_function(&wasm, "orv_render_ptr").expect("render ptr export"),
        "client wasm must export render pointer"
    );
    assert!(
        client_wasm_exports_function(&wasm, "orv_render_len").expect("render len export"),
        "client wasm must export render length"
    );
    let loader =
        std::fs::read_to_string(build_out.join("client").join("app.js")).expect("client js");
    assert_client_loader_contract(&loader);
    let reactive_plan =
        read_json_value(&build_out.join("client/reactive-plan.json")).expect("reactive plan");
    let direct_text = reactive_plan["bindings"]
        .as_array()
        .expect("bindings")
        .iter()
        .find(|binding| binding["kind"] == "signal_text")
        .expect("direct signal text binding");
    assert_eq!(
        direct_text
            .as_object()
            .expect("signal text object")
            .keys()
            .map(String::as_str)
            .collect::<BTreeSet<_>>(),
        ["kind", "source", "target", "selector", "state_key", "span"]
            .into_iter()
            .collect::<BTreeSet<_>>()
    );
    let page =
        std::fs::read_to_string(build_out.join("pages").join("index.html")).expect("client page");
    assert!(page.contains("data-orv-client=\"wasm\""));
    assert!(page.contains("id=\"orv-root\""));
    assert!(page.contains("type=\"module\""));
    assert!(page.contains("../client/app.js"));
    cmd_verify_build(&build_out).expect("verify build");
    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn build_prod_records_client_bootstrap_targets() {
    let out = temp_output_dir("build-prod-client");
    std::fs::create_dir_all(&out).expect("create temp root");
    let entry = out.join("page.orv");
    std::fs::write(
        &entry,
        r"let sig count: int = 0
@out @html { @body { @p count } }",
    )
    .expect("write entry");
    let build_out = out.join("dist");

    cmd_build_with_profile(&entry, &build_out, BuildProfile::Production).expect("build prod");

    let deploy = read_json_value(&build_out.join("deploy").join("manifest.json")).expect("deploy");
    assert_eq!(deploy["client"]["manifest"], "client/manifest.json");
    assert_eq!(
        deploy["client"]["reactive_plan"],
        "client/reactive-plan.json"
    );
    assert_eq!(deploy["client"]["page"], "pages/index.html");
    assert_eq!(deploy["client"]["loader"], "client/app.js");
    assert_eq!(deploy["client"]["wasm"], "client/app.wasm");
    assert!(deploy["client"]["runtime_features"]
        .as_array()
        .expect("runtime features")
        .iter()
        .any(|feature| feature == "client_wasm"));
    assert_eq!(deploy["client"]["capabilities"]["runtime"], "client_wasm");
    assert_eq!(
        deploy["client"]["capabilities"]["bindings"]["signal_text"],
        1
    );
    assert!(deploy["client"]["blocked_by"]
        .as_array()
        .expect("blocked_by")
        .iter()
        .any(|item| item == "dynamic-client-codegen"));
    assert!(deploy["client"]["blockers"]
        .as_array()
        .expect("blockers")
        .iter()
        .any(|item| item["id"] == "dynamic-client-codegen"));
    cmd_verify_build(&build_out).expect("verify prod build");
    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn build_writes_client_bundle_manifest_contract() {
    let out = temp_output_dir("client-bundle-manifest");
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
    assert!(
        manifest_path.is_file(),
        "missing {}",
        manifest_path.display()
    );
    let client_manifest = read_json_value(&manifest_path).expect("client manifest");
    let source_bundle =
        read_json_value(&build_out.join("source-bundle.json")).expect("source bundle");
    let expected_source_hash = stable_json_hash(&source_bundle).expect("source hash");
    let expected_wasm_hash =
        file_content_hash(&build_out.join(CLIENT_WASM_PATH)).expect("wasm hash");
    let expected_loader_hash =
        file_content_hash(&build_out.join(CLIENT_JS_PATH)).expect("loader hash");
    let reactive_plan =
        read_json_value(&build_out.join(CLIENT_REACTIVE_PLAN_PATH)).expect("reactive plan");
    let expected_reactive_plan_hash = stable_json_hash(&reactive_plan).expect("reactive plan hash");
    assert_manifest_artifact(
        &build_out.join("build-manifest.json"),
        "client_manifest",
        CLIENT_MANIFEST_PATH,
    );
    assert_bundle_target(
        &build_out.join("bundle-plan.json"),
        "client_manifest",
        CLIENT_MANIFEST_PATH,
    );
    assert_eq!(client_manifest["kind"], "orv.client.bundle");
    assert_eq!(client_manifest["page"], "pages/index.html");
    assert_eq!(client_manifest["loader"], "client/app.js");
    assert_eq!(client_manifest["loader_hash"], expected_loader_hash);
    assert_eq!(
        client_manifest["reactive_plan_hash"],
        expected_reactive_plan_hash
    );
    assert_eq!(client_manifest["wasm"], "client/app.wasm");
    assert_eq!(client_manifest["wasm_hash"], expected_wasm_hash);
    assert_eq!(client_manifest["source_bundle"], "source-bundle.json");
    assert_eq!(client_manifest["source_bundle_hash"], expected_source_hash);
    assert_eq!(
        client_manifest["exports"]["start"],
        CLIENT_WASM_START_EXPORT
    );
    assert_eq!(
        client_manifest["exports"]["render_ptr"],
        CLIENT_WASM_RENDER_PTR_EXPORT
    );
    assert_eq!(
        client_manifest["exports"]["render_len"],
        CLIENT_WASM_RENDER_LEN_EXPORT
    );
    assert_eq!(client_manifest["capabilities"]["runtime"], "client_wasm");
    assert_eq!(
        client_manifest["capabilities"]["source"],
        CLIENT_REACTIVE_PLAN_PATH
    );
    assert_eq!(client_manifest["capabilities"]["signals"], 1);
    assert_eq!(
        client_manifest["capabilities"]["bindings"]["signal_state"],
        1
    );
    assert_eq!(
        client_manifest["capabilities"]["bindings"]["signal_text"],
        1
    );
    let capability_surfaces = client_manifest["capabilities"]["surfaces"]
        .as_array()
        .expect("capability surfaces");
    assert!(capability_surfaces
        .iter()
        .any(|surface| surface == "signal_text"));
    assert!(capability_surfaces
        .iter()
        .any(|surface| surface == "embedded_reactive_plan"));
    assert!(client_manifest["blocked_by"]
        .as_array()
        .expect("blocked_by")
        .iter()
        .any(|item| item == "dynamic-client-codegen"));
    assert!(client_manifest["blockers"]
        .as_array()
        .expect("blockers")
        .iter()
        .any(|item| item["id"] == "dynamic-client-codegen" && item["artifact"] == CLIENT_JS_PATH));

    cmd_verify_build(&build_out).expect("verify build artifacts");
    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn build_writes_client_reactive_plan_contract() {
    let out = temp_output_dir("client-reactive-plan");
    std::fs::create_dir_all(&out).expect("create temp root");
    let entry = out.join("page.orv");
    std::fs::write(
        &entry,
        "let sig count: int = 0\n@out @html { @body { @p count } }",
    )
    .expect("write entry");
    let build_out = out.join("dist");

    cmd_build(&entry, &build_out).expect("build artifacts");

    let reactive_path = build_out.join("client").join("reactive-plan.json");
    assert!(
        reactive_path.is_file(),
        "missing {}",
        reactive_path.display()
    );
    let reactive_plan = read_json_value(&reactive_path).expect("reactive plan");
    let source_bundle =
        read_json_value(&build_out.join("source-bundle.json")).expect("source bundle");
    let expected_source_hash = stable_json_hash(&source_bundle).expect("source hash");
    assert_manifest_artifact(
        &build_out.join("build-manifest.json"),
        "client_reactive_plan",
        "client/reactive-plan.json",
    );
    assert_bundle_target(
        &build_out.join("bundle-plan.json"),
        "client_reactive_plan",
        "client/reactive-plan.json",
    );
    assert_eq!(reactive_plan["kind"], "orv.client.reactive_plan");
    assert_eq!(reactive_plan["source_bundle"], SOURCE_BUNDLE_PATH);
    assert_eq!(reactive_plan["source_bundle_hash"], expected_source_hash);
    assert!(reactive_plan["signals"]
        .as_array()
        .expect("signals")
        .iter()
        .any(|signal| signal["name"] == "count"
            && signal["state_key"] == "count"
            && signal["initial_value"]["kind"] == "int"
            && signal["initial_value"]["value"] == "0"
            && signal["origin_id"]
                .as_str()
                .is_some_and(|id| !id.is_empty())));
    assert!(reactive_plan["bindings"]
        .as_array()
        .expect("bindings")
        .iter()
        .any(|binding| binding["kind"] == "initial_render"
            && binding["target"] == CLIENT_PAGE_PATH
            && binding["source"] == CLIENT_WASM_PATH));
    assert!(reactive_plan["bindings"]
        .as_array()
        .expect("bindings")
        .iter()
        .any(|binding| binding["kind"] == "signal_state"
            && binding["target"] == CLIENT_JS_PATH
            && binding["state_key"] == "count"
            && binding["source"].as_str().is_some_and(|id| !id.is_empty())));
    assert!(reactive_plan["bindings"]
        .as_array()
        .expect("bindings")
        .iter()
        .any(|binding| binding["kind"] == "signal_text"
            && binding["target"] == CLIENT_PAGE_PATH
            && binding["state_key"] == "count"
            && binding["selector"] == "p"
            && binding["source"].as_str().is_some_and(|id| !id.is_empty())));
    assert!(reactive_plan["blocked_by"]
        .as_array()
        .expect("blocked_by")
        .iter()
        .any(|item| item == "reactive-dom-diff"));
    assert!(!reactive_plan["blocked_by"]
        .as_array()
        .expect("blocked_by")
        .iter()
        .any(|item| item == "dynamic-client-codegen"));
    assert!(reactive_plan["blockers"]
        .as_array()
        .expect("blockers")
        .iter()
        .any(|item| item["id"] == "reactive-dom-diff"
            && item["artifact"] == CLIENT_REACTIVE_PLAN_PATH));
    let client_manifest =
        read_json_value(&build_out.join(CLIENT_MANIFEST_PATH)).expect("client manifest");
    assert_eq!(
        client_manifest["reactive_plan"],
        "client/reactive-plan.json"
    );
    let loader = std::fs::read_to_string(build_out.join(CLIENT_JS_PATH)).expect("client loader");
    let bootstrap = client_loader_bootstrap_json(&loader);
    assert_eq!(bootstrap["embeddedReactivePlan"], reactive_plan);
    assert_eq!(
        bootstrap["embeddedReactivePlanHash"],
        stable_json_hash(&reactive_plan).expect("reactive plan hash")
    );

    cmd_verify_build(&build_out).expect("verify build artifacts");
    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn build_writes_client_signal_text_template_binding_contract() {
    let out = temp_output_dir("client-reactive-text-template-binding");
    std::fs::create_dir_all(&out).expect("create temp root");
    let entry = out.join("page.orv");
    std::fs::write(
        &entry,
        r#"let sig count: int = 0
@out @html { @body { @p "count: {count}" @button onClick={count += 1} "+" } }"#,
    )
    .expect("write entry");
    let build_out = out.join("dist");

    cmd_build(&entry, &build_out).expect("build artifacts");

    let reactive_plan =
        read_json_value(&build_out.join(CLIENT_REACTIVE_PLAN_PATH)).expect("reactive plan");
    assert!(reactive_plan["bindings"]
        .as_array()
        .expect("bindings")
        .iter()
        .any(|binding| binding["kind"] == "signal_text"
            && binding["target"] == CLIENT_PAGE_PATH
            && binding["state_key"] == "count"
            && binding["selector"] == "p"
            && binding["text_template"]
                .as_array()
                .is_some_and(|segments| segments.iter().any(|segment| {
                    segment["kind"] == "signal" && segment["state_key"] == "count"
                }))
            && binding["source"].as_str().is_some_and(|id| !id.is_empty())));
    let loader = std::fs::read_to_string(build_out.join(CLIENT_JS_PATH)).expect("client loader");
    assert!(loader.contains("renderSignalTextBinding"));
    assert!(loader.contains("text_template"));
    cmd_verify_build(&build_out).expect("verify build artifacts");
    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn build_writes_client_multi_signal_text_template_binding_contract() {
    let out = temp_output_dir("client-reactive-multi-signal-text-template-binding");
    std::fs::create_dir_all(&out).expect("create temp root");
    let entry = out.join("page.orv");
    std::fs::write(
        &entry,
        r#"let sig label: string = "Items"
let sig count: int = 0
@out @html { @body { @p "{label}: {count}" } }"#,
    )
    .expect("write entry");
    let build_out = out.join("dist");

    cmd_build(&entry, &build_out).expect("build artifacts");

    let reactive_plan =
        read_json_value(&build_out.join(CLIENT_REACTIVE_PLAN_PATH)).expect("reactive plan");
    assert!(reactive_plan["bindings"]
        .as_array()
        .expect("bindings")
        .iter()
        .any(|binding| binding["kind"] == "signal_text"
            && binding["target"] == CLIENT_PAGE_PATH
            && binding["selector"] == "p"
            && binding["state_keys"] == serde_json::json!(["label", "count"])
            && binding["sources"].as_array().is_some_and(|sources| {
                sources.iter().any(|source| source["state_key"] == "label")
                    && sources.iter().any(|source| source["state_key"] == "count")
            })
            && binding["text_template"]
                == serde_json::json!([
                    {"kind": "signal", "state_key": "label"},
                    {"kind": "text", "value": ": "},
                    {"kind": "signal", "state_key": "count"},
                ])));
    let loader = std::fs::read_to_string(build_out.join(CLIENT_JS_PATH)).expect("client loader");
    assert!(loader.contains("signalTextBindingStateKeys"));
    assert!(loader.contains("state_keys"));
    cmd_verify_build(&build_out).expect("verify build artifacts");
    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn build_writes_client_signal_text_condition_binding_contract() {
    let out = temp_output_dir("client-reactive-text-condition-binding");
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

    let reactive_plan =
        read_json_value(&build_out.join(CLIENT_REACTIVE_PLAN_PATH)).expect("reactive plan");
    assert!(reactive_plan["bindings"]
        .as_array()
        .expect("bindings")
        .iter()
        .any(|binding| binding["kind"] == "signal_text"
            && binding["target"] == CLIENT_PAGE_PATH
            && binding["state_key"] == "count"
            && binding["selector"] == "p"
            && binding["text_condition"]["state_key"] == "count"
            && binding["text_condition"]["op"] == "gt"
            && binding["text_condition"]["rhs"]["kind"] == "int"
            && binding["text_condition"]["rhs"]["value"] == "0"
            && binding["text_condition"]["truthy"] == "has items"
            && binding["text_condition"]["falsy"] == "empty"
            && binding["source"].as_str().is_some_and(|id| !id.is_empty())));
    let loader = std::fs::read_to_string(build_out.join(CLIENT_JS_PATH)).expect("client loader");
    assert!(loader.contains("renderSignalTextCondition"));
    assert!(loader.contains("text_condition"));
    cmd_verify_build(&build_out).expect("verify build artifacts");
    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn build_writes_client_duplicate_signal_slot_cursor_contract() {
    let out = temp_output_dir("client-reactive-duplicate-slot-cursors");
    std::fs::create_dir_all(&out).expect("create temp root");
    let entry = out.join("page.orv");
    std::fs::write(
        &entry,
        r#"let sig first: string = "same"
let sig second: string = "same"
@out @html { @body {
  @p first
  @p second
  @input value={first}
  @input value={second}
} }"#,
    )
    .expect("write entry");
    let build_out = out.join("dist");

    cmd_build(&entry, &build_out).expect("build artifacts");

    let reactive_plan =
        read_json_value(&build_out.join(CLIENT_REACTIVE_PLAN_PATH)).expect("reactive plan");
    let text_bindings = reactive_plan["bindings"]
        .as_array()
        .expect("bindings")
        .iter()
        .filter(|binding| binding["kind"] == "signal_text")
        .collect::<Vec<_>>();
    assert_eq!(text_bindings.len(), 2);
    assert!(text_bindings
        .iter()
        .any(|binding| binding["state_key"] == "first" && binding["selector"] == "p"));
    assert!(text_bindings
        .iter()
        .any(|binding| binding["state_key"] == "second" && binding["selector"] == "p"));
    let attr_bindings = reactive_plan["bindings"]
        .as_array()
        .expect("bindings")
        .iter()
        .filter(|binding| binding["kind"] == "signal_attr")
        .collect::<Vec<_>>();
    assert_eq!(attr_bindings.len(), 2);
    assert!(attr_bindings.iter().any(|binding| {
        binding["state_key"] == "first"
            && binding["selector"] == "input"
            && binding["attr"] == "value"
    }));
    assert!(attr_bindings.iter().any(|binding| {
        binding["state_key"] == "second"
            && binding["selector"] == "input"
            && binding["attr"] == "value"
    }));
    let loader = std::fs::read_to_string(build_out.join(CLIENT_JS_PATH)).expect("client loader");
    assert!(loader.contains("signalTextBindingCursorKey"));
    assert!(loader.contains("signalAttrBindingCursorKey"));
    cmd_verify_build(&build_out).expect("verify build artifacts");
    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn build_writes_client_signal_attr_binding_contract() {
    let out = temp_output_dir("client-reactive-attr-binding");
    std::fs::create_dir_all(&out).expect("create temp root");
    let entry = out.join("page.orv");
    std::fs::write(
        &entry,
        "let sig input: string = \"hi\"\n@out @html { @body { @input value={input} } }",
    )
    .expect("write entry");
    let build_out = out.join("dist");

    cmd_build(&entry, &build_out).expect("build artifacts");

    let reactive_plan =
        read_json_value(&build_out.join(CLIENT_REACTIVE_PLAN_PATH)).expect("reactive plan");
    assert!(reactive_plan["bindings"]
        .as_array()
        .expect("bindings")
        .iter()
        .any(|binding| binding["kind"] == "signal_attr"
            && binding["target"] == CLIENT_PAGE_PATH
            && binding["state_key"] == "input"
            && binding["selector"] == "input"
            && binding["attr"] == "value"
            && binding["source"].as_str().is_some_and(|id| !id.is_empty())));
    let loader = std::fs::read_to_string(build_out.join(CLIENT_JS_PATH)).expect("client loader");
    assert_client_loader_contract(&loader);
    cmd_verify_build(&build_out).expect("verify build artifacts");
    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn build_writes_client_signal_attr_template_binding_contract() {
    let out = temp_output_dir("client-reactive-attr-template-binding");
    std::fs::create_dir_all(&out).expect("create temp root");
    let entry = out.join("page.orv");
    std::fs::write(
        &entry,
        r#"let sig input: string = "hi"
@out @html { @body { @input placeholder="{input}!" } }"#,
    )
    .expect("write entry");
    let build_out = out.join("dist");

    cmd_build(&entry, &build_out).expect("build artifacts");

    let reactive_plan =
        read_json_value(&build_out.join(CLIENT_REACTIVE_PLAN_PATH)).expect("reactive plan");
    assert!(reactive_plan["bindings"]
        .as_array()
        .expect("bindings")
        .iter()
        .any(|binding| binding["kind"] == "signal_attr"
            && binding["target"] == CLIENT_PAGE_PATH
            && binding["state_key"] == "input"
            && binding["selector"] == "input"
            && binding["attr"] == "placeholder"
            && binding["attr_template"]
                .as_array()
                .is_some_and(|segments| segments.iter().any(|segment| {
                    segment["kind"] == "signal" && segment["state_key"] == "input"
                }))
            && binding["source"].as_str().is_some_and(|id| !id.is_empty())));
    let loader = std::fs::read_to_string(build_out.join(CLIENT_JS_PATH)).expect("client loader");
    assert!(loader.contains("renderSignalAttrBinding"));
    assert!(loader.contains("attr_template"));
    cmd_verify_build(&build_out).expect("verify build artifacts");
    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn build_writes_client_multi_signal_attr_template_binding_contract() {
    let out = temp_output_dir("client-reactive-multi-signal-attr-template-binding");
    std::fs::create_dir_all(&out).expect("create temp root");
    let entry = out.join("page.orv");
    std::fs::write(
        &entry,
        r#"let sig prefix: string = "cart"
let sig count: int = 0
@out @html { @body { @input placeholder="{prefix}-{count}" } }"#,
    )
    .expect("write entry");
    let build_out = out.join("dist");

    cmd_build(&entry, &build_out).expect("build artifacts");

    let reactive_plan =
        read_json_value(&build_out.join(CLIENT_REACTIVE_PLAN_PATH)).expect("reactive plan");
    assert!(reactive_plan["bindings"]
        .as_array()
        .expect("bindings")
        .iter()
        .any(|binding| binding["kind"] == "signal_attr"
            && binding["target"] == CLIENT_PAGE_PATH
            && binding["selector"] == "input"
            && binding["attr"] == "placeholder"
            && binding["state_keys"] == serde_json::json!(["prefix", "count"])
            && binding["sources"].as_array().is_some_and(|sources| {
                sources.iter().any(|source| source["state_key"] == "prefix")
                    && sources.iter().any(|source| source["state_key"] == "count")
            })
            && binding["attr_template"]
                == serde_json::json!([
                    {"kind": "signal", "state_key": "prefix"},
                    {"kind": "text", "value": "-"},
                    {"kind": "signal", "state_key": "count"},
                ])));
    let loader = std::fs::read_to_string(build_out.join(CLIENT_JS_PATH)).expect("client loader");
    assert!(loader.contains("signalAttrBindingStateKeys"));
    assert!(loader.contains("state_keys"));
    cmd_verify_build(&build_out).expect("verify build artifacts");
    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn build_writes_client_signal_attr_condition_binding_contract() {
    let out = temp_output_dir("client-reactive-attr-condition-binding");
    std::fs::create_dir_all(&out).expect("create temp root");
    let entry = out.join("page.orv");
    std::fs::write(
        &entry,
        r#"let sig active: bool = false
@out @html { @body { @button class={active ? "enabled" : "disabled"} "Save" } }"#,
    )
    .expect("write entry");
    let build_out = out.join("dist");

    cmd_build(&entry, &build_out).expect("build artifacts");

    let reactive_plan =
        read_json_value(&build_out.join(CLIENT_REACTIVE_PLAN_PATH)).expect("reactive plan");
    assert!(reactive_plan["bindings"]
        .as_array()
        .expect("bindings")
        .iter()
        .any(|binding| binding["kind"] == "signal_attr"
            && binding["target"] == CLIENT_PAGE_PATH
            && binding["state_key"] == "active"
            && binding["selector"] == "button"
            && binding["attr"] == "class"
            && binding["attr_condition"]["state_key"] == "active"
            && binding["attr_condition"]["truthy"] == "enabled"
            && binding["attr_condition"]["falsy"] == "disabled"
            && binding["source"].as_str().is_some_and(|id| !id.is_empty())));
    let loader = std::fs::read_to_string(build_out.join(CLIENT_JS_PATH)).expect("client loader");
    assert!(loader.contains("renderSignalAttrCondition"));
    assert!(loader.contains("attr_condition"));
    cmd_verify_build(&build_out).expect("verify build artifacts");
    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn build_writes_client_signal_attr_comparison_condition_binding_contract() {
    let out = temp_output_dir("client-reactive-attr-comparison-condition-binding");
    std::fs::create_dir_all(&out).expect("create temp root");
    let entry = out.join("page.orv");
    std::fs::write(
        &entry,
        r#"let sig count: int = 0
@out @html { @body { @button class={count > 0 ? "enabled" : "disabled"} "Save" } }"#,
    )
    .expect("write entry");
    let build_out = out.join("dist");

    cmd_build(&entry, &build_out).expect("build artifacts");

    let reactive_plan =
        read_json_value(&build_out.join(CLIENT_REACTIVE_PLAN_PATH)).expect("reactive plan");
    assert!(reactive_plan["bindings"]
        .as_array()
        .expect("bindings")
        .iter()
        .any(|binding| binding["kind"] == "signal_attr"
            && binding["target"] == CLIENT_PAGE_PATH
            && binding["state_key"] == "count"
            && binding["selector"] == "button"
            && binding["attr"] == "class"
            && binding["attr_condition"]["state_key"] == "count"
            && binding["attr_condition"]["op"] == "gt"
            && binding["attr_condition"]["rhs"]["kind"] == "int"
            && binding["attr_condition"]["rhs"]["value"] == "0"
            && binding["attr_condition"]["truthy"] == "enabled"
            && binding["attr_condition"]["falsy"] == "disabled"
            && binding["source"].as_str().is_some_and(|id| !id.is_empty())));
    let loader = std::fs::read_to_string(build_out.join(CLIENT_JS_PATH)).expect("client loader");
    assert!(loader.contains("compareSignalAttrCondition"));
    assert!(loader.contains("decodeSignalConditionOperand"));
    cmd_verify_build(&build_out).expect("verify build artifacts");
    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn build_writes_client_signal_event_binding_contract() {
    let out = temp_output_dir("client-reactive-event-binding");
    std::fs::create_dir_all(&out).expect("create temp root");
    let entry = out.join("page.orv");
    std::fs::write(
            &entry,
            "let sig count: int = 0\n@out @html { @body { @p count @button onClick={count += 1} \"+\" } }",
        )
        .expect("write entry");
    let build_out = out.join("dist");

    cmd_build(&entry, &build_out).expect("build artifacts");

    let reactive_plan =
        read_json_value(&build_out.join(CLIENT_REACTIVE_PLAN_PATH)).expect("reactive plan");
    assert!(reactive_plan["bindings"]
        .as_array()
        .expect("bindings")
        .iter()
        .any(|binding| binding["kind"] == "signal_event"
            && binding["target"] == CLIENT_PAGE_PATH
            && binding["state_key"] == "count"
            && binding["selector"] == "button"
            && binding["event"] == "click"
            && binding["action"]["kind"] == "assign_add"
            && binding["action"]["value"]["kind"] == "int"
            && binding["action"]["value"]["value"] == "1"
            && binding["source"].as_str().is_some_and(|id| !id.is_empty())));
    let loader = std::fs::read_to_string(build_out.join(CLIENT_JS_PATH)).expect("client loader");
    assert_client_loader_contract(&loader);
    cmd_verify_build(&build_out).expect("verify build artifacts");
    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn build_writes_client_signal_event_toggle_binding_contract() {
    let out = temp_output_dir("client-reactive-event-toggle-binding");
    std::fs::create_dir_all(&out).expect("create temp root");
    let entry = out.join("page.orv");
    std::fs::write(
            &entry,
            "let sig muted: bool = false\n@out @html { @body { @button onClick={muted = !muted} \"mute\" } }",
        )
        .expect("write entry");
    let build_out = out.join("dist");

    cmd_build(&entry, &build_out).expect("build artifacts");

    let reactive_plan =
        read_json_value(&build_out.join(CLIENT_REACTIVE_PLAN_PATH)).expect("reactive plan");
    assert!(reactive_plan["bindings"]
        .as_array()
        .expect("bindings")
        .iter()
        .any(|binding| binding["kind"] == "signal_event"
            && binding["target"] == CLIENT_PAGE_PATH
            && binding["state_key"] == "muted"
            && binding["selector"] == "button"
            && binding["event"] == "click"
            && binding["action"]["kind"] == "assign_toggle"
            && binding["source"].as_str().is_some_and(|id| !id.is_empty())));
    let loader = std::fs::read_to_string(build_out.join(CLIENT_JS_PATH)).expect("client loader");
    assert!(loader.contains("assign_toggle"));
    cmd_verify_build(&build_out).expect("verify build artifacts");
    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn build_writes_client_signal_event_input_value_binding_contract() {
    let out = temp_output_dir("client-reactive-event-input-value-binding");
    std::fs::create_dir_all(&out).expect("create temp root");
    let entry = out.join("page.orv");
    std::fs::write(
        &entry,
        r#"let sig email: string = ""
@out @html { @body { @input value={email} onInput={(e) -> email = e.target.value} } }"#,
    )
    .expect("write entry");
    let build_out = out.join("dist");

    cmd_build(&entry, &build_out).expect("build artifacts");

    let reactive_plan =
        read_json_value(&build_out.join(CLIENT_REACTIVE_PLAN_PATH)).expect("reactive plan");
    assert!(reactive_plan["bindings"]
        .as_array()
        .expect("bindings")
        .iter()
        .any(|binding| binding["kind"] == "signal_event"
            && binding["target"] == CLIENT_PAGE_PATH
            && binding["state_key"] == "email"
            && binding["selector"] == "input"
            && binding["event"] == "input"
            && binding["action"]["kind"] == "assign_event_target_value"
            && binding["source"].as_str().is_some_and(|id| !id.is_empty())));
    let loader = std::fs::read_to_string(build_out.join(CLIENT_JS_PATH)).expect("client loader");
    assert!(loader.contains("assign_event_target_value"));
    cmd_verify_build(&build_out).expect("verify build artifacts");
    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn build_writes_client_signal_event_input_checked_binding_contract() {
    let out = temp_output_dir("client-reactive-event-input-checked-binding");
    std::fs::create_dir_all(&out).expect("create temp root");
    let entry = out.join("page.orv");
    std::fs::write(
            &entry,
            r#"let sig accepted: bool = false
@out @html { @body { @input type="checkbox" checked={accepted} onChange={(e) -> accepted = e.target.checked} } }"#,
        )
        .expect("write entry");
    let build_out = out.join("dist");

    cmd_build(&entry, &build_out).expect("build artifacts");

    let reactive_plan =
        read_json_value(&build_out.join(CLIENT_REACTIVE_PLAN_PATH)).expect("reactive plan");
    assert!(reactive_plan["bindings"]
        .as_array()
        .expect("bindings")
        .iter()
        .any(|binding| binding["kind"] == "signal_event"
            && binding["target"] == CLIENT_PAGE_PATH
            && binding["state_key"] == "accepted"
            && binding["selector"] == "input"
            && binding["event"] == "change"
            && binding["action"]["kind"] == "assign_event_target_checked"
            && binding["source"].as_str().is_some_and(|id| !id.is_empty())));
    let loader = std::fs::read_to_string(build_out.join(CLIENT_JS_PATH)).expect("client loader");
    assert!(loader.contains("assign_event_target_checked"));
    cmd_verify_build(&build_out).expect("verify build artifacts");
    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn build_writes_client_signal_event_numeric_input_value_binding_contract() {
    let out = temp_output_dir("client-reactive-event-numeric-input-binding");
    std::fs::create_dir_all(&out).expect("create temp root");
    let entry = out.join("page.orv");
    std::fs::write(
        &entry,
        r#"let sig volume: float = 1.0
let sig quantity: int = 1
@out @html { @body {
  @input value={volume} onInput={(e) -> volume = float.from(e.target.value)}
  @input value={quantity} onInput={(e) -> quantity = int.from(e.target.value)}
} }"#,
    )
    .expect("write entry");
    let build_out = out.join("dist");

    cmd_build(&entry, &build_out).expect("build artifacts");

    let reactive_plan =
        read_json_value(&build_out.join(CLIENT_REACTIVE_PLAN_PATH)).expect("reactive plan");
    assert!(reactive_plan["bindings"]
        .as_array()
        .expect("bindings")
        .iter()
        .any(|binding| binding["kind"] == "signal_event"
            && binding["state_key"] == "volume"
            && binding["event"] == "input"
            && binding["action"]["kind"] == "assign_event_target_value_float"));
    assert!(reactive_plan["bindings"]
        .as_array()
        .expect("bindings")
        .iter()
        .any(|binding| binding["kind"] == "signal_event"
            && binding["state_key"] == "quantity"
            && binding["event"] == "input"
            && binding["action"]["kind"] == "assign_event_target_value_int"));
    let loader = std::fs::read_to_string(build_out.join(CLIENT_JS_PATH)).expect("client loader");
    assert!(loader.contains("assign_event_target_value_float"));
    assert!(loader.contains("assign_event_target_value_int"));
    cmd_verify_build(&build_out).expect("verify build artifacts");
    let _ = std::fs::remove_dir_all(&out);
}
