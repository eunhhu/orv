use super::*;

pub(crate) fn cmd_verify_build(dir: &Path) -> anyhow::Result<()> {
    verify_build_dir(dir)?;
    verify_build_recorded_benchmark_evidence_artifacts(dir)?;
    println!("build: {} verified", dir.display());
    Ok(())
}

pub(crate) fn project_graph_node_kind_count(nodes: &[serde_json::Value], kinds: &[&str]) -> usize {
    nodes
        .iter()
        .filter(|node| {
            node.get("kind")
                .and_then(serde_json::Value::as_str)
                .is_some_and(|kind| kinds.contains(&kind))
        })
        .count()
}

pub(crate) fn project_graph_max_contains_depth(
    nodes: &[serde_json::Value],
    edges: &[serde_json::Value],
) -> anyhow::Result<usize> {
    let mut children: HashMap<u64, Vec<u64>> = HashMap::new();
    for edge in edges {
        if json_str(edge, "kind", "project graph edge")? != "contains" {
            continue;
        }
        let from = edge
            .get("from")
            .and_then(serde_json::Value::as_u64)
            .ok_or_else(|| anyhow::anyhow!("project-graph.json edge from must be an integer"))?;
        let to = edge
            .get("to")
            .and_then(serde_json::Value::as_u64)
            .ok_or_else(|| anyhow::anyhow!("project-graph.json edge to must be an integer"))?;
        children.entry(from).or_default().push(to);
    }
    let mut memo = HashMap::new();
    nodes
        .iter()
        .map(|node| {
            let id = node
                .get("id")
                .and_then(serde_json::Value::as_u64)
                .ok_or_else(|| anyhow::anyhow!("project-graph.json node id must be an integer"))?;
            Ok(project_graph_contains_depth(
                id,
                &children,
                &mut memo,
                &mut Vec::new(),
            ))
        })
        .collect::<anyhow::Result<Vec<_>>>()
        .map(|depths| depths.into_iter().max().unwrap_or(0))
}

pub(crate) fn project_graph_contains_depth(
    node: u64,
    children: &HashMap<u64, Vec<u64>>,
    memo: &mut HashMap<u64, usize>,
    visiting: &mut Vec<u64>,
) -> usize {
    if let Some(depth) = memo.get(&node) {
        return *depth;
    }
    if visiting.contains(&node) {
        return 0;
    }
    visiting.push(node);
    let depth = children.get(&node).map_or(0, |child_nodes| {
        child_nodes
            .iter()
            .map(|child| 1 + project_graph_contains_depth(*child, children, memo, visiting))
            .max()
            .unwrap_or(0)
    });
    visiting.pop();
    memo.insert(node, depth);
    depth
}

pub(crate) fn expected_project_graph_origin_links(
    nodes: &[serde_json::Value],
    origin_map: &orv_compiler::OriginMap,
) -> anyhow::Result<Vec<serde_json::Value>> {
    let mut links = Vec::new();
    for entry in &origin_map.entries {
        if let Some(node) = nodes
            .iter()
            .find(|node| project_graph_node_matches_origin(node, entry))
        {
            let node_id = node
                .get("id")
                .and_then(serde_json::Value::as_u64)
                .ok_or_else(|| anyhow::anyhow!("project-graph.json node id must be an integer"))?;
            links.push(serde_json::json!({
                "kind": "source_node",
                "origin_id": entry.id,
                "node_id": node_id,
            }));
        }
    }
    Ok(links)
}

pub(crate) fn project_graph_node_matches_origin(
    node: &serde_json::Value,
    entry: &orv_compiler::OriginEntry,
) -> bool {
    node.get("file").and_then(serde_json::Value::as_u64) == Some(u64::from(entry.span.file))
        && node
            .pointer("/span/start")
            .and_then(serde_json::Value::as_u64)
            == Some(u64::from(entry.span.start))
        && node
            .pointer("/span/end")
            .and_then(serde_json::Value::as_u64)
            == Some(u64::from(entry.span.end))
}

