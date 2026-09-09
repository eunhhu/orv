use super::*;

pub(crate) fn verify_client_page_target(
    bundle: &serde_json::Value,
    target: &Path,
) -> anyhow::Result<()> {
    let runtime_features = bundle
        .get("runtime_features")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| anyhow::anyhow!("client_page runtime_features must be an array"))?;
    if !runtime_features
        .iter()
        .any(|feature| feature == "client_wasm")
    {
        anyhow::bail!("client_page bundle must declare client_wasm");
    }
    verify_client_page_file(target)
}

pub(crate) fn verify_client_manifest_target(
    dir: &Path,
    bundle: &serde_json::Value,
    target: &Path,
) -> anyhow::Result<()> {
    if json_str(bundle, "path", "client_manifest bundle")? != CLIENT_MANIFEST_PATH {
        anyhow::bail!("client_manifest bundle path must be {CLIENT_MANIFEST_PATH}");
    }
    let runtime_features = bundle
        .get("runtime_features")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| anyhow::anyhow!("client_manifest runtime_features must be an array"))?;
    if !runtime_features
        .iter()
        .any(|feature| feature == "client_wasm")
    {
        anyhow::bail!("client_manifest bundle must declare client_wasm");
    }
    let manifest = read_json_value(target)?;
    verify_client_manifest_value(dir, &manifest)
}

pub(crate) fn verify_client_manifest_value(
    dir: &Path,
    manifest: &serde_json::Value,
) -> anyhow::Result<()> {
    verify_client_manifest_contract_keys(manifest)?;
    if manifest
        .get("schema_version")
        .and_then(serde_json::Value::as_u64)
        != Some(1)
    {
        anyhow::bail!("client_manifest schema_version must be 1");
    }
    if json_str(manifest, "kind", "client manifest")? != "orv.client.bundle" {
        anyhow::bail!("client_manifest kind must be orv.client.bundle");
    }
    verify_client_manifest_paths(dir, manifest)?;
    verify_client_manifest_source_binding(dir, manifest)?;
    verify_client_manifest_artifact_hashes(dir, manifest)?;
    verify_client_manifest_capabilities(dir, manifest)?;
    verify_client_manifest_wasm_hash(dir, manifest)?;
    verify_client_manifest_exports(manifest)?;
    verify_client_manifest_initial_render(dir, manifest)?;
    verify_client_blocker_details(manifest, "client_manifest")
}

pub(crate) fn verify_client_manifest_contract_keys(
    manifest: &serde_json::Value,
) -> anyhow::Result<()> {
    verify_json_object_keys_exact(
        manifest,
        &[
            "schema_version",
            "kind",
            "entry",
            "page",
            "reactive_plan",
            "reactive_plan_hash",
            "loader",
            "loader_hash",
            "wasm",
            "wasm_hash",
            "source_bundle",
            "source_bundle_hash",
            "exports",
            "initial_render",
            "runtime_features",
            "capabilities",
            "blocked_by",
            "blockers",
        ],
        "client_manifest",
    )?;
    verify_json_object_keys_exact(
        manifest
            .get("exports")
            .ok_or_else(|| anyhow::anyhow!("client_manifest exports must be an object"))?,
        &["start", "render_ptr", "render_len", "memory"],
        "client_manifest exports",
    )?;
    verify_json_object_keys_exact(
        manifest
            .get("initial_render")
            .ok_or_else(|| anyhow::anyhow!("client_manifest initial_render must be an object"))?,
        &["content_type", "encoding", "html_hash", "byte_length"],
        "client_manifest initial_render",
    )?;
    verify_client_capabilities_contract_keys(
        manifest
            .get("capabilities")
            .ok_or_else(|| anyhow::anyhow!("client_manifest capabilities must be an object"))?,
        "client_manifest capabilities",
    )?;
    verify_client_blockers_contract_keys(manifest, "client_manifest")
}

pub(crate) fn verify_client_capabilities_contract_keys(
    capabilities: &serde_json::Value,
    context: &str,
) -> anyhow::Result<()> {
    verify_json_object_keys_exact(
        capabilities,
        &[
            "schema_version",
            "runtime",
            "source",
            "signals",
            "bindings",
            "surfaces",
            "event_actions",
        ],
        context,
    )?;
    verify_json_object_keys_exact(
        capabilities
            .get("bindings")
            .ok_or_else(|| anyhow::anyhow!("{context} bindings must be an object"))?,
        &[
            "total",
            "initial_render",
            "signal_state",
            "signal_text",
            "signal_attr",
            "signal_event",
        ],
        &format!("{context} bindings"),
    )
}

pub(crate) fn verify_client_manifest_paths(
    dir: &Path,
    manifest: &serde_json::Value,
) -> anyhow::Result<()> {
    let reactive_plan = json_str(manifest, "reactive_plan", "client manifest")?;
    if reactive_plan != CLIENT_REACTIVE_PLAN_PATH || !dir.join(reactive_plan).is_file() {
        anyhow::bail!("client_manifest reactive_plan must be {CLIENT_REACTIVE_PLAN_PATH}");
    }
    let page = json_str(manifest, "page", "client manifest")?;
    if page != CLIENT_PAGE_PATH || !dir.join(page).is_file() {
        anyhow::bail!("client_manifest page must be {CLIENT_PAGE_PATH}");
    }
    let loader = json_str(manifest, "loader", "client manifest")?;
    if loader != CLIENT_JS_PATH || !dir.join(loader).is_file() {
        anyhow::bail!("client_manifest loader must be {CLIENT_JS_PATH}");
    }
    let wasm = json_str(manifest, "wasm", "client manifest")?;
    if wasm != CLIENT_WASM_PATH || !dir.join(wasm).is_file() {
        anyhow::bail!("client_manifest wasm must be {CLIENT_WASM_PATH}");
    }
    Ok(())
}

