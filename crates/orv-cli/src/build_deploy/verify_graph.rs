use super::*;

pub(crate) fn verify_origin_map_contract(
    origin_map: &orv_compiler::OriginMap,
) -> anyhow::Result<()> {
    if origin_map.version != orv_compiler::ORIGIN_MAP_VERSION {
        anyhow::bail!(
            "origin-map.json version must be {}",
            orv_compiler::ORIGIN_MAP_VERSION
        );
    }
    let mut ids = HashSet::new();
    for entry in &origin_map.entries {
        if entry.id.trim().is_empty() {
            anyhow::bail!("origin-map.json contains entry with empty id");
        }
        if !ids.insert(entry.id.as_str()) {
            let id = &entry.id;
            anyhow::bail!("origin-map.json contains duplicate entry id `{id}`");
        }
        if entry.kind.trim().is_empty() {
            let id = &entry.id;
            anyhow::bail!("origin-map.json entry `{id}` has empty kind");
        }
        if entry.name.trim().is_empty() {
            let id = &entry.id;
            anyhow::bail!("origin-map.json entry `{id}` has empty name");
        }
        if entry.span.start > entry.span.end {
            let id = &entry.id;
            anyhow::bail!("origin-map.json entry `{id}` has invalid span");
        }
        let span = Span::new(
            FileId(entry.span.file),
            ByteRange::new(entry.span.start, entry.span.end),
        );
        let expected_fingerprint = orv_hir::origin_fingerprint(&entry.kind, &entry.name, span);
        if entry.fingerprint != expected_fingerprint {
            let id = &entry.id;
            anyhow::bail!("origin-map.json entry `{id}` fingerprint does not match span");
        }
        let expected_id = orv_hir::origin_id(&entry.kind, &entry.name, span);
        if entry.id != expected_id {
            let id = &entry.id;
            anyhow::bail!("origin-map.json entry `{id}` id does not match fingerprint");
        }
    }
    let mut edge_keys = HashSet::new();
    for edge in &origin_map.edges {
        if edge.kind.trim().is_empty() {
            anyhow::bail!("origin-map.json contains edge with empty kind");
        }
        if !matches!(edge.kind.as_str(), "contains" | "calls") {
            let kind = &edge.kind;
            anyhow::bail!("origin-map.json edge kind `{kind}` is not supported");
        }
        if !edge_keys.insert((edge.from.as_str(), edge.to.as_str(), edge.kind.as_str())) {
            let from = &edge.from;
            let to = &edge.to;
            let kind = &edge.kind;
            anyhow::bail!("origin-map.json contains duplicate edge `{from}` -> `{to}` ({kind})");
        }
        if !ids.contains(edge.from.as_str()) {
            let from = &edge.from;
            anyhow::bail!("origin-map.json edge from `{from}` does not reference an entry");
        }
        if !ids.contains(edge.to.as_str()) {
            let to = &edge.to;
            anyhow::bail!("origin-map.json edge to `{to}` does not reference an entry");
        }
    }
    Ok(())
}

pub(crate) fn verify_origin_map_source_spans(
    origin_map: &orv_compiler::OriginMap,
    source_bundle: &orv_compiler::SourceBundleArtifact,
) -> anyhow::Result<()> {
    for entry in &origin_map.entries {
        let file_index = usize::try_from(entry.span.file)
            .map_err(|_| anyhow::anyhow!("origin-map.json entry span file is too large"))?;
        if file_index >= source_bundle.files.len() {
            let id = &entry.id;
            let file = entry.span.file;
            anyhow::bail!(
                "origin-map.json entry `{id}` span.file {file} does not reference source-bundle file"
            );
        }
        let source_len = source_bundle.files[file_index].source.len();
        let span_end = usize::try_from(entry.span.end)
            .map_err(|_| anyhow::anyhow!("origin-map.json entry span end is too large"))?;
        if span_end > source_len {
            let id = &entry.id;
            anyhow::bail!(
                "origin-map.json entry `{id}` span.end exceeds source-bundle file length"
            );
        }
    }
    Ok(())
}

