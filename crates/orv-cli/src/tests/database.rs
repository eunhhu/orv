use super::*;

#[test]
fn db_plan_subcommand_is_accepted() {
    let parsed = Cli::try_parse_from(["orv", "db", "plan", "fixtures/e2e/hello.orv"]);
    if let Err(err) = parsed {
        panic!("{}", err.render());
    }
}

#[test]
fn db_apply_subcommand_is_accepted() {
    let parsed = Cli::try_parse_from([
        "orv",
        "db",
        "apply",
        "fixtures/e2e/hello.orv",
        "--schema",
        "target/orv-db-schema.json",
        "--history",
        "target/orv-db-history.json",
    ]);
    if let Err(err) = parsed {
        panic!("{}", err.render());
    }
}

#[test]
fn db_migrate_subcommand_is_accepted() {
    let parsed = Cli::try_parse_from([
        "orv",
        "db",
        "migrate",
        "fixtures/e2e/hello.orv",
        "--schema",
        "target/orv-db-schema.json",
        "--history",
        "target/orv-db-history.json",
    ]);
    if let Err(err) = parsed {
        panic!("{}", err.render());
    }
}

#[test]
fn db_rollback_subcommand_is_accepted() {
    let parsed = Cli::try_parse_from([
        "orv",
        "db",
        "rollback",
        "--schema",
        "target/orv-db-schema.json",
    ]);
    if let Err(err) = parsed {
        panic!("{}", err.render());
    }
}

#[test]
fn db_plan_reports_added_nullable_field_from_applied_snapshot() {
    let dir = temp_output_dir("db-plan");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let source = dir.join("app.orv");
    std::fs::write(
        &source,
        r#"struct User {
  id: int
  email: string
  avatar: string?
}"#,
    )
    .expect("write source");
    let applied = dir.join("applied-schema.json");
    std::fs::write(
        &applied,
        r#"{
  "schema_version": 1,
  "structs": {
    "User": {
      "fields": {
        "id": { "type": "int", "optional": false },
        "email": { "type": "string", "optional": false }
      }
    }
  }
}"#,
    )
    .expect("write applied schema");

    let plan = db_plan_json(&source, Some(&applied)).expect("db plan");

    let actions = plan["actions"].as_array().expect("actions array");
    assert!(actions.iter().any(|action| {
        action["kind"] == "add_field"
            && action["struct"] == "User"
            && action["field"] == "avatar"
            && action["type"] == "string?"
            && action["optional"] == true
    }));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn db_apply_writes_current_schema_snapshot() {
    let dir = temp_output_dir("db-apply");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let source = dir.join("app.orv");
    std::fs::write(
        &source,
        r#"struct User {
  id: int
  email: string
}"#,
    )
    .expect("write source");
    let schema = dir.join("schema.json");

    cmd_db_apply(&source, &schema).expect("apply schema");

    let written = read_json_value(&schema).expect("read schema");
    assert_eq!(written["schema_version"], 1);
    assert_eq!(
        written["structs"]["User"]["fields"]["email"]["type"],
        "string"
    );
    let plan = db_plan_json(&source, Some(&schema)).expect("db plan after apply");
    assert_eq!(plan["actions"].as_array().expect("actions").len(), 0);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn db_apply_appends_migration_history_when_requested() {
    let dir = temp_output_dir("db-history");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let first_source = dir.join("first.orv");
    std::fs::write(
        &first_source,
        r#"struct User {
  id: int
  email: string
}"#,
    )
    .expect("write first source");
    let second_source = dir.join("second.orv");
    std::fs::write(
        &second_source,
        r#"struct User {
  id: int
  email: string
  avatar: string?
}"#,
    )
    .expect("write second source");
    let schema = dir.join("schema.json");
    let history = dir.join("history.json");

    cmd_db_apply_with_history(&first_source, &schema, Some(&history)).expect("apply first schema");
    cmd_db_apply_with_history(&second_source, &schema, Some(&history))
        .expect("apply second schema");

    let history = read_json_value(&history).expect("read history");
    assert_eq!(history["schema_version"], 1);
    let entries = history["entries"].as_array().expect("history entries");
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0]["actions"].as_array().expect("actions").len(), 1);
    assert!(entries[1]["actions"]
        .as_array()
        .expect("actions")
        .iter()
        .any(|action| action["kind"] == "add_field" && action["field"] == "avatar"));
    assert_ne!(entries[0]["schema_hash"], entries[1]["schema_hash"]);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn db_migrate_applies_schema_and_history() {
    let dir = temp_output_dir("db-migrate");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let source = dir.join("app.orv");
    std::fs::write(
        &source,
        r#"struct Order {
  id: int
  total: int
}"#,
    )
    .expect("write source");
    let schema = dir.join("schema.json");
    let history = dir.join("history.json");

    cmd_db_migrate(&source, &schema, Some(&history)).expect("migrate schema");

    let written = read_json_value(&schema).expect("read schema");
    assert_eq!(
        written["structs"]["Order"]["fields"]["total"]["type"],
        "int"
    );
    let history = read_json_value(&history).expect("read history");
    assert_eq!(
        history["entries"]
            .as_array()
            .expect("history entries")
            .len(),
        1
    );
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn db_rollback_restores_previous_schema_snapshot() {
    let dir = temp_output_dir("db-rollback");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let original_source = dir.join("original.orv");
    std::fs::write(
        &original_source,
        r#"struct User {
  id: int
  email: string
}"#,
    )
    .expect("write original source");
    let changed_source = dir.join("changed.orv");
    std::fs::write(
        &changed_source,
        r#"struct User {
  id: int
  email: string
  avatar: string?
}"#,
    )
    .expect("write changed source");
    let schema = dir.join("schema.json");

    cmd_db_apply(&original_source, &schema).expect("apply original schema");
    cmd_db_apply(&changed_source, &schema).expect("apply changed schema");
    assert!(
        read_json_value(&schema).expect("read changed schema")["structs"]["User"]["fields"]
            .as_object()
            .expect("fields")
            .contains_key("avatar")
    );

    cmd_db_rollback(&schema).expect("rollback schema");

    let restored = read_json_value(&schema).expect("read restored schema");
    assert!(!restored["structs"]["User"]["fields"]
        .as_object()
        .expect("fields")
        .contains_key("avatar"));
    let plan = db_plan_json(&original_source, Some(&schema)).expect("plan after rollback");
    assert_eq!(plan["actions"].as_array().expect("actions").len(), 0);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn db_verify_accepts_current_schema_snapshot() {
    let dir = temp_output_dir("db-verify-current");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let source = dir.join("app.orv");
    std::fs::write(
        &source,
        r#"struct User {
  id: int
  email: string
}"#,
    )
    .expect("write source");
    let schema = dir.join("schema.json");

    cmd_db_apply(&source, &schema).expect("apply schema");

    cmd_db_verify(&source, &schema).expect("verify current schema");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn db_verify_subcommand_is_accepted() {
    let parsed = Cli::try_parse_from([
        "orv",
        "db",
        "verify",
        "fixtures/e2e/hello.orv",
        "--schema",
        "target/schema.json",
    ]);
    if let Err(err) = parsed {
        panic!("{}", err.render());
    }
}