pub(crate) fn verify_client_manifest_source_binding(
    dir: &Path,
    manifest: &serde_json::Value,
) -> anyhow::Result<()> {
    if json_str(manifest, "source_bundle", "client manifest")? != SOURCE_BUNDLE_PATH {
        anyhow::bail!("client_manifest source_bundle must be {SOURCE_BUNDLE_PATH}");
    }
    let source_bundle = read_json_value(&dir.join(SOURCE_BUNDLE_PATH))?;
    let expected_hash = stable_json_hash(&source_bundle)?;
    if json_str(manifest, "source_bundle_hash", "client manifest")? != expected_hash {
        anyhow::bail!("client_manifest source_bundle_hash does not match source bundle");
    }
    if manifest.get("entry") != source_bundle.get("entry") {
        anyhow::bail!("client_manifest entry does not match source bundle");
    }
    if !manifest
        .get("runtime_features")
        .and_then(serde_json::Value::as_array)
        .is_some_and(|features| features.iter().any(|feature| feature == "client_wasm"))
    {
        anyhow::bail!("client_manifest runtime_features must include client_wasm");
    }
    Ok(())
}

pub(crate) fn verify_client_manifest_wasm_hash(
    dir: &Path,
    manifest: &serde_json::Value,
) -> anyhow::Result<()> {
    let wasm = json_str(manifest, "wasm", "client manifest")?;
    let expected_hash = file_content_hash(&dir.join(wasm))?;
    if json_str(manifest, "wasm_hash", "client manifest")? != expected_hash {
        anyhow::bail!("client_manifest wasm_hash does not match wasm bundle");
    }
    Ok(())
}

pub(crate) fn verify_client_manifest_artifact_hashes(
    dir: &Path,
    manifest: &serde_json::Value,
) -> anyhow::Result<()> {
    let loader = json_str(manifest, "loader", "client manifest")?;
    let expected_loader_hash = file_content_hash(&dir.join(loader))?;
    if json_str(manifest, "loader_hash", "client manifest")? != expected_loader_hash {
        anyhow::bail!("client_manifest loader_hash does not match loader");
    }
    let reactive_plan = json_str(manifest, "reactive_plan", "client manifest")?;
    let reactive_plan = read_json_value(&dir.join(reactive_plan))?;
    let expected_reactive_plan_hash = stable_json_hash(&reactive_plan)?;
    if json_str(manifest, "reactive_plan_hash", "client manifest")? != expected_reactive_plan_hash {
        anyhow::bail!("client_manifest reactive_plan_hash does not match reactive plan");
    }
    Ok(())
}

pub(crate) fn verify_client_manifest_capabilities(
    dir: &Path,
    manifest: &serde_json::Value,
) -> anyhow::Result<()> {
    let reactive_plan = json_str(manifest, "reactive_plan", "client manifest")?;
    let reactive_plan = read_json_value(&dir.join(reactive_plan))?;
    verify_client_reactive_plan_value(dir, &reactive_plan)?;
    let expected = client_bundle_capabilities_json(&reactive_plan);
    if manifest.get("capabilities") != Some(&expected) {
        anyhow::bail!("client_manifest capabilities do not match reactive plan");
    }
    Ok(())
}

pub(crate) fn verify_client_manifest_exports(manifest: &serde_json::Value) -> anyhow::Result<()> {
    let exports = manifest
        .get("exports")
        .ok_or_else(|| anyhow::anyhow!("client_manifest exports must be an object"))?;
    if json_str(exports, "start", "client manifest exports")? != CLIENT_WASM_START_EXPORT
        || json_str(exports, "render_ptr", "client manifest exports")?
            != CLIENT_WASM_RENDER_PTR_EXPORT
        || json_str(exports, "render_len", "client manifest exports")?
            != CLIENT_WASM_RENDER_LEN_EXPORT
        || json_str(exports, "memory", "client manifest exports")? != CLIENT_WASM_MEMORY_EXPORT
    {
        anyhow::bail!("client_manifest exports do not match client WASM ABI");
    }
    Ok(())
}

pub(crate) fn verify_client_manifest_initial_render(
    dir: &Path,
    manifest: &serde_json::Value,
) -> anyhow::Result<()> {
    let manifest_initial_render = manifest
        .get("initial_render")
        .ok_or_else(|| anyhow::anyhow!("client_manifest initial_render must be an object"))?;
    let wasm = json_str(manifest, "wasm", "client manifest")?;
    let wasm_metadata = client_wasm_metadata_value(&dir.join(wasm))?;
    let wasm_initial_render = wasm_metadata
        .get("initial_render")
        .ok_or_else(|| anyhow::anyhow!("client_wasm ORV metadata missing initial_render"))?;
    for field in ["content_type", "encoding", "html_hash", "byte_length"] {
        if manifest_initial_render.get(field) != wasm_initial_render.get(field) {
            anyhow::bail!("client_manifest initial_render does not match client WASM metadata");
        }
    }
    Ok(())
}

pub(crate) fn verify_client_reactive_plan_target(
    dir: &Path,
    bundle: &serde_json::Value,
    target: &Path,
) -> anyhow::Result<()> {
    if json_str(bundle, "path", "client_reactive_plan bundle")? != CLIENT_REACTIVE_PLAN_PATH {
        anyhow::bail!("client_reactive_plan bundle path must be {CLIENT_REACTIVE_PLAN_PATH}");
    }
    let runtime_features = bundle
        .get("runtime_features")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| anyhow::anyhow!("client_reactive_plan runtime_features must be an array"))?;
    if !runtime_features
        .iter()
        .any(|feature| feature == "client_wasm")
    {
        anyhow::bail!("client_reactive_plan bundle must declare client_wasm");
    }
    let plan = read_json_value(target)?;
    verify_client_reactive_plan_value(dir, &plan)
}