pub(crate) fn verify_project_graph_contract(
    dir: &Path,
    origin_map: &orv_compiler::OriginMap,
    source_bundle: &orv_compiler::SourceBundleArtifact,
) -> anyhow::Result<()> {
    let graph = read_json_value(&dir.join("project-graph.json"))?;
    verify_json_object_keys_exact(
        &graph,
        &["schema_version", "stats", "nodes", "edges", "semantic"],
        "project-graph.json",
    )?;
    if graph
        .get("schema_version")
        .and_then(serde_json::Value::as_u64)
        != Some(1)
    {
        anyhow::bail!("project-graph.json schema_version must be 1");
    }
    let nodes = graph
        .get("nodes")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| anyhow::anyhow!("project-graph.json nodes must be an array"))?;
    let edges = graph
        .get("edges")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| anyhow::anyhow!("project-graph.json edges must be an array"))?;
    verify_project_graph_stats(&graph, nodes, edges, origin_map)?;
    let semantic = graph
        .get("semantic")
        .ok_or_else(|| anyhow::anyhow!("project-graph.json semantic must be an object"))?;
    verify_json_object_keys_exact(
        semantic,
        &["origin_map", "origin_edges", "origin_links"],
        "project-graph.json semantic",
    )?;
    let semantic_origin_map = graph
        .pointer("/semantic/origin_map")
        .ok_or_else(|| anyhow::anyhow!("project-graph.json semantic.origin_map is missing"))?;
    if semantic_origin_map != &serde_json::to_value(origin_map)? {
        anyhow::bail!("project-graph.json semantic origin_map does not match origin-map.json");
    }
    let semantic_origin_edges = graph
        .pointer("/semantic/origin_edges")
        .ok_or_else(|| anyhow::anyhow!("project-graph.json semantic.origin_edges is missing"))?;
    if semantic_origin_edges != &serde_json::Value::Array(origin_edges(origin_map)) {
        anyhow::bail!("project-graph.json semantic origin_edges do not match origin-map.json");
    }
    let node_ids = verify_project_graph_nodes(nodes, source_bundle)?;
    verify_project_graph_edges(edges, &node_ids)?;
    verify_project_graph_origin_links(&graph, nodes, origin_map, &node_ids)?;
    Ok(())
}

pub(crate) fn verify_project_graph_stats(
    graph: &serde_json::Value,
    nodes: &[serde_json::Value],
    edges: &[serde_json::Value],
    origin_map: &orv_compiler::OriginMap,
) -> anyhow::Result<()> {
    let stats = graph
        .get("stats")
        .ok_or_else(|| anyhow::anyhow!("project-graph.json stats is missing"))?;
    verify_json_object_keys_exact(
        stats,
        &[
            "node_count",
            "edge_count",
            "file_count",
            "import_count",
            "declaration_count",
            "domain_count",
            "max_source_contains_depth",
            "semantic_origin_count",
            "semantic_edge_count",
            "semantic_call_edge_count",
            "max_semantic_contains_depth",
        ],
        "project-graph.json stats",
    )?;
    for key in stats
        .as_object()
        .expect("project graph stats verified as object")
        .keys()
    {
        if !stats.get(key).is_some_and(serde_json::Value::is_u64) {
            anyhow::bail!("project-graph.json stats.{key} must be an unsigned integer");
        }
    }
    verify_project_graph_stat(stats, "node_count", nodes.len())?;
    verify_project_graph_stat(stats, "edge_count", edges.len())?;
    verify_project_graph_stat(
        stats,
        "file_count",
        project_graph_node_kind_count(nodes, &["file"]),
    )?;
    verify_project_graph_stat(
        stats,
        "import_count",
        project_graph_node_kind_count(nodes, &["import"]),
    )?;
    verify_project_graph_stat(
        stats,
        "declaration_count",
        project_graph_node_kind_count(
            nodes,
            &["struct", "enum", "type_alias", "function", "define"],
        ),
    )?;
    verify_project_graph_stat(
        stats,
        "domain_count",
        project_graph_node_kind_count(nodes, &["domain"]),
    )?;
    verify_project_graph_stat(
        stats,
        "max_source_contains_depth",
        project_graph_max_contains_depth(nodes, edges)?,
    )?;
    verify_project_graph_stat(stats, "semantic_origin_count", origin_map.entries.len())?;
    verify_project_graph_stat(stats, "semantic_edge_count", origin_map.edges.len())?;
    let call_edges = origin_map
        .edges
        .iter()
        .filter(|edge| edge.kind == "calls")
        .count();
    verify_project_graph_stat(stats, "semantic_call_edge_count", call_edges)?;
    verify_project_graph_stat(
        stats,
        "max_semantic_contains_depth",
        origin_map_max_contains_depth(origin_map),
    )?;
    Ok(())
}

