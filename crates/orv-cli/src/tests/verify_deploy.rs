use super::*;

#[test]
fn verify_build_rejects_deploy_commerce_adapter_mismatch() {
    let dir = temp_output_dir("deploy-commerce-adapters-source");
    std::fs::create_dir_all(&dir).expect("create commerce adapter source dir");
    let path = dir.join("app.orv");
    std::fs::write(
        &path,
        r#"@server {
  @listen 8080
  let payments = @payment.connect(@env.PAYMENT_ADAPTER_URL ?? "http://payments.internal/capture")
  @route POST /checkout {
    let captured = payments.capture({ orderId: "o_1", amount: 42, method: "card" })
    @respond 200 { payment: captured.status }
  }
}
"#,
    )
    .expect("write commerce adapter source");
    let out = temp_output_dir("deploy-commerce-adapters-mismatch");

    cmd_build_with_profile(&path, &out, BuildProfile::Production).expect("prod build");
    let adapters_path = out.join("deploy").join("commerce-adapters.json");
    let mut adapters = read_json_value(&adapters_path).expect("commerce adapters");
    adapters["adapters"][0]["endpoint"] = serde_json::json!("http://wrong.example/capture");
    write_json(&adapters_path, &adapters).expect("write corrupt commerce adapters");

    let err = cmd_verify_build(&out).expect_err("commerce adapter mismatch");

    assert!(err
        .to_string()
        .contains("deploy commerce adapters do not match runtime artifact persistence"));
    let _ = std::fs::remove_dir_all(dir);
    let _ = std::fs::remove_dir_all(out);
}

#[test]
fn verify_build_rejects_deploy_commerce_adapters_extra_root_key() {
    let dir = temp_output_dir("deploy-commerce-adapters-extra-root-source");
    std::fs::create_dir_all(&dir).expect("create commerce adapter source dir");
    let path = dir.join("app.orv");
    std::fs::write(
        &path,
        r#"@server {
  @listen 8080
  let payments = @payment.connect(@env.PAYMENT_ADAPTER_URL ?? "http://payments.internal/capture")
  @route POST /checkout {
    let captured = payments.capture({ orderId: "o_1", amount: 42, method: "card" })
    @respond 200 { payment: captured.status }
  }
}
"#,
    )
    .expect("write commerce adapter source");
    let out = temp_output_dir("deploy-commerce-adapters-extra-root");

    cmd_build_with_profile(&path, &out, BuildProfile::Production).expect("prod build");
    let adapters_path = out.join("deploy").join("commerce-adapters.json");
    let mut adapters = read_json_value(&adapters_path).expect("commerce adapters");
    adapters["unexpected"] = serde_json::json!(true);
    write_json(&adapters_path, &adapters).expect("write corrupt commerce adapters");

    let err = cmd_verify_build(&out).expect_err("commerce adapter extra root key");

    assert!(err
        .to_string()
        .contains("deploy commerce adapters keys must match contract"));
    let _ = std::fs::remove_dir_all(dir);
    let _ = std::fs::remove_dir_all(out);
}

#[test]
fn verify_build_rejects_deploy_commerce_adapters_extra_request_key() {
    let dir = temp_output_dir("deploy-commerce-adapters-extra-request-source");
    std::fs::create_dir_all(&dir).expect("create commerce adapter source dir");
    let path = dir.join("app.orv");
    std::fs::write(
        &path,
        r#"@server {
  @listen 8080
  let payments = @payment.connect(@env.PAYMENT_ADAPTER_URL ?? "http://payments.internal/capture")
  @route POST /checkout {
    let captured = payments.capture({ orderId: "o_1", amount: 42, method: "card" })
    @respond 200 { payment: captured.status }
  }
}
"#,
    )
    .expect("write commerce adapter source");
    let out = temp_output_dir("deploy-commerce-adapters-extra-request");

    cmd_build_with_profile(&path, &out, BuildProfile::Production).expect("prod build");
    let adapters_path = out.join("deploy").join("commerce-adapters.json");
    let mut adapters = read_json_value(&adapters_path).expect("commerce adapters");
    adapters["adapters"][0]["request"]["unexpected"] = serde_json::json!("drift");
    write_json(&adapters_path, &adapters).expect("write corrupt commerce adapters");

    let err = cmd_verify_build(&out).expect_err("commerce adapter extra request key");

    assert!(err
        .to_string()
        .contains("deploy commerce adapter adapters[0].request keys must match contract"));
    let _ = std::fs::remove_dir_all(dir);
    let _ = std::fs::remove_dir_all(out);
}

