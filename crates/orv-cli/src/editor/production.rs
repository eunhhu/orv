#![allow(clippy::redundant_pub_crate, clippy::wildcard_imports)]

use super::*;

pub(crate) fn verify_editor_debug_production_context_contract_keys(
    context: &serde_json::Value,
) -> anyhow::Result<()> {
    verify_json_object_keys_exact(
        context,
        &[
            "schema_version",
            "kind",
            "build_dir",
            "source_bundle",
            "graph_contract",
            "preflight",
            "summary",
        ],
        "editor debug production_context",
    )?;
    verify_editor_debug_production_summary_contract_keys(context.get("summary").ok_or_else(
        || anyhow::anyhow!("editor debug production_context.summary must be an object"),
    )?)
}

pub(crate) fn verify_editor_debug_production_summary_contract_keys(
    summary: &serde_json::Value,
) -> anyhow::Result<()> {
    verify_json_object_keys_exact(
        summary,
        &[
            "schema_version",
            "build_dir",
            "graph_contract_count",
            "source_bundle_file_count",
            "project_graph_node_count",
            "origin_entry_count",
            "client_target_count",
            "client_manifest_count",
            "client_capability_surface_count",
            "route_target_count",
            "native_server_target_count",
            "native_server_route_count",
            "native_server_blocker_count",
            "static_target_count",
            "static_verified_count",
            "preflight_target_count",
            "preflight_command_count",
            "preflight_route_count",
            "preflight_required_env_count",
            "preflight_optional_env_count",
            "preflight_smoke_summary_present_count",
            "preflight_smoke_summary_missing_count",
            "preflight_smoke_summary_missing_marker_count",
            "route_policy_count",
            "route_policy_kind_counts",
            "db_target_count",
            "commerce_target_count",
            "db_adapter_count",
            "commerce_adapter_count",
            "adapter_count",
            "missing_artifact_count",
        ],
        "editor debug production_context.summary",
    )
}

pub(crate) fn editor_debug_production_summary_from_context(
    production_context: &serde_json::Value,
) -> serde_json::Value {
    if production_context.is_null() {
        return serde_json::Value::Null;
    }
    production_context
        .get("summary")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}))
}

pub(crate) fn editor_debug_attach_production_context(state: &mut serde_json::Value) {
    let production_context = editor_debug_production_context_json(
        state.get("production").unwrap_or(&serde_json::Value::Null),
    );
    if production_context.is_null() {
        return;
    }
    let Some(debug) = state
        .get_mut("debug")
        .and_then(serde_json::Value::as_object_mut)
    else {
        return;
    };
    debug.insert("production_context".to_string(), production_context.clone());
    if let Some(runner) = debug
        .get_mut("session_runner")
        .and_then(serde_json::Value::as_object_mut)
    {
        if let Some(source_bundle) = production_context.get("source_bundle").cloned() {
            runner.insert("source_bundle".to_string(), source_bundle);
        }
        runner.insert("production_context".to_string(), production_context);
    }
}

pub(crate) fn editor_debug_production_context_json(
    production: &serde_json::Value,
) -> serde_json::Value {
    if production.is_null() {
        return serde_json::Value::Null;
    }
    let source_bundle = production
        .get("build_dir")
        .and_then(serde_json::Value::as_str)
        .filter(|path| !path.trim().is_empty())
        .map(|path| {
            Path::new(path)
                .join(SOURCE_BUNDLE_PATH)
                .display()
                .to_string()
        })
        .map_or(serde_json::Value::Null, serde_json::Value::String);
    serde_json::json!({
        "schema_version": 1,
        "kind": "orv.editor.debug.production_context",
        "build_dir": production
            .get("build_dir")
            .cloned()
            .unwrap_or(serde_json::Value::Null),
        "source_bundle": source_bundle,
        "graph_contract": production
            .get("graph_contract")
            .cloned()
            .unwrap_or_else(|| serde_json::json!([])),
        "preflight": production
            .get("preflight")
            .cloned()
            .unwrap_or_else(|| serde_json::json!([])),
        "summary": production
            .get("summary")
            .cloned()
            .unwrap_or_else(|| production_summary_json(production)),
    })
}

