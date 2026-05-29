use std::path::{Path, PathBuf};
use std::process::Command;

const SHOP_ACCEPTANCE_RUNNER_GOLDEN: &str =
    include_str!("../../../docs/samples/shop-acceptance-runner-v1.golden.json");

fn temp_output_dir(name: &str) -> PathBuf {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
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

fn run_orv(args: &[&str], cwd: Option<&Path>) {
    let mut command = Command::new(orv_bin());
    command.args(args);
    if let Some(cwd) = cwd {
        command.current_dir(cwd);
    }
    let status = command.status().expect("run orv");
    assert!(status.success(), "orv {args:?} failed with {status}");
}

fn read_json(path: &Path) -> serde_json::Value {
    serde_json::from_str(&std::fs::read_to_string(path).expect("read json")).expect("json")
}

fn read_text(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|err| panic!("read {}: {err}", path.display()))
}

fn shop_acceptance_runner_golden() -> serde_json::Value {
    serde_json::from_str(SHOP_ACCEPTANCE_RUNNER_GOLDEN).expect("shop acceptance runner golden")
}

const fn expected_smoke_markers() -> &'static [&'static str] {
    &[
        "pass_marker",
        "build_dir",
        "base_url",
        "graph_contract",
        "dap_summary",
        "dap_source_bundle",
        "server_routes",
        "trace_stream_requested",
    ]
}

fn assert_json_string_array(value: &serde_json::Value, expected: &[&str], context: &str) {
    let actual = value
        .as_array()
        .unwrap_or_else(|| panic!("{context} must be an array"))
        .iter()
        .map(|item| {
            item.as_str()
                .unwrap_or_else(|| panic!("{context} item must be a string"))
        })
        .collect::<Vec<_>>();
    assert_eq!(actual, expected, "{context} drifted");
}

fn assert_acceptance_runner_contract() {
    let script = read_text(&workspace_root().join("scripts/shop_acceptance_smoke.sh"));
    for marker in [
        r#""$ORV_BIN" init "$SHOP_DIR" --template shop"#,
        r#""$ORV_BIN" check ."#,
        r#""$ORV_BIN" build . --prod --out dist"#,
        r#""$ORV_BIN" verify-build dist"#,
        r#""$ORV_BIN" deploy-env-check dist"#,
        r#""$ORV_BIN" run-build dist &"#,
        r#"ORV_BIN="$ORV_BIN" sh dist/deploy/smoke-test.sh"#,
        r#""$ORV_BIN" benchmark-report dist > dist/deploy/benchmark-report.json"#,
        "shop acceptance smoke passed",
        "smoke_output=%s",
        "benchmark_report=%s",
    ] {
        assert!(
            script.contains(marker),
            "shop acceptance runner missing marker {marker:?}"
        );
    }
}

#[test]
fn shop_acceptance_artifacts_expose_human_pass_gate_and_failure_classification() {
    let root = temp_output_dir("shop-acceptance-contract");
    let shop = root.join("shop");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("temp root");
    let shop_arg = shop.display().to_string();

    run_orv(&["init", &shop_arg, "--template", "shop"], None);
    let readme = std::fs::read_to_string(shop.join("README.md")).expect("shop README");
    assert!(readme.contains("orv benchmark-report dist --require-pass"));

    run_orv(&["check", "."], Some(&shop));
    run_orv(&["build", ".", "--prod", "--out", "dist"], Some(&shop));
    run_orv(&["verify-build", "dist"], Some(&shop));
    run_orv(&["deploy-env-check", "dist"], Some(&shop));

    let preflight = assert_preflight_acceptance_contract(&shop);
    assert_generated_smoke_contract(&shop);
    assert_acceptance_runner_contract();
    assert_benchmark_evidence_contract(&shop, &preflight);
    assert_eq!(
        shop_acceptance_runner_inventory(&shop, &preflight),
        shop_acceptance_runner_golden(),
        "Shop Acceptance Smoke v1 runner golden drift"
    );

    let _ = std::fs::remove_dir_all(&root);
}