pub(crate) fn verify_client_reactive_plan_value(
    dir: &Path,
    plan: &serde_json::Value,
) -> anyhow::Result<()> {
    verify_client_reactive_plan_contract_keys(plan)?;
    if plan
        .get("schema_version")
        .and_then(serde_json::Value::as_u64)
        != Some(1)
    {
        anyhow::bail!("client_reactive_plan schema_version must be 1");
    }
    if json_str(plan, "kind", "client reactive plan")? != "orv.client.reactive_plan" {
        anyhow::bail!("client_reactive_plan kind must be orv.client.reactive_plan");
    }
    if json_str(plan, "source_bundle", "client reactive plan")? != SOURCE_BUNDLE_PATH {
        anyhow::bail!("client_reactive_plan source_bundle must be {SOURCE_BUNDLE_PATH}");
    }
    let source_bundle = read_json_value(&dir.join(SOURCE_BUNDLE_PATH))?;
    let expected_hash = stable_json_hash(&source_bundle)?;
    if json_str(plan, "source_bundle_hash", "client reactive plan")? != expected_hash {
        anyhow::bail!("client_reactive_plan source_bundle_hash does not match source bundle");
    }
    if plan.get("entry") != source_bundle.get("entry") {
        anyhow::bail!("client_reactive_plan entry does not match source bundle");
    }
    if !plan
        .get("runtime_features")
        .and_then(serde_json::Value::as_array)
        .is_some_and(|features| features.iter().any(|feature| feature == "client_wasm"))
    {
        anyhow::bail!("client_reactive_plan runtime_features must include client_wasm");
    }
    let signals = plan
        .get("signals")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| anyhow::anyhow!("client_reactive_plan signals must be an array"))?;
    if !signals.iter().all(|signal| {
        signal
            .get("name")
            .and_then(serde_json::Value::as_str)
            .is_some()
            && signal
                .get("origin_id")
                .and_then(serde_json::Value::as_str)
                .is_some()
            && signal
                .get("state_key")
                .and_then(serde_json::Value::as_str)
                .is_some()
            && signal
                .get("initial_value")
                .and_then(|value| value.get("kind"))
                .and_then(serde_json::Value::as_str)
                .is_some()
    }) {
        anyhow::bail!("client_reactive_plan signals must be an array of source-backed signals");
    }
    let bindings = plan
        .get("bindings")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| anyhow::anyhow!("client_reactive_plan bindings must be an array"))?;
    verify_client_reactive_plan_initial_render_binding(dir, bindings)?;
    if !signals.iter().all(|signal| {
        let origin_id = signal
            .get("origin_id")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("");
        let state_key = signal
            .get("state_key")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("");
        bindings.iter().any(|binding| {
            binding.get("kind").and_then(serde_json::Value::as_str) == Some("signal_state")
                && binding.get("target").and_then(serde_json::Value::as_str) == Some(CLIENT_JS_PATH)
                && binding.get("source").and_then(serde_json::Value::as_str) == Some(origin_id)
                && binding.get("state_key").and_then(serde_json::Value::as_str) == Some(state_key)
        })
    }) {
        anyhow::bail!("client_reactive_plan signal_state binding is missing");
    }
    if !client_reactive_plan_signal_text_bindings_are_valid(signals, bindings) {
        anyhow::bail!("client_reactive_plan signal_text binding is invalid");
    }
    if !client_reactive_plan_signal_attr_bindings_are_valid(signals, bindings) {
        anyhow::bail!("client_reactive_plan signal_attr binding is invalid");
    }
    if !client_reactive_plan_signal_event_bindings_are_valid(signals, bindings) {
        anyhow::bail!("client_reactive_plan signal_event binding is invalid");
    }
    if !plan
        .get("blocked_by")
        .and_then(serde_json::Value::as_array)
        .is_some_and(|items| items.iter().any(|item| item == "reactive-dom-diff"))
    {
        anyhow::bail!("client_reactive_plan blocked_by must include reactive-dom-diff");
    }
    verify_client_blocker_details(plan, "client_reactive_plan")?;
    Ok(())
}

pub(crate) fn verify_client_reactive_plan_contract_keys(
    plan: &serde_json::Value,
) -> anyhow::Result<()> {
    verify_json_object_keys_exact(
        plan,
        &[
            "schema_version",
            "kind",
            "entry",
            "source_bundle",
            "source_bundle_hash",
            "runtime_features",
            "signals",
            "bindings",
            "blocked_by",
            "blockers",
        ],
        "client_reactive_plan",
    )?;
    let signals = plan
        .get("signals")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| anyhow::anyhow!("client_reactive_plan signals must be an array"))?;
    for (index, signal) in signals.iter().enumerate() {
        let context = format!("client_reactive_plan signals[{index}]");
        verify_json_object_keys_exact(
            signal,
            &["origin_id", "name", "state_key", "initial_value", "span"],
            &context,
        )?;
        verify_client_value_contract_keys(
            signal.get("initial_value").ok_or_else(|| {
                anyhow::anyhow!("client_reactive_plan signals[{index}].initial_value is missing")
            })?,
            &format!("client_reactive_plan signals[{index}].initial_value"),
        )?;
        verify_client_span_contract_keys(
            signal.get("span").ok_or_else(|| {
                anyhow::anyhow!("client_reactive_plan signals[{index}].span is missing")
            })?,
            &format!("client_reactive_plan signals[{index}].span"),
        )?;
    }
    let bindings = plan
        .get("bindings")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| anyhow::anyhow!("client_reactive_plan bindings must be an array"))?;
    for (index, binding) in bindings.iter().enumerate() {
        verify_client_reactive_plan_binding_contract_keys(binding, index)?;
    }
    verify_client_blockers_contract_keys(plan, "client_reactive_plan")
}

pub(crate) fn verify_client_reactive_plan_binding_contract_keys(
    binding: &serde_json::Value,
    index: usize,
) -> anyhow::Result<()> {
    let context = format!("client_reactive_plan bindings[{index}]");
    let kind = binding
        .get("kind")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("{context} kind must be a string"))?;
    match kind {
        "initial_render" => verify_json_object_keys_exact(
            binding,
            &["kind", "source", "target", "html_hash", "byte_length"],
            &context,
        ),
        "signal_state" => verify_json_object_keys_exact(
            binding,
            &["kind", "source", "target", "state_key"],
            &context,
        ),
        "signal_text" => {
            verify_json_object_keys_allowing_optional(
                binding,
                &["kind", "source", "target", "selector", "state_key", "span"],
                &["state_keys", "sources", "text_template", "text_condition"],
                &context,
            )?;
            verify_client_span_contract_keys(
                binding
                    .get("span")
                    .ok_or_else(|| anyhow::anyhow!("{context}.span is missing"))?,
                &format!("{context}.span"),
            )?;
            verify_client_binding_sources_contract_keys(binding, &context)?;
            verify_client_template_contract_keys(binding, "text_template", &context)?;
            verify_client_condition_contract_keys(binding, "text_condition", &context)
        }
        "signal_attr" => {
            verify_json_object_keys_allowing_optional(
                binding,
                &[
                    "kind",
                    "source",
                    "target",
                    "selector",
                    "state_key",
                    "attr",
                    "span",
                ],
                &["state_keys", "sources", "attr_template", "attr_condition"],
                &context,
            )?;
            verify_client_span_contract_keys(
                binding
                    .get("span")
                    .ok_or_else(|| anyhow::anyhow!("{context}.span is missing"))?,
                &format!("{context}.span"),
            )?;
            verify_client_binding_sources_contract_keys(binding, &context)?;
            verify_client_template_contract_keys(binding, "attr_template", &context)?;
            verify_client_condition_contract_keys(binding, "attr_condition", &context)
        }
        "signal_event" => {
            verify_json_object_keys_exact(
                binding,
                &[
                    "kind",
                    "source",
                    "target",
                    "selector",
                    "state_key",
                    "event",
                    "action",
                    "span",
                ],
                &context,
            )?;
            verify_client_span_contract_keys(
                binding
                    .get("span")
                    .ok_or_else(|| anyhow::anyhow!("{context}.span is missing"))?,
                &format!("{context}.span"),
            )?;
            verify_client_event_action_contract_keys(
                binding
                    .get("action")
                    .ok_or_else(|| anyhow::anyhow!("{context}.action is missing"))?,
                &format!("{context}.action"),
            )
        }
        _ => anyhow::bail!("{context} kind must be a supported client binding kind"),
    }
}

