use super::*;

#[test]
fn reveal_origin_exposes_deploy_commerce_adapter_contract() {
    let dir = temp_output_dir("reveal-commerce-adapters-source");
    std::fs::create_dir_all(&dir).expect("create commerce reveal source dir");
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
    .expect("write commerce reveal source");
    let out = temp_output_dir("reveal-commerce-adapters");

    cmd_build_with_profile(&path, &out, BuildProfile::Production).expect("prod build");
    let origin_map: orv_compiler::OriginMap = serde_json::from_str(
        &std::fs::read_to_string(out.join("origin-map.json")).expect("origin map"),
    )
    .expect("origin map json");
    let route = origin_map
        .entries
        .iter()
        .find(|entry| entry.kind == "route" && entry.name == "POST /checkout")
        .expect("checkout route origin");

    let reveal = reveal_origin_json(&out, &route.id).expect("reveal origin");

    let commerce = reveal["production"]["commerce_adapters"]
        .as_array()
        .expect("commerce adapters");
    assert!(commerce.iter().any(|target| {
        target["path"] == "deploy/commerce-adapters.json"
            && target["exists"] == true
            && target["adapters"][0]["kind"] == "payment"
            && target["adapters"][0]["env"] == "PAYMENT_ADAPTER_URL"
            && target["adapters"][0]["endpoint"] == "http://payments.internal/capture"
            && target["adapters"][0]["request"]["kind"] == "payment.capture"
            && target["adapters"][0]["source_origin_id"]
                .as_str()
                .is_some_and(|origin_id| origin_id.starts_with("ori_"))
    }));
    let _ = std::fs::remove_dir_all(dir);
    let _ = std::fs::remove_dir_all(out);
}

#[test]
fn reveal_origin_links_commerce_connects_to_deploy_adapter_contract() {
    let dir = temp_output_dir("reveal-commerce-connect-origin-source");
    std::fs::create_dir_all(&dir).expect("create commerce connect reveal source dir");
    let path = dir.join("app.orv");
    std::fs::write(
        &path,
        r#"@server {
  @listen 8080
  let payments = @payment.connect(@env.PAYMENT_ADAPTER_URL ?? "http://payments.internal/capture")
  let shipping = @shipping.connect(@env.SHIPPING_ADAPTER_URL ?? "http://shipping.internal/book")
  @route POST /checkout {
    let captured = payments.capture({ orderId: "o_1", amount: 42, method: "card" })
    let booked = shipping.book({ orderId: "o_1", carrier: "post", address: "Seoul" })
    @respond 200 { payment: captured.status, shipment: booked.status }
  }
}
"#,
    )
    .expect("write commerce connect reveal source");
    let out = temp_output_dir("reveal-commerce-connect-origin");

    cmd_build_with_profile(&path, &out, BuildProfile::Production).expect("prod build");
    let origin_map: orv_compiler::OriginMap = serde_json::from_str(
        &std::fs::read_to_string(out.join("origin-map.json")).expect("origin map"),
    )
    .expect("origin map json");
    for (origin_name, kind, endpoint) in [
        (
            "@payment.connect",
            "payment",
            "http://payments.internal/capture",
        ),
        (
            "@shipping.connect",
            "shipping",
            "http://shipping.internal/book",
        ),
    ] {
        let origin = origin_map
            .entries
            .iter()
            .find(|entry| entry.kind == "call" && entry.name == origin_name)
            .expect("commerce connect origin");
        let reveal = reveal_origin_json(&out, &origin.id).expect("reveal commerce origin");
        let target = reveal["production"]["commerce_adapters"]
            .as_array()
            .expect("commerce adapters")
            .iter()
            .find(|target| target["path"] == "deploy/commerce-adapters.json")
            .expect("commerce adapter target")
            .clone();
        let matched = target["matched_adapters"]
            .as_array()
            .expect("matched commerce adapters");

        assert_eq!(target["matched"], true);
        assert_eq!(target["selected_origin_id"], origin.id);
        assert_eq!(target["matched_adapter_count"], 1);
        assert_eq!(matched.len(), 1);
        assert_eq!(matched[0]["source_origin_id"], origin.id);
        assert_eq!(matched[0]["matched_origin_id"], origin.id);
        assert_eq!(matched[0]["match"], "direct");
        assert_eq!(matched[0]["kind"], kind);
        assert_eq!(matched[0]["endpoint"], endpoint);
    }
    let _ = std::fs::remove_dir_all(dir);
    let _ = std::fs::remove_dir_all(out);
}

