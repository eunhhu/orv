use crate::support::{read_json, read_text, run_orv, temp_dir};
use std::path::PathBuf;

use serde_json::{json, Value};

const DB_PERSISTENCE_GOLDEN: &str =
    include_str!("../../../docs/samples/db-persistence-v1.golden.json");

struct PersistenceFixture {
    root: PathBuf,
    manifest: Value,
    runtime: Value,
    deploy: Value,
    container: Value,
    preflight: Value,
    compose: String,
    env_example: String,
    runbook: String,
}

#[test]
fn db_persistence_v1_freezes_local_wal_sqlite_deploy_handoff() {
    let fixture = build_persistence_fixture();

    assert_runtime_feature_contract(&fixture);
    assert_persistence_artifact_contract(&fixture);
    assert_deploy_env_handoff_contract(&fixture);
    assert_runbook_contract(&fixture);
    assert_eq!(
        db_persistence_inventory(&fixture),
        db_persistence_golden(),
        "DB Persistence v1 golden drift"
    );

    let _ = std::fs::remove_dir_all(fixture.root);
}

fn build_persistence_fixture() -> PersistenceFixture {
    let root = temp_dir("db-persistence-contract");
    let out = root.join("dist");
    let source = root.join("app.orv");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("create temp root");
    std::fs::write(
        &source,
        r#"@server {
  @listen 8080
  let waldb = @db.connect "file://data/app.wal.jsonl"
  let shopdb = @db.connect(@env.SHOP_DATABASE_URL ?? "sqlite://data/app.sqlite")
  @route GET /ping { @respond 200 { ok: true } }
}
"#,
    )
    .expect("write source");
    let source_arg = source.display().to_string();
    let out_arg = out.display().to_string();

    run_orv(&["build", &source_arg, "--prod", "--out", &out_arg]);
    run_orv(&["verify-build", &out_arg]);
    run_orv(&["deploy-env-check", &out_arg]);

    PersistenceFixture {
        root,
        manifest: read_json(&out.join("build-manifest.json")),
        runtime: read_json(&out.join("server").join("app.orv-runtime.json")),
        deploy: read_json(&out.join("deploy").join("manifest.json")),
        container: read_json(&out.join("deploy").join("container.json")),
        preflight: read_json(&out.join("deploy").join("preflight.json")),
        compose: read_text(&out.join("deploy").join("compose.yaml")),
        env_example: read_text(&out.join("deploy").join("env.example")),
        runbook: read_text(&out.join("deploy").join("README.md")),
    }
}

fn assert_runtime_feature_contract(fixture: &PersistenceFixture) {
    assert_has_feature(
        &fixture.manifest["capabilities"]["runtime_features"],
        "db_adapter",
    );
    assert_has_feature(&fixture.runtime["runtime_features"], "db_adapter");
    assert_has_feature(&fixture.deploy["server"]["runtime_features"], "db_adapter");
    assert_has_feature(&fixture.preflight["runtime_features"], "db_adapter");
    assert_eq!(
        fixture.preflight["runtime_features"],
        fixture.runtime["runtime_features"]
    );
}

fn assert_has_feature(features: &Value, expected: &str) {
    assert!(
        features
            .as_array()
            .expect("runtime feature array")
            .iter()
            .any(|feature| feature == expected),
        "missing runtime feature {expected}: {features}"
    );
}

fn db_persistence_golden() -> Value {
    serde_json::from_str(DB_PERSISTENCE_GOLDEN).expect("DB persistence golden")
}