#[test]
fn verify_build_rejects_deploy_commerce_adapter_origin_drift_from_origin_map() {
    let dir = temp_output_dir("deploy-commerce-adapter-origin-source");
    std::fs::create_dir_all(&dir).expect("create commerce adapter origin source dir");
    let path = dir.join("app.orv");
    std::fs::write(
        &path,
        r#"@server {
  @listen 8080
  let payments = @payment.connect(@env.PAYMENT_ADAPTER_URL ?? "http://payments.internal/capture")
  @route POST /checkout {
    let captured = payments.capture({ orderId: "o_1", amount: 42, method: "card" })
    @respond 200 { payment: captured.status }
  }
}
"#,
    )
    .expect("write commerce adapter origin source");
    let out = temp_output_dir("deploy-commerce-adapter-origin-mismatch");

    cmd_build_with_profile(&path, &out, BuildProfile::Production).expect("prod build");
    let adapters_path = out.join("deploy").join("commerce-adapters.json");
    let adapters = read_json_value(&adapters_path).expect("commerce adapters");
    let origin_id = adapters["adapters"][0]["source_origin_id"]
        .as_str()
        .expect("commerce source origin")
        .to_string();
    corrupt_origin_entry_kind_and_graph(&out, &origin_id, "domain", "payment");

    let err = cmd_verify_build(&out).expect_err("commerce adapter origin mismatch");
    // OriginMap v2 identity guards reject kind/name drift before adapter-level
    // checks run; the adapter arms are covered by
    // `deploy_adapter_source_origin_rejects_missing_and_non_call_entries`.
    assert!(err.to_string().contains(&format!(
        "origin-map.json entry `{origin_id}` fingerprint does not match span"
    )));
    let _ = std::fs::remove_dir_all(dir);
    let _ = std::fs::remove_dir_all(out);
}

#[test]
fn verify_build_rejects_deploy_db_adapter_mismatch() {
    let dir = temp_output_dir("deploy-db-adapters-source");
    std::fs::create_dir_all(&dir).expect("create db adapter source dir");
    let path = dir.join("app.orv");
    std::fs::write(
        &path,
        r#"@server {
  @listen 8080
  let shopdb = @db.connect(@env.SHOP_DATABASE_URL ?? "postgres://db.internal/shop")
  @route GET /ping { @respond 200 { ok: true } }
}
"#,
    )
    .expect("write db adapter source");
    let out = temp_output_dir("deploy-db-adapters-mismatch");

    cmd_build_with_profile(&path, &out, BuildProfile::Production).expect("prod build");
    let adapters_path = out.join("deploy").join("db-adapters.json");
    let mut adapters = read_json_value(&adapters_path).expect("db adapters");
    adapters["adapters"][0]["endpoint"] = serde_json::json!("postgres://wrong.example/shop");
    write_json(&adapters_path, &adapters).expect("write corrupt db adapters");

    let err = cmd_verify_build(&out).expect_err("db adapter mismatch");

    assert!(err
        .to_string()
        .contains("deploy DB adapters do not match runtime artifact persistence"));
    let _ = std::fs::remove_dir_all(dir);
    let _ = std::fs::remove_dir_all(out);
}