pub(crate) fn editor_production_summary_json(build: &Path) -> anyhow::Result<serde_json::Value> {
    let mut production = serde_json::json!({
        "schema_version": 1,
        "kind": "orv.editor.production",
        "build_dir": build.display().to_string(),
        "graph_contract": editor_production_graph_contract_targets(build)?,
        "client": reveal_client_bundle_targets(build)?,
        "native_server": editor_production_native_server_targets(build)?,
        "static": editor_production_static_targets(build)?,
        "preflight": reveal_preflight_targets(build)?,
        "db_adapters": reveal_db_adapter_targets(build)?,
        "commerce_adapters": reveal_commerce_adapter_targets(build)?,
    });
    let summary = production_summary_json(&production);
    production
        .as_object_mut()
        .expect("editor production state is object")
        .insert("summary".to_string(), summary);
    Ok(production)
}

pub(crate) fn editor_production_graph_contract_targets(
    build: &Path,
) -> anyhow::Result<Vec<serde_json::Value>> {
    let specs = [
        ("source_bundle", SOURCE_BUNDLE_PATH),
        ("project_graph", "project-graph.json"),
        ("origin_map", "origin-map.json"),
    ];
    specs
        .into_iter()
        .map(|(kind, path)| editor_production_graph_contract_target(build, kind, path))
        .collect()
}

pub(crate) fn editor_production_graph_contract_target(
    build: &Path,
    kind: &str,
    path: &str,
) -> anyhow::Result<serde_json::Value> {
    let target = build.join(path);
    let mut value = serde_json::json!({
        "kind": kind,
        "path": path,
        "exists": target.is_file(),
    });
    if !target.is_file() {
        return Ok(value);
    }
    let artifact = read_json_value(&target)?;
    value["artifact_hash"] = serde_json::json!(stable_json_hash(&artifact)?);
    match kind {
        "source_bundle" => add_editor_source_bundle_contract_fields(&artifact, &mut value),
        "project_graph" => add_editor_project_graph_contract_fields(&artifact, &mut value),
        "origin_map" => add_editor_origin_map_contract_fields(&artifact, &mut value),
        _ => {}
    }
    Ok(value)
}

pub(crate) fn editor_production_native_server_targets(
    build: &Path,
) -> anyhow::Result<Vec<serde_json::Value>> {
    let plan = read_json_value(&build.join("bundle-plan.json"))?;
    let Some(bundles) = plan.get("bundles").and_then(serde_json::Value::as_array) else {
        return Ok(Vec::new());
    };
    let mut targets = Vec::new();
    for bundle in bundles {
        if bundle.get("kind").and_then(serde_json::Value::as_str) != Some("native_server_plan") {
            continue;
        }
        let path = json_str(bundle, "path", "bundle target")?;
        let target_path = build.join(path);
        if !target_path.is_file() {
            targets.push(serde_json::json!({
                "kind": "native_server_plan",
                "path": path,
                "exists": false,
            }));
            continue;
        }
        let native_plan = read_json_value(&target_path)?;
        let routes = native_plan
            .get("routes")
            .cloned()
            .unwrap_or_else(|| serde_json::json!([]));
        targets.push(native_server_production_target_json(
            build,
            path,
            &native_plan,
            routes,
        )?);
    }
    Ok(targets)
}

pub(crate) fn editor_production_static_targets(
    build: &Path,
) -> anyhow::Result<Vec<serde_json::Value>> {
    let plan = read_json_value(&build.join("bundle-plan.json"))?;
    let Some(bundles) = plan.get("bundles").and_then(serde_json::Value::as_array) else {
        return Ok(Vec::new());
    };
    let mut targets = Vec::new();
    for bundle in bundles.iter().filter(|bundle| {
        bundle.get("kind").and_then(serde_json::Value::as_str) == Some("static_page")
    }) {
        let path = json_str(bundle, "path", "bundle target")?;
        let target_path = build.join(path);
        let exists = target_path.is_file();
        let verified = exists && verify_static_page_target(bundle, &target_path).is_ok();
        targets.push(serde_json::json!({
            "kind": "static_page",
            "path": path,
            "exists": exists,
            "verified": verified,
            "runtime_features": bundle
                .get("runtime_features")
                .cloned()
                .unwrap_or_else(|| serde_json::json!([])),
        }));
    }
    Ok(targets)
}

