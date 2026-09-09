use super::*;

#[test]
fn build_prod_smoke_test_documents_client_bundle_contract() {
    let dir = temp_output_dir("build-prod-client-smoke-source");
    std::fs::create_dir_all(&dir).expect("create temp root");
    let path = dir.join("app.orv");
    std::fs::write(
        &path,
        r#"@server {
  @listen 8080
  @route GET /ping {
    @respond 200 { ok: true }
  }
}

let sig count: int = 0
@out @html { @body { @p count } }
"#,
    )
    .expect("write source");
    let out = temp_output_dir("build-prod-client-smoke");

    cmd_build_with_profile(&path, &out, BuildProfile::Production).expect("prod build");
    let smoke_path = out.join("deploy").join("smoke-test.sh");
    let smoke = std::fs::read_to_string(&smoke_path).expect("deploy smoke test");
    let client_summary = deploy_client_summary_counts(&out).expect("client summary counts");

    assert!(smoke.contains("ORV_SMOKE_BUILD_DIR="));
    assert!(smoke.contains(r#"cd "$ORV_SMOKE_BUILD_DIR""#));
    assert!(smoke.contains("orv_smoke_file()"));
    assert!(smoke.contains("orv_smoke_grep()"));
    assert!(smoke.contains("orv_smoke_write_output()"));
    assert!(smoke.contains("graph_contract=verified"));
    assert!(smoke.contains("dap_summary=verified"));
    assert!(smoke.contains("dap_source_bundle=verified"));
    assert!(smoke.contains(
        r#"orv_smoke_dap_summary_contains "dap source bundle panel" '"source_bundle": {'"#
    ));
    assert!(smoke.contains(
            r#"orv_smoke_dap_summary_contains "dap source bundle panel path" '"path": "./source-bundle.json"'"#
        ));
    assert!(smoke.contains(
        r#"orv_smoke_dap_summary_contains "dap source bundle panel file count" '"fileCount": 1'"#
    ));
    assert!(smoke
        .contains(r#"orv_smoke_dap_summary_contains "dap source bundle panel hash" '"hash": ""#));
    assert!(smoke.contains("server_routes=1"));
    assert!(smoke.contains("trace_stream_requested=%s"));
    assert!(smoke.contains(r#"orv_smoke_file "client/manifest.json""#));
    assert!(smoke.contains(r#"orv_smoke_file "client/reactive-plan.json""#));
    assert!(smoke.contains(r#"orv_smoke_file "pages/index.html""#));
    assert!(smoke.contains(r#"orv_smoke_file "client/app.js""#));
    assert!(smoke.contains(r#"orv_smoke_file "client/app.wasm""#));
    assert!(smoke.contains(
        r#"orv_smoke_grep "client page marker" "pages/index.html" 'data-orv-client="wasm"'"#
    ));
    assert!(smoke.contains(
            r#"orv_smoke_grep "client manifest reactive plan path" "client/manifest.json" '"reactive_plan": "client/reactive-plan.json"'"#
        ));
    assert!(smoke.contains("client_manifest=client/manifest.json"));
    assert!(smoke.contains("client_reactive_plan=client/reactive-plan.json"));
    assert!(smoke.contains("client_page=pages/index.html"));
    assert!(smoke.contains("client_loader=client/app.js"));
    assert!(smoke.contains("client_wasm=client/app.wasm"));
    assert!(smoke.contains(r#"ORV_SMOKE_CLIENT_ORIGIN="ori_"#));
    assert!(smoke.contains(
            r#"orv_smoke_grep "client manifest reactive plan hash" "client/manifest.json" '"reactive_plan_hash"'"#
        ));
    assert!(smoke.contains(
        r#"orv_smoke_grep "client manifest loader hash" "client/manifest.json" '"loader_hash"'"#
    ));
    assert!(smoke.contains(
        r#"orv_smoke_grep "client manifest wasm hash" "client/manifest.json" '"wasm_hash"'"#
    ));
    assert!(smoke.contains(
            r#"orv_smoke_grep "client manifest source bundle" "client/manifest.json" '"source_bundle": "source-bundle.json"'"#
        ));
    assert!(smoke.contains(
            r#"orv_smoke_grep "client manifest runtime" "client/manifest.json" '"runtime": "client_wasm"'"#
        ));
    assert!(smoke.contains(
        r#"orv_smoke_grep "client manifest capabilities" "client/manifest.json" '"capabilities"'"#
    ));
    assert!(smoke.contains(
            r#"orv_smoke_grep "client manifest capability surfaces" "client/manifest.json" '"surfaces"'"#
        ));
    assert!(smoke.contains(
        r#"orv_smoke_grep "client manifest event actions" "client/manifest.json" '"event_actions"'"#
    ));
    assert!(smoke.contains(
            r#"orv_smoke_grep "client reactive plan kind" "client/reactive-plan.json" '"kind": "orv.client.reactive_plan"'"#
        ));
    assert!(smoke.contains(
            r#"orv_smoke_grep "client reactive plan source bundle" "client/reactive-plan.json" '"source_bundle": "source-bundle.json"'"#
        ));
    assert!(smoke.contains(
            r#"orv_smoke_grep "client reactive plan blocked_by" "client/reactive-plan.json" '"blocked_by"'"#
        ));
    assert!(smoke.contains(
        r#"orv_smoke_grep "client loader bootstrap" "client/app.js" 'ORV_CLIENT_BOOTSTRAP'"#
    ));
    assert!(smoke.contains(
            r#"orv_smoke_grep "client loader embedded reactive plan" "client/app.js" 'embeddedReactivePlan'"#
        ));
    assert!(smoke.contains(
            r#"orv_smoke_grep "client loader embedded reactive plan hash" "client/app.js" 'embeddedReactivePlanHash'"#
        ));
    assert!(smoke.contains(
        r#"orv_smoke_grep "client loader source bundle hash" "client/app.js" 'sourceBundleHash'"#
    ));
    assert!(smoke
        .contains(r#"orv_smoke_grep "client loader wasm reference" "client/app.js" 'app.wasm'"#));
    assert!(smoke.contains(
        r#"orv_smoke_grep "client loader signal setter" "client/app.js" '__ORV_SET_SIGNAL__'"#
    ));
    assert!(smoke.contains(&format!(
            r#"orv_smoke_reveal_contains "reveal client target summary" "$ORV_SMOKE_CLIENT_ORIGIN" '"client_target_count": {}'"#,
            client_summary.targets
        )));
    assert!(smoke.contains(&format!(
            r#"orv_smoke_reveal_contains "reveal client manifest summary" "$ORV_SMOKE_CLIENT_ORIGIN" '"client_manifest_count": {}'"#,
            client_summary.manifests
        )));
    assert!(smoke.contains(&format!(
            r#"orv_smoke_reveal_contains "reveal client capability summary" "$ORV_SMOKE_CLIENT_ORIGIN" '"client_capability_surface_count": {}'"#,
            client_summary.capability_surfaces
        )));
    assert!(smoke.contains(
            r#"orv_smoke_reveal_contains "reveal client manifest target" "$ORV_SMOKE_CLIENT_ORIGIN" '"path": "client/manifest.json"'"#
        ));
    assert!(smoke.contains(&format!(
            r#"orv_smoke_editor_reveal_contains "editor reveal client target summary" "$ORV_SMOKE_CLIENT_ORIGIN" '"client_target_count": {}'"#,
            client_summary.targets
        )));
    assert!(smoke.contains(&format!(
            r#"orv_smoke_editor_reveal_contains "editor reveal client manifest summary" "$ORV_SMOKE_CLIENT_ORIGIN" '"client_manifest_count": {}'"#,
            client_summary.manifests
        )));
    assert!(smoke.contains(&format!(
            r#"orv_smoke_editor_reveal_contains "editor reveal client capability summary" "$ORV_SMOKE_CLIENT_ORIGIN" '"client_capability_surface_count": {}'"#,
            client_summary.capability_surfaces
        )));
    assert!(smoke.contains(&format!(
            r#"orv_smoke_lsp_reveal_contains "lsp reveal client target summary" "$ORV_SMOKE_CLIENT_ORIGIN" '"client_target_count": {}'"#,
            client_summary.targets
        )));
    assert!(smoke.contains(&format!(
            r#"orv_smoke_lsp_reveal_contains "lsp reveal client manifest summary" "$ORV_SMOKE_CLIENT_ORIGIN" '"client_manifest_count": {}'"#,
            client_summary.manifests
        )));
    assert!(smoke.contains(&format!(
            r#"orv_smoke_lsp_reveal_contains "lsp reveal client capability summary" "$ORV_SMOKE_CLIENT_ORIGIN" '"client_capability_surface_count": {}'"#,
            client_summary.capability_surfaces
        )));
    let dap_client_target_gate = format!(
        r#"orv_smoke_dap_summary_contains "dap client target summary" '"client_target_count": {}'"#,
        client_summary.targets
    );
    assert!(smoke.contains(&dap_client_target_gate));
    assert!(smoke.contains(&format!(
            r#"orv_smoke_dap_summary_contains "dap client manifest summary" '"client_manifest_count": {}'"#,
            client_summary.manifests
        )));
    assert!(smoke.contains(&format!(
            r#"orv_smoke_dap_summary_contains "dap client capability summary" '"client_capability_surface_count": {}'"#,
            client_summary.capability_surfaces
        )));
    cmd_verify_build(&out).expect("verify client smoke test");

    let wrong_dap_client_target_gate = format!(
        r#"orv_smoke_dap_summary_contains "dap client target summary" '"client_target_count": {}'"#,
        client_summary.targets + 1
    );
    write_text(
        &smoke_path,
        &smoke.replace(&dap_client_target_gate, &wrong_dap_client_target_gate),
    )
    .expect("write corrupt smoke test");
    let err = cmd_verify_build(&out).expect_err("client summary count mismatch");
    assert!(
        err.to_string()
            .contains("deploy smoke test must include orv_smoke_dap_summary_contains"),
        "{err}"
    );
    write_text(&smoke_path, &smoke).expect("restore smoke test");

    write_text(
        &smoke_path,
        &smoke.replace(
            r#""reveal client target summary""#,
            r#""reveal client summary""#,
        ),
    )
    .expect("write corrupt smoke test");
    let err = cmd_verify_build(&out).expect_err("client reveal smoke mismatch");
    assert!(
        err.to_string()
            .contains("deploy smoke test must include orv_smoke_reveal_contains"),
        "{err}"
    );
    write_text(&smoke_path, &smoke).expect("restore smoke test");

    write_text(
        &smoke_path,
        &smoke.replace("ORV_CLIENT_BOOTSTRAP", "ORV_CLIENT_BOOT"),
    )
    .expect("write corrupt smoke test");
    let err = cmd_verify_build(&out).expect_err("client smoke test mismatch");
    assert!(
        err.to_string()
            .contains(r#"deploy smoke test must include orv_smoke_grep "client loader bootstrap""#),
        "{err}"
    );
    let _ = std::fs::remove_dir_all(dir);
    let _ = std::fs::remove_dir_all(out);
}
