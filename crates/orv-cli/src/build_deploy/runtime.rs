use super::*;

pub(crate) fn cmd_run_artifact(path: &Path, trace: Option<&Path>) -> anyhow::Result<()> {
    let mut stdout = std::io::stdout().lock();
    run_artifact_with_writer_with_trace(path, trace, &mut stdout)
}

pub(crate) fn cmd_run_build(dir: &Path, trace: Option<&Path>) -> anyhow::Result<()> {
    let mut stdout = std::io::stdout().lock();
    run_build_with_writer_with_trace(dir, trace, &mut stdout)
}

pub(crate) fn run_build_with_writer<W: std::io::Write>(
    dir: &Path,
    writer: &mut W,
) -> anyhow::Result<()> {
    run_build_with_writer_with_trace(dir, None, writer)
}

pub(crate) fn run_build_with_writer_with_trace<W: std::io::Write>(
    dir: &Path,
    trace: Option<&Path>,
    writer: &mut W,
) -> anyhow::Result<()> {
    let build_dir = dir
        .canonicalize()
        .map_err(|e| anyhow::anyhow!("failed to resolve build dir {}: {e}", dir.display()))?;
    let plan_path = build_dir.join("bundle-plan.json");
    if plan_path.is_file() {
        let plan = read_json_value(&plan_path)?;
        if let Some(launcher) = bundle_target_path(&plan, "server_launcher")? {
            let launch_path = build_dir.join(launcher);
            verify_server_launcher_target(&build_dir, &launch_path)?;
            let launch = read_server_launch_artifact(&launch_path)?;
            return run_artifact_with_writer_with_build_dir(
                &build_dir.join(launch.artifact),
                &build_dir,
                trace,
                writer,
            );
        }
        return run_static_build_with_writer(&build_dir, writer);
    }
    let launch_path = build_dir.join("server").join("launch.json");
    if launch_path.is_file() {
        verify_server_launcher_target(&build_dir, &launch_path)?;
        let launch = read_server_launch_artifact(&launch_path)?;
        return run_artifact_with_writer_with_build_dir(
            &build_dir.join(launch.artifact),
            &build_dir,
            trace,
            writer,
        );
    }
    run_static_build_with_writer(&build_dir, writer)
}

pub(crate) fn run_static_build_with_writer<W: std::io::Write>(
    dir: &Path,
    writer: &mut W,
) -> anyhow::Result<()> {
    let plan = read_json_value(&dir.join("bundle-plan.json"))?;
    let bundles = plan
        .get("bundles")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| anyhow::anyhow!("bundle plan bundles must be an array"))?;
    if let Some(bundle) = bundles.iter().find(|bundle| {
        bundle.get("kind").and_then(serde_json::Value::as_str) == Some("static_page")
    }) {
        let path = json_str(bundle, "path", "bundle target")?;
        let target = dir.join(path);
        verify_static_page_target(bundle, &target)?;
        let html = std::fs::read_to_string(&target)
            .map_err(|e| anyhow::anyhow!("failed to read {}: {e}", target.display()))?;
        writer.write_all(html.as_bytes())?;
        return Ok(());
    }
    let bundle = bundles
        .iter()
        .find(|bundle| {
            bundle.get("kind").and_then(serde_json::Value::as_str) == Some("client_page")
        })
        .ok_or_else(|| anyhow::anyhow!("build has no server launcher or page target"))?;
    let path = json_str(bundle, "path", "bundle target")?;
    let target = dir.join(path);
    verify_client_page_target(bundle, &target)?;
    let html = std::fs::read_to_string(&target)
        .map_err(|e| anyhow::anyhow!("failed to read {}: {e}", target.display()))?;
    writer.write_all(html.as_bytes())?;
    Ok(())
}

#[cfg(test)]
pub(crate) fn run_artifact_with_writer<W: std::io::Write>(
    path: &Path,
    writer: &mut W,
) -> anyhow::Result<()> {
    run_artifact_with_writer_with_trace(path, None, writer)
}

pub(crate) fn run_artifact_with_writer_with_trace<W: std::io::Write>(
    path: &Path,
    trace: Option<&Path>,
    writer: &mut W,
) -> anyhow::Result<()> {
    let options = orv_runtime::RuntimeOptions {
        request_trace_path: trace.map(Path::to_path_buf),
        ..orv_runtime::RuntimeOptions::default()
    };
    run_artifact_with_writer_with_options(path, writer, options)
}