pub(crate) fn native_server_production_target_json(
    dir: &Path,
    path: &str,
    native_plan: &serde_json::Value,
    routes: serde_json::Value,
) -> anyhow::Result<serde_json::Value> {
    Ok(serde_json::json!({
        "kind": "native_server_plan",
        "path": path,
        "exists": true,
        "status": native_plan
            .get("status")
            .cloned()
            .unwrap_or(serde_json::Value::Null),
        "artifact": native_plan
            .get("artifact")
            .cloned()
            .unwrap_or(serde_json::Value::Null),
        "launcher": native_plan
            .get("launcher")
            .cloned()
            .unwrap_or(serde_json::Value::Null),
        "routes_source": reveal_native_server_routes_source(dir, native_plan)?,
        "router_source": reveal_native_server_router_source(dir, native_plan)?,
        "handlers_source": reveal_native_server_handlers_source(dir, native_plan)?,
        "target": native_plan
            .get("target")
            .cloned()
            .unwrap_or(serde_json::Value::Null),
        "runtime_image": reveal_native_runtime_image_plan(dir, native_plan)?,
        "commands": native_plan
            .get("commands")
            .cloned()
            .unwrap_or(serde_json::Value::Null),
        "runtime_features": native_plan
            .get("runtime_features")
            .cloned()
            .unwrap_or_else(|| serde_json::json!([])),
        "blocked_by": native_plan
            .get("blocked_by")
            .cloned()
            .unwrap_or_else(|| serde_json::json!([])),
        "route_count": json_array_count(Some(&routes)),
        "routes": routes,
    }))
}

pub(crate) fn editor_native_host_production_json(
    production: &serde_json::Value,
) -> serde_json::Value {
    let Some(mut object) = production.as_object().cloned() else {
        return serde_json::Value::Null;
    };
    object.insert("summary".to_string(), production_summary_json(production));
    object.insert(
        "panel_contract".to_string(),
        editor_native_host_production_panel_contract_json(),
    );
    object.insert(
        "panel_html_path".to_string(),
        serde_json::json!(EDITOR_PRODUCTION_PANEL_HTML_PATH),
    );
    object.insert(
        "panel_artifact".to_string(),
        editor_production_panel_artifact_json(),
    );
    serde_json::Value::Object(object)
}

pub(crate) fn editor_production_panel_artifact_json() -> serde_json::Value {
    serde_json::json!({
        "schema_version": 1,
        "kind": "orv.editor.production.panel",
        "path": EDITOR_PRODUCTION_PANEL_HTML_PATH,
        "media_type": "text/html",
        "source": "native-host.production",
        "panel_contract": editor_native_host_production_panel_contract_json(),
    })
}

pub(crate) fn editor_native_host_production_panel_contract_json() -> serde_json::Value {
    serde_json::json!({
        "schema_version": 1,
        "root": "production",
        "sections": [
            {
                "name": "summary",
                "path": "production.summary",
                "kind": "object",
            },
            {
                "name": "graph_contract",
                "path": "production.graph_contract",
                "kind": "array",
            },
            {
                "name": "db_adapters",
                "path": "production.db_adapters",
                "kind": "array",
            },
            {
                "name": "preflight",
                "path": "production.preflight",
                "kind": "array",
            },
            {
                "name": "native_server",
                "path": "production.native_server",
                "kind": "array",
            },
            {
                "name": "static",
                "path": "production.static",
                "kind": "array",
            },
            {
                "name": "route_policies",
                "path": "production.summary.route_policy_kind_counts",
                "kind": "object",
            },
            {
                "name": "client",
                "path": "production.client",
                "kind": "array",
            },
            {
                "name": "commerce_adapters",
                "path": "production.commerce_adapters",
                "kind": "array",
            },
            {
                "name": "panel_artifact",
                "path": "production.panel_artifact",
                "kind": "object",
            },
        ],
    })
}

