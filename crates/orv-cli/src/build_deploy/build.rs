use super::*;

pub(crate) fn bundle_target_path(
    plan: &serde_json::Value,
    kind: &str,
) -> anyhow::Result<Option<String>> {
    let bundles = plan
        .get("bundles")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| anyhow::anyhow!("bundle plan bundles must be an array"))?;
    for bundle in bundles {
        if bundle.get("kind").and_then(serde_json::Value::as_str) == Some(kind) {
            return Ok(Some(json_str(bundle, "path", "bundle target")?.to_string()));
        }
    }
    Ok(None)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum BuildProfile {
    Development,
    Production,
}

impl BuildProfile {
    pub(crate) const fn from_prod_flag(prod: bool) -> Self {
        if prod {
            Self::Production
        } else {
            Self::Development
        }
    }

    const fn is_production(self) -> bool {
        matches!(self, Self::Production)
    }

    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Development => "dev",
            Self::Production => "prod",
        }
    }
}

pub(crate) fn cmd_build(path: &Path, out: &Path) -> anyhow::Result<()> {
    cmd_build_with_profile(path, out, BuildProfile::Development)
}

pub(crate) fn cmd_build_with_profile(
    path: &Path,
    out: &Path,
    profile: BuildProfile,
) -> anyhow::Result<()> {
    let entry = project_entry_path(path)?;
    let loaded = orv_project::load_project(&entry).map_err(|e| anyhow::anyhow!("{e}"))?;
    report_diagnostics(&loaded.diagnostics, &loaded.files)?;
    let resolved = orv_resolve::resolve(&loaded.program);
    report_diagnostics(&resolved.diagnostics, &loaded.files)?;
    let lowered = orv_analyzer::lower_with_diagnostics(&loaded.program, &resolved);
    report_diagnostics(&lowered.diagnostics, &loaded.files)?;
    let origin_map = orv_compiler::origin_map(&lowered.program);
    let graph = project_graph_json(&loaded.graph, &origin_map);
    let manifest = orv_compiler::build_manifest(entry.display().to_string(), &origin_map);
    let bundle_plan = orv_compiler::bundle_plan(&manifest);
    let client_manifest_path = bundle_output_path(&bundle_plan, "client_manifest");
    let client_reactive_plan_path = bundle_output_path(&bundle_plan, "client_reactive_plan");
    let client_page_path = bundle_output_path(&bundle_plan, "client_page");
    let client_js_path = bundle_output_path(&bundle_plan, "client_js");
    let client_wasm_path = bundle_output_path(&bundle_plan, "client_wasm");
    let static_page = bundle_plan
        .bundles
        .iter()
        .find(|bundle| bundle.kind == "static_page")
        .map(|bundle| {
            render_static_page(&lowered).map(|html| (PathBuf::from(bundle.path.clone()), html))
        })
        .transpose()?;
    let static_page_path = static_page
        .as_ref()
        .map(|(path, _)| normalized_artifact_path(&path.display().to_string()));
    let server_artifact_path = SERVER_ARTIFACT_PATH;
    let server_launch_path = SERVER_LAUNCH_PATH;
    let native_server_plan_path = NATIVE_SERVER_PLAN_PATH;
    let native_runtime_image_plan_path = NATIVE_RUNTIME_IMAGE_PLAN_PATH;
    let native_runtime_image_dockerfile_path = NATIVE_RUNTIME_IMAGE_DOCKERFILE_PATH;
    let native_server_source_path = NATIVE_SERVER_SOURCE_PATH;
    let native_server_routes_source_path = NATIVE_SERVER_ROUTES_SOURCE_PATH;
    let native_server_router_source_path = NATIVE_SERVER_ROUTER_SOURCE_PATH;
    let native_server_handlers_source_path = NATIVE_SERVER_HANDLERS_SOURCE_PATH;
    let native_server_package_path = NATIVE_SERVER_PACKAGE_PATH;
    let source_bundle = orv_compiler::source_bundle_artifact(
        entry.display().to_string(),
        loaded
            .files
            .iter()
            .map(|file| (file.path.display().to_string(), file.source.clone())),
    );
    let server_artifact = manifest.capabilities.has_server.then(|| {
        orv_compiler::server_runtime_artifact_with_program(
            &manifest,
            &origin_map,
            &lowered.program,
            loaded
                .files
                .iter()
                .map(|file| (file.path.display().to_string(), file.source.clone())),
        )
    });
    if profile.is_production() {
        validate_prod_server_listen(server_artifact.as_ref())?;
    }

    std::fs::create_dir_all(out)
        .map_err(|e| anyhow::anyhow!("failed to create {}: {e}", out.display()))?;
    write_json(
        &out.join("build-manifest.json"),
        &serde_json::to_value(&manifest)?,
    )?;
    write_json(
        &out.join("bundle-plan.json"),
        &serde_json::to_value(&bundle_plan)?,
    )?;
    write_json(
        &out.join("origin-map.json"),
        &serde_json::to_value(&origin_map)?,
    )?;
    write_json(&out.join("project-graph.json"), &graph)?;
    let source_bundle_value = serde_json::to_value(&source_bundle)?;
    let source_bundle_hash = stable_json_hash(&source_bundle_value)?;
    write_json(&out.join("source-bundle.json"), &source_bundle_value)?;
    let client_initial_render = if manifest.capabilities.client_wasm {
        Some(render_static_page(&lowered)?)
    } else {
        None
    };
    if let Some(server_artifact) = &server_artifact {
        write_json(
            &out.join(server_artifact_path),
            &serde_json::to_value(server_artifact)?,
        )?;
        let launch = orv_compiler::server_launch_artifact(server_artifact_path, server_artifact);
        write_json(
            &out.join(server_launch_path),
            &serde_json::to_value(launch)?,
        )?;
        let native_server_paths = NativeServerPlanPaths {
            plan: native_server_plan_path,
            artifact: server_artifact_path,
            launcher: server_launch_path,
            source: native_server_source_path,
            routes_source: native_server_routes_source_path,
            router_source: native_server_router_source_path,
            handlers_source: native_server_handlers_source_path,
            package: native_server_package_path,
            runtime_image_plan: native_runtime_image_plan_path,
        };
        write_native_server_plan_artifact(out, &native_server_paths, server_artifact)?;
        write_native_runtime_image_plan_artifact(
            out,
            native_runtime_image_plan_path,
            native_runtime_image_dockerfile_path,
            server_artifact_path,
            native_server_plan_path,
            server_artifact,
        )?;
        write_native_runtime_image_dockerfile(out, native_runtime_image_dockerfile_path)?;
        write_native_server_launcher_source(
            out,
            native_server_source_path,
            server_artifact_path,
            native_server_plan_path,
            server_artifact,
        )?;
        write_native_server_routes_source(out, native_server_routes_source_path, server_artifact)?;
        write_native_server_router_source(out, native_server_router_source_path)?;
        write_native_server_handlers_source(
            out,
            native_server_handlers_source_path,
            server_artifact,
        )?;
        write_native_server_launcher_package(out, native_server_package_path)?;
    }
    if let Some((path, html)) = static_page {
        write_text(&out.join(path), &html)?;
    }
    let client_source_binding = ClientSourceBinding {
        source_bundle: &source_bundle,
        source_bundle_hash: &source_bundle_hash,
        origin_map: &origin_map,
        program: &lowered.program,
        initial_render: client_initial_render.as_deref().unwrap_or(""),
    };
    let client_bundle_targets = ClientBundleTargets {
        manifest: client_manifest_path.as_deref(),
        reactive_plan: client_reactive_plan_path.as_deref(),
        page: client_page_path.as_deref(),
        js: client_js_path.as_deref(),
        wasm: client_wasm_path.as_deref(),
    };
    write_client_bundle_artifacts(
        out,
        &entry,
        manifest.capabilities.client_wasm,
        &client_source_binding,
        &client_bundle_targets,
    )?;
    if profile.is_production() {
        write_prod_deploy_artifacts(
            out,
            &entry,
            &manifest,
            &origin_map,
            server_artifact.as_ref(),
            ProdBuildTargets {
                static_page: static_page_path.as_deref(),
                client_manifest: client_manifest_path.as_deref(),
                client_reactive_plan: client_reactive_plan_path.as_deref(),
                client_page: client_page_path.as_deref(),
                client_js: client_js_path.as_deref(),
                client_wasm: client_wasm_path.as_deref(),
                server_artifact: server_artifact_path,
                native_server_plan: native_server_plan_path,
                native_runtime_image_plan: native_runtime_image_plan_path,
                native_server_routes_source: native_server_routes_source_path,
                native_server_router_source: native_server_router_source_path,
                native_server_handlers_source: native_server_handlers_source_path,
            },
        )?;
    }
    println!("build: wrote {}", out.display());
    Ok(())
}

pub(crate) fn bundle_output_path(plan: &orv_compiler::BundlePlan, kind: &str) -> Option<String> {
    plan.bundles
        .iter()
        .find(|bundle| bundle.kind == kind)
        .map(|bundle| normalized_artifact_path(&bundle.path))
}

#[derive(Clone, Copy)]
pub(crate) struct ProdBuildTargets<'a> {
    pub(crate) static_page: Option<&'a str>,
    pub(crate) client_manifest: Option<&'a str>,
    pub(crate) client_reactive_plan: Option<&'a str>,
    pub(crate) client_page: Option<&'a str>,
    pub(crate) client_js: Option<&'a str>,
    pub(crate) client_wasm: Option<&'a str>,
    pub(crate) server_artifact: &'a str,
    pub(crate) native_server_plan: &'a str,
    pub(crate) native_runtime_image_plan: &'a str,
    pub(crate) native_server_routes_source: &'a str,
    pub(crate) native_server_router_source: &'a str,
    pub(crate) native_server_handlers_source: &'a str,
}
