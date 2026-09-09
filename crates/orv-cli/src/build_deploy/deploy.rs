use super::*;

pub(crate) fn cmd_deploy_env_check(dir: &Path) -> anyhow::Result<()> {
    deploy_env_check_with_lookup(dir, |env| std::env::var(env).ok())?;
    println!("deploy env: {} verified", dir.display());
    Ok(())
}

pub(crate) fn deploy_project_graph_node_count(dir: &Path) -> anyhow::Result<usize> {
    let graph = read_json_value(&dir.join("project-graph.json"))?;
    Ok(json_array_count(graph.get("nodes")))
}

pub(crate) fn deploy_graph_contract_count(dir: &Path) -> anyhow::Result<usize> {
    Ok(editor_production_graph_contract_targets(dir)?.len())
}

pub(crate) fn deploy_env_check_with_lookup<F>(dir: &Path, mut lookup: F) -> anyhow::Result<()>
where
    F: FnMut(&str) -> Option<String>,
{
    let preflight_path = dir.join(DEPLOY_PREFLIGHT_PATH);
    if !preflight_path.is_file() {
        anyhow::bail!(
            "missing deploy preflight artifact: {}",
            preflight_path.display()
        );
    }
    let preflight = read_json_value(&preflight_path)?;
    if preflight
        .get("schema_version")
        .and_then(serde_json::Value::as_u64)
        != Some(1)
    {
        anyhow::bail!("deploy preflight schema_version must be 1");
    }
    if json_str(&preflight, "kind", "deploy preflight")? != "orv.deploy.preflight" {
        anyhow::bail!("deploy preflight kind must be orv.deploy.preflight");
    }

    let db_adapters_path = dir.join("deploy").join("db-adapters.json");
    if !db_adapters_path.is_file() {
        anyhow::bail!(
            "missing deploy DB adapters artifact: {}",
            db_adapters_path.display()
        );
    }
    let db_adapters = read_json_value(&db_adapters_path)?;
    verify_deploy_db_adapter_contract_keys(&db_adapters)?;
    if db_adapters
        .get("schema_version")
        .and_then(serde_json::Value::as_u64)
        != Some(1)
    {
        anyhow::bail!("deploy DB adapters schema_version must be 1");
    }
    if json_str(&db_adapters, "kind", "deploy DB adapters")? != "orv.deploy.db_adapters" {
        anyhow::bail!("deploy DB adapters kind must be orv.deploy.db_adapters");
    }

    let commerce_adapters_path = dir.join("deploy").join("commerce-adapters.json");
    if !commerce_adapters_path.is_file() {
        anyhow::bail!(
            "missing deploy commerce adapters artifact: {}",
            commerce_adapters_path.display()
        );
    }
    let commerce_adapters = read_json_value(&commerce_adapters_path)?;
    verify_deploy_commerce_adapter_contract_keys(&commerce_adapters)?;
    if commerce_adapters
        .get("schema_version")
        .and_then(serde_json::Value::as_u64)
        != Some(1)
    {
        anyhow::bail!("deploy commerce adapters schema_version must be 1");
    }
    if json_str(&commerce_adapters, "kind", "deploy commerce adapters")?
        != "orv.deploy.commerce_adapters"
    {
        anyhow::bail!("deploy commerce adapters kind must be orv.deploy.commerce_adapters");
    }
    let (missing, optional_missing) = deploy_env_check_preflight_missing(&preflight, &mut lookup)?;
    if !missing.is_empty() {
        anyhow::bail!(
            "missing required deploy env: {}; optional missing: {}",
            missing.join(", "),
            optional_missing.join(", ")
        );
    }
    Ok(())
}

pub(crate) fn deploy_env_check_preflight_missing<F>(
    preflight: &serde_json::Value,
    lookup: &mut F,
) -> anyhow::Result<(Vec<String>, Vec<String>)>
where
    F: FnMut(&str) -> Option<String>,
{
    let mut missing = Vec::new();
    let mut optional_missing = Vec::new();
    let required_env = preflight
        .get("required_env")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| anyhow::anyhow!("deploy preflight required_env must be an array"))?;
    for env in required_env {
        let variable = json_str(env, "env", "deploy preflight env")?;
        if lookup(variable)
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
            || deploy_preflight_env_has_db_bridge_fallback(env, &mut *lookup)
        {
            continue;
        }
        missing.push(deploy_preflight_env_label(env)?);
    }
    let optional_env = preflight
        .get("optional_env")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| anyhow::anyhow!("deploy preflight optional_env must be an array"))?;
    for env in optional_env {
        let variable = json_str(env, "env", "deploy preflight env")?;
        if lookup(variable)
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
            || deploy_preflight_optional_env_has_db_bridge_fallback(env, &mut *lookup)
        {
            continue;
        }
        optional_missing.push(deploy_preflight_env_label(env)?);
    }
    Ok((missing, optional_missing))
}

pub(crate) fn deploy_preflight_env_has_db_bridge_fallback<F>(
    env: &serde_json::Value,
    lookup: &mut F,
) -> bool
where
    F: FnMut(&str) -> Option<String>,
{
    let kind = env.get("kind").and_then(serde_json::Value::as_str);
    let purpose = env.get("purpose").and_then(serde_json::Value::as_str);
    let variable = env.get("env").and_then(serde_json::Value::as_str);
    if kind == Some("db")
        && purpose == Some("bridge_endpoint")
        && variable != Some("ORV_DB_ADAPTER_ENDPOINT")
    {
        return lookup("ORV_DB_ADAPTER_ENDPOINT")
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty());
    }
    false
}

pub(crate) fn deploy_preflight_optional_env_has_db_bridge_fallback<F>(
    env: &serde_json::Value,
    lookup: &mut F,
) -> bool
where
    F: FnMut(&str) -> Option<String>,
{
    let kind = env.get("kind").and_then(serde_json::Value::as_str);
    let purpose = env.get("purpose").and_then(serde_json::Value::as_str);
    let variable = env.get("env").and_then(serde_json::Value::as_str);
    if kind == Some("db")
        && purpose == Some("bridge_auth_token")
        && variable != Some("ORV_DB_ADAPTER_AUTH_TOKEN")
    {
        return lookup("ORV_DB_ADAPTER_AUTH_TOKEN")
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty());
    }
    false
}

pub(crate) fn deploy_preflight_env_label(env: &serde_json::Value) -> anyhow::Result<String> {
    let kind = json_str(env, "kind", "deploy preflight env")?;
    let variable = json_str(env, "env", "deploy preflight env")?;
    if let Some(provider) = env.get("provider").and_then(serde_json::Value::as_str) {
        return Ok(format!("{kind} {provider} {variable}"));
    }
    Ok(format!("{kind} {variable}"))
}

pub(crate) struct DeployRunbookArtifacts<'a> {
    pub(crate) server_artifact: &'a str,
    pub(crate) compose: &'a str,
    pub(crate) env_example: &'a str,
    pub(crate) db_adapters: &'a str,
    pub(crate) commerce_adapters: &'a str,
    pub(crate) smoke_test: &'a str,
    pub(crate) smoke_output: &'a str,
    pub(crate) preflight: &'a str,
    pub(crate) benchmark_evidence: &'a str,
    pub(crate) participant_notes_template: &'a str,
    pub(crate) runbook: &'a str,
    pub(crate) routes: &'a str,
}

pub(crate) struct DeployServerContract<'a> {
    pub(crate) artifact_path: &'a str,
    pub(crate) entrypoint: &'a str,
    pub(crate) routes_artifact: &'a str,
    pub(crate) runtime: &'a str,
    pub(crate) runtime_image: &'a str,
    pub(crate) listen: Option<&'a orv_compiler::ServerListenArtifact>,
}