pub(crate) fn expected_build_manifest_artifacts(
    plan: &serde_json::Value,
) -> anyhow::Result<std::collections::BTreeMap<String, String>> {
    let mut expected = std::collections::BTreeMap::from([
        (
            "build_manifest".to_string(),
            "build-manifest.json".to_string(),
        ),
        ("origin_map".to_string(), "origin-map.json".to_string()),
        ("bundle_plan".to_string(), "bundle-plan.json".to_string()),
        (
            "project_graph".to_string(),
            "project-graph.json".to_string(),
        ),
        ("source_bundle".to_string(), SOURCE_BUNDLE_PATH.to_string()),
    ]);
    let bundles = plan
        .get("bundles")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| anyhow::anyhow!("bundle plan bundles must be an array"))?;
    for bundle in bundles {
        let kind = json_str(bundle, "kind", "bundle target")?;
        let path = json_str(bundle, "path", "bundle target")?;
        if expected
            .insert(kind.to_string(), path.to_string())
            .is_some()
        {
            anyhow::bail!("bundle plan contains duplicate target kind {kind}");
        }
    }
    Ok(expected)
}

pub(crate) fn json_nonnegative_integer(value: &serde_json::Value) -> bool {
    value.as_u64().is_some() || value.as_i64().is_some_and(|value| value >= 0)
}

pub(crate) fn json_null_or_nonnegative_integer(value: &serde_json::Value) -> bool {
    value.is_null() || json_nonnegative_integer(value)
}

pub(crate) fn json_nonnegative_number(value: &serde_json::Value) -> bool {
    value.as_f64().is_some_and(|value| value >= 0.0)
}

pub(crate) fn json_null_or_nonnegative_number(value: &serde_json::Value) -> bool {
    value.is_null() || json_nonnegative_number(value)
}

pub(crate) fn json_null_or_string(value: &serde_json::Value) -> bool {
    value.is_null() || value.as_str().is_some()
}

pub(crate) fn json_null_or_bool(value: &serde_json::Value) -> bool {
    value.is_null() || value.as_bool().is_some()
}

pub(crate) fn json_u64_value(value: Option<&serde_json::Value>) -> Option<u64> {
    value.and_then(|value| {
        value
            .as_u64()
            .or_else(|| value.as_i64().and_then(|value| u64::try_from(value).ok()))
    })
}

pub(crate) fn read_json_value(path: &Path) -> anyhow::Result<serde_json::Value> {
    let source = std::fs::read_to_string(path)
        .map_err(|e| anyhow::anyhow!("failed to read {}: {e}", path.display()))?;
    serde_json::from_str(&source)
        .map_err(|e| anyhow::anyhow!("failed to parse {}: {e}", path.display()))
}

pub(crate) fn read_origin_map(dir: &Path) -> anyhow::Result<orv_compiler::OriginMap> {
    let value = read_json_value(&dir.join("origin-map.json"))?;
    verify_origin_map_json_keys(&value)?;
    serde_json::from_value(value)
        .map_err(|e| anyhow::anyhow!("failed to parse origin-map.json: {e}"))
}

pub(crate) fn read_server_artifacts(
    dir: &Path,
) -> anyhow::Result<Vec<(String, orv_compiler::ServerRuntimeArtifact)>> {
    let plan = read_json_value(&dir.join("bundle-plan.json"))?;
    let mut artifacts = Vec::new();
    let Some(bundles) = plan.get("bundles").and_then(serde_json::Value::as_array) else {
        return Ok(artifacts);
    };
    for bundle in bundles {
        if bundle.get("kind").and_then(serde_json::Value::as_str) != Some("server_runtime") {
            continue;
        }
        let path = json_str(bundle, "path", "bundle target")?;
        let artifact = read_server_artifact(&dir.join(path))?;
        artifacts.push((path.to_string(), artifact));
    }
    Ok(artifacts)
}

