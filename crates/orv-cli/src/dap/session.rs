#![allow(clippy::redundant_pub_crate, clippy::wildcard_imports)]

use super::*;

#[derive(Default)]
pub(crate) struct DapSession {
    pub(crate) next_seq: u64,
    pub(crate) launched: Option<DapLaunchState>,
    pub(crate) breakpoints: HashMap<PathBuf, Vec<DapBreakpoint>>,
    pub(crate) function_breakpoints: Vec<DapFunctionBreakpoint>,
    pub(crate) instruction_breakpoints: Vec<DapInstructionBreakpoint>,
    pub(crate) data_breakpoints: Vec<DapDataBreakpoint>,
    pub(crate) exception_filters: Option<HashSet<String>>,
    pub(crate) pending_events: Vec<DapPendingEvent>,
}

pub(crate) struct DapLaunchState {
    pub(crate) path: PathBuf,
    pub(crate) uri: String,
    pub(crate) name: String,
    pub(crate) source_bundle: Option<DapLaunchSourceBundle>,
    pub(crate) program: orv_hir::HirProgram,
    pub(crate) node_count: usize,
    pub(crate) diagnostic_count: usize,
    pub(crate) stopped_line: u64,
    pub(crate) stopped_reason: String,
    pub(crate) executable_lines: Vec<u64>,
    pub(crate) runtime: DapRuntimeState,
    pub(crate) sources: Vec<DapSourceInfo>,
    pub(crate) files: Vec<SourceFile>,
    pub(crate) frames: Vec<DapFrameState>,
    pub(crate) current_frame_index: usize,
    pub(crate) live_requested: bool,
    pub(crate) live: Option<DapLiveState>,
    pub(crate) long_running: bool,
    pub(crate) attach_runtime_requested: bool,
    pub(crate) attach_runtime_mode: DapRuntimeAttachMode,
    pub(crate) runtime_request_trace_path: Option<PathBuf>,
    pub(crate) runtime_process: Option<DapRuntimeProcess>,
    pub(crate) attached_server: Option<orv_runtime::server::AttachedServer>,
    pub(crate) async_runtime: Option<DapAsyncRuntimeState>,
}

#[derive(Clone)]
pub(crate) struct DapLaunchSourceBundle {
    pub(crate) path: PathBuf,
    pub(crate) entry: PathBuf,
    pub(crate) file_count: usize,
    pub(crate) hash: String,
}

pub(crate) struct DapLaunchProject {
    pub(crate) loaded: orv_project::LoadedProject,
    pub(crate) entry_path_for_lookup: PathBuf,
    pub(crate) source_bundle: Option<DapLaunchSourceBundle>,
}

pub(crate) struct DapPendingEvent {
    pub(crate) event: String,
    pub(crate) body: serde_json::Value,
}

pub(crate) struct DapLiveState {
    pub(crate) stepper: orv_runtime::DebugStepper<Vec<u8>>,
}

pub(crate) struct DapRuntimeProcess {
    pub(crate) child: Child,
}

impl DapRuntimeProcess {
    fn pid(&self) -> u32 {
        self.child.id()
    }
}

impl Drop for DapRuntimeProcess {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Drop for DapLaunchState {
    fn drop(&mut self) {
        self.attached_server = None;
        self.runtime_process = None;
    }
}

impl DapLaunchState {
    fn ensure_runtime_process_running(&mut self) -> anyhow::Result<()> {
        if !self.attach_runtime_requested {
            return Ok(());
        }
        match self.attach_runtime_mode {
            DapRuntimeAttachMode::Process => self.ensure_child_runtime_process_running(),
            DapRuntimeAttachMode::InProcess => self.ensure_in_process_runtime_running(),
        }
    }

    fn ensure_child_runtime_process_running(&mut self) -> anyhow::Result<()> {
        if let Some(process) = self.runtime_process.as_mut() {
            if let Some(status) = process.child.try_wait()? {
                let pid = process.pid();
                self.runtime_process = None;
                self.set_transport_state("exited", Some(pid), None);
                anyhow::bail!("runtime process exited with {status}");
            }
            let pid = process.pid();
            dap_send_process_signal(pid, "CONT")?;
            self.set_transport_state("running", Some(pid), None);
            return Ok(());
        }

        let exe =
            std::env::current_exe().map_err(|e| anyhow::anyhow!("current_exe failed: {e}"))?;
        let child = ProcessCommand::new(&exe)
            .arg("run")
            .arg(&self.path)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| anyhow::anyhow!("failed to start runtime process: {e}"))?;
        let pid = child.id();
        self.runtime_process = Some(DapRuntimeProcess { child });
        self.set_transport_state("running", Some(pid), None);
        Ok(())
    }

    fn ensure_in_process_runtime_running(&mut self) -> anyhow::Result<()> {
        if let Some(server) = &self.attached_server {
            self.set_transport_state("running", None, Some(server.addr().to_string()));
            return Ok(());
        }
        let server = orv_runtime::server::spawn_attached_server(self.program.clone())
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        let address = server.addr().to_string();
        self.attached_server = Some(server);
        self.set_transport_state("running", None, Some(address));
        Ok(())
    }

    fn suspend_runtime_process(&mut self) -> anyhow::Result<()> {
        if !self.attach_runtime_requested {
            return Ok(());
        }
        match self.attach_runtime_mode {
            DapRuntimeAttachMode::Process => self.suspend_child_runtime_process(),
            DapRuntimeAttachMode::InProcess => {
                self.suspend_in_process_runtime();
                Ok(())
            }
        }
    }

    fn suspend_child_runtime_process(&mut self) -> anyhow::Result<()> {
        let Some(process) = self.runtime_process.as_mut() else {
            self.set_transport_state("detached", None, None);
            return Ok(());
        };
        if let Some(status) = process.child.try_wait()? {
            let pid = process.pid();
            self.runtime_process = None;
            self.set_transport_state("exited", Some(pid), None);
            anyhow::bail!("runtime process exited with {status}");
        }
        let pid = process.pid();
        dap_send_process_signal(pid, "STOP")?;
        self.set_transport_state("suspended", Some(pid), None);
        Ok(())
    }

    fn suspend_in_process_runtime(&mut self) {
        let address = self
            .attached_server
            .as_ref()
            .map(|server| server.addr().to_string());
        self.attached_server = None;
        self.set_transport_state("suspended", None, address);
    }

    fn set_transport_state(
        &mut self,
        state: &str,
        process_id: Option<u32>,
        address: Option<String>,
    ) {
        let Some(async_runtime) = self.async_runtime.as_mut() else {
            return;
        };
        let transport = async_runtime
            .transport
            .get_or_insert_with(DapAsyncTransportState::process_detached);
        transport.state = state.to_string();
        transport.process_id = process_id.map(u64::from);
        transport.address = address;
    }

    fn write_runtime_request_trace_file(&self) -> anyhow::Result<()> {
        let Some(path) = &self.runtime_request_trace_path else {
            return Ok(());
        };
        let frames = self.attached_server.as_ref().map_or_else(
            Vec::new,
            orv_runtime::server::AttachedServer::request_frames,
        );
        orv_runtime::server::write_request_trace_file(path, &frames)
            .map_err(|e| anyhow::anyhow!("{e}"))
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub(crate) enum DapRuntimeAttachMode {
    Process,
    InProcess,
}

impl DapRuntimeAttachMode {
    const fn protocol_name(self) -> &'static str {
        match self {
            Self::Process => "process",
            Self::InProcess => "inProcess",
        }
    }
}

pub(crate) enum DapLiveAdvance {
    Frame { index: usize, output: String },
    Skipped,
    Done,
    Error { message: String },
}

#[derive(Clone)]
pub(crate) struct DapSourceInfo {
    pub(crate) reference: u64,
    pub(crate) name: String,
    pub(crate) path: PathBuf,
    pub(crate) uri: String,
    pub(crate) checksum: String,
}

#[derive(Clone)]
pub(crate) struct DapBreakpoint {
    pub(crate) id: u64,
    pub(crate) line: u64,
    pub(crate) verified: bool,
    pub(crate) condition: Option<String>,
    pub(crate) hit_condition: Option<String>,
    pub(crate) log_message: Option<String>,
    pub(crate) message: Option<String>,
}

#[derive(Clone)]
pub(crate) struct DapFunctionBreakpoint {
    pub(crate) id: u64,
    pub(crate) name: String,
    pub(crate) verified: bool,
    pub(crate) message: Option<String>,
}

#[derive(Clone)]
pub(crate) struct DapDataBreakpoint {
    pub(crate) id: u64,
    pub(crate) data_id: String,
    pub(crate) verified: bool,
    pub(crate) message: Option<String>,
}

#[derive(Clone)]
pub(crate) struct DapInstructionBreakpoint {
    pub(crate) id: u64,
    pub(crate) instruction_reference: String,
    pub(crate) offset: i64,
    pub(crate) frame_index: Option<usize>,
    pub(crate) verified: bool,
    pub(crate) message: Option<String>,
}

#[derive(Clone)]
pub(crate) struct DapRuntimeState {
    pub(crate) status: String,
    pub(crate) stdout: String,
    pub(crate) error: String,
}

#[derive(Clone)]
pub(crate) struct DapAsyncRuntimeState {
    pub(crate) kind: String,
    pub(crate) state: String,
    pub(crate) resume_count: u64,
    pub(crate) pause_count: u64,
    pub(crate) listen: Option<DapAsyncListenState>,
    pub(crate) routes: Vec<DapAsyncRouteState>,
    pub(crate) transport: Option<DapAsyncTransportState>,
}

#[derive(Clone)]
pub(crate) struct DapAsyncRouteState {
    pub(crate) method: String,
    pub(crate) path: String,
}

#[derive(Clone)]
pub(crate) struct DapAsyncTransportState {
    pub(crate) kind: String,
    pub(crate) state: String,
    pub(crate) process_id: Option<u64>,
    pub(crate) address: Option<String>,
}

#[derive(Clone)]
pub(crate) struct DapAsyncListenState {
    pub(crate) kind: String,
    pub(crate) display: String,
    pub(crate) port: Option<u64>,
    pub(crate) variable: Option<String>,
    pub(crate) default_port: Option<u64>,
}

impl DapAsyncRuntimeState {
    pub(super) fn server(
        listen: Option<DapAsyncListenState>,
        routes: Vec<DapAsyncRouteState>,
    ) -> Self {
        Self {
            kind: "server".to_string(),
            state: "paused".to_string(),
            resume_count: 0,
            pause_count: 0,
            listen,
            routes,
            transport: None,
        }
    }
}

impl DapAsyncTransportState {
    pub(super) fn process_detached() -> Self {
        Self {
            kind: "process".to_string(),
            state: "detached".to_string(),
            process_id: None,
            address: None,
        }
    }

