use super::*;

pub(crate) fn client_reactive_plan_signal_text_bindings_are_valid(
    signals: &[serde_json::Value],
    bindings: &[serde_json::Value],
) -> bool {
    bindings
        .iter()
        .filter(|binding| {
            binding.get("kind").and_then(serde_json::Value::as_str) == Some("signal_text")
        })
        .all(|binding| client_reactive_plan_signal_text_binding_is_valid(signals, binding))
}

pub(crate) fn client_reactive_plan_signal_text_binding_is_valid(
    signals: &[serde_json::Value],
    binding: &serde_json::Value,
) -> bool {
    let origin_id = binding
        .get("source")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("");
    let state_key = binding
        .get("state_key")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("");
    binding.get("target").and_then(serde_json::Value::as_str) == Some(CLIENT_PAGE_PATH)
        && binding
            .get("selector")
            .and_then(serde_json::Value::as_str)
            .is_some_and(|selector| !selector.is_empty())
        && signals.iter().any(|signal| {
            signal.get("origin_id").and_then(serde_json::Value::as_str) == Some(origin_id)
                && signal.get("state_key").and_then(serde_json::Value::as_str) == Some(state_key)
        })
        && client_signal_text_state_keys_are_valid(signals, binding)
        && client_signal_text_sources_are_valid(signals, binding)
        && client_signal_text_template_is_valid(signals, binding)
        && client_signal_text_condition_is_valid(signals, binding)
}

pub(crate) fn client_signal_text_binding_state_keys(
    binding: &serde_json::Value,
) -> Option<Vec<&str>> {
    if let Some(state_keys) = binding.get("state_keys") {
        let state_keys = state_keys.as_array()?;
        if state_keys.is_empty() {
            return None;
        }
        return state_keys
            .iter()
            .map(serde_json::Value::as_str)
            .collect::<Option<Vec<_>>>();
    }
    binding
        .get("state_key")
        .and_then(serde_json::Value::as_str)
        .map(|state_key| vec![state_key])
}

pub(crate) fn client_signal_text_state_keys_are_valid(
    signals: &[serde_json::Value],
    binding: &serde_json::Value,
) -> bool {
    let Some(state_keys) = client_signal_text_binding_state_keys(binding) else {
        return false;
    };
    let binding_state_key = binding
        .get("state_key")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("");
    state_keys.contains(&binding_state_key)
        && state_keys.iter().all(|state_key| {
            signals.iter().any(|signal| {
                signal.get("state_key").and_then(serde_json::Value::as_str) == Some(*state_key)
            })
        })
}

pub(crate) fn client_signal_text_sources_are_valid(
    signals: &[serde_json::Value],
    binding: &serde_json::Value,
) -> bool {
    let Some(sources) = binding.get("sources") else {
        return true;
    };
    let Some(sources) = sources.as_array() else {
        return false;
    };
    let Some(state_keys) = client_signal_text_binding_state_keys(binding) else {
        return false;
    };
    !sources.is_empty()
        && sources.iter().all(|source| {
            let origin_id = source
                .get("source")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("");
            let state_key = source
                .get("state_key")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("");
            state_keys.contains(&state_key)
                && signals.iter().any(|signal| {
                    signal.get("origin_id").and_then(serde_json::Value::as_str) == Some(origin_id)
                        && signal.get("state_key").and_then(serde_json::Value::as_str)
                            == Some(state_key)
                })
        })
}

pub(crate) fn client_signal_text_template_is_valid(
    signals: &[serde_json::Value],
    binding: &serde_json::Value,
) -> bool {
    let Some(template) = binding.get("text_template") else {
        return true;
    };
    let Some(segments) = template.as_array() else {
        return false;
    };
    let Some(state_keys) = client_signal_text_binding_state_keys(binding) else {
        return false;
    };
    !segments.is_empty()
        && segments.iter().all(|segment| {
            match segment.get("kind").and_then(serde_json::Value::as_str) {
                Some("text") => segment
                    .get("value")
                    .and_then(serde_json::Value::as_str)
                    .is_some(),
                Some("signal") => {
                    let state_key = segment
                        .get("state_key")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or("");
                    state_keys.contains(&state_key)
                        && signals.iter().any(|signal| {
                            signal.get("state_key").and_then(serde_json::Value::as_str)
                                == Some(state_key)
                        })
                }
                _ => false,
            }
        })
}

pub(crate) fn client_signal_text_condition_is_valid(
    signals: &[serde_json::Value],
    binding: &serde_json::Value,
) -> bool {
    let Some(condition) = binding.get("text_condition") else {
        return true;
    };
    client_signal_condition_binding_is_valid(signals, binding, condition)
}

pub(crate) fn client_reactive_plan_signal_attr_bindings_are_valid(
    signals: &[serde_json::Value],
    bindings: &[serde_json::Value],
) -> bool {
    bindings
        .iter()
        .filter(|binding| {
            binding.get("kind").and_then(serde_json::Value::as_str) == Some("signal_attr")
        })
        .all(|binding| client_reactive_plan_signal_attr_binding_is_valid(signals, binding))
}

pub(crate) fn client_reactive_plan_signal_attr_binding_is_valid(
    signals: &[serde_json::Value],
    binding: &serde_json::Value,
) -> bool {
    let origin_id = binding
        .get("source")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("");
    let state_key = binding
        .get("state_key")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("");
    binding.get("target").and_then(serde_json::Value::as_str) == Some(CLIENT_PAGE_PATH)
        && binding
            .get("selector")
            .and_then(serde_json::Value::as_str)
            .is_some_and(|selector| !selector.is_empty())
        && binding
            .get("attr")
            .and_then(serde_json::Value::as_str)
            .is_some_and(|attr| !attr.is_empty())
        && signals.iter().any(|signal| {
            signal.get("origin_id").and_then(serde_json::Value::as_str) == Some(origin_id)
                && signal.get("state_key").and_then(serde_json::Value::as_str) == Some(state_key)
        })
        && client_signal_attr_state_keys_are_valid(signals, binding)
        && client_signal_attr_sources_are_valid(signals, binding)
        && client_signal_attr_template_is_valid(signals, binding)
        && client_signal_attr_condition_is_valid(signals, binding)
}

pub(crate) fn client_signal_attr_state_keys_are_valid(
    signals: &[serde_json::Value],
    binding: &serde_json::Value,
) -> bool {
    client_signal_text_state_keys_are_valid(signals, binding)
}

pub(crate) fn client_signal_attr_sources_are_valid(
    signals: &[serde_json::Value],
    binding: &serde_json::Value,
) -> bool {
    client_signal_text_sources_are_valid(signals, binding)
}

pub(crate) fn client_signal_attr_template_is_valid(
    signals: &[serde_json::Value],
    binding: &serde_json::Value,
) -> bool {
    let Some(template) = binding.get("attr_template") else {
        return true;
    };
    let Some(segments) = template.as_array() else {
        return false;
    };
    let Some(state_keys) = client_signal_text_binding_state_keys(binding) else {
        return false;
    };
    !segments.is_empty()
        && segments.iter().all(|segment| {
            match segment.get("kind").and_then(serde_json::Value::as_str) {
                Some("text") => segment
                    .get("value")
                    .and_then(serde_json::Value::as_str)
                    .is_some(),
                Some("signal") => {
                    let state_key = segment
                        .get("state_key")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or("");
                    state_keys.contains(&state_key)
                        && signals.iter().any(|signal| {
                            signal.get("state_key").and_then(serde_json::Value::as_str)
                                == Some(state_key)
                        })
                }
                _ => false,
            }
        })
}

pub(crate) fn client_signal_attr_condition_is_valid(
    signals: &[serde_json::Value],
    binding: &serde_json::Value,
) -> bool {
    let Some(condition) = binding.get("attr_condition") else {
        return true;
    };
    client_signal_condition_binding_is_valid(signals, binding, condition)
}

pub(crate) fn client_signal_condition_binding_is_valid(
    signals: &[serde_json::Value],
    binding: &serde_json::Value,
    condition: &serde_json::Value,
) -> bool {
    let state_key = condition
        .get("state_key")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("");
    let binding_state_key = binding
        .get("state_key")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("");
    state_key == binding_state_key
        && condition
            .get("truthy")
            .and_then(serde_json::Value::as_str)
            .is_some()
        && condition
            .get("falsy")
            .and_then(serde_json::Value::as_str)
            .is_some()
        && client_signal_attr_condition_comparison_is_valid(condition)
        && signals.iter().any(|signal| {
            signal.get("state_key").and_then(serde_json::Value::as_str) == Some(state_key)
        })
}

pub(crate) fn client_signal_attr_condition_comparison_is_valid(
    condition: &serde_json::Value,
) -> bool {
    match (condition.get("op"), condition.get("rhs")) {
        (None, None) => true,
        (Some(op), Some(rhs)) => {
            op.as_str()
                .is_some_and(|op| matches!(op, "eq" | "ne" | "lt" | "gt" | "le" | "ge"))
                && client_signal_condition_operand_is_valid(rhs)
        }
        _ => false,
    }
}