pub(crate) fn read_source_bundle_if_present(
    dir: &Path,
) -> anyhow::Result<Option<orv_compiler::SourceBundleArtifact>> {
    let path = dir.join("source-bundle.json");
    if !path.is_file() {
        return Ok(None);
    }
    Ok(Some(read_source_bundle_artifact(&path)?))
}

pub(crate) fn read_source_bundle_artifact(
    path: &Path,
) -> anyhow::Result<orv_compiler::SourceBundleArtifact> {
    let value = read_json_value(path)?;
    verify_source_bundle_artifact_keys(&value)?;
    let artifact: orv_compiler::SourceBundleArtifact = serde_json::from_value(value)
        .map_err(|e| anyhow::anyhow!("failed to parse {}: {e}", path.display()))?;
    orv_compiler::verify_source_bundle_artifact(&artifact)
        .map_err(|errors| anyhow::anyhow!("{}", errors.join("; ")))?;
    Ok(artifact)
}

pub(crate) fn json_str<'a>(
    value: &'a serde_json::Value,
    key: &str,
    context: &str,
) -> anyhow::Result<&'a str> {
    value
        .get(key)
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("{context} field `{key}` must be a string"))
}

pub(crate) fn json_string_array_field(
    value: &serde_json::Value,
    key: &str,
    context: &str,
) -> anyhow::Result<Vec<String>> {
    value
        .get(key)
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| anyhow::anyhow!("{context} field `{key}` must be an array"))?
        .iter()
        .map(|item| {
            item.as_str()
                .map(str::to_string)
                .ok_or_else(|| anyhow::anyhow!("{context} field `{key}` items must be strings"))
        })
        .collect()
}

pub(crate) fn json_optional_str<'a>(
    value: &'a serde_json::Value,
    key: &str,
    context: &str,
) -> anyhow::Result<Option<&'a str>> {
    let Some(value) = value.get(key) else {
        return Ok(None);
    };
    value
        .as_str()
        .filter(|value| !value.trim().is_empty())
        .map(Some)
        .ok_or_else(|| anyhow::anyhow!("{context} field `{key}` must be a non-empty string"))
}

pub(crate) fn json_u32(value: &serde_json::Value, key: &str, context: &str) -> anyhow::Result<u32> {
    let raw = value
        .get(key)
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| anyhow::anyhow!("{context} field `{key}` must be an integer"))?;
    u32::try_from(raw).map_err(|_| anyhow::anyhow!("{context} field `{key}` is too large"))
}

pub(crate) fn cmd_verify_artifact(path: &Path) -> anyhow::Result<()> {
    let artifact = read_server_artifact(path)?;
    orv_compiler::verify_server_runtime_artifact(&artifact)
        .map_err(|errors| anyhow::anyhow!("{}", errors.join("; ")))?;
    println!(
        "artifact: {} verified (routes={}, sources={})",
        path.display(),
        artifact.routes.len(),
        artifact.source_bundle.files.len()
    );
    Ok(())
}

pub(crate) fn cmd_check_artifact(path: &Path) -> anyhow::Result<()> {
    let artifact = read_server_artifact(path)?;
    orv_compiler::verify_server_runtime_artifact(&artifact)
        .map_err(|errors| anyhow::anyhow!("{}", errors.join("; ")))?;
    let lowered = lower_artifact_entry(&artifact)?;
    println!(
        "artifact: {} checked (routes={}, sources={}, items={})",
        path.display(),
        artifact.routes.len(),
        artifact.source_bundle.files.len(),
        lowered.program.items.len()
    );
    Ok(())
}

pub(crate) fn cmd_check_build(dir: &Path) -> anyhow::Result<()> {
    verify_build_dir(dir)?;
    let source_bundle = read_source_bundle_artifact(&dir.join("source-bundle.json"))?;
    let lowered = lower_source_bundle_entry(&source_bundle)?;
    println!(
        "build: {} checked (sources={}, items={})",
        dir.display(),
        source_bundle.files.len(),
        lowered.program.items.len()
    );
    Ok(())
}

