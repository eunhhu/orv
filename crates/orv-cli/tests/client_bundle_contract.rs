use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

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
    let output = Command::new(orv_bin())
        .args(args)
        .output()
        .expect("run orv");
    assert!(
        output.status.success(),
        "orv {args:?} failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn read_json(path: &Path) -> serde_json::Value {
    serde_json::from_str(&std::fs::read_to_string(path).expect("read json")).expect("json")
}

fn assert_keys(value: &serde_json::Value, expected: &[&str], context: &str) {
    let object = value
        .as_object()
        .unwrap_or_else(|| panic!("{context} must be an object"));
    let actual = object.keys().map(String::as_str).collect::<BTreeSet<_>>();
    let expected = expected.iter().copied().collect::<BTreeSet<_>>();
    assert_eq!(actual, expected, "{context} keys drifted");
}

#[test]
fn client_bundle_v1_freezes_public_artifact_graph() {
    let root = temp_output_dir("client-bundle-contract");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("temp root");
    let entry = root.join("page.orv");
    std::fs::write(
        &entry,
        r#"let sig count: int = 0
@out @html { @body { @p "count: {count}" @button onClick={count += 1} "+" } }"#,
    )
    .expect("write fixture");
    let build_out = root.join("dist");
    let entry_arg = entry.display().to_string();
    let build_out_arg = build_out.display().to_string();

    run_orv(&["build", &entry_arg, "--out", &build_out_arg]);
    run_orv(&["verify-build", &build_out_arg]);

    assert_client_artifact_links(&build_out);
    let manifest = read_json(&build_out.join("client").join("manifest.json"));
    assert_manifest_contract(&manifest);
    let reactive_plan = read_json(&build_out.join("client").join("reactive-plan.json"));
    assert_reactive_plan_contract(&reactive_plan);
    assert_generated_client_files(&build_out);

    let _ = std::fs::remove_dir_all(&root);
}

fn assert_client_artifact_links(build_out: &Path) {
    let manifest = read_json(&build_out.join("build-manifest.json"));
    let artifacts = manifest["artifacts"]
        .as_array()
        .expect("manifest artifacts");
    let bundle_plan = read_json(&build_out.join("bundle-plan.json"));
    let bundles = bundle_plan["bundles"].as_array().expect("bundle targets");
    for (kind, path) in client_artifact_targets() {
        assert_json_target(artifacts, kind, path, "manifest artifact");
        assert_json_target(bundles, kind, path, "bundle target");
        assert!(
            build_out.join(path).is_file(),
            "client artifact path {path} must exist"
        );
    }
}

const fn client_artifact_targets() -> &'static [(&'static str, &'static str)] {
    &[
        ("client_manifest", "client/manifest.json"),
        ("client_reactive_plan", "client/reactive-plan.json"),
        ("client_page", "pages/index.html"),
        ("client_js", "client/app.js"),
        ("client_wasm", "client/app.wasm"),
    ]
}

fn assert_json_target(targets: &[serde_json::Value], kind: &str, path: &str, context: &str) {
    assert!(
        targets
            .iter()
            .any(|target| target["kind"] == kind && target["path"] == path),
        "{context} missing {kind} at {path}"
    );
}

fn assert_manifest_contract(manifest: &serde_json::Value) {
    assert_keys(
        manifest,
        &[
            "schema_version",
            "kind",
            "entry",
            "page",
            "reactive_plan",
            "reactive_plan_hash",
            "loader",
            "loader_hash",
            "wasm",
            "wasm_hash",
            "source_bundle",
            "source_bundle_hash",
            "exports",
            "initial_render",
            "runtime_features",
            "capabilities",
            "blocked_by",
            "blockers",
        ],
        "client manifest",
    );
    assert_eq!(manifest["schema_version"], serde_json::json!(1));
    assert_eq!(manifest["kind"], serde_json::json!("orv.client.bundle"));
    assert_eq!(manifest["page"], serde_json::json!("pages/index.html"));
    assert_eq!(
        manifest["reactive_plan"],
        serde_json::json!("client/reactive-plan.json")
    );
    assert_eq!(manifest["loader"], serde_json::json!("client/app.js"));
    assert_eq!(manifest["wasm"], serde_json::json!("client/app.wasm"));
    assert_eq!(
        manifest["source_bundle"],
        serde_json::json!("source-bundle.json")
    );
    assert_hash(&manifest["reactive_plan_hash"], "reactive_plan_hash");
    assert_hash(&manifest["loader_hash"], "loader_hash");
    assert_hash(&manifest["wasm_hash"], "wasm_hash");
    assert_hash(&manifest["source_bundle_hash"], "source_bundle_hash");
    assert_manifest_exports(&manifest["exports"]);
    assert_initial_render(&manifest["initial_render"]);
    assert_capabilities(&manifest["capabilities"]);
    assert_blocker(
        &manifest["blocked_by"],
        &manifest["blockers"],
        "dynamic-client-codegen",
    );
}