pub(crate) fn verify_project_graph_stat(
    stats: &serde_json::Value,
    key: &str,
    expected: usize,
) -> anyhow::Result<()> {
    let actual = stats
        .get(key)
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| anyhow::anyhow!("project-graph.json stats.{key} must be an integer"))?;
    if actual != expected as u64 {
        anyhow::bail!("project-graph.json stats.{key} does not match graph content");
    }
    Ok(())
}

pub(crate) fn verify_project_graph_nodes(
    nodes: &[serde_json::Value],
    source_bundle: &orv_compiler::SourceBundleArtifact,
) -> anyhow::Result<HashSet<u64>> {
    let mut node_ids = HashSet::new();
    let mut file_paths = HashSet::new();
    let source_file_count = u64::try_from(source_bundle.files.len()).unwrap_or(u64::MAX);
    for node in nodes {
        verify_json_object_keys_exact(
            node,
            &["id", "kind", "name", "file", "span"],
            "project graph node",
        )?;
        let id = node
            .get("id")
            .and_then(serde_json::Value::as_u64)
            .ok_or_else(|| anyhow::anyhow!("project-graph.json node id must be an integer"))?;
        if !node_ids.insert(id) {
            anyhow::bail!("project-graph.json contains duplicate node id {id}");
        }
        let kind = json_str(node, "kind", "project graph node")?;
        let name = json_str(node, "name", "project graph node")?;
        if !matches!(
            kind,
            "file" | "import" | "struct" | "enum" | "type_alias" | "function" | "define" | "domain"
        ) {
            anyhow::bail!("project graph node kind {kind} is not supported");
        }
        let file_id = node
            .get("file")
            .and_then(serde_json::Value::as_u64)
            .ok_or_else(|| anyhow::anyhow!("project graph node file must be an integer"))?;
        if file_id >= source_file_count {
            anyhow::bail!(
                "project graph node file {file_id} does not reference source-bundle file"
            );
        }
        let span = node
            .get("span")
            .ok_or_else(|| anyhow::anyhow!("project graph node span must be an object"))?;
        verify_json_object_keys_exact(span, &["file", "start", "end"], "project graph node span")?;
        for key in ["file", "start", "end"] {
            if !span.get(key).is_some_and(serde_json::Value::is_u64) {
                anyhow::bail!("project graph node span.{key} must be an integer");
            }
        }
        let span_file = span
            .get("file")
            .and_then(serde_json::Value::as_u64)
            .expect("project graph span.file verified as integer");
        if span_file != file_id {
            anyhow::bail!("project graph node span.file must match node file");
        }
        let span_start = span
            .get("start")
            .and_then(serde_json::Value::as_u64)
            .expect("project graph span.start verified as integer");
        let span_end = span
            .get("end")
            .and_then(serde_json::Value::as_u64)
            .expect("project graph span.end verified as integer");
        if span_start > span_end {
            anyhow::bail!("project graph node span.start must be <= span.end");
        }
        let file_index = usize::try_from(file_id)
            .map_err(|_| anyhow::anyhow!("project graph node file id is too large"))?;
        let source_len =
            u64::try_from(source_bundle.files[file_index].source.len()).unwrap_or(u64::MAX);
        if span_end > source_len {
            anyhow::bail!("project graph node span.end exceeds source-bundle file length");
        }
        if kind == "file" {
            let actual_path = normalized_artifact_path(name);
            let expected_path = normalized_artifact_path(&source_bundle.files[file_index].path);
            if actual_path != expected_path {
                anyhow::bail!("project graph file node name must match source-bundle file path");
            }
            file_paths.insert(actual_path);
        }
    }
    for file in &source_bundle.files {
        let path = normalized_artifact_path(&file.path);
        if !file_paths.contains(&path) {
            anyhow::bail!("project-graph.json is missing source-bundle file node {path}");
        }
    }
    if file_paths.len() != source_bundle.files.len() {
        anyhow::bail!("project-graph.json file nodes do not match source-bundle files");
    }
    Ok(node_ids)
}