pub(crate) fn cmd_lock(path: &Path, check: bool) -> anyhow::Result<()> {
    let manifest = project_manifest_path(path)?;
    let lock = project_lock_json(&manifest)?;
    let lock_path = manifest
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("orv.lock");
    if check {
        let existing = read_json_value(&lock_path)?;
        if existing != lock {
            anyhow::bail!("orv.lock is out of date; run `orv lock`");
        }
        println!("lock: {} verified", lock_path.display());
    } else {
        write_json_atomic(&lock_path, &lock)?;
        println!("lock: wrote {}", lock_path.display());
    }
    Ok(())
}

pub(crate) fn json_string_field<'a>(
    value: &'a serde_json::Value,
    field: &str,
    context: &str,
) -> anyhow::Result<&'a str> {
    value
        .get(field)
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("{context} {field} must be a string"))
}

pub(crate) fn read_server_artifact(
    path: &Path,
) -> anyhow::Result<orv_compiler::ServerRuntimeArtifact> {
    let source = std::fs::read_to_string(path)
        .map_err(|e| anyhow::anyhow!("failed to read {}: {e}", path.display()))?;
    serde_json::from_str(&source)
        .map_err(|e| anyhow::anyhow!("failed to parse {}: {e}", path.display()))
}

pub(crate) fn read_server_launch_artifact(
    path: &Path,
) -> anyhow::Result<orv_compiler::ServerLaunchArtifact> {
    let source = std::fs::read_to_string(path)
        .map_err(|e| anyhow::anyhow!("failed to read {}: {e}", path.display()))?;
    serde_json::from_str(&source)
        .map_err(|e| anyhow::anyhow!("failed to parse {}: {e}", path.display()))
}

pub(crate) fn validate_prod_server_listen(
    server_artifact: Option<&orv_compiler::ServerRuntimeArtifact>,
) -> anyhow::Result<()> {
    let Some(server_artifact) = server_artifact else {
        return Ok(());
    };
    if server_artifact
        .listen
        .as_ref()
        .and_then(|listen| listen.port)
        == Some(0)
    {
        anyhow::bail!("prod server listen port must be 1..=65535; @listen 0 is test-only");
    }
    Ok(())
}

pub(crate) fn normalize_source_origin_ids(source_origin_ids: &mut Vec<String>) {
    source_origin_ids.sort();
    source_origin_ids.dedup();
}

pub(crate) fn collect_program_persistence_paths(
    program: &orv_hir::HirProgram,
    out: &mut DeployPersistenceAccumulator,
) {
    for stmt in &program.items {
        collect_stmt_persistence_paths(stmt, out);
    }
}

pub(crate) fn collect_stmt_persistence_paths(
    stmt: &orv_hir::HirStmt,
    out: &mut DeployPersistenceAccumulator,
) {
    match stmt {
        orv_hir::HirStmt::Let(stmt) => {
            collect_expr_persistence_paths(&stmt.init, out);
        }
        orv_hir::HirStmt::Const(stmt) => {
            collect_expr_persistence_paths(&stmt.init, out);
        }
        orv_hir::HirStmt::Function(stmt) => {
            collect_function_body_persistence_paths(&stmt.body, out);
        }
        orv_hir::HirStmt::Return(stmt) => {
            if let Some(value) = &stmt.value {
                collect_expr_persistence_paths(value, out);
            }
        }
        orv_hir::HirStmt::Expr(expr) => {
            collect_expr_persistence_paths(expr, out);
        }
        orv_hir::HirStmt::Struct(_)
        | orv_hir::HirStmt::Enum(_)
        | orv_hir::HirStmt::TypeAlias(_)
        | orv_hir::HirStmt::Import(_) => {}
    }
}

pub(crate) fn collect_block_persistence_paths(
    block: &orv_hir::HirBlock,
    out: &mut DeployPersistenceAccumulator,
) {
    for stmt in &block.stmts {
        collect_stmt_persistence_paths(stmt, out);
    }
}