pub(crate) fn production_summary_json(production: &serde_json::Value) -> serde_json::Value {
    let db_adapters = production
        .get("db_adapters")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    let commerce_adapters = production
        .get("commerce_adapters")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    let preflight = production
        .get("preflight")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    let routes = production
        .get("routes")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    let native_server = production
        .get("native_server")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    let static_targets = production
        .get("static")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    let client = production
        .get("client")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    let graph_contract = production
        .get("graph_contract")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    let db_adapter_count = production_adapter_entry_count(&db_adapters);
    let commerce_adapter_count = production_adapter_entry_count(&commerce_adapters);
    serde_json::json!({
        "schema_version": 1,
        "build_dir": production
            .get("build_dir")
            .cloned()
            .unwrap_or_else(|| serde_json::json!("")),
        "graph_contract_count": graph_contract.len(),
        "source_bundle_file_count": production_graph_contract_number(
            &graph_contract,
            "source_bundle",
            "file_count",
        ),
        "project_graph_node_count": production_graph_contract_number(
            &graph_contract,
            "project_graph",
            "node_count",
        ),
        "origin_entry_count": production_graph_contract_number(
            &graph_contract,
            "origin_map",
            "entry_count",
        ),
        "client_target_count": client.len(),
        "client_manifest_count": production_client_manifest_count(&client),
        "client_capability_surface_count": production_client_capability_surface_count(&client),
        "route_target_count": routes.len(),
        "native_server_target_count": native_server.len(),
        "native_server_route_count": production_native_server_route_count(&native_server),
        "native_server_blocker_count": production_native_server_blocker_count(&native_server),
        "static_target_count": static_targets.len(),
        "static_verified_count": production_static_verified_count(&static_targets),
        "preflight_target_count": preflight.len(),
        "preflight_command_count": production_preflight_command_count(&preflight),
        "preflight_route_count": production_preflight_route_count(&preflight),
        "preflight_required_env_count": production_preflight_env_count(&preflight, "required_env"),
        "preflight_optional_env_count": production_preflight_env_count(&preflight, "optional_env"),
        "preflight_smoke_summary_present_count": production_preflight_smoke_summary_present_count(&preflight),
        "preflight_smoke_summary_missing_count": production_preflight_smoke_summary_missing_count(&preflight),
        "preflight_smoke_summary_missing_marker_count": production_preflight_smoke_summary_missing_marker_count(&preflight),
        "route_policy_count": production_preflight_route_policy_count(&preflight),
        "route_policy_kind_counts": production_preflight_route_policy_kind_counts(&preflight),
        "db_target_count": db_adapters.len(),
        "commerce_target_count": commerce_adapters.len(),
        "db_adapter_count": db_adapter_count,
        "commerce_adapter_count": commerce_adapter_count,
        "adapter_count": db_adapter_count + commerce_adapter_count,
        "missing_artifact_count": production_missing_artifact_count(&graph_contract)
            + production_missing_artifact_count(&db_adapters)
            + production_missing_artifact_count(&commerce_adapters)
            + production_missing_artifact_count(&preflight)
            + production_missing_artifact_count(&native_server)
            + production_missing_artifact_count(&static_targets)
            + production_missing_artifact_count(&client),
    })
}

pub(crate) fn production_graph_contract_number(
    targets: &[serde_json::Value],
    kind: &str,
    key: &str,
) -> usize {
    targets
        .iter()
        .find(|target| target.get("kind").and_then(serde_json::Value::as_str) == Some(kind))
        .and_then(|target| target.get(key))
        .and_then(serde_json::Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())
        .unwrap_or(0)
}

pub(crate) fn production_client_manifest_count(targets: &[serde_json::Value]) -> usize {
    targets
        .iter()
        .filter(|target| {
            target.get("kind").and_then(serde_json::Value::as_str) == Some("client_manifest")
        })
        .count()
}

pub(crate) fn production_client_capability_surface_count(targets: &[serde_json::Value]) -> usize {
    targets
        .iter()
        .find(|target| {
            target.get("kind").and_then(serde_json::Value::as_str) == Some("client_manifest")
        })
        .and_then(|target| target.pointer("/capabilities/surfaces"))
        .and_then(serde_json::Value::as_array)
        .map_or(0, Vec::len)
}