pub(crate) fn verify_client_binding_sources_contract_keys(
    binding: &serde_json::Value,
    context: &str,
) -> anyhow::Result<()> {
    let Some(sources) = binding.get("sources") else {
        return Ok(());
    };
    let sources = sources
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("{context}.sources must be an array"))?;
    for (index, source) in sources.iter().enumerate() {
        verify_json_object_keys_exact(
            source,
            &["source", "state_key"],
            &format!("{context}.sources[{index}]"),
        )?;
    }
    Ok(())
}

pub(crate) fn verify_client_template_contract_keys(
    binding: &serde_json::Value,
    field: &str,
    context: &str,
) -> anyhow::Result<()> {
    let Some(template) = binding.get(field) else {
        return Ok(());
    };
    let template = template
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("{context}.{field} must be an array"))?;
    for (index, segment) in template.iter().enumerate() {
        let segment_context = format!("{context}.{field}[{index}]");
        let kind = segment
            .get("kind")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| anyhow::anyhow!("{segment_context} kind must be a string"))?;
        match kind {
            "text" => verify_json_object_keys_exact(segment, &["kind", "value"], &segment_context)?,
            "signal" => {
                verify_json_object_keys_exact(segment, &["kind", "state_key"], &segment_context)?;
            }
            _ => anyhow::bail!("{segment_context} kind must be text or signal"),
        }
    }
    Ok(())
}

pub(crate) fn verify_client_condition_contract_keys(
    binding: &serde_json::Value,
    field: &str,
    context: &str,
) -> anyhow::Result<()> {
    let Some(condition) = binding.get(field) else {
        return Ok(());
    };
    verify_json_object_keys_allowing_optional(
        condition,
        &["state_key", "truthy", "falsy"],
        &["op", "rhs"],
        &format!("{context}.{field}"),
    )?;
    if let Some(rhs) = condition.get("rhs") {
        verify_client_value_contract_keys(rhs, &format!("{context}.{field}.rhs"))?;
    }
    Ok(())
}

pub(crate) fn verify_client_event_action_contract_keys(
    action: &serde_json::Value,
    context: &str,
) -> anyhow::Result<()> {
    let kind = action
        .get("kind")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("{context} kind must be a string"))?;
    match kind {
        "assign_toggle"
        | "assign_event_target_value"
        | "assign_event_target_checked"
        | "assign_event_target_value_float"
        | "assign_event_target_value_int" => {
            verify_json_object_keys_exact(action, &["kind"], context)
        }
        "assign" | "assign_add" | "assign_sub" => {
            verify_json_object_keys_exact(action, &["kind", "value"], context)?;
            verify_client_value_contract_keys(
                action
                    .get("value")
                    .ok_or_else(|| anyhow::anyhow!("{context}.value is missing"))?,
                &format!("{context}.value"),
            )
        }
        _ => anyhow::bail!("{context} kind must be a supported client action kind"),
    }
}

pub(crate) fn verify_client_value_contract_keys(
    value: &serde_json::Value,
    context: &str,
) -> anyhow::Result<()> {
    let kind = value
        .get("kind")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("{context} kind must be a string"))?;
    match kind {
        "int" | "float" | "string" | "bool" | "void" => {
            verify_json_object_keys_exact(value, &["kind", "value"], context)
        }
        "dynamic" => {
            verify_json_object_keys_exact(value, &["kind", "span"], context)?;
            verify_client_span_contract_keys(
                value
                    .get("span")
                    .ok_or_else(|| anyhow::anyhow!("{context}.span is missing"))?,
                &format!("{context}.span"),
            )
        }
        _ => anyhow::bail!("{context} kind must be a supported client value kind"),
    }
}

pub(crate) fn verify_client_span_contract_keys(
    span: &serde_json::Value,
    context: &str,
) -> anyhow::Result<()> {
    verify_json_object_keys_exact(span, &["file", "start", "end"], context)?;
    for key in ["file", "start", "end"] {
        if span.get(key).and_then(serde_json::Value::as_u64).is_none() {
            anyhow::bail!("{context}.{key} must be an unsigned integer");
        }
    }
    Ok(())
}

pub(crate) fn verify_client_blockers_contract_keys(
    value: &serde_json::Value,
    context: &str,
) -> anyhow::Result<()> {
    let blockers = value
        .get("blockers")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| anyhow::anyhow!("{context} blockers must be an array"))?;
    for (index, blocker) in blockers.iter().enumerate() {
        verify_json_object_keys_exact(
            blocker,
            &["id", "artifact", "reason"],
            &format!("{context} blockers[{index}]"),
        )?;
    }
    Ok(())
}

