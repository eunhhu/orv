use super::*;

pub(crate) fn origin_map_max_contains_depth(origin_map: &orv_compiler::OriginMap) -> usize {
    let mut children: HashMap<String, Vec<String>> = HashMap::new();
    for edge in origin_map
        .edges
        .iter()
        .filter(|edge| edge.kind == "contains")
    {
        children
            .entry(edge.from.clone())
            .or_default()
            .push(edge.to.clone());
    }
    let mut memo = HashMap::new();
    origin_map
        .entries
        .iter()
        .map(|entry| origin_map_contains_depth(&entry.id, &children, &mut memo, &mut Vec::new()))
        .max()
        .unwrap_or(0)
}

pub(crate) fn origin_map_contains_depth(
    node: &str,
    children: &HashMap<String, Vec<String>>,
    memo: &mut HashMap<String, usize>,
    visiting: &mut Vec<String>,
) -> usize {
    if let Some(depth) = memo.get(node) {
        return *depth;
    }
    if visiting.iter().any(|visiting| visiting == node) {
        return 0;
    }
    visiting.push(node.to_string());
    let depth = children.get(node).map_or(0, |child_nodes| {
        child_nodes
            .iter()
            .map(|child| 1 + origin_map_contains_depth(child, children, memo, visiting))
            .max()
            .unwrap_or(0)
    });
    visiting.pop();
    memo.insert(node.to_string(), depth);
    depth
}

pub(crate) fn origin_response_ids_for_route(
    origin_map: &orv_compiler::OriginMap,
    route_origin_id: &str,
) -> Vec<String> {
    origin_map
        .edges
        .iter()
        .filter(|edge| edge.from == route_origin_id && edge.kind == "contains")
        .filter_map(|edge| {
            origin_map
                .entries
                .iter()
                .find(|entry| {
                    entry.id == edge.to && entry.kind == "domain" && entry.name == "respond"
                })
                .map(|entry| entry.id.clone())
        })
        .collect()
}

