use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{json, Value};

const DB_ADAPTERS_GOLDEN: &str = include_str!("../../../docs/samples/db-adapters-v1.golden.json");

fn temp_dir(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time")
        .as_nanos();
    std::env::temp_dir().join(format!("orv-cli-{name}-{}-{nanos}", std::process::id()))
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

fn run_orv_json(args: &[&str]) -> Value {
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
    serde_json::from_slice(&output.stdout).expect("orv json")
}

fn read_text(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|err| panic!("read {}: {err}", path.display()))
}

fn read_json(path: &Path) -> Value {
    serde_json::from_str(&read_text(path))
        .unwrap_or_else(|err| panic!("parse {}: {err}", path.display()))
}

fn adapters_without_source_origin_ids(adapters: &Value) -> Value {
    Value::Array(
        adapters
            .as_array()
            .expect("adapters")
            .iter()
            .map(|adapter| {
                let mut adapter = adapter.clone();
                adapter
                    .as_object_mut()
                    .expect("adapter object")
                    .remove("source_origin_id");
                adapter
                    .as_object_mut()
                    .expect("adapter object")
                    .remove("source_origin_ids");
                adapter
            })
            .collect(),
    )
}

fn db_adapters_golden() -> Value {
    serde_json::from_str(DB_ADAPTERS_GOLDEN).expect("db adapters golden")
}

struct DbFixture {
    root: PathBuf,
    out_arg: String,
    deploy: Value,
    container: Value,
    adapters: Value,
    preflight: Value,
    compose: String,
    env_example: String,
    runbook: String,
    smoke_test: String,
}

#[test]
fn db_adapters_v1_freezes_external_bridge_artifacts() {
    let fixture = build_external_db_fixture();

    assert_adapter_artifact_contract(&fixture.adapters);
    assert_deploy_handoff_contract(&fixture);
    assert_preflight_and_smoke_contract(&fixture);
    assert_reveal_contract(&fixture);
    assert_eq!(
        db_adapters_inventory(&fixture),
        db_adapters_golden(),
        "DB Adapters v1 golden drift"
    );

    let _ = std::fs::remove_dir_all(fixture.root);
}

fn build_external_db_fixture() -> DbFixture {
    let root = temp_dir("db-adapters-contract");
    let out = root.join("dist");
    let source = root.join("app.orv");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("create temp root");
    std::fs::write(
        &source,
        r#"@server {
  @listen 8080
  let analytics = @db.connect "postgres://db.internal/shop"
  let shopdb = @db.connect(@env.SHOP_DATABASE_URL ?? "mysql://db.internal/shop")
  @route GET /ping { @respond 200 { ok: true } }
}
"#,
    )
    .expect("write source");
    let source_arg = source.display().to_string();
    let out_arg = out.display().to_string();

    run_orv(&["build", &source_arg, "--prod", "--out", &out_arg]);
    run_orv(&["verify-build", &out_arg]);

    DbFixture {
        root,
        out_arg,
        deploy: read_json(&out.join("deploy").join("manifest.json")),
        container: read_json(&out.join("deploy").join("container.json")),
        adapters: read_json(&out.join("deploy").join("db-adapters.json")),
        preflight: read_json(&out.join("deploy").join("preflight.json")),
        compose: read_text(&out.join("deploy").join("compose.yaml")),
        env_example: read_text(&out.join("deploy").join("env.example")),
        runbook: read_text(&out.join("deploy").join("README.md")),
        smoke_test: read_text(&out.join("deploy").join("smoke-test.sh")),
    }
}

fn assert_adapter_artifact_contract(adapters: &Value) {
    assert_eq!(adapters["schema_version"], json!(1));
    assert_eq!(adapters["artifact"], json!("server/app.orv-runtime.json"));
    let entries = adapters["adapters"].as_array().expect("db adapters");
    assert_eq!(entries.len(), 2);
    assert_external_adapter(
        &entries[0],
        "mysql",
        Some("SHOP_DATABASE_URL"),
        Some("mysql://db.internal/shop"),
        "mysql://db.internal/shop",
        "ORV_DB_ADAPTER_MYSQL_ENDPOINT",
        "ORV_DB_ADAPTER_MYSQL_AUTH_TOKEN",
    );
    assert_external_adapter(
        &entries[1],
        "postgres",
        None,
        None,
        "postgres://db.internal/shop",
        "ORV_DB_ADAPTER_POSTGRES_ENDPOINT",
        "ORV_DB_ADAPTER_POSTGRES_AUTH_TOKEN",
    );
}