#[test]
fn db_verify_rejects_schema_drift() {
    let dir = temp_output_dir("db-verify-drift");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let original = dir.join("original.orv");
    std::fs::write(
        &original,
        r#"struct User {
  id: int
  email: string
}"#,
    )
    .expect("write original");
    let changed = dir.join("changed.orv");
    std::fs::write(
        &changed,
        r#"struct User {
  id: int
  email: string
  avatar: string?
}"#,
    )
    .expect("write changed");
    let schema = dir.join("schema.json");

    cmd_db_apply(&original, &schema).expect("apply schema");

    let err = cmd_db_verify(&changed, &schema).expect_err("schema drift");
    assert!(
        err.to_string().contains("db schema drift: 1 action(s)"),
        "{err}"
    );
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn db_squash_writes_compacted_history_actions() {
    let dir = temp_output_dir("db-squash");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let first_source = dir.join("first.orv");
    std::fs::write(
        &first_source,
        r#"struct User {
  id: int
  email: string
}"#,
    )
    .expect("write first");
    let second_source = dir.join("second.orv");
    std::fs::write(
        &second_source,
        r#"struct User {
  id: int
  email: string
  avatar: string?
}"#,
    )
    .expect("write second");
    let schema = dir.join("schema.json");
    let history = dir.join("history.json");
    let squashed = dir.join("squashed.json");

    cmd_db_apply_with_history(&first_source, &schema, Some(&history)).expect("apply first schema");
    cmd_db_apply_with_history(&second_source, &schema, Some(&history))
        .expect("apply second schema");

    cmd_db_squash(&history, &squashed).expect("squash history");

    let value = read_json_value(&squashed).expect("read squashed");
    assert_eq!(value["schema_version"], 1);
    assert_eq!(value["entries"], 2);
    assert!(value["schema_hash"].as_str().expect("schema hash").len() >= 16);
    assert!(value["actions"]
        .as_array()
        .expect("actions")
        .iter()
        .any(|action| action["kind"] == "add_field" && action["field"] == "avatar"));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn db_squash_subcommand_is_accepted() {
    let parsed = Cli::try_parse_from([
        "orv",
        "db",
        "squash",
        "--history",
        "target/history.json",
        "--out",
        "target/squashed.json",
    ]);
    if let Err(err) = parsed {
        panic!("{}", err.render());
    }
}

