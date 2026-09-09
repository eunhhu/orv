use super::*;

#[test]
fn init_shop_template_writes_deploy_guide() {
    let dir = temp_output_dir("init-shop-guide");

    cmd_init(&dir, Some("starter-shop"), InitTemplate::Shop).expect("init shop project");

    let guide = std::fs::read_to_string(dir.join("README.md")).expect("shop README");
    assert!(guide.contains("starter-shop"));
    assert!(guide.contains("orv check ."));
    assert!(guide.contains("orv build . --prod --out dist"));
    assert!(guide.contains("orv verify-build dist"));
    assert!(guide.contains("orv deploy-env-check dist"));
    assert!(guide.contains("orv benchmark-report dist"));
    assert!(guide.contains("orv benchmark-report dist --require-pass"));
    assert!(guide.contains("keeps the local reference server in the foreground"));
    assert!(guide.contains("sh dist/deploy/smoke-test.sh"));
    assert!(guide.contains("deploy/README.md"));
    assert!(guide.contains("deploy/compose.yaml"));
    assert!(guide.contains("deploy/env.example"));
    assert!(guide.contains("deploy/db-adapters.json"));
    assert!(guide.contains("deploy/commerce-adapters.json"));
    assert!(guide.contains("deploy/preflight.json"));
    assert!(guide.contains("deploy/benchmark-evidence.json"));
    assert!(guide.contains("deploy/smoke-output.txt"));
    assert!(guide.contains("- `pass_marker`"));
    assert!(guide.contains("- `dap_source_bundle`"));
    assert!(guide.contains("- `trace_stream_requested`"));
    assert!(guide.contains("5-hour shop benchmark"));
    assert!(guide.contains("deploy/smoke-test.sh"));
    assert!(guide.contains("server/native-server.json"));
    assert!(guide.contains("server/native/Cargo.toml"));
    assert!(guide.contains("server/native/main.rs"));
    assert!(guide.contains("server/native/routes.rs"));
    assert!(guide.contains("server/native/router.rs"));
    assert!(guide.contains("server/native/handlers.rs"));
    assert!(guide.contains("cd dist"));
    assert!(guide.contains("PORT=8080 docker compose -f deploy/compose.yaml up --build -d"));
    assert!(guide.contains("./deploy/smoke-test.sh"));
    assert!(guide.contains("cargo build --manifest-path dist/server/native/Cargo.toml --release"));
    assert!(
        guide.contains("ORV_BUILD_DIR=dist ./dist/server/native/target/release/orv-native-server")
    );
    assert!(guide.contains("The generated launcher path can infer `dist`"));
    assert!(guide.contains("Persistent database: `data/shop.sqlite`"));
    assert!(guide.contains("SHOP_DATABASE_URL"));
    assert!(guide.contains("Commerce records: `data/payments.jsonl`, `data/shipments.jsonl`"));
    assert!(guide.contains("PAYMENT_ADAPTER_URL"));
    assert!(guide.contains("SHIPPING_ADAPTER_URL"));
    assert!(guide.contains("provider-mode adapters"));
    assert!(guide.contains("stripe://"));
    assert!(guide.contains("carrier://"));
    assert!(guide.contains("STRIPE_SECRET_KEY"));
    assert!(guide.contains("STRIPE_WEBHOOK_SECRET"));
    assert!(guide.contains("STRIPE_WEBHOOK_SECRET_PREVIOUS"));
    assert!(guide.contains("CARRIER_API_KEY"));
    assert!(guide.contains("CARRIER_WEBHOOK_SECRET"));
    assert!(guide.contains("Compose mounts `data/` into `/app/data`"));
    assert!(guide.contains("Back up `data/shop.sqlite` and commerce record logs"));
    assert!(guide.contains("Browser home"));
    assert!(guide.contains("http://localhost:8080/"));
    assert!(guide.contains("Theme tokens"));
    assert!(guide.contains("@design"));
    assert!(guide.contains("@colors"));
    assert!(guide.contains("@spacing"));
    assert!(guide.contains("@typography"));
    assert!(guide.contains("Product field edits"));
    assert!(guide.contains("ProductInput.badge"));
    assert!(guide.contains("/admin/catalog"));
    assert!(guide.contains("Admin dashboard: http://localhost:8080/admin"));
    assert!(guide.contains("@Auth required role=\"admin\""));
    assert!(guide.contains("admin@example.test"));
    assert!(guide.contains("Argon2"));
    assert!(guide.contains("hash.password"));
    assert!(guide.contains("hash.verify"));
    assert!(guide.contains("never persists plaintext passwords"));
    assert!(guide.contains("orv_session"));
    assert!(guide.contains("orv_session_role"));
    assert!(guide.contains("HttpOnly"));
    assert!(guide.contains("SameSite=Lax"));
    assert!(guide.contains("Secure"));
    assert!(guide.contains("@session required"));
    assert!(guide.contains("@session.id"));
    assert!(guide.contains("@csrf"));
    assert!(guide.contains("orv_csrf"));
    assert!(guide.contains("GET /"));
    assert!(guide.contains("GET /catalog"));
    assert!(guide.contains("GET /cart"));
    assert!(guide.contains("GET /account/sessions"));
    assert!(guide.contains("GET /admin"));
    assert!(guide.contains("GET /admin/catalog"));
    assert!(guide.contains("GET /admin/summary"));
    assert!(guide.contains("GET /admin/orders"));
    assert!(guide.contains("GET /admin/payments"));
    assert!(guide.contains("GET /admin/shipments"));
    assert!(guide.contains("GET /admin/webhooks"));
    assert!(guide.contains("GET /admin/audit"));
    assert!(guide.contains("POST /members"));
    assert!(guide.contains("POST /members/login"));
    assert!(guide.contains("POST /checkout"));
    assert!(guide.contains("POST /cart/items"));
    assert!(guide.contains("POST /payments"));
    assert!(guide.contains("POST /webhooks/stripe"));
    assert!(guide.contains("Stripe webhook"));
    assert!(guide.contains("POST /shipments"));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn deploy_env_check_subcommand_is_accepted() {
    let parsed = Cli::try_parse_from(["orv", "deploy-env-check", "target/orv-build-test"]);
    if let Err(err) = parsed {
        panic!("{}", err.render());
    }
}

#[test]
fn build_prod_writes_deploy_manifest_and_server_entrypoint() {
    let (src_dir, path) = prod_server_source("build-prod-source");
    let out = temp_output_dir("build-prod-artifacts");

    cmd_build_with_profile(&path, &out, BuildProfile::Production).expect("prod build");

    let deploy_manifest_path = out.join("deploy").join("manifest.json");
    let deploy_container_path = out.join("deploy").join("container.json");
    let deploy_dockerfile_path = out.join("deploy").join("Dockerfile");
    let deploy_compose_path = out.join("deploy").join("compose.yaml");
    let deploy_env_example_path = out.join("deploy").join("env.example");
    let deploy_runbook_path = out.join("deploy").join("README.md");
    let deploy_routes_path = out.join("deploy").join("routes.json");
    let deploy_smoke_test_path = out.join("deploy").join("smoke-test.sh");
    let deploy_preflight_path = out.join("deploy").join("preflight.json");
    let deploy_benchmark_evidence_path = out.join("deploy").join("benchmark-evidence.json");
    let server_entrypoint_path = out.join("deploy").join("server.sh");
    let native_server_plan_path = out.join("server").join("native-server.json");
    assert!(
        deploy_manifest_path.is_file(),
        "missing {}",
        deploy_manifest_path.display()
    );
    assert!(
        deploy_container_path.is_file(),
        "missing {}",
        deploy_container_path.display()
    );
    assert!(
        deploy_dockerfile_path.is_file(),
        "missing {}",
        deploy_dockerfile_path.display()
    );
    assert!(
        deploy_compose_path.is_file(),
        "missing {}",
        deploy_compose_path.display()
    );
    assert!(
        deploy_env_example_path.is_file(),
        "missing {}",
        deploy_env_example_path.display()
    );
    assert!(
        deploy_runbook_path.is_file(),
        "missing {}",
        deploy_runbook_path.display()
    );
    assert!(
        deploy_routes_path.is_file(),
        "missing {}",
        deploy_routes_path.display()
    );
    assert!(
        deploy_smoke_test_path.is_file(),
        "missing {}",
        deploy_smoke_test_path.display()
    );
    assert!(
        deploy_preflight_path.is_file(),
        "missing {}",
        deploy_preflight_path.display()
    );
    assert!(
        deploy_benchmark_evidence_path.is_file(),
        "missing {}",
        deploy_benchmark_evidence_path.display()
    );
    assert!(
        server_entrypoint_path.is_file(),
        "missing {}",
        server_entrypoint_path.display()
    );
    assert!(
        native_server_plan_path.is_file(),
        "missing {}",
        native_server_plan_path.display()
    );
    let deploy = read_json_value(&deploy_manifest_path).expect("deploy manifest");
    assert_eq!(deploy["schema_version"], 1);
    assert_eq!(deploy["profile"], "prod");
    assert_eq!(deploy["entry"], path.display().to_string());
    assert_eq!(deploy["source_bundle"], "source-bundle.json");
    assert_eq!(deploy["server"]["artifact"], "server/app.orv-runtime.json");
    assert_eq!(deploy["server"]["entrypoint"], "deploy/server.sh");
    assert_eq!(deploy["server"]["container"], "deploy/container.json");
    assert_eq!(deploy["server"]["dockerfile"], "deploy/Dockerfile");
    assert_eq!(deploy["server"]["compose"], "deploy/compose.yaml");
    assert_eq!(deploy["server"]["env_example"], "deploy/env.example");
    assert_eq!(deploy["server"]["runbook"], "deploy/README.md");
    assert_eq!(deploy["server"]["smoke_test"], "deploy/smoke-test.sh");
    assert_eq!(deploy["server"]["smoke_output"], "deploy/smoke-output.txt");
    assert_eq!(deploy["server"]["preflight"], "deploy/preflight.json");
    assert_eq!(
        deploy["server"]["benchmark_evidence"],
        "deploy/benchmark-evidence.json"
    );
    assert_eq!(deploy["server"]["native_plan"], "server/native-server.json");
    assert_eq!(
        deploy["server"]["native_runtime_image_plan"],
        "server/runtime-image.json"
    );
    assert_eq!(
        deploy["server"]["native_routes_source"],
        "server/native/routes.rs"
    );
    assert_eq!(
        deploy["server"]["native_router_source"],
        "server/native/router.rs"
    );
    assert_eq!(
        deploy["server"]["native_handlers_source"],
        "server/native/handlers.rs"
    );
    assert_eq!(
        deploy["server"]["runtime_image"],
        "ghcr.io/orv-lang/orv-reference:latest"
    );
    assert_eq!(deploy["server"]["listen"]["port"], 8080);
    assert!(deploy["server"]["routes"]
        .as_array()
        .expect("server routes")
        .iter()
        .any(|route| route["method"] == "GET" && route["path"] == "/ping"));
    assert_eq!(deploy["server"]["routes_artifact"], "deploy/routes.json");
    let container = read_json_value(&deploy_container_path).expect("deploy container");
    assert_eq!(container["schema_version"], 1);
    assert_eq!(container["kind"], "reference-server-container");
    assert_eq!(container["artifact"], "server/app.orv-runtime.json");
    assert_eq!(container["entrypoint"], "deploy/server.sh");
    assert_eq!(container["routes_artifact"], "deploy/routes.json");
    assert_eq!(container["dockerfile"], "deploy/Dockerfile");
    assert_eq!(container["runtime"], "reference-interpreter");
    assert_eq!(
        container["runtime_image"],
        deploy["server"]["runtime_image"]
    );
    assert_eq!(container["protocol"], "http1");
    assert_eq!(container["listen"], deploy["server"]["listen"]);
    assert_eq!(container["ports"][0]["container"], 8080);
    assert_eq!(container["ports"][0]["protocol"], "tcp");
    assert_eq!(container["command"][0], "./deploy/server.sh");
    let dockerfile = std::fs::read_to_string(&deploy_dockerfile_path).expect("Dockerfile");
    assert!(dockerfile.contains("ARG ORV_RUNTIME_IMAGE=ghcr.io/orv-lang/orv-reference:latest"));
    assert!(dockerfile.contains("FROM ${ORV_RUNTIME_IMAGE}"));
    assert!(dockerfile.contains("COPY . /app"));
    assert!(dockerfile.contains("EXPOSE 8080"));
    assert!(dockerfile.contains(r#"ENTRYPOINT ["./deploy/server.sh"]"#));
    let compose = std::fs::read_to_string(&deploy_compose_path).expect("compose");
    assert!(compose.contains("dockerfile: deploy/Dockerfile"));
    assert!(compose.contains("ORV_RUNTIME_IMAGE: ghcr.io/orv-lang/orv-reference:latest"));
    assert!(compose.contains(r#""8080:8080""#));
    assert!(compose.contains(r#"PORT: "8080""#));
    let env_example = std::fs::read_to_string(&deploy_env_example_path).expect("env example");
    assert!(env_example.contains("PORT=8080"));
    let runbook = std::fs::read_to_string(&deploy_runbook_path).expect("deploy runbook");
    assert!(runbook.contains("docker compose -f deploy/compose.yaml up --build -d"));
    assert!(runbook.contains("deploy/env.example"));
    assert!(runbook.contains("PORT=8080"));
    assert!(runbook.contains("cargo build --manifest-path server/native/Cargo.toml --release"));
    assert!(runbook.contains("ORV_BUILD_DIR=. ./server/native/target/release/orv-native-server"));
    assert!(
        runbook.contains("docker build -f server/native/Dockerfile -t orv-native-server:latest .")
    );
    assert!(runbook.contains("ORV_BUILD_DIR is an explicit override"));
    assert!(runbook.contains("./deploy/server.sh --trace deploy/request-trace.json"));
    assert!(runbook.contains("./deploy/smoke-test.sh"));
    assert!(runbook.contains("deploy/smoke-output.txt"));
    assert!(runbook.contains("deploy/preflight.json"));
    assert!(runbook.contains("deploy/benchmark-evidence.json"));
    assert!(runbook.contains("## Benchmark Evidence"));
    assert!(runbook.contains("## Smoke Output Markers"));
    assert!(runbook.contains("- `pass_marker`"));
    assert!(runbook.contains("- `dap_source_bundle`"));
    assert!(runbook.contains("- `trace_stream_requested`"));
    assert!(runbook.contains("orv verify-build ."));
    assert!(runbook.contains("orv deploy-env-check ."));
    assert!(runbook.contains("orv editor run-debug . --control next"));
    assert!(runbook.contains("orv benchmark-report ."));
    assert!(runbook.contains("orv benchmark-report . --require-pass"));
    assert!(runbook.contains("/__orv/trace/events"));
    assert!(runbook.contains("orv editor trace . --trace deploy/request-trace.json"));
    assert!(runbook.contains("ORV_SMOKE_TRACE_STREAM=1 ./deploy/smoke-test.sh"));
    assert!(runbook.contains("- GET /ping"));
    let routes = read_json_value(&deploy_routes_path).expect("deploy routes");
    assert_eq!(routes["schema_version"], 1);
    assert_eq!(routes["artifact"], "server/app.orv-runtime.json");
    assert!(json_routes_include(&routes["routes"], "GET", "/ping"));
    let smoke_test = std::fs::read_to_string(&deploy_smoke_test_path).expect("smoke test");
    assert!(smoke_test.contains(r#"BASE_URL="${ORV_BASE_URL:-http://127.0.0.1:8080}""#));
    assert!(smoke_test.contains("command -v curl"));
    assert!(smoke_test.contains("orv deploy smoke test requires curl"));
    assert!(
        smoke_test.contains(r#"ORV_SMOKE_OUTPUT="${ORV_SMOKE_OUTPUT:-deploy/smoke-output.txt}""#)
    );
    assert!(smoke_test.contains(r#"> "$ORV_SMOKE_OUTPUT""#));
    assert!(smoke_test.contains("orv_smoke_write_output()"));
    assert!(smoke_test.contains("\norv_smoke_write_output\n"));
    assert!(smoke_test.contains("graph_contract=verified"));
    assert!(smoke_test.contains("dap_summary=verified"));
    assert!(smoke_test.contains("dap_source_bundle=verified"));
    assert!(smoke_test.contains(
        r#"orv_smoke_dap_summary_contains "dap source bundle panel" '"source_bundle": {'"#
    ));
    assert!(smoke_test.contains(
            r#"orv_smoke_dap_summary_contains "dap source bundle panel path" '"path": "./source-bundle.json"'"#
        ));
    assert!(smoke_test.contains(
        r#"orv_smoke_dap_summary_contains "dap source bundle panel file count" '"fileCount": 1'"#
    ));
    assert!(smoke_test
        .contains(r#"orv_smoke_dap_summary_contains "dap source bundle panel hash" '"hash": ""#));
    assert!(smoke_test.contains("orv_smoke_dap_summary_capture()"));
    assert!(smoke_test.contains("orv_smoke_dap_summary_cleanup()"));
    assert!(smoke_test.contains("\norv_smoke_dap_summary_cleanup\n"));
    assert!(smoke_test.contains(
        r#"orv_smoke_dap_summary_contains "dap smoke required markers" '"smoke_test_required_markers": ['"#
    ));
    assert!(smoke_test.contains(
        r#"orv_smoke_dap_summary_contains "dap smoke summary required markers" '"required_markers": ['"#
    ));
    assert!(smoke_test.contains(
        r#"orv_smoke_dap_summary_contains "dap smoke marker dap source bundle" '"dap_source_bundle"'"#
    ));
    assert!(smoke_test.contains("server_routes=1"));
    assert!(smoke_test.contains("trace_stream_requested=%s"));
    assert!(smoke_test.contains("orv_smoke_reveal_contains()"));
    assert!(smoke_test.contains("orv_smoke_editor_reveal_contains()"));
    assert!(smoke_test.contains("orv_smoke_lsp_reveal_contains()"));
    assert!(smoke_test.contains("lsp reveal"));
    assert!(smoke_test.contains("orv_smoke_trace_stream()"));
    assert!(smoke_test.contains("ORV_SMOKE_TRACE_STREAM"));
    assert!(smoke_test.contains("editor trace-stream"));
    assert!(smoke_test.contains(r#"'"kind":"orv.production.trace.frame"'"#));
    assert!(smoke_test.contains(r#"'"index":0'"#));
    assert!(smoke_test.contains(r#"'"frame":{'"#));
    assert!(smoke_test.contains(r#"'"trace_frame_event_count":'"#));
    assert!(smoke_test.contains("orv_smoke_curl()"));
    assert!(smoke_test.contains("orv_smoke_origin_header()"));
    assert!(smoke_test.contains("orv_smoke_response_origin_header()"));
    assert!(smoke_test.contains("orv_smoke_curl_origin()"));
    assert!(smoke_test.contains("orv_smoke_curl_origin_response()"));
    assert!(smoke_test.contains("orv deploy smoke test failed: %s"));
    assert!(smoke_test.contains(r#"READY_PATH="/ping""#));
    assert!(smoke_test.contains("for attempt in 1 2 3 4 5"));
    assert!(smoke_test.contains("sleep 1"));
    assert!(smoke_test.contains(r#"ORV_SMOKE_ORIGIN_GET_PING="ori_"#));
    assert!(smoke_test.contains(r#"ORV_SMOKE_RESPONSE_ORIGIN_GET_PING="ori_"#));
    assert!(smoke_test.contains(
            r#"orv_smoke_curl_origin_response "GET /ping" "$ORV_SMOKE_ORIGIN_GET_PING" "$ORV_SMOKE_RESPONSE_ORIGIN_GET_PING" "$BASE_URL/ping""#
        ));
    assert!(smoke_test.contains(
            r#"orv_smoke_reveal_contains "reveal GET /ping response source" "$ORV_SMOKE_RESPONSE_ORIGIN_GET_PING" '@respond'"#
        ));
    assert!(smoke_test.contains(
            r#"orv_smoke_reveal_contains "reveal GET /ping response production" "$ORV_SMOKE_RESPONSE_ORIGIN_GET_PING" '"response_origin_dispatch": true'"#
        ));
    assert!(smoke_test.contains(
            r#"orv_smoke_reveal_contains "reveal GET /ping native target summary" "$ORV_SMOKE_ORIGIN_GET_PING" '"native_server_target_count": 1'"#
        ));
    assert!(smoke_test.contains(
            r#"orv_smoke_editor_reveal_contains "editor reveal GET /ping native route summary" "$ORV_SMOKE_ORIGIN_GET_PING" '"native_server_route_count": 1'"#
        ));
    assert!(smoke_test.contains(
            r#"orv_smoke_lsp_reveal_contains "lsp reveal GET /ping native target summary" "$ORV_SMOKE_ORIGIN_GET_PING" '"native_server_target_count": 1'"#
        ));
    assert!(smoke_test.contains(
            r#"orv_smoke_dap_summary_contains "dap native target summary" '"native_server_target_count": 1'"#
        ));
    assert!(smoke_test.contains(
            r#"orv_smoke_dap_summary_contains "dap native route summary" '"native_server_route_count": 1'"#
        ));
    assert!(smoke_test.contains(
            r#"orv_smoke_lsp_reveal_contains "lsp reveal GET /ping response origin" "$ORV_SMOKE_RESPONSE_ORIGIN_GET_PING" '"name": "respond"'"#
        ));
    assert!(smoke_test.contains(
            r#"orv_smoke_lsp_reveal_contains "lsp reveal GET /ping response production" "$ORV_SMOKE_RESPONSE_ORIGIN_GET_PING" '"response_origin_dispatch": true'"#
        ));
    assert!(smoke_test.contains(
        r#"orv_smoke_reveal_contains "reveal smoke required markers" "$ORV_SMOKE_ORIGIN_GET_PING" '"smoke_test_required_markers": ['"#
    ));
    assert!(smoke_test.contains(
        r#"orv_smoke_reveal_contains "reveal smoke summary required markers" "$ORV_SMOKE_ORIGIN_GET_PING" '"required_markers": ['"#
    ));
    assert!(smoke_test.contains(
        r#"orv_smoke_reveal_contains "reveal smoke marker dap source bundle" "$ORV_SMOKE_ORIGIN_GET_PING" '"dap_source_bundle"'"#
    ));
    assert!(smoke_test.contains(
        r#"orv_smoke_editor_reveal_contains "editor reveal smoke required markers" "$ORV_SMOKE_ORIGIN_GET_PING" '"smoke_test_required_markers": ['"#
    ));
    assert!(smoke_test.contains(
        r#"orv_smoke_lsp_reveal_contains "lsp reveal smoke required markers" "$ORV_SMOKE_ORIGIN_GET_PING" '"smoke_test_required_markers": ['"#
    ));
    let preflight = read_json_value(&deploy_preflight_path).expect("deploy preflight");
    assert_eq!(preflight["schema_version"], 1);
    assert_eq!(preflight["kind"], "orv.deploy.preflight");
    assert_eq!(preflight["artifact"], "server/app.orv-runtime.json");
    assert_eq!(preflight["artifacts"]["smoke_test"], "deploy/smoke-test.sh");
    assert_eq!(
        preflight["artifacts"]["smoke_output"],
        "deploy/smoke-output.txt"
    );
    assert_eq!(
        preflight["smoke_output_contract"]["output"],
        "deploy/smoke-output.txt"
    );
    assert_eq!(
        preflight["smoke_output_contract"]["required_markers"],
        serde_json::json!(deploy_benchmark::SMOKE_REQUIRED_MARKERS)
    );
    assert_eq!(preflight["artifacts"]["preflight"], "deploy/preflight.json");
    assert_eq!(
        preflight["artifacts"]["benchmark_evidence"],
        "deploy/benchmark-evidence.json"
    );
    assert_eq!(preflight["artifacts"]["source_bundle"], SOURCE_BUNDLE_PATH);
    assert_eq!(
        preflight["artifacts"]["project_graph"],
        "project-graph.json"
    );
    assert_eq!(preflight["artifacts"]["origin_map"], "origin-map.json");
    assert_eq!(
        preflight["artifacts"]["build_manifest"],
        "build-manifest.json"
    );
    assert_eq!(preflight["artifacts"]["bundle_plan"], "bundle-plan.json");
    assert_eq!(preflight["commands"]["verify_build"], "orv verify-build .");
    assert_eq!(preflight["commands"]["env_check"], "orv deploy-env-check .");
    assert_eq!(preflight["commands"]["run_build"], "orv run-build .");
    assert_eq!(
        preflight["commands"]["trace_run_build"],
        "orv run-build . --trace deploy/request-trace.json"
    );
    assert_eq!(
        preflight["commands"]["smoke_test"],
        "./deploy/smoke-test.sh"
    );
    assert_eq!(
        preflight["commands"]["editor_run_debug"],
        "orv editor run-debug . --control next"
    );
    assert_eq!(
        preflight["commands"]["benchmark_prepare"],
        "orv benchmark-prepare . --participants 2"
    );
    assert_eq!(
        preflight["commands"]["benchmark_report"],
        "orv benchmark-report ."
    );
    assert_eq!(
        preflight["commands"]["benchmark_report_require_pass"],
        "orv benchmark-report . --require-pass"
    );
    assert_eq!(
        preflight["commands"]["trace_stream_smoke"],
        "ORV_SMOKE_TRACE_STREAM=1 ./deploy/smoke-test.sh"
    );
    assert_eq!(
        preflight["commands"]["compose_up"],
        "docker compose -f deploy/compose.yaml up --build -d"
    );
    assert_eq!(preflight["listen"], deploy["server"]["listen"]);
    assert_eq!(preflight["routes"], deploy["server"]["routes"]);
    assert_eq!(
        preflight["runtime_features"],
        deploy["server"]["runtime_features"]
    );
    let evidence = read_json_value(&deploy_benchmark_evidence_path).expect("benchmark evidence");
    assert_eq!(evidence["schema_version"], 1);
    assert_eq!(evidence["kind"], "orv.benchmark.shop_5h.evidence");
    assert_eq!(evidence["preflight"], "deploy/preflight.json");
    assert!(evidence["preflight_hash"].as_str().is_some());
    assert_eq!(evidence["benchmark"], preflight["benchmark"]);
    assert_eq!(evidence["commands"], preflight["commands"]);
    assert_eq!(evidence["artifacts"], preflight["artifacts"]);
    assert_eq!(
        evidence["smoke_output_contract"],
        preflight["smoke_output_contract"]
    );
    assert_eq!(evidence["recording_status"], "not_recorded");
    assert_eq!(
        evidence["task_entries"]
            .as_array()
            .expect("benchmark tasks")
            .len(),
        10
    );
    assert_eq!(
        evidence["data"]["elapsed_time_per_task"],
        "task_entries[*].elapsed_minutes"
    );
    assert!(evidence["data"]
        .as_object()
        .expect("benchmark data")
        .contains_key("smoke_test_output"));
    let script = std::fs::read_to_string(&server_entrypoint_path).expect("server entrypoint");
    assert!(script.contains("orv run-artifact"));

    cmd_verify_build(&out).expect("verify prod build");
    let _ = std::fs::remove_dir_all(src_dir);
    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn build_prod_mounts_file_db_connect_adapter_wal() {
    let dir = temp_output_dir("build-prod-file-db-connect-source");
    std::fs::create_dir_all(&dir).expect("create file db source dir");
    let path = dir.join("app.orv");
    std::fs::write(
        &path,
        r#"@server {
  @listen 8080
  let appdb = @db.connect "file://data/app.wal.jsonl"
  @route GET /ping { @respond 200 { ok: true } }
}
"#,
    )
    .expect("write file db source");
    let out = temp_output_dir("build-prod-file-db-connect");

    cmd_build_with_profile(&path, &out, BuildProfile::Production).expect("prod build");

    let manifest = read_json_value(&out.join("build-manifest.json")).expect("manifest");
    let deploy = read_json_value(&out.join("deploy").join("manifest.json")).expect("deploy");
    let runtime =
        read_json_value(&out.join("server").join("app.orv-runtime.json")).expect("runtime");
    let container = read_json_value(&out.join("deploy").join("container.json")).expect("container");
    let compose =
        std::fs::read_to_string(out.join("deploy").join("compose.yaml")).expect("compose");
    let runbook = std::fs::read_to_string(out.join("deploy").join("README.md")).expect("runbook");

    assert_eq!(
        deploy["server"]["persistence"]["wal_paths"][0],
        serde_json::json!("data/app.wal.jsonl")
    );
    assert!(manifest["capabilities"]["runtime_features"]
        .as_array()
        .expect("runtime features")
        .iter()
        .any(|feature| feature == "db_adapter"));
    assert!(runtime["runtime_features"]
        .as_array()
        .expect("runtime features")
        .iter()
        .any(|feature| feature == "db_adapter"));
    assert!(deploy["server"]["runtime_features"]
        .as_array()
        .expect("runtime features")
        .iter()
        .any(|feature| feature == "db_adapter"));
    assert_eq!(
        container["persistence"]["volumes"][0]["host"],
        serde_json::json!("data")
    );
    assert!(compose.contains("../data:/app/data"));
    assert!(runbook.contains("- WAL: data/app.wal.jsonl"));
    assert!(runbook.contains("- Compose volume: ../data:/app/data"));
    cmd_verify_build(&out).expect("verify prod build");
    let _ = std::fs::remove_dir_all(dir);
    let _ = std::fs::remove_dir_all(out);
}

#[test]
fn build_prod_mounts_sqlite_db_connect_adapter_file() {
    let dir = temp_output_dir("build-prod-sqlite-db-connect-source");
    std::fs::create_dir_all(&dir).expect("create sqlite db source dir");
    let path = dir.join("app.orv");
    std::fs::write(
        &path,
        r#"@server {
  @listen 8080
  let appdb = @db.connect "sqlite://data/app.sqlite"
  @route GET /ping { @respond 200 { ok: true } }
}
"#,
    )
    .expect("write sqlite db source");
    let out = temp_output_dir("build-prod-sqlite-db-connect");

    cmd_build_with_profile(&path, &out, BuildProfile::Production).expect("prod build");

    let deploy = read_json_value(&out.join("deploy").join("manifest.json")).expect("deploy");
    let container = read_json_value(&out.join("deploy").join("container.json")).expect("container");
    let compose =
        std::fs::read_to_string(out.join("deploy").join("compose.yaml")).expect("compose");
    let runbook = std::fs::read_to_string(out.join("deploy").join("README.md")).expect("runbook");

    assert_eq!(
        deploy["server"]["persistence"]["db_paths"],
        serde_json::json!(["data/app.sqlite"])
    );
    assert_eq!(
        container["persistence"]["volumes"][0]["host"],
        serde_json::json!("data")
    );
    assert!(compose.contains("../data:/app/data"));
    assert!(runbook.contains("- DB: data/app.sqlite"));
    cmd_verify_build(&out).expect("verify prod build");
    let _ = std::fs::remove_dir_all(dir);
    let _ = std::fs::remove_dir_all(out);
}

#[test]
fn build_prod_records_env_configured_sqlite_db_adapter() {
    let dir = temp_output_dir("build-prod-env-sqlite-db-connect-source");
    std::fs::create_dir_all(&dir).expect("create env sqlite db source dir");
    let path = dir.join("app.orv");
    std::fs::write(
        &path,
        r#"@server {
  @listen 8080
  let appdb = @db.connect(@env.SHOP_DATABASE_URL ?? "sqlite://data/app.sqlite")
  @route GET /ping { @respond 200 { ok: true } }
}
"#,
    )
    .expect("write env sqlite db source");
    let out = temp_output_dir("build-prod-env-sqlite-db-connect");

    cmd_build_with_profile(&path, &out, BuildProfile::Production).expect("prod build");

    let deploy = read_json_value(&out.join("deploy").join("manifest.json")).expect("deploy");
    let container = read_json_value(&out.join("deploy").join("container.json")).expect("container");
    let compose =
        std::fs::read_to_string(out.join("deploy").join("compose.yaml")).expect("compose");
    let runbook = std::fs::read_to_string(out.join("deploy").join("README.md")).expect("runbook");

    assert_eq!(
        deploy["server"]["persistence"]["db_paths"],
        serde_json::json!(["data/app.sqlite"])
    );
    assert_eq!(
        deploy["server"]["persistence"]["db_env"],
        serde_json::json!([
            {
                "env": "SHOP_DATABASE_URL",
                "default": "sqlite://data/app.sqlite"
            }
        ])
    );
    assert_eq!(
        container["persistence"]["db_env"],
        deploy["server"]["persistence"]["db_env"]
    );
    assert!(compose.contains("../data:/app/data"));
    assert!(
        compose.contains(r#"SHOP_DATABASE_URL: "${SHOP_DATABASE_URL:-sqlite://data/app.sqlite}""#)
    );
    assert!(runbook.contains("- DB: data/app.sqlite"));
    assert!(
        runbook.contains("- DB adapter env: SHOP_DATABASE_URL default sqlite://data/app.sqlite")
    );
    cmd_verify_build(&out).expect("verify prod build");
    let _ = std::fs::remove_dir_all(dir);
    let _ = std::fs::remove_dir_all(out);
}

#[test]
fn build_prod_records_external_db_adapter_endpoints_without_volumes() {
    let dir = temp_output_dir("build-prod-external-db-connect-source");
    std::fs::create_dir_all(&dir).expect("create external db source dir");
    let path = dir.join("app.orv");
    std::fs::write(
        &path,
        r#"@server {
  @listen 8080
  let analytics = @db.connect "postgres://db.internal/shop"
  let shopdb = @db.connect(@env.SHOP_DATABASE_URL ?? "mysql://db.internal/shop")
  @route GET /ping { @respond 200 { ok: true } }
}
"#,
    )
    .expect("write external db source");
    let out = temp_output_dir("build-prod-external-db-connect");

    cmd_build_with_profile(&path, &out, BuildProfile::Production).expect("prod build");

    let deploy = read_json_value(&out.join("deploy").join("manifest.json")).expect("deploy");
    let container = read_json_value(&out.join("deploy").join("container.json")).expect("container");
    let db_adapters_path = out.join("deploy").join("db-adapters.json");
    let db_adapters = read_json_value(&db_adapters_path).expect("db adapters");
    let compose =
        std::fs::read_to_string(out.join("deploy").join("compose.yaml")).expect("compose");
    let env_example =
        std::fs::read_to_string(out.join("deploy").join("env.example")).expect("env example");
    let smoke_test =
        std::fs::read_to_string(out.join("deploy").join("smoke-test.sh")).expect("smoke test");
    let preflight = read_json_value(&out.join("deploy").join("preflight.json")).expect("preflight");
    let runbook = std::fs::read_to_string(out.join("deploy").join("README.md")).expect("runbook");

    assert_eq!(
        deploy["server"]["persistence"]["db_endpoints"],
        serde_json::json!(["mysql://db.internal/shop", "postgres://db.internal/shop"])
    );
    assert_eq!(deploy["server"]["db_adapters"], "deploy/db-adapters.json");
    assert_eq!(db_adapters["schema_version"], 1);
    assert_eq!(db_adapters["artifact"], "server/app.orv-runtime.json");
    let adapters = db_adapters["adapters"].as_array().expect("db adapters");
    assert_eq!(adapters.len(), 2);
    assert!(adapters.iter().all(|adapter| adapter["source_origin_id"]
        .as_str()
        .is_some_and(|origin_id| origin_id.starts_with("ori_"))));
    assert_eq!(
        adapter_values_without_source_origin_ids(&db_adapters["adapters"]),
        serde_json::json!([
            {
                "kind": "db",
                "mode": "external",
                "provider": "mysql",
                "env": "SHOP_DATABASE_URL",
                "default": "mysql://db.internal/shop",
                "endpoint": "mysql://db.internal/shop",
                "adapter_status": "unsupported_runtime",
                "runtime": {
                    "status": "unsupported_runtime",
                    "query_methods": ["create", "find", "update", "delete", "transaction"]
                },
                "bridge": {
                    "contract": "http-json-v1",
                    "method": "POST",
                    "content_type": "application/json",
                    "query_methods": [
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
                    ],
                    "body": {
                        "kind": "orv.db.adapter",
                        "contract": "http-json-v1",
                        "provider": "adapter provider",
                        "url": "adapter url",
                        "method": "db method",
                        "args": "runtime value array"
                    },
                    "retry": {
                        "attempts": 3,
                        "on": ["5xx", "connect_error", "read_error", "timeout"]
                    },
                    "env": [
                        {
                            "env": "ORV_DB_ADAPTER_MYSQL_ENDPOINT",
                            "required": true,
                            "purpose": "bridge_endpoint"
                        },
                        {
                            "env": "ORV_DB_ADAPTER_MYSQL_AUTH_TOKEN",
                            "required": false,
                            "purpose": "bridge_auth_token"
                        },
                        {
                            "env": "ORV_DB_ADAPTER_ENDPOINT",
                            "required": false,
                            "purpose": "bridge_endpoint_fallback"
                        },
                        {
                            "env": "ORV_DB_ADAPTER_AUTH_TOKEN",
                            "required": false,
                            "purpose": "bridge_auth_token_fallback"
                        }
                    ]
                }
            },
            {
                "kind": "db",
                "mode": "external",
                "provider": "postgres",
                "env": null,
                "default": null,
                "endpoint": "postgres://db.internal/shop",
                "adapter_status": "unsupported_runtime",
                "runtime": {
                    "status": "unsupported_runtime",
                    "query_methods": ["create", "find", "update", "delete", "transaction"]
                },
                "bridge": {
                    "contract": "http-json-v1",
                    "method": "POST",
                    "content_type": "application/json",
                    "query_methods": [
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
                    ],
                    "body": {
                        "kind": "orv.db.adapter",
                        "contract": "http-json-v1",
                        "provider": "adapter provider",
                        "url": "adapter url",
                        "method": "db method",
                        "args": "runtime value array"
                    },
                    "retry": {
                        "attempts": 3,
                        "on": ["5xx", "connect_error", "read_error", "timeout"]
                    },
                    "env": [
                        {
                            "env": "ORV_DB_ADAPTER_POSTGRES_ENDPOINT",
                            "required": true,
                            "purpose": "bridge_endpoint"
                        },
                        {
                            "env": "ORV_DB_ADAPTER_POSTGRES_AUTH_TOKEN",
                            "required": false,
                            "purpose": "bridge_auth_token"
                        },
                        {
                            "env": "ORV_DB_ADAPTER_ENDPOINT",
                            "required": false,
                            "purpose": "bridge_endpoint_fallback"
                        },
                        {
                            "env": "ORV_DB_ADAPTER_AUTH_TOKEN",
                            "required": false,
                            "purpose": "bridge_auth_token_fallback"
                        }
                    ]
                }
            }
        ])
    );
    assert!(container["persistence"]["volumes"]
        .as_array()
        .expect("volumes")
        .is_empty());
    assert_eq!(
        container["persistence"]["db_endpoints"],
        deploy["server"]["persistence"]["db_endpoints"]
    );
    assert!(
        compose.contains(r#"SHOP_DATABASE_URL: "${SHOP_DATABASE_URL:-mysql://db.internal/shop}""#)
    );
    assert!(
        compose.contains(r#"ORV_DB_ADAPTER_MYSQL_ENDPOINT: "${ORV_DB_ADAPTER_MYSQL_ENDPOINT}""#)
    );
    assert!(compose
        .contains(r#"ORV_DB_ADAPTER_POSTGRES_ENDPOINT: "${ORV_DB_ADAPTER_POSTGRES_ENDPOINT}""#));
    assert!(compose.contains(r#"ORV_DB_ADAPTER_ENDPOINT: "${ORV_DB_ADAPTER_ENDPOINT}""#));
    assert!(env_example.contains("SHOP_DATABASE_URL=mysql://db.internal/shop"));
    assert!(env_example.contains("ORV_DB_ADAPTER_MYSQL_ENDPOINT="));
    assert!(env_example.contains("ORV_DB_ADAPTER_POSTGRES_ENDPOINT="));
    assert!(env_example.contains("ORV_DB_ADAPTER_ENDPOINT="));
    assert!(preflight["required_env"]
        .as_array()
        .expect("required preflight env")
        .iter()
        .any(|env| env["env"] == "ORV_DB_ADAPTER_MYSQL_ENDPOINT"
            && env["provider"] == "mysql"
            && env["purpose"] == "bridge_endpoint"));
    assert!(preflight["required_env"]
        .as_array()
        .expect("required preflight env")
        .iter()
        .any(|env| env["env"] == "ORV_DB_ADAPTER_POSTGRES_ENDPOINT"
            && env["provider"] == "postgres"
            && env["purpose"] == "bridge_endpoint"));
    assert!(runbook.contains("- DB endpoint: mysql://db.internal/shop"));
    assert!(runbook.contains("- DB endpoint: postgres://db.internal/shop"));
    assert!(
        runbook.contains("- DB adapter env: SHOP_DATABASE_URL default mysql://db.internal/shop")
    );
    assert!(runbook
        .contains("- DB bridge env: mysql ORV_DB_ADAPTER_MYSQL_ENDPOINT required bridge_endpoint"));
    assert!(runbook.contains(
        "- DB bridge env: postgres ORV_DB_ADAPTER_POSTGRES_ENDPOINT required bridge_endpoint"
    ));
    assert!(smoke_test.contains(r#"orv_smoke_file "deploy/db-adapters.json""#));
    assert!(smoke_test.contains(
            r#"orv_smoke_grep "db adapter bridge contract" "deploy/db-adapters.json" '"contract": "http-json-v1"'"#
        ));
    assert!(smoke_test.contains(
            r#"orv_smoke_db_bridge_schema "mysql bridge" "${ORV_DB_ADAPTER_MYSQL_ENDPOINT:-${ORV_DB_ADAPTER_ENDPOINT:-}}" "mysql" "mysql://db.internal/shop" "${ORV_DB_ADAPTER_MYSQL_AUTH_TOKEN:-${ORV_DB_ADAPTER_AUTH_TOKEN:-}}""#
        ));
    assert!(smoke_test.contains(
            r#"orv_smoke_db_bridge_schema "postgres bridge" "${ORV_DB_ADAPTER_POSTGRES_ENDPOINT:-${ORV_DB_ADAPTER_ENDPOINT:-}}" "postgres" "postgres://db.internal/shop" "${ORV_DB_ADAPTER_POSTGRES_AUTH_TOKEN:-${ORV_DB_ADAPTER_AUTH_TOKEN:-}}""#
        ));
    assert!(runbook.contains("deploy/db-adapters.json"));
    cmd_verify_build(&out).expect("verify prod build");
    let _ = std::fs::remove_dir_all(dir);
    let _ = std::fs::remove_dir_all(out);
}

#[test]
fn build_prod_mounts_file_commerce_adapter_records() {
    let dir = temp_output_dir("build-prod-file-commerce-source");
    std::fs::create_dir_all(&dir).expect("create file commerce source dir");
    let path = dir.join("app.orv");
    std::fs::write(
        &path,
        r#"@server {
  @listen 8080
  let payments = @payment.connect("file://records/payments.jsonl")
  let shipping = @shipping.connect("file://records/shipments.jsonl")
  @route POST /checkout {
    let captured = payments.capture({ orderId: "o_1", amount: 42, method: "card" })
    let booked = shipping.book({ orderId: "o_1", carrier: "post", address: "Seoul" })
    @respond 200 { payment: captured.status, shipment: booked.status }
  }
}
"#,
    )
    .expect("write file commerce source");
    let out = temp_output_dir("build-prod-file-commerce");

    cmd_build_with_profile(&path, &out, BuildProfile::Production).expect("prod build");

    let deploy = read_json_value(&out.join("deploy").join("manifest.json")).expect("deploy");
    let container = read_json_value(&out.join("deploy").join("container.json")).expect("container");
    let compose =
        std::fs::read_to_string(out.join("deploy").join("compose.yaml")).expect("compose");
    let runbook = std::fs::read_to_string(out.join("deploy").join("README.md")).expect("runbook");

    assert_eq!(
        deploy["server"]["persistence"]["record_paths"],
        serde_json::json!(["records/payments.jsonl", "records/shipments.jsonl"])
    );
    assert_eq!(
        container["persistence"]["volumes"][0]["host"],
        serde_json::json!("records")
    );
    assert!(compose.contains("../records:/app/records"));
    assert!(runbook.contains("- Record log: records/payments.jsonl"));
    assert!(runbook.contains("- Record log: records/shipments.jsonl"));
    cmd_verify_build(&out).expect("verify prod build");
    let _ = std::fs::remove_dir_all(dir);
    let _ = std::fs::remove_dir_all(out);
}

#[test]
fn build_prod_records_http_commerce_adapter_endpoints() {
    let dir = temp_output_dir("build-prod-http-commerce-source");
    std::fs::create_dir_all(&dir).expect("create http commerce source dir");
    let path = dir.join("app.orv");
    std::fs::write(
        &path,
        r#"@server {
  @listen 8080
  let payments = @payment.connect("http://payments.internal/capture")
  let shipping = @shipping.connect("http://shipping.internal/book")
  @route POST /checkout {
    let captured = payments.capture({ orderId: "o_1", amount: 42, method: "card" })
    let booked = shipping.book({ orderId: "o_1", carrier: "post", address: "Seoul" })
    @respond 200 { payment: captured.status, shipment: booked.status }
  }
}
"#,
    )
    .expect("write http commerce source");
    let out = temp_output_dir("build-prod-http-commerce");

    cmd_build_with_profile(&path, &out, BuildProfile::Production).expect("prod build");

    let deploy = read_json_value(&out.join("deploy").join("manifest.json")).expect("deploy");
    let container = read_json_value(&out.join("deploy").join("container.json")).expect("container");
    let compose =
        std::fs::read_to_string(out.join("deploy").join("compose.yaml")).expect("compose");
    let runbook = std::fs::read_to_string(out.join("deploy").join("README.md")).expect("runbook");

    assert_eq!(
        deploy["server"]["persistence"]["commerce_endpoints"],
        serde_json::json!([
            "http://payments.internal/capture",
            "http://shipping.internal/book"
        ])
    );
    assert_eq!(
        container["persistence"]["commerce_endpoints"],
        deploy["server"]["persistence"]["commerce_endpoints"]
    );
    assert!(container["persistence"]["volumes"]
        .as_array()
        .expect("volumes")
        .is_empty());
    assert!(!compose.contains("../records:/app/records"));
    assert!(runbook.contains("- Commerce endpoint: http://payments.internal/capture"));
    assert!(runbook.contains("- Commerce endpoint: http://shipping.internal/book"));
    cmd_verify_build(&out).expect("verify prod build");
    let _ = std::fs::remove_dir_all(dir);
    let _ = std::fs::remove_dir_all(out);
}

#[test]
fn build_prod_records_provider_commerce_adapters() {
    let dir = temp_output_dir("build-prod-provider-commerce-source");
    std::fs::create_dir_all(&dir).expect("create provider commerce source dir");
    let path = dir.join("app.orv");
    std::fs::write(
        &path,
        r#"@server {
  @listen 8080
  let payments = @payment.connect(@env.PAYMENT_ADAPTER_URL ?? "stripe://local")
  let shipping = @shipping.connect(@env.SHIPPING_ADAPTER_URL ?? "carrier://local")
  @route POST /checkout {
    let captured = payments.capture({ orderId: "o_1", amount: 42, method: "card" })
    let booked = shipping.book({ orderId: "o_1", carrier: "post", address: "Seoul" })
    @respond 200 { payment: captured.status, shipment: booked.status }
  }
}
"#,
    )
    .expect("write provider commerce source");
    let out = temp_output_dir("build-prod-provider-commerce");

    cmd_build_with_profile(&path, &out, BuildProfile::Production).expect("prod build");

    let deploy = read_json_value(&out.join("deploy").join("manifest.json")).expect("deploy");
    let container = read_json_value(&out.join("deploy").join("container.json")).expect("container");
    let compose =
        std::fs::read_to_string(out.join("deploy").join("compose.yaml")).expect("compose");
    let commerce_adapters = read_json_value(&out.join("deploy").join("commerce-adapters.json"))
        .expect("commerce adapters");
    let env_example =
        std::fs::read_to_string(out.join("deploy").join("env.example")).expect("env example");
    let runbook = std::fs::read_to_string(out.join("deploy").join("README.md")).expect("runbook");

    assert_eq!(
        deploy["server"]["persistence"]["commerce_endpoints"],
        serde_json::json!([])
    );
    assert!(container["persistence"]["volumes"]
        .as_array()
        .expect("volumes")
        .is_empty());
    assert!(compose.contains(r#"PAYMENT_ADAPTER_URL: "${PAYMENT_ADAPTER_URL:-stripe://local}""#));
    assert!(compose.contains(r#"SHIPPING_ADAPTER_URL: "${SHIPPING_ADAPTER_URL:-carrier://local}""#));
    assert!(compose.contains(r#"STRIPE_SECRET_KEY: "${STRIPE_SECRET_KEY}""#));
    assert!(compose.contains(r#"STRIPE_API_ENDPOINT: "${STRIPE_API_ENDPOINT}""#));
    assert!(compose.contains(r#"STRIPE_WEBHOOK_SECRET: "${STRIPE_WEBHOOK_SECRET}""#));
    assert!(
        compose.contains(r#"STRIPE_WEBHOOK_SECRET_PREVIOUS: "${STRIPE_WEBHOOK_SECRET_PREVIOUS}""#)
    );
    assert!(compose.contains(r#"CARRIER_API_KEY: "${CARRIER_API_KEY}""#));
    assert!(compose.contains(r#"CARRIER_API_ENDPOINT: "${CARRIER_API_ENDPOINT}""#));
    assert!(compose.contains(r#"CARRIER_WEBHOOK_SECRET: "${CARRIER_WEBHOOK_SECRET}""#));
    assert_eq!(
        adapter_values_without_source_origin_ids(&commerce_adapters["adapters"]),
        serde_json::json!([
            {
                "kind": "payment",
                "surface": "library_provider_package",
                "package": "orv-commerce",
                "provider_package": "orv-stripe",
                "mode": "provider",
                "provider": "stripe",
                "env": "PAYMENT_ADAPTER_URL",
                "default": "stripe://local",
                "endpoint": null,
                "record_path": null,
                "provider_env": [
                    {
                        "env": "STRIPE_API_ENDPOINT",
                        "required": false,
                        "purpose": "api_endpoint"
                    },
                    {
                        "env": "STRIPE_SECRET_KEY",
                        "required": true,
                        "purpose": "api_secret"
                    },
                    {
                        "env": "STRIPE_WEBHOOK_SECRET",
                        "required": false,
                        "purpose": "webhook_signature"
                    },
                    {
                        "env": "STRIPE_WEBHOOK_SECRET_PREVIOUS",
                        "required": false,
                        "purpose": "webhook_signature_previous"
                    }
                ],
                "request": {
                    "method": "POST",
                    "content_type": "application/json",
                    "kind": "payment.capture",
                    "body": {
                        "kind": "payment.capture",
                        "payload": "payment capture payload"
                    }
                }
            },
            {
                "kind": "shipping",
                "surface": "library_provider_package",
                "package": "orv-commerce",
                "provider_package": "orv-carrier",
                "mode": "provider",
                "provider": "carrier",
                "env": "SHIPPING_ADAPTER_URL",
                "default": "carrier://local",
                "endpoint": null,
                "record_path": null,
                "provider_env": [
                    {
                        "env": "CARRIER_API_ENDPOINT",
                        "required": false,
                        "purpose": "api_endpoint"
                    },
                    {
                        "env": "CARRIER_API_KEY",
                        "required": true,
                        "purpose": "api_key"
                    },
                    {
                        "env": "CARRIER_WEBHOOK_SECRET",
                        "required": false,
                        "purpose": "webhook_signature"
                    }
                ],
                "request": {
                    "method": "POST",
                    "content_type": "application/json",
                    "kind": "shipping.booking",
                    "body": {
                        "kind": "shipping.booking",
                        "payload": "shipping booking payload"
                    }
                }
            }
        ])
    );
    assert!(commerce_adapters["adapters"]
        .as_array()
        .expect("commerce adapters")
        .iter()
        .all(|adapter| adapter["source_origin_id"]
            .as_str()
            .is_some_and(|origin_id| origin_id.starts_with("ori_"))));
    assert!(env_example.contains("STRIPE_API_ENDPOINT="));
    assert!(env_example.contains("STRIPE_SECRET_KEY="));
    assert!(env_example.contains("STRIPE_WEBHOOK_SECRET="));
    assert!(env_example.contains("STRIPE_WEBHOOK_SECRET_PREVIOUS="));
    assert!(env_example.contains("CARRIER_API_ENDPOINT="));
    assert!(env_example.contains("CARRIER_API_KEY="));
    assert!(env_example.contains("CARRIER_WEBHOOK_SECRET="));
    assert!(runbook.contains("- Commerce adapter env: PAYMENT_ADAPTER_URL default stripe://local"));
    assert!(
        runbook.contains("- Commerce adapter env: SHIPPING_ADAPTER_URL default carrier://local")
    );
    assert!(runbook.contains(
        "- Commerce provider env: payment stripe STRIPE_API_ENDPOINT optional api_endpoint"
    ));
    assert!(runbook
        .contains("- Commerce provider env: payment stripe STRIPE_SECRET_KEY required api_secret"));
    assert!(runbook.contains(
        "- Commerce provider env: payment stripe STRIPE_WEBHOOK_SECRET optional webhook_signature"
    ));
    assert!(runbook.contains(
            "- Commerce provider env: payment stripe STRIPE_WEBHOOK_SECRET_PREVIOUS optional webhook_signature_previous"
        ));
    assert!(runbook.contains(
        "- Commerce provider env: shipping carrier CARRIER_API_ENDPOINT optional api_endpoint"
    ));
    assert!(runbook
        .contains("- Commerce provider env: shipping carrier CARRIER_API_KEY required api_key"));
    assert!(runbook.contains(
            "- Commerce provider env: shipping carrier CARRIER_WEBHOOK_SECRET optional webhook_signature"
        ));
    assert!(runbook.contains(
        "- Secret store: supply commerce provider credentials through deployment secret manager or vault values, not deploy/env.example."
    ));
    assert!(runbook.contains(
        "- Stripe webhook rotation: set STRIPE_WEBHOOK_SECRET to the new value and STRIPE_WEBHOOK_SECRET_PREVIOUS to the previous value during overlap."
    ));
    assert!(runbook.contains(
        "- Stripe replay window: STRIPE_WEBHOOK_TOLERANCE_SECONDS defaults to 300 seconds; override only with provider runbook approval."
    ));
    assert!(runbook.contains(
        "- Provider replay: payment and shipping calls use stable idempotency keys; inspect provider records before retrying checkout compensation."
    ));
    assert!(runbook.contains("orv deploy-env-check ."));
    cmd_verify_build(&out).expect("verify prod build");
    let _ = std::fs::remove_dir_all(dir);
    let _ = std::fs::remove_dir_all(out);
}

#[test]
fn deploy_env_check_reports_missing_required_provider_credentials() {
    let dir = temp_output_dir("deploy-env-check-provider-source");
    std::fs::create_dir_all(&dir).expect("create provider commerce source dir");
    let path = dir.join("app.orv");
    std::fs::write(
        &path,
        r#"@server {
  @listen 8080
  let payments = @payment.connect(@env.PAYMENT_ADAPTER_URL ?? "stripe://local")
  let shipping = @shipping.connect(@env.SHIPPING_ADAPTER_URL ?? "carrier://local")
  @route POST /checkout {
    let captured = payments.capture({ orderId: "o_1", amount: 42, method: "card" })
    let booked = shipping.book({ orderId: "o_1", carrier: "post", address: "Seoul" })
    @respond 200 { payment: captured.status, shipment: booked.status }
  }
}
"#,
    )
    .expect("write provider commerce source");
    let out = temp_output_dir("deploy-env-check-provider");

    cmd_build_with_profile(&path, &out, BuildProfile::Production).expect("prod build");

    let err = deploy_env_check_with_lookup(&out, |_| None).expect_err("required envs are missing");
    let message = err.to_string();
    assert!(message.contains("STRIPE_SECRET_KEY"), "{message}");
    assert!(message.contains("CARRIER_API_KEY"), "{message}");

    deploy_env_check_with_lookup(&out, |env| match env {
        "STRIPE_SECRET_KEY" => Some("sk_test".to_string()),
        "CARRIER_API_KEY" => Some("carrier_key".to_string()),
        _ => None,
    })
    .expect("optional webhook envs may be absent");
    let _ = std::fs::remove_dir_all(dir);
    let _ = std::fs::remove_dir_all(out);
}

#[test]
fn deploy_env_check_reports_missing_required_db_adapter_env() {
    let dir = temp_output_dir("deploy-env-check-db-source");
    std::fs::create_dir_all(&dir).expect("create db adapter source dir");
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
    .expect("write db adapter source");
    let out = temp_output_dir("deploy-env-check-db");

    cmd_build_with_profile(&path, &out, BuildProfile::Production).expect("prod build");

    let err = deploy_env_check_with_lookup(&out, |_| None).expect_err("required DB env missing");
    let message = err.to_string();
    assert!(message.contains("SHOP_DATABASE_URL"), "{message}");

    deploy_env_check_with_lookup(&out, |env| match env {
        "SHOP_DATABASE_URL" => Some("postgres://db.internal/shop".to_string()),
        _ => None,
    })
    .expect("configured DB env passes");
    let _ = std::fs::remove_dir_all(dir);
    let _ = std::fs::remove_dir_all(out);
}

#[test]
fn deploy_env_check_reports_missing_required_db_bridge_endpoint() {
    let dir = temp_output_dir("deploy-env-check-db-bridge-source");
    std::fs::create_dir_all(&dir).expect("create db bridge source dir");
    let path = dir.join("app.orv");
    std::fs::write(
        &path,
        r#"@server {
  @listen 8080
  let shopdb = @db.connect "postgres://db.internal/shop"
  @route GET /ping { @respond 200 { ok: true } }
}
"#,
    )
    .expect("write db bridge source");
    let out = temp_output_dir("deploy-env-check-db-bridge");

    cmd_build_with_profile(&path, &out, BuildProfile::Production).expect("prod build");

    let err = deploy_env_check_with_lookup(&out, |_| None).expect_err("required DB bridge missing");
    let message = err.to_string();
    assert!(
        message.contains("ORV_DB_ADAPTER_POSTGRES_ENDPOINT"),
        "{message}"
    );

    deploy_env_check_with_lookup(&out, |env| match env {
        "ORV_DB_ADAPTER_ENDPOINT" => Some("http://db-adapter.internal/shared".to_string()),
        _ => None,
    })
    .expect("generic DB bridge endpoint fallback passes");

    deploy_env_check_with_lookup(&out, |env| match env {
        "ORV_DB_ADAPTER_POSTGRES_ENDPOINT" => {
            Some("http://db-adapter.internal/postgres".to_string())
        }
        _ => None,
    })
    .expect("configured DB bridge endpoint passes");
    let _ = std::fs::remove_dir_all(dir);
    let _ = std::fs::remove_dir_all(out);
}

#[test]
fn deploy_adapter_source_origin_rejects_missing_and_non_call_entries() {
    let call_entry = orv_compiler::OriginEntry {
        id: "ori_db_connect".to_string(),
        kind: "call".to_string(),
        name: "@db.connect".to_string(),
        span: orv_compiler::OriginSpan {
            file: 0,
            start: 0,
            end: 4,
        },
        fingerprint: "fp_call".to_string(),
    };
    let route_entry = orv_compiler::OriginEntry {
        id: "ori_route".to_string(),
        kind: "route".to_string(),
        name: "GET /ping".to_string(),
        span: orv_compiler::OriginSpan {
            file: 0,
            start: 5,
            end: 9,
        },
        fingerprint: "fp_route".to_string(),
    };
    let mut entries_by_id = std::collections::HashMap::new();
    entries_by_id.insert(call_entry.id.as_str(), &call_entry);
    entries_by_id.insert(route_entry.id.as_str(), &route_entry);

    crate::build_deploy::verify_deploy_adapter_source_origin(
        &entries_by_id,
        "ori_db_connect",
        "deploy DB adapter",
        "@db.connect",
    )
    .expect("call entry origin must pass");

    let missing = crate::build_deploy::verify_deploy_adapter_source_origin(
        &entries_by_id,
        "ori_gone",
        "deploy DB adapter",
        "@db.connect",
    )
    .expect_err("missing origin must fail");
    assert!(missing
        .to_string()
        .contains("deploy DB adapter source_origin_id `ori_gone` not found in origin-map.json"));

    let non_call = crate::build_deploy::verify_deploy_adapter_source_origin(
        &entries_by_id,
        "ori_route",
        "deploy commerce adapter payment",
        "@payment.connect",
    )
    .expect_err("non-call origin must fail");
    assert!(non_call.to_string().contains(
        "deploy commerce adapter payment source_origin_id `ori_route` must reference origin-map call @payment.connect"
    ));
}