fn assert_external_adapter(
    adapter: &Value,
    provider: &str,
    env: Option<&str>,
    default: Option<&str>,
    endpoint: &str,
    endpoint_env: &str,
    auth_env: &str,
) {
    assert_eq!(adapter["kind"], json!("db"));
    assert_eq!(adapter["mode"], json!("external"));
    assert_eq!(adapter["provider"], json!(provider));
    assert_eq!(
        adapter["env"],
        env.map_or(Value::Null, |value| json!(value))
    );
    assert_eq!(
        adapter["default"],
        default.map_or(Value::Null, |value| json!(value))
    );
    assert_eq!(adapter["endpoint"], json!(endpoint));
    assert_eq!(adapter["adapter_status"], json!("unsupported_runtime"));
    assert_eq!(adapter["runtime"]["status"], json!("unsupported_runtime"));
    assert_eq!(
        adapter["runtime"]["query_methods"],
        json!(["create", "find", "update", "delete", "transaction"])
    );
    assert!(adapter["source_origin_id"]
        .as_str()
        .is_some_and(|origin_id| origin_id.starts_with("ori_")));
    assert_eq!(
        adapter["source_origin_ids"],
        json!([adapter["source_origin_id"].as_str().expect("origin id")])
    );
    assert_bridge_contract(&adapter["bridge"], provider, endpoint_env, auth_env);
}

fn assert_bridge_contract(bridge: &Value, provider: &str, endpoint_env: &str, auth_env: &str) {
    assert_eq!(bridge["contract"], json!("http-json-v1"));
    assert_eq!(bridge["method"], json!("POST"));
    assert_eq!(bridge["content_type"], json!("application/json"));
    assert_eq!(
        bridge["query_methods"],
        json!([
            "create",
            "find",
            "findAll",
            "update",
            "delete",
            "upsert",
            "search",
            "count",
            "sum",
            "transaction",
            "schema"
        ])
    );
    assert_eq!(bridge["body"]["kind"], json!("orv.db.adapter"));
    assert_eq!(bridge["body"]["contract"], json!("http-json-v1"));
    assert_eq!(bridge["retry"]["attempts"], json!(3));
    assert!(bridge["retry"]["on"]
        .as_array()
        .expect("retry on")
        .iter()
        .any(|item| item == "5xx"));
    assert_bridge_env(bridge, provider, endpoint_env, auth_env);
}

fn assert_bridge_env(bridge: &Value, provider: &str, endpoint_env: &str, auth_env: &str) {
    let envs = bridge["env"].as_array().expect("bridge env");
    assert!(envs.iter().any(|env| {
        env["env"] == endpoint_env && env["required"] == true && env["purpose"] == "bridge_endpoint"
    }));
    assert!(envs.iter().any(|env| {
        env["env"] == auth_env && env["required"] == false && env["purpose"] == "bridge_auth_token"
    }));
    assert!(envs.iter().any(|env| {
        env["env"] == "ORV_DB_ADAPTER_ENDPOINT"
            && env["required"] == false
            && env["purpose"] == "bridge_endpoint_fallback"
    }));
    assert!(envs.iter().any(|env| {
        env["env"] == "ORV_DB_ADAPTER_AUTH_TOKEN"
            && env["required"] == false
            && env["purpose"] == "bridge_auth_token_fallback"
    }));
    let provider_marker = provider.to_ascii_uppercase();
    assert!(envs.iter().any(|env| env["env"]
        .as_str()
        .is_some_and(|name| name.contains(&provider_marker))));
}