pub(crate) fn collect_function_body_persistence_paths(
    body: &orv_hir::HirFunctionBody,
    out: &mut DeployPersistenceAccumulator,
) {
    match body {
        orv_hir::HirFunctionBody::Block(block) => {
            collect_block_persistence_paths(block, out);
        }
        orv_hir::HirFunctionBody::Expr(expr) => {
            collect_expr_persistence_paths(expr, out);
        }
    }
}

pub(crate) fn collect_expr_persistence_paths(
    expr: &orv_hir::HirExpr,
    out: &mut DeployPersistenceAccumulator,
) {
    use orv_hir::HirExprKind;

    if let HirExprKind::Call { callee, args } = &expr.kind {
        let call_name = hir_call_name(callee);
        if call_name == "@db.wal" {
            if let Some(path) = args.first().and_then(hir_static_string) {
                out.wal_paths.push(path);
            }
        } else if call_name == "@db.connect" {
            if let Some(arg) = args.first() {
                collect_db_adapter_persistence_arg(
                    arg,
                    hir_source_origin_id("call", &call_name, expr.span),
                    out,
                );
            }
        } else if let Some(kind) = deploy_commerce_adapter_kind_for_call(&call_name) {
            if let Some(arg) = args.first() {
                collect_commerce_adapter_persistence_arg(
                    kind,
                    arg,
                    hir_source_origin_id("call", &call_name, expr.span),
                    out,
                );
            }
        }
    }

    match &expr.kind {
        HirExprKind::Integer(_)
        | HirExprKind::Float(_)
        | HirExprKind::Regex { .. }
        | HirExprKind::True
        | HirExprKind::False
        | HirExprKind::Void
        | HirExprKind::TypeName(_)
        | HirExprKind::Ident(_)
        | HirExprKind::Break
        | HirExprKind::Continue => {}
        HirExprKind::String(segments) => {
            for segment in segments {
                if let orv_hir::HirStringSegment::Interp(expr) = segment {
                    collect_expr_persistence_paths(expr, out);
                }
            }
        }
        HirExprKind::Unary { expr, .. }
        | HirExprKind::Paren(expr)
        | HirExprKind::Out(expr)
        | HirExprKind::Throw(expr)
        | HirExprKind::Await(expr)
        | HirExprKind::Cast { expr, .. } => {
            collect_expr_persistence_paths(expr, out);
        }
        HirExprKind::Binary { lhs, rhs, .. } => {
            collect_expr_persistence_paths(lhs, out);
            collect_expr_persistence_paths(rhs, out);
        }
        HirExprKind::Html(block) | HirExprKind::Block(block) => {
            collect_block_persistence_paths(block, out);
        }
        HirExprKind::Route { handler, .. } => {
            collect_block_persistence_paths(handler, out);
        }
        HirExprKind::Respond { status, payload } => {
            collect_expr_persistence_paths(status, out);
            collect_expr_persistence_paths(payload, out);
        }
        HirExprKind::Server {
            listen,
            routes,
            body_stmts,
        } => {
            if let Some(listen) = listen {
                collect_expr_persistence_paths(listen, out);
            }
            for route in routes {
                collect_expr_persistence_paths(route, out);
            }
            for stmt in body_stmts {
                collect_stmt_persistence_paths(stmt, out);
            }
        }
        HirExprKind::Domain { args, .. } => {
            for arg in args {
                collect_expr_persistence_paths(arg, out);
            }
        }
        HirExprKind::If {
            cond,
            then,
            else_branch,
        } => {
            collect_expr_persistence_paths(cond, out);
            collect_block_persistence_paths(then, out);
            if let Some(else_branch) = else_branch {
                collect_expr_persistence_paths(else_branch, out);
            }
        }
        HirExprKind::When { scrutinee, arms } => {
            collect_expr_persistence_paths(scrutinee, out);
            for arm in arms {
                collect_pattern_persistence_paths(&arm.pattern, out);
                collect_expr_persistence_paths(&arm.body, out);
            }
        }
        HirExprKind::Assign { value, .. } => {
            collect_expr_persistence_paths(value, out);
        }
        HirExprKind::AssignField { object, value, .. } => {
            collect_expr_persistence_paths(object, out);
            collect_expr_persistence_paths(value, out);
        }
        HirExprKind::AssignIndex {
            object,
            index,
            value,
        } => {
            collect_expr_persistence_paths(object, out);
            collect_expr_persistence_paths(index, out);
            collect_expr_persistence_paths(value, out);
        }
        HirExprKind::Call { callee, args } => {
            collect_expr_persistence_paths(callee, out);
            for arg in args {
                collect_expr_persistence_paths(arg, out);
            }
        }
        HirExprKind::For { iter, body, .. } => {
            collect_expr_persistence_paths(iter, out);
            collect_block_persistence_paths(body, out);
        }
        HirExprKind::While { cond, body } => {
            collect_expr_persistence_paths(cond, out);
            collect_block_persistence_paths(body, out);
        }
        HirExprKind::Range { start, end, .. } => {
            collect_expr_persistence_paths(start, out);
            collect_expr_persistence_paths(end, out);
        }
        HirExprKind::Array(items) | HirExprKind::Tuple(items) => {
            for item in items {
                collect_expr_persistence_paths(item, out);
            }
        }
        HirExprKind::Object(fields) | HirExprKind::TypedObject { fields, .. } => {
            for field in fields {
                collect_expr_persistence_paths(&field.value, out);
            }
        }
        HirExprKind::Index { target, index } => {
            collect_expr_persistence_paths(target, out);
            collect_expr_persistence_paths(index, out);
        }
        HirExprKind::Slice { target, start, end } => {
            collect_expr_persistence_paths(target, out);
            if let Some(start) = start {
                collect_expr_persistence_paths(start, out);
            }
            if let Some(end) = end {
                collect_expr_persistence_paths(end, out);
            }
        }
        HirExprKind::Field { target, .. } | HirExprKind::OptionalField { target, .. } => {
            collect_expr_persistence_paths(target, out);
        }
        HirExprKind::Lambda { body, .. } => {
            collect_function_body_persistence_paths(body, out);
        }
        HirExprKind::Try { try_block, catch } => {
            collect_block_persistence_paths(try_block, out);
            if let Some(catch) = catch {
                collect_block_persistence_paths(&catch.body, out);
            }
        }
    }
}