    pub(super) fn in_process_detached() -> Self {
        Self {
            kind: "in-process".to_string(),
            state: "detached".to_string(),
            process_id: None,
            address: None,
        }
    }
}

#[derive(Clone)]
pub(crate) struct DapVariable {
    pub(crate) name: String,
    pub(crate) value: String,
    pub(crate) value_type: String,
    pub(crate) line: u64,
    pub(crate) variables_reference: u64,
}

#[derive(Clone)]
pub(crate) struct DapFrameState {
    pub(crate) source: DapSourceInfo,
    pub(crate) line: u64,
    pub(crate) locals: Vec<DapVariable>,
    pub(crate) stack: Vec<DapStackFrameState>,
    pub(crate) output: String,
}

#[derive(Clone)]
pub(crate) struct DapStackFrameState {
    pub(crate) name: String,
    pub(crate) source: DapSourceInfo,
    pub(crate) line: u64,
}

impl DapSession {
    pub(crate) fn message_response(
        &mut self,
        request: &serde_json::Value,
    ) -> Option<serde_json::Value> {
        if request.get("type").and_then(serde_json::Value::as_str) != Some("request") {
            return None;
        }
        let seq = self.next_response_seq();
        let request_seq = request
            .get("seq")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0);
        let command = request
            .get("command")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("");
        let result = match command {
            "initialize" => {
                self.queue_event("initialized", serde_json::json!({}));
                Ok(serde_json::json!({
                    "supportsConfigurationDoneRequest": true,
                    "supportsTerminateRequest": true,
                    "supportsTerminateThreadsRequest": true,
                    "supportsLoadedSourcesRequest": true,
                    "supportsEvaluateForHovers": true,
                    "supportsCompletionsRequest": true,
                    "supportsBreakpointLocationsRequest": true,
                    "supportsConditionalBreakpoints": true,
                    "supportsHitConditionalBreakpoints": true,
                    "supportsFunctionBreakpoints": true,
                    "supportsDataBreakpoints": true,
                    "supportsExceptionInfoRequest": true,
                    "supportsRestartRequest": true,
                    "supportsSetVariable": true,
                    "supportsSetExpression": true,
                    "supportsModulesRequest": true,
                    "supportsGotoTargetsRequest": true,
                    "supportsStepBack": true,
                    "supportsStepInTargetsRequest": true,
                    "supportsRestartFrame": true,
                    "supportsPauseRequest": true,
                    "supportsCancelRequest": true,
                    "supportsInstructionBreakpoints": true,
                    "supportsDisassembleRequest": true,
                    "supportsReadMemoryRequest": true,
                    "supportsOrvRuntimeAttach": true,
                    "supportsOrvRuntimeTracePath": true,
                    "supportsOrvSourceBundleLaunch": true,
                    "exceptionBreakpointFilters": [
                        {
                            "filter": "orv.diagnostics",
                            "label": "ORV diagnostics",
                            "default": true,
                        },
                        {
                            "filter": "orv.runtime",
                            "label": "ORV runtime errors",
                            "default": true,
                        },
                    ],
                }))
            }
            "launch" => self.launch_result(request),
            "attach" => self.attach_result(request),
            "restart" => self.restart_result(request),
            "configurationDone" => self.configuration_done_result(),
            "cancel" => Ok(serde_json::json!({})),
            "setExceptionBreakpoints" => self.set_exception_breakpoints_result(request),
            "setBreakpoints" => self.set_breakpoints_result(request),
            "setFunctionBreakpoints" => self.set_function_breakpoints_result(request),
            "setInstructionBreakpoints" => self.set_instruction_breakpoints_result(request),
            "dataBreakpointInfo" => self.data_breakpoint_info_result(request),
            "setDataBreakpoints" => self.set_data_breakpoints_result(request),
            "breakpointLocations" => self.breakpoint_locations_result(request),
            "gotoTargets" => self.goto_targets_result(request),
            "threads" => Ok(serde_json::json!({
                "threads": [
                    {
                        "id": 1,
                        "name": "orv reference runtime",
                    },
                ],
            })),
            "stackTrace" => self.stack_trace_result(request),
            "scopes" => self.scopes_result(request),
            "variables" => self.variables_result(request),
            "setVariable" => self.set_variable_result(request),
            "evaluate" => self.evaluate_result(request),
            "setExpression" => self.set_expression_result(request),
            "completions" => self.completions_result(request),
            "exceptionInfo" => self.exception_info_result(request),
            "loadedSources" => self.loaded_sources_result(),
            "modules" => self.modules_result(request),
            "source" => self.source_result(request),
            "disassemble" => self.disassemble_result(request),
            "readMemory" => self.read_memory_result(request),
            "continue" => self.continue_result(request),
            "reverseContinue" => self.reverse_continue_result(request),
            "goto" => self.goto_result(request),
            "stepBack" => self.step_back_result(request),
            "restartFrame" => self.restart_frame_result(request),
            "next" => self.next_result(request),
            "stepInTargets" => self.step_in_targets_result(request),
            "stepIn" => self.step_in_result(request),
            "stepOut" => self.step_out_result(request),
            "pause" => self.pause_result(request),
            "terminateThreads" => self.terminate_threads_result(request),
            "disconnect" | "terminate" => {
                let flush = self
                    .launched
                    .as_ref()
                    .map_or_else(|| Ok(()), DapLaunchState::write_runtime_request_trace_file);
                flush.map(|()| {
                    self.queue_event("terminated", serde_json::json!({}));
                    self.launched = None;
                    serde_json::json!({})
                })
            }
            _ => Err(anyhow::anyhow!("unsupported DAP command `{command}`")),
        };
        Some(match result {
            Ok(body) => dap_success_response(seq, request_seq, command, &body),
            Err(err) => dap_error_response(seq, request_seq, command, &err.to_string()),
        })
    }

    const fn next_response_seq(&mut self) -> u64 {
        self.next_seq += 1;
        self.next_seq
    }

    fn require_reference_thread(request: &serde_json::Value, command: &str) -> anyhow::Result<()> {
        let thread_id = request
            .pointer("/arguments/threadId")
            .and_then(serde_json::Value::as_u64)
            .ok_or_else(|| anyhow::anyhow!("{command}.arguments.threadId is required"))?;
        if thread_id != 1 {
            anyhow::bail!("unknown ORV thread id {thread_id}");
        }
        Ok(())
    }