pub(crate) fn production_native_server_route_count(targets: &[serde_json::Value]) -> usize {
    targets
        .iter()
        .map(|target| {
            target
                .get("route_count")
                .and_then(serde_json::Value::as_u64)
                .and_then(|value| usize::try_from(value).ok())
                .unwrap_or_else(|| json_array_count(target.get("routes")))
        })
        .sum()
}

pub(crate) fn production_native_server_blocker_count(targets: &[serde_json::Value]) -> usize {
    targets
        .iter()
        .map(|target| json_array_count(target.get("blocked_by")))
        .sum()
}

pub(crate) fn production_static_verified_count(targets: &[serde_json::Value]) -> usize {
    targets
        .iter()
        .filter(|target| target.get("verified").and_then(serde_json::Value::as_bool) == Some(true))
        .count()
}

pub(crate) fn production_adapter_entry_count(targets: &[serde_json::Value]) -> usize {
    targets
        .iter()
        .map(|target| json_array_count(target.get("adapters")))
        .sum()
}

pub(crate) fn production_preflight_env_count(targets: &[serde_json::Value], key: &str) -> usize {
    targets
        .iter()
        .map(|target| json_array_count(target.get(key)))
        .sum()
}

pub(crate) fn production_preflight_command_count(targets: &[serde_json::Value]) -> usize {
    targets
        .iter()
        .map(|target| json_object_count(target.get("commands")))
        .sum()
}

pub(crate) fn production_preflight_route_count(targets: &[serde_json::Value]) -> usize {
    targets
        .iter()
        .map(|target| json_array_count(target.get("routes")))
        .sum()
}

pub(crate) fn production_preflight_smoke_summary_present_count(
    targets: &[serde_json::Value],
) -> usize {
    targets
        .iter()
        .filter(|target| production_preflight_smoke_summary_present(target))
        .count()
}

pub(crate) fn production_preflight_smoke_summary_missing_count(
    targets: &[serde_json::Value],
) -> usize {
    targets
        .iter()
        .filter(|target| target.get("benchmark_evidence").is_some())
        .filter(|target| !production_preflight_smoke_summary_present(target))
        .count()
}

pub(crate) fn production_preflight_smoke_summary_missing_marker_count(
    targets: &[serde_json::Value],
) -> usize {
    targets
        .iter()
        .map(production_preflight_smoke_summary_missing_marker_count_from_target)
        .sum()
}

pub(crate) fn production_preflight_smoke_summary_present(target: &serde_json::Value) -> bool {
    target
        .pointer("/benchmark_evidence/smoke_test_summary/present")
        .and_then(serde_json::Value::as_bool)
        == Some(true)
}

pub(crate) fn production_preflight_smoke_summary_missing_marker_count_from_target(
    target: &serde_json::Value,
) -> usize {
    target
        .pointer("/benchmark_evidence/smoke_test_summary/missing_markers")
        .and_then(serde_json::Value::as_array)
        .map_or(0, Vec::len)
}

pub(crate) fn production_preflight_route_policy_count_from_value(
    production: &serde_json::Value,
) -> usize {
    production
        .get("preflight")
        .and_then(serde_json::Value::as_array)
        .map_or(0, |targets| {
            production_preflight_route_policy_count(targets)
        })
}

pub(crate) fn production_preflight_route_policy_count(targets: &[serde_json::Value]) -> usize {
    targets
        .iter()
        .flat_map(production_preflight_routes)
        .map(|route| json_array_count(route.get("policies")))
        .sum()
}

pub(crate) fn production_preflight_route_policy_kind_counts(
    targets: &[serde_json::Value],
) -> serde_json::Value {
    let mut counts = BTreeMap::new();
    for route in targets.iter().flat_map(production_preflight_routes) {
        for policy in route
            .get("policies")
            .and_then(serde_json::Value::as_array)
            .into_iter()
            .flatten()
        {
            if let Some(kind) = policy.get("kind").and_then(serde_json::Value::as_str) {
                *counts.entry(kind.to_string()).or_insert(0usize) += 1;
            }
        }
    }
    serde_json::to_value(counts).unwrap_or_else(|_| serde_json::json!({}))
}