fn assert_hash(value: &serde_json::Value, context: &str) {
    assert!(
        value.as_str().is_some_and(|hash| {
            hash.len() == 16 && hash.as_bytes().iter().all(u8::is_ascii_hexdigit)
        }),
        "{context} must be a stable 16-hex hash"
    );
}

fn assert_manifest_exports(exports: &serde_json::Value) {
    assert_keys(
        exports,
        &["start", "render_ptr", "render_len", "memory"],
        "client manifest exports",
    );
    assert_eq!(exports["start"], serde_json::json!("orv_start"));
    assert_eq!(exports["render_ptr"], serde_json::json!("orv_render_ptr"));
    assert_eq!(exports["render_len"], serde_json::json!("orv_render_len"));
    assert_eq!(exports["memory"], serde_json::json!("memory"));
}

fn assert_initial_render(initial_render: &serde_json::Value) {
    assert_keys(
        initial_render,
        &["content_type", "encoding", "html_hash", "byte_length"],
        "client initial render",
    );
    assert_eq!(
        initial_render["content_type"],
        serde_json::json!("text/html")
    );
    assert_eq!(initial_render["encoding"], serde_json::json!("utf-8"));
    assert_hash(&initial_render["html_hash"], "initial_render.html_hash");
    assert!(initial_render["byte_length"].is_u64());
}

fn assert_capabilities(capabilities: &serde_json::Value) {
    assert_keys(
        capabilities,
        &[
            "schema_version",
            "runtime",
            "source",
            "signals",
            "bindings",
            "surfaces",
            "event_actions",
        ],
        "client capabilities",
    );
    assert_eq!(capabilities["schema_version"], serde_json::json!(1));
    assert_eq!(capabilities["runtime"], serde_json::json!("client_wasm"));
    assert_eq!(
        capabilities["source"],
        serde_json::json!("client/reactive-plan.json")
    );
    assert_eq!(capabilities["signals"], serde_json::json!(1));
    assert!(capabilities["surfaces"]
        .as_array()
        .expect("capability surfaces")
        .iter()
        .any(|surface| surface == "signal_text_template"));
    assert!(capabilities["event_actions"]
        .as_array()
        .expect("event actions")
        .iter()
        .any(|action| action == "assign_add"));
}

fn assert_reactive_plan_contract(reactive_plan: &serde_json::Value) {
    assert_keys(
        reactive_plan,
        &[
            "schema_version",
            "kind",
            "entry",
            "source_bundle",
            "source_bundle_hash",
            "runtime_features",
            "signals",
            "bindings",
            "blocked_by",
            "blockers",
        ],
        "client reactive plan",
    );
    assert_eq!(reactive_plan["schema_version"], serde_json::json!(1));
    assert_eq!(
        reactive_plan["kind"],
        serde_json::json!("orv.client.reactive_plan")
    );
    assert_eq!(
        reactive_plan["source_bundle"],
        serde_json::json!("source-bundle.json")
    );
    assert_hash(
        &reactive_plan["source_bundle_hash"],
        "reactive_plan.source_bundle_hash",
    );
    assert!(reactive_plan["signals"]
        .as_array()
        .expect("signals")
        .iter()
        .any(|signal| signal["name"] == "count" && signal["state_key"] == "count"));
    let bindings = reactive_plan["bindings"].as_array().expect("bindings");
    for kind in [
        "initial_render",
        "signal_state",
        "signal_text",
        "signal_event",
    ] {
        assert!(
            bindings.iter().any(|binding| binding["kind"] == kind),
            "client reactive plan missing binding {kind}"
        );
    }
    assert_blocker(
        &reactive_plan["blocked_by"],
        &reactive_plan["blockers"],
        "reactive-dom-diff",
    );
}

fn assert_blocker(blocked_by: &serde_json::Value, blockers: &serde_json::Value, id: &str) {
    assert!(blocked_by
        .as_array()
        .expect("blocked_by")
        .iter()
        .any(|item| item == id));
    assert!(blockers
        .as_array()
        .expect("blockers")
        .iter()
        .any(|blocker| blocker["id"] == id));
}

fn assert_generated_client_files(build_out: &Path) {
    let loader = std::fs::read_to_string(build_out.join("client/app.js")).expect("client loader");
    for marker in [
        "embeddedReactivePlan",
        "embeddedReactivePlanHash",
        "sourceBundleHash",
        "orvReactiveBindings",
    ] {
        assert!(loader.contains(marker), "client loader missing {marker}");
    }
    let wasm = std::fs::read(build_out.join("client/app.wasm")).expect("client wasm");
    assert!(
        wasm.starts_with(b"\0asm"),
        "client wasm must be a wasm module"
    );
}
