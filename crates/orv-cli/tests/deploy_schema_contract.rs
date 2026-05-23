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
    let status = Command::new(orv_bin())
        .args(args)
        .status()
        .expect("run orv");
    assert!(status.success(), "orv {args:?} failed with {status}");
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

fn write_prod_server_fixture(out: &Path) -> PathBuf {
    let fixture = out.join("app.orv");
    std::fs::write(
        &fixture,
        r#"@server {
  @listen 8080

  @route GET /ping {
    @respond 200 { ok: true, msg: "pong" }
  }
}
"#,
    )
    .expect("write fixture");
    fixture
}

#[test]
fn prod_build_deploy_and_benchmark_json_contracts_freeze_public_shape() {
    let out = build_prod_contract_fixture();

    assert_build_manifest_contract(&read_json(&out.join("build-manifest.json")));
    assert_source_bundle_contract(&read_json(&out.join("source-bundle.json")));
    assert_bundle_plan_contract(&read_json(&out.join("bundle-plan.json")));
    assert_deploy_manifest_contract(&read_json(&out.join("deploy").join("manifest.json")));
    let preflight = read_json(&out.join("deploy").join("preflight.json"));
    assert_preflight_contract(&preflight);
    assert_benchmark_evidence_contract(
        &read_json(&out.join("deploy").join("benchmark-evidence.json")),
        &preflight,
    );

    let _ = std::fs::remove_dir_all(&out);
}

fn build_prod_contract_fixture() -> PathBuf {
    let out = temp_output_dir("deploy-schema-contract");
    let _ = std::fs::remove_dir_all(&out);
    std::fs::create_dir_all(&out).expect("temp output dir");
    let fixture = write_prod_server_fixture(&out);
    let fixture_arg = fixture.display().to_string();
    let out_arg = out.display().to_string();

    run_orv(&["build", &fixture_arg, "--out", &out_arg, "--prod"]);
    run_orv(&["verify-build", &out_arg]);

    out
}

fn assert_build_manifest_contract(build_manifest: &serde_json::Value) {
    assert_keys(
        build_manifest,
        &[
            "schema_version",
            "entry",
            "runtime",
            "artifacts",
            "capabilities",
        ],
        "build manifest",
    );
    assert_eq!(build_manifest["schema_version"], serde_json::json!(1));
    assert!(build_manifest["artifacts"].is_array());
    assert!(build_manifest["capabilities"].is_object());
}

fn assert_source_bundle_contract(source_bundle: &serde_json::Value) {
    assert_keys(
        source_bundle,
        &["schema_version", "entry", "files"],
        "source bundle",
    );
    assert_eq!(source_bundle["schema_version"], serde_json::json!(1));
    assert!(source_bundle["files"].is_array());
}

fn assert_bundle_plan_contract(bundle_plan: &serde_json::Value) {
    assert_keys(bundle_plan, &["schema_version", "bundles"], "bundle plan");
    assert_eq!(bundle_plan["schema_version"], serde_json::json!(1));
    assert!(bundle_plan["bundles"].is_array());
}

fn assert_deploy_manifest_contract(deploy: &serde_json::Value) {
    assert_keys(
        deploy,
        &[
            "schema_version",
            "profile",
            "entry",
            "runtime",
            "runtime_features",
            "source_bundle",
            "server",
            "static",
            "client",
        ],
        "deploy manifest",
    );
    assert_eq!(deploy["schema_version"], serde_json::json!(1));
    assert_eq!(deploy["profile"], serde_json::json!("prod"));
    assert_keys(
        &deploy["server"],
        &[
            "runtime",
            "runtime_features",
            "artifact",
            "entrypoint",
            "routes_artifact",
            "native_plan",
            "native_runtime_image_plan",
            "native_routes_source",
            "native_router_source",
            "native_handlers_source",
            "container",
            "dockerfile",
            "compose",
            "env_example",
            "db_adapters",
            "commerce_adapters",
            "smoke_test",
            "smoke_output",
            "preflight",
            "benchmark_evidence",
            "runbook",
            "runtime_image",
            "protocol",
            "listen",
            "routes",
            "persistence",
        ],
        "deploy manifest server",
    );
    assert!(deploy["server"]["routes"].is_array());
}

fn assert_preflight_contract(preflight: &serde_json::Value) {
    assert_keys(
        preflight,
        &[
            "schema_version",
            "kind",
            "artifact",
            "runtime",
            "runtime_features",
            "security_features",
            "listen",
            "routes",
            "persistence",
            "required_env",
            "optional_env",
            "commands",
            "artifacts",
            "smoke_output_contract",
            "benchmark",
            "client",
        ],
        "deploy preflight",
    );
    assert_eq!(preflight["schema_version"], serde_json::json!(1));
    assert_eq!(preflight["kind"], serde_json::json!("orv.deploy.preflight"));
    assert_keys(
        &preflight["commands"],
        &[
            "verify_build",
            "env_check",
            "run_build",
            "smoke_test",
            "editor_run_debug",
            "benchmark_report",
            "benchmark_report_require_pass",
            "compose_up",
            "trace",
            "trace_run_build",
            "editor_trace",
            "trace_stream_smoke",
        ],
        "deploy preflight commands",
    );
    assert_keys(
        &preflight["artifacts"],
        &[
            "server",
            "routes",
            "source_bundle",
            "project_graph",
            "origin_map",
            "build_manifest",
            "bundle_plan",
            "env_example",
            "db_adapters",
            "commerce_adapters",
            "smoke_test",
            "smoke_output",
            "preflight",
            "benchmark_evidence",
            "runbook",
        ],
        "deploy preflight artifacts",
    );
    assert_keys(
        &preflight["smoke_output_contract"],
        &["output", "required_markers"],
        "smoke output contract",
    );
    assert!(preflight["smoke_output_contract"]["required_markers"].is_array());
}

fn assert_benchmark_evidence_contract(evidence: &serde_json::Value, preflight: &serde_json::Value) {
    assert_keys(
        evidence,
        &[
            "schema_version",
            "kind",
            "preflight",
            "preflight_hash",
            "benchmark",
            "commands",
            "artifacts",
            "smoke_output_contract",
            "recording_status",
            "task_entries",
            "data",
        ],
        "benchmark evidence",
    );
    assert_eq!(evidence["schema_version"], serde_json::json!(1));
    assert_eq!(
        evidence["kind"],
        serde_json::json!("orv.benchmark.shop_5h.evidence")
    );
    assert!(evidence["preflight_hash"].is_string());
    assert_eq!(evidence["commands"], preflight["commands"]);
    assert_eq!(evidence["artifacts"], preflight["artifacts"]);
    assert_eq!(
        evidence["smoke_output_contract"],
        preflight["smoke_output_contract"]
    );
    assert!(evidence["task_entries"].is_array());
    assert!(evidence["data"].is_object());
}