pub(crate) fn collect_pattern_persistence_paths(
    pattern: &orv_hir::HirPattern,
    out: &mut DeployPersistenceAccumulator,
) {
    match pattern {
        orv_hir::HirPattern::Literal(expr)
        | orv_hir::HirPattern::Guard(expr)
        | orv_hir::HirPattern::Not(expr)
        | orv_hir::HirPattern::Contains(expr) => {
            collect_expr_persistence_paths(expr, out);
        }
        orv_hir::HirPattern::Range { start, end, .. } => {
            collect_expr_persistence_paths(start, out);
            collect_expr_persistence_paths(end, out);
        }
        orv_hir::HirPattern::Wildcard => {}
    }
}

pub(crate) const SOURCE_BUNDLE_PATH: &str = "source-bundle.json";
pub(crate) const ORV_REFERENCE_RUNTIME_IMAGE: &str = "ghcr.io/orv-lang/orv-reference:latest";
pub(crate) const SERVER_ARTIFACT_PATH: &str = "server/app.orv-runtime.json";
pub(crate) const SERVER_LAUNCH_PATH: &str = "server/launch.json";

pub(crate) fn relative_bundle_path(from: &str, to: &str) -> String {
    let depth = from.split('/').count().saturating_sub(1);
    format!("{}{}", "../".repeat(depth), to)
}