fn db_persistence_inventory(fixture: &PersistenceFixture) -> Value {
    let persistence = &fixture.deploy["server"]["persistence"];
    serde_json::json!({
        "schema_version": 1,
        "kind": "orv.db_persistence.inventory",
        "runtime_features": {
            "manifest_has_db_adapter": has_feature(&fixture.manifest["capabilities"]["runtime_features"], "db_adapter"),
            "runtime_has_db_adapter": has_feature(&fixture.runtime["runtime_features"], "db_adapter"),
            "deploy_has_db_adapter": has_feature(&fixture.deploy["server"]["runtime_features"], "db_adapter"),
            "preflight_has_db_adapter": has_feature(&fixture.preflight["runtime_features"], "db_adapter"),
            "preflight_matches_runtime": fixture.preflight["runtime_features"] == fixture.runtime["runtime_features"],
        },
        "persistence": persistence,
        "handoff": {
            "container_matches_deploy": fixture.container["persistence"] == *persistence,
            "preflight_matches_deploy": fixture.preflight["persistence"] == *persistence,
            "compose_has_volume": fixture.compose.contains("../data:/app/data"),
            "compose_has_env_default": fixture.compose.contains(r#"SHOP_DATABASE_URL: "${SHOP_DATABASE_URL:-sqlite://data/app.sqlite}""#),
            "env_example_has_default": fixture.env_example.contains("SHOP_DATABASE_URL=sqlite://data/app.sqlite"),
            "required_env_count": fixture.preflight["required_env"]
                .as_array()
                .expect("required env")
                .len(),
            "runbook_has_wal": fixture.runbook.contains("- WAL: data/app.wal.jsonl"),
            "runbook_has_db": fixture.runbook.contains("- DB: data/app.sqlite"),
            "runbook_has_env": fixture
                .runbook
                .contains("- DB adapter env: SHOP_DATABASE_URL default sqlite://data/app.sqlite"),
            "runbook_has_compose_mount": fixture
                .runbook
                .contains("- Compose volume: ../data:/app/data"),
        }
    })
}

fn has_feature(features: &Value, expected: &str) -> bool {
    features
        .as_array()
        .expect("runtime feature array")
        .iter()
        .any(|feature| feature == expected)
}

fn assert_persistence_artifact_contract(fixture: &PersistenceFixture) {
    let persistence = &fixture.deploy["server"]["persistence"];
    assert_eq!(persistence["wal_paths"], json!(["data/app.wal.jsonl"]));
    assert_eq!(persistence["db_paths"], json!(["data/app.sqlite"]));
    assert_eq!(
        persistence["db_env"],
        json!([
            {
                "env": "SHOP_DATABASE_URL",
                "default": "sqlite://data/app.sqlite"
            }
        ])
    );
    assert_eq!(fixture.container["persistence"], *persistence);
    assert_eq!(fixture.preflight["persistence"], *persistence);
    assert_eq!(persistence["db_endpoints"], json!([]));
    assert_eq!(persistence["db_adapters"], json!([]));
    assert_volume_contract(persistence);
}

fn assert_volume_contract(persistence: &Value) {
    let volumes = persistence["volumes"].as_array().expect("volumes");
    assert_eq!(volumes.len(), 1);
    assert_eq!(volumes[0]["host"], json!("data"));
    assert_eq!(volumes[0]["container"], json!("/app/data"));
    assert_eq!(volumes[0]["compose_mount"], json!("../data:/app/data"));
}

fn assert_deploy_env_handoff_contract(fixture: &PersistenceFixture) {
    assert!(fixture.compose.contains("../data:/app/data"));
    assert!(fixture
        .compose
        .contains(r#"SHOP_DATABASE_URL: "${SHOP_DATABASE_URL:-sqlite://data/app.sqlite}""#));
    assert!(fixture
        .env_example
        .contains("SHOP_DATABASE_URL=sqlite://data/app.sqlite"));
    assert!(fixture.preflight["required_env"]
        .as_array()
        .expect("required env")
        .is_empty());
}

fn assert_runbook_contract(fixture: &PersistenceFixture) {
    assert!(fixture.runbook.contains("- WAL: data/app.wal.jsonl"));
    assert!(fixture.runbook.contains("- DB: data/app.sqlite"));
    assert!(fixture
        .runbook
        .contains("- DB adapter env: SHOP_DATABASE_URL default sqlite://data/app.sqlite"));
    assert!(fixture
        .runbook
        .contains("- Compose volume: ../data:/app/data"));
}