#[test]
fn reveal_origin_exposes_deploy_db_adapter_contract() {
    let dir = temp_output_dir("reveal-db-adapters-source");
    std::fs::create_dir_all(&dir).expect("create db reveal source dir");
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
    .expect("write db reveal source");
    let out = temp_output_dir("reveal-db-adapters");

    cmd_build_with_profile(&path, &out, BuildProfile::Production).expect("prod build");
    let origin_map: orv_compiler::OriginMap = serde_json::from_str(
        &std::fs::read_to_string(out.join("origin-map.json")).expect("origin map"),
    )
    .expect("origin map json");
    let route = origin_map
        .entries
        .iter()
        .find(|entry| entry.kind == "route" && entry.name == "GET /ping")
        .expect("ping route origin");

    let reveal = reveal_origin_json(&out, &route.id).expect("reveal origin");

    let db_adapters = reveal["production"]["db_adapters"]
        .as_array()
        .expect("db adapters");
    assert!(db_adapters.iter().any(|target| {
        target["path"] == "deploy/db-adapters.json"
            && target["exists"] == true
            && target["adapters"][0]["kind"] == "db"
            && target["adapters"][0]["provider"] == "postgres"
            && target["adapters"][0]["env"] == "SHOP_DATABASE_URL"
            && target["adapters"][0]["endpoint"] == "postgres://db.internal/shop"
            && target["adapters"][0]["adapter_status"] == "unsupported_runtime"
            && target["adapters"][0]["source_origin_id"]
                .as_str()
                .is_some_and(|origin_id| origin_id.starts_with("ori_"))
    }));
    let _ = std::fs::remove_dir_all(dir);
    let _ = std::fs::remove_dir_all(out);
}

#[test]
fn reveal_origin_links_db_connect_to_deploy_adapter_contract() {
    let dir = temp_output_dir("reveal-db-connect-origin-source");
    std::fs::create_dir_all(&dir).expect("create db connect reveal source dir");
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
    .expect("write db connect reveal source");
    let out = temp_output_dir("reveal-db-connect-origin");

    cmd_build_with_profile(&path, &out, BuildProfile::Production).expect("prod build");
    let origin_map: orv_compiler::OriginMap = serde_json::from_str(
        &std::fs::read_to_string(out.join("origin-map.json")).expect("origin map"),
    )
    .expect("origin map json");
    let db_connect = origin_map
        .entries
        .iter()
        .find(|entry| entry.kind == "call" && entry.name == "@db.connect")
        .expect("db connect origin");

    let reveal = reveal_origin_json(&out, &db_connect.id).expect("reveal db connect origin");
    let db_adapters = reveal["production"]["db_adapters"]
        .as_array()
        .expect("db adapters");
    let target = db_adapters
        .iter()
        .find(|target| target["path"] == "deploy/db-adapters.json")
        .expect("db adapter target");
    let matched = target["matched_adapters"]
        .as_array()
        .expect("matched db adapters");

    assert_eq!(target["matched"], true);
    assert_eq!(target["selected_origin_id"], db_connect.id);
    assert_eq!(target["matched_adapter_count"], 1);
    assert_eq!(matched.len(), 1);
    assert_eq!(matched[0]["source_origin_id"], db_connect.id);
    assert_eq!(matched[0]["matched_origin_id"], db_connect.id);
    assert_eq!(matched[0]["match"], "direct");
    assert_eq!(matched[0]["provider"], "postgres");
    assert_eq!(matched[0]["bridge"]["contract"], "http-json-v1");
    let _ = std::fs::remove_dir_all(dir);
    let _ = std::fs::remove_dir_all(out);
}

