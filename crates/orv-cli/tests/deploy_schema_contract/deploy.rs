use crate::common::assert_keys;

pub fn assert_deploy_manifest_contract(deploy: &serde_json::Value) {
    assert_keys(
        deploy,
        &[
            "schema_version",
            "profile",
            "entry",
            "runtime",
            "runtime_features",
            "source_bundle",
            "server",
            "static",
            "client",
        ],
        "deploy manifest",
    );
    assert_eq!(deploy["schema_version"], serde_json::json!(1));
    assert_eq!(deploy["profile"], serde_json::json!("prod"));
    assert_keys(
        &deploy["server"],
        &[
            "runtime",
            "runtime_features",
            "artifact",
            "entrypoint",
            "routes_artifact",
            "native_plan",
            "native_runtime_image_plan",
            "native_routes_source",
            "native_router_source",
            "native_handlers_source",
            "container",
            "dockerfile",
            "compose",
            "env_example",
            "db_adapters",
            "commerce_adapters",
            "smoke_test",
            "smoke_output",
            "preflight",
            "benchmark_evidence",
            "participant_notes_template",
            "runbook",
            "runtime_image",
            "protocol",
            "listen",
            "routes",
            "persistence",
        ],
        "deploy manifest server",
    );
    assert_eq!(
        deploy["server"]["participant_notes_template"],
        serde_json::json!("deploy/participant-notes-template.md")
    );
    assert!(deploy["server"]["routes"].is_array());
}

pub fn assert_deploy_routes_contract(routes: &serde_json::Value, deploy: &serde_json::Value) {
    assert_keys(
        routes,
        &[
            "schema_version",
            "artifact",
            "runtime",
            "protocol",
            "routes",
        ],
        "deploy routes",
    );
    assert_eq!(routes["schema_version"], serde_json::json!(1));
    assert_eq!(
        routes["artifact"],
        serde_json::json!("server/app.orv-runtime.json")
    );
    assert_eq!(
        routes["runtime"],
        serde_json::json!("reference-interpreter")
    );
    assert_eq!(routes["protocol"], serde_json::json!("http1"));
    assert_eq!(routes["routes"], deploy["server"]["routes"]);
    let route = routes["routes"]
        .as_array()
        .expect("deploy routes")
        .iter()
        .find(|route| route["method"] == "GET" && route["path"] == "/ping")
        .expect("GET /ping deploy route");
    assert!(route["origin_id"]
        .as_str()
        .is_some_and(|origin_id| origin_id.starts_with("ori_")));
    assert!(route["response_origin_ids"]
        .as_array()
        .is_some_and(|items| !items.is_empty()));
}

pub fn assert_deploy_container_contract(container: &serde_json::Value, deploy: &serde_json::Value) {
    assert_keys(
        container,
        &[
            "schema_version",
            "kind",
            "dockerfile",
            "artifact",
            "entrypoint",
            "routes_artifact",
            "runtime",
            "runtime_image",
            "protocol",
            "listen",
            "ports",
            "command",
            "persistence",
        ],
        "deploy container",
    );
    assert_eq!(container["schema_version"], serde_json::json!(1));
    assert_eq!(
        container["kind"],
        serde_json::json!("reference-server-container")
    );
    assert_eq!(
        container["dockerfile"],
        serde_json::json!("deploy/Dockerfile")
    );
    assert_eq!(
        container["artifact"],
        serde_json::json!("server/app.orv-runtime.json")
    );
    assert_eq!(
        container["entrypoint"],
        serde_json::json!("deploy/server.sh")
    );
    assert_eq!(
        container["routes_artifact"],
        serde_json::json!("deploy/routes.json")
    );
    assert_eq!(
        container["runtime"],
        serde_json::json!("reference-interpreter")
    );
    assert_eq!(
        container["runtime_image"],
        deploy["server"]["runtime_image"]
    );
    assert_eq!(container["protocol"], serde_json::json!("http1"));
    assert_eq!(container["listen"], deploy["server"]["listen"]);
    assert_eq!(
        container["command"],
        serde_json::json!(["./deploy/server.sh"])
    );
    assert_eq!(container["persistence"], deploy["server"]["persistence"]);
    let port = container["ports"]
        .as_array()
        .expect("deploy container ports")
        .first()
        .expect("deploy container port");
    assert_keys(port, &["container", "protocol"], "deploy container port");
    assert_eq!(port["container"], serde_json::json!(8080));
    assert_eq!(port["protocol"], serde_json::json!("tcp"));
}

pub fn assert_preflight_contract(preflight: &serde_json::Value) {
    assert_keys(
        preflight,
        &[
            "schema_version",
            "kind",
            "artifact",
            "runtime",
            "runtime_features",
            "security_features",
            "listen",
            "routes",
            "persistence",
            "required_env",
            "optional_env",
            "commands",
            "artifacts",
            "smoke_output_contract",
            "benchmark",
            "client",
        ],
        "deploy preflight",
    );
    assert_eq!(preflight["schema_version"], serde_json::json!(1));
    assert_eq!(preflight["kind"], serde_json::json!("orv.deploy.preflight"));
    assert_keys(
        &preflight["commands"],
        &[
            "verify_build",
            "env_check",
            "run_build",
            "smoke_test",
            "editor_run_debug",
            "benchmark_prepare",
            "benchmark_report",
            "benchmark_report_require_pass",
            "compose_up",
            "trace",
            "trace_run_build",
            "editor_trace",
            "trace_stream_smoke",
        ],
        "deploy preflight commands",
    );
    assert_keys(
        &preflight["artifacts"],
        &[
            "server",
            "routes",
            "source_bundle",
            "project_graph",
            "origin_map",
            "build_manifest",
            "bundle_plan",
            "env_example",
            "db_adapters",
            "commerce_adapters",
            "smoke_test",
            "smoke_output",
            "preflight",
            "benchmark_evidence",
            "participant_notes_template",
            "runbook",
        ],
        "deploy preflight artifacts",
    );
    assert_keys(
        &preflight["smoke_output_contract"],
        &["output", "required_markers"],
        "smoke output contract",
    );
    assert!(preflight["smoke_output_contract"]["required_markers"].is_array());
}
