#![allow(clippy::redundant_pub_crate, clippy::wildcard_imports)]

use super::*;

pub(crate) fn dap_launch_runtime_state(
    lowered: &orv_analyzer::LowerResult,
    diagnostic_count: usize,
    files: &[SourceFile],
    sources: &[DapSourceInfo],
    live_requested: bool,
) -> (
    DapRuntimeState,
    Vec<DapFrameState>,
    Option<DapLiveState>,
    bool,
) {
    if diagnostic_count == 0 && dap_program_has_long_running_runtime(&lowered.program) {
        let (runtime, frames) = dap_long_running_runtime_state(&lowered.program, files, sources);
        return (runtime, frames, None, true);
    }
    if live_requested && diagnostic_count == 0 {
        let (runtime, frames, live) = dap_live_runtime_state(lowered, files, sources);
        return (runtime, frames, live, false);
    }
    let (runtime, frames) = dap_runtime_state(lowered, diagnostic_count, files, sources);
    (runtime, frames, None, false)
}

pub(crate) fn dap_source_info(file: &SourceFile, reference: u64) -> DapSourceInfo {
    let name = file
        .path
        .file_name()
        .and_then(std::ffi::OsStr::to_str)
        .unwrap_or("source.orv")
        .to_string();
    DapSourceInfo {
        reference,
        name,
        path: file.path.clone(),
        uri: lsp_file_uri_for_path(&file.path),
        checksum: sha256_hex(file.source.as_bytes()),
    }
}

pub(crate) fn dap_launch_live(request: &serde_json::Value) -> bool {
    request
        .pointer("/arguments/live")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
}

pub(crate) fn dap_launch_attach_runtime(request: &serde_json::Value) -> bool {
    request
        .pointer("/arguments/attachRuntime")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
}

pub(crate) fn dap_launch_attach_runtime_mode(
    request: &serde_json::Value,
) -> anyhow::Result<DapRuntimeAttachMode> {
    match request
        .pointer("/arguments/attachRuntimeMode")
        .and_then(serde_json::Value::as_str)
    {
        None | Some("process") => Ok(DapRuntimeAttachMode::Process),
        Some("inProcess" | "in-process") => Ok(DapRuntimeAttachMode::InProcess),
        Some(mode) => anyhow::bail!("unsupported attachRuntimeMode `{mode}`"),
    }
}

pub(crate) fn dap_launch_runtime_request_trace_path(
    request: &serde_json::Value,
) -> anyhow::Result<Option<PathBuf>> {
    request
        .pointer("/arguments/runtimeRequestTracePath")
        .or_else(|| request.pointer("/arguments/requestTracePath"))
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|path| !path.is_empty())
        .map(dap_path_from_protocol_string)
        .transpose()
}

pub(crate) fn dap_launch_executable_lines(entry_path: &Path, frames: &[DapFrameState]) -> Vec<u64> {
    let mut executable_lines = if frames.is_empty() {
        dap_verified_breakpoint_lines(entry_path).unwrap_or_else(|_| vec![1])
    } else {
        frames.iter().map(|frame| frame.line).collect::<Vec<_>>()
    };
    if executable_lines.is_empty() {
        executable_lines.push(1);
    }
    executable_lines.sort_unstable();
    executable_lines.dedup();
    executable_lines
}

pub(crate) fn dap_loaded_project_for_launch(
    request: &serde_json::Value,
    path: &Path,
) -> anyhow::Result<DapLaunchProject> {
    let Some(source_bundle_path) = dap_launch_source_bundle_path(request)? else {
        return Ok(DapLaunchProject {
            loaded: orv_project::load_project(path).map_err(|e| anyhow::anyhow!("{e}"))?,
            entry_path_for_lookup: path.to_path_buf(),
            source_bundle: None,
        });
    };
    let source_bundle = read_source_bundle_artifact(&source_bundle_path)?;
    let entry = source_bundle_entry_path(&source_bundle)?;
    let hash = stable_json_hash(&serde_json::to_value(&source_bundle)?)?;
    let source_bundle_meta = DapLaunchSourceBundle {
        path: source_bundle_path,
        entry: PathBuf::from(&source_bundle.entry),
        file_count: source_bundle.files.len(),
        hash,
    };
    let loaded = load_project_from_source_bundle_artifact(&source_bundle)?;
    Ok(DapLaunchProject {
        loaded,
        entry_path_for_lookup: entry,
        source_bundle: Some(source_bundle_meta),
    })
}

pub(crate) fn dap_launch_source_bundle_json(
    bundle: Option<&DapLaunchSourceBundle>,
) -> serde_json::Value {
    bundle.map_or(serde_json::Value::Null, |bundle| {
        serde_json::json!({
            "path": bundle.path.display().to_string(),
            "entry": bundle.entry.display().to_string(),
            "fileCount": bundle.file_count,
            "hash": bundle.hash,
        })
    })
}

pub(crate) fn dap_launch_source_bundle_path(
    request: &serde_json::Value,
) -> anyhow::Result<Option<PathBuf>> {
    request
        .pointer("/arguments/sourceBundle")
        .or_else(|| request.pointer("/arguments/source_bundle"))
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|path| !path.is_empty())
        .map(dap_path_from_protocol_string)
        .transpose()
}