pub(crate) fn verify_project_graph_edges(
    edges: &[serde_json::Value],
    node_ids: &HashSet<u64>,
) -> anyhow::Result<()> {
    for edge in edges {
        verify_json_object_keys_exact(edge, &["from", "to", "kind"], "project graph edge")?;
        let from = edge
            .get("from")
            .and_then(serde_json::Value::as_u64)
            .ok_or_else(|| anyhow::anyhow!("project-graph.json edge from must be an integer"))?;
        let to = edge
            .get("to")
            .and_then(serde_json::Value::as_u64)
            .ok_or_else(|| anyhow::anyhow!("project-graph.json edge to must be an integer"))?;
        if !node_ids.contains(&from) {
            anyhow::bail!("project-graph.json edge from {from} does not reference a node");
        }
        if !node_ids.contains(&to) {
            anyhow::bail!("project-graph.json edge to {to} does not reference a node");
        }
        let kind = json_str(edge, "kind", "project graph edge")?;
        if !matches!(kind, "contains" | "imports") {
            anyhow::bail!("project graph edge kind {kind} is not supported");
        }
    }
    Ok(())
}

pub(crate) fn verify_project_graph_origin_links(
    graph: &serde_json::Value,
    nodes: &[serde_json::Value],
    origin_map: &orv_compiler::OriginMap,
    node_ids: &HashSet<u64>,
) -> anyhow::Result<()> {
    let origin_links = graph
        .pointer("/semantic/origin_links")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| {
            anyhow::anyhow!("project-graph.json semantic.origin_links must be an array")
        })?;
    let origin_ids = origin_map
        .entries
        .iter()
        .map(|entry| entry.id.as_str())
        .collect::<HashSet<_>>();
    for link in origin_links {
        verify_json_object_keys_exact(
            link,
            &["kind", "origin_id", "node_id"],
            "project graph origin link",
        )?;
        if json_str(link, "kind", "project graph origin link")? != "source_node" {
            anyhow::bail!("project graph origin link kind must be source_node");
        }
        let origin_id = json_str(link, "origin_id", "project graph origin link")?;
        if !origin_ids.contains(origin_id) {
            anyhow::bail!(
                "project-graph.json origin link `{origin_id}` does not reference origin-map.json"
            );
        }
        let node_id = link
            .get("node_id")
            .and_then(serde_json::Value::as_u64)
            .ok_or_else(|| {
                anyhow::anyhow!("project-graph.json origin link node_id must be an integer")
            })?;
        if !node_ids.contains(&node_id) {
            anyhow::bail!(
                "project-graph.json origin link node_id {node_id} does not reference a node"
            );
        }
    }
    let expected = expected_project_graph_origin_links(nodes, origin_map)?;
    if origin_links != &expected {
        anyhow::bail!(
            "project-graph.json semantic origin_links do not match graph nodes and origin-map.json"
        );
    }
    Ok(())
}

