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

    let _ = std::fs::remove_dir_all(&root);
}
