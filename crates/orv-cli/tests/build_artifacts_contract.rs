use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_dir(name: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    std::env::temp_dir().join(format!("orv-{name}-{}-{nonce}", std::process::id()))
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates dir")
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

const fn orv_bin() -> &'static str {
    env!("CARGO_BIN_EXE_orv")
}

fn run_orv(args: &[&str]) -> Output {
    Command::new(orv_bin())
        .args(args)
        .output()
        .expect("run orv")
}

fn assert_success(output: &Output, label: &str) {
    assert!(
        output.status.success(),
        "{label} failed\nstdout:\n{}\nstderr:\n{}",
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
fn build_artifacts_v1_freezes_common_build_artifact_shapes() {
    let entry = workspace_root()
        .join("fixtures")
        .join("e2e")
        .join("hello.orv");
    let out = temp_dir("build-artifacts-contract");
    let entry_arg = entry.display().to_string();
    let out_arg = out.display().to_string();

    let build = run_orv(&["build", &entry_arg, "--out", &out_arg]);
    assert_success(&build, "orv build");

    assert_build_manifest_contract(&read_json(&out.join("build-manifest.json")));
    assert_bundle_plan_contract(&read_json(&out.join("bundle-plan.json")));
    assert_source_bundle_contract(&read_json(&out.join("source-bundle.json")));
    let origin_map = read_json(&out.join("origin-map.json"));
    assert_origin_map_root_contract(&origin_map);
    assert_project_graph_root_contract(&read_json(&out.join("project-graph.json")), &origin_map);

    let verify = run_orv(&["verify-build", &out_arg]);
    assert_success(&verify, "orv verify-build");

    let _ = std::fs::remove_dir_all(out);
}

fn assert_build_manifest_contract(manifest: &serde_json::Value) {
    assert_keys(
        manifest,
        &[
            "schema_version",
            "entry",
            "runtime",
            "artifacts",
            "capabilities",
        ],
        "build manifest",
    );
    assert_eq!(manifest["schema_version"], serde_json::json!(1));
    assert_eq!(
        manifest["runtime"],
        serde_json::json!("reference-interpreter")
    );
    assert!(manifest["entry"].is_string());
    assert_keys(
        &manifest["capabilities"],
        &[
            "has_server",
            "server_routes",
            "client_wasm",
            "runtime_features",
        ],
        "build manifest capabilities",
    );
    assert_eq!(
        manifest["capabilities"]["has_server"],
        serde_json::json!(true)
    );
    assert_eq!(
        manifest["capabilities"]["server_routes"],
        serde_json::json!(1)
    );
    assert_eq!(
        manifest["capabilities"]["client_wasm"],
        serde_json::json!(false)
    );
    assert!(manifest["capabilities"]["runtime_features"]
        .as_array()
        .expect("runtime features")
        .iter()
        .any(|feature| feature == "http_server"));

    let artifacts = manifest["artifacts"].as_array().expect("artifacts array");
    for artifact in artifacts {
        assert_keys(artifact, &["kind", "path"], "build manifest artifact");
        assert!(artifact["kind"].is_string());
        assert!(artifact["path"].is_string());
    }
    assert_artifact(artifacts, "origin_map", "origin-map.json");
    assert_artifact(artifacts, "bundle_plan", "bundle-plan.json");
    assert_artifact(artifacts, "project_graph", "project-graph.json");
    assert_artifact(artifacts, "source_bundle", "source-bundle.json");
    assert_artifact(artifacts, "server_runtime", "server/app.orv-runtime.json");
}

fn assert_bundle_plan_contract(plan: &serde_json::Value) {
    assert_keys(plan, &["schema_version", "bundles"], "bundle plan");
    assert_eq!(plan["schema_version"], serde_json::json!(1));
    let bundles = plan["bundles"].as_array().expect("bundles array");
    for bundle in bundles {
        assert_keys(
            bundle,
            &["kind", "path", "runtime_features"],
            "bundle target",
        );
        assert!(bundle["kind"].is_string());
        assert!(bundle["path"].is_string());
        assert!(bundle["runtime_features"].is_array());
    }
    assert_bundle(bundles, "server_runtime", "server/app.orv-runtime.json");
    assert_bundle(bundles, "server_launcher", "server/launch.json");
    assert_bundle(bundles, "native_server_plan", "server/native-server.json");
}

fn assert_source_bundle_contract(source_bundle: &serde_json::Value) {
    assert_keys(
        source_bundle,
        &["schema_version", "entry", "files"],
        "source bundle",
    );
    assert_eq!(source_bundle["schema_version"], serde_json::json!(1));
    assert!(source_bundle["entry"].is_string());
    let files = source_bundle["files"].as_array().expect("files array");
    assert!(!files.is_empty(), "source bundle must include source files");
    for file in files {
        assert_keys(file, &["path", "content_hash", "source"], "source file");
        assert!(file["path"].is_string());
        assert!(file["content_hash"]
            .as_str()
            .is_some_and(|hash| hash.starts_with("fnv1a64:")));
        assert!(file["source"].is_string());
    }
}

fn assert_origin_map_root_contract(origin_map: &serde_json::Value) {
    assert_keys(origin_map, &["version", "entries", "edges"], "origin map");
    assert_eq!(origin_map["version"], serde_json::json!(2));
    assert!(origin_map["entries"].is_array());
    assert!(origin_map["edges"].is_array());
}

fn assert_project_graph_root_contract(graph: &serde_json::Value, origin_map: &serde_json::Value) {
    assert_keys(
        graph,
        &["schema_version", "stats", "nodes", "edges", "semantic"],
        "project graph",
    );
    assert_eq!(graph["schema_version"], serde_json::json!(1));
    assert!(graph["stats"].is_object());
    assert!(graph["nodes"].is_array());
    assert!(graph["edges"].is_array());
    assert_eq!(graph["semantic"]["origin_map"], *origin_map);
}

fn assert_artifact(artifacts: &[serde_json::Value], kind: &str, path: &str) {
    assert!(
        artifacts
            .iter()
            .any(|artifact| artifact["kind"] == kind && artifact["path"] == path),
        "missing manifest artifact {kind} at {path}"
    );
}

fn assert_bundle(bundles: &[serde_json::Value], kind: &str, path: &str) {
    assert!(
        bundles
            .iter()
            .any(|bundle| bundle["kind"] == kind && bundle["path"] == path),
        "missing bundle target {kind} at {path}"
    );
}