#[test]
fn reveal_origin_exposes_deploy_preflight_contract() {
    let dir = temp_output_dir("reveal-preflight-source");
    std::fs::create_dir_all(&dir).expect("create preflight reveal source dir");
    let path = dir.join("app.orv");
    std::fs::write(
        &path,
        r#"@server {
  @listen 8080
  let shopdb = @db.connect(@env.SHOP_DATABASE_URL)
  @route GET /ping { @respond 200 { ok: true } }
}
"#,
    )
    .expect("write preflight reveal source");
    let out = temp_output_dir("reveal-preflight");

    cmd_build_with_profile(&path, &out, BuildProfile::Production).expect("prod build");
    let origin_map: orv_compiler::OriginMap = serde_json::from_str(
        &std::fs::read_to_string(out.join("origin-map.json")).expect("origin map"),
    )
    .expect("origin map json");
    let route = origin_map
        .entries
        .iter()
        .find(|entry| entry.kind == "route" && entry.name == "GET /ping")
        .expect("ping route origin");

    let reveal = reveal_origin_json(&out, &route.id).expect("reveal origin");

    let preflight = reveal["production"]["preflight"]
        .as_array()
        .expect("preflight targets");
    assert!(preflight.iter().any(|target| {
        target["path"] == "deploy/preflight.json"
            && target["exists"] == true
            && target["commands"]["verify_build"] == "orv verify-build ."
            && target["commands"]["env_check"] == "orv deploy-env-check ."
            && target["commands"]["benchmark_prepare"] == "orv benchmark-prepare . --participants 2"
            && target["commands"]["benchmark_report"] == "orv benchmark-report ."
            && target["commands"]["benchmark_report_require_pass"]
                == "orv benchmark-report . --require-pass"
            && target["artifacts"]["smoke_test"] == "deploy/smoke-test.sh"
            && target["artifacts"]["smoke_output"] == "deploy/smoke-output.txt"
            && target["artifacts"]["benchmark_evidence"] == "deploy/benchmark-evidence.json"
            && target["smoke_output_contract"]["output"] == "deploy/smoke-output.txt"
            && target["smoke_output_contract"]["required_markers"]
                == serde_json::json!(deploy_benchmark::SMOKE_REQUIRED_MARKERS)
            && target["benchmark"]["kind"] == "orv.benchmark.shop_5h"
            && target["benchmark"]["max_elapsed_minutes"] == 300
            && target["benchmark_evidence"]["exists"] == true
            && target["benchmark_evidence"]["path"] == "deploy/benchmark-evidence.json"
            && target["benchmark_evidence"]["recording_status"] == "not_recorded"
            && target["benchmark_evidence"]["report_status"] == "incomplete"
            && target["benchmark_evidence"]["task_count"] == 10
            && target["benchmark_evidence"]["recorded_task_count"] == 0
            && target["benchmark_evidence"]["missing_task_count"] == 10
            && target["benchmark_evidence"]["missing_data_count"] == 16
            && target["benchmark_evidence"]["failed_data_count"] == 0
            && target["benchmark_evidence"]["failed_data"]
                .as_array()
                .expect("failed data")
                .is_empty()
            && target["benchmark_evidence"]["smoke_test_required_markers"]
                == serde_json::json!(deploy_benchmark::SMOKE_REQUIRED_MARKERS)
            && target["benchmark_evidence"]["smoke_test_summary"]["present"] == false
            && target["benchmark_evidence"]["smoke_test_summary"]["required_markers"]
                == serde_json::json!(deploy_benchmark::SMOKE_REQUIRED_MARKERS)
            && target["benchmark_evidence"]["smoke_test_output_source"].is_null()
            && target["benchmark_evidence"]["missing_data"]
                .as_array()
                .expect("missing data")
                .iter()
                .any(|item| item == "smoke_test_output")
            && target["benchmark_evidence"]["missing_data"]
                .as_array()
                .expect("missing data")
                .iter()
                .any(|item| item == "recording_status.recorded")
            && target["benchmark_evidence"]["missing_data"]
                .as_array()
                .expect("missing data")
                .iter()
                .any(|item| item == "ai_assistance_used")
            && target["benchmark_evidence"]["missing_data"]
                .as_array()
                .expect("missing data")
                .iter()
                .any(|item| item == "generated_artifact_edits")
            && target["benchmark_evidence"]["missing_data"]
                .as_array()
                .expect("missing data")
                .iter()
                .any(|item| item == "manual_undocumented_security_steps")
            && target["benchmark_evidence"]["missing_data"]
                .as_array()
                .expect("missing data")
                .iter()
                .any(|item| item == "human_evidence_review.raw_notes_reviewed")
            && target["benchmark_evidence"]["missing_data"]
                .as_array()
                .expect("missing data")
                .iter()
                .any(|item| item == "participant_runs.minimum")
            && target["benchmark_evidence"]["participant_raw_notes_artifacts"][0]["checked"]
                == false
            && target["benchmark_evidence"]["participant_raw_notes_artifacts"][0]["retained"]
                .is_null()
            && target["routes"][0]["method"] == "GET"
            && target["routes"][0]["path"] == "/ping"
            && target["required_env"][0]["kind"] == "db"
            && target["required_env"][0]["env"] == "SHOP_DATABASE_URL"
            && target["required_env"][0]["required"] == true
    }));
    let _ = std::fs::remove_dir_all(dir);
    let _ = std::fs::remove_dir_all(out);
}