pub(crate) fn run_artifact_with_writer_with_build_dir<W: std::io::Write>(
    path: &Path,
    build_dir: &Path,
    trace: Option<&Path>,
    writer: &mut W,
) -> anyhow::Result<()> {
    let options = orv_runtime::RuntimeOptions {
        request_trace_path: trace.map(|path| build_runtime_path(build_dir, path)),
        working_dir: Some(build_dir.to_path_buf()),
    };
    run_artifact_with_writer_with_options(path, writer, options)
}

pub(crate) fn build_runtime_path(build_dir: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        build_dir.join(path)
    }
}

pub(crate) fn run_artifact_with_writer_with_options<W: std::io::Write>(
    path: &Path,
    writer: &mut W,
    options: orv_runtime::RuntimeOptions,
) -> anyhow::Result<()> {
    let artifact = read_server_artifact(path)?;
    orv_compiler::verify_server_runtime_artifact(&artifact)
        .map_err(|errors| anyhow::anyhow!("{}", errors.join("; ")))?;
    let lowered = lower_artifact_entry(&artifact)?;
    orv_runtime::run_with_writer_with_options(&lowered.program, writer, options)
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    Ok(())
}

pub(crate) fn lower_artifact_entry(
    artifact: &orv_compiler::ServerRuntimeArtifact,
) -> anyhow::Result<orv_analyzer::LowerResult> {
    let entry = artifact_entry_path(artifact)?;
    let loaded = orv_project::load_project_from_sources(
        &entry,
        artifact
            .source_bundle
            .files
            .iter()
            .map(|file| (PathBuf::from(&file.path), file.source.clone())),
    )
    .map_err(|e| anyhow::anyhow!("{e}"))?;
    report_diagnostics(&loaded.diagnostics, &loaded.files)?;
    let resolved = orv_resolve::resolve(&loaded.program);
    report_diagnostics(&resolved.diagnostics, &loaded.files)?;
    let lowered = orv_analyzer::lower_with_diagnostics(&loaded.program, &resolved);
    report_diagnostics(&lowered.diagnostics, &loaded.files)?;
    Ok(lowered)
}

pub(crate) fn lower_source_bundle_entry(
    artifact: &orv_compiler::SourceBundleArtifact,
) -> anyhow::Result<orv_analyzer::LowerResult> {
    let loaded = load_project_from_source_bundle_artifact(artifact)?;
    report_diagnostics(&loaded.diagnostics, &loaded.files)?;
    let resolved = orv_resolve::resolve(&loaded.program);
    report_diagnostics(&resolved.diagnostics, &loaded.files)?;
    let lowered = orv_analyzer::lower_with_diagnostics(&loaded.program, &resolved);
    report_diagnostics(&lowered.diagnostics, &loaded.files)?;
    Ok(lowered)
}

pub(crate) fn load_project_from_source_bundle_artifact(
    artifact: &orv_compiler::SourceBundleArtifact,
) -> anyhow::Result<orv_project::LoadedProject> {
    let entry = source_bundle_entry_path(artifact)?;
    orv_project::load_project_from_sources(
        &entry,
        artifact
            .files
            .iter()
            .map(|file| (PathBuf::from(&file.path), file.source.clone())),
    )
    .map_err(|e| anyhow::anyhow!("{e}"))
}

pub(crate) fn artifact_entry_path(
    artifact: &orv_compiler::ServerRuntimeArtifact,
) -> anyhow::Result<PathBuf> {
    let entry = normalized_artifact_path(&artifact.entry);
    if let Some(file) = artifact.source_bundle.files.iter().find(|file| {
        let path = normalized_artifact_path(&file.path);
        path == entry || path.ends_with(&entry)
    }) {
        return Ok(PathBuf::from(&file.path));
    }
    if artifact.source_bundle.files.len() == 1 {
        return Ok(PathBuf::from(&artifact.source_bundle.files[0].path));
    }
    anyhow::bail!("entry source `{}` not found in artifact", artifact.entry)
}

pub(crate) fn source_bundle_entry_path(
    artifact: &orv_compiler::SourceBundleArtifact,
) -> anyhow::Result<PathBuf> {
    let entry = normalized_artifact_path(&artifact.entry);
    if let Some(file) = artifact.files.iter().find(|file| {
        let path = normalized_artifact_path(&file.path);
        path == entry || path.ends_with(&entry)
    }) {
        return Ok(PathBuf::from(&file.path));
    }
    if artifact.files.len() == 1 {
        return Ok(PathBuf::from(&artifact.files[0].path));
    }
    anyhow::bail!(
        "entry source `{}` not found in source bundle",
        artifact.entry
    )
}

pub(crate) fn normalized_artifact_path(path: &str) -> String {
    path.replace('\\', "/")
}