#[test]
fn db_recover_archive_subcommand_is_accepted() {
    let parsed = Cli::try_parse_from([
        "orv",
        "db",
        "recover",
        "--archive",
        "target/archive.json",
        "--out",
        "target/data.json",
        "--until-record",
        "1",
    ]);
    if let Err(err) = parsed {
        panic!("{}", err.render());
    }
}

#[test]
fn db_restore_wal_subcommand_is_accepted() {
    let parsed = Cli::try_parse_from([
        "orv",
        "db",
        "restore",
        "--wal",
        "target/db.wal.jsonl",
        "--data",
        "target/data.json",
        "--at",
        "2023-11-14T22:13:20Z",
    ]);
    if let Err(err) = parsed {
        panic!("{}", err.render());
    }
}

#[test]
fn db_restore_raw_wal_replays_point_in_time_snapshot() {
    let dir = temp_output_dir("db-restore-raw-wal");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let wal = dir.join("db.wal.jsonl");
    let data = dir.join("data.json");
    std::fs::write(
            &wal,
            concat!(
                "{\"schema_version\":1,\"op\":\"create\",\"table\":\"users\",\"data\":{\"name\":\"Ada\"},\"ts_unix_ms\":1700000000000}\n",
                "{\"schema_version\":1,\"op\":\"create\",\"table\":\"users\",\"data\":{\"name\":\"Grace\"},\"ts_unix_ms\":1700000001000}\n",
            ),
        )
        .expect("write wal");
    std::fs::write(
        &data,
        serde_json::json!({
            "schema_version": 1,
            "tables": {
                "users": {
                    "next_id": 1,
                    "rows": [{ "id": 1, "name": "stale" }]
                }
            }
        })
        .to_string(),
    )
    .expect("write stale data");

    cmd_db_restore_from_inputs(None, Some(&wal), None, Some("2023-11-14T22:13:20Z"), &data)
        .expect("restore raw wal");

    let snapshot = read_json_value(&data).expect("read restored data");
    let rows = snapshot["tables"]["users"]["rows"]
        .as_array()
        .expect("users rows");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["name"], "Ada");
    let rollback = read_json_value(&rollback_schema_path(&data)).expect("read rollback");
    assert_eq!(rollback["tables"]["users"]["rows"][0]["name"], "stale");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn db_recover_archive_rejects_wal_hash_mismatch() {
    let dir = temp_output_dir("db-recover-archive-hash");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let wal = dir.join("db.wal.jsonl");
    let archive = dir.join("archive.json");
    let target_dir = dir.join("archive-target");
    let out = dir.join("data.json");
    let mut db = orv_runtime::db::InMemoryDb::load_wal(&wal).expect("open wal");
    db.create_logged(
        "users",
        vec![(
            "name".to_string(),
            orv_runtime::Value::Str("Ada".to_string()),
        )],
    )
    .expect("create user");
    cmd_db_archive(
        &wal,
        &archive,
        Some(&format!("file://{}", target_dir.display())),
    )
    .expect("archive wal");
    let archived_wal = db_archive_manifest_wal_path(&archive).expect("archive wal path");
    let tampered = std::fs::read_to_string(&archived_wal)
        .expect("read archived wal")
        .replace("Ada", "Eve");
    std::fs::write(&archived_wal, tampered).expect("tamper archived wal");

    let err = cmd_db_recover_from_inputs(None, Some(&archive), &out, None, None, None)
        .expect_err("tampered archive recover");

    assert!(err.to_string().contains("db archive WAL hash mismatch"));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn db_recover_archive_uses_archived_wal_target() {
    let dir = temp_output_dir("db-recover-archive-target");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let wal = dir.join("db.wal.jsonl");
    let archive = dir.join("archive.json");
    let target_dir = dir.join("archive-target");
    let out = dir.join("data.json");
    let mut db = orv_runtime::db::InMemoryDb::load_wal(&wal).expect("open wal");
    db.create_logged(
        "users",
        vec![(
            "name".to_string(),
            orv_runtime::Value::Str("Ada".to_string()),
        )],
    )
    .expect("create first user");
    db.create_logged(
        "users",
        vec![(
            "name".to_string(),
            orv_runtime::Value::Str("Grace".to_string()),
        )],
    )
    .expect("create second user");
    cmd_db_archive(
        &wal,
        &archive,
        Some(&format!("file://{}", target_dir.display())),
    )
    .expect("archive wal");
    std::fs::remove_file(&wal).expect("remove original wal");

    cmd_db_recover_from_inputs(None, Some(&archive), &out, Some(1), None, None)
        .expect("recover from archive");

    let snapshot = read_json_value(&out).expect("snapshot");
    let rows = snapshot["tables"]["users"]["rows"]
        .as_array()
        .expect("users rows");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["name"], "Ada");
    let _ = std::fs::remove_dir_all(dir);
}