pub(crate) fn client_signal_condition_operand_is_valid(value: &serde_json::Value) -> bool {
    let Some(kind) = value.get("kind").and_then(serde_json::Value::as_str) else {
        return false;
    };
    match kind {
        "int" | "float" | "string" => value
            .get("value")
            .and_then(serde_json::Value::as_str)
            .is_some(),
        "bool" => value
            .get("value")
            .and_then(serde_json::Value::as_bool)
            .is_some(),
        _ => false,
    }
}

pub(crate) fn client_reactive_plan_signal_event_bindings_are_valid(
    signals: &[serde_json::Value],
    bindings: &[serde_json::Value],
) -> bool {
    bindings
        .iter()
        .filter(|binding| {
            binding.get("kind").and_then(serde_json::Value::as_str) == Some("signal_event")
        })
        .all(|binding| client_reactive_plan_signal_event_binding_is_valid(signals, binding))
}

pub(crate) fn client_reactive_plan_signal_event_binding_is_valid(
    signals: &[serde_json::Value],
    binding: &serde_json::Value,
) -> bool {
    let origin_id = binding
        .get("source")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("");
    let state_key = binding
        .get("state_key")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("");
    binding.get("target").and_then(serde_json::Value::as_str) == Some(CLIENT_PAGE_PATH)
        && binding
            .get("selector")
            .and_then(serde_json::Value::as_str)
            .is_some_and(|selector| !selector.is_empty())
        && binding
            .get("event")
            .and_then(serde_json::Value::as_str)
            .is_some_and(|event| !event.is_empty())
        && client_signal_event_action_is_valid(binding.get("action"))
        && signals.iter().any(|signal| {
            signal.get("origin_id").and_then(serde_json::Value::as_str) == Some(origin_id)
                && signal.get("state_key").and_then(serde_json::Value::as_str) == Some(state_key)
        })
}

pub(crate) fn client_signal_event_action_is_valid(action: Option<&serde_json::Value>) -> bool {
    let Some(action) = action else {
        return false;
    };
    match action.get("kind").and_then(serde_json::Value::as_str) {
        Some(
            "assign_toggle"
            | "assign_event_target_value"
            | "assign_event_target_checked"
            | "assign_event_target_value_float"
            | "assign_event_target_value_int",
        ) => true,
        Some("assign" | "assign_add" | "assign_sub") => action
            .get("value")
            .and_then(|value| value.get("kind"))
            .and_then(serde_json::Value::as_str)
            .is_some(),
        _ => false,
    }
}

pub(crate) fn client_wasm_metadata_value(target: &Path) -> anyhow::Result<serde_json::Value> {
    let bytes = std::fs::read(target)
        .map_err(|e| anyhow::anyhow!("failed to read {}: {e}", target.display()))?;
    client_wasm_metadata_value_from_bytes(&bytes)
}

pub(crate) fn bundle_plan_has_client_target(plan: &serde_json::Value) -> anyhow::Result<bool> {
    let bundles = plan
        .get("bundles")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| anyhow::anyhow!("bundle plan bundles must be an array"))?;
    Ok(bundles.iter().any(|target| {
        target
            .get("kind")
            .and_then(serde_json::Value::as_str)
            .is_some_and(is_client_bundle_kind)
    }))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct DeployClientSummaryCounts {
    pub(crate) targets: usize,
    pub(crate) manifests: usize,
    pub(crate) capability_surfaces: usize,
}

pub(crate) fn deploy_client_summary_counts(
    dir: &Path,
) -> anyhow::Result<DeployClientSummaryCounts> {
    let targets = reveal_client_bundle_targets(dir)?;
    Ok(DeployClientSummaryCounts {
        targets: targets.len(),
        manifests: production_client_manifest_count(&targets),
        capability_surfaces: production_client_capability_surface_count(&targets),
    })
}

pub(crate) fn is_client_bundle_kind(kind: &str) -> bool {
    matches!(
        kind,
        "client_manifest" | "client_reactive_plan" | "client_page" | "client_js" | "client_wasm"
    )
}

pub(crate) fn write_client_bundle_artifacts(
    out: &Path,
    entry: &Path,
    enabled: bool,
    binding: &ClientSourceBinding<'_>,
    targets: &ClientBundleTargets<'_>,
) -> anyhow::Result<()> {
    if !enabled {
        return Ok(());
    }
    let page_path = targets
        .page
        .ok_or_else(|| anyhow::anyhow!("missing client_page bundle target"))?;
    let manifest_path = targets
        .manifest
        .ok_or_else(|| anyhow::anyhow!("missing client_manifest bundle target"))?;
    let reactive_plan_path = targets
        .reactive_plan
        .ok_or_else(|| anyhow::anyhow!("missing client_reactive_plan bundle target"))?;
    let js_path = targets
        .js
        .ok_or_else(|| anyhow::anyhow!("missing client_js bundle target"))?;
    let wasm_path = targets
        .wasm
        .ok_or_else(|| anyhow::anyhow!("missing client_wasm bundle target"))?;
    write_client_wasm_bundle(
        &out.join(wasm_path),
        binding.source_bundle,
        binding.source_bundle_hash,
        binding.initial_render,
    )?;
    write_client_js_loader(&out.join(js_path), entry, binding)?;
    let loader_src = relative_bundle_path(page_path, js_path);
    write_client_page_shell(&out.join(page_path), entry, &loader_src)?;
    write_client_reactive_plan(out, reactive_plan_path, entry, binding)?;
    write_client_bundle_manifest(out, manifest_path, entry, binding, targets)
}

pub(crate) struct ClientSourceBinding<'a> {
    pub(crate) source_bundle: &'a orv_compiler::SourceBundleArtifact,
    pub(crate) source_bundle_hash: &'a str,
    pub(crate) origin_map: &'a orv_compiler::OriginMap,
    pub(crate) program: &'a orv_hir::HirProgram,
    pub(crate) initial_render: &'a str,
}

pub(crate) struct ClientBundleTargets<'a> {
    pub(crate) manifest: Option<&'a str>,
    pub(crate) reactive_plan: Option<&'a str>,
    pub(crate) page: Option<&'a str>,
    pub(crate) js: Option<&'a str>,
    pub(crate) wasm: Option<&'a str>,
}

pub(crate) fn write_client_reactive_plan(
    out: &Path,
    path: &str,
    entry: &Path,
    binding: &ClientSourceBinding<'_>,
) -> anyhow::Result<()> {
    let plan = client_reactive_plan_json(entry, binding);
    write_json(&out.join(path), &plan)
}

pub(crate) fn client_reactive_plan_json(
    entry: &Path,
    binding: &ClientSourceBinding<'_>,
) -> serde_json::Value {
    let signals = client_reactive_plan_signals(binding);
    let mut bindings = vec![serde_json::json!({
        "kind": "initial_render",
        "target": CLIENT_PAGE_PATH,
        "source": CLIENT_WASM_PATH,
        "html_hash": format!("{:016x}", fnv1a64(binding.initial_render.as_bytes())),
        "byte_length": binding.initial_render.len(),
    })];
    bindings.extend(signals.iter().map(|signal| {
        serde_json::json!({
            "kind": "signal_state",
            "target": CLIENT_JS_PATH,
            "source": signal["origin_id"].clone(),
            "state_key": signal["state_key"].clone(),
        })
    }));
    bindings.extend(client_reactive_dom_bindings(binding));
    serde_json::json!({
        "schema_version": 1,
        "kind": "orv.client.reactive_plan",
        "entry": entry.display().to_string(),
        "source_bundle": SOURCE_BUNDLE_PATH,
        "source_bundle_hash": binding.source_bundle_hash,
        "runtime_features": ["client_wasm"],
        "signals": signals,
        "bindings": bindings,
        "blocked_by": ["reactive-dom-diff"],
        "blockers": client_reactive_plan_blockers_json(),
    })
}

pub(crate) fn client_reactive_plan_blockers_json() -> Vec<serde_json::Value> {
    vec![serde_json::json!({
        "id": "reactive-dom-diff",
        "artifact": CLIENT_REACTIVE_PLAN_PATH,
        "reason": "full DOM diff codegen is not emitted yet",
    })]
}

pub(crate) fn client_reactive_plan_signals(
    binding: &ClientSourceBinding<'_>,
) -> Vec<serde_json::Value> {
    let initial_values = client_signal_initial_values(binding.program);
    binding
        .origin_map
        .entries
        .iter()
        .filter(|entry| entry.kind == "signal")
        .map(|signal| {
            serde_json::json!({
                "name": &signal.name,
                "origin_id": &signal.id,
                "state_key": &signal.name,
                "initial_value": initial_values
                    .get(&signal.id)
                    .cloned()
                    .unwrap_or_else(|| serde_json::json!({"kind": "dynamic"})),
                "span": {
                    "file": signal.span.file,
                    "start": signal.span.start,
                    "end": signal.span.end,
                },
            })
        })
        .collect()
}

#[derive(Clone, Debug)]
pub(crate) struct ClientSignalDomSource {
    pub(crate) origin_id: String,
    pub(crate) state_key: String,
}