pub(crate) fn production_preflight_routes(target: &serde_json::Value) -> Vec<&serde_json::Value> {
    target
        .get("routes")
        .and_then(serde_json::Value::as_array)
        .map(|routes| routes.iter().collect())
        .unwrap_or_default()
}

pub(crate) fn production_missing_artifact_count(targets: &[serde_json::Value]) -> usize {
    targets
        .iter()
        .filter(|target| target.get("exists").and_then(serde_json::Value::as_bool) == Some(false))
        .count()
}

pub(crate) fn write_editor_production_panel_html_if_configured(
    out: &Path,
    state: &serde_json::Value,
) -> anyhow::Result<bool> {
    let Some(production) = state.get("production") else {
        return Ok(false);
    };
    let production = editor_native_host_production_json(production);
    let html = editor_production_panel_html(&production)?;
    write_text(&out.join(EDITOR_PRODUCTION_PANEL_HTML_PATH), &html)?;
    Ok(true)
}

pub(crate) fn editor_production_panel_html(
    production: &serde_json::Value,
) -> anyhow::Result<String> {
    let summary = production
        .get("summary")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));
    let build_dir = html_escape_text(
        production
            .get("build_dir")
            .and_then(serde_json::Value::as_str)
            .unwrap_or(""),
    );
    let db_target_count = json_usize_field(&summary, "db_target_count");
    let commerce_target_count = json_usize_field(&summary, "commerce_target_count");
    let preflight_target_count = json_usize_field(&summary, "preflight_target_count");
    let preflight_command_count = json_usize_field(&summary, "preflight_command_count");
    let preflight_smoke_summary_present_count =
        json_usize_field(&summary, "preflight_smoke_summary_present_count");
    let preflight_smoke_summary_missing_count =
        json_usize_field(&summary, "preflight_smoke_summary_missing_count");
    let preflight_smoke_summary_missing_marker_count =
        json_usize_field(&summary, "preflight_smoke_summary_missing_marker_count");
    let route_policy_count = json_usize_field(&summary, "route_policy_count");
    let native_server_target_count = json_usize_field(&summary, "native_server_target_count");
    let native_server_route_count = json_usize_field(&summary, "native_server_route_count");
    let native_server_blocker_count = json_usize_field(&summary, "native_server_blocker_count");
    let static_target_count = json_usize_field(&summary, "static_target_count");
    let static_verified_count = json_usize_field(&summary, "static_verified_count");
    let client_target_count = json_usize_field(&summary, "client_target_count");
    let graph_contract_count = json_usize_field(&summary, "graph_contract_count");
    let adapter_count = json_usize_field(&summary, "adapter_count");
    let missing_artifact_count = json_usize_field(&summary, "missing_artifact_count");
    let graph_contract = html_escape_text(&serde_json::to_string_pretty(
        production
            .get("graph_contract")
            .unwrap_or(&serde_json::Value::Null),
    )?);
    let client = html_escape_text(&serde_json::to_string_pretty(
        production.get("client").unwrap_or(&serde_json::Value::Null),
    )?);
    let native_server = html_escape_text(&serde_json::to_string_pretty(
        production
            .get("native_server")
            .unwrap_or(&serde_json::Value::Null),
    )?);
    let static_targets = html_escape_text(&serde_json::to_string_pretty(
        production.get("static").unwrap_or(&serde_json::Value::Null),
    )?);
    let db_adapters = html_escape_text(&serde_json::to_string_pretty(
        production
            .get("db_adapters")
            .unwrap_or(&serde_json::Value::Null),
    )?);
    let commerce_adapters = html_escape_text(&serde_json::to_string_pretty(
        production
            .get("commerce_adapters")
            .unwrap_or(&serde_json::Value::Null),
    )?);
    let preflight = html_escape_text(&serde_json::to_string_pretty(
        production
            .get("preflight")
            .unwrap_or(&serde_json::Value::Null),
    )?);
    let route_policies = html_escape_text(&serde_json::to_string_pretty(
        summary
            .get("route_policy_kind_counts")
            .unwrap_or(&serde_json::Value::Null),
    )?);
    let panel_contract = html_escape_text(&serde_json::to_string_pretty(
        production
            .get("panel_contract")
            .unwrap_or(&serde_json::Value::Null),
    )?);
    let production_json = html_script_json(&serde_json::to_string_pretty(production)?);
    let mut html = String::new();
    html.push_str(
        "<!doctype html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n<title>orv Production Panel</title>\n<style>\n:root{color-scheme:light dark;--bg:#f7f6f2;--fg:#151713;--muted:#6b7067;--panel:#fff;--line:#d8d9d2;--accent:#67610f;--bad:#a43737;}\n@media (prefers-color-scheme: dark){:root{--bg:#11130f;--fg:#eef0ea;--muted:#a8aea2;--panel:#191c17;--line:#30362d;--accent:#d8cc65;--bad:#ff9d9d;}}\n*{box-sizing:border-box}body{margin:0;background:var(--bg);color:var(--fg);font:14px/1.45 ui-sans-serif,system-ui,-apple-system,BlinkMacSystemFont,\"Segoe UI\",sans-serif;}header{padding:24px 28px 12px;border-bottom:1px solid var(--line);}h1{font-size:24px;margin:0 0 8px}h2{font-size:13px;text-transform:uppercase;letter-spacing:.08em;color:var(--muted);margin:0 0 12px}.muted{color:var(--muted)}.summary{display:grid;grid-template-columns:repeat(4,minmax(0,1fr));gap:10px;margin-top:16px}.metric{border:1px solid var(--line);border-radius:6px;padding:10px;background:var(--panel)}.metric b{display:block;font-size:22px;line-height:1.1}.metric .bad{color:var(--bad)}main{display:grid;grid-template-columns:1fr 1fr;gap:16px;padding:16px 28px 28px}.panel{background:var(--panel);border:1px solid var(--line);border-radius:8px;padding:16px}.wide{grid-column:1/-1}pre{margin:0;white-space:pre-wrap;overflow:auto;font:12px/1.45 ui-monospace,SFMono-Regular,Menlo,Consolas,monospace}@media (max-width:900px){main,.summary{grid-template-columns:1fr}main{padding:14px}header{padding:18px 14px 8px}}\n</style>\n</head>\n<body>\n",
    );
    writeln!(
        &mut html,
        "<header><h1>Production Panel</h1><div class=\"muted\">{build_dir}</div><section class=\"summary\"><div class=\"metric\"><span>Graph Contracts</span><b>{graph_contract_count}</b></div><div class=\"metric\"><span>Client Targets</span><b>{client_target_count}</b></div><div class=\"metric\"><span>Native Plans</span><b>{native_server_target_count}</b></div><div class=\"metric\"><span>Native Routes</span><b>{native_server_route_count}</b></div><div class=\"metric\"><span>Native Blockers</span><b class=\"{}\">{native_server_blocker_count}</b></div><div class=\"metric\"><span>Static Pages</span><b>{static_verified_count}/{static_target_count}</b></div><div class=\"metric\"><span>Preflight</span><b>{preflight_target_count}</b></div><div class=\"metric\"><span>Preflight Commands</span><b>{preflight_command_count}</b></div><div class=\"metric\"><span>Smoke Summary</span><b>{preflight_smoke_summary_present_count}/{preflight_target_count}</b></div><div class=\"metric\"><span>Smoke Gaps</span><b class=\"{}\">{}</b></div><div class=\"metric\"><span>Route Policies</span><b>{route_policy_count}</b></div><div class=\"metric\"><span>DB Targets</span><b>{db_target_count}</b></div><div class=\"metric\"><span>Commerce Targets</span><b>{commerce_target_count}</b></div><div class=\"metric\"><span>Adapters</span><b>{adapter_count}</b></div><div class=\"metric\"><span>Missing</span><b class=\"{}\">{missing_artifact_count}</b></div></section></header>",
        if native_server_blocker_count == 0 { "" } else { "bad" },
        if preflight_smoke_summary_missing_count + preflight_smoke_summary_missing_marker_count == 0 {
            ""
        } else {
            "bad"
        },
        preflight_smoke_summary_missing_count + preflight_smoke_summary_missing_marker_count,
        if missing_artifact_count == 0 { "" } else { "bad" }
    )?;
    writeln!(
        &mut html,
        "<main><section class=\"panel wide\"><h2>Graph Contract</h2><pre>{graph_contract}</pre></section><section class=\"panel wide\"><h2>Client Bundles</h2><pre>{client}</pre></section><section class=\"panel wide\"><h2>Native Server</h2><pre>{native_server}</pre></section><section class=\"panel wide\"><h2>Static Pages</h2><pre>{static_targets}</pre></section><section class=\"panel wide\"><h2>Preflight</h2><pre>{preflight}</pre></section><section class=\"panel\"><h2>Route Policy Summary</h2><pre>{route_policies}</pre></section><section class=\"panel\"><h2>DB Adapters</h2><pre>{db_adapters}</pre></section><section class=\"panel\"><h2>Commerce Adapters</h2><pre>{commerce_adapters}</pre></section><section class=\"panel wide\"><h2>Panel Contract</h2><pre>{panel_contract}</pre></section></main>"
    )?;
    writeln!(
        &mut html,
        "<script id=\"orv-production\" type=\"application/json\">{production_json}</script>"
    )?;
    html.push_str("</body>\n</html>\n");
    Ok(html)
}