pub(crate) fn verify_client_blocker_details(
    value: &serde_json::Value,
    context: &str,
) -> anyhow::Result<()> {
    let blocked_by = value
        .get("blocked_by")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| anyhow::anyhow!("{context} blocked_by must be an array"))?;
    let blockers = value
        .get("blockers")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| anyhow::anyhow!("{context} blockers must be an array"))?;
    for blocked in blocked_by {
        let Some(id) = blocked.as_str() else {
            anyhow::bail!("{context} blocked_by entries must be strings");
        };
        if !blockers.iter().any(|blocker| {
            blocker.get("id").and_then(serde_json::Value::as_str) == Some(id)
                && blocker
                    .get("artifact")
                    .and_then(serde_json::Value::as_str)
                    .is_some_and(|artifact| !artifact.is_empty())
                && blocker
                    .get("reason")
                    .and_then(serde_json::Value::as_str)
                    .is_some_and(|reason| !reason.is_empty())
        }) {
            anyhow::bail!("{context} blockers must describe blocked_by entry {id}");
        }
    }
    Ok(())
}

pub(crate) fn verify_client_page_file(target: &Path) -> anyhow::Result<()> {
    let html = std::fs::read_to_string(target)
        .map_err(|e| anyhow::anyhow!("failed to read {}: {e}", target.display()))?;
    let trimmed = html.trim_start();
    if trimmed.is_empty() {
        anyhow::bail!("client_page bundle is empty: {}", target.display());
    }
    if !(trimmed.starts_with("<html") || trimmed.starts_with("<!doctype")) {
        anyhow::bail!("client_page bundle is not html: {}", target.display());
    }
    if !html.contains("data-orv-client=\"wasm\"") {
        anyhow::bail!("client_page bundle does not declare wasm bootstrap");
    }
    if !html.contains("type=\"module\"") || !html.contains("client/app.js") {
        anyhow::bail!("client_page bundle does not load client/app.js");
    }
    Ok(())
}

pub(crate) fn verify_client_reactive_plan_initial_render_binding(
    dir: &Path,
    bindings: &[serde_json::Value],
) -> anyhow::Result<()> {
    let binding = bindings
        .iter()
        .find(|binding| {
            binding.get("kind").and_then(serde_json::Value::as_str) == Some("initial_render")
                && binding.get("target").and_then(serde_json::Value::as_str)
                    == Some(CLIENT_PAGE_PATH)
                && binding.get("source").and_then(serde_json::Value::as_str)
                    == Some(CLIENT_WASM_PATH)
        })
        .ok_or_else(|| {
            anyhow::anyhow!(
                "client_reactive_plan initial_render binding must target {CLIENT_PAGE_PATH} from {CLIENT_WASM_PATH}"
            )
        })?;
    let manifest = read_json_value(&dir.join(CLIENT_MANIFEST_PATH))?;
    let initial_render = manifest
        .get("initial_render")
        .ok_or_else(|| anyhow::anyhow!("client_manifest initial_render must be an object"))?;
    for field in ["html_hash", "byte_length"] {
        if binding.get(field) != initial_render.get(field) {
            anyhow::bail!(
                "client_reactive_plan initial_render binding does not match client manifest"
            );
        }
    }
    Ok(())
}

pub(crate) fn verify_client_js_target(dir: &Path, target: &Path) -> anyhow::Result<()> {
    let source = std::fs::read_to_string(target)
        .map_err(|e| anyhow::anyhow!("failed to read {}: {e}", target.display()))?;
    if !source.contains("ORV_CLIENT_BOOTSTRAP") {
        anyhow::bail!("client_js bundle does not declare ORV bootstrap metadata");
    }
    if !source.contains("sourceBundleUrl") || !source.contains("../source-bundle.json") {
        anyhow::bail!("client_js bundle does not reference source bundle metadata");
    }
    if !source.contains("sourceBundleHash") {
        anyhow::bail!("client_js bundle does not declare source bundle hash metadata");
    }
    if !source.contains("sourceFileCount") {
        anyhow::bail!("client_js bundle does not declare source bundle file count metadata");
    }
    if !source.contains("manifestUrl")
        || !source.contains("./manifest.json")
        || !source.contains("loadClientManifest")
        || !source.contains("client manifest fetch failed")
        || !source.contains("client manifest hash mismatch")
        || !source.contains("client manifest export mismatch")
        || !source.contains("validateWasmBundle")
        || !source.contains("client wasm hash mismatch")
    {
        anyhow::bail!("client_js bundle does not verify client manifest contract");
    }
    if !source.contains("reactivePlanUrl")
        || !source.contains("./reactive-plan.json")
        || !source.contains("loadReactivePlan")
        || !source.contains("embeddedReactivePlan")
        || !source.contains("embeddedReactivePlanHash")
        || !source.contains("loadEmbeddedReactivePlan")
        || !source.contains("validateReactivePlan")
        || !source.contains("client embedded reactive plan hash mismatch")
        || !source.contains("validateReactiveBindings")
        || !source.contains("client reactive plan fetch failed")
        || !source.contains("client reactive plan hash mismatch")
        || !source.contains("client reactive plan initial_render binding mismatch")
        || !source.contains("client reactive plan signal_state binding mismatch")
        || !source.contains("client reactive plan signal_text binding mismatch")
        || !source.contains("client reactive plan signal_attr binding mismatch")
        || !source.contains("client reactive plan signal_event binding mismatch")
        || !source.contains("renderSignalTextBinding")
        || !source.contains("text_template")
        || !source.contains("renderSignalTextCondition")
        || !source.contains("text_condition")
        || !source.contains("signalTextBindingStateKeys")
        || !source.contains("signalTextBindingCursorKey")
        || !source.contains("state_keys")
        || !source.contains("renderSignalAttrBinding")
        || !source.contains("attr_template")
        || !source.contains("signalAttrBindingStateKeys")
        || !source.contains("signalAttrBindingCursorKey")
        || !source.contains("renderSignalAttrCondition")
        || !source.contains("attr_condition")
        || !source.contains("compareSignalAttrCondition")
        || !source.contains("decodeSignalConditionOperand")
        || !source.contains("createReactiveState")
        || !source.contains("bindReactiveDom")
        || !source.contains("bindReactiveAttrs")
        || !source.contains("bindReactiveEvents")
        || !source.contains("applySignalAction")
        || !source.contains("assign_add")
        || !source.contains("assign_sub")
        || !source.contains("assign_toggle")
        || !source.contains("assign_event_target_value")
        || !source.contains("assign_event_target_checked")
        || !source.contains("assign_event_target_value_float")
        || !source.contains("assign_event_target_value_int")
        || !source.contains("setSignal")
        || !source.contains("orvReactiveSignals")
        || !source.contains("orvReactiveBindings")
        || !source.contains("orvReactiveDomBindings")
        || !source.contains("orvReactiveAttrBindings")
        || !source.contains("orvReactiveEventBindings")
        || !source.contains("orvReactiveStateHash")
        || !source.contains("__ORV_CLIENT_REACTIVE_STATE__")
        || !source.contains("__ORV_SET_SIGNAL__")
    {
        anyhow::bail!("client_js bundle does not verify client reactive plan contract");
    }
    if !source.contains("loadSourceBundle")
        || !source.contains("sourceFileCount")
        || !source.contains("fnv1a64")
        || !source.contains("source bundle hash mismatch")
    {
        anyhow::bail!("client_js bundle does not verify source bundle hash");
    }
    if !source.contains("app.wasm") {
        anyhow::bail!("client_js bundle does not reference app.wasm");
    }
    if !source.contains("WebAssembly.instantiate") {
        anyhow::bail!("client_js bundle does not instantiate wasm");
    }
    if !source.contains("readInitialRender")
        || !source.contains("orv_render_ptr")
        || !source.contains("orv_render_len")
        || !source.contains("TextDecoder")
        || !source.contains("#orv-root")
        || !source.contains("initialRenderMountHtml")
        || !source.contains("DOMParser")
        || !source.contains("root.innerHTML")
    {
        anyhow::bail!("client_js bundle does not decode initial render from wasm");
    }
    if !source.contains("instance.exports.orv_start()") {
        anyhow::bail!("client_js bundle does not call {CLIENT_WASM_START_EXPORT}");
    }
    if !source.contains("validateInitialRender")
        || !source.contains("initial_render")
        || !source.contains("html_hash")
        || !source.contains("client initial render hash mismatch")
        || !source.contains("client initial render byte length mismatch")
    {
        anyhow::bail!("client_js bundle does not verify initial render contract");
    }
    let source_bundle_value = read_json_value(&dir.join(SOURCE_BUNDLE_PATH))?;
    let source_bundle_hash = stable_json_hash(&source_bundle_value)?;
    let source_bundle = read_source_bundle_artifact(&dir.join(SOURCE_BUNDLE_PATH))?;
    let reactive_plan = read_json_value(&dir.join(CLIENT_REACTIVE_PLAN_PATH))?;
    let expected = client_js_loader_script(&source_bundle, &source_bundle_hash, &reactive_plan)?;
    if source != expected {
        anyhow::bail!("client_js bundle must match generated loader");
    }
    Ok(())
}