#[derive(Clone, Debug)]
pub(crate) struct ClientSignalTextBinding {
    pub(crate) origin_id: String,
    pub(crate) state_key: String,
    pub(crate) text_template: Option<Vec<serde_json::Value>>,
    pub(crate) text_condition: Option<serde_json::Value>,
    pub(crate) signal_sources: Vec<ClientSignalDomSource>,
}

#[derive(Clone, Debug)]
pub(crate) struct ClientSignalAttrBinding {
    pub(crate) origin_id: String,
    pub(crate) state_key: String,
    pub(crate) attr_template: Option<Vec<serde_json::Value>>,
    pub(crate) attr_condition: Option<serde_json::Value>,
    pub(crate) signal_sources: Vec<ClientSignalDomSource>,
}

pub(crate) fn client_reactive_dom_bindings(
    binding: &ClientSourceBinding<'_>,
) -> Vec<serde_json::Value> {
    let signals = client_signal_dom_sources(binding.program);
    let mut bindings = Vec::new();
    for stmt in &binding.program.items {
        collect_client_dom_bindings_stmt(stmt, false, &signals, &mut bindings);
    }
    bindings
}

pub(crate) fn client_signal_dom_sources(
    program: &orv_hir::HirProgram,
) -> HashMap<orv_hir::NameId, ClientSignalDomSource> {
    program
        .items
        .iter()
        .filter_map(|stmt| {
            let orv_hir::HirStmt::Let(stmt) = stmt else {
                return None;
            };
            (stmt.kind == orv_hir::HirLetKind::Signal).then(|| {
                (
                    stmt.name.id,
                    ClientSignalDomSource {
                        origin_id: orv_hir::origin_id("signal", &stmt.name.name, stmt.span),
                        state_key: stmt.name.name.clone(),
                    },
                )
            })
        })
        .collect()
}

pub(crate) fn collect_client_dom_bindings_stmt(
    stmt: &orv_hir::HirStmt,
    inside_html: bool,
    signals: &HashMap<orv_hir::NameId, ClientSignalDomSource>,
    out: &mut Vec<serde_json::Value>,
) {
    match stmt {
        orv_hir::HirStmt::Let(stmt) => {
            collect_client_dom_bindings_expr(&stmt.init, inside_html, signals, out);
        }
        orv_hir::HirStmt::Const(stmt) => {
            collect_client_dom_bindings_expr(&stmt.init, inside_html, signals, out);
        }
        orv_hir::HirStmt::Function(stmt) => {
            collect_client_dom_bindings_function_body(&stmt.body, inside_html, signals, out);
        }
        orv_hir::HirStmt::Return(stmt) => {
            if let Some(value) = &stmt.value {
                collect_client_dom_bindings_expr(value, inside_html, signals, out);
            }
        }
        orv_hir::HirStmt::Expr(expr) => {
            collect_client_dom_bindings_expr(expr, inside_html, signals, out);
        }
        orv_hir::HirStmt::Struct(_)
        | orv_hir::HirStmt::Enum(_)
        | orv_hir::HirStmt::TypeAlias(_)
        | orv_hir::HirStmt::Import(_) => {}
    }
}

pub(crate) fn collect_client_dom_bindings_function_body(
    body: &orv_hir::HirFunctionBody,
    inside_html: bool,
    signals: &HashMap<orv_hir::NameId, ClientSignalDomSource>,
    out: &mut Vec<serde_json::Value>,
) {
    match body {
        orv_hir::HirFunctionBody::Block(block) => {
            collect_client_dom_bindings_block(block, inside_html, signals, out);
        }
        orv_hir::HirFunctionBody::Expr(expr) => {
            collect_client_dom_bindings_expr(expr, inside_html, signals, out);
        }
    }
}

pub(crate) fn collect_client_dom_bindings_block(
    block: &orv_hir::HirBlock,
    inside_html: bool,
    signals: &HashMap<orv_hir::NameId, ClientSignalDomSource>,
    out: &mut Vec<serde_json::Value>,
) {
    for stmt in &block.stmts {
        collect_client_dom_bindings_stmt(stmt, inside_html, signals, out);
    }
}

pub(crate) fn collect_client_dom_bindings_expr(
    expr: &orv_hir::HirExpr,
    inside_html: bool,
    signals: &HashMap<orv_hir::NameId, ClientSignalDomSource>,
    out: &mut Vec<serde_json::Value>,
) {
    match &expr.kind {
        orv_hir::HirExprKind::Html(block) => {
            collect_client_dom_bindings_block(block, true, signals, out);
        }
        orv_hir::HirExprKind::Domain { name, args, .. } => {
            if inside_html {
                collect_client_dom_bindings_for_tag(name, args, signals, out);
                collect_client_attr_bindings_for_tag(name, args, signals, out);
                collect_client_event_bindings_for_tag(name, args, signals, out);
            }
            for arg in args {
                collect_client_dom_bindings_expr(arg, inside_html, signals, out);
            }
        }
        orv_hir::HirExprKind::Block(block) => {
            collect_client_dom_bindings_block(block, inside_html, signals, out);
        }
        orv_hir::HirExprKind::Out(inner)
        | orv_hir::HirExprKind::Unary { expr: inner, .. }
        | orv_hir::HirExprKind::Paren(inner)
        | orv_hir::HirExprKind::Throw(inner)
        | orv_hir::HirExprKind::Await(inner)
        | orv_hir::HirExprKind::Cast { expr: inner, .. } => {
            collect_client_dom_bindings_expr(inner, inside_html, signals, out);
        }
        orv_hir::HirExprKind::Binary { lhs, rhs, .. }
        | orv_hir::HirExprKind::Range {
            start: lhs,
            end: rhs,
            ..
        } => {
            collect_client_dom_bindings_expr(lhs, inside_html, signals, out);
            collect_client_dom_bindings_expr(rhs, inside_html, signals, out);
        }
        orv_hir::HirExprKind::String(segments) => {
            for segment in segments {
                if let orv_hir::HirStringSegment::Interp(expr) = segment {
                    collect_client_dom_bindings_expr(expr, inside_html, signals, out);
                }
            }
        }
        orv_hir::HirExprKind::If {
            cond,
            then,
            else_branch,
        } => {
            collect_client_dom_bindings_expr(cond, inside_html, signals, out);
            collect_client_dom_bindings_block(then, inside_html, signals, out);
            if let Some(else_branch) = else_branch {
                collect_client_dom_bindings_expr(else_branch, inside_html, signals, out);
            }
        }
        _ => collect_client_dom_bindings_nested_expr(expr, inside_html, signals, out),
    }
}

pub(crate) fn collect_client_dom_bindings_nested_expr(
    expr: &orv_hir::HirExpr,
    inside_html: bool,
    signals: &HashMap<orv_hir::NameId, ClientSignalDomSource>,
    out: &mut Vec<serde_json::Value>,
) {
    match &expr.kind {
        orv_hir::HirExprKind::Assign { value, .. } => {
            collect_client_dom_bindings_expr(value, inside_html, signals, out);
        }
        orv_hir::HirExprKind::AssignField { object, value, .. } => {
            collect_client_dom_bindings_expr(object, inside_html, signals, out);
            collect_client_dom_bindings_expr(value, inside_html, signals, out);
        }
        orv_hir::HirExprKind::AssignIndex {
            object,
            index,
            value,
        } => {
            collect_client_dom_bindings_expr(object, inside_html, signals, out);
            collect_client_dom_bindings_expr(index, inside_html, signals, out);
            collect_client_dom_bindings_expr(value, inside_html, signals, out);
        }
        orv_hir::HirExprKind::Call { callee, args } => {
            collect_client_dom_bindings_expr(callee, inside_html, signals, out);
            for arg in args {
                collect_client_dom_bindings_expr(arg, inside_html, signals, out);
            }
        }
        orv_hir::HirExprKind::For { iter, body, .. } => {
            collect_client_dom_bindings_expr(iter, inside_html, signals, out);
            collect_client_dom_bindings_block(body, inside_html, signals, out);
        }
        orv_hir::HirExprKind::While { cond, body } => {
            collect_client_dom_bindings_expr(cond, inside_html, signals, out);
            collect_client_dom_bindings_block(body, inside_html, signals, out);
        }
        orv_hir::HirExprKind::Array(items) | orv_hir::HirExprKind::Tuple(items) => {
            for item in items {
                collect_client_dom_bindings_expr(item, inside_html, signals, out);
            }
        }
        orv_hir::HirExprKind::Object(fields) | orv_hir::HirExprKind::TypedObject { fields, .. } => {
            for field in fields {
                collect_client_dom_bindings_expr(&field.value, inside_html, signals, out);
            }
        }
        orv_hir::HirExprKind::Index { target, index } => {
            collect_client_dom_bindings_expr(target, inside_html, signals, out);
            collect_client_dom_bindings_expr(index, inside_html, signals, out);
        }
        orv_hir::HirExprKind::Slice { target, start, end } => {
            collect_client_dom_bindings_expr(target, inside_html, signals, out);
            if let Some(start) = start {
                collect_client_dom_bindings_expr(start, inside_html, signals, out);
            }
            if let Some(end) = end {
                collect_client_dom_bindings_expr(end, inside_html, signals, out);
            }
        }
        orv_hir::HirExprKind::Field { target, .. }
        | orv_hir::HirExprKind::OptionalField { target, .. } => {
            collect_client_dom_bindings_expr(target, inside_html, signals, out);
        }
        orv_hir::HirExprKind::Lambda { body, .. } => {
            collect_client_dom_bindings_function_body(body, inside_html, signals, out);
        }
        orv_hir::HirExprKind::Try { try_block, catch } => {
            collect_client_dom_bindings_block(try_block, inside_html, signals, out);
            if let Some(catch) = catch {
                collect_client_dom_bindings_block(&catch.body, inside_html, signals, out);
            }
        }
        orv_hir::HirExprKind::Route { handler, .. } => {
            collect_client_dom_bindings_block(handler, inside_html, signals, out);
        }
        orv_hir::HirExprKind::Respond { status, payload } => {
            collect_client_dom_bindings_expr(status, inside_html, signals, out);
            collect_client_dom_bindings_expr(payload, inside_html, signals, out);
        }
        orv_hir::HirExprKind::Server {
            listen,
            routes,
            body_stmts,
        } => {
            if let Some(listen) = listen {
                collect_client_dom_bindings_expr(listen, inside_html, signals, out);
            }
            for route in routes {
                collect_client_dom_bindings_expr(route, inside_html, signals, out);
            }
            for stmt in body_stmts {
                collect_client_dom_bindings_stmt(stmt, inside_html, signals, out);
            }
        }
        _ => {}
    }
}