#[test]
fn verify_build_rejects_deploy_db_adapters_extra_root_key() {
    let dir = temp_output_dir("deploy-db-adapters-extra-root-source");
    std::fs::create_dir_all(&dir).expect("create db adapter source dir");
    let path = dir.join("app.orv");
    std::fs::write(
        &path,
        r#"@server {
  @listen 8080
  let shopdb = @db.connect(@env.SHOP_DATABASE_URL ?? "postgres://db.internal/shop")
  @route GET /ping { @respond 200 { ok: true } }
}
"#,
    )
    .expect("write db adapter source");
    let out = temp_output_dir("deploy-db-adapters-extra-root");

    cmd_build_with_profile(&path, &out, BuildProfile::Production).expect("prod build");
    let adapters_path = out.join("deploy").join("db-adapters.json");
    let mut adapters = read_json_value(&adapters_path).expect("db adapters");
    adapters["unexpected"] = serde_json::json!(true);
    write_json(&adapters_path, &adapters).expect("write corrupt db adapters");

    let err = cmd_verify_build(&out).expect_err("db adapter extra root key");

    assert!(err
        .to_string()
        .contains("deploy DB adapters keys must match contract"));
    let _ = std::fs::remove_dir_all(dir);
    let _ = std::fs::remove_dir_all(out);
}

#[test]
fn verify_build_rejects_deploy_db_adapters_extra_bridge_key() {
    let dir = temp_output_dir("deploy-db-adapters-extra-bridge-source");
    std::fs::create_dir_all(&dir).expect("create db adapter source dir");
    let path = dir.join("app.orv");
    std::fs::write(
        &path,
        r#"@server {
  @listen 8080
  let shopdb = @db.connect(@env.SHOP_DATABASE_URL ?? "postgres://db.internal/shop")
  @route GET /ping { @respond 200 { ok: true } }
}
"#,
    )
    .expect("write db adapter source");
    let out = temp_output_dir("deploy-db-adapters-extra-bridge");

    cmd_build_with_profile(&path, &out, BuildProfile::Production).expect("prod build");
    let adapters_path = out.join("deploy").join("db-adapters.json");
    let mut adapters = read_json_value(&adapters_path).expect("db adapters");
    adapters["adapters"][0]["bridge"]["unexpected"] = serde_json::json!("drift");
    write_json(&adapters_path, &adapters).expect("write corrupt db adapters");

    let err = cmd_verify_build(&out).expect_err("db adapter extra bridge key");

    assert!(err
        .to_string()
        .contains("deploy DB adapter adapters[0].bridge keys must match contract"));
    let _ = std::fs::remove_dir_all(dir);
    let _ = std::fs::remove_dir_all(out);
}

#[test]
fn verify_build_rejects_deploy_db_adapter_origin_drift_from_origin_map() {
    let dir = temp_output_dir("deploy-db-adapter-origin-source");
    std::fs::create_dir_all(&dir).expect("create db adapter origin source dir");
    let path = dir.join("app.orv");
    std::fs::write(
        &path,
        r#"@server {
  @listen 8080
  let shopdb = @db.connect(@env.SHOP_DATABASE_URL ?? "postgres://db.internal/shop")
  @route GET /ping { @respond 200 { ok: true } }
}
"#,
    )
    .expect("write db adapter origin source");
    let out = temp_output_dir("deploy-db-adapter-origin-mismatch");

    cmd_build_with_profile(&path, &out, BuildProfile::Production).expect("prod build");
    let adapters_path = out.join("deploy").join("db-adapters.json");
    let adapters = read_json_value(&adapters_path).expect("db adapters");
    let origin_id = adapters["adapters"][0]["source_origin_id"]
        .as_str()
        .expect("db source origin")
        .to_string();
    corrupt_origin_entry_kind_and_graph(&out, &origin_id, "domain", "db");

    let err = cmd_verify_build(&out).expect_err("db adapter origin mismatch");
    // OriginMap v2 identity guards reject kind/name drift before adapter-level
    // checks run; the adapter arms are covered by
    // `deploy_adapter_source_origin_rejects_missing_and_non_call_entries`.
    assert!(err.to_string().contains(&format!(
        "origin-map.json entry `{origin_id}` fingerprint does not match span"
    )));
    let _ = std::fs::remove_dir_all(dir);
    let _ = std::fs::remove_dir_all(out);
}

