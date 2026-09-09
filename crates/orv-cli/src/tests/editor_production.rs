use super::*;

#[test]
fn editor_export_with_build_carries_production_context_into_debug_runner() {
    let dir = temp_output_dir("editor-export-debug-production-context");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let path = dir.join("app.orv");
    std::fs::write(&path, "let total: int = 41\n@out total\n").expect("write source");
    let build_out = dir.join("dist");
    let editor_out = dir.join("editor");

    cmd_build_with_profile(&path, &build_out, BuildProfile::Production).expect("prod build");
    cmd_editor_export_with_options(&path, &editor_out, Some(&build_out), None)
        .expect("editor export with build");

    let state = read_json_value(&editor_out.join("state.json")).expect("editor state");
    let runner =
        read_json_value(&editor_out.join(EDITOR_DEBUG_SESSION_RUNNER_PATH)).expect("runner");
    let native_host =
        read_json_value(&editor_out.join(EDITOR_NATIVE_HOST_MANIFEST_PATH)).expect("native host");
    let production_context = &state["debug"]["production_context"];

    assert_eq!(
        production_context["kind"],
        "orv.editor.debug.production_context"
    );
    assert_eq!(
        production_context["build_dir"],
        build_out.display().to_string()
    );
    assert_eq!(
        production_context["summary"]["graph_contract_count"],
        serde_json::json!(3)
    );
    assert_eq!(
        production_context["summary"]["source_bundle_file_count"],
        serde_json::json!(1)
    );
    assert_eq!(
        production_context["source_bundle"],
        build_out.join(SOURCE_BUNDLE_PATH).display().to_string()
    );
    assert!(production_context["graph_contract"]
        .as_array()
        .expect("graph contract")
        .iter()
        .any(|target| target["path"] == SOURCE_BUNDLE_PATH));
    assert_eq!(runner["production_context"], *production_context);
    assert_eq!(
        runner["source_bundle"],
        build_out.join(SOURCE_BUNDLE_PATH).display().to_string()
    );
    assert_eq!(
        native_host["debug"]["production_context"],
        *production_context
    );
    assert_eq!(native_host["capabilities"]["dap_production_context"], true);
    assert!(native_host["debug"]["panel_contract"]["sections"]
        .as_array()
        .expect("native host debug sections")
        .iter()
        .any(|section| section["name"] == "production_context"
            && section["path"] == "debug.production_context"));

    let run = editor_debug_runner_session_json(
        &editor_out.join(EDITOR_DEBUG_SESSION_RUNNER_PATH),
        &[EditorDebugControl::Next],
        &[],
        &[],
        &[],
        &[],
        &[],
    )
    .expect("run debug runner with production context");
    assert_eq!(run["production_context"], *production_context);
    assert_eq!(
        run["panels"]["debug"]["production_context"],
        *production_context
    );
    assert_eq!(
        run["panels"]["debug"]["production_summary"],
        production_context["summary"]
    );
    assert!(
        run["panels"]["debug"]["result_artifact"]["panel_contract"]["sections"]
            .as_array()
            .expect("debug result panel sections")
            .iter()
            .any(|section| section["name"] == "production_context"
                && section["path"] == "panels.debug.production_context")
    );
    assert!(
        run["panels"]["debug"]["result_artifact"]["panel_contract"]["sections"]
            .as_array()
            .expect("debug result panel sections")
            .iter()
            .any(|section| section["name"] == "production_summary"
                && section["path"] == "panels.debug.production_summary")
    );
    let result_html = editor_debug_runner_result_html(&run).expect("debug result html");
    assert!(result_html.contains("Production Summary"));
    assert!(result_html.contains("Production Context"));
    assert!(result_html.contains("source-bundle.json"));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn editor_run_debug_result_summarizes_native_production_targets() {
    let dir = temp_output_dir("editor-run-debug-production-summary");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let path = dir.join("server.orv");
    std::fs::write(
        &path,
        r"@server {
  @listen 8080
  @route GET /ping {
    @respond 200 { ok: true }
  }
}",
    )
    .expect("write source");
    let build_out = dir.join("dist");
    let editor_out = dir.join("editor");

    cmd_build_with_profile(&path, &build_out, BuildProfile::Production).expect("prod build");
    cmd_editor_export_with_options(&path, &editor_out, Some(&build_out), None)
        .expect("editor export with build");

    let run = editor_debug_runner_session_json(
        &editor_out.join(EDITOR_DEBUG_SESSION_RUNNER_PATH),
        &[EditorDebugControl::Next],
        &[],
        &[],
        &[],
        &[],
        &[],
    )
    .expect("run debug runner with production summary");
    assert_eq!(
        run["panels"]["debug"]["production_summary"]["native_server_target_count"],
        1
    );
    assert_eq!(
        run["panels"]["debug"]["production_summary"]["native_server_route_count"],
        1
    );
    assert_eq!(
        run["panels"]["debug"]["production_summary"]["preflight_target_count"],
        1
    );
    assert_eq!(
        run["panels"]["debug"]["production_context"]["preflight"][0]["benchmark_evidence"]
            ["smoke_test_required_markers"],
        serde_json::json!(deploy_benchmark::SMOKE_REQUIRED_MARKERS)
    );
    assert_eq!(
        run["panels"]["debug"]["production_context"]["preflight"][0]["benchmark_evidence"]
            ["smoke_test_summary"]["required_markers"],
        serde_json::json!(deploy_benchmark::SMOKE_REQUIRED_MARKERS)
    );
    assert!(
        run["panels"]["debug"]["production_context"]["preflight"][0]["benchmark_evidence"]
            ["missing_data"]
            .as_array()
            .expect("missing data")
            .iter()
            .any(|item| item == "recording_status.recorded")
    );
    assert_eq!(
        run["panels"]["debug"]["production_context"]["preflight"][0]["benchmark_evidence"]
            ["participant_raw_notes_artifacts"][0]["checked"],
        false
    );
    let result_html = editor_debug_runner_result_html(&run).expect("debug result html");
    assert!(result_html.contains("Production Summary"));
    assert!(result_html.contains("native_server_target_count"));
    assert!(result_html.contains("native plans, 1 routes"));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn editor_run_debug_result_summarizes_client_production_targets() {
    let dir = temp_output_dir("editor-run-debug-client-production-summary");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let path = dir.join("page.orv");
    std::fs::write(
        &path,
        "let sig count: int = 0\n@out @html { @body { @p count } }",
    )
    .expect("write source");
    let build_out = dir.join("dist");
    let editor_out = dir.join("editor");

    cmd_build_with_profile(&path, &build_out, BuildProfile::Production).expect("prod build");
    cmd_editor_export_with_options(&path, &editor_out, Some(&build_out), None)
        .expect("editor export with build");

    let run = editor_debug_runner_session_json(
        &editor_out.join(EDITOR_DEBUG_SESSION_RUNNER_PATH),
        &[EditorDebugControl::Next],
        &[],
        &[],
        &[],
        &[],
        &[],
    )
    .expect("run debug runner with client production summary");
    assert_eq!(
        run["panels"]["debug"]["production_summary"]["client_target_count"],
        5
    );
    assert_eq!(
        run["panels"]["debug"]["production_summary"]["client_manifest_count"],
        1
    );
    assert!(
        run["panels"]["debug"]["production_summary"]["client_capability_surface_count"]
            .as_u64()
            .is_some_and(|count| count >= 2),
        "{run}"
    );
    let result_html = editor_debug_runner_result_html(&run).expect("debug result html");
    assert!(result_html.contains("Production Summary"));
    assert!(result_html.contains("client_target_count"));
    assert!(result_html.contains("client targets, 1 manifests"));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn editor_run_debug_result_summarizes_static_production_targets() {
    let dir = temp_output_dir("editor-run-debug-static-production-summary");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let path = dir.join("page.orv");
    std::fs::write(&path, r#"@out @html { @body { @h1 "Home" } }"#).expect("write source");
    let build_out = dir.join("dist");
    let editor_out = dir.join("editor");

    cmd_build_with_profile(&path, &build_out, BuildProfile::Production).expect("prod build");
    cmd_editor_export_with_options(&path, &editor_out, Some(&build_out), None)
        .expect("editor export with build");

    let run = editor_debug_runner_session_json(
        &editor_out.join(EDITOR_DEBUG_SESSION_RUNNER_PATH),
        &[EditorDebugControl::Next],
        &[],
        &[],
        &[],
        &[],
        &[],
    )
    .expect("run debug runner with static production summary");
    assert_eq!(
        run["panels"]["debug"]["production_summary"]["static_target_count"],
        1
    );
    assert_eq!(
        run["panels"]["debug"]["production_summary"]["static_verified_count"],
        1
    );
    let result_html = editor_debug_runner_result_html(&run).expect("debug result html");
    assert!(result_html.contains("Production Summary"));
    assert!(result_html.contains("static_target_count"));
    assert!(result_html.contains("1/1"));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn editor_export_with_build_embeds_production_adapter_summary() {
    let dir = temp_output_dir("editor-export-production-source");
    std::fs::create_dir_all(&dir).expect("create editor export source dir");
    let path = dir.join("app.orv");
    std::fs::write(
        &path,
        r#"@server {
  @listen 8080
  let shopdb = @db.connect(@env.SHOP_DATABASE_URL ?? "postgres://db.internal/shop")
  let payments = @payment.connect(@env.PAYMENT_ADAPTER_URL ?? "http://payments.internal/capture")
  @route POST /checkout {
    @csrf
    let captured = payments.capture({ orderId: "o_1", amount: 42, method: "card" })
    @respond 200 { payment: captured.status }
  }
}
"#,
    )
    .expect("write editor export source");
    let out = temp_output_dir("editor-export-production");

    cmd_build_with_profile(&path, &out, BuildProfile::Production).expect("prod build");
    let state =
        editor_export_state_json_with_trace(&path, Some(&out), None).expect("editor export state");
    let html = editor_export_html(&state).expect("editor html");

    let graph_contract = state["production"]["graph_contract"]
        .as_array()
        .expect("graph contract targets");
    let source_bundle_target = graph_contract
        .iter()
        .find(|target| target["kind"] == "source_bundle")
        .expect("source bundle target");
    let project_graph_target = graph_contract
        .iter()
        .find(|target| target["kind"] == "project_graph")
        .expect("project graph target");
    let origin_map_target = graph_contract
        .iter()
        .find(|target| target["kind"] == "origin_map")
        .expect("origin map target");
    assert_eq!(source_bundle_target["path"], SOURCE_BUNDLE_PATH);
    assert_eq!(source_bundle_target["exists"], true);
    assert_eq!(source_bundle_target["file_count"], 1);
    assert!(source_bundle_target["artifact_hash"].as_str().is_some());
    assert!(source_bundle_target["files"]
        .as_array()
        .expect("source bundle files")
        .iter()
        .any(|file| file["path"]
            .as_str()
            .is_some_and(|path| path.ends_with("app.orv"))
            && file["content_hash"].as_str().is_some()));
    assert_eq!(project_graph_target["path"], "project-graph.json");
    assert_eq!(project_graph_target["exists"], true);
    assert!(project_graph_target["node_count"]
        .as_u64()
        .is_some_and(|count| count > 0));
    assert!(project_graph_target["semantic_origin_count"]
        .as_u64()
        .is_some_and(|count| count > 0));
    assert_eq!(origin_map_target["path"], "origin-map.json");
    assert_eq!(origin_map_target["exists"], true);
    assert!(origin_map_target["entry_count"]
        .as_u64()
        .is_some_and(|count| count > 0));
    assert_eq!(
        state["production"]["db_adapters"][0]["path"],
        "deploy/db-adapters.json"
    );
    assert_eq!(
        state["production"]["commerce_adapters"][0]["path"],
        "deploy/commerce-adapters.json"
    );
    let db_origin_id = state["production"]["db_adapters"][0]["adapters"][0]["source_origin_id"]
        .as_str()
        .expect("db adapter source origin");
    let commerce_origin_id = state["production"]["commerce_adapters"][0]["adapters"][0]
        ["source_origin_id"]
        .as_str()
        .expect("commerce adapter source origin");
    assert_eq!(
        state["production"]["db_adapters"][0]["source_reveal_commands"][0]["source_origin_id"],
        db_origin_id
    );
    assert_eq!(
        state["production"]["db_adapters"][0]["source_reveal_commands"][0]["command"],
        serde_json::json!([
            "orv",
            "editor",
            "reveal",
            out.display().to_string(),
            db_origin_id
        ])
    );
    assert_eq!(
        state["production"]["commerce_adapters"][0]["source_reveal_commands"][0]
            ["source_origin_id"],
        commerce_origin_id
    );
    assert_eq!(
        state["production"]["commerce_adapters"][0]["source_reveal_commands"][0]["command"],
        serde_json::json!([
            "orv",
            "editor",
            "reveal",
            out.display().to_string(),
            commerce_origin_id
        ])
    );
    assert_eq!(
        state["production"]["preflight"][0]["path"],
        "deploy/preflight.json"
    );
    assert_eq!(
        state["production"]["preflight"][0]["commands"]["verify_build"],
        "orv verify-build ."
    );
    assert_eq!(
        state["production"]["preflight"][0]["commands"]["benchmark_prepare"],
        "orv benchmark-prepare . --participants 2"
    );
    assert_eq!(
        state["production"]["preflight"][0]["commands"]["benchmark_report"],
        "orv benchmark-report ."
    );
    assert_eq!(
        state["production"]["preflight"][0]["commands"]["benchmark_report_require_pass"],
        "orv benchmark-report . --require-pass"
    );
    assert_eq!(
        state["production"]["preflight"][0]["artifacts"]["benchmark_evidence"],
        "deploy/benchmark-evidence.json"
    );
    assert_eq!(
        state["production"]["preflight"][0]["artifacts"]["smoke_output"],
        "deploy/smoke-output.txt"
    );
    assert_eq!(
        state["production"]["preflight"][0]["smoke_output_contract"]["output"],
        "deploy/smoke-output.txt"
    );
    assert_eq!(
        state["production"]["preflight"][0]["smoke_output_contract"]["required_markers"],
        serde_json::json!(deploy_benchmark::SMOKE_REQUIRED_MARKERS)
    );
    assert_eq!(
        state["production"]["preflight"][0]["benchmark_evidence"]["recording_status"],
        "not_recorded"
    );
    assert_eq!(
        state["production"]["preflight"][0]["benchmark_evidence"]["report_status"],
        "incomplete"
    );
    assert_eq!(
        state["production"]["preflight"][0]["benchmark_evidence"]["missing_task_count"],
        10
    );
    assert_eq!(
        state["production"]["preflight"][0]["benchmark_evidence"]["missing_data_count"],
        16
    );
    assert_eq!(
        state["production"]["preflight"][0]["benchmark_evidence"]["failed_data_count"],
        0
    );
    assert_eq!(
        state["production"]["preflight"][0]["benchmark_evidence"]["failed_data"],
        serde_json::json!([])
    );
    assert_eq!(
        state["production"]["preflight"][0]["benchmark_evidence"]["smoke_test_required_markers"],
        serde_json::json!(deploy_benchmark::SMOKE_REQUIRED_MARKERS)
    );
    assert_eq!(
        state["production"]["preflight"][0]["benchmark_evidence"]["smoke_test_summary"]["present"],
        false
    );
    assert_eq!(
        state["production"]["preflight"][0]["benchmark_evidence"]["smoke_test_summary"]
            ["required_markers"],
        serde_json::json!(deploy_benchmark::SMOKE_REQUIRED_MARKERS)
    );
    assert_eq!(
        state["production"]["preflight"][0]["benchmark_evidence"]
            ["participant_raw_notes_artifacts"][0]["checked"],
        false
    );
    assert_eq!(
        state["production"]["summary"]["schema_version"],
        serde_json::json!(1)
    );
    assert_eq!(state["production"]["summary"]["graph_contract_count"], 3);
    assert_eq!(
        state["production"]["native_server"][0]["path"],
        "server/native-server.json"
    );
    assert_eq!(
        state["production"]["summary"]["native_server_target_count"],
        1
    );
    assert_eq!(
        state["production"]["summary"]["native_server_route_count"],
        1
    );
    assert_eq!(state["production"]["summary"]["static_target_count"], 0);
    assert_eq!(state["production"]["summary"]["preflight_target_count"], 1);
    assert_eq!(
        state["production"]["summary"]["preflight_smoke_summary_present_count"],
        0
    );
    assert_eq!(
        state["production"]["summary"]["preflight_smoke_summary_missing_count"],
        1
    );
    assert_eq!(
        state["production"]["summary"]["preflight_smoke_summary_missing_marker_count"],
        0
    );
    assert!(
        state["production"]["preflight"][0]["benchmark_evidence"]["missing_data"]
            .as_array()
            .expect("missing data")
            .iter()
            .any(|item| item == "smoke_test_output")
    );
    assert!(
        state["production"]["preflight"][0]["benchmark_evidence"]["missing_data"]
            .as_array()
            .expect("missing data")
            .iter()
            .any(|item| item == "recording_status.recorded")
    );
    assert!(
        state["production"]["preflight"][0]["benchmark_evidence"]["missing_data"]
            .as_array()
            .expect("missing data")
            .iter()
            .any(|item| item == "ai_assistance_used")
    );
    assert!(
        state["production"]["preflight"][0]["benchmark_evidence"]["missing_data"]
            .as_array()
            .expect("missing data")
            .iter()
            .any(|item| item == "generated_artifact_edits")
    );
    assert!(
        state["production"]["preflight"][0]["benchmark_evidence"]["missing_data"]
            .as_array()
            .expect("missing data")
            .iter()
            .any(|item| item == "manual_undocumented_security_steps")
    );
    assert!(
        state["production"]["preflight"][0]["benchmark_evidence"]["missing_data"]
            .as_array()
            .expect("missing data")
            .iter()
            .any(|item| item == "human_evidence_review.raw_notes_reviewed")
    );
    assert!(
        state["production"]["preflight"][0]["benchmark_evidence"]["missing_data"]
            .as_array()
            .expect("missing data")
            .iter()
            .any(|item| item == "participant_runs.minimum")
    );
    let checkout_route = json_route(
        &state["production"]["preflight"][0]["routes"],
        "POST",
        "/checkout",
    )
    .expect("checkout route");
    assert!(checkout_route["policies"]
        .as_array()
        .expect("checkout policies")
        .iter()
        .any(|policy| policy["kind"] == "csrf"
            && policy["surface"] == "first_party_compiler_plugin"
            && policy["required"] == true
            && policy["origin_id"]
                .as_str()
                .is_some_and(|origin_id| origin_id.starts_with("ori_"))));
    assert!(checkout_route["policies"]
        .as_array()
        .expect("checkout policies")
        .iter()
        .any(|policy| policy["kind"] == "rate_limit"
            && policy["surface"] == "shop_template"
            && policy["limit"] == 10
            && policy["window_seconds"] == 60));
    let native_host = editor_native_host_manifest_json(&path, &state);
    assert_eq!(
        native_host["production"]["db_adapters"][0]["path"],
        "deploy/db-adapters.json"
    );
    assert_eq!(
        native_host["production"]["commerce_adapters"][0]["path"],
        "deploy/commerce-adapters.json"
    );
    assert_eq!(
        native_host["production"]["db_adapters"][0]["source_reveal_commands"][0]
            ["source_origin_id"],
        db_origin_id
    );
    assert_eq!(
        native_host["production"]["db_adapters"][0]["source_reveal_commands"][0]["command"],
        serde_json::json!([
            "orv",
            "editor",
            "reveal",
            out.display().to_string(),
            db_origin_id
        ])
    );
    assert_eq!(
        native_host["production"]["commerce_adapters"][0]["source_reveal_commands"][0]
            ["source_origin_id"],
        commerce_origin_id
    );
    assert_eq!(
        native_host["production"]["commerce_adapters"][0]["source_reveal_commands"][0]["command"],
        serde_json::json!([
            "orv",
            "editor",
            "reveal",
            out.display().to_string(),
            commerce_origin_id
        ])
    );
    assert_eq!(
        native_host["production"]["preflight"][0]["path"],
        "deploy/preflight.json"
    );
    assert_eq!(
        native_host["production"]["graph_contract"],
        state["production"]["graph_contract"]
    );
    assert_eq!(
        native_host["production"]["summary"],
        state["production"]["summary"]
    );
    assert_eq!(
        native_host["production"]["summary"]["schema_version"],
        serde_json::json!(1)
    );
    assert_eq!(
        native_host["production"]["summary"]["graph_contract_count"],
        3
    );
    assert_eq!(
        native_host["production"]["summary"]["source_bundle_file_count"],
        1
    );
    assert!(
        native_host["production"]["summary"]["project_graph_node_count"]
            .as_u64()
            .is_some_and(|count| count > 0)
    );
    assert!(native_host["production"]["summary"]["origin_entry_count"]
        .as_u64()
        .is_some_and(|count| count > 0));
    assert_eq!(
        native_host["production"]["summary"]["preflight_target_count"],
        1
    );
    assert_eq!(
        native_host["production"]["summary"]["preflight_command_count"],
        13
    );
    assert_eq!(
        native_host["production"]["summary"]["preflight_route_count"],
        1
    );
    assert_eq!(
        native_host["production"]["summary"]["native_server_target_count"],
        1
    );
    assert_eq!(
        native_host["production"]["summary"]["native_server_route_count"],
        1
    );
    assert_eq!(
        native_host["production"]["summary"]["static_target_count"],
        0
    );
    assert_eq!(
        native_host["production"]["summary"]["route_policy_count"],
        2
    );
    assert_eq!(
        native_host["production"]["summary"]["route_policy_kind_counts"]["csrf"],
        1
    );
    assert_eq!(
        native_host["production"]["summary"]["route_policy_kind_counts"]["rate_limit"],
        1
    );
    assert_eq!(
        native_host["production"]["summary"]["preflight_optional_env_count"],
        5
    );
    assert_eq!(
        native_host["production"]["summary"]["preflight_smoke_summary_present_count"],
        0
    );
    assert_eq!(
        native_host["production"]["summary"]["preflight_smoke_summary_missing_count"],
        1
    );
    assert_eq!(
        native_host["production"]["summary"]["preflight_smoke_summary_missing_marker_count"],
        0
    );
    assert_eq!(native_host["production"]["summary"]["db_target_count"], 1);
    assert_eq!(
        native_host["production"]["summary"]["commerce_target_count"],
        1
    );
    assert!(
        native_host["production"]["summary"]["adapter_count"]
            .as_u64()
            .is_some_and(|count| count >= 2),
        "{native_host}"
    );
    assert_eq!(
        native_host["production"]["summary"]["missing_artifact_count"],
        0
    );
    assert_eq!(
        native_host["production"]["panel_contract"]["root"],
        "production"
    );
    let production_sections = native_host["production"]["panel_contract"]["sections"]
        .as_array()
        .expect("production panel sections");
    assert!(production_sections
        .iter()
        .any(|section| section["name"] == "summary" && section["path"] == "production.summary"));
    assert!(production_sections
        .iter()
        .any(|section| section["name"] == "graph_contract"
            && section["path"] == "production.graph_contract"));
    assert!(production_sections
        .iter()
        .any(|section| section["name"] == "db_adapters"
            && section["path"] == "production.db_adapters"));
    assert!(
        production_sections
            .iter()
            .any(|section| section["name"] == "preflight"
                && section["path"] == "production.preflight")
    );
    assert!(production_sections
        .iter()
        .any(|section| section["name"] == "native_server"
            && section["path"] == "production.native_server"));
    assert!(production_sections
        .iter()
        .any(|section| section["name"] == "static" && section["path"] == "production.static"));
    assert!(production_sections
        .iter()
        .any(|section| section["name"] == "route_policies"
            && section["path"] == "production.summary.route_policy_kind_counts"));
    assert!(production_sections
        .iter()
        .any(|section| section["name"] == "commerce_adapters"
            && section["path"] == "production.commerce_adapters"));
    assert!(production_sections
        .iter()
        .any(|section| section["name"] == "panel_artifact"
            && section["path"] == "production.panel_artifact"));
    assert_eq!(
        native_host["production"]["panel_html_path"],
        EDITOR_PRODUCTION_PANEL_HTML_PATH
    );
    assert_eq!(
        native_host["production"]["panel_artifact"]["path"],
        EDITOR_PRODUCTION_PANEL_HTML_PATH
    );
    assert_eq!(
        native_host["production"]["panel_artifact"]["kind"],
        "orv.editor.production.panel"
    );
    assert_eq!(native_host["capabilities"]["production_adapters"], true);
    assert_eq!(
        native_host["capabilities"]["production_graph_contract"],
        true
    );
    assert_eq!(native_host["capabilities"]["production_preflight"], true);
    assert_eq!(
        native_host["capabilities"]["production_route_policies"],
        true
    );
    assert!(html.contains("Production"));
    assert!(html.contains("Graph source_bundle source-bundle.json"));
    assert!(html.contains("Preflight"));
    assert!(html.contains("commands 13"));
    assert!(html.contains("route_policies 2"));
    assert!(html.contains("smoke_summary_present false"));
    assert!(html.contains("DB Adapters"));
    assert!(html.contains("Commerce Adapters"));
    assert!(html.contains("deploy/db-adapters.json"));
    let editor_out = dir.join("editor");
    cmd_editor_export_with_options(&path, &editor_out, Some(&out), None)
        .expect("editor export with production panel");
    let export_native_host =
        read_json_value(&editor_out.join(EDITOR_NATIVE_HOST_MANIFEST_PATH)).expect("native host");
    let production_panel =
        std::fs::read_to_string(editor_out.join(EDITOR_PRODUCTION_PANEL_HTML_PATH))
            .expect("production panel");
    assert_eq!(
        export_native_host["artifacts"]["production_panel_html"],
        EDITOR_PRODUCTION_PANEL_HTML_PATH
    );
    let export_panels = export_native_host["panels"]
        .as_array()
        .expect("native host panel inventory");
    assert!(export_panels.iter().any(|panel| {
        panel["name"] == "production"
            && panel["artifact"]["path"] == EDITOR_PRODUCTION_PANEL_HTML_PATH
    }));
    assert!(production_panel.contains("Production Panel"));
    assert!(production_panel.contains("Graph Contract"));
    assert!(production_panel.contains("source-bundle.json"));
    assert!(production_panel.contains("project-graph.json"));
    assert!(production_panel.contains("origin-map.json"));
    assert!(production_panel.contains("Native Server"));
    assert!(production_panel.contains("Native Plans</span><b>1</b>"));
    assert!(production_panel.contains("Native Routes</span><b>1</b>"));
    assert!(production_panel.contains("Static Pages"));
    assert!(production_panel.contains("Static Pages</span><b>0/0</b>"));
    assert!(production_panel.contains("Preflight"));
    assert!(production_panel
        .contains("\"benchmark_prepare\": \"orv benchmark-prepare . --participants 2\""));
    assert!(production_panel.contains("\"benchmark_report\": \"orv benchmark-report .\""));
    assert!(production_panel
        .contains("\"benchmark_report_require_pass\": \"orv benchmark-report . --require-pass\""));
    assert!(production_panel.contains("\"report_status\": \"incomplete\""));
    assert!(production_panel.contains("\"missing_task_count\": 10"));
    assert!(production_panel.contains("\"smoke_test_required_markers\""));
    assert!(production_panel.contains("\"dap_source_bundle\""));
    assert!(production_panel.contains("\"smoke_test_summary\""));
    assert!(production_panel.contains("\"required_markers\""));
    assert!(production_panel.contains("\"present\": false"));
    assert!(production_panel.contains("\"preflight_smoke_summary_present_count\": 0"));
    assert!(production_panel.contains("\"preflight_smoke_summary_missing_count\": 1"));
    assert!(production_panel.contains("\"preflight_smoke_summary_missing_marker_count\": 0"));
    assert!(production_panel.contains("Smoke Summary</span><b>0/1</b>"));
    assert!(production_panel.contains("Smoke Gaps</span><b class=\"bad\">1</b>"));
    assert!(production_panel.contains("\"smoke_test_output\""));
    assert!(production_panel.contains("Preflight Commands</span><b>13</b>"));
    assert!(production_panel.contains("Route Policies"));
    assert!(production_panel.contains("Route Policy Summary"));
    assert!(production_panel.contains("\"csrf\": 1"));
    assert!(production_panel.contains("\"rate_limit\": 1"));
    assert!(production_panel.contains("DB Adapters"));
    assert!(production_panel.contains("Commerce Adapters"));
    assert!(production_panel.contains("deploy/preflight.json"));
    assert!(production_panel.contains("deploy/benchmark-evidence.json"));
    assert!(production_panel.contains("deploy/db-adapters.json"));
    let _ = std::fs::remove_dir_all(dir);
    let _ = std::fs::remove_dir_all(out);
}

#[test]
fn editor_export_with_build_embeds_production_client_capabilities() {
    let dir = temp_output_dir("editor-export-production-client-source");
    std::fs::create_dir_all(&dir).expect("create editor export client source dir");
    let path = dir.join("page.orv");
    std::fs::write(
        &path,
        "let sig count: int = 0\n@out @html { @body { @p count } }",
    )
    .expect("write editor export client source");
    let out = temp_output_dir("editor-export-production-client");

    cmd_build_with_profile(&path, &out, BuildProfile::Production).expect("prod build");
    let state =
        editor_export_state_json_with_trace(&path, Some(&out), None).expect("editor export state");
    let html = editor_export_html(&state).expect("editor html");
    let client_targets = state["production"]["client"]
        .as_array()
        .expect("production client targets");
    let client_manifest = client_targets
        .iter()
        .find(|target| target["kind"] == "client_manifest")
        .expect("client manifest target");

    assert_eq!(client_manifest["path"], CLIENT_MANIFEST_PATH);
    assert_eq!(
        client_manifest["capabilities"]["runtime"],
        serde_json::json!("client_wasm")
    );
    assert_eq!(
        client_manifest["capabilities"]["bindings"]["signal_text"],
        1
    );
    assert!(client_manifest["capabilities"]["surfaces"]
        .as_array()
        .expect("client capability surfaces")
        .iter()
        .any(|surface| surface == "signal_text"));

    let native_host = editor_native_host_manifest_json(&path, &state);
    assert_eq!(native_host["capabilities"]["client_bundles"], true);
    assert_eq!(
        native_host["production"]["summary"]["client_manifest_count"],
        1
    );
    assert!(
        native_host["production"]["summary"]["client_target_count"]
            .as_u64()
            .is_some_and(|count| count >= 5),
        "{native_host}"
    );
    assert!(
        native_host["production"]["summary"]["client_capability_surface_count"]
            .as_u64()
            .is_some_and(|count| count >= 2),
        "{native_host}"
    );
    let production_sections = native_host["production"]["panel_contract"]["sections"]
        .as_array()
        .expect("production panel sections");
    assert!(production_sections
        .iter()
        .any(|section| section["name"] == "client" && section["path"] == "production.client"));
    assert!(html.contains("Client Bundles"));
    assert!(html.contains("client/app.wasm"));

    let editor_out = dir.join("editor");
    cmd_editor_export_with_options(&path, &editor_out, Some(&out), None)
        .expect("editor export with production client");
    let export_native_host =
        read_json_value(&editor_out.join(EDITOR_NATIVE_HOST_MANIFEST_PATH)).expect("native host");
    let production_panel =
        std::fs::read_to_string(editor_out.join(EDITOR_PRODUCTION_PANEL_HTML_PATH))
            .expect("production panel");
    assert_eq!(export_native_host["capabilities"]["client_bundles"], true);
    assert!(production_panel.contains("Client Bundles"));
    assert!(production_panel.contains("signal_text"));
    let _ = std::fs::remove_dir_all(dir);
    let _ = std::fs::remove_dir_all(out);
}