pub(crate) fn verify_client_wasm_target(dir: &Path, target: &Path) -> anyhow::Result<()> {
    let bytes = std::fs::read(target)
        .map_err(|e| anyhow::anyhow!("failed to read {}: {e}", target.display()))?;
    verify_client_wasm_bytes(dir, target, &bytes)
}

pub(crate) fn verify_client_wasm_bytes(
    dir: &Path,
    target: &Path,
    bytes: &[u8],
) -> anyhow::Result<()> {
    if bytes.len() < WASM_MODULE_HEADER.len() {
        anyhow::bail!("client_wasm bundle is too small: {}", target.display());
    }
    if &bytes[..4] != b"\0asm" {
        anyhow::bail!("client_wasm bundle has invalid magic: {}", target.display());
    }
    if &bytes[4..8] != b"\x01\0\0\0" {
        anyhow::bail!(
            "client_wasm bundle has unsupported version: {}",
            target.display()
        );
    }
    let metadata = client_wasm_metadata_value_from_bytes(bytes)?;
    if metadata
        .get("schema_version")
        .and_then(serde_json::Value::as_u64)
        != Some(1)
    {
        anyhow::bail!("client_wasm ORV metadata schema_version must be 1");
    }
    if metadata
        .get("source_bundle")
        .and_then(serde_json::Value::as_str)
        != Some(CLIENT_WASM_SOURCE_BUNDLE_PATH)
    {
        anyhow::bail!("client_wasm ORV metadata source_bundle is invalid");
    }
    let source_bundle = read_json_value(&dir.join("source-bundle.json"))?;
    let expected_source_bundle_hash = stable_json_hash(&source_bundle)?;
    if metadata
        .get("source_bundle_hash")
        .and_then(serde_json::Value::as_str)
        != Some(expected_source_bundle_hash.as_str())
    {
        anyhow::bail!("client_wasm ORV metadata source_bundle_hash is invalid");
    }
    if metadata.get("entry") != source_bundle.get("entry") {
        anyhow::bail!("client_wasm ORV metadata entry is invalid");
    }
    if !metadata
        .get("runtime_features")
        .and_then(serde_json::Value::as_array)
        .is_some_and(|features| features.iter().any(|feature| feature == "client_wasm"))
    {
        anyhow::bail!("client_wasm ORV metadata must include client_wasm runtime feature");
    }
    let initial_render = metadata
        .get("initial_render")
        .ok_or_else(|| anyhow::anyhow!("client_wasm ORV metadata missing initial_render"))?;
    if initial_render
        .get("content_type")
        .and_then(serde_json::Value::as_str)
        != Some("text/html")
    {
        anyhow::bail!("client_wasm initial_render content_type is invalid");
    }
    if initial_render
        .get("encoding")
        .and_then(serde_json::Value::as_str)
        != Some("utf-8")
    {
        anyhow::bail!("client_wasm initial_render encoding is invalid");
    }
    if initial_render
        .get("html_hash")
        .and_then(serde_json::Value::as_str)
        .is_none_or(str::is_empty)
    {
        anyhow::bail!("client_wasm initial_render html_hash is required");
    }
    if initial_render
        .get("byte_length")
        .and_then(serde_json::Value::as_u64)
        .is_none()
    {
        anyhow::bail!("client_wasm initial_render byte_length is required");
    }
    if initial_render
        .get("ptr_export")
        .and_then(serde_json::Value::as_str)
        != Some(CLIENT_WASM_RENDER_PTR_EXPORT)
        || initial_render
            .get("len_export")
            .and_then(serde_json::Value::as_str)
            != Some(CLIENT_WASM_RENDER_LEN_EXPORT)
        || initial_render
            .get("memory_export")
            .and_then(serde_json::Value::as_str)
            != Some(CLIENT_WASM_MEMORY_EXPORT)
    {
        anyhow::bail!("client_wasm initial_render export metadata is invalid");
    }
    if client_wasm_export_index(bytes, CLIENT_WASM_START_EXPORT, 0)? != Some(0) {
        anyhow::bail!("client_wasm bundle must export `{CLIENT_WASM_START_EXPORT}` function 0");
    }
    if !client_wasm_exports_function(bytes, CLIENT_WASM_RENDER_PTR_EXPORT)?
        || !client_wasm_exports_function(bytes, CLIENT_WASM_RENDER_LEN_EXPORT)?
    {
        anyhow::bail!("client_wasm bundle must export initial render pointer and length");
    }
    if client_wasm_export_index(bytes, CLIENT_WASM_MEMORY_EXPORT, 2)? != Some(0) {
        anyhow::bail!("client_wasm bundle must export initial render memory 0");
    }
    verify_client_wasm_initial_render_data(bytes, initial_render)?;
    Ok(())
}