fn shop_acceptance_runner_inventory(
    shop: &Path,
    preflight: &serde_json::Value,
) -> serde_json::Value {
    let script = read_text(&workspace_root().join("scripts/shop_acceptance_smoke.sh"));
    let smoke = read_text(&shop.join("dist").join("deploy").join("smoke-test.sh"));
    let evidence = read_json(
        &shop
            .join("dist")
            .join("deploy")
            .join("benchmark-evidence.json"),
    );
    let notes_template = read_text(
        &shop
            .join("dist")
            .join("deploy")
            .join("participant-notes-template.md"),
    );
    serde_json::json!({
        "schema_version": 1,
        "kind": "orv.shop_acceptance.runner_inventory",
        "runner": {
            "path": "scripts/shop_acceptance_smoke.sh",
            "env": marker_inventory(&script, &[
                "ORV_BIN",
                "ORV_SHOP_ACCEPTANCE_DIR",
                "ORV_BASE_URL",
                "ORV_SHOP_ACCEPTANCE_READY_ATTEMPTS",
            ]),
            "command_order": marker_inventory(&script, &[
                r#""$ORV_BIN" init "$SHOP_DIR" --template shop"#,
                r#""$ORV_BIN" check ."#,
                r#""$ORV_BIN" build . --prod --out dist"#,
                r#""$ORV_BIN" verify-build dist"#,
                r#""$ORV_BIN" deploy-env-check dist"#,
                r#""$ORV_BIN" run-build dist &"#,
                r#"ORV_BIN="$ORV_BIN" sh dist/deploy/smoke-test.sh"#,
                r#""$ORV_BIN" benchmark-report dist > dist/deploy/benchmark-report.json"#,
            ]),
            "lifecycle": {
                "cleanup_trap": script.contains("trap cleanup EXIT INT TERM"),
                "kills_server_pid": script.contains(r#"kill "$SERVER_PID""#),
                "waits_for_home": script.contains(r#"curl -fsS "${ORV_BASE_URL:-http://127.0.0.1:8080}/""#),
            },
            "stdout_handoff": marker_inventory(&script, &[
                "shop acceptance smoke passed",
                "shop_dir=%s",
                "smoke_output=%s",
                "benchmark_report=%s",
            ]),
        },
        "generated": {
            "commands": {
                "run_build": preflight["commands"]["run_build"].clone(),
                "smoke_test": preflight["commands"]["smoke_test"].clone(),
                "benchmark_prepare": preflight["commands"]["benchmark_prepare"].clone(),
                "benchmark_report": preflight["commands"]["benchmark_report"].clone(),
                "benchmark_report_require_pass": preflight["commands"]["benchmark_report_require_pass"].clone(),
            },
            "artifacts": {
                "participant_notes_template": preflight["artifacts"]["participant_notes_template"].clone(),
            },
            "smoke_output_contract": preflight["smoke_output_contract"].clone(),
            "smoke_script": marker_inventory(&smoke, &[
                "orv deploy smoke test passed",
                "graph_contract=verified",
                "dap_summary=verified",
                "dap_source_bundle=verified",
                "server_routes=",
                "trace_stream_requested=%s",
            ]),
            "benchmark_evidence": {
                "benchmark_matches_preflight": evidence["benchmark"] == preflight["benchmark"],
                "smoke_contract_matches_preflight": evidence["smoke_output_contract"] == preflight["smoke_output_contract"],
                "artifacts_match_preflight": evidence["artifacts"] == preflight["artifacts"],
                "participant_run_count": evidence["data"]["participant_runs"].as_array().map_or(0, Vec::len),
                "recommended_participant_count": evidence["data"]["recommended_participant_count"].clone(),
                "failure_categories": evidence["data"]["failure_classification"]["allowed_categories"].clone(),
                "smoke_required_markers": evidence["data"]["smoke_test_required_markers"].clone(),
            },
            "participant_notes_template": marker_inventory(&notes_template, &[
                "data.participant_runs[].raw_notes_artifact",
                "participant_profile: non_developer",
                "YYYY-MM-DDTHH:MM:SSZ",
                "generated_artifact_edits: false",
                "manual_undocumented_security_steps: false",
                "ai_assistance_used: false",
            ]),
        },
    })
}

fn marker_inventory(text: &str, markers: &[&str]) -> Vec<serde_json::Value> {
    markers
        .iter()
        .map(|marker| {
            serde_json::json!({
                "marker": marker,
                "present": text.contains(marker),
            })
        })
        .collect()
}

fn assert_preflight_acceptance_contract(shop: &Path) -> serde_json::Value {
    let preflight = read_json(&shop.join("dist").join("deploy").join("preflight.json"));
    assert!(preflight["benchmark"]["automated_gate"]
        .as_array()
        .expect("automated gate")
        .iter()
        .any(|item| item == "orv benchmark-report dist --require-pass"));
    assert!(preflight["benchmark"]["data_to_record"]
        .as_array()
        .expect("data to record")
        .iter()
        .any(|item| item == "failure classification"));
    assert_eq!(
        preflight["commands"]["run_build"],
        serde_json::json!("orv run-build .")
    );
    assert_eq!(
        preflight["commands"]["smoke_test"],
        serde_json::json!("./deploy/smoke-test.sh")
    );
    assert_eq!(
        preflight["commands"]["benchmark_prepare"],
        serde_json::json!("orv benchmark-prepare . --participants 2")
    );
    assert_eq!(
        preflight["commands"]["benchmark_report"],
        serde_json::json!("orv benchmark-report .")
    );
    assert_eq!(
        preflight["commands"]["benchmark_report_require_pass"],
        serde_json::json!("orv benchmark-report . --require-pass")
    );
    assert_eq!(
        preflight["smoke_output_contract"]["output"],
        serde_json::json!("deploy/smoke-output.txt")
    );
    assert_eq!(
        preflight["artifacts"]["participant_notes_template"],
        serde_json::json!("deploy/participant-notes-template.md")
    );
    assert_json_string_array(
        &preflight["smoke_output_contract"]["required_markers"],
        expected_smoke_markers(),
        "preflight smoke required markers",
    );
    preflight
}

fn assert_generated_smoke_contract(shop: &Path) {
    let smoke = read_text(&shop.join("dist").join("deploy").join("smoke-test.sh"));
    for marker in [
        "orv deploy smoke test passed",
        "graph_contract=verified",
        "dap_summary=verified",
        "dap_source_bundle=verified",
        "server_routes=",
        "trace_stream_requested=%s",
    ] {
        assert!(
            smoke.contains(marker),
            "generated smoke test missing marker {marker:?}"
        );
    }
}

fn assert_benchmark_evidence_contract(shop: &Path, preflight: &serde_json::Value) {
    let evidence = read_json(
        &shop
            .join("dist")
            .join("deploy")
            .join("benchmark-evidence.json"),
    );
    let failure = &evidence["data"]["failure_classification"];
    assert!(failure["primary"].is_null());
    let categories = failure["allowed_categories"]
        .as_array()
        .expect("failure categories");
    for category in [
        "syntax",
        "scaffold",
        "compiler_runtime_error",
        "editor",
        "documentation",
    ] {
        assert!(
            categories.iter().any(|item| item == category),
            "missing failure category {category}"
        );
    }
    assert_eq!(
        evidence["data"]["recommended_participant_count"]["minimum"],
        serde_json::json!(2)
    );
    assert_eq!(
        evidence["data"]["recommended_participant_count"]["target"],
        serde_json::json!(3)
    );
    let participant_runs = evidence["data"]["participant_runs"]
        .as_array()
        .expect("participant runs");
    assert_eq!(participant_runs.len(), 1);
    assert_eq!(participant_runs[0]["status"], "not_recorded");
    assert_eq!(participant_runs[0]["participant_profile"], "non_developer");
    assert_eq!(evidence["benchmark"], preflight["benchmark"]);
    assert_eq!(
        evidence["smoke_output_contract"],
        preflight["smoke_output_contract"]
    );
    assert_json_string_array(
        &evidence["data"]["smoke_test_required_markers"],
        expected_smoke_markers(),
        "evidence smoke required markers",
    );
    assert_eq!(
        evidence["artifacts"]["participant_notes_template"],
        serde_json::json!("deploy/participant-notes-template.md")
    );
    assert!(shop
        .join("dist")
        .join("deploy")
        .join("participant-notes-template.md")
        .is_file());
}