pub(crate) fn production_adapter_count(production: &serde_json::Value) -> usize {
    json_array_count(production.get("db_adapters"))
        + json_array_count(production.get("commerce_adapters"))
}

pub(crate) fn production_client_bundle_count(production: &serde_json::Value) -> usize {
    json_array_count(production.get("client"))
}

pub(crate) fn editor_production_summary_text(state: &serde_json::Value) -> String {
    let Some(production) = state.get("production") else {
        return "No production build attached.".to_string();
    };
    let mut lines = Vec::new();
    if let Some(build_dir) = production
        .get("build_dir")
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.is_empty())
    {
        lines.push(format!("build {build_dir}"));
    }
    for target in production
        .get("graph_contract")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
    {
        let kind = json_str_or_empty(target, "kind");
        let path = json_str_or_empty(target, "path");
        let hash = json_str_or_empty(target, "artifact_hash");
        let exists = target
            .get("exists")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);
        lines.push(format!(
            "Graph {kind} {path} (exists {exists}, hash {hash})"
        ));
    }
    for target in production
        .get("client")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
    {
        let kind = json_str_or_empty(target, "kind");
        let path = json_str_or_empty(target, "path");
        lines.push(format!("Client {kind} {path}"));
    }
    for target in production
        .get("preflight")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
    {
        let path = json_str_or_empty(target, "path");
        let commands = json_object_count(target.get("commands"));
        let required_env = json_array_count(target.get("required_env"));
        let optional_env = json_array_count(target.get("optional_env"));
        let route_count = json_array_count(target.get("routes"));
        let route_policies = production_preflight_route_policy_count(std::slice::from_ref(target));
        let smoke_summary_present = production_preflight_smoke_summary_present(target);
        let smoke_summary_missing_markers =
            production_preflight_smoke_summary_missing_marker_count_from_target(target);
        lines.push(format!(
            "Preflight {path} (commands {commands}, routes {route_count}, route_policies {route_policies}, required_env {required_env}, optional_env {optional_env}, smoke_summary_present {smoke_summary_present}, smoke_summary_missing_markers {smoke_summary_missing_markers})"
        ));
    }
    for target in production
        .get("db_adapters")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
    {
        let path = json_str_or_empty(target, "path");
        let adapters = json_array_count(target.get("adapters"));
        lines.push(format!("DB Adapters {path} ({adapters})"));
    }
    for target in production
        .get("commerce_adapters")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
    {
        let path = json_str_or_empty(target, "path");
        let adapters = json_array_count(target.get("adapters"));
        lines.push(format!("Commerce Adapters {path} ({adapters})"));
    }
    if lines.is_empty() {
        "No production contracts.".to_string()
    } else {
        lines.join("\n")
    }
}