#[derive(Clone, Debug)]
pub(crate) struct ClientSignalEventAction {
    pub(crate) origin_id: String,
    pub(crate) state_key: String,
    pub(crate) action: serde_json::Value,
}

pub(crate) fn collect_client_event_bindings_for_tag(
    tag: &str,
    args: &[orv_hir::HirExpr],
    signals: &HashMap<orv_hir::NameId, ClientSignalDomSource>,
    out: &mut Vec<serde_json::Value>,
) {
    for_each_client_tag_attr_assignment(args, |target, value| {
        let Some(event) = client_event_attr_name(&target.name) else {
            return;
        };
        let Some(action) = client_signal_event_action(value, signals) else {
            return;
        };
        out.push(serde_json::json!({
            "kind": "signal_event",
            "target": CLIENT_PAGE_PATH,
            "source": action.origin_id,
            "state_key": action.state_key,
            "selector": tag,
            "event": event,
            "action": action.action,
            "span": {
                "file": value.span.file.index(),
                "start": value.span.range.start,
                "end": value.span.range.end,
            },
        }));
    });
}

pub(crate) fn client_event_attr_name(name: &str) -> Option<String> {
    let rest = name.strip_prefix("on")?;
    let mut chars = rest.chars();
    let first = chars.next()?;
    first
        .is_ascii_uppercase()
        .then(|| format!("{}{}", first.to_ascii_lowercase(), chars.as_str()))
}

pub(crate) fn client_signal_event_action(
    expr: &orv_hir::HirExpr,
    signals: &HashMap<orv_hir::NameId, ClientSignalDomSource>,
) -> Option<ClientSignalEventAction> {
    if let orv_hir::HirExprKind::Block(block) = &expr.kind {
        let [orv_hir::HirStmt::Expr(expr)] = block.stmts.as_slice() else {
            return None;
        };
        return client_signal_event_action(expr, signals);
    }
    if let orv_hir::HirExprKind::Lambda { body, .. } = &expr.kind {
        return client_signal_event_action_from_function_body(body, signals);
    }
    let orv_hir::HirExprKind::Assign { target, value } = &expr.kind else {
        return None;
    };
    let signal = signals.get(&target.id)?;
    let action = client_signal_assignment_action(target.id, value);
    Some(ClientSignalEventAction {
        origin_id: signal.origin_id.clone(),
        state_key: signal.state_key.clone(),
        action,
    })
}

pub(crate) fn client_signal_event_action_from_function_body(
    body: &orv_hir::HirFunctionBody,
    signals: &HashMap<orv_hir::NameId, ClientSignalDomSource>,
) -> Option<ClientSignalEventAction> {
    match body {
        orv_hir::HirFunctionBody::Expr(expr) => client_signal_event_action(expr, signals),
        orv_hir::HirFunctionBody::Block(block) => {
            let [orv_hir::HirStmt::Expr(expr)] = block.stmts.as_slice() else {
                return None;
            };
            client_signal_event_action(expr, signals)
        }
    }
}

pub(crate) fn client_signal_assignment_action(
    target: orv_hir::NameId,
    value: &orv_hir::HirExpr,
) -> serde_json::Value {
    if client_expr_is_event_target_value(value) {
        return serde_json::json!({
            "kind": "assign_event_target_value",
        });
    }
    if client_expr_is_event_target_checked(value) {
        return serde_json::json!({
            "kind": "assign_event_target_checked",
        });
    }
    if let Some(kind) = client_event_target_value_conversion_action(value) {
        return serde_json::json!({
            "kind": kind,
        });
    }
    if let orv_hir::HirExprKind::Unary {
        op: orv_hir::UnaryOp::Not,
        expr,
    } = &value.kind
    {
        if client_expr_is_ident(expr, target) {
            return serde_json::json!({
                "kind": "assign_toggle",
            });
        }
    }
    if let orv_hir::HirExprKind::Binary { op, lhs, rhs } = &value.kind {
        if client_expr_is_ident(lhs, target) {
            let kind = match op {
                orv_hir::BinaryOp::Add => Some("assign_add"),
                orv_hir::BinaryOp::Sub => Some("assign_sub"),
                _ => None,
            };
            if let Some(kind) = kind {
                return serde_json::json!({
                    "kind": kind,
                    "value": client_signal_initial_value_json(rhs),
                });
            }
        }
    }
    serde_json::json!({
        "kind": "assign",
        "value": client_signal_initial_value_json(value),
    })
}

pub(crate) fn client_expr_is_ident(expr: &orv_hir::HirExpr, id: orv_hir::NameId) -> bool {
    matches!(&expr.kind, orv_hir::HirExprKind::Ident(ident) if ident.id == id)
}

pub(crate) fn client_expr_is_event_target_value(expr: &orv_hir::HirExpr) -> bool {
    client_expr_is_event_target_field(expr, "value")
}

pub(crate) fn client_expr_is_event_target_checked(expr: &orv_hir::HirExpr) -> bool {
    client_expr_is_event_target_field(expr, "checked")
}

pub(crate) fn client_expr_is_event_target_field(
    expr: &orv_hir::HirExpr,
    expected_field: &str,
) -> bool {
    let orv_hir::HirExprKind::Field {
        target,
        field: value,
        ..
    } = &expr.kind
    else {
        return false;
    };
    if value != expected_field {
        return false;
    }
    let orv_hir::HirExprKind::Field {
        target: event,
        field,
        ..
    } = &target.kind
    else {
        return false;
    };
    field == "target" && matches!(event.kind, orv_hir::HirExprKind::Ident(_))
}

pub(crate) fn client_event_target_value_conversion_action(
    expr: &orv_hir::HirExpr,
) -> Option<&'static str> {
    let orv_hir::HirExprKind::Call { callee, args } = &expr.kind else {
        return None;
    };
    let [arg] = args.as_slice() else {
        return None;
    };
    if !client_expr_is_event_target_value(arg) {
        return None;
    }
    let orv_hir::HirExprKind::Field { target, field, .. } = &callee.kind else {
        return None;
    };
    if field != "from" {
        return None;
    }
    match &target.kind {
        orv_hir::HirExprKind::TypeName(name)
        | orv_hir::HirExprKind::Ident(orv_hir::HirIdent { name, .. }) => match name.as_str() {
            "float" => Some("assign_event_target_value_float"),
            "int" => Some("assign_event_target_value_int"),
            _ => None,
        },
        _ => None,
    }
}