    fn launch_result(&mut self, request: &serde_json::Value) -> anyhow::Result<serde_json::Value> {
        let path = dap_program_path(request)?;
        let project = dap_loaded_project_for_launch(request, &path)?;
        let DapLaunchProject {
            loaded,
            entry_path_for_lookup,
            source_bundle,
        } = project;
        let file = lsp_source_file_for_path(&loaded.files, &entry_path_for_lookup)
            .or_else(|| lsp_source_file_for_path(&loaded.files, &path))
            .ok_or_else(|| anyhow::anyhow!("launch program is not part of loaded project"))?;
        let resolved = orv_resolve::resolve(&loaded.program);
        let lowered = orv_analyzer::lower_with_diagnostics(&loaded.program, &resolved);
        let diagnostic_count =
            loaded.diagnostics.len() + resolved.diagnostics.len() + lowered.diagnostics.len();
        let entry_path = file.path.clone();
        let entry_uri = lsp_file_uri_for_path(&entry_path);
        let entry_name = entry_path
            .file_name()
            .and_then(std::ffi::OsStr::to_str)
            .unwrap_or("app.orv")
            .to_string();
        let sources: Vec<DapSourceInfo> = loaded
            .files
            .iter()
            .enumerate()
            .map(|(index, file)| {
                dap_source_info(file, u64::try_from(index + 1).unwrap_or(u64::MAX))
            })
            .collect();
        let live_requested = dap_launch_live(request);
        let attach_runtime_requested = dap_launch_attach_runtime(request);
        let attach_runtime_mode = dap_launch_attach_runtime_mode(request)?;
        let runtime_request_trace_path = dap_launch_runtime_request_trace_path(request)?;
        let (runtime, mut frames, live, long_running) = dap_launch_runtime_state(
            &lowered,
            diagnostic_count,
            &loaded.files,
            &sources,
            live_requested,
        );
        let mut async_runtime = dap_async_runtime_state(&lowered.program, long_running);
        dap_attach_runtime_transport_if_requested(
            &mut async_runtime,
            attach_runtime_requested,
            attach_runtime_mode,
        );
        self.revalidate_instruction_breakpoints(frames.len());
        let executable_lines = dap_launch_executable_lines(&entry_path, &frames);
        let current_frame_index = self.first_verified_breakpoint_frame(&frames).unwrap_or(0);
        let stopped_line = frames
            .get(current_frame_index)
            .map_or(executable_lines[0], |frame| frame.line);
        let stopped_reason = self.launch_stopped_reason(&runtime, &frames, current_frame_index);
        let source_bundle_json = dap_launch_source_bundle_json(source_bundle.as_ref());
        self.launched = Some(DapLaunchState {
            path: entry_path.clone(),
            uri: entry_uri.clone(),
            name: entry_name.clone(),
            source_bundle,
            program: lowered.program,
            node_count: loaded.graph.nodes.len(),
            diagnostic_count,
            stopped_line,
            stopped_reason,
            executable_lines,
            runtime: runtime.clone(),
            sources,
            files: loaded.files.clone(),
            frames: std::mem::take(&mut frames),
            current_frame_index,
            live_requested,
            live,
            long_running,
            attach_runtime_requested,
            attach_runtime_mode,
            runtime_request_trace_path,
            runtime_process: None,
            attached_server: None,
            async_runtime: async_runtime.clone(),
        });
        if self
            .launched
            .as_ref()
            .is_some_and(|launched| !launched.frames.is_empty())
        {
            self.queue_frame_outputs(0, current_frame_index);
        } else if !runtime.stdout.is_empty() {
            self.queue_stdout_output(&runtime.stdout);
        }
        if !runtime.error.is_empty() {
            self.queue_event(
                "output",
                serde_json::json!({
                    "category": "stderr",
                    "output": runtime.error,
                }),
            );
        }
        Ok(serde_json::json!({
            "entry": {
                "name": entry_name,
                "path": entry_path.display().to_string(),
                "uri": entry_uri,
            },
            "projectGraphNodes": loaded.graph.nodes.len(),
            "sourceBundle": source_bundle_json,
            "diagnostics": diagnostic_count,
            "runtime": dap_runtime_json(&runtime, async_runtime.as_ref()),
        }))
    }

    fn attach_result(&mut self, request: &serde_json::Value) -> anyhow::Result<serde_json::Value> {
        let mut arguments = request
            .get("arguments")
            .cloned()
            .unwrap_or_else(|| serde_json::json!({}));
        let arguments_object = arguments
            .as_object_mut()
            .ok_or_else(|| anyhow::anyhow!("attach.arguments must be an object"))?;
        arguments_object.insert("attachRuntime".to_string(), serde_json::Value::Bool(true));
        self.launch_result(&serde_json::json!({
            "arguments": arguments,
        }))
    }

    fn launch_stopped_reason(
        &self,
        runtime: &DapRuntimeState,
        frames: &[DapFrameState],
        current_frame_index: usize,
    ) -> String {
        if self.exception_filter_enabled(runtime.status.as_str()) {
            "exception".to_string()
        } else if let Some(reason) = self.breakpoint_frame_reason(frames, current_frame_index) {
            reason.to_string()
        } else {
            "entry".to_string()
        }
    }

    fn set_exception_breakpoints_result(
        &mut self,
        request: &serde_json::Value,
    ) -> anyhow::Result<serde_json::Value> {
        let filters = request
            .pointer("/arguments/filters")
            .and_then(serde_json::Value::as_array)
            .map_or_else(HashSet::new, |filters| {
                filters
                    .iter()
                    .filter_map(serde_json::Value::as_str)
                    .filter(|filter| matches!(*filter, "orv.diagnostics" | "orv.runtime"))
                    .map(str::to_string)
                    .collect()
            });
        self.exception_filters = Some(filters);
        Ok(dap_set_exception_breakpoints_result(request))
    }

    fn exception_filter_enabled(&self, runtime_status: &str) -> bool {
        let filter = match runtime_status {
            "diagnostics" => "orv.diagnostics",
            "error" => "orv.runtime",
            _ => return false,
        };
        self.exception_filters
            .as_ref()
            .is_none_or(|filters| filters.contains(filter))
    }

    fn configuration_done_result(&mut self) -> anyhow::Result<serde_json::Value> {
        self.require_launch("configurationDone")?;
        self.queue_stopped_event();
        Ok(serde_json::json!({}))
    }

    fn restart_result(&mut self, request: &serde_json::Value) -> anyhow::Result<serde_json::Value> {
        let live_requested = request
            .pointer("/arguments/live")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or_else(|| {
                self.launched
                    .as_ref()
                    .is_some_and(|launched| launched.live_requested)
            });
        let attach_runtime_requested = request
            .pointer("/arguments/attachRuntime")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or_else(|| {
                self.launched
                    .as_ref()
                    .is_some_and(|launched| launched.attach_runtime_requested)
            });
        let attach_runtime_mode = if request.pointer("/arguments/attachRuntimeMode").is_some() {
            dap_launch_attach_runtime_mode(request)?
        } else {
            self.launched
                .as_ref()
                .map_or(DapRuntimeAttachMode::Process, |launched| {
                    launched.attach_runtime_mode
                })
        };
        let runtime_request_trace_path =
            dap_launch_runtime_request_trace_path(request)?.or_else(|| {
                self.launched
                    .as_ref()
                    .and_then(|launched| launched.runtime_request_trace_path.clone())
            });
        let path = request
            .pointer("/arguments/program")
            .and_then(serde_json::Value::as_str)
            .map(dap_path_from_protocol_string)
            .transpose()?
            .or_else(|| self.launched.as_ref().map(|launched| launched.path.clone()))
            .ok_or_else(|| anyhow::anyhow!("launch is required before restart"))?;
        let has_program_override = request.pointer("/arguments/program").is_some();
        let source_bundle_path = dap_launch_source_bundle_path(request)?.or_else(|| {
            if has_program_override {
                None
            } else {
                self.launched
                    .as_ref()
                    .and_then(|launched| launched.source_bundle.as_ref())
                    .map(|source_bundle| source_bundle.path.clone())
            }
        });
        let mut arguments = serde_json::json!({
                "program": path.display().to_string(),
                "live": live_requested,
                "attachRuntime": attach_runtime_requested,
                "attachRuntimeMode": attach_runtime_mode.protocol_name(),
        });
        if let Some(path) = source_bundle_path {
            arguments["sourceBundle"] = serde_json::json!(path.display().to_string());
        }
        if let Some(path) = runtime_request_trace_path {
            arguments["runtimeRequestTracePath"] = serde_json::json!(path.display().to_string());
        }
        let restart_request = serde_json::json!({
            "arguments": arguments,
        });
        self.launch_result(&restart_request)
    }