#[test]
fn reveal_origin_links_client_signal_to_client_bundle_targets() {
    let out = temp_output_dir("reveal-client-origin");
    std::fs::create_dir_all(&out).expect("create temp root");
    let entry = out.join("page.orv");
    std::fs::write(
        &entry,
        "let sig count: int = 0\n@out @html { @body { @p count } }",
    )
    .expect("write entry");
    let build_out = out.join("dist");

    cmd_build(&entry, &build_out).expect("build artifacts");
    let origin_map: orv_compiler::OriginMap = serde_json::from_str(
        &std::fs::read_to_string(build_out.join("origin-map.json")).expect("origin map"),
    )
    .expect("origin map json");
    let signal = origin_map
        .entries
        .iter()
        .find(|entry| entry.kind == "signal" && entry.name == "count")
        .expect("signal origin");

    let reveal = reveal_origin_json(&build_out, &signal.id).expect("reveal origin");

    assert_eq!(reveal["origin"]["kind"], "signal");
    assert!(reveal["source"]["snippet"]
        .as_str()
        .is_some_and(|snippet| snippet.contains("let sig count")));
    let client = reveal["production"]["client"]
        .as_array()
        .expect("client targets");
    assert!(client.iter().any(|target| {
        target["kind"] == "client_manifest"
            && target["path"] == CLIENT_MANIFEST_PATH
            && target["source_bundle"] == SOURCE_BUNDLE_PATH
            && target["source_bundle_hash"]
                .as_str()
                .is_some_and(|hash| !hash.is_empty())
            && target["wasm_hash"]
                .as_str()
                .is_some_and(|hash| !hash.is_empty())
            && target["capabilities"]["runtime"] == "client_wasm"
            && target["capabilities"]["bindings"]["signal_text"] == 1
            && target["capabilities"]["surfaces"]
                .as_array()
                .expect("manifest capability surfaces")
                .iter()
                .any(|surface| surface == "signal_text")
            && target["blockers"]
                .as_array()
                .expect("manifest blockers")
                .iter()
                .any(|blocker| {
                    blocker["id"] == "dynamic-client-codegen"
                        && blocker["artifact"] == CLIENT_JS_PATH
                })
    }));
    assert!(client.iter().any(|target| {
        target["kind"] == "client_reactive_plan"
            && target["path"] == CLIENT_REACTIVE_PLAN_PATH
            && target["signal_count"] == 1
            && target["source_bundle_hash"]
                .as_str()
                .is_some_and(|hash| !hash.is_empty())
            && target["blockers"]
                .as_array()
                .expect("reactive blockers")
                .iter()
                .any(|blocker| {
                    blocker["id"] == "reactive-dom-diff"
                        && blocker["artifact"] == CLIENT_REACTIVE_PLAN_PATH
                })
    }));
    assert!(client
        .iter()
        .any(|target| target["kind"] == "client_page" && target["path"] == "pages/index.html"));
    assert!(client
        .iter()
        .any(|target| target["kind"] == "client_js" && target["path"] == "client/app.js"));
    assert!(client
        .iter()
        .any(|target| target["kind"] == "client_wasm" && target["path"] == "client/app.wasm"));
    assert!(reveal["production"]["routes"]
        .as_array()
        .expect("routes")
        .is_empty());
    assert_eq!(reveal["production"]["summary"]["client_target_count"], 5);
    assert_eq!(reveal["production"]["summary"]["client_manifest_count"], 1);
    assert!(
        reveal["production"]["summary"]["client_capability_surface_count"]
            .as_u64()
            .is_some_and(|count| count >= 2)
    );
    let lsp_reveal = lsp_reveal_json(&build_out, &signal.id).expect("lsp reveal");
    assert_eq!(
        lsp_reveal["production"]["summary"]["client_target_count"],
        5
    );
    assert_eq!(
        lsp_reveal["production"]["summary"]["client_manifest_count"],
        1
    );
    let editor_reveal = editor_reveal_json(&build_out, &signal.id).expect("editor reveal");
    assert_eq!(
        editor_reveal["production"]["summary"]["client_target_count"],
        5
    );
    assert_eq!(
        editor_reveal["production"]["summary"]["client_manifest_count"],
        1
    );
    let _ = std::fs::remove_dir_all(&out);
}