pub(crate) fn verify_server_runtime_origin_contract(
    artifact: &orv_compiler::ServerRuntimeArtifact,
    origin_map: &orv_compiler::OriginMap,
) -> anyhow::Result<()> {
    let entries_by_id: HashMap<&str, &orv_compiler::OriginEntry> = origin_map
        .entries
        .iter()
        .map(|entry| (entry.id.as_str(), entry))
        .collect();
    let contains_edges: HashSet<(&str, &str)> = origin_map
        .edges
        .iter()
        .filter(|edge| edge.kind == "contains")
        .map(|edge| (edge.from.as_str(), edge.to.as_str()))
        .collect();
    if let Some(listen) = &artifact.listen {
        let Some(entry) = entries_by_id.get(listen.origin_id.as_str()).copied() else {
            let origin_id = &listen.origin_id;
            anyhow::bail!("server listen origin_id `{origin_id}` not found in origin-map.json");
        };
        if entry.kind != "listen" {
            let origin_id = &listen.origin_id;
            anyhow::bail!("server listen origin_id `{origin_id}` must reference origin-map listen");
        }
        if entry.name != listen.name {
            let origin_id = &listen.origin_id;
            anyhow::bail!("server listen origin_id `{origin_id}` name does not match origin-map");
        }
    }
    let mut route_ids = HashSet::new();
    for route in &artifact.routes {
        if !route_ids.insert(route.origin_id.as_str()) {
            let origin_id = &route.origin_id;
            anyhow::bail!("server route origin_id `{origin_id}` is duplicated");
        }
        let Some(entry) = entries_by_id.get(route.origin_id.as_str()).copied() else {
            let origin_id = &route.origin_id;
            anyhow::bail!(
                "server route {} {} origin_id `{origin_id}` not found in origin-map.json",
                route.method,
                route.path
            );
        };
        if entry.kind != "route" {
            let origin_id = &route.origin_id;
            anyhow::bail!(
                "server route {} {} origin_id `{origin_id}` must reference origin-map route",
                route.method,
                route.path
            );
        }
        let expected_name = format!("{} {}", route.method, route.path);
        if entry.name != expected_name {
            let origin_id = &route.origin_id;
            anyhow::bail!(
                "server route {} {} origin_id `{origin_id}` name does not match origin-map",
                route.method,
                route.path
            );
        }
        let expected_response_origin_ids =
            origin_response_ids_for_route(origin_map, &route.origin_id);
        if route.response_origin_ids != expected_response_origin_ids {
            anyhow::bail!(
                "server route {} {} response_origin_ids do not match origin-map contains edges",
                route.method,
                route.path
            );
        }
        let mut response_ids = HashSet::new();
        for response_origin_id in &route.response_origin_ids {
            if !response_ids.insert(response_origin_id.as_str()) {
                anyhow::bail!(
                    "server route {} {} response_origin_id `{response_origin_id}` is duplicated",
                    route.method,
                    route.path
                );
            }
            verify_route_response_origin(
                route,
                response_origin_id,
                &entries_by_id,
                &contains_edges,
            )?;
        }
        for response in &route.responses {
            if !route
                .response_origin_ids
                .iter()
                .any(|origin_id| origin_id == &response.origin_id)
            {
                let origin_id = &response.origin_id;
                anyhow::bail!(
                    "server route {} {} response descriptor `{origin_id}` is missing from response_origin_ids",
                    route.method,
                    route.path
                );
            }
            verify_route_response_origin(
                route,
                &response.origin_id,
                &entries_by_id,
                &contains_edges,
            )?;
        }
        for policy in &route.policies {
            verify_route_policy_origin(route, policy, &entries_by_id, &contains_edges)?;
        }
    }
    Ok(())
}

pub(crate) fn verify_route_response_origin(
    route: &orv_compiler::ServerRouteArtifact,
    response_origin_id: &str,
    entries_by_id: &HashMap<&str, &orv_compiler::OriginEntry>,
    contains_edges: &HashSet<(&str, &str)>,
) -> anyhow::Result<()> {
    let Some(entry) = entries_by_id.get(response_origin_id).copied() else {
        anyhow::bail!(
            "server route {} {} response_origin_id `{response_origin_id}` not found in origin-map.json",
            route.method,
            route.path
        );
    };
    if entry.kind != "domain" || entry.name != "respond" {
        anyhow::bail!(
            "server route {} {} response_origin_id `{response_origin_id}` must reference origin-map respond domain",
            route.method,
            route.path
        );
    }
    if !contains_edges.contains(&(route.origin_id.as_str(), response_origin_id)) {
        anyhow::bail!(
            "server route {} {} response_origin_id `{response_origin_id}` is not contained by route origin",
            route.method,
            route.path
        );
    }
    Ok(())
}