#[test]
fn verify_build_rejects_deploy_routes_mismatch() {
    let (src_dir, path) = prod_server_source("deploy-routes-source");
    let out = temp_output_dir("deploy-routes-mismatch");

    cmd_build_with_profile(&path, &out, BuildProfile::Production).expect("prod build");
    let routes_path = out.join("deploy").join("routes.json");
    let mut routes = read_json_value(&routes_path).expect("routes");
    routes["routes"][0]["path"] = serde_json::json!("/wrong");
    write_json(&routes_path, &routes).expect("write corrupt routes");

    let err = cmd_verify_build(&out).expect_err("routes mismatch");

    assert!(err
        .to_string()
        .contains("deploy routes do not match runtime artifact"));
    let _ = std::fs::remove_dir_all(src_dir);
    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn verify_build_rejects_deploy_container_env_ports_mismatch() {
    let (src_dir, path) = env_prod_server_source("deploy-container-env-ports-source");
    let out = temp_output_dir("deploy-container-env-ports-mismatch");

    cmd_build_with_profile(&path, &out, BuildProfile::Production).expect("prod build");
    let container_path = out.join("deploy").join("container.json");
    let mut container = read_json_value(&container_path).expect("container");
    container["ports"][0]["env"] = serde_json::json!("HTTP_PORT");
    write_json(&container_path, &container).expect("write corrupt container");

    let err = cmd_verify_build(&out).expect_err("container ports mismatch");

    assert!(err
        .to_string()
        .contains("deploy container ports do not match runtime artifact"));
    let _ = std::fs::remove_dir_all(src_dir);
    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn verify_deploy_artifact_cases() {
    verify_artifact_cases(
        "verify_deploy_artifact_cases",
        prod_server_source,
        BuildProfile::Production,
        &[
            json_case(
                "deploy_manifest_extra_root_key",
                "deploy/manifest.json",
                "deploy manifest keys must match contract",
                |deploy| {
                    deploy["unexpected"] = serde_json::json!(true);
                },
            ),
            json_case(
                "deploy_manifest_extra_server_key",
                "deploy/manifest.json",
                "deploy server keys must match contract",
                |deploy| {
                    deploy["server"]["unexpected"] = serde_json::json!("drift");
                },
            ),
            json_case(
                "deploy_manifest_server_protocol_mismatch",
                "deploy/manifest.json",
                "deploy server protocol must be http1",
                |deploy| {
                    deploy["server"]["protocol"] = serde_json::json!("http/1.1");
                },
            ),
            json_case(
                "deploy_routes_extra_root_key",
                "deploy/routes.json",
                "deploy routes keys must match contract",
                |routes| {
                    routes["unexpected"] = serde_json::json!(true);
                },
            ),
            json_case(
                "deploy_container_mismatch",
                "deploy/container.json",
                "deploy container artifact must be server/app.orv-runtime.json",
                |container| {
                    container["artifact"] = serde_json::json!("server/wrong.orv-runtime.json");
                },
            ),
            json_case(
                "deploy_container_extra_root_key",
                "deploy/container.json",
                "deploy container keys must match contract",
                |container| {
                    container["unexpected"] = serde_json::json!(true);
                },
            ),
            json_case(
                "deploy_container_command_drift",
                "deploy/container.json",
                "deploy container command must be [\"./deploy/server.sh\"]",
                |container| {
                    container["command"] = serde_json::json!(["./deploy/server.sh", "--debug"]);
                },
            ),
            artifact_case("deploy_dockerfile_extra_drift", |out| {
                let dockerfile_path = out.join("deploy").join("Dockerfile");
                let mut dockerfile = std::fs::read_to_string(&dockerfile_path).expect("Dockerfile");
                dockerfile.push_str("RUN echo stale-deploy-drift\n");
                write_text(&dockerfile_path, &dockerfile).expect("write corrupt Dockerfile");

                let err = cmd_verify_build(out).expect_err("deploy Dockerfile drift must fail");

                assert!(err
                    .to_string()
                    .contains("deploy Dockerfile must match generated artifact"));
            }),
            artifact_case("deploy_server_entrypoint_extra_drift", |out| {
                let entrypoint_path = out.join("deploy").join("server.sh");
                let mut script =
                    std::fs::read_to_string(&entrypoint_path).expect("server entrypoint");
                script.push_str("# stale deploy entrypoint drift\n");
                write_text(&entrypoint_path, &script).expect("write corrupt entrypoint");

                let err =
                    cmd_verify_build(out).expect_err("deploy server entrypoint drift must fail");

                assert!(err
                    .to_string()
                    .contains("deploy server entrypoint must match generated artifact"));
            }),
            json_case(
                "deploy_container_runtime_image_mismatch",
                "deploy/container.json",
                "deploy container runtime_image must be",
                |container| {
                    container["runtime_image"] = serde_json::json!("example.invalid/orv:wrong");
                },
            ),
            artifact_case("deploy_compose_port_mismatch", |out| {
                let compose_path = out.join("deploy").join("compose.yaml");
                let mut compose = std::fs::read_to_string(&compose_path).expect("compose");
                compose = compose.replace(r#""8080:8080""#, r#""9090:9090""#);
                write_text(&compose_path, &compose).expect("write corrupt compose");

                let err = cmd_verify_build(out).expect_err("compose port mismatch");

                assert!(err.to_string().contains("deploy compose must publish 8080"));
            }),
            artifact_case("deploy_compose_extra_drift", |out| {
                let compose_path = out.join("deploy").join("compose.yaml");
                let mut compose = std::fs::read_to_string(&compose_path).expect("compose");
                compose.push_str("# unexpected deploy drift\n");
                write_text(&compose_path, &compose).expect("write corrupt compose");

                let err = cmd_verify_build(out).expect_err("compose extra drift");

                assert!(err
                    .to_string()
                    .contains("deploy compose must match generated artifact"));
            }),
            artifact_case("deploy_runbook_route_mismatch", |out| {
                let runbook_path = out.join("deploy").join("README.md");
                let mut runbook = std::fs::read_to_string(&runbook_path).expect("runbook");
                runbook = runbook.replace("- GET /ping", "- GET /wrong");
                write_text(&runbook_path, &runbook).expect("write corrupt runbook");

                let err = cmd_verify_build(out).expect_err("runbook route mismatch");

                assert!(err
                    .to_string()
                    .contains("deploy runbook must list route GET /ping"));
            }),
            artifact_case("deploy_runbook_extra_drift", |out| {
                let runbook_path = out.join("deploy").join("README.md");
                let mut runbook = std::fs::read_to_string(&runbook_path).expect("runbook");
                runbook.push_str(
                    "\n## Unexpected Drift\n\nThis stale note must not survive verify-build.\n",
                );
                write_text(&runbook_path, &runbook).expect("write corrupt runbook");

                let err = cmd_verify_build(out).expect_err("runbook extra drift");

                assert!(err
                    .to_string()
                    .contains("deploy runbook must match generated artifact"));
            }),
            json_case(
                "deploy_container_listen_mismatch",
                "deploy/container.json",
                "deploy container listen does not match runtime artifact",
                |container| {
                    container["listen"] = serde_json::json!({
                        "origin_id": "ori_wrong",
                        "name": "port 9090",
                        "port": 9090,
                    });
                },
            ),
        ],
    );
}

#[test]
fn verify_deploy_static_artifact_cases() {
    verify_artifact_cases(
        "verify_deploy_static_artifact_cases",
        |name| source_fixture(name, r#"@out @html { @body { @h1 "Home" } }"#),
        BuildProfile::Production,
        &[
            json_case(
                "deploy_static_target_drift",
                "deploy/manifest.json",
                "deploy static path does not match bundle static_page target",
                |deploy| {
                    deploy["static"]["path"] = serde_json::json!(SOURCE_BUNDLE_PATH);
                },
            ),
            json_case(
                "deploy_static_extra_root_key",
                "deploy/manifest.json",
                "deploy static keys must match contract",
                |deploy| {
                    deploy["static"]["unexpected"] = serde_json::json!(true);
                },
            ),
            json_case(
                "missing_deploy_static_target_for_static_bundle",
                "deploy/manifest.json",
                "deploy static target missing for bundle static_page",
                |deploy| {
                    deploy["static"] = serde_json::Value::Null;
                },
            ),
        ],
    );
}
