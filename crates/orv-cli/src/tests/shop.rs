use super::*;

#[test]
fn init_accepts_shop_template_flag() {
    let parsed = Cli::try_parse_from(["orv", "init", "target/new-shop", "--template", "shop"]);
    if let Err(err) = parsed {
        panic!("{}", err.render());
    }
}

#[test]
fn init_shop_template_scaffolds_shopping_routes() {
    let dir = temp_output_dir("init-shop-template");

    cmd_init(&dir, Some("starter-shop"), InitTemplate::Shop).expect("init shop project");

    let entry = dir.join("src").join("main.orv");
    let source = std::fs::read_to_string(&entry).expect("entry source");
    assert!(source.contains("@listen 8080"));
    assert!(source.contains(
        r#"let shopdb = @db.connect(@env.SHOP_DATABASE_URL ?? "sqlite://data/shop.sqlite")"#
    ));
    assert!(source.contains("@design"));
    assert!(source.contains("@colors"));
    assert!(source.contains(r##"primary: "#315c5a""##));
    assert!(source.contains("@spacing"));
    assert!(source.contains("@typography"));
    assert!(source.contains("@design.colors.surface"));
    assert!(source.contains("@design.spacing.lg"));
    assert!(source.contains("@design.typography.fontFamily"));
    assert!(source.contains("@route GET / {\n"));
    assert!(source.contains("@serve @html"));
    assert!(source.contains("@a href=\"/catalog\" \"Shop catalog\""));
    assert!(source.contains("@route GET /catalog"));
    assert!(source.contains("Shop Catalog"));
    assert!(source.contains("@a href=\"/cart\" \"Cart\""));
    assert!(source.contains("@form action=\"/cart/items\" method=post"));
    assert!(source.contains("@route GET /cart"));
    assert!(source.contains("@route POST /cart/items"));
    assert!(source.contains("@a href=\"/account/sessions\" \"My sessions\""));
    assert!(source.contains("@route GET /account/sessions"));
    assert!(source.contains("Account Sessions"));
    assert!(source.contains("@a href=\"/admin\" \"Admin dashboard\""));
    assert!(source.contains("@route GET /admin"));
    assert!(source.contains("@Auth required role=\"admin\""));
    assert!(source.matches("@Auth required role=\"admin\"").count() >= 8);
    assert!(source.contains(r#"handle: "admin""#));
    assert!(source.contains(r#"email: "admin@example.test""#));
    assert!(source.contains("Operations dashboard"));
    assert!(source.contains("@a href=\"/admin/summary\" \"Operations summary\""));
    assert!(source.contains("@route GET /admin/summary"));
    assert!(source.contains("@a href=\"/admin/catalog\" \"Catalog read model\""));
    assert!(source.contains("@route GET /admin/catalog"));
    assert!(source.contains("@a href=\"/admin/orders\" \"Order read model\""));
    assert!(source.contains("@route GET /admin/orders"));
    assert!(source.contains("@a href=\"/admin/payments\" \"Payment read model\""));
    assert!(source.contains("@route GET /admin/payments"));
    assert!(source.contains("@a href=\"/admin/shipments\" \"Shipment read model\""));
    assert!(source.contains("@route GET /admin/shipments"));
    assert!(source.contains("@a href=\"/admin/webhooks\" \"Webhook read model\""));
    assert!(source.contains("@route GET /admin/webhooks"));
    assert!(source.contains("@a href=\"/admin/audit\" \"Audit read model\""));
    assert!(source.contains("@route GET /admin/audit"));
    assert!(source.contains(r#"shopdb.count("Product", {})"#));
    assert!(source.contains(r#"shopdb.count("WebhookEvent", {})"#));
    assert!(source.contains(r#"shopdb.count("AuditEvent", {})"#));
    assert!(source.contains(r#"shopdb.findAll("Order", {})"#));
    assert!(source.contains(r#"shopdb.findAll("Payment", {})"#));
    assert!(source.contains(r#"shopdb.findAll("Shipment", {})"#));
    assert!(source.contains(r#"shopdb.findAll("WebhookEvent", {})"#));
    assert!(source.contains(r#"shopdb.findAll("AuditEvent", {})"#));
    assert!(source.contains("@form action=\"/products\" method=post"));
    assert!(source.contains("badge: string(trim, min=1)"));
    assert!(source.contains("@input type=text name=badge value=\"New arrival\" required"));
    assert!(source.contains("badge: @body.badge"));
    assert!(source.contains("{product.badge}"));
    assert!(source.contains("@input type=number name=stock required"));
    assert!(source.contains("@form action=\"/checkout\" method=post"));
    assert!(source.contains("@input type=password name=password required"));
    assert!(source.contains("@input type=hidden name=_csrf value=\"orv-reference-csrf\""));
    assert!(source.matches("@csrf").count() >= 8);
    assert!(source.contains("struct ProductInput"));
    assert!(source.contains("struct CheckoutInput"));
    assert!(source.contains("@body: ProductInput"));
    assert!(source.contains("@body: MemberSignupInput"));
    assert!(source.contains("@body: MemberLoginInput"));
    assert!(source.contains("@body: CartItemInput"));
    assert!(source.contains("@body: OrderInput"));
    assert!(source.contains("@body: CheckoutInput"));
    assert!(source.contains("@body: PaymentInput"));
    assert!(source.contains("@body: ShipmentInput"));
    assert!(source.contains("@route POST /checkout"));
    assert!(source.contains("One-step checkout"));
    assert!(source.contains("@route POST /members"));
    assert!(source.contains(r#"role: "member""#));
    assert!(source.contains("hash.password(@body.password)"));
    assert!(source.contains("hash.verify(@body.password, member.passwordHash)"));
    assert!(source.contains("admin-reference-password"));
    assert!(source.contains("passwordHash: passwordHash"));
    assert!(source.contains("@form action=\"/members/login\" method=post"));
    assert!(source.contains("@route POST /members/login"));
    assert!(source.contains(r#"shopdb.create("Session""#));
    assert!(source.contains(r#"role: member.role ?? "member""#));
    assert!(source.contains("@route POST /payments"));
    assert!(source.contains("@route POST /webhooks/stripe"));
    assert!(source.contains(r#"@header["stripe-signature"]"#));
    assert!(source.contains("payments.verifyWebhook"));
    assert!(source.contains("let eventId = @body.id"));
    assert!(source.contains(r#"shopdb.find("WebhookEvent""#));
    assert!(source.contains("duplicate: true"));
    assert!(source.contains("let mut reconciledPayment = void"));
    assert!(source.contains(r#"let reconcileOrderId = @body["orderId"]"#));
    assert!(source.contains(r#"let reconcilePaymentStatus = @body["paymentStatus"]"#));
    assert!(source.contains(r#"let reconcileOrderStatus = @body["orderStatus"]"#));
    assert!(source.contains(r#"shopdb.update("Payment", { orderId: reconciledOrderId }"#));
    assert!(source.contains(r#"shopdb.update("Order", { id: reconciledOrderId }"#));
    assert!(source.contains("reconciledPayment: reconciledPayment"));
    assert!(source.contains(r#"shopdb.create("WebhookEvent""#));
    assert!(source.contains(r#"shopdb.create("AuditEvent""#));
    assert!(source.contains("checkout.complete"));
    assert!(source.contains("payment.capture"));
    assert!(source.contains("shipment.book"));
    assert!(source.contains("webhook.received"));
    assert!(source.contains("@route POST /shipments"));
    assert!(source
        .contains(r#"@payment.connect(@env.PAYMENT_ADAPTER_URL ?? "file://data/payments.jsonl")"#));
    assert!(source.contains(
        r#"@shipping.connect(@env.SHIPPING_ADAPTER_URL ?? "file://data/shipments.jsonl")"#
    ));
    cmd_check(&dir).expect("check shop project");
    let out = dir.join("dist");
    cmd_build_with_profile(&dir, &out, BuildProfile::Production).expect("build shop project");
    assert!(out.join("server").join("app.orv-runtime.json").is_file());
    assert!(out.join("deploy").join("manifest.json").is_file());
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn init_shop_template_prod_artifacts_keep_full_service_routes() {
    let dir = temp_output_dir("init-shop-prod-routes");

    cmd_init(&dir, Some("starter-shop"), InitTemplate::Shop).expect("init shop project");
    let out = dir.join("dist");
    cmd_build_with_profile(&dir, &out, BuildProfile::Production).expect("build shop project");

    let manifest = read_json_value(&out.join("build-manifest.json")).expect("manifest");
    let deploy = read_json_value(&out.join("deploy").join("manifest.json")).expect("deploy");
    let runtime =
        read_json_value(&out.join("server").join("app.orv-runtime.json")).expect("runtime");
    let container = read_json_value(&out.join("deploy").join("container.json")).expect("container");
    let compose =
        std::fs::read_to_string(out.join("deploy").join("compose.yaml")).expect("compose");
    let env_example =
        std::fs::read_to_string(out.join("deploy").join("env.example")).expect("env example");
    let commerce_adapters = read_json_value(&out.join("deploy").join("commerce-adapters.json"))
        .expect("commerce adapters");
    let preflight = read_json_value(&out.join("deploy").join("preflight.json")).expect("preflight");
    let benchmark_evidence = read_json_value(&out.join("deploy").join("benchmark-evidence.json"))
        .expect("benchmark evidence");
    let smoke_test =
        std::fs::read_to_string(out.join("deploy").join("smoke-test.sh")).expect("smoke test");
    let native_routes =
        std::fs::read_to_string(out.join("server").join("native").join("routes.rs"))
            .expect("native routes source");
    for (method, path) in [
        ("GET", "/"),
        ("GET", "/catalog"),
        ("GET", "/cart"),
        ("GET", "/account/sessions"),
        ("GET", "/admin"),
        ("GET", "/admin/catalog"),
        ("GET", "/admin/summary"),
        ("GET", "/admin/orders"),
        ("GET", "/admin/payments"),
        ("GET", "/admin/shipments"),
        ("GET", "/admin/webhooks"),
        ("GET", "/admin/audit"),
        ("GET", "/products/:sku"),
        ("GET", "/members/:handle"),
        ("GET", "/orders/:customer"),
        ("POST", "/checkout"),
        ("POST", "/cart/items"),
        ("POST", "/members"),
        ("POST", "/members/login"),
        ("POST", "/payments"),
        ("POST", "/webhooks/stripe"),
        ("POST", "/shipments"),
        ("GET", "/shipments/:orderId"),
    ] {
        assert!(json_routes_include(&runtime["routes"], method, path));
        assert!(json_routes_include(
            &deploy["server"]["routes"],
            method,
            path
        ));
        assert!(native_routes_source_includes(&native_routes, method, path));
    }
    for feature in [
        "auth_roles",
        "csrf_protection",
        "payment_adapter",
        "rate_limit",
        "session_cookies",
        "shipping_adapter",
    ] {
        assert!(manifest["capabilities"]["runtime_features"]
            .as_array()
            .expect("manifest runtime features")
            .iter()
            .any(|item| item == feature));
        assert!(runtime["runtime_features"]
            .as_array()
            .expect("runtime features")
            .iter()
            .any(|item| item == feature));
        assert!(deploy["server"]["runtime_features"]
            .as_array()
            .expect("deploy runtime features")
            .iter()
            .any(|item| item == feature));
    }
    let admin_route = json_route(&runtime["routes"], "GET", "/admin").expect("admin route");
    assert!(admin_route["policies"]
        .as_array()
        .expect("admin policies")
        .iter()
        .any(|policy| policy["kind"] == "auth"
            && policy["surface"] == "first_party_compiler_plugin"
            && policy["role"] == "admin"
            && policy["required"] == true
            && policy["origin_id"]
                .as_str()
                .is_some_and(|origin_id| origin_id.starts_with("ori_"))));
    let account_sessions_route =
        json_route(&runtime["routes"], "GET", "/account/sessions").expect("sessions route");
    assert!(account_sessions_route["policies"]
        .as_array()
        .expect("session policies")
        .iter()
        .any(|policy| policy["kind"] == "session"
            && policy["surface"] == "first_party_compiler_plugin"
            && policy["required"] == true
            && policy["origin_id"]
                .as_str()
                .is_some_and(|origin_id| origin_id.starts_with("ori_"))));
    let checkout_route =
        json_route(&preflight["routes"], "POST", "/checkout").expect("checkout route");
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
    assert_eq!(
        deploy["server"]["native_routes_source"],
        serde_json::json!("server/native/routes.rs")
    );
    assert_eq!(
        deploy["server"]["native_router_source"],
        serde_json::json!("server/native/router.rs")
    );
    assert_eq!(
        deploy["server"]["native_handlers_source"],
        serde_json::json!("server/native/handlers.rs")
    );
    assert!(native_routes.contains("pub fn orv_native_match_route("));
    assert!(native_routes.contains("pub struct OrvNativeRouteMatch"));
    assert!(native_routes.contains("pub struct OrvNativeParam"));
    assert!(native_routes.contains("pub struct OrvNativeRoutePolicy"));
    assert!(native_routes.contains("surface: Some(\"first_party_compiler_plugin\")"));
    assert!(native_routes.contains("surface: Some(\"shop_template\")"));
    assert!(native_routes.contains("surface: Some(\"provider_package_template\")"));
    assert!(native_routes.contains("pub policies: &'static [OrvNativeRoutePolicy]"));
    assert!(native_routes.contains("kind: \"auth\""));
    assert!(native_routes.contains("role: Some(\"admin\")"));
    assert!(native_routes.contains("kind: \"csrf\""));
    assert!(native_routes.contains("kind: \"rate_limit\""));
    assert!(native_routes.contains("limit: Some(10)"));
    assert!(native_routes.contains("window_seconds: Some(60)"));
    assert!(native_routes.contains("orv_native_route_path_params(route.path, path)"));
    assert!(native_routes.contains("orv_native_match_route_segment(pattern_segment"));
    assert!(native_routes.contains("fn orv_native_route_param_segment(segment: &str)"));
    assert_eq!(
        deploy["server"]["persistence"]["db_paths"][0],
        serde_json::json!("data/shop.sqlite")
    );
    assert_eq!(
        deploy["server"]["persistence"]["db_env"],
        serde_json::json!([
            {
                "env": "SHOP_DATABASE_URL",
                "default": "sqlite://data/shop.sqlite"
            }
        ])
    );
    assert_eq!(
        deploy["server"]["persistence"]["record_paths"],
        serde_json::json!(["data/payments.jsonl", "data/shipments.jsonl"])
    );
    assert_eq!(
        deploy["server"]["commerce_adapters"],
        serde_json::json!("deploy/commerce-adapters.json")
    );
    assert_eq!(
        deploy["server"]["smoke_test"],
        serde_json::json!("deploy/smoke-test.sh")
    );
    assert_eq!(
        deploy["server"]["smoke_output"],
        serde_json::json!("deploy/smoke-output.txt")
    );
    assert_eq!(
        deploy["server"]["preflight"],
        serde_json::json!("deploy/preflight.json")
    );
    assert_eq!(
        deploy["server"]["benchmark_evidence"],
        serde_json::json!("deploy/benchmark-evidence.json")
    );
    assert_eq!(
        deploy["server"]["persistence"]["commerce_env"],
        serde_json::json!([
            {
                "env": "PAYMENT_ADAPTER_URL",
                "default": "file://data/payments.jsonl"
            },
            {
                "env": "SHIPPING_ADAPTER_URL",
                "default": "file://data/shipments.jsonl"
            }
        ])
    );
    assert_eq!(
        adapter_values_without_source_origin_ids(&commerce_adapters["adapters"]),
        serde_json::json!([
            {
                "kind": "payment",
                "surface": "library_provider_package",
                "package": "orv-commerce",
                "provider_package": null,
                "mode": "file",
                "env": "PAYMENT_ADAPTER_URL",
                "default": "file://data/payments.jsonl",
                "endpoint": null,
                "record_path": "data/payments.jsonl",
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
                "kind": "payment",
                "surface": "library_provider_package",
                "package": "orv-commerce",
                "provider_package": "orv-stripe",
                "mode": "provider",
                "env": null,
                "default": null,
                "endpoint": null,
                "record_path": null,
                "request": {
                    "method": "POST",
                    "content_type": "application/json",
                    "kind": "payment.capture",
                    "body": {
                        "kind": "payment.capture",
                        "payload": "payment capture payload"
                    }
                },
                "provider": "stripe",
                "provider_env": [
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
                ]
            },
            {
                "kind": "shipping",
                "surface": "library_provider_package",
                "package": "orv-commerce",
                "provider_package": null,
                "mode": "file",
                "env": "SHIPPING_ADAPTER_URL",
                "default": "file://data/shipments.jsonl",
                "endpoint": null,
                "record_path": "data/shipments.jsonl",
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
    assert_eq!(
        container["persistence"]["volumes"][0]["host"],
        serde_json::json!("data")
    );
    assert_eq!(
        container["persistence"]["volumes"][0]["container"],
        serde_json::json!("/app/data")
    );
    assert!(compose.contains("../data:/app/data"));
    assert!(
        compose.contains(r#"SHOP_DATABASE_URL: "${SHOP_DATABASE_URL:-sqlite://data/shop.sqlite}""#)
    );
    assert!(compose
        .contains(r#"PAYMENT_ADAPTER_URL: "${PAYMENT_ADAPTER_URL:-file://data/payments.jsonl}""#));
    assert!(compose.contains(
        r#"SHIPPING_ADAPTER_URL: "${SHIPPING_ADAPTER_URL:-file://data/shipments.jsonl}""#
    ));
    assert!(env_example.contains("PORT=8080"));
    assert!(env_example.contains("SHOP_DATABASE_URL=sqlite://data/shop.sqlite"));
    assert!(env_example.contains("PAYMENT_ADAPTER_URL=file://data/payments.jsonl"));
    assert!(env_example.contains("SHIPPING_ADAPTER_URL=file://data/shipments.jsonl"));
    assert!(env_example.contains("STRIPE_WEBHOOK_SECRET="));
    assert!(env_example.contains("STRIPE_WEBHOOK_SECRET_PREVIOUS="));
    assert_eq!(preflight["schema_version"], serde_json::json!(1));
    assert_eq!(preflight["kind"], serde_json::json!("orv.deploy.preflight"));
    assert_eq!(
        preflight["commands"]["verify_build"],
        serde_json::json!("orv verify-build .")
    );
    assert_eq!(
        preflight["commands"]["env_check"],
        serde_json::json!("orv deploy-env-check .")
    );
    assert_eq!(
        preflight["commands"]["smoke_test"],
        serde_json::json!("./deploy/smoke-test.sh")
    );
    assert_eq!(
        preflight["commands"]["editor_run_debug"],
        serde_json::json!("orv editor run-debug . --control next")
    );
    assert_eq!(
        preflight["commands"]["benchmark_prepare"],
        serde_json::json!("orv benchmark-prepare . --participants 2")
    );
    assert_eq!(
        preflight["commands"]["benchmark_report"],
        serde_json::json!("orv benchmark-report .")
    );
    assert_eq!(
        preflight["commands"]["benchmark_report_require_pass"],
        serde_json::json!("orv benchmark-report . --require-pass")
    );
    assert_eq!(
        preflight["commands"]["trace_run_build"],
        serde_json::json!("orv run-build . --trace deploy/request-trace.json")
    );
    assert_eq!(
        preflight["commands"]["trace_stream_smoke"],
        serde_json::json!("ORV_SMOKE_TRACE_STREAM=1 ./deploy/smoke-test.sh")
    );
    assert_eq!(
        preflight["artifacts"]["commerce_adapters"],
        serde_json::json!("deploy/commerce-adapters.json")
    );
    assert_eq!(
        preflight["artifacts"]["source_bundle"],
        serde_json::json!(SOURCE_BUNDLE_PATH)
    );
    assert_eq!(
        preflight["artifacts"]["project_graph"],
        serde_json::json!("project-graph.json")
    );
    assert_eq!(
        preflight["artifacts"]["origin_map"],
        serde_json::json!("origin-map.json")
    );
    assert_eq!(
        preflight["artifacts"]["build_manifest"],
        serde_json::json!("build-manifest.json")
    );
    assert_eq!(
        preflight["artifacts"]["bundle_plan"],
        serde_json::json!("bundle-plan.json")
    );
    assert_eq!(
        preflight["security_features"],
        serde_json::json!([
            "auth_roles",
            "csrf_protection",
            "rate_limit",
            "session_cookies"
        ])
    );
    assert_eq!(preflight["benchmark"]["kind"], "orv.benchmark.shop_5h");
    assert_eq!(preflight["benchmark"]["max_elapsed_minutes"], 300);
    assert_eq!(
        preflight["artifacts"]["benchmark_evidence"],
        serde_json::json!("deploy/benchmark-evidence.json")
    );
    assert_eq!(
        preflight["artifacts"]["smoke_output"],
        serde_json::json!("deploy/smoke-output.txt")
    );
    assert!(preflight["benchmark"]["success_criteria"]
        .as_array()
        .expect("benchmark success criteria")
        .iter()
        .any(|criterion| criterion
            .as_str()
            .is_some_and(|value| value.contains("complete checkout"))));
    assert!(preflight["benchmark"]["data_to_record"]
        .as_array()
        .expect("benchmark data")
        .iter()
        .any(|item| item == "smoke-test output"));
    assert_eq!(
        benchmark_evidence["kind"],
        serde_json::json!("orv.benchmark.shop_5h.evidence")
    );
    assert_eq!(benchmark_evidence["benchmark"], preflight["benchmark"]);
    assert_eq!(benchmark_evidence["commands"], preflight["commands"]);
    assert_eq!(benchmark_evidence["artifacts"], preflight["artifacts"]);
    assert_eq!(
        benchmark_evidence["task_entries"]
            .as_array()
            .expect("benchmark evidence task entries")
            .len(),
        10
    );
    assert_eq!(
        benchmark_evidence["data"]["smoke_test_output"],
        serde_json::Value::Null
    );
    assert_eq!(
        benchmark_evidence["data"]["smoke_test_required_markers"],
        serde_json::json!(deploy_benchmark::SMOKE_REQUIRED_MARKERS)
    );
    assert!(preflight["optional_env"]
        .as_array()
        .expect("optional preflight env")
        .iter()
        .any(|env| env["env"] == "SHOP_DATABASE_URL"
            && env["default"] == "sqlite://data/shop.sqlite"));
    assert!(preflight["optional_env"]
        .as_array()
        .expect("optional preflight env")
        .iter()
        .any(|env| env["env"] == "STRIPE_WEBHOOK_SECRET"
            && env["provider"] == "stripe"
            && env["required"] == false));
    assert!(smoke_test.contains(r#"BASE_URL="${ORV_BASE_URL:-http://127.0.0.1:8080}""#));
    assert!(smoke_test.contains(r#"ORV_BIN="${ORV_BIN:-orv}""#));
    assert!(smoke_test.contains("command -v curl"));
    assert!(smoke_test.contains("orv deploy smoke test requires curl"));
    assert!(smoke_test.contains("orv deploy smoke test requires orv"));
    assert!(smoke_test.contains("orv_smoke_reveal_contains()"));
    assert!(smoke_test.contains("orv_smoke_editor_reveal_contains()"));
    assert!(smoke_test.contains("orv_smoke_lsp_reveal_contains()"));
    assert!(smoke_test.contains("orv_smoke_dap_summary_contains()"));
    assert!(smoke_test.contains("lsp reveal"));
    assert!(smoke_test.contains("editor run-debug . --control next"));
    assert!(smoke_test.contains("orv_smoke_trace_stream()"));
    assert!(smoke_test.contains("ORV_SMOKE_TRACE_STREAM"));
    assert!(smoke_test.contains("editor trace-stream"));
    assert!(smoke_test.contains("orv deploy smoke test failed: live trace stream"));
    assert!(smoke_test.contains("orv_smoke_graph_contract()"));
    assert!(smoke_test.contains("\norv_smoke_graph_contract\n"));
    assert!(smoke_test.contains(r#""$ORV_BIN" verify-build ."#));
    assert!(smoke_test.contains("source-bundle.json"));
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
    assert!(smoke_test.contains("project-graph.json"));
    assert!(smoke_test.contains("origin-map.json"));
    assert!(smoke_test.contains("orv_smoke_curl()"));
    assert!(smoke_test.contains("orv_smoke_origin_header()"));
    assert!(smoke_test.contains("orv_smoke_response_origin_header()"));
    assert!(smoke_test.contains("orv_smoke_curl_origin()"));
    assert!(smoke_test.contains("orv_smoke_curl_origin_response()"));
    assert!(smoke_test.contains("orv_smoke_fetch()"));
    assert!(smoke_test.contains("orv_smoke_fetch_origin()"));
    assert!(smoke_test.contains("orv_smoke_fetch_capture_origin()"));
    assert!(smoke_test.contains("orv_smoke_body_contains()"));
    assert!(smoke_test.contains("orv_smoke_cookie_from_headers()"));
    assert!(smoke_test.contains("orv deploy smoke test failed: %s"));
    assert!(smoke_test.contains(r#"READY_PATH="/health""#));
    assert!(smoke_test.contains("for attempt in 1 2 3 4 5"));
    assert!(smoke_test.contains("sleep 1"));
    assert!(smoke_test.contains(r#"ORV_SMOKE_ORIGIN_GET_HEALTH="ori_"#));
    assert!(smoke_test.contains(r#"ORV_SMOKE_RESPONSE_ORIGIN_GET_HEALTH="ori_"#));
    assert!(smoke_test.contains(
            r#"orv_smoke_curl_origin_response "GET /health" "$ORV_SMOKE_ORIGIN_GET_HEALTH" "$ORV_SMOKE_RESPONSE_ORIGIN_GET_HEALTH" "$BASE_URL/health""#
        ));
    assert!(smoke_test.contains(
            r#"orv_smoke_fetch_capture_origin "GET / home" "$SMOKE_HOME_BODY" "$SMOKE_HEADERS" "$ORV_SMOKE_ORIGIN_GET_ROOT" "$BASE_URL/""#
        ));
    assert!(smoke_test
        .contains(r#"orv_smoke_body_contains "home title" "$SMOKE_HOME_BODY" 'Miol Shop'"#));
    assert!(smoke_test.contains(
            r#"orv_smoke_body_contains "home copy" "$SMOKE_HOME_BODY" 'Catalog, member signup, payment capture, and shipment booking are ready.'"#
        ));
    assert!(smoke_test.contains(
            r#"orv_smoke_body_contains "home theme surface" "$SMOKE_HOME_BODY" 'background-color: #f8fafc'"#
        ));
    assert!(smoke_test.contains(
            r#"orv_smoke_body_contains "home theme typography" "$SMOKE_HOME_BODY" 'font-family: Inter, system-ui, sans-serif'"#
        ));
    assert!(smoke_test.contains(
            r#"orv_smoke_reveal_contains "reveal GET / source" "$ORV_SMOKE_ORIGIN_GET_ROOT" '@route GET /'"#
        ));
    assert!(smoke_test.contains(
            r#"orv_smoke_reveal_contains "reveal GET / production" "$ORV_SMOKE_ORIGIN_GET_ROOT" '"path": "/"'"#
        ));
    assert!(smoke_test.contains(
            r#"orv_smoke_reveal_contains "reveal GET / native target summary" "$ORV_SMOKE_ORIGIN_GET_ROOT" '"native_server_target_count": 1'"#
        ));
    assert!(smoke_test.contains(
            r#"orv_smoke_editor_reveal_contains "editor reveal GET / native route summary" "$ORV_SMOKE_ORIGIN_GET_ROOT" '"native_server_route_count": 1'"#
        ));
    assert!(smoke_test.contains(
            r#"orv_smoke_lsp_reveal_contains "lsp reveal GET / native target summary" "$ORV_SMOKE_ORIGIN_GET_ROOT" '"native_server_target_count": 1'"#
        ));
    assert!(smoke_test.contains(
            r#"orv_smoke_reveal_contains "reveal GET /health response source" "$ORV_SMOKE_RESPONSE_ORIGIN_GET_HEALTH" '@respond'"#
        ));
    assert!(smoke_test.contains(
            r#"orv_smoke_reveal_contains "reveal GET /health response production" "$ORV_SMOKE_RESPONSE_ORIGIN_GET_HEALTH" '"response_origin_dispatch": true'"#
        ));
    assert!(smoke_test.contains(
            r#"orv_smoke_lsp_reveal_contains "lsp reveal GET /health response origin" "$ORV_SMOKE_RESPONSE_ORIGIN_GET_HEALTH" '"name": "respond"'"#
        ));
    assert!(smoke_test.contains(
            r#"orv_smoke_lsp_reveal_contains "lsp reveal GET /health response production" "$ORV_SMOKE_RESPONSE_ORIGIN_GET_HEALTH" '"response_origin_dispatch": true'"#
        ));
    assert!(smoke_test.contains(r#"ORV_SMOKE_DB_CONNECT_ORIGIN="ori_"#));
    assert!(smoke_test.contains(
            r#"orv_smoke_reveal_contains "reveal DB source" "$ORV_SMOKE_DB_CONNECT_ORIGIN" '@db.connect'"#
        ));
    assert!(smoke_test.contains(
            r#"orv_smoke_reveal_contains "reveal DB preflight" "$ORV_SMOKE_DB_CONNECT_ORIGIN" '"preflight"'"#
        ));
    assert!(smoke_test.contains(
            r#"orv_smoke_reveal_contains "reveal DB smoke summary" "$ORV_SMOKE_DB_CONNECT_ORIGIN" '"smoke_test_summary"'"#
        ));
    assert!(smoke_test.contains(
            r#"orv_smoke_reveal_contains "reveal DB smoke summary count" "$ORV_SMOKE_DB_CONNECT_ORIGIN" '"preflight_smoke_summary_missing_count"'"#
        ));
    assert!(smoke_test.contains(
            r#"orv_smoke_reveal_contains "reveal DB sqlite path" "$ORV_SMOKE_DB_CONNECT_ORIGIN" 'sqlite://data/shop.sqlite'"#
        ));
    assert!(smoke_test.contains(
            r#"orv_smoke_editor_reveal_contains "editor reveal DB smoke summary" "$ORV_SMOKE_DB_CONNECT_ORIGIN" '"smoke_test_summary"'"#
        ));
    assert!(smoke_test.contains(
            r#"orv_smoke_editor_reveal_contains "editor reveal DB smoke summary count" "$ORV_SMOKE_DB_CONNECT_ORIGIN" '"preflight_smoke_summary_missing_count"'"#
        ));
    assert!(smoke_test.contains(
            r#"orv_smoke_lsp_reveal_contains "lsp reveal DB origin" "$ORV_SMOKE_DB_CONNECT_ORIGIN" '@db.connect'"#
        ));
    assert!(smoke_test.contains(
            r#"orv_smoke_lsp_reveal_contains "lsp reveal DB smoke summary" "$ORV_SMOKE_DB_CONNECT_ORIGIN" '"smoke_test_summary"'"#
        ));
    assert!(smoke_test.contains(
            r#"orv_smoke_lsp_reveal_contains "lsp reveal DB smoke summary count" "$ORV_SMOKE_DB_CONNECT_ORIGIN" '"preflight_smoke_summary_missing_count"'"#
        ));
    assert!(smoke_test.contains(r#"ORV_SMOKE_PAYMENT_CONNECT_ORIGIN="ori_"#));
    assert!(smoke_test.contains(r#"ORV_SMOKE_SHIPPING_CONNECT_ORIGIN="ori_"#));
    assert!(smoke_test.contains(
            r#"orv_smoke_reveal_contains "reveal payment source" "$ORV_SMOKE_PAYMENT_CONNECT_ORIGIN" '@payment.connect'"#
        ));
    assert!(smoke_test.contains(
            r#"orv_smoke_reveal_contains "reveal payment match" "$ORV_SMOKE_PAYMENT_CONNECT_ORIGIN" '"matched": true'"#
        ));
    assert!(smoke_test.contains(
            r#"orv_smoke_reveal_contains "reveal payment record path" "$ORV_SMOKE_PAYMENT_CONNECT_ORIGIN" 'file://data/payments.jsonl'"#
        ));
    assert!(smoke_test.contains(
            r#"orv_smoke_reveal_contains "reveal payment request kind" "$ORV_SMOKE_PAYMENT_CONNECT_ORIGIN" 'payment.capture'"#
        ));
    assert!(smoke_test.contains(
            r#"orv_smoke_lsp_reveal_contains "lsp reveal payment match" "$ORV_SMOKE_PAYMENT_CONNECT_ORIGIN" '"matched": true'"#
        ));
    assert!(smoke_test.contains(
            r#"orv_smoke_reveal_contains "reveal shipping source" "$ORV_SMOKE_SHIPPING_CONNECT_ORIGIN" '@shipping.connect'"#
        ));
    assert!(smoke_test.contains(
            r#"orv_smoke_reveal_contains "reveal shipping match" "$ORV_SMOKE_SHIPPING_CONNECT_ORIGIN" '"matched": true'"#
        ));
    assert!(smoke_test.contains(
            r#"orv_smoke_reveal_contains "reveal shipping record path" "$ORV_SMOKE_SHIPPING_CONNECT_ORIGIN" 'file://data/shipments.jsonl'"#
        ));
    assert!(smoke_test.contains(
            r#"orv_smoke_reveal_contains "reveal shipping request kind" "$ORV_SMOKE_SHIPPING_CONNECT_ORIGIN" 'shipping.booking'"#
        ));
    assert!(smoke_test.contains(
            r#"orv_smoke_lsp_reveal_contains "lsp reveal shipping match" "$ORV_SMOKE_SHIPPING_CONNECT_ORIGIN" '"matched": true'"#
        ));
    assert!(smoke_test
        .contains("CSRF_COOKIE=\"$(orv_smoke_cookie_from_headers orv_csrf \"$SMOKE_HEADERS\")\""));
    assert!(smoke_test.contains(r#"-H "x-csrf-token: ${CSRF_TOKEN}""#));
    assert!(smoke_test
            .contains(r#"orv_smoke_curl_origin "POST /products" "$ORV_SMOKE_ORIGIN_POST_PRODUCTS" -X POST "$BASE_URL/products""#));
    assert!(smoke_test.contains(r#"SMOKE_SKU="orv-smoke-sku-${SMOKE_ID}""#));
    assert!(smoke_test.contains(r#"SMOKE_SKU_SECOND="orv-smoke-sku-${SMOKE_ID}-2""#));
    assert!(smoke_test.contains(r#"SMOKE_SKU_THIRD="orv-smoke-sku-${SMOKE_ID}-3""#));
    assert!(smoke_test.contains(r#"SMOKE_BADGE="orv-smoke-badge-${SMOKE_ID}""#));
    assert!(smoke_test.contains(r#"SMOKE_BADGE_SECOND="orv-smoke-badge-${SMOKE_ID}-2""#));
    assert!(smoke_test.contains(r#"SMOKE_BADGE_THIRD="orv-smoke-badge-${SMOKE_ID}-3""#));
    assert!(smoke_test.contains(
        r#"orv_smoke_curl_origin "POST /products second" "$ORV_SMOKE_ORIGIN_POST_PRODUCTS""#
    ));
    assert!(smoke_test.contains(
        r#"orv_smoke_curl_origin "POST /products third" "$ORV_SMOKE_ORIGIN_POST_PRODUCTS""#
    ));
    assert!(smoke_test
            .contains(r#"orv_smoke_curl_origin "POST /members" "$ORV_SMOKE_ORIGIN_POST_MEMBERS" -X POST "$BASE_URL/members""#));
    assert!(smoke_test.contains(
            r#"orv_smoke_curl_capture_origin "POST /members/login smoke" "$SMOKE_MEMBER_HEADERS" "$ORV_SMOKE_ORIGIN_POST_MEMBERS_LOGIN""#
        ));
    assert!(smoke_test.contains("MEMBER_SESSION_COOKIE=\"$(orv_smoke_cookie_from_headers orv_session \"$SMOKE_MEMBER_HEADERS\")\""));
    assert!(smoke_test.contains(
            r#"orv_smoke_curl_origin "GET /account/sessions" "$ORV_SMOKE_ORIGIN_GET_ACCOUNT_SESSIONS" -H "cookie: ${MEMBER_SESSION_COOKIE}" "$BASE_URL/account/sessions""#
        ));
    assert!(smoke_test.contains(r#"SMOKE_HANDLE="orv-smoke-${SMOKE_ID}""#));
    assert!(smoke_test.contains(r#"SMOKE_PASSWORD="orv-smoke-password-${SMOKE_ID}""#));
    assert!(smoke_test.contains(r#"\"password\":\"${SMOKE_PASSWORD}\""#));
    assert!(smoke_test.contains(
            r#"orv_smoke_curl_origin "POST /cart/items" "$ORV_SMOKE_ORIGIN_POST_CART_ITEMS" -X POST "$BASE_URL/cart/items""#
        ));
    assert!(smoke_test.contains(
            r#"orv_smoke_fetch_origin "POST /checkout" "$SMOKE_CHECKOUT_BODY" "$ORV_SMOKE_ORIGIN_POST_CHECKOUT" -X POST "$BASE_URL/checkout""#
        ));
    assert!(smoke_test.contains(
            r#"orv_smoke_body_contains "checkout shipped order" "$SMOKE_CHECKOUT_BODY" '"status":"shipped"'"#
        ));
    assert!(smoke_test.contains(
            r#"orv_smoke_body_contains "checkout captured payment" "$SMOKE_CHECKOUT_BODY" '"status":"captured"'"#
        ));
    assert!(smoke_test.contains(
        r#"orv_smoke_body_contains "checkout shipment tracking" "$SMOKE_CHECKOUT_BODY" 'TRK-LOCAL'"#
    ));
    assert!(smoke_test.contains(
            r#"orv_smoke_curl_capture_origin "POST /members/login admin" "$SMOKE_ADMIN_HEADERS" "$ORV_SMOKE_ORIGIN_POST_MEMBERS_LOGIN""#
        ));
    assert!(smoke_test.contains("ADMIN_SESSION_COOKIE=\"$(orv_smoke_cookie_from_headers orv_session \"$SMOKE_ADMIN_HEADERS\")\""));
    assert!(smoke_test.contains("ADMIN_ROLE_COOKIE=\"$(orv_smoke_cookie_from_headers orv_session_role \"$SMOKE_ADMIN_HEADERS\")\""));
    assert!(smoke_test.contains(
            r#"orv_smoke_curl_origin "GET /admin/summary" "$ORV_SMOKE_ORIGIN_GET_ADMIN_SUMMARY" -H "cookie: ${ADMIN_SESSION_COOKIE}; ${ADMIN_ROLE_COOKIE}" "$BASE_URL/admin/summary""#
        ));
    assert!(smoke_test.contains(
            r#"orv_smoke_fetch_origin "GET /admin dashboard content" "$SMOKE_ADMIN_BODY" "$ORV_SMOKE_ORIGIN_GET_ADMIN" -H "cookie: ${ADMIN_SESSION_COOKIE}; ${ADMIN_ROLE_COOKIE}" "$BASE_URL/admin""#
        ));
    assert!(smoke_test.contains(
        r#"orv_smoke_body_contains "admin dashboard title" "$SMOKE_ADMIN_BODY" 'Miol Shop Admin'"#
    ));
    assert!(smoke_test.contains(
            r#"orv_smoke_body_contains "admin dashboard summary link" "$SMOKE_ADMIN_BODY" '/admin/summary'"#
        ));
    assert!(smoke_test.contains(
            r#"orv_smoke_body_contains "admin dashboard webhook link" "$SMOKE_ADMIN_BODY" '/admin/webhooks'"#
        ));
    assert!(smoke_test.contains(
            r#"orv_smoke_body_contains "admin dashboard sqlite storage" "$SMOKE_ADMIN_BODY" 'data/shop.sqlite'"#
        ));
    assert!(smoke_test.contains(
            r#"orv_smoke_body_contains "admin summary webhook events" "$SMOKE_ADMIN_SUMMARY_BODY" '"webhookEvents"'"#
        ));
    assert!(smoke_test.contains(
            r#"orv_smoke_body_contains "admin summary audit events" "$SMOKE_ADMIN_SUMMARY_BODY" '"auditEvents"'"#
        ));
    assert!(smoke_test.contains(
        r#"orv_smoke_body_contains "catalog smoke product" "$SMOKE_CATALOG_BODY" "$SMOKE_SKU""#
    ));
    assert!(smoke_test.contains(
            r#"orv_smoke_body_contains "catalog second smoke product" "$SMOKE_CATALOG_BODY" "$SMOKE_SKU_SECOND""#
        ));
    assert!(smoke_test.contains(
            r#"orv_smoke_body_contains "catalog third smoke product" "$SMOKE_CATALOG_BODY" "$SMOKE_SKU_THIRD""#
        ));
    assert!(smoke_test.contains(
            r#"orv_smoke_body_contains "catalog smoke product field" "$SMOKE_CATALOG_BODY" "$SMOKE_BADGE""#
        ));
    assert!(smoke_test.contains(
            r#"orv_smoke_body_contains "catalog second smoke product field" "$SMOKE_CATALOG_BODY" "$SMOKE_BADGE_SECOND""#
        ));
    assert!(smoke_test.contains(
            r#"orv_smoke_body_contains "catalog third smoke product field" "$SMOKE_CATALOG_BODY" "$SMOKE_BADGE_THIRD""#
        ));
    assert!(smoke_test
        .contains(r#"orv_smoke_body_contains "cart smoke item" "$SMOKE_CART_BODY" "$SMOKE_SKU""#));
    assert!(smoke_test.contains(
        r#"orv_smoke_body_contains "account smoke session" "$SMOKE_ACCOUNT_BODY" "$SMOKE_HANDLE""#
    ));
    assert!(smoke_test.contains(
            r#"orv_smoke_body_contains "admin catalog smoke product" "$SMOKE_ADMIN_CATALOG_BODY" "$SMOKE_SKU""#
        ));
    assert!(smoke_test.contains(
            r#"orv_smoke_body_contains "admin catalog second smoke product" "$SMOKE_ADMIN_CATALOG_BODY" "$SMOKE_SKU_SECOND""#
        ));
    assert!(smoke_test.contains(
            r#"orv_smoke_body_contains "admin catalog third smoke product" "$SMOKE_ADMIN_CATALOG_BODY" "$SMOKE_SKU_THIRD""#
        ));
    assert!(smoke_test.contains(
            r#"orv_smoke_body_contains "admin catalog smoke product field" "$SMOKE_ADMIN_CATALOG_BODY" "$SMOKE_BADGE""#
        ));
    assert!(smoke_test.contains(
            r#"orv_smoke_body_contains "admin catalog second smoke product field" "$SMOKE_ADMIN_CATALOG_BODY" "$SMOKE_BADGE_SECOND""#
        ));
    assert!(smoke_test.contains(
            r#"orv_smoke_body_contains "admin catalog third smoke product field" "$SMOKE_ADMIN_CATALOG_BODY" "$SMOKE_BADGE_THIRD""#
        ));
    assert!(smoke_test.contains(
        r#"orv_smoke_body_contains "admin orders shipped" "$SMOKE_ADMIN_ORDERS_BODY" 'shipped'"#
    ));
    assert!(smoke_test.contains(
            r#"orv_smoke_body_contains "admin payments captured" "$SMOKE_ADMIN_PAYMENTS_BODY" 'captured'"#
        ));
    assert!(smoke_test.contains(
            r#"orv_smoke_body_contains "admin shipments tracking" "$SMOKE_ADMIN_SHIPMENTS_BODY" 'TRK-LOCAL'"#
        ));
    assert!(smoke_test.contains(
            r#"orv_smoke_fetch_origin "GET /admin/webhooks content" "$SMOKE_ADMIN_WEBHOOKS_BODY" "$ORV_SMOKE_ORIGIN_GET_ADMIN_WEBHOOKS" -H "cookie: ${ADMIN_SESSION_COOKIE}; ${ADMIN_ROLE_COOKIE}" "$BASE_URL/admin/webhooks""#
        ));
    assert!(smoke_test.contains(
        r#"orv_smoke_body_contains "admin webhooks title" "$SMOKE_ADMIN_WEBHOOKS_BODY" 'Webhooks'"#
    ));
    assert!(smoke_test.contains(
            r#"orv_smoke_body_contains "admin audit checkout" "$SMOKE_ADMIN_AUDIT_BODY" 'checkout.complete'"#
        ));
    let runbook =
        std::fs::read_to_string(out.join("deploy").join("README.md")).expect("deploy runbook");
    assert!(runbook.contains("deploy/env.example"));
    assert!(runbook.contains("deploy/commerce-adapters.json"));
    assert!(runbook.contains("deploy/smoke-test.sh"));
    assert!(runbook.contains("deploy/smoke-output.txt"));
    assert!(runbook.contains("deploy/preflight.json"));
    assert!(runbook.contains("deploy/benchmark-evidence.json"));
    assert!(runbook.contains("## Benchmark Evidence"));
    assert!(runbook.contains("./deploy/smoke-test.sh"));
    assert!(runbook.contains("ORV_SMOKE_TRACE_STREAM=1 ./deploy/smoke-test.sh"));
    assert!(runbook.contains("orv verify-build ."));
    assert!(runbook.contains("orv editor run-debug . --control next"));
    assert!(runbook.contains("orv benchmark-report ."));
    assert!(runbook.contains("orv benchmark-report . --require-pass"));
    assert!(
        runbook.contains("- DB adapter env: SHOP_DATABASE_URL default sqlite://data/shop.sqlite")
    );
    assert!(runbook.contains("- Record log: data/payments.jsonl"));
    assert!(runbook.contains("- Record log: data/shipments.jsonl"));
    assert!(runbook.contains(
        "- Commerce adapter env: PAYMENT_ADAPTER_URL default file://data/payments.jsonl"
    ));
    assert!(runbook.contains(
        "- Commerce adapter env: SHIPPING_ADAPTER_URL default file://data/shipments.jsonl"
    ));
    assert!(runbook.contains(
        "- Commerce provider env: payment stripe STRIPE_WEBHOOK_SECRET optional webhook_signature"
    ));
    cmd_verify_build(&out).expect("verify shop prod build");
    let _ = std::fs::remove_dir_all(dir);
}