pub(crate) fn collect_client_attr_bindings_for_tag(
    tag: &str,
    args: &[orv_hir::HirExpr],
    signals: &HashMap<orv_hir::NameId, ClientSignalDomSource>,
    out: &mut Vec<serde_json::Value>,
) {
    for_each_client_tag_attr_assignment(args, |target, value| {
        if client_event_attr_name(&target.name).is_some() {
            return;
        }
        let Some(binding) = client_signal_attr_binding(value, signals) else {
            return;
        };
        let mut attr_binding = serde_json::json!({
            "kind": "signal_attr",
            "target": CLIENT_PAGE_PATH,
            "source": binding.origin_id,
            "state_key": binding.state_key,
            "selector": tag,
            "attr": &target.name,
            "span": {
                "file": value.span.file.index(),
                "start": value.span.range.start,
                "end": value.span.range.end,
            },
        });
        if binding.signal_sources.len() > 1 {
            attr_binding
                .as_object_mut()
                .expect("signal attr binding is an object")
                .insert(
                    "state_keys".to_string(),
                    serde_json::Value::Array(
                        binding
                            .signal_sources
                            .iter()
                            .map(|source| serde_json::json!(&source.state_key))
                            .collect(),
                    ),
                );
            attr_binding
                .as_object_mut()
                .expect("signal attr binding is an object")
                .insert(
                    "sources".to_string(),
                    serde_json::Value::Array(
                        binding
                            .signal_sources
                            .iter()
                            .map(|source| {
                                serde_json::json!({
                                    "source": &source.origin_id,
                                    "state_key": &source.state_key,
                                })
                            })
                            .collect(),
                    ),
                );
        }
        if let Some(attr_template) = binding.attr_template {
            attr_binding
                .as_object_mut()
                .expect("signal attr binding is an object")
                .insert(
                    "attr_template".to_string(),
                    serde_json::Value::Array(attr_template),
                );
        }
        if let Some(attr_condition) = binding.attr_condition {
            attr_binding
                .as_object_mut()
                .expect("signal attr binding is an object")
                .insert("attr_condition".to_string(), attr_condition);
        }
        out.push(attr_binding);
    });
}

pub(crate) fn for_each_client_tag_attr_assignment<'a>(
    args: &'a [orv_hir::HirExpr],
    mut visit: impl FnMut(&'a orv_hir::HirIdent, &'a orv_hir::HirExpr),
) {
    for arg in args {
        match &arg.kind {
            orv_hir::HirExprKind::Assign { target, value } => visit(target, value),
            orv_hir::HirExprKind::Block(block) => {
                for stmt in &block.stmts {
                    let orv_hir::HirStmt::Expr(expr) = stmt else {
                        break;
                    };
                    let orv_hir::HirExprKind::Assign { target, value } = &expr.kind else {
                        break;
                    };
                    visit(target, value);
                }
            }
            _ => {}
        }
    }
}

pub(crate) fn collect_client_dom_bindings_for_tag(
    tag: &str,
    args: &[orv_hir::HirExpr],
    signals: &HashMap<orv_hir::NameId, ClientSignalDomSource>,
    out: &mut Vec<serde_json::Value>,
) {
    for arg in args {
        let Some(binding) = client_signal_text_binding(arg, signals) else {
            continue;
        };
        let mut value = serde_json::json!({
            "kind": "signal_text",
            "target": CLIENT_PAGE_PATH,
            "source": binding.origin_id,
            "state_key": binding.state_key,
            "selector": tag,
            "span": {
                "file": arg.span.file.index(),
                "start": arg.span.range.start,
                "end": arg.span.range.end,
            },
        });
        if binding.signal_sources.len() > 1 {
            value
                .as_object_mut()
                .expect("signal text binding is an object")
                .insert(
                    "state_keys".to_string(),
                    serde_json::Value::Array(
                        binding
                            .signal_sources
                            .iter()
                            .map(|source| serde_json::json!(&source.state_key))
                            .collect(),
                    ),
                );
            value
                .as_object_mut()
                .expect("signal text binding is an object")
                .insert(
                    "sources".to_string(),
                    serde_json::Value::Array(
                        binding
                            .signal_sources
                            .iter()
                            .map(|source| {
                                serde_json::json!({
                                    "source": &source.origin_id,
                                    "state_key": &source.state_key,
                                })
                            })
                            .collect(),
                    ),
                );
        }
        if let Some(text_template) = binding.text_template {
            value
                .as_object_mut()
                .expect("signal text binding is an object")
                .insert(
                    "text_template".to_string(),
                    serde_json::Value::Array(text_template),
                );
        }
        if let Some(text_condition) = binding.text_condition {
            value
                .as_object_mut()
                .expect("signal text binding is an object")
                .insert("text_condition".to_string(), text_condition);
        }
        out.push(value);
    }
}

pub(crate) fn client_signal_text_binding(
    expr: &orv_hir::HirExpr,
    signals: &HashMap<orv_hir::NameId, ClientSignalDomSource>,
) -> Option<ClientSignalTextBinding> {
    match &expr.kind {
        orv_hir::HirExprKind::Ident(ident) => {
            let signal = signals.get(&ident.id)?;
            Some(ClientSignalTextBinding {
                origin_id: signal.origin_id.clone(),
                state_key: signal.state_key.clone(),
                text_template: None,
                text_condition: None,
                signal_sources: vec![signal.clone()],
            })
        }
        orv_hir::HirExprKind::String(segments) => {
            client_signal_text_template_binding(segments, signals)
        }
        orv_hir::HirExprKind::If { .. } => client_signal_text_condition_binding(expr, signals),
        orv_hir::HirExprKind::Block(block) => {
            let [orv_hir::HirStmt::Expr(expr)] = block.stmts.as_slice() else {
                return None;
            };
            client_signal_text_binding(expr, signals)
        }
        _ => None,
    }
}

pub(crate) fn client_signal_text_template_binding(
    segments: &[orv_hir::HirStringSegment],
    signals: &HashMap<orv_hir::NameId, ClientSignalDomSource>,
) -> Option<ClientSignalTextBinding> {
    let mut sources: Vec<ClientSignalDomSource> = Vec::new();
    let mut text_template = Vec::new();
    for segment in segments {
        match segment {
            orv_hir::HirStringSegment::Str(text) => {
                if !text.is_empty() {
                    text_template.push(serde_json::json!({
                        "kind": "text",
                        "value": text,
                    }));
                }
            }
            orv_hir::HirStringSegment::Interp(expr) => {
                let orv_hir::HirExprKind::Ident(ident) = &expr.kind else {
                    return None;
                };
                let signal = signals.get(&ident.id)?;
                if !sources
                    .iter()
                    .any(|source| source.state_key == signal.state_key)
                {
                    sources.push(signal.clone());
                }
                text_template.push(serde_json::json!({
                    "kind": "signal",
                    "state_key": &signal.state_key,
                }));
            }
        }
    }
    let signal = sources.first()?;
    Some(ClientSignalTextBinding {
        origin_id: signal.origin_id.clone(),
        state_key: signal.state_key.clone(),
        text_template: Some(text_template),
        text_condition: None,
        signal_sources: sources,
    })
}

pub(crate) fn client_signal_text_condition_binding(
    expr: &orv_hir::HirExpr,
    signals: &HashMap<orv_hir::NameId, ClientSignalDomSource>,
) -> Option<ClientSignalTextBinding> {
    let orv_hir::HirExprKind::If {
        cond,
        then,
        else_branch,
    } = &expr.kind
    else {
        return None;
    };
    let (signal, mut text_condition) = client_signal_condition_json(cond, signals)?;
    let truthy = client_plain_string_block(then)?;
    let falsy = client_plain_string_expr(else_branch.as_deref()?)?;
    let condition = text_condition.as_object_mut()?;
    condition.insert("truthy".to_string(), serde_json::json!(truthy));
    condition.insert("falsy".to_string(), serde_json::json!(falsy));
    Some(ClientSignalTextBinding {
        origin_id: signal.origin_id.clone(),
        state_key: signal.state_key.clone(),
        text_template: None,
        text_condition: Some(text_condition),
        signal_sources: vec![signal.clone()],
    })
}

pub(crate) fn client_signal_attr_binding(
    expr: &orv_hir::HirExpr,
    signals: &HashMap<orv_hir::NameId, ClientSignalDomSource>,
) -> Option<ClientSignalAttrBinding> {
    if let Some(binding) = client_signal_attr_condition_binding(expr, signals) {
        return Some(binding);
    }
    if let Some(binding) = client_signal_text_binding(expr, signals) {
        return Some(ClientSignalAttrBinding {
            origin_id: binding.origin_id,
            state_key: binding.state_key,
            attr_template: binding.text_template,
            attr_condition: None,
            signal_sources: binding.signal_sources,
        });
    }
    None
}

pub(crate) fn client_signal_attr_condition_binding(
    expr: &orv_hir::HirExpr,
    signals: &HashMap<orv_hir::NameId, ClientSignalDomSource>,
) -> Option<ClientSignalAttrBinding> {
    let orv_hir::HirExprKind::If {
        cond,
        then,
        else_branch,
    } = &expr.kind
    else {
        return None;
    };
    let (signal, mut attr_condition) = client_signal_condition_json(cond, signals)?;
    let truthy = client_plain_string_block(then)?;
    let falsy = client_plain_string_expr(else_branch.as_deref()?)?;
    let condition = attr_condition.as_object_mut()?;
    condition.insert("truthy".to_string(), serde_json::json!(truthy));
    condition.insert("falsy".to_string(), serde_json::json!(falsy));
    Some(ClientSignalAttrBinding {
        origin_id: signal.origin_id.clone(),
        state_key: signal.state_key.clone(),
        attr_template: None,
        attr_condition: Some(attr_condition),
        signal_sources: vec![signal.clone()],
    })
}