pub(crate) fn origin_entries_by_id(
    origin_map: &orv_compiler::OriginMap,
) -> HashMap<&str, &orv_compiler::OriginEntry> {
    origin_map
        .entries
        .iter()
        .map(|entry| (entry.id.as_str(), entry))
        .collect()
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct DeployRouteRevealSummaryCounts {
    pub(crate) routes: usize,
    pub(crate) native_targets: usize,
    pub(crate) native_routes: usize,
}

pub(crate) fn deploy_route_reveal_summary_counts(
    dir: &Path,
    origin_id: &str,
    origin_map: &orv_compiler::OriginMap,
    artifact: &orv_compiler::ServerRuntimeArtifact,
) -> anyhow::Result<DeployRouteRevealSummaryCounts> {
    let server_artifacts = [(SERVER_ARTIFACT_PATH.to_string(), artifact.clone())];
    let routes = reveal_routes(origin_id, origin_map, &server_artifacts);
    let native_targets = reveal_native_server_targets(dir, origin_id, origin_map)?;
    Ok(DeployRouteRevealSummaryCounts {
        routes: routes.len(),
        native_targets: native_targets.len(),
        native_routes: production_native_server_route_count(&native_targets),
    })
}

pub(crate) fn deploy_route_reveal_summary_requirements(
    path: &str,
    origin_ref: &str,
    summary: DeployRouteRevealSummaryCounts,
) -> Vec<String> {
    vec![
        format!(
            r#"orv_smoke_reveal_contains "reveal GET {path} route summary" "{origin_ref}" '"route_target_count": {}'"#,
            summary.routes
        ),
        format!(
            r#"orv_smoke_reveal_contains "reveal GET {path} native target summary" "{origin_ref}" '"native_server_target_count": {}'"#,
            summary.native_targets
        ),
        format!(
            r#"orv_smoke_reveal_contains "reveal GET {path} native route summary" "{origin_ref}" '"native_server_route_count": {}'"#,
            summary.native_routes
        ),
        format!(
            r#"orv_smoke_editor_reveal_contains "editor reveal GET {path} native target summary" "{origin_ref}" '"native_server_target_count": {}'"#,
            summary.native_targets
        ),
        format!(
            r#"orv_smoke_editor_reveal_contains "editor reveal GET {path} native route summary" "{origin_ref}" '"native_server_route_count": {}'"#,
            summary.native_routes
        ),
        format!(
            r#"orv_smoke_lsp_reveal_contains "lsp reveal GET {path} native target summary" "{origin_ref}" '"native_server_target_count": {}'"#,
            summary.native_targets
        ),
        format!(
            r#"orv_smoke_lsp_reveal_contains "lsp reveal GET {path} native route summary" "{origin_ref}" '"native_server_route_count": {}'"#,
            summary.native_routes
        ),
    ]
}

pub(crate) fn reveal_origin_json(dir: &Path, origin_id: &str) -> anyhow::Result<serde_json::Value> {
    let origin_map = read_origin_map(dir)?;
    let entry = origin_map
        .entries
        .iter()
        .find(|entry| entry.id == origin_id)
        .ok_or_else(|| anyhow::anyhow!("origin id `{origin_id}` not found"))?;
    let graph = read_json_value(&dir.join("project-graph.json"))?;
    let file_paths = graph_file_paths(&graph);
    let server_artifacts = read_server_artifacts(dir)?;
    let source_bundle = read_source_bundle_if_present(dir)?;
    let mut production = serde_json::json!({
        "graph_contract": editor_production_graph_contract_targets(dir)?,
        "routes": reveal_routes(origin_id, &origin_map, &server_artifacts),
        "native_server": reveal_native_server_targets(dir, origin_id, &origin_map)?,
        "preflight": reveal_preflight_targets(dir)?,
        "static": reveal_static_targets(dir, origin_id, &origin_map)?,
        "db_adapters": reveal_db_adapter_targets_for_origin(dir, origin_id, &origin_map)?,
        "commerce_adapters": reveal_commerce_adapter_targets_for_origin(dir, origin_id, &origin_map)?,
        "client": reveal_client_targets(dir, origin_id, entry, &origin_map)?,
    });
    let summary = production_summary_json(&production);
    production
        .as_object_mut()
        .expect("reveal production payload is object")
        .insert("summary".to_string(), summary);
    Ok(serde_json::json!({
        "schema_version": 1,
        "origin": entry,
        "source": reveal_source(entry, &file_paths, &server_artifacts, source_bundle.as_ref()),
        "project_graph": reveal_project_graph_node(&graph, origin_id),
        "production": production,
    }))
}

pub(crate) fn graph_file_paths(graph: &serde_json::Value) -> HashMap<u32, String> {
    let mut paths = HashMap::new();
    let Some(nodes) = graph.get("nodes").and_then(serde_json::Value::as_array) else {
        return paths;
    };
    for node in nodes {
        if node.get("kind").and_then(serde_json::Value::as_str) != Some("file") {
            continue;
        }
        let Some(file) = node.get("file").and_then(serde_json::Value::as_u64) else {
            continue;
        };
        let Some(path) = node.get("name").and_then(serde_json::Value::as_str) else {
            continue;
        };
        if let Ok(file) = u32::try_from(file) {
            paths.insert(file, path.to_string());
        }
    }
    paths
}

pub(crate) fn reveal_source(
    entry: &orv_compiler::OriginEntry,
    file_paths: &HashMap<u32, String>,
    server_artifacts: &[(String, orv_compiler::ServerRuntimeArtifact)],
    source_bundle: Option<&orv_compiler::SourceBundleArtifact>,
) -> serde_json::Value {
    let mut path = file_paths.get(&entry.span.file).cloned();
    let mut source = None;
    if let Ok(file_index) = usize::try_from(entry.span.file) {
        for (_, artifact) in server_artifacts {
            if let Some(file) = artifact.source_bundle.files.get(file_index) {
                path = Some(file.path.clone());
                source = Some(file.source.clone());
                break;
            }
        }
        if source.is_none() {
            if let Some(file) = source_bundle.and_then(|bundle| bundle.files.get(file_index)) {
                path = Some(file.path.clone());
                source = Some(file.source.clone());
            }
        }
    }
    if source.is_none() {
        if let Some(path) = &path {
            source = std::fs::read_to_string(path).ok();
        }
    }
    let snippet = source.as_deref().and_then(|source| {
        byte_snippet(source, entry.span.start, entry.span.end).map(ToString::to_string)
    });
    serde_json::json!({
        "file": entry.span.file,
        "path": path,
        "start": entry.span.start,
        "end": entry.span.end,
        "snippet": snippet,
        "content": source,
    })
}

pub(crate) fn byte_snippet(source: &str, start: u32, end: u32) -> Option<&str> {
    let start = usize::try_from(start).ok()?;
    let end = usize::try_from(end).ok()?;
    source.get(start..end)
}

pub(crate) fn reveal_project_graph_node(
    graph: &serde_json::Value,
    origin_id: &str,
) -> serde_json::Value {
    let Some(nodes) = graph.get("nodes").and_then(serde_json::Value::as_array) else {
        return serde_json::Value::Null;
    };
    let Some(links) = graph
        .get("semantic")
        .and_then(|semantic| semantic.get("origin_links"))
        .and_then(serde_json::Value::as_array)
    else {
        return serde_json::Value::Null;
    };
    let Some(link) = links
        .iter()
        .find(|link| link.get("origin_id").and_then(serde_json::Value::as_str) == Some(origin_id))
    else {
        return serde_json::Value::Null;
    };
    let Some(node_id) = link.get("node_id") else {
        return serde_json::Value::Null;
    };
    nodes
        .iter()
        .find(|node| node.get("id") == Some(node_id))
        .cloned()
        .unwrap_or(serde_json::Value::Null)
}

pub(crate) fn reveal_routes(
    origin_id: &str,
    origin_map: &orv_compiler::OriginMap,
    server_artifacts: &[(String, orv_compiler::ServerRuntimeArtifact)],
) -> Vec<serde_json::Value> {
    let mut routes = Vec::new();
    for (artifact_path, artifact) in server_artifacts {
        for (route, match_kind) in artifact.routes.iter().filter_map(|route| {
            origin_route_match_kind(origin_map, &route.origin_id, origin_id)
                .map(|match_kind| (route, match_kind))
        }) {
            routes.push(serde_json::json!({
                "artifact": artifact_path,
                "method": route.method,
                "path": route.path,
                "origin_id": route.origin_id,
                "match": match_kind,
                "matched_origin_id": origin_id,
                "policies": route.policies,
            }));
        }
    }
    routes
}

pub(crate) fn reveal_native_server_targets(
    dir: &Path,
    origin_id: &str,
    origin_map: &orv_compiler::OriginMap,
) -> anyhow::Result<Vec<serde_json::Value>> {
    let plan = read_json_value(&dir.join("bundle-plan.json"))?;
    let Some(bundles) = plan.get("bundles").and_then(serde_json::Value::as_array) else {
        return Ok(Vec::new());
    };
    let mut targets = Vec::new();
    for bundle in bundles {
        if bundle.get("kind").and_then(serde_json::Value::as_str) != Some("native_server_plan") {
            continue;
        }
        let path = json_str(bundle, "path", "bundle target")?;
        let target_path = dir.join(path);
        if !target_path.is_file() {
            continue;
        }
        let native_plan = read_json_value(&target_path)?;
        let matching_routes = native_plan
            .get("routes")
            .and_then(serde_json::Value::as_array)
            .map(|routes| {
                routes
                    .iter()
                    .filter(|route| {
                        route
                            .get("origin_id")
                            .and_then(serde_json::Value::as_str)
                            .is_some_and(|route_origin_id| {
                                origin_route_match_kind(origin_map, route_origin_id, origin_id)
                                    .is_some()
                            })
                    })
                    .cloned()
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        if matching_routes.is_empty() {
            continue;
        }
        targets.push(native_server_production_target_json(
            dir,
            path,
            &native_plan,
            serde_json::json!(matching_routes),
        )?);
    }
    Ok(targets)
}

pub(crate) fn origin_route_match_kind(
    origin_map: &orv_compiler::OriginMap,
    route_origin_id: &str,
    selected_origin_id: &str,
) -> Option<&'static str> {
    if route_origin_id == selected_origin_id {
        return Some("direct");
    }
    if origin_contains(origin_map, route_origin_id, selected_origin_id) {
        return Some("contains");
    }
    if origin_reaches_through_calls(origin_map, route_origin_id, selected_origin_id) {
        return Some("calls");
    }
    None
}

pub(crate) fn origin_contains(
    origin_map: &orv_compiler::OriginMap,
    ancestor_id: &str,
    descendant_id: &str,
) -> bool {
    if ancestor_id == descendant_id {
        return true;
    }
    let mut stack = vec![ancestor_id];
    let mut seen = HashSet::<&str>::new();
    while let Some(current) = stack.pop() {
        if !seen.insert(current) {
            continue;
        }
        for edge in origin_map
            .edges
            .iter()
            .filter(|edge| edge.kind == "contains" && edge.from == current)
        {
            if edge.to == descendant_id {
                return true;
            }
            stack.push(edge.to.as_str());
        }
    }
    false
}

pub(crate) fn origin_reaches_through_calls(
    origin_map: &orv_compiler::OriginMap,
    start_id: &str,
    target_id: &str,
) -> bool {
    if start_id == target_id {
        return true;
    }
    let mut stack = vec![start_id];
    let mut seen = HashSet::<&str>::new();
    while let Some(current) = stack.pop() {
        if !seen.insert(current) {
            continue;
        }
        for edge in origin_map.edges.iter().filter(|edge| {
            matches!(edge.kind.as_str(), "contains" | "calls") && edge.from == current
        }) {
            if edge.to == target_id {
                return true;
            }
            stack.push(edge.to.as_str());
        }
    }
    false
}

pub(crate) fn origin_is_html_projection_origin(
    origin_map: &orv_compiler::OriginMap,
    origin_id: &str,
) -> bool {
    origin_map.entries.iter().any(|entry| {
        entry.id == origin_id
            && entry.kind == "domain"
            && matches!(entry.name.as_str(), "html" | "out")
    }) || origin_map.entries.iter().any(|entry| {
        entry.kind == "domain"
            && entry.name == "html"
            && origin_contains(origin_map, &entry.id, origin_id)
    })
}

pub(crate) fn reveal_native_server_routes_source(
    dir: &Path,
    native_plan: &serde_json::Value,
) -> anyhow::Result<serde_json::Value> {
    let Some(path) = native_plan
        .get("routes_source")
        .and_then(serde_json::Value::as_str)
    else {
        return Ok(serde_json::Value::Null);
    };
    let target_path = dir.join(path);
    if !target_path.is_file() {
        return Ok(serde_json::json!({
            "path": path,
            "exists": false,
        }));
    }
    let source = std::fs::read_to_string(&target_path)
        .map_err(|e| anyhow::anyhow!("failed to read {}: {e}", target_path.display()))?;
    Ok(serde_json::json!({
        "path": path,
        "exists": true,
        "route_count": source.matches("OrvNativeRoute { method:").count(),
    }))
}

pub(crate) fn reveal_native_server_router_source(
    dir: &Path,
    native_plan: &serde_json::Value,
) -> anyhow::Result<serde_json::Value> {
    let Some(path) = native_plan
        .get("router_source")
        .and_then(serde_json::Value::as_str)
    else {
        return Ok(serde_json::Value::Null);
    };
    let target_path = dir.join(path);
    if !target_path.is_file() {
        return Ok(serde_json::json!({
            "path": path,
            "exists": false,
        }));
    }
    let source = std::fs::read_to_string(&target_path)
        .map_err(|e| anyhow::anyhow!("failed to read {}: {e}", target_path.display()))?;
    Ok(serde_json::json!({
        "path": path,
        "exists": true,
        "dispatch": source.contains("pub fn orv_native_dispatch("),
        "handler_count_contract": source.contains("ORV_NATIVE_HANDLER_COUNT"),
        "response_origin_dispatch": source.contains("pub response_origin_id: Option<&'static str>")
            && source.contains("response_origin_id: response.response_origin_id"),
    }))
}

pub(crate) fn reveal_native_server_handlers_source(
    dir: &Path,
    native_plan: &serde_json::Value,
) -> anyhow::Result<serde_json::Value> {
    let Some(path) = native_plan
        .get("handlers_source")
        .and_then(serde_json::Value::as_str)
    else {
        return Ok(serde_json::Value::Null);
    };
    let target_path = dir.join(path);
    if !target_path.is_file() {
        return Ok(serde_json::json!({
            "path": path,
            "exists": false,
        }));
    }
    let source = std::fs::read_to_string(&target_path)
        .map_err(|e| anyhow::anyhow!("failed to read {}: {e}", target_path.display()))?;
    Ok(serde_json::json!({
        "path": path,
        "exists": true,
        "handler_count_contract": source.contains("ORV_NATIVE_HANDLER_COUNT"),
        "body_lowering_placeholder": source.contains("native route body lowering pending"),
        "response_origin_dispatch": source.contains("pub response_origin_id: Option<&'static str>")
            && (source.contains("response_origin_id: route_match.route.response_origin_ids.first().copied()")
                || source.contains("response_origin_id: Some(")),
    }))
}

pub(crate) fn reveal_native_runtime_image_plan(
    dir: &Path,
    native_plan: &serde_json::Value,
) -> anyhow::Result<serde_json::Value> {
    let Some(path) = native_plan
        .get("runtime_image_plan")
        .and_then(serde_json::Value::as_str)
    else {
        return Ok(serde_json::Value::Null);
    };
    let target_path = dir.join(path);
    if !target_path.is_file() {
        return Ok(serde_json::json!({
            "path": path,
            "exists": false,
        }));
    }
    let image_plan = read_json_value(&target_path)?;
    Ok(serde_json::json!({
        "path": path,
        "exists": true,
        "kind": image_plan
            .get("kind")
            .cloned()
            .unwrap_or(serde_json::Value::Null),
        "status": image_plan
            .get("status")
            .cloned()
            .unwrap_or(serde_json::Value::Null),
        "artifact": image_plan
            .get("artifact")
            .cloned()
            .unwrap_or(serde_json::Value::Null),
        "reference_image": image_plan
            .get("reference_image")
            .cloned()
            .unwrap_or(serde_json::Value::Null),
        "target": image_plan
            .get("target")
            .cloned()
            .unwrap_or(serde_json::Value::Null),
        "runtime_features": image_plan
            .get("runtime_features")
            .cloned()
            .unwrap_or_else(|| serde_json::json!([])),
        "blocked_by": image_plan
            .get("blocked_by")
            .cloned()
            .unwrap_or_else(|| serde_json::json!([])),
    }))
}

pub(crate) fn reveal_commerce_adapter_targets(
    dir: &Path,
) -> anyhow::Result<Vec<serde_json::Value>> {
    reveal_commerce_adapter_targets_impl(dir, None, None)
}

pub(crate) fn reveal_commerce_adapter_targets_for_origin(
    dir: &Path,
    origin_id: &str,
    origin_map: &orv_compiler::OriginMap,
) -> anyhow::Result<Vec<serde_json::Value>> {
    reveal_commerce_adapter_targets_impl(dir, Some(origin_id), Some(origin_map))
}

pub(crate) fn reveal_commerce_adapter_targets_impl(
    dir: &Path,
    origin_id: Option<&str>,
    origin_map: Option<&orv_compiler::OriginMap>,
) -> anyhow::Result<Vec<serde_json::Value>> {
    let deploy_manifest_path = dir.join("deploy").join("manifest.json");
    if !deploy_manifest_path.is_file() {
        return Ok(Vec::new());
    }
    let deploy = read_json_value(&deploy_manifest_path)?;
    let Some(path) = deploy
        .get("server")
        .and_then(|server| server.get("commerce_adapters"))
        .and_then(serde_json::Value::as_str)
    else {
        return Ok(Vec::new());
    };
    let target_path = dir.join(path);
    if !target_path.is_file() {
        return Ok(vec![missing_adapter_reveal_target(
            "commerce_adapters",
            path,
            origin_id,
        )]);
    }
    let artifact = read_json_value(&target_path)?;
    let adapters = artifact
        .get("adapters")
        .cloned()
        .unwrap_or_else(|| serde_json::json!([]));
    let matched_adapters = origin_id
        .zip(origin_map)
        .map(|(origin_id, origin_map)| {
            reveal_adapter_origin_matches(&adapters, origin_id, origin_map)
        })
        .unwrap_or_default();
    Ok(vec![serde_json::json!({
        "kind": "commerce_adapters",
        "path": path,
        "exists": true,
        "selected_origin_id": origin_id,
        "matched": !matched_adapters.is_empty(),
        "matched_adapter_count": matched_adapters.len(),
        "artifact": artifact
            .get("artifact")
            .cloned()
            .unwrap_or(serde_json::Value::Null),
        "adapters": adapters,
        "source_reveal_commands": adapter_source_reveal_commands(dir, &adapters),
        "matched_adapters": matched_adapters,
    })])
}

pub(crate) fn reveal_db_adapter_targets(dir: &Path) -> anyhow::Result<Vec<serde_json::Value>> {
    reveal_db_adapter_targets_impl(dir, None, None)
}

pub(crate) fn reveal_db_adapter_targets_for_origin(
    dir: &Path,
    origin_id: &str,
    origin_map: &orv_compiler::OriginMap,
) -> anyhow::Result<Vec<serde_json::Value>> {
    reveal_db_adapter_targets_impl(dir, Some(origin_id), Some(origin_map))
}

pub(crate) fn reveal_db_adapter_targets_impl(
    dir: &Path,
    origin_id: Option<&str>,
    origin_map: Option<&orv_compiler::OriginMap>,
) -> anyhow::Result<Vec<serde_json::Value>> {
    let deploy_manifest_path = dir.join("deploy").join("manifest.json");
    if !deploy_manifest_path.is_file() {
        return Ok(Vec::new());
    }
    let deploy = read_json_value(&deploy_manifest_path)?;
    let Some(path) = deploy
        .get("server")
        .and_then(|server| server.get("db_adapters"))
        .and_then(serde_json::Value::as_str)
    else {
        return Ok(Vec::new());
    };
    let target_path = dir.join(path);
    if !target_path.is_file() {
        return Ok(vec![missing_adapter_reveal_target(
            "db_adapters",
            path,
            origin_id,
        )]);
    }
    let artifact = read_json_value(&target_path)?;
    let adapters = artifact
        .get("adapters")
        .cloned()
        .unwrap_or_else(|| serde_json::json!([]));
    let matched_adapters = origin_id
        .zip(origin_map)
        .map(|(origin_id, origin_map)| {
            reveal_adapter_origin_matches(&adapters, origin_id, origin_map)
        })
        .unwrap_or_default();
    Ok(vec![serde_json::json!({
        "kind": "db_adapters",
        "path": path,
        "exists": true,
        "selected_origin_id": origin_id,
        "matched": !matched_adapters.is_empty(),
        "matched_adapter_count": matched_adapters.len(),
        "artifact": artifact
            .get("artifact")
            .cloned()
            .unwrap_or(serde_json::Value::Null),
        "adapters": adapters,
        "source_reveal_commands": adapter_source_reveal_commands(dir, &adapters),
        "matched_adapters": matched_adapters,
    })])
}

pub(crate) fn missing_adapter_reveal_target(
    kind: &str,
    path: &str,
    origin_id: Option<&str>,
) -> serde_json::Value {
    serde_json::json!({
        "kind": kind,
        "path": path,
        "exists": false,
        "selected_origin_id": origin_id,
        "matched": false,
        "matched_adapter_count": 0,
        "artifact": serde_json::Value::Null,
        "adapters": [],
        "source_reveal_commands": [],
        "matched_adapters": [],
    })
}

pub(crate) fn adapter_source_reveal_commands(
    dir: &Path,
    adapters: &serde_json::Value,
) -> Vec<serde_json::Value> {
    let Some(adapters) = adapters.as_array() else {
        return Vec::new();
    };
    let build_dir = dir.display().to_string();
    adapters
        .iter()
        .enumerate()
        .flat_map(|(index, adapter)| {
            let build_dir = build_dir.clone();
            adapter_source_origin_ids(adapter)
                .into_iter()
                .map(move |origin_id| {
                    let command = editor_reveal_command_json(&build_dir, Some(&origin_id));
                    serde_json::json!({
                        "adapter_index": index,
                        "kind": adapter
                            .get("kind")
                            .cloned()
                            .unwrap_or(serde_json::Value::Null),
                        "provider": adapter
                            .get("provider")
                            .cloned()
                            .unwrap_or(serde_json::Value::Null),
                        "env": adapter
                            .get("env")
                            .cloned()
                            .unwrap_or(serde_json::Value::Null),
                        "endpoint": adapter
                            .get("endpoint")
                            .cloned()
                            .unwrap_or(serde_json::Value::Null),
                        "record_path": adapter
                            .get("record_path")
                            .cloned()
                            .unwrap_or(serde_json::Value::Null),
                        "source_origin_id": origin_id,
                        "command": command,
                    })
                })
        })
        .collect()
}

pub(crate) fn reveal_adapter_origin_matches(
    adapters: &serde_json::Value,
    origin_id: &str,
    origin_map: &orv_compiler::OriginMap,
) -> Vec<serde_json::Value> {
    let Some(adapters) = adapters.as_array() else {
        return Vec::new();
    };
    adapters
        .iter()
        .filter_map(|adapter| {
            let source_origin_ids = adapter_source_origin_ids(adapter);
            let match_kind = if source_origin_ids.iter().any(|source| source == origin_id) {
                "direct"
            } else if source_origin_ids
                .iter()
                .any(|source| origin_contains(origin_map, source, origin_id))
            {
                "source_contains_selected"
            } else if source_origin_ids
                .iter()
                .any(|source| origin_contains(origin_map, origin_id, source))
            {
                "selected_contains_source"
            } else {
                return None;
            };
            let mut value = adapter.clone();
            if let Some(adapter) = value.as_object_mut() {
                adapter.insert("match".to_string(), serde_json::json!(match_kind));
                adapter.insert(
                    "matched_origin_id".to_string(),
                    serde_json::json!(origin_id),
                );
            }
            Some(value)
        })
        .collect()
}

pub(crate) fn reveal_preflight_targets(dir: &Path) -> anyhow::Result<Vec<serde_json::Value>> {
    let deploy_manifest_path = dir.join("deploy").join("manifest.json");
    if !deploy_manifest_path.is_file() {
        return Ok(Vec::new());
    }
    let deploy = read_json_value(&deploy_manifest_path)?;
    let Some(path) = deploy
        .get("server")
        .and_then(|server| server.get("preflight"))
        .and_then(serde_json::Value::as_str)
    else {
        return Ok(Vec::new());
    };
    let target_path = dir.join(path);
    if !target_path.is_file() {
        return Ok(vec![serde_json::json!({
            "kind": "preflight",
            "path": path,
            "exists": false,
        })]);
    }
    let artifact = read_json_value(&target_path)?;
    let benchmark_evidence = reveal_benchmark_evidence_summary(dir, &artifact)?;
    Ok(vec![serde_json::json!({
        "kind": "preflight",
        "path": path,
        "exists": true,
        "artifact": artifact
            .get("artifact")
            .cloned()
            .unwrap_or(serde_json::Value::Null),
        "commands": artifact
            .get("commands")
            .cloned()
            .unwrap_or_else(|| serde_json::json!({})),
        "artifacts": artifact
            .get("artifacts")
            .cloned()
            .unwrap_or_else(|| serde_json::json!({})),
        "smoke_output_contract": artifact
            .get("smoke_output_contract")
            .cloned()
            .unwrap_or_else(|| {
                artifact
                    .pointer("/artifacts/smoke_output")
                    .and_then(serde_json::Value::as_str)
                    .map_or(serde_json::Value::Null, smoke_output_contract_value)
            }),
        "benchmark": artifact
            .get("benchmark")
            .cloned()
            .unwrap_or(serde_json::Value::Null),
        "benchmark_evidence": benchmark_evidence,
        "runtime_features": artifact
            .get("runtime_features")
            .cloned()
            .unwrap_or_else(|| serde_json::json!([])),
        "security_features": artifact
            .get("security_features")
            .cloned()
            .unwrap_or_else(|| serde_json::json!([])),
        "listen": artifact
            .get("listen")
            .cloned()
            .unwrap_or(serde_json::Value::Null),
        "routes": artifact
            .get("routes")
            .cloned()
            .unwrap_or_else(|| serde_json::json!([])),
        "required_env": artifact
            .get("required_env")
            .cloned()
            .unwrap_or_else(|| serde_json::json!([])),
        "optional_env": artifact
            .get("optional_env")
            .cloned()
            .unwrap_or_else(|| serde_json::json!([])),
    })])
}

pub(crate) fn reveal_static_targets(
    dir: &Path,
    origin_id: &str,
    origin_map: &orv_compiler::OriginMap,
) -> anyhow::Result<Vec<serde_json::Value>> {
    if !origin_is_html_projection_origin(origin_map, origin_id) {
        return Ok(Vec::new());
    }
    let plan = read_json_value(&dir.join("bundle-plan.json"))?;
    let Some(bundles) = plan.get("bundles").and_then(serde_json::Value::as_array) else {
        return Ok(Vec::new());
    };
    let mut targets = Vec::new();
    for bundle in bundles.iter().filter(|bundle| {
        bundle.get("kind").and_then(serde_json::Value::as_str) == Some("static_page")
    }) {
        let path = json_str(bundle, "path", "bundle target")?;
        let target_path = dir.join(path);
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

pub(crate) fn reveal_client_targets(
    dir: &Path,
    origin_id: &str,
    entry: &orv_compiler::OriginEntry,
    origin_map: &orv_compiler::OriginMap,
) -> anyhow::Result<Vec<serde_json::Value>> {
    if !matches!(entry.kind.as_str(), "signal" | "await")
        && !origin_is_html_projection_origin(origin_map, origin_id)
    {
        return Ok(Vec::new());
    }
    reveal_client_bundle_targets(dir)
}

pub(crate) fn reveal_client_bundle_targets(dir: &Path) -> anyhow::Result<Vec<serde_json::Value>> {
    let plan = read_json_value(&dir.join("bundle-plan.json"))?;
    let Some(bundles) = plan.get("bundles").and_then(serde_json::Value::as_array) else {
        return Ok(Vec::new());
    };
    let mut targets = Vec::new();
    for bundle in bundles {
        let kind = bundle
            .get("kind")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default();
        if !is_client_bundle_kind(kind) {
            continue;
        }
        let path = json_str(bundle, "path", "bundle target")?;
        let mut target = serde_json::json!({
            "kind": kind,
            "path": path,
            "exists": dir.join(path).is_file(),
            "runtime_features": bundle
                .get("runtime_features")
                .cloned()
                .unwrap_or_else(|| serde_json::json!([])),
        });
        if kind == "client_manifest" {
            add_client_manifest_reveal_fields(dir, path, &mut target)?;
        } else if kind == "client_reactive_plan" {
            add_client_reactive_plan_reveal_fields(dir, path, &mut target)?;
        }
        targets.push(target);
    }
    Ok(targets)
}

pub(crate) fn add_client_manifest_reveal_fields(
    dir: &Path,
    path: &str,
    target: &mut serde_json::Value,
) -> anyhow::Result<()> {
    let manifest_path = dir.join(path);
    if !manifest_path.is_file() {
        return Ok(());
    }
    let manifest = read_json_value(&manifest_path)?;
    target["source_bundle"] = manifest
        .get("source_bundle")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    target["source_bundle_hash"] = manifest
        .get("source_bundle_hash")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    target["wasm_hash"] = manifest
        .get("wasm_hash")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    target["exports"] = manifest
        .get("exports")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    target["capabilities"] = manifest
        .get("capabilities")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    target["blocked_by"] = manifest
        .get("blocked_by")
        .cloned()
        .unwrap_or_else(|| serde_json::json!([]));
    target["blockers"] = manifest
        .get("blockers")
        .cloned()
        .unwrap_or_else(|| serde_json::json!([]));
    Ok(())
}

pub(crate) fn add_client_reactive_plan_reveal_fields(
    dir: &Path,
    path: &str,
    target: &mut serde_json::Value,
) -> anyhow::Result<()> {
    let plan_path = dir.join(path);
    if !plan_path.is_file() {
        return Ok(());
    }
    let plan = read_json_value(&plan_path)?;
    target["source_bundle"] = plan
        .get("source_bundle")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    target["source_bundle_hash"] = plan
        .get("source_bundle_hash")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    target["signal_count"] = plan
        .get("signals")
        .and_then(serde_json::Value::as_array)
        .map_or_else(
            || serde_json::json!(0),
            |signals| serde_json::json!(signals.len()),
        );
    target["blocked_by"] = plan
        .get("blocked_by")
        .cloned()
        .unwrap_or_else(|| serde_json::json!([]));
    target["blockers"] = plan
        .get("blockers")
        .cloned()
        .unwrap_or_else(|| serde_json::json!([]));
    Ok(())
}

pub(crate) fn deploy_smoke_client_reveal_origin(
    origin_map: &orv_compiler::OriginMap,
) -> Option<&str> {
    origin_map
        .entries
        .iter()
        .find(|entry| matches!(entry.kind.as_str(), "signal" | "await"))
        .or_else(|| {
            origin_map
                .entries
                .iter()
                .find(|entry| entry.kind == "domain" && entry.name == "html")
        })
        .map(|entry| entry.id.as_str())
}

pub(crate) fn deploy_smoke_reveal_marker_contract_section(origin_ref: &str) -> String {
    format!(
        r#"orv_smoke_reveal_contains "reveal smoke required markers" "{origin_ref}" '"smoke_test_required_markers": ['
orv_smoke_reveal_contains "reveal smoke summary required markers" "{origin_ref}" '"required_markers": ['
orv_smoke_reveal_contains "reveal smoke marker dap source bundle" "{origin_ref}" '"dap_source_bundle"'
orv_smoke_editor_reveal_contains "editor reveal smoke required markers" "{origin_ref}" '"smoke_test_required_markers": ['
orv_smoke_editor_reveal_contains "editor reveal smoke summary required markers" "{origin_ref}" '"required_markers": ['
orv_smoke_editor_reveal_contains "editor reveal smoke marker dap source bundle" "{origin_ref}" '"dap_source_bundle"'
orv_smoke_lsp_reveal_contains "lsp reveal smoke required markers" "{origin_ref}" '"smoke_test_required_markers": ['
orv_smoke_lsp_reveal_contains "lsp reveal smoke summary required markers" "{origin_ref}" '"required_markers": ['
orv_smoke_lsp_reveal_contains "lsp reveal smoke marker dap source bundle" "{origin_ref}" '"dap_source_bundle"'

"#
    )
}

pub(crate) fn deploy_smoke_client_reveal_section(
    out: &Path,
    client: &serde_json::Value,
) -> anyhow::Result<String> {
    if client.is_null() {
        return Ok(String::new());
    }
    let manifest = json_str_or_empty(client, "manifest");
    let summary = deploy_client_summary_counts(out)?;
    let target_count = summary.targets;
    let manifest_count = summary.manifests;
    let capability_surface_count = summary.capability_surfaces;
    Ok(format!(
        r#"orv_smoke_reveal_contains "reveal client target summary" "$ORV_SMOKE_CLIENT_ORIGIN" '"client_target_count": {target_count}'
orv_smoke_reveal_contains "reveal client manifest summary" "$ORV_SMOKE_CLIENT_ORIGIN" '"client_manifest_count": {manifest_count}'
orv_smoke_reveal_contains "reveal client capability summary" "$ORV_SMOKE_CLIENT_ORIGIN" '"client_capability_surface_count": {capability_surface_count}'
orv_smoke_reveal_contains "reveal client manifest target" "$ORV_SMOKE_CLIENT_ORIGIN" '"path": "{manifest}"'
orv_smoke_editor_reveal_contains "editor reveal client target summary" "$ORV_SMOKE_CLIENT_ORIGIN" '"client_target_count": {target_count}'
orv_smoke_editor_reveal_contains "editor reveal client manifest summary" "$ORV_SMOKE_CLIENT_ORIGIN" '"client_manifest_count": {manifest_count}'
orv_smoke_editor_reveal_contains "editor reveal client capability summary" "$ORV_SMOKE_CLIENT_ORIGIN" '"client_capability_surface_count": {capability_surface_count}'
orv_smoke_lsp_reveal_contains "lsp reveal client target summary" "$ORV_SMOKE_CLIENT_ORIGIN" '"client_target_count": {target_count}'
orv_smoke_lsp_reveal_contains "lsp reveal client manifest summary" "$ORV_SMOKE_CLIENT_ORIGIN" '"client_manifest_count": {manifest_count}'
orv_smoke_lsp_reveal_contains "lsp reveal client capability summary" "$ORV_SMOKE_CLIENT_ORIGIN" '"client_capability_surface_count": {capability_surface_count}'
orv_smoke_dap_summary_contains "dap client target summary" '"client_target_count": {target_count}'
orv_smoke_dap_summary_contains "dap client manifest summary" '"client_manifest_count": {manifest_count}'
orv_smoke_dap_summary_contains "dap client capability summary" '"client_capability_surface_count": {capability_surface_count}'

"#
    ))
}