pub(crate) fn write_prod_routes_artifact(
    out: &Path,
    server_artifact_path: &str,
    server_artifact: &orv_compiler::ServerRuntimeArtifact,
) -> anyhow::Result<()> {
    let routes = serde_json::json!({
        "schema_version": 1,
        "artifact": server_artifact_path,
        "runtime": server_artifact.runtime.clone(),
        "protocol": "http1",
        "routes": server_artifact.routes.clone(),
    });
    write_json(&out.join("deploy").join("routes.json"), &routes)
}

pub(crate) fn write_prod_container_artifacts(
    out: &Path,
    server_artifact_path: &str,
    entrypoint: &str,
    routes_artifact: &str,
    dockerfile_path: &str,
    server_artifact: &orv_compiler::ServerRuntimeArtifact,
    persistence: &DeployPersistence,
) -> anyhow::Result<()> {
    let container = serde_json::json!({
        "schema_version": 1,
        "kind": "reference-server-container",
        "dockerfile": dockerfile_path,
        "artifact": server_artifact_path,
        "entrypoint": entrypoint,
        "routes_artifact": routes_artifact,
        "runtime": server_artifact.runtime.clone(),
        "runtime_image": ORV_REFERENCE_RUNTIME_IMAGE,
        "protocol": "http1",
        "listen": server_artifact.listen.clone(),
        "ports": deploy_ports_value(server_artifact.listen.as_ref()),
        "command": ["./deploy/server.sh"],
        "persistence": deploy_persistence_value(persistence),
    });
    write_json(&out.join("deploy").join("container.json"), &container)?;
    let dockerfile =
        deploy_dockerfile_content(ORV_REFERENCE_RUNTIME_IMAGE, server_artifact.listen.as_ref());
    write_text(&out.join(dockerfile_path), &dockerfile)
}

pub(crate) fn write_prod_compose_artifact(
    out: &Path,
    dockerfile_path: &str,
    server_artifact: &orv_compiler::ServerRuntimeArtifact,
    persistence: &DeployPersistence,
) -> anyhow::Result<()> {
    let compose = deploy_compose_content(
        dockerfile_path,
        server_artifact.listen.as_ref(),
        persistence,
    );
    write_text(&out.join("deploy").join("compose.yaml"), &compose)
}

pub(crate) fn write_prod_env_example_artifact(
    out: &Path,
    path: &str,
    server_artifact: &orv_compiler::ServerRuntimeArtifact,
    persistence: &DeployPersistence,
) -> anyhow::Result<()> {
    let env_example = deploy_env_example_content(server_artifact.listen.as_ref(), persistence);
    write_text(&out.join(path), &env_example)
}

pub(crate) fn write_prod_preflight_artifact(
    out: &Path,
    path: &str,
    artifacts: &DeployRunbookArtifacts<'_>,
    server_artifact: &orv_compiler::ServerRuntimeArtifact,
    persistence: &DeployPersistence,
    client: &serde_json::Value,
) -> anyhow::Result<()> {
    let preflight =
        deploy_preflight_artifact_value(artifacts, server_artifact, persistence, Some(client));
    write_json(&out.join(path), &preflight)
}

pub(crate) fn write_prod_server_entrypoint(
    out: &Path,
    server_artifact_path: &str,
) -> anyhow::Result<()> {
    let script = deploy_server_entrypoint_content(server_artifact_path);
    let path = out.join("deploy").join("server.sh");
    write_text(&path, &script)?;
    set_executable_if_supported(&path)
}

#[cfg(unix)]
pub(crate) fn set_executable_if_supported(path: &Path) -> anyhow::Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let mut permissions = std::fs::metadata(path)
        .map_err(|e| anyhow::anyhow!("failed to stat {}: {e}", path.display()))?
        .permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(path, permissions)
        .map_err(|e| anyhow::anyhow!("failed to chmod {}: {e}", path.display()))
}

#[cfg(not(unix))]
pub(crate) fn set_executable_if_supported(_path: &Path) -> anyhow::Result<()> {
    Ok(())
}