pub(crate) fn client_signal_condition_json<'a>(
    expr: &orv_hir::HirExpr,
    signals: &'a HashMap<orv_hir::NameId, ClientSignalDomSource>,
) -> Option<(&'a ClientSignalDomSource, serde_json::Value)> {
    match &expr.kind {
        orv_hir::HirExprKind::Ident(ident) => {
            let signal = signals.get(&ident.id)?;
            Some((
                signal,
                serde_json::json!({
                    "state_key": &signal.state_key,
                }),
            ))
        }
        orv_hir::HirExprKind::Binary { op, lhs, rhs } => {
            client_signal_comparison_condition_json(*op, lhs, rhs, signals)
        }
        orv_hir::HirExprKind::Paren(inner) => client_signal_condition_json(inner, signals),
        _ => None,
    }
}

pub(crate) fn client_signal_comparison_condition_json<'a>(
    op: orv_hir::BinaryOp,
    lhs: &orv_hir::HirExpr,
    rhs: &orv_hir::HirExpr,
    signals: &'a HashMap<orv_hir::NameId, ClientSignalDomSource>,
) -> Option<(&'a ClientSignalDomSource, serde_json::Value)> {
    if let Some(signal) = client_signal_condition_ident(lhs, signals) {
        let op = client_signal_comparison_op(op)?;
        let rhs = client_signal_condition_operand_json(rhs)?;
        return Some((
            signal,
            serde_json::json!({
                "state_key": &signal.state_key,
                "op": op,
                "rhs": rhs,
            }),
        ));
    }
    let signal = client_signal_condition_ident(rhs, signals)?;
    let op = client_signal_inverted_comparison_op(op)?;
    let rhs = client_signal_condition_operand_json(lhs)?;
    Some((
        signal,
        serde_json::json!({
            "state_key": &signal.state_key,
            "op": op,
            "rhs": rhs,
        }),
    ))
}

pub(crate) fn client_signal_condition_ident<'a>(
    expr: &orv_hir::HirExpr,
    signals: &'a HashMap<orv_hir::NameId, ClientSignalDomSource>,
) -> Option<&'a ClientSignalDomSource> {
    match &expr.kind {
        orv_hir::HirExprKind::Ident(ident) => signals.get(&ident.id),
        orv_hir::HirExprKind::Paren(inner) => client_signal_condition_ident(inner, signals),
        _ => None,
    }
}

pub(crate) fn client_signal_condition_operand_json(
    expr: &orv_hir::HirExpr,
) -> Option<serde_json::Value> {
    match &expr.kind {
        orv_hir::HirExprKind::Integer(value) => Some(serde_json::json!({
            "kind": "int",
            "value": value,
        })),
        orv_hir::HirExprKind::Float(value) => Some(serde_json::json!({
            "kind": "float",
            "value": value,
        })),
        orv_hir::HirExprKind::String(segments)
            if segments
                .iter()
                .all(|segment| matches!(segment, orv_hir::HirStringSegment::Str(_))) =>
        {
            let value = segments
                .iter()
                .map(|segment| match segment {
                    orv_hir::HirStringSegment::Str(value) => value.as_str(),
                    orv_hir::HirStringSegment::Interp(_) => "",
                })
                .collect::<String>();
            Some(serde_json::json!({
                "kind": "string",
                "value": value,
            }))
        }
        orv_hir::HirExprKind::True => Some(serde_json::json!({
            "kind": "bool",
            "value": true,
        })),
        orv_hir::HirExprKind::False => Some(serde_json::json!({
            "kind": "bool",
            "value": false,
        })),
        orv_hir::HirExprKind::Paren(inner) => client_signal_condition_operand_json(inner),
        _ => None,
    }
}

pub(crate) fn client_signal_comparison_op(op: orv_hir::BinaryOp) -> Option<&'static str> {
    match op {
        orv_hir::BinaryOp::Eq => Some("eq"),
        orv_hir::BinaryOp::Ne => Some("ne"),
        orv_hir::BinaryOp::Lt => Some("lt"),
        orv_hir::BinaryOp::Gt => Some("gt"),
        orv_hir::BinaryOp::Le => Some("le"),
        orv_hir::BinaryOp::Ge => Some("ge"),
        _ => None,
    }
}

pub(crate) fn client_signal_inverted_comparison_op(op: orv_hir::BinaryOp) -> Option<&'static str> {
    match op {
        orv_hir::BinaryOp::Eq => Some("eq"),
        orv_hir::BinaryOp::Ne => Some("ne"),
        orv_hir::BinaryOp::Lt => Some("gt"),
        orv_hir::BinaryOp::Gt => Some("lt"),
        orv_hir::BinaryOp::Le => Some("ge"),
        orv_hir::BinaryOp::Ge => Some("le"),
        _ => None,
    }
}

pub(crate) fn client_plain_string_block(block: &orv_hir::HirBlock) -> Option<String> {
    let [orv_hir::HirStmt::Expr(expr)] = block.stmts.as_slice() else {
        return None;
    };
    client_plain_string_expr(expr)
}

pub(crate) fn client_plain_string_expr(expr: &orv_hir::HirExpr) -> Option<String> {
    match &expr.kind {
        orv_hir::HirExprKind::String(segments)
            if segments
                .iter()
                .all(|segment| matches!(segment, orv_hir::HirStringSegment::Str(_))) =>
        {
            Some(
                segments
                    .iter()
                    .map(|segment| match segment {
                        orv_hir::HirStringSegment::Str(text) => text.as_str(),
                        orv_hir::HirStringSegment::Interp(_) => "",
                    })
                    .collect(),
            )
        }
        orv_hir::HirExprKind::Block(block) => client_plain_string_block(block),
        _ => None,
    }
}

pub(crate) fn client_signal_initial_values(
    program: &orv_hir::HirProgram,
) -> HashMap<String, serde_json::Value> {
    program
        .items
        .iter()
        .filter_map(|stmt| {
            let orv_hir::HirStmt::Let(stmt) = stmt else {
                return None;
            };
            (stmt.kind == orv_hir::HirLetKind::Signal).then(|| {
                (
                    orv_hir::origin_id("signal", &stmt.name.name, stmt.span),
                    client_signal_initial_value_json(&stmt.init),
                )
            })
        })
        .collect()
}

pub(crate) fn client_signal_initial_value_json(expr: &orv_hir::HirExpr) -> serde_json::Value {
    match &expr.kind {
        orv_hir::HirExprKind::Integer(value) => {
            serde_json::json!({"kind": "int", "value": value})
        }
        orv_hir::HirExprKind::Float(value) => {
            serde_json::json!({"kind": "float", "value": value})
        }
        orv_hir::HirExprKind::String(segments)
            if segments
                .iter()
                .all(|segment| matches!(segment, orv_hir::HirStringSegment::Str(_))) =>
        {
            let value = segments
                .iter()
                .map(|segment| match segment {
                    orv_hir::HirStringSegment::Str(value) => value.as_str(),
                    orv_hir::HirStringSegment::Interp(_) => "",
                })
                .collect::<String>();
            serde_json::json!({"kind": "string", "value": value})
        }
        orv_hir::HirExprKind::True => serde_json::json!({"kind": "bool", "value": true}),
        orv_hir::HirExprKind::False => serde_json::json!({"kind": "bool", "value": false}),
        orv_hir::HirExprKind::Void => serde_json::json!({"kind": "void", "value": null}),
        _ => serde_json::json!({
            "kind": "dynamic",
            "span": {
                "file": expr.span.file.index(),
                "start": expr.span.range.start,
                "end": expr.span.range.end,
            },
        }),
    }
}

pub(crate) fn write_client_bundle_manifest(
    out: &Path,
    path: &str,
    entry: &Path,
    binding: &ClientSourceBinding<'_>,
    targets: &ClientBundleTargets<'_>,
) -> anyhow::Result<()> {
    let page = targets
        .page
        .ok_or_else(|| anyhow::anyhow!("missing client_page bundle target"))?;
    let loader = targets
        .js
        .ok_or_else(|| anyhow::anyhow!("missing client_js bundle target"))?;
    let wasm = targets
        .wasm
        .ok_or_else(|| anyhow::anyhow!("missing client_wasm bundle target"))?;
    let reactive_plan = targets
        .reactive_plan
        .ok_or_else(|| anyhow::anyhow!("missing client_reactive_plan bundle target"))?;
    let loader_hash = file_content_hash(&out.join(loader))?;
    let reactive_plan_value = read_json_value(&out.join(reactive_plan))?;
    let reactive_plan_hash = stable_json_hash(&reactive_plan_value)?;
    let wasm_hash = file_content_hash(&out.join(wasm))?;
    let manifest = serde_json::json!({
        "schema_version": 1,
        "kind": "orv.client.bundle",
        "entry": entry.display().to_string(),
        "reactive_plan": reactive_plan,
        "reactive_plan_hash": reactive_plan_hash,
        "page": page,
        "loader": loader,
        "loader_hash": loader_hash,
        "wasm": wasm,
        "wasm_hash": wasm_hash,
        "source_bundle": SOURCE_BUNDLE_PATH,
        "source_bundle_hash": binding.source_bundle_hash,
        "runtime_features": ["client_wasm"],
        "exports": {
            "start": CLIENT_WASM_START_EXPORT,
            "render_ptr": CLIENT_WASM_RENDER_PTR_EXPORT,
            "render_len": CLIENT_WASM_RENDER_LEN_EXPORT,
            "memory": CLIENT_WASM_MEMORY_EXPORT,
        },
        "initial_render": {
            "content_type": "text/html",
            "encoding": "utf-8",
            "html_hash": format!("{:016x}", fnv1a64(binding.initial_render.as_bytes())),
            "byte_length": binding.initial_render.len(),
        },
        "capabilities": client_bundle_capabilities_json(&reactive_plan_value),
        "blocked_by": ["dynamic-client-codegen", "reactive-dom-diff"],
        "blockers": client_manifest_blockers_json(),
    });
    write_json(&out.join(path), &manifest)
}