    fn loaded_sources_result(&self) -> anyhow::Result<serde_json::Value> {
        let launched = self
            .launched
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("launch is required before loadedSources"))?;
        Ok(serde_json::json!({
            "sources": launched
                .sources
                .iter()
                .map(dap_source_json)
                .collect::<Vec<_>>(),
        }))
    }

    fn modules_result(&self, request: &serde_json::Value) -> anyhow::Result<serde_json::Value> {
        let launched = self
            .launched
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("launch is required before modules"))?;
        let start = request
            .pointer("/arguments/startModule")
            .and_then(serde_json::Value::as_u64)
            .and_then(|value| usize::try_from(value).ok())
            .unwrap_or(0);
        let total = launched.sources.len();
        let available = total.saturating_sub(start);
        let module_count = request
            .pointer("/arguments/moduleCount")
            .and_then(serde_json::Value::as_u64)
            .and_then(|value| usize::try_from(value).ok())
            .unwrap_or(available);
        Ok(serde_json::json!({
            "modules": launched
                .sources
                .iter()
                .skip(start)
                .take(module_count)
                .map(dap_module_json)
                .collect::<Vec<_>>(),
            "totalModules": total,
        }))
    }

    fn source_result(&self, request: &serde_json::Value) -> anyhow::Result<serde_json::Value> {
        let launched = self
            .launched
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("launch is required before source"))?;
        let source = if let Some(reference) = dap_source_reference(request) {
            launched
                .sources
                .iter()
                .find(|source| source.reference == reference)
                .ok_or_else(|| anyhow::anyhow!("unknown sourceReference {reference}"))?
        } else {
            let requested_path = dap_normalize_path(&dap_source_path(request)?);
            launched
                .sources
                .iter()
                .find(|source| dap_normalize_path(&source.path) == requested_path)
                .ok_or_else(|| {
                    anyhow::anyhow!(
                        "source `{}` is not part of the launched project",
                        requested_path.display()
                    )
                })?
        };
        let content = launched
            .files
            .iter()
            .find(|file| dap_normalize_path(&file.path) == dap_normalize_path(&source.path))
            .map(|file| file.source.clone())
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "source `{}` is not part of the loaded project snapshot",
                    source.path.display()
                )
            })?;
        Ok(serde_json::json!({
            "content": content,
            "mimeType": "text/x-orv",
        }))
    }

    fn disassemble_result(&self, request: &serde_json::Value) -> anyhow::Result<serde_json::Value> {
        let launched = self
            .launched
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("launch is required before disassemble"))?;
        let memory_reference = request
            .pointer("/arguments/memoryReference")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| anyhow::anyhow!("disassemble.arguments.memoryReference is required"))?;
        let instruction_offset = request
            .pointer("/arguments/instructionOffset")
            .and_then(serde_json::Value::as_i64)
            .unwrap_or(0);
        let start = dap_disassemble_start_index(memory_reference, instruction_offset)?;
        let available = launched.frames.len().saturating_sub(start);
        let instruction_count = request
            .pointer("/arguments/instructionCount")
            .and_then(serde_json::Value::as_u64)
            .and_then(|value| usize::try_from(value).ok())
            .unwrap_or(available);
        Ok(serde_json::json!({
            "instructions": launched
                .frames
                .iter()
                .enumerate()
                .skip(start)
                .take(instruction_count)
                .map(|(index, frame)| dap_disassembled_instruction_json(index, frame))
                .collect::<Vec<_>>(),
        }))
    }

    fn read_memory_result(&self, request: &serde_json::Value) -> anyhow::Result<serde_json::Value> {
        let launched = self
            .launched
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("launch is required before readMemory"))?;
        let memory_reference = request
            .pointer("/arguments/memoryReference")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| anyhow::anyhow!("readMemory.arguments.memoryReference is required"))?;
        let frame_index = dap_memory_reference_frame_index(memory_reference, "readMemory")?;
        let frame = launched
            .frames
            .get(frame_index)
            .ok_or_else(|| anyhow::anyhow!("unknown ORV memoryReference `{memory_reference}`"))?;
        let offset = request
            .pointer("/arguments/offset")
            .and_then(serde_json::Value::as_i64)
            .unwrap_or(0);
        if offset < 0 {
            anyhow::bail!("readMemory.arguments.offset must be non-negative");
        }
        let offset = usize::try_from(offset).unwrap_or(usize::MAX);
        let count = request
            .pointer("/arguments/count")
            .and_then(serde_json::Value::as_u64)
            .and_then(|value| usize::try_from(value).ok())
            .ok_or_else(|| anyhow::anyhow!("readMemory.arguments.count is required"))?;
        let source = launched
            .files
            .iter()
            .find(|file| dap_normalize_path(&file.path) == dap_normalize_path(&frame.source.path))
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "source `{}` is not part of the loaded project snapshot",
                    frame.source.path.display()
                )
            })?;
        let line = source
            .source
            .lines()
            .nth(usize::try_from(frame.line.saturating_sub(1)).unwrap_or(usize::MAX))
            .ok_or_else(|| anyhow::anyhow!("frame line {} is outside source", frame.line))?;
        let bytes = line.as_bytes();
        let start = offset.min(bytes.len());
        let end = start.saturating_add(count).min(bytes.len());
        let data = &bytes[start..end];
        Ok(serde_json::json!({
            "address": memory_reference,
            "data": dap_base64_encode(data),
            "unreadableBytes": count.saturating_sub(data.len()),
        }))
    }

    fn set_breakpoints_result(
        &mut self,
        request: &serde_json::Value,
    ) -> anyhow::Result<serde_json::Value> {
        let path = dap_normalize_path(&dap_breakpoint_source_path(
            self.launched.as_ref(),
            request,
        )?);
        let verified_lines = dap_verified_breakpoint_lines(&path).unwrap_or_default();
        let breakpoints = request
            .pointer("/arguments/breakpoints")
            .and_then(serde_json::Value::as_array)
            .map_or_else(Vec::new, |items| {
                items
                    .iter()
                    .enumerate()
                    .map(|(index, breakpoint)| {
                        let line = breakpoint
                            .get("line")
                            .and_then(serde_json::Value::as_u64)
                            .unwrap_or(0);
                        let verified = line > 0 && verified_lines.binary_search(&line).is_ok();
                        DapBreakpoint {
                            id: u64::try_from(index + 1).unwrap_or(u64::MAX),
                            line,
                            verified,
                            condition: breakpoint
                                .get("condition")
                                .and_then(serde_json::Value::as_str)
                                .map(str::trim)
                                .filter(|condition| !condition.is_empty())
                                .map(str::to_string),
                            hit_condition: breakpoint
                                .get("hitCondition")
                                .and_then(serde_json::Value::as_str)
                                .map(str::trim)
                                .filter(|condition| !condition.is_empty())
                                .map(str::to_string),
                            log_message: breakpoint
                                .get("logMessage")
                                .and_then(serde_json::Value::as_str)
                                .map(str::trim)
                                .filter(|message| !message.is_empty())
                                .map(str::to_string),
                            message: (!verified)
                                .then(|| "no executable ORV node on this line".to_string()),
                        }
                    })
                    .collect()
            });
        self.breakpoints.insert(path, breakpoints.clone());
        let response_breakpoints = breakpoints
            .iter()
            .map(|breakpoint| {
                let mut value = serde_json::json!({
                    "id": breakpoint.id,
                    "verified": breakpoint.verified,
                    "line": breakpoint.line,
                });
                if let Some(message) = &breakpoint.message {
                    value["message"] = serde_json::Value::String(message.clone());
                }
                value
            })
            .collect::<Vec<_>>();
        Ok(serde_json::json!({
            "breakpoints": response_breakpoints,
        }))
    }

    fn breakpoint_locations_result(
        &self,
        request: &serde_json::Value,
    ) -> anyhow::Result<serde_json::Value> {
        let path = dap_breakpoint_source_path(self.launched.as_ref(), request)?;
        let loaded = orv_project::load_project(&path).map_err(|e| anyhow::anyhow!("{e}"))?;
        let file = lsp_source_file_for_path(&loaded.files, &path)
            .ok_or_else(|| anyhow::anyhow!("breakpoint source is not part of loaded project"))?;
        let line = request
            .pointer("/arguments/line")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(1);
        let end_line = request
            .pointer("/arguments/endLine")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(line);
        Ok(serde_json::json!({
            "breakpoints": dap_breakpoint_locations_json(
                &loaded.graph,
                &loaded.files,
                file.id,
                line,
                end_line,
            ),
        }))
    }

    fn set_function_breakpoints_result(
        &mut self,
        request: &serde_json::Value,
    ) -> anyhow::Result<serde_json::Value> {
        let breakpoints = request
            .pointer("/arguments/breakpoints")
            .and_then(serde_json::Value::as_array)
            .map_or_else(Vec::new, |items| {
                items
                    .iter()
                    .enumerate()
                    .map(|(index, breakpoint)| {
                        let name = breakpoint
                            .get("name")
                            .and_then(serde_json::Value::as_str)
                            .map(str::trim)
                            .unwrap_or("");
                        let verified = !name.is_empty();
                        DapFunctionBreakpoint {
                            id: u64::try_from(index + 1).unwrap_or(u64::MAX),
                            name: name.to_string(),
                            verified,
                            message: (!verified)
                                .then(|| "function breakpoint name must not be empty".to_string()),
                        }
                    })
                    .collect()
            });
        let response_breakpoints = breakpoints
            .iter()
            .map(|breakpoint| {
                let mut value = serde_json::json!({
                    "id": breakpoint.id,
                    "verified": breakpoint.verified,
                });
                if let Some(message) = &breakpoint.message {
                    value["message"] = serde_json::Value::String(message.clone());
                }
                value
            })
            .collect::<Vec<_>>();
        self.function_breakpoints = breakpoints;
        Ok(serde_json::json!({
            "breakpoints": response_breakpoints,
        }))
    }

    fn data_breakpoint_info_result(
        &self,
        request: &serde_json::Value,
    ) -> anyhow::Result<serde_json::Value> {
        let launched = self
            .launched
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("launch is required before dataBreakpointInfo"))?;
        let variables_reference = request
            .pointer("/arguments/variablesReference")
            .and_then(serde_json::Value::as_u64)
            .ok_or_else(|| {
                anyhow::anyhow!("dataBreakpointInfo.arguments.variablesReference is required")
            })?;
        let name = request
            .pointer("/arguments/name")
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .ok_or_else(|| anyhow::anyhow!("dataBreakpointInfo.arguments.name is required"))?;
        if variables_reference != 2
            || !dap_current_locals(launched)
                .iter()
                .any(|local| local.name == name)
        {
            return Ok(serde_json::json!({
                "dataId": null,
                "description": format!("no ORV local data breakpoint for {name}"),
                "accessTypes": [],
                "canPersist": false,
            }));
        }
        Ok(serde_json::json!({
            "dataId": format!("local:{name}"),
            "description": format!("local {name}"),
            "accessTypes": ["write", "readWrite"],
            "canPersist": true,
        }))
    }

    fn set_data_breakpoints_result(
        &mut self,
        request: &serde_json::Value,
    ) -> anyhow::Result<serde_json::Value> {
        let breakpoints = request
            .pointer("/arguments/breakpoints")
            .and_then(serde_json::Value::as_array)
            .map_or_else(Vec::new, |items| {
                items
                    .iter()
                    .enumerate()
                    .map(|(index, breakpoint)| {
                        let data_id = breakpoint
                            .get("dataId")
                            .and_then(serde_json::Value::as_str)
                            .map(str::trim)
                            .unwrap_or("");
                        let verified = dap_data_breakpoint_local_name(data_id).is_some();
                        DapDataBreakpoint {
                            id: u64::try_from(index + 1).unwrap_or(u64::MAX),
                            data_id: data_id.to_string(),
                            verified,
                            message: (!verified)
                                .then(|| "unsupported ORV data breakpoint".to_string()),
                        }
                    })
                    .collect()
            });
        let response_breakpoints = breakpoints
            .iter()
            .map(|breakpoint| {
                let mut value = serde_json::json!({
                    "id": breakpoint.id,
                    "verified": breakpoint.verified,
                });
                if let Some(message) = &breakpoint.message {
                    value["message"] = serde_json::Value::String(message.clone());
                }
                value
            })
            .collect::<Vec<_>>();
        self.data_breakpoints = breakpoints;
        Ok(serde_json::json!({
            "breakpoints": response_breakpoints,
        }))
    }

    fn set_instruction_breakpoints_result(
        &mut self,
        request: &serde_json::Value,
    ) -> anyhow::Result<serde_json::Value> {
        let frame_count = self.launched.as_ref().map(|launched| launched.frames.len());
        let breakpoints = request
            .pointer("/arguments/breakpoints")
            .and_then(serde_json::Value::as_array)
            .map_or_else(Vec::new, |items| {
                items
                    .iter()
                    .enumerate()
                    .map(|(index, breakpoint)| {
                        let instruction_reference = breakpoint
                            .get("instructionReference")
                            .and_then(serde_json::Value::as_str)
                            .map_or("", str::trim)
                            .to_string();
                        let offset = breakpoint
                            .get("offset")
                            .and_then(serde_json::Value::as_i64)
                            .unwrap_or(0);
                        dap_instruction_breakpoint(
                            u64::try_from(index + 1).unwrap_or(u64::MAX),
                            instruction_reference,
                            offset,
                            frame_count,
                        )
                    })
                    .collect()
            });
        let response_breakpoints = breakpoints
            .iter()
            .map(dap_instruction_breakpoint_json)
            .collect::<Vec<_>>();
        self.instruction_breakpoints = breakpoints;
        Ok(serde_json::json!({
            "breakpoints": response_breakpoints,
        }))
    }

    fn goto_targets_result(
        &self,
        request: &serde_json::Value,
    ) -> anyhow::Result<serde_json::Value> {
        let launched = self
            .launched
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("launch is required before gotoTargets"))?;
        let path = dap_breakpoint_source_path(Some(launched), request)?;
        let normalized = dap_normalize_path(&path);
        let source = launched
            .sources
            .iter()
            .find(|source| dap_normalize_path(&source.path) == normalized)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "source `{}` is not part of the launched project",
                    path.display()
                )
            })?;
        let line = request
            .pointer("/arguments/line")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(1);
        let end_line = request
            .pointer("/arguments/endLine")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(line);
        let verified_lines = dap_verified_breakpoint_lines(&path).unwrap_or_default();
        Ok(serde_json::json!({
            "targets": verified_lines
                .into_iter()
                .filter(|target_line| *target_line >= line && *target_line <= end_line)
                .map(|target_line| dap_goto_target_json(source, target_line))
                .collect::<Vec<_>>(),
        }))
    }

    fn stack_trace_result(&self, request: &serde_json::Value) -> anyhow::Result<serde_json::Value> {
        Self::require_reference_thread(request, "stackTrace")?;
        let launched = self
            .launched
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("launch is required before stackTrace"))?;
        let frames = dap_stack_frames_json(launched);
        let total_frames = frames.len();
        let frames = dap_paginate_json_values(frames, request, "startFrame", "levels");
        Ok(serde_json::json!({
            "stackFrames": frames,
            "totalFrames": total_frames,
        }))
    }

    fn scopes_result(&self, request: &serde_json::Value) -> anyhow::Result<serde_json::Value> {
        let launched = self
            .launched
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("launch is required before scopes"))?;
        let frame_id = request
            .pointer("/arguments/frameId")
            .and_then(serde_json::Value::as_u64)
            .ok_or_else(|| anyhow::anyhow!("scopes.arguments.frameId is required"))?;
        if frame_id != 1 {
            return dap_non_current_scopes_result(launched, frame_id);
        }
        let (source, _) = dap_current_source_and_line(launched);
        let project_variable_count = dap_project_variables(launched).len();
        let local_variable_count = dap_current_locals(launched).len();
        let scope_source = dap_source_json_with_reference(&source, 0);
        Ok(serde_json::json!({
            "scopes": [
                {
                    "name": "Project",
                    "variablesReference": 1,
                    "namedVariables": project_variable_count,
                    "expensive": false,
                    "source": scope_source,
                },
                {
                    "name": "Locals",
                    "variablesReference": 2,
                    "namedVariables": local_variable_count,
                    "expensive": false,
                    "source": scope_source,
                },
            ],
        }))
    }

    fn variables_result(&self, request: &serde_json::Value) -> anyhow::Result<serde_json::Value> {
        let launched = self
            .launched
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("launch is required before variables"))?;
        let variables_reference = request
            .pointer("/arguments/variablesReference")
            .and_then(serde_json::Value::as_u64)
            .ok_or_else(|| anyhow::anyhow!("variables.arguments.variablesReference is required"))?;
        if variables_reference == 2 {
            let variables = dap_current_locals(launched)
                .iter()
                .map(dap_variable_json)
                .collect::<Vec<_>>();
            return Ok(serde_json::json!({
                "variables": dap_filter_and_paginate_variables(variables, request),
            }));
        }
        if variables_reference != 1 {
            anyhow::bail!("unknown variablesReference {variables_reference}");
        }
        let variables = dap_project_variables(launched);
        Ok(serde_json::json!({
            "variables": dap_filter_and_paginate_variables(variables, request),
        }))
    }

    fn evaluate_result(&self, request: &serde_json::Value) -> anyhow::Result<serde_json::Value> {
        let launched = self
            .launched
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("launch is required before evaluate"))?;
        let expression = request
            .pointer("/arguments/expression")
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|expression| !expression.is_empty())
            .ok_or_else(|| anyhow::anyhow!("evaluate.arguments.expression is required"))?;
        let (result, value_type) = dap_evaluate_project_value(launched, expression)
            .ok_or_else(|| anyhow::anyhow!("unknown evaluate expression `{expression}`"))?;
        Ok(serde_json::json!({
            "result": result,
            "type": value_type,
            "variablesReference": 0,
        }))
    }

    fn set_variable_result(
        &mut self,
        request: &serde_json::Value,
    ) -> anyhow::Result<serde_json::Value> {
        let variables_reference = request
            .pointer("/arguments/variablesReference")
            .and_then(serde_json::Value::as_u64)
            .ok_or_else(|| {
                anyhow::anyhow!("setVariable.arguments.variablesReference is required")
            })?;
        if variables_reference != 2 {
            anyhow::bail!("setVariable currently supports only Locals variablesReference");
        }
        let name = request
            .pointer("/arguments/name")
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .ok_or_else(|| anyhow::anyhow!("setVariable.arguments.name is required"))?;
        let value = request
            .pointer("/arguments/value")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| anyhow::anyhow!("setVariable.arguments.value is required"))?;
        let variable = self.set_current_local_value(name, value)?;
        Ok(dap_set_value_json(&variable))
    }

    fn set_expression_result(
        &mut self,
        request: &serde_json::Value,
    ) -> anyhow::Result<serde_json::Value> {
        let expression = request
            .pointer("/arguments/expression")
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|expression| !expression.is_empty())
            .ok_or_else(|| anyhow::anyhow!("setExpression.arguments.expression is required"))?;
        let value = request
            .pointer("/arguments/value")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| anyhow::anyhow!("setExpression.arguments.value is required"))?;
        let variable = self.set_current_local_value(expression, value)?;
        Ok(dap_set_value_json(&variable))
    }

    fn completions_result(&self, request: &serde_json::Value) -> anyhow::Result<serde_json::Value> {
        let launched = self
            .launched
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("launch is required before completions"))?;
        let prefix = request
            .pointer("/arguments/text")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("");
        Ok(serde_json::json!({
            "targets": dap_completion_targets_json(launched, prefix),
        }))
    }

    fn exception_info_result(
        &self,
        request: &serde_json::Value,
    ) -> anyhow::Result<serde_json::Value> {
        Self::require_reference_thread(request, "exceptionInfo")?;
        let launched = self
            .launched
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("launch is required before exceptionInfo"))?;
        Ok(dap_exception_info_json(&launched.runtime))
    }

    fn continue_result(
        &mut self,
        request: &serde_json::Value,
    ) -> anyhow::Result<serde_json::Value> {
        Self::require_reference_thread(request, "continue")?;
        if self.launch_is_long_running() {
            return self.continue_long_running_result();
        }
        if self.launch_is_live() {
            return self.continue_live_result();
        }
        let (next_breakpoint, start_frame, has_frames) = {
            let launched = self
                .launched
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("launch is required before continue"))?;
            (
                self.next_verified_breakpoint_frame(launched),
                launched.current_frame_index.saturating_add(1),
                !launched.frames.is_empty(),
            )
        };
        self.queue_event(
            "continued",
            serde_json::json!({
                "threadId": 1,
                "allThreadsContinued": false,
            }),
        );
        if let Some(index) = next_breakpoint {
            self.queue_frame_outputs(start_frame, index);
            let stopped = self.launched.as_ref().and_then(|launched| {
                launched.frames.get(index).map(|frame| {
                    (
                        frame.line,
                        self.breakpoint_frame_reason(&launched.frames, index)
                            .unwrap_or("breakpoint"),
                    )
                })
            });
            let launched = self
                .launched
                .as_mut()
                .ok_or_else(|| anyhow::anyhow!("launch is required before continue"))?;
            if let Some((line, reason)) = stopped {
                launched.stopped_line = line;
                launched.stopped_reason = reason.to_string();
            }
            launched.current_frame_index = index;
            self.queue_stopped_event();
            return Ok(serde_json::json!({
                "allThreadsContinued": false,
            }));
        }
        if has_frames {
            let end_frame = self
                .launched
                .as_ref()
                .and_then(|launched| launched.frames.len().checked_sub(1))
                .unwrap_or(0);
            self.queue_frame_outputs(start_frame, end_frame);
        }
        self.queue_event("terminated", serde_json::json!({}));
        self.launched = None;
        Ok(serde_json::json!({
            "allThreadsContinued": false,
        }))
    }

    fn reverse_continue_result(
        &mut self,
        request: &serde_json::Value,
    ) -> anyhow::Result<serde_json::Value> {
        Self::require_reference_thread(request, "reverseContinue")?;
        let target_frame = {
            let launched = self
                .launched
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("launch is required before reverseContinue"))?;
            self.previous_verified_breakpoint_frame(launched)
                .or_else(|| (launched.current_frame_index > 0).then_some(0))
        };
        let Some(target_frame) = target_frame else {
            anyhow::bail!("no previous runtime frame");
        };
        self.queue_event(
            "continued",
            serde_json::json!({
                "threadId": 1,
                "allThreadsContinued": false,
            }),
        );
        let stopped_reason = {
            let launched = self
                .launched
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("launch is required before reverseContinue"))?;
            launched
                .frames
                .get(target_frame)
                .and_then(|_| self.breakpoint_frame_reason(&launched.frames, target_frame))
                .unwrap_or("entry")
        };
        let launched = self
            .launched
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("launch is required before reverseContinue"))?;
        launched.current_frame_index = target_frame;
        if let Some(frame) = launched.frames.get(target_frame) {
            launched.stopped_line = frame.line;
        }
        launched.stopped_reason = stopped_reason.to_string();
        self.queue_stopped_event();
        Ok(serde_json::json!({
            "allThreadsContinued": false,
        }))
    }

    fn goto_result(&mut self, request: &serde_json::Value) -> anyhow::Result<serde_json::Value> {
        Self::require_reference_thread(request, "goto")?;
        let target_id = request
            .pointer("/arguments/targetId")
            .and_then(serde_json::Value::as_u64)
            .ok_or_else(|| anyhow::anyhow!("goto.arguments.targetId is required"))?;
        let target_frame = {
            let launched = self
                .launched
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("launch is required before goto"))?;
            launched
                .frames
                .iter()
                .enumerate()
                .find_map(|(index, frame)| {
                    (dap_goto_target_id(frame.source.reference, frame.line) == target_id)
                        .then_some(index)
                })
        };
        let Some(target_frame) = target_frame else {
            anyhow::bail!("unknown goto targetId {target_id}");
        };
        let launched = self
            .launched
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("launch is required before goto"))?;
        let line = launched.frames[target_frame].line;
        launched.current_frame_index = target_frame;
        launched.stopped_line = line;
        launched.stopped_reason = "goto".to_string();
        self.queue_stopped_event();
        Ok(serde_json::json!({}))
    }

    fn step_back_result(
        &mut self,
        request: &serde_json::Value,
    ) -> anyhow::Result<serde_json::Value> {
        Self::require_reference_thread(request, "stepBack")?;
        let target_frame = {
            let launched = self
                .launched
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("launch is required before stepBack"))?;
            (launched.current_frame_index > 0).then_some(launched.current_frame_index - 1)
        };
        let Some(target_frame) = target_frame else {
            anyhow::bail!("no previous runtime frame");
        };
        let launched = self
            .launched
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("launch is required before stepBack"))?;
        launched.current_frame_index = target_frame;
        if let Some(frame) = launched.frames.get(target_frame) {
            launched.stopped_line = frame.line;
        }
        launched.stopped_reason = "step".to_string();
        self.queue_stopped_event();
        Ok(serde_json::json!({}))
    }

    fn restart_frame_result(
        &mut self,
        request: &serde_json::Value,
    ) -> anyhow::Result<serde_json::Value> {
        let frame_id = request
            .pointer("/arguments/frameId")
            .and_then(serde_json::Value::as_u64)
            .ok_or_else(|| anyhow::anyhow!("restartFrame.arguments.frameId is required"))?;
        let target_frame = {
            let launched = self
                .launched
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("launch is required before restartFrame"))?;
            dap_restart_frame_target_index(launched, frame_id)
                .ok_or_else(|| anyhow::anyhow!("no restartable runtime frame"))?
        };
        let launched = self
            .launched
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("launch is required before restartFrame"))?;
        launched.current_frame_index = target_frame;
        if let Some(frame) = launched.frames.get(target_frame) {
            launched.stopped_line = frame.line;
        }
        launched.stopped_reason = "restart".to_string();
        self.queue_stopped_event();
        Ok(serde_json::json!({}))
    }

    fn next_result(&mut self, request: &serde_json::Value) -> anyhow::Result<serde_json::Value> {
        Self::require_reference_thread(request, "next")?;
        if self.launch_is_live() {
            return self.next_live_result();
        }
        let (start_frame, target_frame) = {
            let launched = self
                .launched
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("launch is required before next"))?;
            let current = launched
                .frames
                .get(launched.current_frame_index)
                .ok_or_else(|| anyhow::anyhow!("no current runtime frame"))?;
            let current_depth = current.stack.len();
            let start = launched.current_frame_index.saturating_add(1);
            let target = launched
                .frames
                .iter()
                .enumerate()
                .skip(start)
                .find_map(|(index, frame)| (frame.stack.len() <= current_depth).then_some(index));
            (start, target)
        };
        let Some(target_frame) = target_frame else {
            self.launched = None;
            self.queue_event("terminated", serde_json::json!({}));
            return Ok(serde_json::json!({}));
        };
        self.queue_frame_outputs(start_frame, target_frame);
        let launched = self
            .launched
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("launch is required before next"))?;
        launched.current_frame_index = target_frame;
        if let Some(frame) = launched.frames.get(target_frame) {
            launched.stopped_line = frame.line;
        }
        launched.stopped_reason = "step".to_string();
        self.queue_stopped_event();
        Ok(serde_json::json!({}))
    }

    fn step_out_result(
        &mut self,
        request: &serde_json::Value,
    ) -> anyhow::Result<serde_json::Value> {
        Self::require_reference_thread(request, "stepOut")?;
        if self.launch_is_live() {
            return self.step_out_live_result();
        }
        let (start_frame, target_frame) = {
            let launched = self
                .launched
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("launch is required before stepOut"))?;
            let current = launched
                .frames
                .get(launched.current_frame_index)
                .ok_or_else(|| anyhow::anyhow!("no current runtime frame"))?;
            let current_depth = current.stack.len();
            if current_depth == 0 {
                anyhow::bail!("no caller frame");
            }
            let start = launched.current_frame_index.saturating_add(1);
            let target = launched
                .frames
                .iter()
                .enumerate()
                .skip(start)
                .find_map(|(index, frame)| (frame.stack.len() < current_depth).then_some(index));
            (start, target)
        };
        let Some(target_frame) = target_frame else {
            self.launched = None;
            self.queue_event("terminated", serde_json::json!({}));
            return Ok(serde_json::json!({}));
        };
        self.queue_frame_outputs(start_frame, target_frame);
        let launched = self
            .launched
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("launch is required before stepOut"))?;
        launched.current_frame_index = target_frame;
        if let Some(frame) = launched.frames.get(target_frame) {
            launched.stopped_line = frame.line;
        }
        launched.stopped_reason = "step".to_string();
        self.queue_stopped_event();
        Ok(serde_json::json!({}))
    }

    fn step_in_targets_result(
        &self,
        request: &serde_json::Value,
    ) -> anyhow::Result<serde_json::Value> {
        let frame_id = request
            .pointer("/arguments/frameId")
            .and_then(serde_json::Value::as_u64)
            .ok_or_else(|| anyhow::anyhow!("stepInTargets.arguments.frameId is required"))?;
        let launched = self
            .launched
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("launch is required before stepInTargets"))?;
        if frame_id != 1 {
            if dap_stack_scope_frame(launched, frame_id).is_none() {
                anyhow::bail!("unknown ORV frameId {frame_id}");
            }
            return Ok(serde_json::json!({
                "targets": [],
            }));
        }
        Ok(serde_json::json!({
            "targets": dap_step_in_targets_json(launched),
        }))
    }

    fn step_in_result(&mut self, request: &serde_json::Value) -> anyhow::Result<serde_json::Value> {
        Self::require_reference_thread(request, "stepIn")?;
        if self.launch_is_live() {
            if request
                .pointer("/arguments/targetId")
                .and_then(serde_json::Value::as_u64)
                .is_some()
            {
                anyhow::bail!("stepIn targetId is unavailable in live debug mode");
            }
            return self.step_in_live_result();
        }
        if let Some(target_id) = request
            .pointer("/arguments/targetId")
            .and_then(serde_json::Value::as_u64)
        {
            let (start_frame, target_frame) = {
                let launched = self
                    .launched
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("launch is required before stepIn"))?;
                let target_frame = dap_step_in_target_indices(launched)
                    .into_iter()
                    .find(|index| dap_step_in_target_id(*index) == target_id)
                    .ok_or_else(|| anyhow::anyhow!("unknown stepIn targetId {target_id}"))?;
                (launched.current_frame_index.saturating_add(1), target_frame)
            };
            self.queue_frame_outputs(start_frame, target_frame);
            let launched = self
                .launched
                .as_mut()
                .ok_or_else(|| anyhow::anyhow!("launch is required before stepIn"))?;
            launched.current_frame_index = target_frame;
            if let Some(frame) = launched.frames.get(target_frame) {
                launched.stopped_line = frame.line;
            }
            launched.stopped_reason = "step".to_string();
            self.queue_stopped_event();
            return Ok(serde_json::json!({}));
        }
        let next_frame = {
            let launched = self
                .launched
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("launch is required before debug control"))?;
            (!launched.frames.is_empty()).then_some(launched.current_frame_index + 1)
        };
        if let Some(next_frame) = next_frame {
            let launched = self
                .launched
                .as_mut()
                .ok_or_else(|| anyhow::anyhow!("launch is required before debug control"))?;
            let Some(frame) = launched.frames.get(next_frame) else {
                self.launched = None;
                self.queue_event("terminated", serde_json::json!({}));
                return Ok(serde_json::json!({}));
            };
            launched.current_frame_index = next_frame;
            launched.stopped_line = frame.line;
            launched.stopped_reason = "step".to_string();
            self.queue_current_frame_output();
            self.queue_stopped_event();
            return Ok(serde_json::json!({}));
        }
        let next_line = {
            let launched = self
                .launched
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("launch is required before debug control"))?;
            dap_following_executable_line(&launched.executable_lines, launched.stopped_line)
        };
        let Some(next_line) = next_line else {
            self.launched = None;
            self.queue_event("terminated", serde_json::json!({}));
            return Ok(serde_json::json!({}));
        };
        let launched = self
            .launched
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("launch is required before debug control"))?;
        launched.stopped_line = next_line;
        launched.stopped_reason = "step".to_string();
        self.queue_stopped_event();
        Ok(serde_json::json!({}))
    }

    fn continue_live_result(&mut self) -> anyhow::Result<serde_json::Value> {
        self.queue_event(
            "continued",
            serde_json::json!({
                "threadId": 1,
                "allThreadsContinued": false,
            }),
        );
        loop {
            match self.advance_live_frame()? {
                DapLiveAdvance::Frame { index, output } => {
                    self.queue_stdout_output(&output);
                    let stopped = self.launched.as_ref().and_then(|launched| {
                        launched.frames.get(index).and_then(|frame| {
                            self.breakpoint_frame_reason(&launched.frames, index)
                                .map(|reason| (frame.line, reason.to_string()))
                        })
                    });
                    if let Some((line, reason)) = stopped {
                        let launched = self
                            .launched
                            .as_mut()
                            .ok_or_else(|| anyhow::anyhow!("launch is required before continue"))?;
                        launched.current_frame_index = index;
                        launched.stopped_line = line;
                        launched.stopped_reason = reason;
                        self.queue_stopped_event();
                        return Ok(serde_json::json!({
                            "allThreadsContinued": false,
                        }));
                    }
                }
                DapLiveAdvance::Skipped => {}
                DapLiveAdvance::Done => {
                    self.queue_event("terminated", serde_json::json!({}));
                    self.launched = None;
                    return Ok(serde_json::json!({
                        "allThreadsContinued": false,
                    }));
                }
                DapLiveAdvance::Error { message } => {
                    self.queue_event(
                        "output",
                        serde_json::json!({
                            "category": "stderr",
                            "output": message,
                        }),
                    );
                    if let Some(launched) = self.launched.as_mut() {
                        launched.stopped_reason = "exception".to_string();
                    }
                    self.queue_stopped_event();
                    return Ok(serde_json::json!({
                        "allThreadsContinued": false,
                    }));
                }
            }
        }
    }

    fn continue_long_running_result(&mut self) -> anyhow::Result<serde_json::Value> {
        let launched = self
            .launched
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("launch is required before continue"))?;
        launched.ensure_runtime_process_running()?;
        launched.runtime.status = "running".to_string();
        if let Some(async_runtime) = launched.async_runtime.as_mut() {
            if async_runtime.state != "running" {
                async_runtime.resume_count = async_runtime.resume_count.saturating_add(1);
            }
            async_runtime.state = "running".to_string();
        }
        self.queue_event(
            "continued",
            serde_json::json!({
                "threadId": 1,
                "allThreadsContinued": false,
            }),
        );
        Ok(serde_json::json!({
            "allThreadsContinued": false,
        }))
    }

    fn next_live_result(&mut self) -> anyhow::Result<serde_json::Value> {
        let current_depth = self
            .launched
            .as_ref()
            .and_then(|launched| launched.frames.get(launched.current_frame_index))
            .map(|frame| frame.stack.len())
            .ok_or_else(|| anyhow::anyhow!("no current runtime frame"))?;
        self.advance_live_until(|frame| frame.stack.len() <= current_depth, "step")
    }

    fn step_in_live_result(&mut self) -> anyhow::Result<serde_json::Value> {
        self.advance_live_until(|_| true, "step")
    }

    fn step_out_live_result(&mut self) -> anyhow::Result<serde_json::Value> {
        let current_depth = self
            .launched
            .as_ref()
            .and_then(|launched| launched.frames.get(launched.current_frame_index))
            .map(|frame| frame.stack.len())
            .ok_or_else(|| anyhow::anyhow!("no current runtime frame"))?;
        if current_depth == 0 {
            anyhow::bail!("no caller frame");
        }
        self.advance_live_until(|frame| frame.stack.len() < current_depth, "step")
    }

    fn advance_live_until(
        &mut self,
        mut is_target: impl FnMut(&DapFrameState) -> bool,
        stopped_reason: &str,
    ) -> anyhow::Result<serde_json::Value> {
        loop {
            match self.advance_live_frame()? {
                DapLiveAdvance::Frame { index, output } => {
                    self.queue_stdout_output(&output);
                    let target = self
                        .launched
                        .as_ref()
                        .and_then(|launched| launched.frames.get(index))
                        .is_some_and(&mut is_target);
                    if target {
                        let launched = self.launched.as_mut().ok_or_else(|| {
                            anyhow::anyhow!("launch is required before debug control")
                        })?;
                        launched.current_frame_index = index;
                        if let Some(frame) = launched.frames.get(index) {
                            launched.stopped_line = frame.line;
                        }
                        launched.stopped_reason = stopped_reason.to_string();
                        self.queue_stopped_event();
                        return Ok(serde_json::json!({}));
                    }
                }
                DapLiveAdvance::Skipped => {}
                DapLiveAdvance::Done => {
                    self.launched = None;
                    self.queue_event("terminated", serde_json::json!({}));
                    return Ok(serde_json::json!({}));
                }
                DapLiveAdvance::Error { message } => {
                    self.queue_event(
                        "output",
                        serde_json::json!({
                            "category": "stderr",
                            "output": message,
                        }),
                    );
                    if let Some(launched) = self.launched.as_mut() {
                        launched.stopped_reason = "exception".to_string();
                    }
                    self.queue_stopped_event();
                    return Ok(serde_json::json!({}));
                }
            }
        }
    }

    fn advance_live_frame(&mut self) -> anyhow::Result<DapLiveAdvance> {
        let step = {
            let launched = self
                .launched
                .as_mut()
                .ok_or_else(|| anyhow::anyhow!("launch is required before debug control"))?;
            let live = launched
                .live
                .as_mut()
                .ok_or_else(|| anyhow::anyhow!("launch is not in live debug mode"))?;
            live.stepper.step()
        };
        match step {
            Ok(Some(debug_frame)) => {
                let launched = self
                    .launched
                    .as_mut()
                    .ok_or_else(|| anyhow::anyhow!("launch is required before debug control"))?;
                let frames = dap_runtime_frames(&[debug_frame], &launched.files, &launched.sources);
                let Some(frame) = frames.into_iter().next() else {
                    return Ok(DapLiveAdvance::Skipped);
                };
                let output = frame.output.clone();
                launched.runtime.stdout.push_str(&output);
                launched.frames.push(frame);
                Ok(DapLiveAdvance::Frame {
                    index: launched.frames.len().saturating_sub(1),
                    output,
                })
            }
            Ok(None) => {
                if let Some(launched) = self.launched.as_mut() {
                    launched.runtime.status = "ok".to_string();
                    launched.live = None;
                }
                Ok(DapLiveAdvance::Done)
            }
            Err(err) => {
                let message = err.to_string();
                if let Some(launched) = self.launched.as_mut() {
                    launched.runtime.status = "error".to_string();
                    launched.runtime.error.clone_from(&message);
                    launched.live = None;
                }
                Ok(DapLiveAdvance::Error { message })
            }
        }
    }

    fn launch_is_live(&self) -> bool {
        self.launched
            .as_ref()
            .is_some_and(|launched| launched.live.is_some())
    }

    fn launch_is_long_running(&self) -> bool {
        self.launched
            .as_ref()
            .is_some_and(|launched| launched.long_running)
    }

    fn pause_result(&mut self, request: &serde_json::Value) -> anyhow::Result<serde_json::Value> {
        Self::require_reference_thread(request, "pause")?;
        let launched = self
            .launched
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("launch is required before debug control"))?;
        if launched.long_running {
            launched.write_runtime_request_trace_file()?;
            launched.suspend_runtime_process()?;
            launched.runtime.status = "paused".to_string();
            if let Some(async_runtime) = launched.async_runtime.as_mut() {
                if async_runtime.state != "paused" {
                    async_runtime.pause_count = async_runtime.pause_count.saturating_add(1);
                }
                async_runtime.state = "paused".to_string();
            }
        }
        launched.stopped_reason = "pause".to_string();
        self.queue_stopped_event();
        Ok(serde_json::json!({}))
    }

    fn terminate_threads_result(
        &mut self,
        request: &serde_json::Value,
    ) -> anyhow::Result<serde_json::Value> {
        self.require_launch("terminateThreads")?;
        let terminates_reference_thread = request
            .pointer("/arguments/threadIds")
            .and_then(serde_json::Value::as_array)
            .is_none_or(|thread_ids| {
                thread_ids
                    .iter()
                    .any(|thread_id| thread_id.as_u64() == Some(1))
            });
        if !terminates_reference_thread {
            anyhow::bail!("unknown ORV thread id");
        }
        if let Some(launched) = &self.launched {
            launched.write_runtime_request_trace_file()?;
        }
        self.queue_event("terminated", serde_json::json!({}));
        self.launched = None;
        Ok(serde_json::json!({}))
    }

    fn require_launch(&self, command: &str) -> anyhow::Result<()> {
        self.launched
            .as_ref()
            .map(|_| ())
            .ok_or_else(|| anyhow::anyhow!("launch is required before {command}"))
    }

    fn queue_stopped_event(&mut self) {
        let Some(launched) = &self.launched else {
            return;
        };
        self.queue_event(
            "stopped",
            serde_json::json!({
                "reason": launched.stopped_reason,
                "threadId": 1,
                "allThreadsStopped": false,
            }),
        );
    }

    fn queue_event(&mut self, event: &str, body: serde_json::Value) {
        self.pending_events.push(DapPendingEvent {
            event: event.to_string(),
            body,
        });
    }

    fn revalidate_instruction_breakpoints(&mut self, frame_count: usize) {
        for breakpoint in &mut self.instruction_breakpoints {
            *breakpoint = dap_instruction_breakpoint(
                breakpoint.id,
                breakpoint.instruction_reference.clone(),
                breakpoint.offset,
                Some(frame_count),
            );
        }
    }

    fn set_current_local_value(&mut self, name: &str, value: &str) -> anyhow::Result<DapVariable> {
        let launched = self
            .launched
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("launch is required before setting variables"))?;
        let frame = launched
            .frames
            .get_mut(launched.current_frame_index)
            .ok_or_else(|| anyhow::anyhow!("no current runtime frame"))?;
        let variable = frame
            .locals
            .iter_mut()
            .find(|variable| variable.name == name)
            .ok_or_else(|| anyhow::anyhow!("unknown local variable `{name}`"))?;
        variable.value = value.to_string();
        Ok(variable.clone())
    }

    fn queue_current_frame_output(&mut self) {
        let output = self
            .launched
            .as_ref()
            .and_then(|launched| launched.frames.get(launched.current_frame_index))
            .map(|frame| frame.output.clone())
            .unwrap_or_default();
        self.queue_stdout_output(&output);
    }

    fn queue_frame_outputs(&mut self, start: usize, end: usize) {
        let outputs = self.launched.as_ref().map_or_else(Vec::new, |launched| {
            if start > end {
                return Vec::new();
            }
            launched
                .frames
                .iter()
                .enumerate()
                .skip(start)
                .take(end.saturating_sub(start).saturating_add(1))
                .flat_map(|(index, frame)| {
                    let mut outputs = Vec::new();
                    if !frame.output.is_empty() {
                        outputs.push(("stdout".to_string(), frame.output.clone()));
                    }
                    outputs.extend(
                        self.logpoint_outputs(&launched.frames, index)
                            .into_iter()
                            .map(|output| ("console".to_string(), output)),
                    );
                    outputs
                })
                .collect()
        });
        for (category, output) in outputs {
            self.queue_output(&category, &output);
        }
    }

    fn queue_stdout_output(&mut self, output: &str) {
        self.queue_output("stdout", output);
    }

    fn queue_output(&mut self, category: &str, output: &str) {
        if output.is_empty() {
            return;
        }
        self.queue_event(
            "output",
            serde_json::json!({
                "category": category,
                "output": output,
            }),
        );
    }

    pub(crate) fn drain_pending_events(&mut self) -> Vec<serde_json::Value> {
        std::mem::take(&mut self.pending_events)
            .into_iter()
            .map(|event| {
                dap_event_response(self.next_response_seq(), event.event.as_str(), &event.body)
            })
            .collect()
    }

    fn first_verified_breakpoint_frame(&self, frames: &[DapFrameState]) -> Option<usize> {
        frames
            .iter()
            .enumerate()
            .find_map(|(index, _)| self.breakpoint_frame_reason(frames, index).map(|_| index))
    }

    fn next_verified_breakpoint_frame(&self, launched: &DapLaunchState) -> Option<usize> {
        launched
            .frames
            .iter()
            .enumerate()
            .skip(launched.current_frame_index.saturating_add(1))
            .find_map(|(index, _)| {
                self.breakpoint_frame_reason(&launched.frames, index)
                    .map(|_| index)
            })
    }

    fn previous_verified_breakpoint_frame(&self, launched: &DapLaunchState) -> Option<usize> {
        (0..launched.current_frame_index).rev().find(|index| {
            self.breakpoint_frame_reason(&launched.frames, *index)
                .is_some()
        })
    }

    fn breakpoint_frame_reason(
        &self,
        frames: &[DapFrameState],
        index: usize,
    ) -> Option<&'static str> {
        let frame = frames.get(index)?;
        if self.has_verified_line_breakpoint(frames, index) {
            return Some("breakpoint");
        }
        if self.has_verified_function_breakpoint(frame) {
            return Some("function breakpoint");
        }
        if self.has_verified_instruction_breakpoint(index) {
            return Some("instruction breakpoint");
        }
        self.has_verified_data_breakpoint(frames, index)
            .then_some("data breakpoint")
    }

    fn has_verified_line_breakpoint(&self, frames: &[DapFrameState], index: usize) -> bool {
        let Some(frame) = frames.get(index) else {
            return false;
        };
        let normalized = dap_normalize_path(&frame.source.path);
        self.breakpoints
            .get(&normalized)
            .is_some_and(|breakpoints| {
                breakpoints.iter().any(|breakpoint| {
                    breakpoint.verified
                        && breakpoint.log_message.is_none()
                        && breakpoint.line == frame.line
                        && dap_breakpoint_condition_matches(frame, breakpoint.condition.as_deref())
                        && self.line_breakpoint_hit_condition_matches(
                            frames,
                            index,
                            &normalized,
                            breakpoint,
                        )
                })
            })
    }

    fn logpoint_outputs(&self, frames: &[DapFrameState], index: usize) -> Vec<String> {
        let Some(frame) = frames.get(index) else {
            return Vec::new();
        };
        let normalized = dap_normalize_path(&frame.source.path);
        self.breakpoints
            .get(&normalized)
            .map_or_else(Vec::new, |breakpoints| {
                breakpoints
                    .iter()
                    .filter(|breakpoint| {
                        breakpoint.verified
                            && breakpoint.line == frame.line
                            && breakpoint.log_message.is_some()
                            && dap_breakpoint_condition_matches(
                                frame,
                                breakpoint.condition.as_deref(),
                            )
                            && self.line_breakpoint_hit_condition_matches(
                                frames,
                                index,
                                &normalized,
                                breakpoint,
                            )
                    })
                    .filter_map(|breakpoint| breakpoint.log_message.as_deref())
                    .map(dap_logpoint_output)
                    .collect()
            })
    }

    fn line_breakpoint_hit_condition_matches(
        &self,
        frames: &[DapFrameState],
        index: usize,
        normalized_path: &Path,
        breakpoint: &DapBreakpoint,
    ) -> bool {
        let Some(hit_condition) = breakpoint.hit_condition.as_deref() else {
            return true;
        };
        let hit_count = frames[..=index]
            .iter()
            .filter(|frame| {
                dap_normalize_path(&frame.source.path) == normalized_path
                    && frame.line == breakpoint.line
                    && dap_breakpoint_condition_matches(frame, breakpoint.condition.as_deref())
            })
            .count();
        dap_hit_condition_matches(hit_condition, hit_count)
    }

    fn has_verified_function_breakpoint(&self, frame: &DapFrameState) -> bool {
        let Some(function_name) = frame.stack.last().map(|frame| frame.name.as_str()) else {
            return false;
        };
        self.function_breakpoints
            .iter()
            .any(|breakpoint| breakpoint.verified && breakpoint.name == function_name)
    }

    fn has_verified_instruction_breakpoint(&self, index: usize) -> bool {
        self.instruction_breakpoints
            .iter()
            .any(|breakpoint| breakpoint.verified && breakpoint.frame_index == Some(index))
    }

    fn has_verified_data_breakpoint(&self, frames: &[DapFrameState], index: usize) -> bool {
        let Some(frame) = frames.get(index) else {
            return false;
        };
        self.data_breakpoints
            .iter()
            .filter(|breakpoint| breakpoint.verified)
            .any(|breakpoint| {
                let Some(name) = dap_data_breakpoint_local_name(&breakpoint.data_id) else {
                    return false;
                };
                let Some(current) = dap_frame_local_value(frame, name) else {
                    return false;
                };
                let previous = frames[..index]
                    .iter()
                    .rev()
                    .find_map(|frame| dap_frame_local_value(frame, name));
                previous != Some(current)
            })
    }
}

pub(crate) struct DapScopeFrame {
    pub(crate) name: String,
    pub(crate) source: DapSourceInfo,
    pub(crate) line: u64,
}