fn assert_deploy_handoff_contract(fixture: &DbFixture) {
    assert_eq!(
        fixture.deploy["server"]["db_adapters"],
        json!("deploy/db-adapters.json")
    );
    assert_eq!(
        fixture.deploy["server"]["persistence"]["db_endpoints"],
        json!(["mysql://db.internal/shop", "postgres://db.internal/shop"])
    );
    assert_eq!(
        fixture.container["persistence"]["db_endpoints"],
        fixture.deploy["server"]["persistence"]["db_endpoints"]
    );
    assert!(fixture.container["persistence"]["volumes"]
        .as_array()
        .expect("volumes")
        .is_empty());
    assert!(fixture
        .compose
        .contains(r#"SHOP_DATABASE_URL: "${SHOP_DATABASE_URL:-mysql://db.internal/shop}""#));
    assert!(fixture
        .compose
        .contains(r#"ORV_DB_ADAPTER_MYSQL_ENDPOINT: "${ORV_DB_ADAPTER_MYSQL_ENDPOINT}""#));
    assert!(fixture
        .compose
        .contains(r#"ORV_DB_ADAPTER_POSTGRES_ENDPOINT: "${ORV_DB_ADAPTER_POSTGRES_ENDPOINT}""#));
    assert!(fixture
        .compose
        .contains(r#"ORV_DB_ADAPTER_ENDPOINT: "${ORV_DB_ADAPTER_ENDPOINT}""#));
    assert!(fixture
        .env_example
        .contains("SHOP_DATABASE_URL=mysql://db.internal/shop"));
    assert!(fixture
        .env_example
        .contains("ORV_DB_ADAPTER_MYSQL_ENDPOINT="));
    assert!(fixture
        .env_example
        .contains("ORV_DB_ADAPTER_POSTGRES_ENDPOINT="));
}

fn assert_preflight_and_smoke_contract(fixture: &DbFixture) {
    let required_env = fixture.preflight["required_env"]
        .as_array()
        .expect("required preflight env");
    for (provider, env) in [
        ("mysql", "ORV_DB_ADAPTER_MYSQL_ENDPOINT"),
        ("postgres", "ORV_DB_ADAPTER_POSTGRES_ENDPOINT"),
    ] {
        assert!(required_env.iter().any(|entry| {
            entry["env"] == env
                && entry["provider"] == provider
                && entry["purpose"] == "bridge_endpoint"
        }));
    }
    assert!(fixture
        .runbook
        .contains("- DB endpoint: mysql://db.internal/shop"));
    assert!(fixture
        .runbook
        .contains("- DB endpoint: postgres://db.internal/shop"));
    assert!(fixture
        .runbook
        .contains("- DB adapter env: SHOP_DATABASE_URL default mysql://db.internal/shop"));
    assert!(fixture
        .runbook
        .contains("- DB bridge env: mysql ORV_DB_ADAPTER_MYSQL_ENDPOINT required bridge_endpoint"));
    assert!(fixture
        .smoke_test
        .contains(r#"orv_smoke_file "deploy/db-adapters.json""#));
    assert!(fixture.smoke_test.contains(
        r#"orv_smoke_grep "db adapter bridge contract" "deploy/db-adapters.json" '"contract": "http-json-v1"'"#
    ));
    assert!(fixture.smoke_test.contains(
        r#"orv_smoke_db_bridge_schema "mysql bridge" "${ORV_DB_ADAPTER_MYSQL_ENDPOINT:-${ORV_DB_ADAPTER_ENDPOINT:-}}" "mysql" "mysql://db.internal/shop" "${ORV_DB_ADAPTER_MYSQL_AUTH_TOKEN:-${ORV_DB_ADAPTER_AUTH_TOKEN:-}}""#
    ));
}

fn assert_reveal_contract(fixture: &DbFixture) {
    let postgres_id = fixture.adapters["adapters"][1]["source_origin_id"]
        .as_str()
        .expect("postgres source origin id");
    let reveal = run_orv_json(&["reveal", &fixture.out_arg, postgres_id]);
    let target = reveal["production"]["db_adapters"]
        .as_array()
        .expect("db adapters")
        .iter()
        .find(|target| target["path"] == "deploy/db-adapters.json")
        .expect("db adapter target");
    let matched = target["matched_adapters"]
        .as_array()
        .expect("matched adapters");
    assert_eq!(target["matched"], json!(true));
    assert_eq!(matched[0]["source_origin_id"], json!(postgres_id));
    assert_eq!(matched[0]["provider"], json!("postgres"));
    assert_eq!(matched[0]["bridge"]["contract"], json!("http-json-v1"));
}

fn db_adapters_inventory(fixture: &DbFixture) -> Value {
    let postgres_id = fixture.adapters["adapters"][1]["source_origin_id"]
        .as_str()
        .expect("postgres source origin id");
    let reveal = run_orv_json(&["reveal", &fixture.out_arg, postgres_id]);
    let target = reveal["production"]["db_adapters"]
        .as_array()
        .expect("db adapters")
        .iter()
        .find(|target| target["path"] == "deploy/db-adapters.json")
        .expect("db adapter target");
    let matched = &target["matched_adapters"][0];
    let source_commands = target["source_reveal_commands"]
        .as_array()
        .expect("source reveal commands");
    json!({
        "schema_version": 1,
        "kind": "orv.db_adapters.inventory",
        "artifact": {
            "schema_version": fixture.adapters["schema_version"].clone(),
            "kind": fixture.adapters["kind"].clone(),
            "artifact": fixture.adapters["artifact"].clone(),
            "adapters": adapters_without_source_origin_ids(&fixture.adapters["adapters"]),
        },
        "source_origin_linkage": {
            "all_source_origins_present": fixture.adapters["adapters"].as_array().expect("adapters").iter().all(|adapter| {
                adapter["source_origin_id"].as_str().is_some_and(|origin_id| origin_id.starts_with("ori_"))
                    && adapter["source_origin_ids"].as_array().is_some_and(|ids| ids.len() == 1)
            }),
        },
        "deploy_handoff": {
            "manifest_path": fixture.deploy["server"]["db_adapters"].clone(),
            "db_endpoints": fixture.deploy["server"]["persistence"]["db_endpoints"].clone(),
            "container_endpoints_match_manifest": fixture.container["persistence"]["db_endpoints"] == fixture.deploy["server"]["persistence"]["db_endpoints"],
            "container_volume_count": fixture.container["persistence"]["volumes"].as_array().map_or(0, Vec::len),
            "compose": marker_inventory(&fixture.compose, &[
                r#"SHOP_DATABASE_URL: "${SHOP_DATABASE_URL:-mysql://db.internal/shop}""#,
                r#"ORV_DB_ADAPTER_MYSQL_ENDPOINT: "${ORV_DB_ADAPTER_MYSQL_ENDPOINT}""#,
                r#"ORV_DB_ADAPTER_POSTGRES_ENDPOINT: "${ORV_DB_ADAPTER_POSTGRES_ENDPOINT}""#,
                r#"ORV_DB_ADAPTER_ENDPOINT: "${ORV_DB_ADAPTER_ENDPOINT}""#,
            ]),
            "env_example": marker_inventory(&fixture.env_example, &[
                "SHOP_DATABASE_URL=mysql://db.internal/shop",
                "ORV_DB_ADAPTER_MYSQL_ENDPOINT=",
                "ORV_DB_ADAPTER_POSTGRES_ENDPOINT=",
            ]),
            "runbook": marker_inventory(&fixture.runbook, &[
                "- DB endpoint: mysql://db.internal/shop",
                "- DB endpoint: postgres://db.internal/shop",
                "- DB adapter env: SHOP_DATABASE_URL default mysql://db.internal/shop",
                "- DB bridge env: mysql ORV_DB_ADAPTER_MYSQL_ENDPOINT required bridge_endpoint",
            ]),
            "smoke": marker_inventory(&fixture.smoke_test, &[
                r#"orv_smoke_file "deploy/db-adapters.json""#,
                r#"orv_smoke_grep "db adapter bridge contract" "deploy/db-adapters.json" '"contract": "http-json-v1"'"#,
                r#"orv_smoke_db_bridge_schema "mysql bridge" "${ORV_DB_ADAPTER_MYSQL_ENDPOINT:-${ORV_DB_ADAPTER_ENDPOINT:-}}" "mysql" "mysql://db.internal/shop" "${ORV_DB_ADAPTER_MYSQL_AUTH_TOKEN:-${ORV_DB_ADAPTER_AUTH_TOKEN:-}}""#,
            ]),
        },
        "env_gate": {
            "required": preflight_env_inventory(&fixture.preflight["required_env"], &[
                "ORV_DB_ADAPTER_MYSQL_ENDPOINT",
                "ORV_DB_ADAPTER_POSTGRES_ENDPOINT",
            ]),
        },
        "reveal": {
            "target_kind": target["kind"].clone(),
            "target_path": target["path"].clone(),
            "matched": target["matched"].clone(),
            "matched_adapter_count": target["matched_adapter_count"].clone(),
            "matched_adapter": {
                "kind": matched["kind"].clone(),
                "provider": matched["provider"].clone(),
                "endpoint": matched["endpoint"].clone(),
                "bridge_contract": matched["bridge"]["contract"].clone(),
                "match": matched["match"].clone(),
            },
            "source_reveal_command_count": source_commands.len(),
            "selected_source_reveal_command": source_commands.iter().find(|command| {
                command["source_origin_id"] == target["selected_origin_id"]
            }).map(|command| {
                let argv = command["command"].as_array().expect("reveal command argv");
                json!({
                    "kind": command["kind"].clone(),
                    "provider": command["provider"].clone(),
                    "argv_len": argv.len(),
                    "argv_prefix": argv.iter().take(3).cloned().collect::<Vec<_>>(),
                    "source_origin_matches": command["source_origin_id"] == target["selected_origin_id"],
                })
            }).unwrap_or(Value::Null),
        },
    })
}

fn preflight_env_inventory(envs: &Value, names: &[&str]) -> Vec<Value> {
    let envs = envs.as_array().expect("preflight env array");
    names
        .iter()
        .map(|name| {
            let env = envs
                .iter()
                .find(|env| env["env"] == *name)
                .unwrap_or_else(|| panic!("missing preflight env {name}"));
            json!({
                "env": env["env"].clone(),
                "required": env["required"].clone(),
                "purpose": env["purpose"].clone(),
                "provider": env.get("provider").cloned().unwrap_or(Value::Null),
            })
        })
        .collect()
}

fn marker_inventory(text: &str, markers: &[&str]) -> Vec<Value> {
    markers
        .iter()
        .map(|marker| {
            json!({
                "marker": marker,
                "present": text.contains(marker),
            })
        })
        .collect()
}