pub(crate) fn client_bundle_capabilities_json(
    reactive_plan: &serde_json::Value,
) -> serde_json::Value {
    let empty = Vec::new();
    let bindings = reactive_plan
        .get("bindings")
        .and_then(serde_json::Value::as_array)
        .unwrap_or(&empty);
    let signals = reactive_plan
        .get("signals")
        .and_then(serde_json::Value::as_array)
        .unwrap_or(&empty);
    let mut surfaces = BTreeSet::new();
    if client_binding_count(bindings, "initial_render") > 0 {
        surfaces.insert("wasm_initial_render");
    }
    surfaces.insert("embedded_reactive_plan");
    surfaces.insert("source_bundle_validation");
    if client_binding_count(bindings, "signal_state") > 0 {
        surfaces.insert("signal_state");
    }
    if client_binding_count(bindings, "signal_text") > 0 {
        surfaces.insert("signal_text");
    }
    if client_binding_has_field(bindings, "signal_text", "text_template") {
        surfaces.insert("signal_text_template");
    }
    if client_binding_has_field(bindings, "signal_text", "text_condition") {
        surfaces.insert("signal_text_condition");
    }
    if client_binding_count(bindings, "signal_attr") > 0 {
        surfaces.insert("signal_attr");
    }
    if client_binding_has_field(bindings, "signal_attr", "attr_template") {
        surfaces.insert("signal_attr_template");
    }
    if client_binding_has_field(bindings, "signal_attr", "attr_condition") {
        surfaces.insert("signal_attr_condition");
    }
    if client_binding_count(bindings, "signal_event") > 0 {
        surfaces.insert("signal_event");
    }
    serde_json::json!({
        "schema_version": 1,
        "runtime": "client_wasm",
        "source": CLIENT_REACTIVE_PLAN_PATH,
        "signals": signals.len(),
        "bindings": {
            "total": bindings.len(),
            "initial_render": client_binding_count(bindings, "initial_render"),
            "signal_state": client_binding_count(bindings, "signal_state"),
            "signal_text": client_binding_count(bindings, "signal_text"),
            "signal_attr": client_binding_count(bindings, "signal_attr"),
            "signal_event": client_binding_count(bindings, "signal_event"),
        },
        "surfaces": surfaces.into_iter().collect::<Vec<_>>(),
        "event_actions": client_event_action_kinds(bindings),
    })
}

pub(crate) fn client_binding_count(bindings: &[serde_json::Value], kind: &str) -> usize {
    bindings
        .iter()
        .filter(|binding| binding.get("kind").and_then(serde_json::Value::as_str) == Some(kind))
        .count()
}

pub(crate) fn client_binding_has_field(
    bindings: &[serde_json::Value],
    kind: &str,
    field: &str,
) -> bool {
    bindings.iter().any(|binding| {
        binding.get("kind").and_then(serde_json::Value::as_str) == Some(kind)
            && binding.get(field).is_some()
    })
}

pub(crate) fn client_event_action_kinds(bindings: &[serde_json::Value]) -> Vec<String> {
    let mut actions = BTreeSet::new();
    for binding in bindings {
        if binding.get("kind").and_then(serde_json::Value::as_str) != Some("signal_event") {
            continue;
        }
        if let Some(kind) = binding
            .pointer("/action/kind")
            .and_then(serde_json::Value::as_str)
        {
            actions.insert(kind.to_string());
        }
    }
    actions.into_iter().collect()
}

pub(crate) fn client_manifest_blockers_json() -> Vec<serde_json::Value> {
    vec![
        serde_json::json!({
            "id": "dynamic-client-codegen",
            "artifact": CLIENT_JS_PATH,
            "reason": "optimized source-to-JS client codegen is not emitted yet",
        }),
        serde_json::json!({
            "id": "reactive-dom-diff",
            "artifact": CLIENT_REACTIVE_PLAN_PATH,
            "reason": "full DOM diff codegen is not emitted yet",
        }),
    ]
}
pub(crate) const CLIENT_MANIFEST_PATH: &str = "client/manifest.json";
pub(crate) const CLIENT_REACTIVE_PLAN_PATH: &str = "client/reactive-plan.json";
pub(crate) const CLIENT_PAGE_PATH: &str = "pages/index.html";
pub(crate) const CLIENT_JS_PATH: &str = "client/app.js";
pub(crate) const CLIENT_WASM_PATH: &str = "client/app.wasm";
pub(crate) const CLIENT_WASM_SOURCE_BUNDLE_PATH: &str = "../source-bundle.json";
pub(crate) const CLIENT_JS_LOADER_TEMPLATE: &str = include_str!("../client_loader_template.js");

pub(crate) fn write_client_wasm_bundle(
    path: &Path,
    source_bundle: &orv_compiler::SourceBundleArtifact,
    source_bundle_hash: &str,
    initial_render: &str,
) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| anyhow::anyhow!("failed to create {}: {e}", parent.display()))?;
    }
    std::fs::write(
        path,
        client_wasm_bundle_bytes(source_bundle, source_bundle_hash, initial_render)?,
    )
    .map_err(|e| anyhow::anyhow!("failed to write {}: {e}", path.display()))
}

pub(crate) fn client_wasm_bundle_bytes(
    source_bundle: &orv_compiler::SourceBundleArtifact,
    source_bundle_hash: &str,
    initial_render: &str,
) -> anyhow::Result<Vec<u8>> {
    let render_bytes = initial_render.as_bytes();
    let render_len = i32::try_from(render_bytes.len())
        .map_err(|_| anyhow::anyhow!("client initial render exceeds wasm32 i32 length"))?;
    let mut bytes = WASM_MODULE_HEADER.to_vec();
    let mut custom_section = Vec::new();
    push_wasm_len(&mut custom_section, CLIENT_WASM_CUSTOM_SECTION_NAME.len());
    custom_section.extend_from_slice(CLIENT_WASM_CUSTOM_SECTION_NAME.as_bytes());
    let payload = client_wasm_metadata_json(source_bundle, source_bundle_hash, initial_render);
    custom_section.extend_from_slice(payload.as_bytes());

    bytes.push(0);
    push_wasm_len(&mut bytes, custom_section.len());
    bytes.extend(custom_section);

    let mut type_section = Vec::new();
    push_wasm_u32_leb(&mut type_section, 2);
    type_section.push(0x60);
    push_wasm_u32_leb(&mut type_section, 0);
    push_wasm_u32_leb(&mut type_section, 0);
    type_section.push(0x60);
    push_wasm_u32_leb(&mut type_section, 0);
    push_wasm_u32_leb(&mut type_section, 1);
    type_section.push(0x7f);
    push_wasm_section(&mut bytes, 1, &type_section);

    let mut function_section = Vec::new();
    push_wasm_u32_leb(&mut function_section, 3);
    push_wasm_u32_leb(&mut function_section, 0);
    push_wasm_u32_leb(&mut function_section, 1);
    push_wasm_u32_leb(&mut function_section, 1);
    push_wasm_section(&mut bytes, 3, &function_section);

    let mut memory_section = Vec::new();
    push_wasm_u32_leb(&mut memory_section, 1);
    memory_section.push(0x00);
    push_wasm_u32_leb(&mut memory_section, wasm_min_pages(render_bytes.len())?);
    push_wasm_section(&mut bytes, 5, &memory_section);

    let mut export_section = Vec::new();
    push_wasm_u32_leb(&mut export_section, 4);
    push_wasm_len(&mut export_section, CLIENT_WASM_START_EXPORT.len());
    export_section.extend_from_slice(CLIENT_WASM_START_EXPORT.as_bytes());
    export_section.push(0);
    push_wasm_u32_leb(&mut export_section, 0);
    push_wasm_len(&mut export_section, CLIENT_WASM_RENDER_PTR_EXPORT.len());
    export_section.extend_from_slice(CLIENT_WASM_RENDER_PTR_EXPORT.as_bytes());
    export_section.push(0);
    push_wasm_u32_leb(&mut export_section, 1);
    push_wasm_len(&mut export_section, CLIENT_WASM_RENDER_LEN_EXPORT.len());
    export_section.extend_from_slice(CLIENT_WASM_RENDER_LEN_EXPORT.as_bytes());
    export_section.push(0);
    push_wasm_u32_leb(&mut export_section, 2);
    push_wasm_len(&mut export_section, CLIENT_WASM_MEMORY_EXPORT.len());
    export_section.extend_from_slice(CLIENT_WASM_MEMORY_EXPORT.as_bytes());
    export_section.push(2);
    push_wasm_u32_leb(&mut export_section, 0);
    push_wasm_section(&mut bytes, 7, &export_section);

    let mut code_section = Vec::new();
    push_wasm_u32_leb(&mut code_section, 3);
    push_wasm_u32_leb(&mut code_section, 2);
    push_wasm_u32_leb(&mut code_section, 0);
    code_section.push(0x0b);
    push_wasm_const_i32_function(&mut code_section, 0);
    push_wasm_const_i32_function(&mut code_section, render_len);
    push_wasm_section(&mut bytes, 10, &code_section);
    if !render_bytes.is_empty() {
        let mut data_section = Vec::new();
        push_wasm_u32_leb(&mut data_section, 1);
        data_section.push(0x00);
        data_section.push(0x41);
        push_wasm_u32_leb(&mut data_section, 0);
        data_section.push(0x0b);
        push_wasm_len(&mut data_section, render_bytes.len());
        data_section.extend_from_slice(render_bytes);
        push_wasm_section(&mut bytes, 11, &data_section);
    }
    Ok(bytes)
}