pub(crate) fn adapter_source_origin_ids(adapter: &serde_json::Value) -> Vec<String> {
    let mut ids = adapter
        .get("source_origin_ids")
        .and_then(serde_json::Value::as_array)
        .map(|ids| {
            ids.iter()
                .filter_map(serde_json::Value::as_str)
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if let Some(id) = adapter
        .get("source_origin_id")
        .and_then(serde_json::Value::as_str)
    {
        ids.push(id.to_string());
    }
    normalize_source_origin_ids(&mut ids);
    ids
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct DeployPersistence {
    pub(crate) wal_paths: Vec<String>,
    pub(crate) db_paths: Vec<String>,
    pub(crate) db_endpoints: Vec<String>,
    pub(crate) db_env: Vec<DeployAdapterEnv>,
    pub(crate) db_adapters: Vec<DeployDbAdapter>,
    pub(crate) record_paths: Vec<String>,
    pub(crate) commerce_endpoints: Vec<String>,
    pub(crate) commerce_env: Vec<DeployAdapterEnv>,
    pub(crate) commerce_adapters: Vec<DeployCommerceAdapter>,
    pub(crate) volumes: Vec<DeployPersistenceVolume>,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub(crate) struct DeployAdapterEnv {
    pub(crate) env: String,
    pub(crate) default: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub(crate) struct DeployDbAdapter {
    pub(crate) mode: String,
    pub(crate) provider: String,
    pub(crate) env: Option<String>,
    pub(crate) default: Option<String>,
    pub(crate) endpoint: Option<String>,
    pub(crate) adapter_status: String,
    pub(crate) bridge_env: Vec<DeployProviderEnv>,
    pub(crate) source_origin_ids: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub(crate) struct DeployCommerceAdapter {
    pub(crate) kind: String,
    pub(crate) mode: String,
    pub(crate) provider: Option<String>,
    pub(crate) env: Option<String>,
    pub(crate) default: Option<String>,
    pub(crate) endpoint: Option<String>,
    pub(crate) record_path: Option<String>,
    pub(crate) provider_env: Vec<DeployProviderEnv>,
    pub(crate) source_origin_ids: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub(crate) struct DeployProviderEnv {
    pub(crate) env: String,
    pub(crate) required: bool,
    pub(crate) purpose: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub(crate) struct DeployPreflightEnv {
    pub(crate) kind: String,
    pub(crate) env: String,
    pub(crate) required: bool,
    pub(crate) purpose: String,
    pub(crate) default: Option<String>,
    pub(crate) provider: Option<String>,
}

#[derive(Default)]
pub(crate) struct DeployPersistenceAccumulator {
    pub(crate) wal_paths: Vec<String>,
    pub(crate) db_paths: Vec<String>,
    pub(crate) db_endpoints: Vec<String>,
    pub(crate) db_env: Vec<DeployAdapterEnv>,
    pub(crate) db_adapters: Vec<DeployDbAdapter>,
    pub(crate) record_paths: Vec<String>,
    pub(crate) commerce_endpoints: Vec<String>,
    pub(crate) commerce_env: Vec<DeployAdapterEnv>,
    pub(crate) commerce_adapters: Vec<DeployCommerceAdapter>,
}

impl DeployPersistenceAccumulator {
    fn into_persistence(mut self) -> DeployPersistence {
        self.wal_paths.sort();
        self.wal_paths.dedup();
        self.db_paths.sort();
        self.db_paths.dedup();
        self.db_endpoints.sort();
        self.db_endpoints.dedup();
        self.db_env.sort();
        self.db_env.dedup();
        self.db_adapters = merge_deploy_db_adapters(self.db_adapters);
        self.record_paths.sort();
        self.record_paths.dedup();
        self.commerce_endpoints.sort();
        self.commerce_endpoints.dedup();
        self.commerce_env.sort();
        self.commerce_env.dedup();
        self.commerce_adapters = merge_deploy_commerce_adapters(self.commerce_adapters);
        let mut persistent_paths = self.wal_paths.clone();
        persistent_paths.extend(self.db_paths.clone());
        persistent_paths.extend(self.record_paths.clone());
        persistent_paths.sort();
        persistent_paths.dedup();
        DeployPersistence {
            volumes: deploy_persistence_volumes(&persistent_paths),
            wal_paths: self.wal_paths,
            db_paths: self.db_paths,
            db_endpoints: self.db_endpoints,
            db_env: self.db_env,
            db_adapters: self.db_adapters,
            record_paths: self.record_paths,
            commerce_endpoints: self.commerce_endpoints,
            commerce_env: self.commerce_env,
            commerce_adapters: self.commerce_adapters,
        }
    }
}

pub(crate) fn merge_deploy_db_adapters(adapters: Vec<DeployDbAdapter>) -> Vec<DeployDbAdapter> {
    let mut merged = Vec::<DeployDbAdapter>::new();
    for mut adapter in adapters {
        normalize_source_origin_ids(&mut adapter.source_origin_ids);
        if let Some(existing) = merged
            .iter_mut()
            .find(|existing| same_db_adapter_contract(existing, &adapter))
        {
            existing.source_origin_ids.extend(adapter.source_origin_ids);
            normalize_source_origin_ids(&mut existing.source_origin_ids);
        } else {
            merged.push(adapter);
        }
    }
    merged.sort();
    merged
}

pub(crate) fn same_db_adapter_contract(a: &DeployDbAdapter, b: &DeployDbAdapter) -> bool {
    a.mode == b.mode
        && a.provider == b.provider
        && a.env == b.env
        && a.default == b.default
        && a.endpoint == b.endpoint
        && a.adapter_status == b.adapter_status
        && a.bridge_env == b.bridge_env
}

pub(crate) fn merge_deploy_commerce_adapters(
    adapters: Vec<DeployCommerceAdapter>,
) -> Vec<DeployCommerceAdapter> {
    let mut merged = Vec::<DeployCommerceAdapter>::new();
    for mut adapter in adapters {
        normalize_source_origin_ids(&mut adapter.source_origin_ids);
        if let Some(existing) = merged
            .iter_mut()
            .find(|existing| same_commerce_adapter_contract(existing, &adapter))
        {
            existing.source_origin_ids.extend(adapter.source_origin_ids);
            normalize_source_origin_ids(&mut existing.source_origin_ids);
        } else {
            merged.push(adapter);
        }
    }
    merged.sort();
    merged
}

pub(crate) fn same_commerce_adapter_contract(
    a: &DeployCommerceAdapter,
    b: &DeployCommerceAdapter,
) -> bool {
    a.kind == b.kind
        && a.mode == b.mode
        && a.provider == b.provider
        && a.env == b.env
        && a.default == b.default
        && a.endpoint == b.endpoint
        && a.record_path == b.record_path
        && a.provider_env == b.provider_env
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct DeployPersistenceVolume {
    pub(crate) host: String,
    pub(crate) container: String,
    pub(crate) compose_mount: String,
}

pub(crate) fn server_artifact_deploy_persistence(
    artifact: &orv_compiler::ServerRuntimeArtifact,
) -> anyhow::Result<DeployPersistence> {
    let entry_path = artifact
        .source_bundle
        .files
        .first()
        .ok_or_else(|| anyhow::anyhow!("server artifact source bundle is empty"))?
        .path
        .clone();
    let loaded = orv_project::load_project_from_sources(
        Path::new(&entry_path),
        artifact
            .source_bundle
            .files
            .iter()
            .map(|file| (PathBuf::from(&file.path), file.source.clone())),
    )
    .map_err(|e| anyhow::anyhow!("failed to rehydrate deploy persistence sources: {e}"))?;
    if !loaded.diagnostics.is_empty() {
        anyhow::bail!("deploy persistence source reanalysis produced diagnostics");
    }
    let resolved = orv_resolve::resolve(&loaded.program);
    if !resolved.diagnostics.is_empty() {
        anyhow::bail!("deploy persistence resolve reanalysis produced diagnostics");
    }
    let lowered = orv_analyzer::lower_with_diagnostics(&loaded.program, &resolved);
    if !lowered.diagnostics.is_empty() {
        anyhow::bail!("deploy persistence lowering reanalysis produced diagnostics");
    }
    let mut persistence = DeployPersistenceAccumulator::default();
    collect_program_persistence_paths(&lowered.program, &mut persistence);
    Ok(persistence.into_persistence())
}

pub(crate) fn deploy_persistence_value(persistence: &DeployPersistence) -> serde_json::Value {
    serde_json::json!({
        "wal_paths": persistence.wal_paths,
        "db_paths": persistence.db_paths,
        "db_endpoints": persistence.db_endpoints,
        "db_env": deploy_adapter_env_value(&persistence.db_env),
        "db_adapters": deploy_db_adapter_value(&persistence.db_adapters),
        "record_paths": persistence.record_paths,
        "commerce_endpoints": persistence.commerce_endpoints,
        "commerce_env": deploy_adapter_env_value(&persistence.commerce_env),
        "commerce_adapters": deploy_commerce_adapter_value(&persistence.commerce_adapters),
        "volumes": persistence.volumes.iter().map(|volume| {
            serde_json::json!({
                "host": volume.host,
                "container": volume.container,
                "compose_mount": volume.compose_mount,
            })
        }).collect::<Vec<_>>(),
    })
}

pub(crate) fn deploy_security_runtime_features(runtime_features: &[String]) -> Vec<String> {
    let mut features = runtime_features
        .iter()
        .filter(|feature| {
            matches!(
                feature.as_str(),
                "auth_roles" | "csrf_protection" | "rate_limit" | "session_cookies"
            )
        })
        .cloned()
        .collect::<Vec<_>>();
    features.sort();
    features
}

pub(crate) fn deploy_preflight_env_values(
    listen: Option<&orv_compiler::ServerListenArtifact>,
    persistence: &DeployPersistence,
    required: bool,
) -> serde_json::Value {
    serde_json::Value::Array(
        deploy_preflight_env_contract(listen, persistence)
            .into_iter()
            .filter(|env| env.required == required)
            .map(|env| {
                serde_json::json!({
                    "kind": env.kind,
                    "env": env.env,
                    "required": env.required,
                    "purpose": env.purpose,
                    "default": env.default,
                    "provider": env.provider,
                })
            })
            .collect(),
    )
}

pub(crate) fn deploy_preflight_env_contract(
    listen: Option<&orv_compiler::ServerListenArtifact>,
    persistence: &DeployPersistence,
) -> Vec<DeployPreflightEnv> {
    let mut envs = BTreeSet::new();
    if let Some(env) = listen.and_then(|listen| listen.env.as_ref()) {
        envs.insert(DeployPreflightEnv {
            kind: "listen".to_string(),
            env: env.variable.clone(),
            required: env.default_port.is_none(),
            purpose: "port".to_string(),
            default: env.default_port.map(|port| port.to_string()),
            provider: None,
        });
    }
    for env in &persistence.db_env {
        envs.insert(DeployPreflightEnv {
            kind: "db".to_string(),
            env: env.env.clone(),
            required: env.default.is_none(),
            purpose: "adapter_url".to_string(),
            default: env.default.clone(),
            provider: None,
        });
    }
    for adapter in &persistence.db_adapters {
        for env in &adapter.bridge_env {
            envs.insert(DeployPreflightEnv {
                kind: "db".to_string(),
                env: env.env.clone(),
                required: env.required,
                purpose: env.purpose.clone(),
                default: None,
                provider: Some(adapter.provider.clone()),
            });
        }
    }
    for adapter in &persistence.commerce_adapters {
        if let Some(env) = &adapter.env {
            envs.insert(DeployPreflightEnv {
                kind: adapter.kind.clone(),
                env: env.clone(),
                required: adapter.default.is_none(),
                purpose: "adapter_url".to_string(),
                default: adapter.default.clone(),
                provider: adapter.provider.clone(),
            });
        }
        for env in &adapter.provider_env {
            envs.insert(DeployPreflightEnv {
                kind: adapter.kind.clone(),
                env: env.env.clone(),
                required: env.required,
                purpose: env.purpose.clone(),
                default: None,
                provider: adapter.provider.clone(),
            });
        }
    }
    envs.into_iter().collect()
}

pub(crate) fn deploy_db_adapter_value(adapters: &[DeployDbAdapter]) -> Vec<serde_json::Value> {
    adapters
        .iter()
        .map(|adapter| {
            serde_json::json!({
                "kind": "db",
                "mode": adapter.mode,
                "provider": adapter.provider,
                "env": adapter.env.as_deref(),
                "default": adapter.default.as_deref(),
                "endpoint": adapter.endpoint.as_deref(),
                "adapter_status": adapter.adapter_status,
                "source_origin_id": adapter.source_origin_ids.first().map(String::as_str),
                "source_origin_ids": adapter.source_origin_ids.clone(),
                "runtime": {
                    "status": adapter.adapter_status,
                    "query_methods": ["create", "find", "update", "delete", "transaction"],
                },
                "bridge": deploy_db_adapter_bridge_value(&adapter.bridge_env),
            })
        })
        .collect()
}

pub(crate) fn deploy_db_adapter_bridge_value(envs: &[DeployProviderEnv]) -> serde_json::Value {
    serde_json::json!({
        "contract": "http-json-v1",
        "method": "POST",
        "content_type": "application/json",
        "query_methods": [
            "create",
            "find",
            "findAll",
            "update",
            "delete",
            "upsert",
            "search",
            "count",
            "sum",
            "transaction",
            "schema",
        ],
        "body": {
            "kind": "orv.db.adapter",
            "contract": "http-json-v1",
            "provider": "adapter provider",
            "url": "adapter url",
            "method": "db method",
            "args": "runtime value array",
        },
        "retry": {
            "attempts": 3,
            "on": ["5xx", "connect_error", "read_error", "timeout"],
        },
        "env": deploy_provider_env_value(envs),
    })
}

pub(crate) fn deploy_adapter_env_value(envs: &[DeployAdapterEnv]) -> Vec<serde_json::Value> {
    envs.iter()
        .map(|env| {
            serde_json::json!({
                "env": env.env,
                "default": env.default.as_deref(),
            })
        })
        .collect()
}

pub(crate) fn deploy_commerce_adapter_value(
    adapters: &[DeployCommerceAdapter],
) -> Vec<serde_json::Value> {
    adapters
        .iter()
        .map(|adapter| {
            let mut value = serde_json::json!({
                "kind": adapter.kind,
                "surface": deploy_commerce_adapter_surface(&adapter.kind),
                "package": deploy_commerce_adapter_package(&adapter.kind),
                "provider_package": adapter.provider.as_deref().and_then(deploy_commerce_provider_package),
                "mode": adapter.mode,
                "env": adapter.env.as_deref(),
                "default": adapter.default.as_deref(),
                "endpoint": adapter.endpoint.as_deref(),
                "record_path": adapter.record_path.as_deref(),
                "source_origin_id": adapter.source_origin_ids.first().map(String::as_str),
                "source_origin_ids": adapter.source_origin_ids.clone(),
                "request": deploy_commerce_adapter_request_value(&adapter.kind),
            });
            if let Some(provider) = &adapter.provider {
                value
                    .as_object_mut()
                    .expect("commerce adapter value is an object")
                    .insert(
                        "provider".to_string(),
                        serde_json::Value::String(provider.clone()),
                    );
            }
            if !adapter.provider_env.is_empty() {
                value
                    .as_object_mut()
                    .expect("commerce adapter value is an object")
                    .insert(
                        "provider_env".to_string(),
                        serde_json::Value::Array(deploy_provider_env_value(&adapter.provider_env)),
                    );
            }
            value
        })
        .collect()
}

pub(crate) fn deploy_provider_env_value(envs: &[DeployProviderEnv]) -> Vec<serde_json::Value> {
    envs.iter()
        .map(|env| {
            serde_json::json!({
                "env": env.env,
                "required": env.required,
                "purpose": env.purpose,
            })
        })
        .collect()
}

pub(crate) fn deploy_persistence_volumes(wal_paths: &[String]) -> Vec<DeployPersistenceVolume> {
    let mut dirs = BTreeSet::new();
    for wal_path in wal_paths {
        if let Some(dir) = deploy_persistent_parent_dir(wal_path) {
            dirs.insert(dir);
        }
    }
    dirs.into_iter()
        .map(|host| {
            let container = format!("/app/{host}");
            DeployPersistenceVolume {
                compose_mount: format!("../{host}:{container}"),
                host,
                container,
            }
        })
        .collect()
}

pub(crate) fn deploy_persistent_parent_dir(path: &str) -> Option<String> {
    let path = Path::new(path);
    if path.is_absolute() {
        return None;
    }
    let parent = path.parent()?;
    if parent.as_os_str().is_empty()
        || parent
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return None;
    }
    Some(parent.to_string_lossy().replace('\\', "/"))
}

pub(crate) fn deploy_compose_volumes(persistence: &DeployPersistence) -> String {
    if persistence.volumes.is_empty() {
        return String::new();
    }
    let mut out = String::from("    volumes:\n");
    for volume in &persistence.volumes {
        let _ = writeln!(out, "      - {}", volume.compose_mount);
    }
    out
}

pub(crate) fn deploy_compose_content(
    dockerfile_path: &str,
    listen: Option<&orv_compiler::ServerListenArtifact>,
    persistence: &DeployPersistence,
) -> String {
    let ports = deploy_compose_ports(listen);
    let environment = deploy_compose_environment(listen, persistence);
    let volumes = deploy_compose_volumes(persistence);
    format!(
        r#"services:
  orv-app:
    build:
      context: ..
      dockerfile: {dockerfile_path}
      args:
        ORV_RUNTIME_IMAGE: {ORV_REFERENCE_RUNTIME_IMAGE}
    image: orv-reference-app:latest
{ports}{environment}{volumes}"#
    )
}

pub(crate) fn deploy_dockerfile_content(
    runtime_image: &str,
    listen: Option<&orv_compiler::ServerListenArtifact>,
) -> String {
    let expose = deploy_exposed_port(listen)
        .map(|port| format!("EXPOSE {port}\n"))
        .unwrap_or_default();
    format!(
        r#"ARG ORV_RUNTIME_IMAGE={runtime_image}
FROM ${{ORV_RUNTIME_IMAGE}}
WORKDIR /app
COPY . /app
ENV ORV_HOST=0.0.0.0
{expose}ENTRYPOINT ["./deploy/server.sh"]
"#
    )
}

pub(crate) fn deploy_env_example_content(
    listen: Option<&orv_compiler::ServerListenArtifact>,
    persistence: &DeployPersistence,
) -> String {
    let mut env_example = String::from("# orv deploy environment\n");
    for assignment in deploy_env_example_assignments(listen, persistence) {
        let _ = writeln!(env_example, "{assignment}");
    }
    env_example
}

pub(crate) fn deploy_runbook_persistence_section(persistence: &DeployPersistence) -> String {
    let has_db_bridge_env = persistence
        .db_adapters
        .iter()
        .any(|adapter| !adapter.bridge_env.is_empty());
    let has_provider_env = persistence
        .commerce_adapters
        .iter()
        .any(|adapter| !adapter.provider_env.is_empty());
    if persistence.wal_paths.is_empty()
        && persistence.db_paths.is_empty()
        && persistence.db_endpoints.is_empty()
        && persistence.db_env.is_empty()
        && persistence.record_paths.is_empty()
        && persistence.commerce_endpoints.is_empty()
        && persistence.commerce_env.is_empty()
        && !has_db_bridge_env
        && !has_provider_env
    {
        return String::new();
    }
    let mut out = String::from("## Persistent Data\n\n");
    for path in &persistence.wal_paths {
        let _ = writeln!(out, "- WAL: {path}");
    }
    for path in &persistence.db_paths {
        let _ = writeln!(out, "- DB: {path}");
    }
    for endpoint in &persistence.db_endpoints {
        let _ = writeln!(out, "- DB endpoint: {endpoint}");
    }
    for env in &persistence.db_env {
        match &env.default {
            Some(default) => {
                let _ = writeln!(out, "- DB adapter env: {} default {default}", env.env);
            }
            None => {
                let _ = writeln!(out, "- DB adapter env: {}", env.env);
            }
        }
    }
    for adapter in &persistence.db_adapters {
        for env in &adapter.bridge_env {
            let required = if env.required { "required" } else { "optional" };
            let _ = writeln!(
                out,
                "- DB bridge env: {} {} {required} {}",
                adapter.provider, env.env, env.purpose
            );
        }
    }
    for path in &persistence.record_paths {
        let _ = writeln!(out, "- Record log: {path}");
    }
    for endpoint in &persistence.commerce_endpoints {
        let _ = writeln!(out, "- Commerce endpoint: {endpoint}");
    }
    for env in &persistence.commerce_env {
        match &env.default {
            Some(default) => {
                let _ = writeln!(out, "- Commerce adapter env: {} default {default}", env.env);
            }
            None => {
                let _ = writeln!(out, "- Commerce adapter env: {}", env.env);
            }
        }
    }
    for adapter in &persistence.commerce_adapters {
        let Some(provider) = &adapter.provider else {
            continue;
        };
        for env in &adapter.provider_env {
            let required = if env.required { "required" } else { "optional" };
            let _ = writeln!(
                out,
                "- Commerce provider env: {} {provider} {} {required} {}",
                adapter.kind, env.env, env.purpose
            );
        }
    }
    let operations = deploy_runbook_operations_section(has_db_bridge_env, has_provider_env);
    if !operations.is_empty() {
        out.push_str(&operations);
    }
    for volume in &persistence.volumes {
        let _ = writeln!(out, "- Compose volume: {}", volume.compose_mount);
    }
    out.push('\n');
    out
}

pub(crate) fn deploy_runbook_operations_section(
    has_db_bridge_env: bool,
    has_provider_env: bool,
) -> String {
    if !has_db_bridge_env && !has_provider_env {
        return String::new();
    }
    let mut out = String::from("\n## Provider Operations\n\n");
    if has_provider_env {
        out.push_str("- Secret store: supply commerce provider credentials through deployment secret manager or vault values, not deploy/env.example.\n");
        out.push_str("- Stripe webhook rotation: set STRIPE_WEBHOOK_SECRET to the new value and STRIPE_WEBHOOK_SECRET_PREVIOUS to the previous value during overlap.\n");
        out.push_str("- Stripe replay window: STRIPE_WEBHOOK_TOLERANCE_SECONDS defaults to 300 seconds; override only with provider runbook approval.\n");
        out.push_str("- Provider replay: payment and shipping calls use stable idempotency keys; inspect provider records before retrying checkout compensation.\n");
    }
    if has_db_bridge_env {
        out.push_str("- DB bridge secret store: supply ORV_DB_ADAPTER_*_AUTH_TOKEN values through deployment secret manager or vault values, not deploy/env.example.\n");
        out.push_str("- DB bridge rotation: prefer provider-specific auth token envs before ORV_DB_ADAPTER_AUTH_TOKEN fallback during rotation.\n");
        out.push_str("- DB bridge replay: bridge calls use bounded transient retry; confirm provider-side idempotency before replaying writes.\n");
    }
    out
}

pub(crate) fn file_adapter_path(url: &str) -> Option<String> {
    let path = url.strip_prefix("file://")?;
    if path.is_empty() {
        return None;
    }
    Some(path.to_string())
}

pub(crate) fn sqlite_adapter_path(url: &str) -> Option<String> {
    let path = url.strip_prefix("sqlite://")?;
    if path.is_empty() {
        return None;
    }
    Some(path.to_string())
}

pub(crate) fn http_adapter_endpoint(url: &str) -> Option<String> {
    url.strip_prefix("http://")
        .filter(|target| !target.is_empty())
        .map(|_| url.to_string())
}

pub(crate) fn collect_db_adapter_persistence_arg(
    arg: &orv_hir::HirExpr,
    source_origin_id: Option<String>,
    out: &mut DeployPersistenceAccumulator,
) {
    if let Some(url) = hir_static_string(arg) {
        collect_db_adapter_url(&url, None, None, source_origin_id.clone(), out);
    }
    if let Some(env) = hir_env_configured_string(arg) {
        if let Some(default) = &env.default {
            collect_db_adapter_url(
                default,
                Some(env.env.clone()),
                Some(default.clone()),
                source_origin_id.clone(),
                out,
            );
        } else {
            out.db_adapters.push(DeployDbAdapter {
                mode: "env".to_string(),
                provider: "unknown".to_string(),
                env: Some(env.env.clone()),
                default: None,
                endpoint: None,
                adapter_status: "env_required".to_string(),
                bridge_env: Vec::new(),
                source_origin_ids: source_origin_id.clone().into_iter().collect(),
            });
        }
        out.db_env.push(env);
    }
}

pub(crate) fn collect_db_adapter_url(
    url: &str,
    env: Option<String>,
    default: Option<String>,
    source_origin_id: Option<String>,
    out: &mut DeployPersistenceAccumulator,
) {
    if let Some(path) = file_adapter_path(url) {
        out.wal_paths.push(path);
    }
    if let Some(path) = sqlite_adapter_path(url) {
        out.db_paths.push(path);
    }
    if let Some(provider) = external_db_adapter_provider(url) {
        out.db_endpoints.push(url.to_string());
        out.db_adapters.push(DeployDbAdapter {
            mode: "external".to_string(),
            provider: provider.to_string(),
            env,
            default,
            endpoint: Some(url.to_string()),
            adapter_status: "unsupported_runtime".to_string(),
            bridge_env: db_adapter_bridge_env(provider),
            source_origin_ids: source_origin_id.into_iter().collect(),
        });
    }
}

pub(crate) fn hir_source_origin_id(kind: &str, name: &str, span: Span) -> Option<String> {
    (span.file != FileId::DUMMY).then(|| orv_hir::origin_id(kind, name, span))
}

pub(crate) fn external_db_adapter_provider(url: &str) -> Option<&'static str> {
    if url
        .strip_prefix("postgres://")
        .is_some_and(|target| !target.is_empty())
    {
        return Some("postgres");
    }
    if url
        .strip_prefix("mysql://")
        .is_some_and(|target| !target.is_empty())
    {
        return Some("mysql");
    }
    None
}

pub(crate) fn db_adapter_bridge_env(provider: &str) -> Vec<DeployProviderEnv> {
    match provider {
        "postgres" => vec![
            deploy_provider_env("ORV_DB_ADAPTER_POSTGRES_ENDPOINT", true, "bridge_endpoint"),
            deploy_provider_env(
                "ORV_DB_ADAPTER_POSTGRES_AUTH_TOKEN",
                false,
                "bridge_auth_token",
            ),
            deploy_provider_env("ORV_DB_ADAPTER_ENDPOINT", false, "bridge_endpoint_fallback"),
            deploy_provider_env(
                "ORV_DB_ADAPTER_AUTH_TOKEN",
                false,
                "bridge_auth_token_fallback",
            ),
        ],
        "mysql" => vec![
            deploy_provider_env("ORV_DB_ADAPTER_MYSQL_ENDPOINT", true, "bridge_endpoint"),
            deploy_provider_env(
                "ORV_DB_ADAPTER_MYSQL_AUTH_TOKEN",
                false,
                "bridge_auth_token",
            ),
            deploy_provider_env("ORV_DB_ADAPTER_ENDPOINT", false, "bridge_endpoint_fallback"),
            deploy_provider_env(
                "ORV_DB_ADAPTER_AUTH_TOKEN",
                false,
                "bridge_auth_token_fallback",
            ),
        ],
        _ => Vec::new(),
    }
}

pub(crate) fn collect_commerce_adapter_persistence_arg(
    kind: &str,
    arg: &orv_hir::HirExpr,
    source_origin_id: Option<String>,
    out: &mut DeployPersistenceAccumulator,
) {
    if let Some(url) = hir_static_string(arg) {
        collect_commerce_adapter_url(kind, &url, None, None, source_origin_id.clone(), out);
    }
    if let Some(env) = hir_env_configured_string(arg) {
        if let Some(default) = &env.default {
            collect_commerce_adapter_url(
                kind,
                default,
                Some(env.env.clone()),
                Some(default.clone()),
                source_origin_id.clone(),
                out,
            );
        } else {
            out.commerce_adapters.push(DeployCommerceAdapter {
                kind: kind.to_string(),
                mode: "env".to_string(),
                provider: None,
                env: Some(env.env.clone()),
                default: None,
                endpoint: None,
                record_path: None,
                provider_env: Vec::new(),
                source_origin_ids: source_origin_id.clone().into_iter().collect(),
            });
        }
        out.commerce_env.push(env);
    }
}

pub(crate) fn collect_commerce_adapter_url(
    kind: &str,
    url: &str,
    env: Option<String>,
    default: Option<String>,
    source_origin_id: Option<String>,
    out: &mut DeployPersistenceAccumulator,
) {
    let mut mode = "local".to_string();
    let mut provider = commerce_provider(url, kind);
    let mut record_path = None;
    let mut endpoint = None;
    if let Some(path) = file_adapter_path(url) {
        mode = "file".to_string();
        provider = None;
        record_path = Some(path.clone());
        out.record_paths.push(path);
    }
    if let Some(http_endpoint) = http_adapter_endpoint(url) {
        mode = "http".to_string();
        provider = None;
        endpoint = Some(http_endpoint.clone());
        out.commerce_endpoints.push(http_endpoint);
    }
    if provider.is_some() {
        mode = "provider".to_string();
    }
    let provider_env = provider
        .as_deref()
        .map(|provider| commerce_provider_env_for_url(provider, url))
        .unwrap_or_default();
    out.commerce_adapters.push(DeployCommerceAdapter {
        kind: kind.to_string(),
        mode,
        provider,
        env,
        default,
        endpoint,
        record_path,
        provider_env,
        source_origin_ids: source_origin_id.into_iter().collect(),
    });
}

pub(crate) fn deploy_provider_env(env: &str, required: bool, purpose: &str) -> DeployProviderEnv {
    DeployProviderEnv {
        env: env.to_string(),
        required,
        purpose: purpose.to_string(),
    }
}

pub(crate) fn hir_env_configured_string(expr: &orv_hir::HirExpr) -> Option<DeployAdapterEnv> {
    match &expr.kind {
        orv_hir::HirExprKind::Paren(inner) => hir_env_configured_string(inner),
        orv_hir::HirExprKind::Binary {
            op: orv_hir::BinaryOp::Coalesce,
            lhs,
            rhs,
        } => {
            let env = hir_env_variable(lhs)?;
            Some(DeployAdapterEnv {
                env,
                default: hir_static_string(rhs),
            })
        }
        _ => hir_env_variable(expr).map(|env| DeployAdapterEnv { env, default: None }),
    }
}

pub(crate) fn hir_env_variable(expr: &orv_hir::HirExpr) -> Option<String> {
    match &expr.kind {
        orv_hir::HirExprKind::Paren(inner) => hir_env_variable(inner),
        orv_hir::HirExprKind::Field { target, field, .. } => match &target.kind {
            orv_hir::HirExprKind::Domain { name, args, .. } if name == "env" && args.is_empty() => {
                Some(field.clone())
            }
            _ => None,
        },
        _ => None,
    }
}

pub(crate) fn hir_static_string(expr: &orv_hir::HirExpr) -> Option<String> {
    if let orv_hir::HirExprKind::Paren(inner) = &expr.kind {
        return hir_static_string(inner);
    }
    let orv_hir::HirExprKind::String(segments) = &expr.kind else {
        return None;
    };
    let mut out = String::new();
    for segment in segments {
        match segment {
            orv_hir::HirStringSegment::Str(value) => out.push_str(value),
            orv_hir::HirStringSegment::Interp(_) => return None,
        }
    }
    Some(out)
}

pub(crate) fn hir_call_name(expr: &orv_hir::HirExpr) -> String {
    match &expr.kind {
        orv_hir::HirExprKind::Ident(ident) => ident.name.clone(),
        orv_hir::HirExprKind::Field { target, field, .. } => {
            format!("{}.{}", hir_call_name(target), field)
        }
        orv_hir::HirExprKind::OptionalField { target, field, .. } => {
            format!("{}?.{}", hir_call_name(target), field)
        }
        orv_hir::HirExprKind::Domain { name, .. } => format!("@{name}"),
        orv_hir::HirExprKind::TypeName(name) => name.clone(),
        _ => "<expr>".to_string(),
    }
}

pub(crate) fn deploy_ports_value(
    listen: Option<&orv_compiler::ServerListenArtifact>,
) -> serde_json::Value {
    let Some(listen) = listen else {
        return serde_json::json!([]);
    };
    if let Some(port) = listen.port.filter(|port| *port > 0) {
        return serde_json::json!([
            {
                "container": port,
                "protocol": "tcp",
            }
        ]);
    }
    let Some(env) = &listen.env else {
        return serde_json::json!([]);
    };
    let mut port = serde_json::json!({
        "env": env.variable.clone(),
        "protocol": "tcp",
    });
    if let Some(default_port) = env.default_port.filter(|port| *port > 0) {
        port["default"] = serde_json::json!(default_port);
    }
    serde_json::json!([port])
}

pub(crate) fn deploy_exposed_port(
    listen: Option<&orv_compiler::ServerListenArtifact>,
) -> Option<u16> {
    listen
        .and_then(|listen| {
            listen
                .port
                .or_else(|| listen.env.as_ref().and_then(|env| env.default_port))
        })
        .filter(|port| *port > 0)
}

pub(crate) struct DeployComposePort {
    pub(crate) binding: String,
    pub(crate) environment: String,
    pub(crate) display: String,
}

pub(crate) fn deploy_compose_port(
    listen: Option<&orv_compiler::ServerListenArtifact>,
) -> Option<DeployComposePort> {
    let listen = listen?;
    if let Some(port) = listen.port.filter(|port| *port > 0) {
        return Some(DeployComposePort {
            binding: format!("\"{port}:{port}\""),
            environment: format!("PORT: \"{port}\""),
            display: port.to_string(),
        });
    }
    let env = listen.env.as_ref()?;
    let variable = &env.variable;
    if let Some(default_port) = env.default_port.filter(|port| *port > 0) {
        return Some(DeployComposePort {
            binding: format!("\"${{{variable}:-{default_port}}}:{default_port}\""),
            environment: format!("PORT: \"${{{variable}:-{default_port}}}\""),
            display: default_port.to_string(),
        });
    }
    Some(DeployComposePort {
        binding: format!("\"${{{variable}}}:${{{variable}}}\""),
        environment: format!("PORT: \"${{{variable}}}\""),
        display: format!("${{{variable}}}"),
    })
}

pub(crate) fn deploy_compose_ports(listen: Option<&orv_compiler::ServerListenArtifact>) -> String {
    deploy_compose_port(listen)
        .map(|port| format!("    ports:\n      - {}\n", port.binding))
        .unwrap_or_default()
}

pub(crate) fn deploy_compose_environment_lines(
    listen: Option<&orv_compiler::ServerListenArtifact>,
    persistence: &DeployPersistence,
) -> Vec<String> {
    let mut lines = Vec::new();
    if let Some(port) = deploy_compose_port(listen) {
        lines.push(port.environment);
    }
    lines.push(r#"ORV_HOST: "${ORV_HOST:-0.0.0.0}""#.to_string());
    for env in &persistence.db_env {
        let variable = &env.env;
        let value = match &env.default {
            Some(default) => format!("{variable}: \"${{{variable}:-{default}}}\""),
            None => format!("{variable}: \"${{{variable}}}\""),
        };
        lines.push(value);
    }
    for env in deploy_db_bridge_envs(persistence) {
        let variable = &env.env;
        lines.push(format!("{variable}: \"${{{variable}}}\""));
    }
    for env in &persistence.commerce_env {
        let variable = &env.env;
        let value = match &env.default {
            Some(default) => format!("{variable}: \"${{{variable}:-{default}}}\""),
            None => format!("{variable}: \"${{{variable}}}\""),
        };
        lines.push(value);
    }
    for env in deploy_commerce_provider_envs(persistence) {
        let variable = &env.env;
        lines.push(format!("{variable}: \"${{{variable}}}\""));
    }
    lines
}

pub(crate) fn deploy_compose_environment(
    listen: Option<&orv_compiler::ServerListenArtifact>,
    persistence: &DeployPersistence,
) -> String {
    let lines = deploy_compose_environment_lines(listen, persistence);
    if lines.is_empty() {
        return String::new();
    }
    let mut out = String::from("    environment:\n");
    for line in lines {
        let _ = writeln!(out, "      {line}");
    }
    out
}

pub(crate) fn deploy_env_example_assignments(
    listen: Option<&orv_compiler::ServerListenArtifact>,
    persistence: &DeployPersistence,
) -> Vec<String> {
    let mut assignments = Vec::new();
    if let Some(port) = deploy_env_example_port_assignment(listen) {
        assignments.push(port);
    }
    assignments.push("ORV_HOST=0.0.0.0".to_string());
    assignments.extend(persistence.db_env.iter().map(deploy_adapter_env_assignment));
    assignments.extend(
        deploy_db_bridge_envs(persistence)
            .iter()
            .map(deploy_provider_env_assignment),
    );
    assignments.extend(
        persistence
            .commerce_env
            .iter()
            .map(deploy_adapter_env_assignment),
    );
    assignments.extend(
        deploy_commerce_provider_envs(persistence)
            .iter()
            .map(deploy_provider_env_assignment),
    );
    assignments
}

pub(crate) fn deploy_env_example_port_assignment(
    listen: Option<&orv_compiler::ServerListenArtifact>,
) -> Option<String> {
    let listen = listen?;
    if let Some(port) = listen.port.filter(|port| *port > 0) {
        return Some(format!("PORT={port}"));
    }
    let env = listen.env.as_ref()?;
    let value = env
        .default_port
        .filter(|port| *port > 0)
        .map_or_else(String::new, |port| port.to_string());
    Some(format!("{}={value}", env.variable))
}

pub(crate) fn deploy_adapter_env_assignment(env: &DeployAdapterEnv) -> String {
    match &env.default {
        Some(default) => format!("{}={default}", env.env),
        None => format!("{}=", env.env),
    }
}

pub(crate) fn deploy_provider_env_assignment(env: &DeployProviderEnv) -> String {
    format!("{}=", env.env)
}

pub(crate) fn deploy_db_bridge_envs(persistence: &DeployPersistence) -> Vec<DeployProviderEnv> {
    let mut envs = BTreeSet::new();
    for adapter in &persistence.db_adapters {
        for env in &adapter.bridge_env {
            envs.insert(env.clone());
        }
    }
    envs.into_iter().collect()
}

pub(crate) fn deploy_commerce_provider_envs(
    persistence: &DeployPersistence,
) -> Vec<DeployProviderEnv> {
    let mut envs = BTreeSet::new();
    for adapter in &persistence.commerce_adapters {
        for env in &adapter.provider_env {
            envs.insert(env.clone());
        }
    }
    envs.into_iter().collect()
}

pub(crate) fn deploy_runbook_port_assignment(
    listen: Option<&orv_compiler::ServerListenArtifact>,
) -> Option<String> {
    let listen = listen?;
    if let Some(port) = listen.port.filter(|port| *port > 0) {
        return Some(format!("PORT={port}"));
    }
    let env = listen.env.as_ref()?;
    let variable = &env.variable;
    if let Some(default_port) = env.default_port.filter(|port| *port > 0) {
        return Some(format!("PORT=${{{variable}:-{default_port}}}"));
    }
    Some(format!("PORT=${{{variable}}}"))
}

pub(crate) fn deploy_runbook_trace_events_url(
    listen: Option<&orv_compiler::ServerListenArtifact>,
) -> String {
    let port = deploy_listen_url_port(listen);
    format!("http://127.0.0.1:{port}/__orv/trace/events")
}

pub(crate) fn deploy_listen_url_port(
    listen: Option<&orv_compiler::ServerListenArtifact>,
) -> String {
    let port = listen
        .and_then(|listen| {
            listen
                .port
                .filter(|port| *port > 0)
                .map(|port| port.to_string())
                .or_else(|| {
                    listen.env.as_ref().map(|env| {
                        let variable = &env.variable;
                        env.default_port.filter(|port| *port > 0).map_or_else(
                            || format!("${{{variable}}}"),
                            |port| format!("${{{variable}:-{port}}}"),
                        )
                    })
                })
        })
        .unwrap_or_else(|| "8080".to_string());
    port
}

pub(crate) fn deploy_routes_include(
    artifact: &orv_compiler::ServerRuntimeArtifact,
    method: &str,
    path: &str,
) -> bool {
    artifact
        .routes
        .iter()
        .any(|route| route.method == method && route.path == path)
}
pub(crate) const DEPLOY_PREFLIGHT_PATH: &str = "deploy/preflight.json";

pub(crate) fn write_prod_deploy_artifacts(
    out: &Path,
    entry: &Path,
    manifest: &orv_compiler::BuildManifest,
    origin_map: &orv_compiler::OriginMap,
    server_artifact: Option<&orv_compiler::ServerRuntimeArtifact>,
    targets: ProdBuildTargets<'_>,
) -> anyhow::Result<()> {
    let client = prod_deploy_client_json(out, manifest.capabilities.client_wasm, targets)?;
    let server = if let Some(server_artifact) = server_artifact {
        let entrypoint = "deploy/server.sh";
        let routes_artifact = "deploy/routes.json";
        let container = "deploy/container.json";
        let dockerfile = "deploy/Dockerfile";
        let compose = "deploy/compose.yaml";
        let env_example = "deploy/env.example";
        let db_adapters = "deploy/db-adapters.json";
        let commerce_adapters = "deploy/commerce-adapters.json";
        let smoke_test = DEPLOY_SMOKE_TEST_PATH;
        let smoke_output = DEPLOY_SMOKE_OUTPUT_PATH;
        let preflight = DEPLOY_PREFLIGHT_PATH;
        let benchmark_evidence = DEPLOY_BENCHMARK_EVIDENCE_PATH;
        let participant_notes_template = DEPLOY_PARTICIPANT_NOTES_TEMPLATE_PATH;
        let runbook = "deploy/README.md";
        let persistence = server_artifact_deploy_persistence(server_artifact)?;
        write_prod_server_entrypoint(out, targets.server_artifact)?;
        write_prod_routes_artifact(out, targets.server_artifact, server_artifact)?;
        write_prod_container_artifacts(
            out,
            targets.server_artifact,
            entrypoint,
            routes_artifact,
            dockerfile,
            server_artifact,
            &persistence,
        )?;
        write_prod_compose_artifact(out, dockerfile, server_artifact, &persistence)?;
        write_prod_env_example_artifact(out, env_example, server_artifact, &persistence)?;
        write_prod_db_adapters_artifact(out, db_adapters, targets.server_artifact, &persistence)?;
        write_prod_commerce_adapters_artifact(
            out,
            commerce_adapters,
            targets.server_artifact,
            &persistence,
        )?;
        write_prod_smoke_test_artifact(
            out,
            smoke_test,
            server_artifact,
            origin_map,
            &persistence,
            &client,
        )?;
        let deploy_artifacts = DeployRunbookArtifacts {
            server_artifact: targets.server_artifact,
            compose,
            env_example,
            db_adapters,
            commerce_adapters,
            smoke_test,
            smoke_output,
            preflight,
            benchmark_evidence,
            participant_notes_template,
            runbook,
            routes: routes_artifact,
        };
        write_prod_preflight_artifact(
            out,
            preflight,
            &deploy_artifacts,
            server_artifact,
            &persistence,
            &client,
        )?;
        write_prod_benchmark_evidence_artifact(
            out,
            benchmark_evidence,
            &deploy_artifacts,
            server_artifact,
            &persistence,
            &client,
        )?;
        write_prod_participant_notes_template_artifact(out, participant_notes_template)?;
        write_prod_deploy_runbook(
            out,
            &deploy_artifacts,
            server_artifact,
            &persistence,
            &client,
        )?;
        serde_json::json!({
            "runtime": server_artifact.runtime.clone(),
            "runtime_features": server_artifact.runtime_features.clone(),
            "artifact": targets.server_artifact,
            "entrypoint": entrypoint,
            "routes_artifact": routes_artifact,
            "native_plan": targets.native_server_plan,
            "native_runtime_image_plan": targets.native_runtime_image_plan,
            "native_routes_source": targets.native_server_routes_source,
            "native_router_source": targets.native_server_router_source,
            "native_handlers_source": targets.native_server_handlers_source,
            "container": container,
            "dockerfile": dockerfile,
            "compose": compose,
            "env_example": env_example,
            "db_adapters": db_adapters,
            "commerce_adapters": commerce_adapters,
            "smoke_test": smoke_test,
            "smoke_output": smoke_output,
            "preflight": preflight,
            "benchmark_evidence": benchmark_evidence,
            "participant_notes_template": participant_notes_template,
            "runbook": runbook,
            "runtime_image": ORV_REFERENCE_RUNTIME_IMAGE,
            "protocol": "http1",
            "listen": server_artifact.listen.clone(),
            "routes": server_artifact.routes.clone(),
            "persistence": deploy_persistence_value(&persistence),
        })
    } else {
        serde_json::Value::Null
    };
    let static_target = targets.static_page.map_or(serde_json::Value::Null, |path| {
        serde_json::json!({
            "path": path,
            "runtime_features": [],
        })
    });
    let deploy = serde_json::json!({
        "schema_version": 1,
        "profile": "prod",
        "entry": entry.display().to_string(),
        "runtime": manifest.runtime.clone(),
        "runtime_features": manifest.capabilities.runtime_features.clone(),
        "source_bundle": "source-bundle.json",
        "server": server,
        "static": static_target,
        "client": client,
    });
    write_json(&out.join("deploy").join("manifest.json"), &deploy)
}

pub(crate) fn write_prod_commerce_adapters_artifact(
    out: &Path,
    path: &str,
    server_artifact_path: &str,
    persistence: &DeployPersistence,
) -> anyhow::Result<()> {
    let artifact = serde_json::json!({
        "schema_version": 1,
        "kind": "orv.deploy.commerce_adapters",
        "artifact": server_artifact_path,
        "adapters": deploy_commerce_adapter_value(&persistence.commerce_adapters),
    });
    write_json(&out.join(path), &artifact)
}

pub(crate) fn write_prod_db_adapters_artifact(
    out: &Path,
    path: &str,
    server_artifact_path: &str,
    persistence: &DeployPersistence,
) -> anyhow::Result<()> {
    let artifact = serde_json::json!({
        "schema_version": 1,
        "kind": "orv.deploy.db_adapters",
        "artifact": server_artifact_path,
        "adapters": deploy_db_adapter_value(&persistence.db_adapters),
    });
    write_json(&out.join(path), &artifact)
}

pub(crate) fn deploy_preflight_artifact_value(
    artifacts: &DeployRunbookArtifacts<'_>,
    server_artifact: &orv_compiler::ServerRuntimeArtifact,
    persistence: &DeployPersistence,
    client: Option<&serde_json::Value>,
) -> serde_json::Value {
    serde_json::json!({
        "schema_version": 1,
        "kind": "orv.deploy.preflight",
        "artifact": artifacts.server_artifact,
        "runtime": server_artifact.runtime.clone(),
        "runtime_features": server_artifact.runtime_features.clone(),
        "security_features": deploy_security_runtime_features(&server_artifact.runtime_features),
        "listen": server_artifact.listen.clone(),
        "routes": server_artifact.routes.clone(),
        "persistence": deploy_persistence_value(persistence),
        "required_env": deploy_preflight_env_values(server_artifact.listen.as_ref(), persistence, true),
        "optional_env": deploy_preflight_env_values(server_artifact.listen.as_ref(), persistence, false),
        "commands": deploy_preflight_commands_value(artifacts),
        "artifacts": deploy_preflight_artifacts_value(artifacts),
        "smoke_output_contract": deploy_smoke_output_contract_value(artifacts),
        "benchmark": deploy_preflight_benchmark_value(),
        "client": deploy_preflight_client_value(client),
    })
}

pub(crate) fn deploy_preflight_commands_value(
    artifacts: &DeployRunbookArtifacts<'_>,
) -> serde_json::Value {
    serde_json::json!({
        "verify_build": "orv verify-build .",
        "env_check": "orv deploy-env-check .",
        "run_build": "orv run-build .",
        "smoke_test": format!("./{}", artifacts.smoke_test),
        "editor_run_debug": "orv editor run-debug . --control next",
        "benchmark_prepare": "orv benchmark-prepare . --participants 2",
        "benchmark_report": "orv benchmark-report .",
        "benchmark_report_require_pass": "orv benchmark-report . --require-pass",
        "compose_up": format!("docker compose -f {} up --build -d", artifacts.compose),
        "trace": "./deploy/server.sh --trace deploy/request-trace.json",
        "trace_run_build": "orv run-build . --trace deploy/request-trace.json",
        "editor_trace": "orv editor trace . --trace deploy/request-trace.json",
        "trace_stream_smoke": format!("ORV_SMOKE_TRACE_STREAM=1 ./{}", artifacts.smoke_test),
    })
}

pub(crate) fn deploy_preflight_artifacts_value(
    artifacts: &DeployRunbookArtifacts<'_>,
) -> serde_json::Value {
    serde_json::json!({
        "server": artifacts.server_artifact,
        "routes": artifacts.routes,
        "source_bundle": SOURCE_BUNDLE_PATH,
        "project_graph": "project-graph.json",
        "origin_map": "origin-map.json",
        "build_manifest": "build-manifest.json",
        "bundle_plan": "bundle-plan.json",
        "env_example": artifacts.env_example,
        "db_adapters": artifacts.db_adapters,
        "commerce_adapters": artifacts.commerce_adapters,
        "smoke_test": artifacts.smoke_test,
        "smoke_output": artifacts.smoke_output,
        "preflight": artifacts.preflight,
        "benchmark_evidence": artifacts.benchmark_evidence,
        "participant_notes_template": artifacts.participant_notes_template,
        "runbook": artifacts.runbook,
    })
}

pub(crate) fn write_prod_deploy_runbook(
    out: &Path,
    artifacts: &DeployRunbookArtifacts<'_>,
    server_artifact: &orv_compiler::ServerRuntimeArtifact,
    persistence: &DeployPersistence,
    client: &serde_json::Value,
) -> anyhow::Result<()> {
    let runbook = deploy_runbook_content(artifacts, server_artifact, persistence, Some(client));
    write_text(&out.join("deploy").join("README.md"), &runbook)
}

pub(crate) fn deploy_runbook_content(
    artifacts: &DeployRunbookArtifacts<'_>,
    server_artifact: &orv_compiler::ServerRuntimeArtifact,
    persistence: &DeployPersistence,
    client: Option<&serde_json::Value>,
) -> String {
    let compose_path = artifacts.compose;
    let env_example_path = artifacts.env_example;
    let db_adapters_path = artifacts.db_adapters;
    let commerce_adapters_path = artifacts.commerce_adapters;
    let smoke_test_path = artifacts.smoke_test;
    let smoke_output_path = artifacts.smoke_output;
    let preflight_path = artifacts.preflight;
    let benchmark_evidence_path = artifacts.benchmark_evidence;
    let participant_notes_template_path = artifacts.participant_notes_template;
    let routes_artifact = artifacts.routes;
    let port_prefix = deploy_runbook_port_assignment(server_artifact.listen.as_ref())
        .map(|port| format!("{port} "))
        .unwrap_or_default();
    let trace_events_url = deploy_runbook_trace_events_url(server_artifact.listen.as_ref());
    let routes = server_artifact
        .routes
        .iter()
        .map(|route| format!("- {} {}\n", route.method, route.path))
        .collect::<String>();
    let persistence_section = deploy_runbook_persistence_section(persistence);
    let client_section = match client {
        Some(client) => deploy_runbook_client_section(client),
        None => String::new(),
    };
    let smoke_required_markers = deploy_benchmark::SMOKE_REQUIRED_MARKERS
        .iter()
        .map(|marker| format!("- `{marker}`\n"))
        .collect::<String>();
    format!(
        r#"# orv deploy

## Run

```sh
{port_prefix}docker compose -f {compose_path} up --build -d
```

Containers bind all IPv4 interfaces by default (`ORV_HOST=0.0.0.0`) so published
ports can reach the server. Override `ORV_HOST` with an IPv4 or IPv6 address
when needed. Direct local `orv run` and `orv run-build` default to `127.0.0.1`.

## Artifacts

- Compose: {compose_path}
- Env example: {env_example_path}
- DB adapters: {db_adapters_path}
- Commerce adapters: {commerce_adapters_path}
- Smoke test: {smoke_test_path}
- Smoke output: {smoke_output_path}
- Preflight: {preflight_path}
- Benchmark evidence: {benchmark_evidence_path}
- Participant notes template: {participant_notes_template_path}
- Routes: {routes_artifact}

## Native Launcher

```sh
cargo build --manifest-path server/native/Cargo.toml --release
ORV_BUILD_DIR=. ./server/native/target/release/orv-native-server
```

The generated launcher path can infer the build directory; ORV_BUILD_DIR is an explicit override.

## Native Runtime Image

```sh
docker build -f server/native/Dockerfile -t orv-native-server:latest .
```

## Request Trace

```sh
./deploy/server.sh --trace deploy/request-trace.json
curl -N {trace_events_url}
orv editor trace . --trace deploy/request-trace.json
ORV_SMOKE_TRACE_STREAM=1 ./{smoke_test_path}
```

## Deploy Preflight

```sh
orv verify-build .
orv deploy-env-check .
orv editor run-debug . --control next
orv benchmark-prepare . --participants 2
orv benchmark-report .
```

## Smoke Test

```sh
./{smoke_test_path}
```

## Benchmark Evidence

Run `orv benchmark-prepare . --participants 2` before the human run to create participant raw-notes files and seed `{benchmark_evidence_path}` participant rows. Record human-run timing and observation data in `{benchmark_evidence_path}` after the preflight and smoke commands pass. The file keeps the 5-hour shop benchmark tasks, data-to-record fields, and preflight hash together so benchmark reports stay tied to the checked build contract.
The generated smoke test writes `{smoke_output_path}` on success, and `orv benchmark-report .` uses it when the evidence `smoke_test_output` field is still empty. If evidence copies `smoke_test_output`, the copied value must match the retained `{smoke_output_path}` artifact or the benchmark report fails.

## Participant Notes Template

`orv benchmark-prepare . --participants 2` copies `{participant_notes_template_path}` once per participant under `deploy/evidence/`, then sets each `data.participant_runs[].raw_notes_artifact` value in `{benchmark_evidence_path}` to that forward-slash relative path. `orv benchmark-report . --require-pass` requires retained non-empty raw notes for the recorded participants, rejects raw notes that still contain generated placeholder fields, empty Task Notes, or generated template instruction prose, and treats duplicate identity fields or non-exact-once participant_id/run_id identity as incomplete.

## Smoke Output Markers

The benchmark report requires these markers in `{smoke_output_path}`:

{smoke_required_markers}

```sh
orv benchmark-report . --require-pass
```

{client_section}
{persistence_section}
## Routes

{routes}"#
    )
}

pub(crate) fn deploy_server_entrypoint_content(server_artifact_path: &str) -> String {
    format!(
        r#"#!/usr/bin/env sh
set -eu
SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
BUILD_DIR=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)
exec orv run-artifact "$BUILD_DIR/{server_artifact_path}" "$@"
"#
    )
}