pub(crate) fn verify_deploy_smoke_client_contract(
    dir: &Path,
    smoke: &str,
    client: Option<&serde_json::Value>,
) -> anyhow::Result<()> {
    let Some(client) = client.filter(|value| !value.is_null()) else {
        return Ok(());
    };
    if !smoke.contains("orv_smoke_file()") || !smoke.contains("orv_smoke_grep()") {
        anyhow::bail!("deploy smoke test must include client file contract helpers");
    }
    if !smoke.contains(r#"ORV_SMOKE_CLIENT_ORIGIN="ori_"#) {
        anyhow::bail!("deploy smoke test must declare a client reveal origin");
    }
    for key in ["manifest", "reactive_plan", "page", "loader", "wasm"] {
        let path = json_str(client, key, "deploy client")?;
        let command = format!(r#"orv_smoke_file "{path}""#);
        if !smoke.contains(&command) {
            anyhow::bail!("deploy smoke test must check client {key} {path}");
        }
    }
    let reactive_plan = json_str(client, "reactive_plan", "deploy client")?;
    let page = json_str(client, "page", "deploy client")?;
    let loader = json_str(client, "loader", "deploy client")?;
    let manifest = json_str(client, "manifest", "deploy client")?;
    let client_summary = deploy_client_summary_counts(dir)?;
    for required in [
        format!(r#"orv_smoke_grep "client page marker" "{page}" 'data-orv-client="wasm"'"#),
        format!(r#"orv_smoke_grep "client loader reference" "{page}" 'app.js'"#),
        format!(
            r#"orv_smoke_grep "client manifest reactive plan path" "{manifest}" '"reactive_plan": "{reactive_plan}"'"#
        ),
        format!("client_manifest={manifest}"),
        format!("client_reactive_plan={reactive_plan}"),
        format!("client_page={page}"),
        format!("client_loader={loader}"),
        format!("client_wasm={}", json_str(client, "wasm", "deploy client")?),
        format!(
            r#"orv_smoke_grep "client manifest reactive plan hash" "{manifest}" '"reactive_plan_hash"'"#
        ),
        format!(r#"orv_smoke_grep "client manifest loader hash" "{manifest}" '"loader_hash"'"#),
        format!(r#"orv_smoke_grep "client manifest wasm hash" "{manifest}" '"wasm_hash"'"#),
        format!(
            r#"orv_smoke_grep "client manifest source bundle" "{manifest}" '"source_bundle": "source-bundle.json"'"#
        ),
        format!(
            r#"orv_smoke_grep "client manifest runtime" "{manifest}" '"runtime": "client_wasm"'"#
        ),
        format!(r#"orv_smoke_grep "client manifest capabilities" "{manifest}" '"capabilities"'"#),
        format!(
            r#"orv_smoke_grep "client manifest capability surfaces" "{manifest}" '"surfaces"'"#
        ),
        format!(r#"orv_smoke_grep "client manifest event actions" "{manifest}" '"event_actions"'"#),
        format!(
            r#"orv_smoke_grep "client reactive plan kind" "{reactive_plan}" '"kind": "orv.client.reactive_plan"'"#
        ),
        format!(
            r#"orv_smoke_grep "client reactive plan source bundle" "{reactive_plan}" '"source_bundle": "source-bundle.json"'"#
        ),
        format!(
            r#"orv_smoke_grep "client reactive plan blocked_by" "{reactive_plan}" '"blocked_by"'"#
        ),
        format!(r#"orv_smoke_grep "client loader bootstrap" "{loader}" 'ORV_CLIENT_BOOTSTRAP'"#),
        format!(
            r#"orv_smoke_grep "client loader embedded reactive plan" "{loader}" 'embeddedReactivePlan'"#
        ),
        format!(
            r#"orv_smoke_grep "client loader embedded reactive plan hash" "{loader}" 'embeddedReactivePlanHash'"#
        ),
        format!(
            r#"orv_smoke_grep "client loader source bundle hash" "{loader}" 'sourceBundleHash'"#
        ),
        format!(r#"orv_smoke_grep "client loader wasm reference" "{loader}" 'app.wasm'"#),
        format!(r#"orv_smoke_grep "client loader signal setter" "{loader}" '__ORV_SET_SIGNAL__'"#),
        format!(
            r#"orv_smoke_reveal_contains "reveal client target summary" "$ORV_SMOKE_CLIENT_ORIGIN" '"client_target_count": {}'"#,
            client_summary.targets
        ),
        format!(
            r#"orv_smoke_reveal_contains "reveal client manifest summary" "$ORV_SMOKE_CLIENT_ORIGIN" '"client_manifest_count": {}'"#,
            client_summary.manifests
        ),
        format!(
            r#"orv_smoke_reveal_contains "reveal client capability summary" "$ORV_SMOKE_CLIENT_ORIGIN" '"client_capability_surface_count": {}'"#,
            client_summary.capability_surfaces
        ),
        format!(
            r#"orv_smoke_reveal_contains "reveal client manifest target" "$ORV_SMOKE_CLIENT_ORIGIN" '"path": "{manifest}"'"#
        ),
        format!(
            r#"orv_smoke_editor_reveal_contains "editor reveal client target summary" "$ORV_SMOKE_CLIENT_ORIGIN" '"client_target_count": {}'"#,
            client_summary.targets
        ),
        format!(
            r#"orv_smoke_editor_reveal_contains "editor reveal client manifest summary" "$ORV_SMOKE_CLIENT_ORIGIN" '"client_manifest_count": {}'"#,
            client_summary.manifests
        ),
        format!(
            r#"orv_smoke_editor_reveal_contains "editor reveal client capability summary" "$ORV_SMOKE_CLIENT_ORIGIN" '"client_capability_surface_count": {}'"#,
            client_summary.capability_surfaces
        ),
        format!(
            r#"orv_smoke_lsp_reveal_contains "lsp reveal client target summary" "$ORV_SMOKE_CLIENT_ORIGIN" '"client_target_count": {}'"#,
            client_summary.targets
        ),
        format!(
            r#"orv_smoke_lsp_reveal_contains "lsp reveal client manifest summary" "$ORV_SMOKE_CLIENT_ORIGIN" '"client_manifest_count": {}'"#,
            client_summary.manifests
        ),
        format!(
            r#"orv_smoke_lsp_reveal_contains "lsp reveal client capability summary" "$ORV_SMOKE_CLIENT_ORIGIN" '"client_capability_surface_count": {}'"#,
            client_summary.capability_surfaces
        ),
        format!(
            r#"orv_smoke_dap_summary_contains "dap client target summary" '"client_target_count": {}'"#,
            client_summary.targets
        ),
        format!(
            r#"orv_smoke_dap_summary_contains "dap client manifest summary" '"client_manifest_count": {}'"#,
            client_summary.manifests
        ),
        format!(
            r#"orv_smoke_dap_summary_contains "dap client capability summary" '"client_capability_surface_count": {}'"#,
            client_summary.capability_surfaces
        ),
    ] {
        if !smoke.contains(&required) {
            anyhow::bail!("deploy smoke test must include {required}");
        }
    }
    Ok(())
}

pub(crate) fn verify_deploy_runbook_client_section(
    runbook: &str,
    client: Option<&serde_json::Value>,
) -> anyhow::Result<()> {
    let Some(client) = client.filter(|value| !value.is_null()) else {
        return Ok(());
    };
    for key in ["manifest", "reactive_plan", "page", "loader", "wasm"] {
        let path = json_str(client, key, "deploy client")?;
        if !runbook.contains(path) {
            anyhow::bail!("deploy runbook must document client {key} {path}");
        }
    }
    let runtime = client
        .pointer("/capabilities/runtime")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("client_wasm");
    if !runbook.contains(runtime) {
        anyhow::bail!("deploy runbook must document client runtime {runtime}");
    }
    for surface in client
        .pointer("/capabilities/surfaces")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(serde_json::Value::as_str)
    {
        if !runbook.contains(surface) {
            anyhow::bail!("deploy runbook must document client capability surface {surface}");
        }
    }
    for blocker in client
        .get("blockers")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
    {
        for key in ["id", "artifact"] {
            let value = json_str(blocker, key, "deploy client blocker")?;
            if !runbook.contains(value) {
                anyhow::bail!("deploy runbook must document client blocker {value}");
            }
        }
    }
    Ok(())
}

pub(crate) fn verify_deploy_client_target(
    dir: &Path,
    client: Option<&serde_json::Value>,
) -> anyhow::Result<()> {
    let Some(client) = client.filter(|value| !value.is_null()) else {
        return Ok(());
    };
    verify_json_object_keys_exact(
        client,
        &[
            "manifest",
            "reactive_plan",
            "page",
            "loader",
            "wasm",
            "runtime_features",
            "capabilities",
            "blocked_by",
            "blockers",
        ],
        "deploy client",
    )?;
    let expected_runtime_features = serde_json::json!(["client_wasm"]);
    if client.get("runtime_features") != Some(&expected_runtime_features) {
        anyhow::bail!("deploy client runtime_features must be [\"client_wasm\"]");
    }
    let manifest = json_str(client, "manifest", "deploy client")?;
    let manifest_target = dir.join(manifest);
    if !manifest_target.is_file() {
        anyhow::bail!(
            "missing deploy client manifest: {}",
            manifest_target.display()
        );
    }
    let manifest_value = read_json_value(&manifest_target)?;
    verify_client_manifest_value(dir, &manifest_value)?;
    let reactive_plan = json_str(client, "reactive_plan", "deploy client")?;
    if manifest_value
        .get("reactive_plan")
        .and_then(serde_json::Value::as_str)
        != Some(reactive_plan)
    {
        anyhow::bail!("deploy client reactive_plan does not match client manifest");
    }
    let reactive_plan_target = dir.join(reactive_plan);
    if !reactive_plan_target.is_file() {
        anyhow::bail!(
            "missing deploy client reactive plan: {}",
            reactive_plan_target.display()
        );
    }
    let reactive_plan_value = read_json_value(&reactive_plan_target)?;
    verify_client_reactive_plan_value(dir, &reactive_plan_value)?;
    if client.get("capabilities") != manifest_value.get("capabilities") {
        anyhow::bail!("deploy client capabilities do not match client manifest");
    }
    if client.get("blocked_by") != manifest_value.get("blocked_by") {
        anyhow::bail!("deploy client blocked_by does not match client manifest");
    }
    if client.get("blockers") != manifest_value.get("blockers") {
        anyhow::bail!("deploy client blockers do not match client manifest");
    }
    let page = json_str(client, "page", "deploy client")?;
    let page_target = dir.join(page);
    if !page_target.is_file() {
        anyhow::bail!("missing deploy client page: {}", page_target.display());
    }
    verify_client_page_file(&page_target)?;
    let loader = json_str(client, "loader", "deploy client")?;
    let loader_target = dir.join(loader);
    if !loader_target.is_file() {
        anyhow::bail!("missing deploy client loader: {}", loader_target.display());
    }
    verify_client_js_target(dir, &loader_target)?;
    let wasm = json_str(client, "wasm", "deploy client")?;
    let wasm_target = dir.join(wasm);
    if !wasm_target.is_file() {
        anyhow::bail!("missing deploy client wasm: {}", wasm_target.display());
    }
    verify_client_wasm_target(dir, &wasm_target)
}