pub(crate) fn client_wasm_metadata_json(
    source_bundle: &orv_compiler::SourceBundleArtifact,
    source_bundle_hash: &str,
    initial_render: &str,
) -> String {
    serde_json::json!({
        "schema_version": 1,
        "runtime_features": ["client_wasm"],
        "source_bundle": CLIENT_WASM_SOURCE_BUNDLE_PATH,
        "source_bundle_hash": source_bundle_hash,
        "entry": &source_bundle.entry,
        "initial_render": {
            "content_type": "text/html",
            "encoding": "utf-8",
            "html_hash": format!("{:016x}", fnv1a64(initial_render.as_bytes())),
            "byte_length": initial_render.len(),
            "ptr_export": CLIENT_WASM_RENDER_PTR_EXPORT,
            "len_export": CLIENT_WASM_RENDER_LEN_EXPORT,
            "memory_export": CLIENT_WASM_MEMORY_EXPORT,
        },
    })
    .to_string()
}

pub(crate) fn write_client_js_loader(
    path: &Path,
    entry: &Path,
    binding: &ClientSourceBinding<'_>,
) -> anyhow::Result<()> {
    let reactive_plan = client_reactive_plan_json(entry, binding);
    let script = client_js_loader_script(
        binding.source_bundle,
        binding.source_bundle_hash,
        &reactive_plan,
    )?;
    write_text(path, &script)
}

pub(crate) fn client_js_loader_script(
    source_bundle: &orv_compiler::SourceBundleArtifact,
    source_bundle_hash: &str,
    reactive_plan: &serde_json::Value,
) -> anyhow::Result<String> {
    let reactive_plan_hash = stable_json_hash(reactive_plan)?;
    let bootstrap = serde_json::to_string_pretty(&serde_json::json!({
        "schemaVersion": 1,
        "runtimeFeatures": ["client_wasm"],
        "manifestUrl": "./manifest.json",
        "reactivePlanUrl": "./reactive-plan.json",
        "wasmUrl": "./app.wasm",
        "manifestReactivePlan": CLIENT_REACTIVE_PLAN_PATH,
        "manifestWasm": CLIENT_WASM_PATH,
        "sourceBundleUrl": "../source-bundle.json",
        "manifestSourceBundle": SOURCE_BUNDLE_PATH,
        "sourceBundleHash": source_bundle_hash,
        "sourceFileCount": source_bundle.files.len(),
        "entry": &source_bundle.entry,
        "embeddedReactivePlan": reactive_plan,
        "embeddedReactivePlanHash": reactive_plan_hash,
        "exports": {
            "start": CLIENT_WASM_START_EXPORT,
            "renderPtr": CLIENT_WASM_RENDER_PTR_EXPORT,
            "renderLen": CLIENT_WASM_RENDER_LEN_EXPORT,
            "memory": CLIENT_WASM_MEMORY_EXPORT,
        },
    }))?;
    Ok(CLIENT_JS_LOADER_TEMPLATE.replace("__ORV_BOOTSTRAP__", &bootstrap))
}

pub(crate) fn write_client_page_shell(
    path: &Path,
    entry: &Path,
    loader_src: &str,
) -> anyhow::Result<()> {
    let entry = html_attr_escape(&entry.display().to_string());
    let loader_src = html_attr_escape(loader_src);
    let html = format!(
        r#"<!doctype html>
<html>
<head>
<meta charset="utf-8">
<meta name="orv-runtime" content="client_wasm">
</head>
<body data-orv-client="wasm" data-orv-entry="{entry}">
<div id="orv-root"></div>
<script type="module" src="{loader_src}"></script>
</body>
</html>"#
    );
    write_text(path, &html)
}

pub(crate) fn html_attr_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

pub(crate) fn prod_deploy_client_json(
    out: &Path,
    enabled: bool,
    targets: ProdBuildTargets<'_>,
) -> anyhow::Result<serde_json::Value> {
    if !enabled {
        return Ok(serde_json::Value::Null);
    }
    let client_manifest = targets
        .client_manifest
        .ok_or_else(|| anyhow::anyhow!("missing client_manifest bundle target"))?;
    let client_manifest_value = read_json_value(&out.join(client_manifest))?;
    let reactive_plan = targets
        .client_reactive_plan
        .ok_or_else(|| anyhow::anyhow!("missing client_reactive_plan bundle target"))?;
    if json_str(&client_manifest_value, "reactive_plan", "client manifest")? != reactive_plan {
        anyhow::bail!("client manifest reactive_plan does not match bundle target");
    }
    Ok(serde_json::json!({
        "manifest": client_manifest,
        "reactive_plan": reactive_plan,
        "page": targets.client_page.ok_or_else(|| anyhow::anyhow!("missing client_page bundle target"))?,
        "loader": targets.client_js.ok_or_else(|| anyhow::anyhow!("missing client_js bundle target"))?,
        "wasm": targets.client_wasm.ok_or_else(|| anyhow::anyhow!("missing client_wasm bundle target"))?,
        "runtime_features": ["client_wasm"],
        "capabilities": client_manifest_value
            .get("capabilities")
            .cloned()
            .unwrap_or(serde_json::Value::Null),
        "blocked_by": client_manifest_value
            .get("blocked_by")
            .cloned()
            .unwrap_or_else(|| serde_json::json!([])),
        "blockers": client_manifest_value
            .get("blockers")
            .cloned()
            .unwrap_or_else(|| serde_json::json!([])),
    }))
}

pub(crate) fn deploy_preflight_client_value(
    client: Option<&serde_json::Value>,
) -> serde_json::Value {
    let Some(client) = client.filter(|value| !value.is_null()) else {
        return serde_json::Value::Null;
    };
    serde_json::json!({
        "manifest": client.get("manifest").and_then(serde_json::Value::as_str),
        "page": client.get("page").and_then(serde_json::Value::as_str),
        "loader": client.get("loader").and_then(serde_json::Value::as_str),
        "wasm": client.get("wasm").and_then(serde_json::Value::as_str),
        "runtime_features": client.get("runtime_features").cloned().unwrap_or_else(|| serde_json::json!([])),
        "capabilities": client.get("capabilities").cloned().unwrap_or(serde_json::Value::Null),
        "blocked_by": client.get("blocked_by").cloned().unwrap_or_else(|| serde_json::json!([])),
        "blockers": client.get("blockers").cloned().unwrap_or_else(|| serde_json::json!([])),
    })
}

pub(crate) fn deploy_runbook_client_section(client: &serde_json::Value) -> String {
    if client.is_null() {
        return String::new();
    }
    let manifest = json_str_or_empty(client, "manifest");
    let reactive_plan = json_str_or_empty(client, "reactive_plan");
    let page = json_str_or_empty(client, "page");
    let loader = json_str_or_empty(client, "loader");
    let wasm = json_str_or_empty(client, "wasm");
    let runtime = client
        .pointer("/capabilities/runtime")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("client_wasm");
    let surfaces = client
        .pointer("/capabilities/surfaces")
        .and_then(serde_json::Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(serde_json::Value::as_str)
                .collect::<Vec<_>>()
                .join(", ")
        })
        .filter(|items| !items.is_empty())
        .unwrap_or_else(|| "none".to_string());
    let mut blockers = String::new();
    for blocker in client
        .get("blockers")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
    {
        let id = json_str_or_empty(blocker, "id");
        let artifact = json_str_or_empty(blocker, "artifact");
        let reason = json_str_or_empty(blocker, "reason");
        let _ = writeln!(blockers, "- Client blocker: {id} {artifact} {reason}");
    }
    format!(
        r#"## Client Bundle

- Client manifest: {manifest}
- Client reactive plan: {reactive_plan}
- Client page: {page}
- Client loader: {loader}
- Client WASM: {wasm}
- Client runtime: {runtime}
- Client capability surfaces: {surfaces}
{blockers}
"#
    )
}
