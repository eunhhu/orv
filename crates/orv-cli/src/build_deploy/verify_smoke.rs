use super::*;

pub(crate) fn verify_deploy_smoke_test_artifact(
    dir: &Path,
    path: &str,
    listen: Option<&orv_compiler::ServerListenArtifact>,
    artifact: &orv_compiler::ServerRuntimeArtifact,
    origin_map: &orv_compiler::OriginMap,
    persistence: &DeployPersistence,
    client: Option<&serde_json::Value>,
) -> anyhow::Result<()> {
    let smoke_path = dir.join(path);
    if !smoke_path.is_file() {
        anyhow::bail!("missing deploy smoke test: {}", smoke_path.display());
    }
    verify_executable_if_supported(&smoke_path, "deploy smoke test")?;
    verify_shell_syntax_if_supported(&smoke_path, "deploy smoke test")?;
    let smoke = std::fs::read_to_string(&smoke_path)
        .map_err(|e| anyhow::anyhow!("failed to read {}: {e}", smoke_path.display()))?;
    let base_url = deploy_smoke_base_url(listen);
    let base_assignment = format!(r#"BASE_URL="${{ORV_BASE_URL:-{base_url}}}""#);
    if !smoke.contains(&base_assignment) {
        anyhow::bail!("deploy smoke test must include {base_assignment}");
    }
    if !smoke.contains("command -v curl") || !smoke.contains("orv deploy smoke test requires curl")
    {
        anyhow::bail!("deploy smoke test must check curl availability");
    }
    if !smoke.contains(r#"ORV_SMOKE_OUTPUT="${ORV_SMOKE_OUTPUT:-deploy/smoke-output.txt}""#)
        || !smoke.contains(r#"> "$ORV_SMOKE_OUTPUT""#)
        || !smoke.contains("orv_smoke_write_output()")
        || !smoke.contains("\norv_smoke_write_output\n")
        || !smoke.contains("graph_contract=verified")
        || !smoke.contains("dap_summary=verified")
        || !smoke.contains("dap_source_bundle=verified")
        || !smoke.contains("server_routes=")
        || !smoke.contains("trace_stream_requested=%s")
    {
        anyhow::bail!("deploy smoke test must write deploy smoke output artifact");
    }
    if !smoke.contains(r#"ORV_BIN="${ORV_BIN:-orv}""#)
        || !smoke.contains("orv_smoke_reveal_contains()")
        || !smoke.contains("orv_smoke_editor_reveal_contains()")
        || !smoke.contains("orv_smoke_lsp_reveal_contains()")
        || !smoke.contains("orv_smoke_dap_summary_contains()")
        || !smoke.contains("editor reveal")
        || !smoke.contains("lsp reveal")
        || !smoke.contains("editor run-debug . --control next")
        || !smoke.contains("orv deploy smoke test requires orv")
    {
        anyhow::bail!(
            "deploy smoke test must verify source, editor, LSP, and DAP production surfaces with the ORV CLI"
        );
    }
    if !smoke.contains("orv_smoke_trace_stream()")
        || !smoke.contains("ORV_SMOKE_TRACE_STREAM")
        || !smoke.contains("editor trace-stream")
        || !smoke.contains(r#"'"kind":"orv.production.trace.frame"'"#)
        || !smoke.contains(r#"'"index":0'"#)
        || !smoke.contains(r#"'"frame":{'"#)
        || !smoke.contains(r#"'"trace_frame_event_count":'"#)
    {
        anyhow::bail!("deploy smoke test must optionally verify live trace stream");
    }
    let source_bundle_file_count = artifact.source_bundle.files.len();
    let source_bundle_hash = deploy_smoke_source_bundle_hash(dir)?;
    let graph_contract_count = deploy_graph_contract_count(dir)?;
    let project_graph_node_count = deploy_project_graph_node_count(dir)?;
    let origin_entry_count = origin_map.entries.len();
    let dap_graph_contract_summary = format!(
        r#"orv_smoke_dap_summary_contains "dap graph summary" '"graph_contract_count": {graph_contract_count}'"#
    );
    let dap_source_bundle_summary = format!(
        r#"orv_smoke_dap_summary_contains "dap source bundle summary" '"source_bundle_file_count": {source_bundle_file_count}'"#
    );
    let dap_source_bundle_panel_file_count = format!(
        r#"orv_smoke_dap_summary_contains "dap source bundle panel file count" '"fileCount": {source_bundle_file_count}'"#
    );
    let dap_source_bundle_panel_hash =
        deploy_smoke_dap_source_bundle_panel_hash_check(&source_bundle_hash);
    let dap_project_graph_summary = format!(
        r#"orv_smoke_dap_summary_contains "dap project graph summary" '"project_graph_node_count": {project_graph_node_count}'"#
    );
    let dap_origin_summary = format!(
        r#"orv_smoke_dap_summary_contains "dap origin summary" '"origin_entry_count": {origin_entry_count}'"#
    );
    if !smoke.contains("orv_smoke_graph_contract()")
        || !smoke.contains("\norv_smoke_graph_contract\n")
        || !smoke.contains(&dap_graph_contract_summary)
        || !smoke.contains(&dap_source_bundle_summary)
        || !smoke.contains(&dap_project_graph_summary)
        || !smoke.contains(&dap_origin_summary)
        || !smoke.contains(
            r#"orv_smoke_dap_summary_contains "dap source bundle panel" '"source_bundle": {'"#,
        )
        || !smoke.contains(
            r#"orv_smoke_dap_summary_contains "dap source bundle panel path" '"path": "./source-bundle.json"'"#,
        )
        || !smoke.contains(&dap_source_bundle_panel_file_count)
        || !smoke.contains(&dap_source_bundle_panel_hash)
        || !smoke.contains(r#""$ORV_BIN" verify-build ."#)
        || !smoke.contains("source-bundle.json")
        || !smoke.contains("project-graph.json")
        || !smoke.contains("origin-map.json")
    {
        anyhow::bail!("deploy smoke test must verify the build graph contract");
    }
    if !smoke.contains("orv_smoke_dap_summary_capture()")
        || !smoke.contains("orv_smoke_dap_summary_cleanup()")
        || !smoke.contains("\norv_smoke_dap_summary_cleanup\n")
    {
        anyhow::bail!("deploy smoke test must cache and clean DAP production summary output");
    }
    if !smoke.contains(
        r#"orv_smoke_dap_summary_contains "dap smoke required markers" '"smoke_test_required_markers": ['"#,
    ) || !smoke.contains(
        r#"orv_smoke_dap_summary_contains "dap smoke summary required markers" '"required_markers": ['"#,
    ) || !smoke.contains(
        r#"orv_smoke_dap_summary_contains "dap smoke marker dap source bundle" '"dap_source_bundle"'"#,
    ) {
        anyhow::bail!(
            "deploy smoke test must verify smoke marker contract in DAP production context"
        );
    }
    if !smoke.contains("ORV_SMOKE_BUILD_DIR=") || !smoke.contains(r#"cd "$ORV_SMOKE_BUILD_DIR""#) {
        anyhow::bail!("deploy smoke test must run from its build directory");
    }
    if !smoke.contains("orv_smoke_curl()") || !smoke.contains("orv deploy smoke test failed: %s") {
        anyhow::bail!("deploy smoke test must label failed curl steps");
    }
    if !artifact.routes.is_empty()
        && (!smoke.contains("orv_smoke_origin_header()")
            || !smoke.contains("orv_smoke_curl_origin()")
            || !smoke.contains("expected_origin")
            || !smoke.contains("wrong x-orv-origin-id"))
    {
        anyhow::bail!("deploy smoke test must verify exact route origin headers");
    }
    let has_single_response_origin = artifact
        .routes
        .iter()
        .any(|route| deploy_smoke_unique_response_origin(route).is_some());
    if has_single_response_origin
        && (!smoke.contains("orv_smoke_response_origin_header()")
            || !smoke.contains("orv_smoke_curl_origin_response()")
            || !smoke.contains("expected_response_origin")
            || !smoke.contains("wrong x-orv-response-origin-id"))
    {
        anyhow::bail!("deploy smoke test must verify exact response origin headers");
    }
    for route in &artifact.routes {
        let assignment = format!(
            r#"{}="{}""#,
            deploy_smoke_origin_var_name(&route.method, &route.path),
            route.origin_id
        );
        if !smoke.contains(&assignment) {
            let method = &route.method;
            let path = &route.path;
            anyhow::bail!("deploy smoke test must declare expected origin for {method} {path}");
        }
        if let Some(response_origin_id) = deploy_smoke_unique_response_origin(route) {
            let assignment = format!(
                r#"{}="{}""#,
                deploy_smoke_response_origin_var_name(&route.method, &route.path),
                response_origin_id
            );
            if !smoke.contains(&assignment) {
                let method = &route.method;
                let path = &route.path;
                anyhow::bail!(
                    "deploy smoke test must declare expected response origin for {method} {path}"
                );
            }
        }
    }
    if !artifact.routes.is_empty() {
        let native_summary = deploy_native_server_summary_counts(dir)?;
        let dap_native_target_summary = format!(
            r#"orv_smoke_dap_summary_contains "dap native target summary" '"native_server_target_count": {}'"#,
            native_summary.targets
        );
        let dap_native_route_summary = format!(
            r#"orv_smoke_dap_summary_contains "dap native route summary" '"native_server_route_count": {}'"#,
            native_summary.routes
        );
        if !smoke.contains(&dap_native_target_summary) || !smoke.contains(&dap_native_route_summary)
        {
            anyhow::bail!("deploy smoke test must check DAP native production summary counters");
        }
    }
    if !artifact.routes.is_empty()
        && (!smoke.contains(r#"orv_smoke_reveal_contains "reveal smoke required markers" "#)
            || !smoke
                .contains(r#"orv_smoke_reveal_contains "reveal smoke summary required markers" "#)
            || !smoke
                .contains(r#"orv_smoke_reveal_contains "reveal smoke marker dap source bundle" "#)
            || !smoke.contains(
                r#"orv_smoke_editor_reveal_contains "editor reveal smoke required markers" "#,
            )
            || !smoke.contains(
                r#"orv_smoke_editor_reveal_contains "editor reveal smoke summary required markers" "#,
            )
            || !smoke.contains(
                r#"orv_smoke_editor_reveal_contains "editor reveal smoke marker dap source bundle" "#,
            )
            || !smoke.contains(r#"orv_smoke_lsp_reveal_contains "lsp reveal smoke required markers" "#)
            || !smoke.contains(
                r#"orv_smoke_lsp_reveal_contains "lsp reveal smoke summary required markers" "#,
            )
            || !smoke.contains(
                r#"orv_smoke_lsp_reveal_contains "lsp reveal smoke marker dap source bundle" "#,
            ))
    {
        anyhow::bail!("deploy smoke test must verify smoke marker contract across reveal surfaces");
    }
    if deploy_routes_include(artifact, "POST", "/checkout")
        && !smoke.contains("orv_smoke_cookie_from_headers()")
    {
        anyhow::bail!("deploy smoke test must extract cookies for protected shop routes");
    }
    if deploy_routes_include(artifact, "POST", "/checkout")
        && (!smoke.contains("orv_smoke_fetch()") || !smoke.contains("orv_smoke_body_contains()"))
    {
        anyhow::bail!("deploy smoke test must inspect shop response bodies");
    }
    verify_deploy_smoke_client_contract(dir, &smoke, client)?;
    verify_deploy_smoke_db_adapter_contract(&smoke, persistence)?;
    if let Some(ready_path) = deploy_smoke_ready_path(artifact) {
        let ready_assignment = format!(r#"READY_PATH="{ready_path}""#);
        if !smoke.contains(&ready_assignment) {
            anyhow::bail!("deploy smoke test must include {ready_assignment}");
        }
        if !smoke.contains("for attempt in 1 2 3 4 5") || !smoke.contains("sleep 1") {
            anyhow::bail!("deploy smoke test must wait for server readiness");
        }
    }
    for route in artifact.routes.iter().filter(|route| {
        route.method == "GET"
            && !route.path.contains(':')
            && !route.path.starts_with("/admin")
            && route.path != "/account/sessions"
    }) {
        let origin_ref = deploy_smoke_origin_var_ref(&route.method, &route.path);
        let command = if deploy_smoke_unique_response_origin(route).is_some() {
            let response_origin_ref =
                deploy_smoke_response_origin_var_ref(&route.method, &route.path);
            format!(
                r#"orv_smoke_curl_origin_response "GET {}" "{}" "{}" "$BASE_URL{}""#,
                route.path, origin_ref, response_origin_ref, route.path
            )
        } else {
            format!(
                r#"orv_smoke_curl_origin "GET {}" "{}" "$BASE_URL{}""#,
                route.path, origin_ref, route.path
            )
        };
        if !smoke.contains(&command) {
            let method = &route.method;
            let path = &route.path;
            anyhow::bail!("deploy smoke test must cover {method} {path}");
        }
        if deploy_smoke_unique_response_origin(route).is_some() {
            let response_origin_ref =
                deploy_smoke_response_origin_var_ref(&route.method, &route.path);
            for required in [
                format!(
                    r#"orv_smoke_reveal_contains "reveal GET {} response source" "{}" '@respond'"#,
                    route.path, response_origin_ref
                ),
                format!(
                    r#"orv_smoke_reveal_contains "reveal GET {} response production" "{}" '"response_origin_dispatch": true'"#,
                    route.path, response_origin_ref
                ),
                format!(
                    r#"orv_smoke_editor_reveal_contains "editor reveal GET {} response source" "{}" '@respond'"#,
                    route.path, response_origin_ref
                ),
                format!(
                    r#"orv_smoke_editor_reveal_contains "editor reveal GET {} response production" "{}" '"response_origin_dispatch": true'"#,
                    route.path, response_origin_ref
                ),
                format!(
                    r#"orv_smoke_lsp_reveal_contains "lsp reveal GET {} response origin" "{}" '"name": "respond"'"#,
                    route.path, response_origin_ref
                ),
                format!(
                    r#"orv_smoke_lsp_reveal_contains "lsp reveal GET {} response production" "{}" '"response_origin_dispatch": true'"#,
                    route.path, response_origin_ref
                ),
            ] {
                if !smoke.contains(&required) {
                    let method = &route.method;
                    let path = &route.path;
                    anyhow::bail!(
                        "deploy smoke test must reveal response origin for {method} {path}"
                    );
                }
            }
        }
        let summary =
            deploy_route_reveal_summary_counts(dir, &route.origin_id, origin_map, artifact)?;
        for required in deploy_route_reveal_summary_requirements(&route.path, &origin_ref, summary)
        {
            if !smoke.contains(&required) {
                let method = &route.method;
                let path = &route.path;
                anyhow::bail!(
                    "deploy smoke test must verify reveal production summary for {method} {path}"
                );
            }
        }
    }
    if deploy_routes_include(artifact, "POST", "/checkout") {
        for path in ["/products", "/members", "/cart/items"] {
            let origin_ref = deploy_smoke_origin_var_ref("POST", path);
            let command = format!(
                r#"orv_smoke_curl_origin "POST {path}" "{origin_ref}" -X POST "$BASE_URL{path}""#
            );
            if !smoke.contains(&command) {
                anyhow::bail!("deploy smoke test must cover POST {path}");
            }
        }
        let checkout_origin_ref = deploy_smoke_origin_var_ref("POST", "/checkout");
        let checkout_command = format!(
            r#"orv_smoke_fetch_origin "POST /checkout" "$SMOKE_CHECKOUT_BODY" "{checkout_origin_ref}" -X POST "$BASE_URL/checkout""#
        );
        if !smoke.contains(&checkout_command) {
            anyhow::bail!("deploy smoke test must cover POST /checkout with captured body");
        }
        if !smoke.contains(r#"SMOKE_SKU="orv-smoke-sku-${SMOKE_ID}""#) {
            anyhow::bail!("deploy smoke test must use unique smoke SKU");
        }
        if (!persistence.db_paths.is_empty() || !persistence.db_env.is_empty())
            && !smoke.contains(r#"ORV_SMOKE_DB_CONNECT_ORIGIN="ori_"#)
        {
            anyhow::bail!("deploy smoke test must declare a DB connect source origin");
        }
        if deploy_smoke_has_commerce_record(persistence, "payment", "data/payments.jsonl")
            && !smoke.contains(r#"ORV_SMOKE_PAYMENT_CONNECT_ORIGIN="ori_"#)
        {
            anyhow::bail!("deploy smoke test must declare a payment connect source origin");
        }
        if deploy_smoke_has_commerce_record(persistence, "shipping", "data/shipments.jsonl")
            && !smoke.contains(r#"ORV_SMOKE_SHIPPING_CONNECT_ORIGIN="ori_"#)
        {
            anyhow::bail!("deploy smoke test must declare a shipping connect source origin");
        }
        if !smoke.contains(r#"SMOKE_SKU_SECOND="orv-smoke-sku-${SMOKE_ID}-2""#)
            || !smoke.contains(r#"SMOKE_SKU_THIRD="orv-smoke-sku-${SMOKE_ID}-3""#)
        {
            anyhow::bail!("deploy smoke test must create three unique smoke SKUs");
        }
        if !smoke.contains(r#"SMOKE_HANDLE="orv-smoke-${SMOKE_ID}""#) {
            anyhow::bail!("deploy smoke test must use unique smoke member handle");
        }
        if !smoke.contains(
            "CSRF_COOKIE=\"$(orv_smoke_cookie_from_headers orv_csrf \"$SMOKE_HEADERS\")\"",
        ) || !smoke.contains(r#"-H "x-csrf-token: ${CSRF_TOKEN}""#)
        {
            anyhow::bail!("deploy smoke test must send reference CSRF cookie/token");
        }
        if deploy_routes_include(artifact, "GET", "/account/sessions") {
            let origin_ref = deploy_smoke_origin_var_ref("GET", "/account/sessions");
            let command = format!(
                r#"orv_smoke_curl_origin "GET /account/sessions" "{origin_ref}" -H "cookie: ${{MEMBER_SESSION_COOKIE}}" "$BASE_URL/account/sessions""#
            );
            if !smoke.contains(&command) {
                anyhow::bail!(
                    "deploy smoke test must cover GET /account/sessions with a session cookie"
                );
            }
        }
        for required in [
            r#"orv_smoke_body_contains "home title" "$SMOKE_HOME_BODY" 'Miol Shop'"#,
            r#"orv_smoke_body_contains "home copy" "$SMOKE_HOME_BODY" 'Catalog, member signup, payment capture, and shipment booking are ready.'"#,
            r#"orv_smoke_body_contains "home theme surface" "$SMOKE_HOME_BODY" 'background-color: #f8fafc'"#,
            r#"orv_smoke_body_contains "home theme typography" "$SMOKE_HOME_BODY" 'font-family: Inter, system-ui, sans-serif'"#,
            r#"orv_smoke_reveal_contains "reveal GET / source" "$ORV_SMOKE_ORIGIN_GET_ROOT" '@route GET /'"#,
            r#"orv_smoke_reveal_contains "reveal GET / production" "$ORV_SMOKE_ORIGIN_GET_ROOT" '"path": "/"'"#,
            r#"orv_smoke_editor_reveal_contains "editor reveal GET / source" "$ORV_SMOKE_ORIGIN_GET_ROOT" '@route GET /'"#,
            r#"orv_smoke_editor_reveal_contains "editor reveal GET / production" "$ORV_SMOKE_ORIGIN_GET_ROOT" '"path": "/"'"#,
            r#"orv_smoke_lsp_reveal_contains "lsp reveal GET / origin" "$ORV_SMOKE_ORIGIN_GET_ROOT" '"name": "GET /"'"#,
            r#"orv_smoke_lsp_reveal_contains "lsp reveal GET / production" "$ORV_SMOKE_ORIGIN_GET_ROOT" '"path": "/"'"#,
            r#"orv_smoke_body_contains "catalog smoke product" "$SMOKE_CATALOG_BODY" "$SMOKE_SKU""#,
            r#"orv_smoke_body_contains "catalog second smoke product" "$SMOKE_CATALOG_BODY" "$SMOKE_SKU_SECOND""#,
            r#"orv_smoke_body_contains "catalog third smoke product" "$SMOKE_CATALOG_BODY" "$SMOKE_SKU_THIRD""#,
            r#"orv_smoke_body_contains "cart smoke item" "$SMOKE_CART_BODY" "$SMOKE_SKU""#,
            r#"orv_smoke_body_contains "account smoke session" "$SMOKE_ACCOUNT_BODY" "$SMOKE_HANDLE""#,
            r#"orv_smoke_body_contains "checkout shipped order" "$SMOKE_CHECKOUT_BODY" '"status":"shipped"'"#,
            r#"orv_smoke_body_contains "checkout captured payment" "$SMOKE_CHECKOUT_BODY" '"status":"captured"'"#,
            r#"orv_smoke_body_contains "checkout shipment tracking" "$SMOKE_CHECKOUT_BODY" 'TRK-LOCAL'"#,
            r#"orv_smoke_body_contains "admin catalog smoke product" "$SMOKE_ADMIN_CATALOG_BODY" "$SMOKE_SKU""#,
            r#"orv_smoke_body_contains "admin catalog second smoke product" "$SMOKE_ADMIN_CATALOG_BODY" "$SMOKE_SKU_SECOND""#,
            r#"orv_smoke_body_contains "admin catalog third smoke product" "$SMOKE_ADMIN_CATALOG_BODY" "$SMOKE_SKU_THIRD""#,
            r#"orv_smoke_body_contains "admin orders shipped" "$SMOKE_ADMIN_ORDERS_BODY" 'shipped'"#,
            r#"orv_smoke_body_contains "admin payments captured" "$SMOKE_ADMIN_PAYMENTS_BODY" 'captured'"#,
            r#"orv_smoke_body_contains "admin shipments tracking" "$SMOKE_ADMIN_SHIPMENTS_BODY" 'TRK-LOCAL'"#,
            r#"orv_smoke_body_contains "admin audit checkout" "$SMOKE_ADMIN_AUDIT_BODY" 'checkout.complete'"#,
        ] {
            if !smoke.contains(required) {
                anyhow::bail!("deploy smoke test must include {required}");
            }
        }
        if !persistence.db_paths.is_empty() || !persistence.db_env.is_empty() {
            for required in [
                r#"orv_smoke_reveal_contains "reveal DB source" "$ORV_SMOKE_DB_CONNECT_ORIGIN" '@db.connect'"#,
                r#"orv_smoke_reveal_contains "reveal DB preflight" "$ORV_SMOKE_DB_CONNECT_ORIGIN" '"preflight"'"#,
                r#"orv_smoke_reveal_contains "reveal DB smoke summary" "$ORV_SMOKE_DB_CONNECT_ORIGIN" '"smoke_test_summary"'"#,
                r#"orv_smoke_reveal_contains "reveal DB smoke summary count" "$ORV_SMOKE_DB_CONNECT_ORIGIN" '"preflight_smoke_summary_missing_count"'"#,
                r#"orv_smoke_reveal_contains "reveal DB sqlite path" "$ORV_SMOKE_DB_CONNECT_ORIGIN" 'sqlite://data/shop.sqlite'"#,
                r#"orv_smoke_editor_reveal_contains "editor reveal DB source" "$ORV_SMOKE_DB_CONNECT_ORIGIN" '@db.connect'"#,
                r#"orv_smoke_editor_reveal_contains "editor reveal DB preflight" "$ORV_SMOKE_DB_CONNECT_ORIGIN" '"preflight"'"#,
                r#"orv_smoke_editor_reveal_contains "editor reveal DB smoke summary" "$ORV_SMOKE_DB_CONNECT_ORIGIN" '"smoke_test_summary"'"#,
                r#"orv_smoke_editor_reveal_contains "editor reveal DB smoke summary count" "$ORV_SMOKE_DB_CONNECT_ORIGIN" '"preflight_smoke_summary_missing_count"'"#,
                r#"orv_smoke_lsp_reveal_contains "lsp reveal DB origin" "$ORV_SMOKE_DB_CONNECT_ORIGIN" '@db.connect'"#,
                r#"orv_smoke_lsp_reveal_contains "lsp reveal DB preflight" "$ORV_SMOKE_DB_CONNECT_ORIGIN" '"preflight"'"#,
                r#"orv_smoke_lsp_reveal_contains "lsp reveal DB smoke summary" "$ORV_SMOKE_DB_CONNECT_ORIGIN" '"smoke_test_summary"'"#,
                r#"orv_smoke_lsp_reveal_contains "lsp reveal DB smoke summary count" "$ORV_SMOKE_DB_CONNECT_ORIGIN" '"preflight_smoke_summary_missing_count"'"#,
            ] {
                if !smoke.contains(required) {
                    anyhow::bail!("deploy smoke test must include {required}");
                }
            }
        }
        if deploy_smoke_has_commerce_record(persistence, "payment", "data/payments.jsonl") {
            for required in [
                r#"orv_smoke_reveal_contains "reveal payment source" "$ORV_SMOKE_PAYMENT_CONNECT_ORIGIN" '@payment.connect'"#,
                r#"orv_smoke_reveal_contains "reveal payment match" "$ORV_SMOKE_PAYMENT_CONNECT_ORIGIN" '"matched": true'"#,
                r#"orv_smoke_reveal_contains "reveal payment record path" "$ORV_SMOKE_PAYMENT_CONNECT_ORIGIN" 'file://data/payments.jsonl'"#,
                r#"orv_smoke_reveal_contains "reveal payment request kind" "$ORV_SMOKE_PAYMENT_CONNECT_ORIGIN" 'payment.capture'"#,
                r#"orv_smoke_editor_reveal_contains "editor reveal payment source" "$ORV_SMOKE_PAYMENT_CONNECT_ORIGIN" '@payment.connect'"#,
                r#"orv_smoke_editor_reveal_contains "editor reveal payment match" "$ORV_SMOKE_PAYMENT_CONNECT_ORIGIN" '"matched": true'"#,
                r#"orv_smoke_lsp_reveal_contains "lsp reveal payment origin" "$ORV_SMOKE_PAYMENT_CONNECT_ORIGIN" '@payment.connect'"#,
                r#"orv_smoke_lsp_reveal_contains "lsp reveal payment match" "$ORV_SMOKE_PAYMENT_CONNECT_ORIGIN" '"matched": true'"#,
            ] {
                if !smoke.contains(required) {
                    anyhow::bail!("deploy smoke test must include {required}");
                }
            }
        }
        if deploy_smoke_has_commerce_record(persistence, "shipping", "data/shipments.jsonl") {
            for required in [
                r#"orv_smoke_reveal_contains "reveal shipping source" "$ORV_SMOKE_SHIPPING_CONNECT_ORIGIN" '@shipping.connect'"#,
                r#"orv_smoke_reveal_contains "reveal shipping match" "$ORV_SMOKE_SHIPPING_CONNECT_ORIGIN" '"matched": true'"#,
                r#"orv_smoke_reveal_contains "reveal shipping record path" "$ORV_SMOKE_SHIPPING_CONNECT_ORIGIN" 'file://data/shipments.jsonl'"#,
                r#"orv_smoke_reveal_contains "reveal shipping request kind" "$ORV_SMOKE_SHIPPING_CONNECT_ORIGIN" 'shipping.booking'"#,
                r#"orv_smoke_editor_reveal_contains "editor reveal shipping source" "$ORV_SMOKE_SHIPPING_CONNECT_ORIGIN" '@shipping.connect'"#,
                r#"orv_smoke_editor_reveal_contains "editor reveal shipping match" "$ORV_SMOKE_SHIPPING_CONNECT_ORIGIN" '"matched": true'"#,
                r#"orv_smoke_lsp_reveal_contains "lsp reveal shipping origin" "$ORV_SMOKE_SHIPPING_CONNECT_ORIGIN" '@shipping.connect'"#,
                r#"orv_smoke_lsp_reveal_contains "lsp reveal shipping match" "$ORV_SMOKE_SHIPPING_CONNECT_ORIGIN" '"matched": true'"#,
            ] {
                if !smoke.contains(required) {
                    anyhow::bail!("deploy smoke test must include {required}");
                }
            }
        }
        for route in artifact.routes.iter().filter(|route| {
            route.method == "GET" && !route.path.contains(':') && route.path.starts_with("/admin")
        }) {
            let origin_ref = deploy_smoke_origin_var_ref(&route.method, &route.path);
            let command = format!(
                r#"orv_smoke_curl_origin "GET {}" "{}" -H "cookie: ${{ADMIN_SESSION_COOKIE}}; ${{ADMIN_ROLE_COOKIE}}" "$BASE_URL{}""#,
                route.path, origin_ref, route.path
            );
            if !smoke.contains(&command) {
                let path = &route.path;
                anyhow::bail!("deploy smoke test must cover GET {path} with an admin role cookie");
            }
        }
    }
    Ok(())
}

pub(crate) fn verify_deploy_smoke_db_adapter_contract(
    smoke: &str,
    persistence: &DeployPersistence,
) -> anyhow::Result<()> {
    if persistence.db_adapters.is_empty() {
        return Ok(());
    }
    if !smoke.contains(r#"orv_smoke_file "deploy/db-adapters.json""#)
        || !smoke.contains(
            r#"orv_smoke_grep "db adapter bridge contract" "deploy/db-adapters.json" '"contract": "http-json-v1"'"#,
        )
        || !smoke.contains("orv_smoke_db_bridge_schema()")
    {
        anyhow::bail!("deploy smoke test must check DB adapter bridge contract");
    }
    for adapter in &persistence.db_adapters {
        let Some(endpoint_env) = adapter
            .bridge_env
            .iter()
            .find(|env| env.purpose == "bridge_endpoint")
        else {
            continue;
        };
        let Some(endpoint) = &adapter.endpoint else {
            continue;
        };
        let auth_env = adapter
            .bridge_env
            .iter()
            .find(|env| env.purpose == "bridge_auth_token")
            .map(|env| env.env.as_str())
            .unwrap_or("");
        let endpoint_expr = format!("${{{}:-${{ORV_DB_ADAPTER_ENDPOINT:-}}}}", endpoint_env.env);
        let auth_expr = format!("${{{auth_env}:-${{ORV_DB_ADAPTER_AUTH_TOKEN:-}}}}");
        let command = format!(
            r#"orv_smoke_db_bridge_schema "{} bridge" "{}" "{}" "{}" "{}""#,
            adapter.provider, endpoint_expr, adapter.provider, endpoint, auth_expr
        );
        if !smoke.contains(&command) {
            let provider = &adapter.provider;
            anyhow::bail!("deploy smoke test must probe DB bridge endpoint for {provider}");
        }
    }
    Ok(())
}

pub(crate) fn verify_deploy_smoke_output_contract_keys(
    value: &serde_json::Value,
    context: &str,
) -> anyhow::Result<()> {
    verify_json_object_keys_exact(value, &["output", "required_markers"], context)
}