pub(crate) fn verify_route_policy_origin(
    route: &orv_compiler::ServerRouteArtifact,
    policy: &orv_compiler::ServerRoutePolicyArtifact,
    entries_by_id: &HashMap<&str, &orv_compiler::OriginEntry>,
    contains_edges: &HashSet<(&str, &str)>,
) -> anyhow::Result<()> {
    let Some(policy_origin_id) = policy.origin_id.as_deref() else {
        return Ok(());
    };
    let Some(entry) = entries_by_id.get(policy_origin_id).copied() else {
        anyhow::bail!(
            "server route {} {} policy `{}` origin_id `{policy_origin_id}` not found in origin-map.json",
            route.method,
            route.path,
            policy.kind
        );
    };
    let expected_domain = match policy.kind.as_str() {
        "auth" => "Auth",
        "csrf" => "csrf",
        "session" => "session",
        _ => return Ok(()),
    };
    if entry.kind != "domain" || entry.name != expected_domain {
        anyhow::bail!(
            "server route {} {} policy `{}` origin_id `{policy_origin_id}` must reference origin-map {expected_domain} domain",
            route.method,
            route.path,
            policy.kind
        );
    }
    if !contains_edges.contains(&(route.origin_id.as_str(), policy_origin_id)) {
        anyhow::bail!(
            "server route {} {} policy `{}` origin_id `{policy_origin_id}` is not contained by route origin",
            route.method,
            route.path,
            policy.kind
        );
    }
    Ok(())
}

pub(crate) fn verify_origin_map_json_keys(value: &serde_json::Value) -> anyhow::Result<()> {
    verify_json_object_keys_exact(value, &["version", "entries", "edges"], "origin-map.json")?;
    if value.get("version").and_then(serde_json::Value::as_u64)
        != Some(u64::from(orv_compiler::ORIGIN_MAP_VERSION))
    {
        anyhow::bail!(
            "origin-map.json version must be {}",
            orv_compiler::ORIGIN_MAP_VERSION
        );
    }
    let entries = value
        .get("entries")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| anyhow::anyhow!("origin-map.json entries must be an array"))?;
    for (index, entry) in entries.iter().enumerate() {
        verify_json_object_keys_exact(
            entry,
            &["id", "kind", "name", "span", "fingerprint"],
            &format!("origin-map.json entries[{index}]"),
        )?;
        for key in ["id", "kind", "name", "fingerprint"] {
            if !entry.get(key).is_some_and(serde_json::Value::is_string) {
                anyhow::bail!("origin-map.json entries[{index}].{key} must be a string");
            }
        }
        let span = entry
            .get("span")
            .ok_or_else(|| anyhow::anyhow!("origin-map.json entries[{index}].span is missing"))?;
        verify_json_object_keys_exact(
            span,
            &["file", "start", "end"],
            &format!("origin-map.json entries[{index}].span"),
        )?;
        for key in ["file", "start", "end"] {
            if !span.get(key).is_some_and(serde_json::Value::is_u64) {
                anyhow::bail!(
                    "origin-map.json entries[{index}].span.{key} must be an unsigned integer"
                );
            }
        }
    }
    let edges = value
        .get("edges")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| anyhow::anyhow!("origin-map.json edges must be an array"))?;
    for (index, edge) in edges.iter().enumerate() {
        verify_json_object_keys_exact(
            edge,
            &["from", "to", "kind"],
            &format!("origin-map.json edges[{index}]"),
        )?;
        for key in ["from", "to", "kind"] {
            if !edge.get(key).is_some_and(serde_json::Value::is_string) {
                anyhow::bail!("origin-map.json edges[{index}].{key} must be a string");
            }
        }
    }
    Ok(())
}
