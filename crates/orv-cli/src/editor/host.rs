#![allow(clippy::redundant_pub_crate, clippy::wildcard_imports)]

use super::*;

pub(crate) fn cmd_editor_host(dir: &Path, listen: &str, once: bool) -> anyhow::Result<()> {
    let root = dir.canonicalize().map_err(|e| {
        anyhow::anyhow!(
            "failed to canonicalize editor export directory {}: {e}",
            dir.display()
        )
    })?;
    if !root.join(EDITOR_NATIVE_HOST_MANIFEST_PATH).is_file() {
        anyhow::bail!(
            "editor host requires {} under {}",
            EDITOR_NATIVE_HOST_MANIFEST_PATH,
            root.display()
        );
    }
    let listener = std::net::TcpListener::bind(listen)?;
    let address = listener.local_addr()?;
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "schema_version": 1,
            "kind": "orv.editor.native_host.server",
            "url": format!("http://{address}/"),
            "root": root.display().to_string(),
            "action_endpoint": "/__orv/native-host/action",
        }))?
    );
    std::io::Write::flush(&mut std::io::stdout())?;
    if once {
        let (mut stream, _) = listener.accept()?;
        editor_native_host_bridge_handle_stream(&root, &mut stream)?;
        return Ok(());
    }
    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                if let Err(error) = editor_native_host_bridge_handle_stream(&root, &mut stream) {
                    eprintln!("editor host request error: {error}");
                }
            }
            Err(error) => eprintln!("editor host accept error: {error}"),
        }
    }
    Ok(())
}

pub(crate) fn editor_native_host_bridge_js() -> &'static str {
    r#"(function () {
  const result = {
    json: "trace/action-result.json",
    html: "trace/action-result.html"
  };

  function runnerCommand(action) {
    if (Array.isArray(action && action.runner_command)) return action.runner_command;
    const command = ["orv", "editor", "run-action", "native-host.json"];
    if (action && action.action) command.push("--action", action.action);
    if (action && Number.isInteger(action.frame_index)) command.push("--frame-index", String(action.frame_index));
    if (action && action.slot) command.push("--slot", action.slot);
    return command;
  }

  function postToNativeHost(payload) {
    if (window.webkit && window.webkit.messageHandlers && window.webkit.messageHandlers.orvNativeHost) {
      window.webkit.messageHandlers.orvNativeHost.postMessage(payload);
      return { posted: true, target: "webkit.messageHandlers.orvNativeHost" };
    }
    if (window.chrome && window.chrome.webview && typeof window.chrome.webview.postMessage === "function") {
      window.chrome.webview.postMessage(payload);
      return { posted: true, target: "chrome.webview" };
    }
    if (typeof window.__ORV_NATIVE_HOST_POST_MESSAGE__ === "function") {
      window.__ORV_NATIVE_HOST_POST_MESSAGE__(payload);
      return { posted: true, target: "__ORV_NATIVE_HOST_POST_MESSAGE__" };
    }
    window.dispatchEvent(new CustomEvent("orv:native-host-command", { detail: payload }));
    return { posted: false, target: "orv:native-host-command" };
  }

  function postToLocalBridge(payload) {
    if (typeof window.fetch !== "function") return null;
    if (!window.location || !["http:", "https:"].includes(window.location.protocol)) return null;
    const endpoint = "/__orv/native-host/action";
    fetch(endpoint, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(payload)
    }).then(response => {
      if (!response.ok) throw new Error(`native-host action failed: ${response.status}`);
      return response.json();
    }).then(body => {
      const event = payload.refresh && payload.refresh.event ? payload.refresh.event : "orv:trace-action-result";
      window.dispatchEvent(new CustomEvent(event, { detail: body }));
    }).catch(error => {
      const fallback = postToNativeHost(payload);
      window.dispatchEvent(new CustomEvent("orv:native-host-command-error", {
        detail: { payload, error: String(error), fallback }
      }));
    });
    return { posted: true, target: endpoint };
  }

  function sourcePermissions() {
    return window.orvNativeHostSourcePermissions || {};
  }

  function isSourceRevealAction(action) {
    const name = String((action && action.action) || "");
    if (name.includes("reveal")) return true;
    if (action && action.source && action.source.path) return true;
    const command = action && Array.isArray(action.command) ? action.command : [];
    return command.includes("reveal");
  }

  function sourceRevealAllowed(action) {
    const permissions = sourcePermissions();
    if (permissions.allowed !== false || !isSourceRevealAction(action)) return true;
    const detail = {
      kind: "orv.editor.native_host.source_permission.blocked",
      action,
      permissions
    };
    window.dispatchEvent(new CustomEvent(permissions.blocked_event || "orv:source-permission-blocked", { detail }));
    return false;
  }

  const host = window.orvNativeHost || {};
  host.runAction = function runAction(action) {
    if (!sourceRevealAllowed(action)) {
      return { posted: false, target: "source-permission", blocked: true };
    }
    const payload = {
      kind: "orv.editor.native_host.command",
      action,
      command: runnerCommand(action || {}),
      result,
      refresh: {
        event: "orv:trace-action-result",
        panel: "trace_action_result",
        json: result.json,
        html: result.html
      }
    };
    const localBridge = postToLocalBridge(payload);
    if (localBridge) return localBridge;
    return postToNativeHost(payload);
  };
  window.orvNativeHost = host;
}());
"#
}

pub(crate) fn editor_native_host_open_url(url: &str) -> anyhow::Result<serde_json::Value> {
    #[cfg(target_os = "macos")]
    let status = ProcessCommand::new("open").arg(url).status()?;
    #[cfg(target_os = "windows")]
    let status = ProcessCommand::new("cmd")
        .args(["/C", "start", "", url])
        .status()?;
    #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
    let status = ProcessCommand::new("xdg-open").arg(url).status()?;
    Ok(serde_json::json!({
        "success": status.success(),
        "code": status.code(),
    }))
}

#[derive(Debug, Clone)]
pub(crate) struct EditorHostHttpResponse {
    pub(crate) status: u16,
    pub(crate) reason: &'static str,
    pub(crate) content_type: &'static str,
    pub(crate) body: Vec<u8>,
}

impl EditorHostHttpResponse {
    fn to_http_bytes(&self) -> Vec<u8> {
        let mut bytes = format!(
            "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nAccess-Control-Allow-Origin: *\r\nAccess-Control-Allow-Methods: GET, POST, OPTIONS\r\nAccess-Control-Allow-Headers: Content-Type\r\nConnection: close\r\n\r\n",
            self.status,
            self.reason,
            self.content_type,
            self.body.len()
        )
        .into_bytes();
        bytes.extend_from_slice(&self.body);
        bytes
    }
}

pub(crate) fn editor_native_host_bridge_action_json(
    root: &Path,
    payload: &serde_json::Value,
) -> anyhow::Result<serde_json::Value> {
    validate_editor_native_host_bridge_payload(payload)?;
    let action = payload
        .get("action")
        .filter(|value| value.is_object())
        .unwrap_or(payload);
    let action_id = action
        .get("action")
        .and_then(serde_json::Value::as_str)
        .or_else(|| payload.get("action").and_then(serde_json::Value::as_str))
        .unwrap_or("trace.route.reveal");
    let frame_index = action
        .get("frame_index")
        .and_then(serde_json::Value::as_u64)
        .or_else(|| {
            payload
                .get("frame_index")
                .and_then(serde_json::Value::as_u64)
        });
    let slot = action
        .get("slot")
        .and_then(serde_json::Value::as_str)
        .or_else(|| payload.get("slot").and_then(serde_json::Value::as_str));
    let result = editor_native_host_run_action_json(root, action_id, frame_index, slot)?;
    write_editor_trace_action_result_if_configured(root, &result)?;
    write_editor_trace_action_result_html_if_configured(root, &result)?;
    Ok(serde_json::json!({
        "schema_version": 1,
        "kind": "orv.editor.native_host.bridge.action.response",
        "status": "passed",
        "request": payload,
        "result": result,
        "refresh": payload.get("refresh").cloned().unwrap_or_else(|| serde_json::json!({
            "event": "orv:trace-action-result",
            "panel": "trace_action_result",
            "json": EDITOR_TRACE_ACTION_RESULT_PATH,
            "html": EDITOR_TRACE_ACTION_RESULT_HTML_PATH,
        })),
    }))
}

pub(crate) fn validate_editor_native_host_bridge_payload(
    payload: &serde_json::Value,
) -> anyhow::Result<()> {
    match payload.get("kind").and_then(serde_json::Value::as_str) {
        Some("orv.editor.native_host.command") => {
            verify_json_object_keys_allowing_optional(
                payload,
                &["kind", "action", "command", "refresh"],
                &["result"],
                "native-host bridge command",
            )?;
            if let Some(action) = payload.get("action").filter(|value| value.is_object()) {
                validate_editor_native_host_reveal_action(action)?;
            }
            verify_json_object_keys_exact(
                payload.get("refresh").ok_or_else(|| {
                    anyhow::anyhow!("native-host bridge command refresh must be an object")
                })?,
                &["event", "panel", "json", "html"],
                "native-host bridge command refresh",
            )?;
        }
        Some("orv.editor.native_host.reveal_action") => {
            validate_editor_native_host_reveal_action(payload)?;
        }
        _ => {}
    }
    Ok(())
}

pub(crate) fn editor_native_host_bridge_http_response(
    root: &Path,
    method: &str,
    raw_path: &str,
    body: &[u8],
) -> EditorHostHttpResponse {
    let path = raw_path.split('?').next().unwrap_or(raw_path);
    match (method, path) {
        ("OPTIONS", "/__orv/native-host/action") => editor_host_empty_response(204, "No Content"),
        ("POST", "/__orv/native-host/action") => match serde_json::from_slice(body) {
            Ok(payload) => match editor_native_host_bridge_action_json(root, &payload) {
                Ok(value) => editor_host_json_response(200, "OK", &value),
                Err(error) => editor_host_json_response(
                    500,
                    "Internal Server Error",
                    &serde_json::json!({
                        "schema_version": 1,
                        "kind": "orv.editor.native_host.bridge.error",
                        "status": "failed",
                        "error": error.to_string(),
                    }),
                ),
            },
            Err(error) => editor_host_json_response(
                400,
                "Bad Request",
                &serde_json::json!({
                    "schema_version": 1,
                    "kind": "orv.editor.native_host.bridge.error",
                    "status": "failed",
                    "error": format!("invalid JSON payload: {error}"),
                }),
            ),
        },
        ("GET", _) => editor_host_static_response(root, path),
        ("HEAD", _) => {
            let mut response = editor_host_static_response(root, path);
            response.body.clear();
            response
        }
        _ => editor_host_json_response(
            405,
            "Method Not Allowed",
            &serde_json::json!({
                "schema_version": 1,
                "kind": "orv.editor.native_host.bridge.error",
                "status": "failed",
                "error": "method not allowed",
            }),
        ),
    }
}

pub(crate) fn editor_native_host_bridge_handle_stream(
    root: &Path,
    stream: &mut std::net::TcpStream,
) -> anyhow::Result<()> {
    let mut reader = std::io::BufReader::new(stream.try_clone()?);
    let mut request_line = String::new();
    if std::io::BufRead::read_line(&mut reader, &mut request_line)? == 0 {
        return Ok(());
    }
    let mut content_length = 0usize;
    loop {
        let mut line = String::new();
        if std::io::BufRead::read_line(&mut reader, &mut line)? == 0 {
            break;
        }
        let header = line.trim_end_matches('\n').trim_end_matches('\r');
        if header.is_empty() {
            break;
        }
        if let Some((name, value)) = header.split_once(':') {
            if name.eq_ignore_ascii_case("Content-Length") {
                content_length = value.trim().parse::<usize>().map_err(|error| {
                    anyhow::anyhow!("invalid Content-Length header `{}`: {error}", value.trim())
                })?;
            }
        }
    }
    let mut body = vec![0_u8; content_length];
    if content_length > 0 {
        std::io::Read::read_exact(&mut reader, &mut body)?;
    }
    let mut parts = request_line.split_whitespace();
    let Some(method) = parts.next() else {
        return editor_native_host_bridge_write_response(
            stream,
            &editor_host_json_response(
                400,
                "Bad Request",
                &serde_json::json!({
                    "schema_version": 1,
                    "kind": "orv.editor.native_host.bridge.error",
                    "status": "failed",
                    "error": "missing HTTP method",
                }),
            ),
        );
    };
    let Some(path) = parts.next() else {
        return editor_native_host_bridge_write_response(
            stream,
            &editor_host_json_response(
                400,
                "Bad Request",
                &serde_json::json!({
                    "schema_version": 1,
                    "kind": "orv.editor.native_host.bridge.error",
                    "status": "failed",
                    "error": "missing HTTP path",
                }),
            ),
        );
    };
    let response = editor_native_host_bridge_http_response(root, method, path, &body);
    editor_native_host_bridge_write_response(stream, &response)
}

pub(super) fn editor_native_host_bridge_write_response(
    stream: &mut std::net::TcpStream,
    response: &EditorHostHttpResponse,
) -> anyhow::Result<()> {
    std::io::Write::write_all(stream, &response.to_http_bytes())?;
    std::io::Write::flush(stream)?;
    Ok(())
}

pub(super) fn editor_host_json_response(
    status: u16,
    reason: &'static str,
    value: &serde_json::Value,
) -> EditorHostHttpResponse {
    let body = serde_json::to_vec_pretty(value).unwrap_or_else(|error| {
        format!(
            "{{\"kind\":\"orv.editor.native_host.bridge.error\",\"error\":\"json encode failed: {error}\"}}"
        )
        .into_bytes()
    });
    EditorHostHttpResponse {
        status,
        reason,
        content_type: "application/json; charset=utf-8",
        body,
    }
}

pub(super) fn editor_host_empty_response(
    status: u16,
    reason: &'static str,
) -> EditorHostHttpResponse {
    EditorHostHttpResponse {
        status,
        reason,
        content_type: "text/plain; charset=utf-8",
        body: Vec::new(),
    }
}

pub(super) fn editor_host_static_response(root: &Path, raw_path: &str) -> EditorHostHttpResponse {
    let Some(path) = editor_host_static_file_path(root, raw_path) else {
        return editor_host_json_response(
            400,
            "Bad Request",
            &serde_json::json!({
                "schema_version": 1,
                "kind": "orv.editor.native_host.bridge.error",
                "status": "failed",
                "error": "invalid static path",
            }),
        );
    };
    if !path.is_file() {
        return editor_host_json_response(
            404,
            "Not Found",
            &serde_json::json!({
                "schema_version": 1,
                "kind": "orv.editor.native_host.bridge.error",
                "status": "failed",
                "error": "artifact not found",
                "path": raw_path,
            }),
        );
    }
    match std::fs::read(&path) {
        Ok(body) => EditorHostHttpResponse {
            status: 200,
            reason: "OK",
            content_type: editor_host_content_type(&path),
            body,
        },
        Err(error) => editor_host_json_response(
            500,
            "Internal Server Error",
            &serde_json::json!({
                "schema_version": 1,
                "kind": "orv.editor.native_host.bridge.error",
                "status": "failed",
                "error": error.to_string(),
                "path": raw_path,
            }),
        ),
    }
}

pub(super) fn editor_host_static_file_path(root: &Path, raw_path: &str) -> Option<PathBuf> {
    let path = raw_path.trim_start_matches('/');
    let path = if path.is_empty() { "index.html" } else { path };
    let decoded = editor_host_percent_decode_path(path)?;
    let mut relative = PathBuf::new();
    for component in Path::new(&decoded).components() {
        match component {
            Component::Normal(part) => relative.push(part),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => return None,
        }
    }
    if relative.as_os_str().is_empty() {
        relative.push("index.html");
    }
    Some(root.join(relative))
}

pub(super) fn editor_host_percent_decode_path(path: &str) -> Option<String> {
    let bytes = path.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0usize;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            let high = *bytes.get(index + 1)?;
            let low = *bytes.get(index + 2)?;
            decoded.push((editor_host_hex_value(high)? << 4) | editor_host_hex_value(low)?);
            index += 3;
        } else {
            decoded.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8(decoded).ok()
}

const fn editor_host_hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

pub(super) fn editor_host_content_type(path: &Path) -> &'static str {
    match path.extension().and_then(std::ffi::OsStr::to_str) {
        Some("html") => "text/html; charset=utf-8",
        Some("js") => "text/javascript; charset=utf-8",
        Some("json") => "application/json; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("sse") => "text/event-stream; charset=utf-8",
        Some("svg") => "image/svg+xml",
        Some("wasm") => "application/wasm",
        _ => "application/octet-stream",
    }
}

pub(crate) fn editor_native_host_manifest_json(
    entry: &Path,
    state: &serde_json::Value,
) -> serde_json::Value {
    let debug = state.get("debug").unwrap_or(&serde_json::Value::Null);
    let adapter = debug.get("adapter").unwrap_or(&serde_json::Value::Null);
    let capabilities = debug
        .get("capabilities")
        .cloned()
        .unwrap_or_else(editor_debug_capabilities_json);
    let runner = debug
        .get("session_runner")
        .unwrap_or(&serde_json::Value::Null);
    let result_artifact = debug
        .get("result_artifact")
        .cloned()
        .or_else(|| runner.get("result").cloned())
        .unwrap_or_else(editor_debug_result_artifact_json);
    let controls = debug
        .get("controls")
        .and_then(serde_json::Value::as_array)
        .map_or(0, Vec::len);
    let configurations = debug
        .get("configurations")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    let configuration_count = configurations.len();
    let source_inventory = debug.get("source_inventory").cloned().unwrap_or_else(|| {
        serde_json::json!({
            "schema_version": 1,
            "kind": "orv.editor.debug.source_inventory",
            "protocol": "dap",
            "source_count": 0,
            "sources": [],
        })
    });
    let source_count = json_array_count(source_inventory.get("sources"));
    let production_context = debug
        .get("production_context")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let debug_production_context = !production_context.is_null();
    let breakpoint_count = debug
        .get("breakpoint_sources")
        .and_then(serde_json::Value::as_array)
        .map_or(0, |sources| {
            sources
                .iter()
                .map(|source| json_usize_field(source, "line_count"))
                .sum::<usize>()
        });
    let function_breakpoint_count = debug
        .get("function_breakpoints")
        .and_then(serde_json::Value::as_array)
        .map_or(0, Vec::len);
    let data_breakpoint_count = debug
        .get("data_breakpoints")
        .and_then(serde_json::Value::as_array)
        .map_or(0, Vec::len);
    let exception_filter_count = debug
        .get("exception_filters")
        .and_then(serde_json::Value::as_array)
        .map_or(0, Vec::len);
    let control_commands = debug
        .get("controls")
        .and_then(serde_json::Value::as_array)
        .map(|controls| {
            controls
                .iter()
                .map(|control| {
                    serde_json::json!({
                        "name": control.get("name").cloned().unwrap_or_else(|| serde_json::json!("control")),
                        "request": control.get("request").cloned().unwrap_or_else(|| serde_json::json!({})),
                        "command": control.get("runner_command").cloned().unwrap_or_else(|| serde_json::json!([])),
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let breakpoint_commands = editor_native_host_breakpoint_commands_json(debug);
    let function_breakpoint_commands = editor_native_host_function_breakpoint_commands_json(debug);
    let data_breakpoint_commands = editor_native_host_data_breakpoint_commands_json(debug);
    let exception_filter_commands = editor_native_host_exception_filter_commands_json(debug);
    let trace_enabled = state.get("trace").is_some();
    let production_enabled = state.get("production").is_some();
    let production_state = state
        .get("production")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let production_adapters = production_adapter_count(&production_state) > 0;
    let client_bundles = production_client_bundle_count(&production_state) > 0;
    let production_preflight = json_array_count(production_state.get("preflight")) > 0;
    let production_route_policies =
        production_preflight_route_policy_count_from_value(&production_state) > 0;
    let production_graph_contract = json_array_count(production_state.get("graph_contract")) > 0;
    let production = editor_native_host_production_json(&production_state);
    let runtime = editor_native_host_runtime_json(state);
    let trace = editor_native_host_trace_json(state);
    let trace_reveal_actions = json_array_count(trace.get("actions")) > 0;
    let panels =
        editor_native_host_panel_inventory_json(&result_artifact, &runtime, &production, &trace);
    let mut artifacts = serde_json::json!({
        "shell": "index.html",
        "state": "state.json",
        "debug_session_runner": EDITOR_DEBUG_SESSION_RUNNER_PATH,
        "debug_session_result": EDITOR_DEBUG_SESSION_RESULT_PATH,
        "debug_session_result_html": EDITOR_DEBUG_SESSION_RESULT_HTML_PATH,
        "runtime_panel_html": EDITOR_RUNTIME_PANEL_HTML_PATH,
        "native_host_bridge_js": EDITOR_NATIVE_HOST_BRIDGE_JS_PATH,
        "native_host_desktop_package": EDITOR_NATIVE_HOST_DESKTOP_PACKAGE_PATH,
        "native_host_desktop_launcher": EDITOR_NATIVE_HOST_DESKTOP_LAUNCHER_PATH,
        "native_host_desktop_packaging": EDITOR_NATIVE_HOST_DESKTOP_PACKAGING_PATH,
        "native_host_desktop_package_script": EDITOR_NATIVE_HOST_DESKTOP_PACKAGE_SCRIPT_PATH,
        "native_host_desktop_app_package": EDITOR_NATIVE_HOST_DESKTOP_APP_PACKAGE_PATH,
        "native_host_desktop_app_info_plist": EDITOR_NATIVE_HOST_DESKTOP_APP_INFO_PLIST_PATH,
        "native_host_desktop_app_entitlements": EDITOR_NATIVE_HOST_DESKTOP_APP_ENTITLEMENTS_PATH,
        "native_host_desktop_app_main": EDITOR_NATIVE_HOST_DESKTOP_APP_MAIN_PATH,
    });
    if trace_enabled {
        let artifacts = artifacts
            .as_object_mut()
            .expect("native host artifacts is object");
        artifacts.insert(
            "trace_panel_html".to_string(),
            serde_json::json!(EDITOR_TRACE_PANEL_HTML_PATH),
        );
        artifacts.insert(
            "trace_action_result".to_string(),
            serde_json::json!(EDITOR_TRACE_ACTION_RESULT_PATH),
        );
        artifacts.insert(
            "trace_action_result_html".to_string(),
            serde_json::json!(EDITOR_TRACE_ACTION_RESULT_HTML_PATH),
        );
    }
    if production_enabled {
        artifacts
            .as_object_mut()
            .expect("native host artifacts is object")
            .insert(
                "production_panel_html".to_string(),
                serde_json::json!(EDITOR_PRODUCTION_PANEL_HTML_PATH),
            );
    }
    serde_json::json!({
        "schema_version": 1,
        "kind": "orv.editor.native_host",
        "entry": entry.display().to_string(),
        "artifacts": artifacts,
        "debug": {
            "protocol": adapter.get("protocol").cloned().unwrap_or_else(|| serde_json::json!("dap")),
            "adapter_command": adapter.get("command").cloned().unwrap_or_else(|| serde_json::json!(["orv", "dap", "serve", "--stdio"])),
            "capabilities": capabilities,
            "runner_command": runner.get("command").cloned().unwrap_or_else(|| editor_debug_control_runner_command(EditorDebugControl::Next)),
            "configurations": configurations,
            "configuration_count": configuration_count,
            "source_inventory": source_inventory,
            "source_count": source_count,
            "production_context": production_context,
            "control_commands": control_commands,
            "breakpoint_commands": breakpoint_commands,
            "function_breakpoint_commands": function_breakpoint_commands,
            "data_breakpoint_commands": data_breakpoint_commands,
            "exception_filter_commands": exception_filter_commands,
            "panel_contract": editor_native_host_debug_panel_contract_json(),
            "control_count": controls,
            "breakpoint_argument": runner
                .pointer("/session/breakpoint_argument")
                .cloned()
                .unwrap_or_else(|| serde_json::json!("--breakpoint")),
            "breakpoint_format": runner
                .pointer("/session/breakpoint_format")
                .cloned()
                .unwrap_or_else(|| serde_json::json!("<path>:<line>")),
            "function_breakpoint_argument": runner
                .pointer("/session/function_breakpoint_argument")
                .cloned()
                .unwrap_or_else(|| serde_json::json!("--function-breakpoint")),
            "function_breakpoint_format": runner
                .pointer("/session/function_breakpoint_format")
                .cloned()
                .unwrap_or_else(|| serde_json::json!("<function-name>")),
            "data_breakpoint_argument": runner
                .pointer("/session/data_breakpoint_argument")
                .cloned()
                .unwrap_or_else(|| serde_json::json!("--data-breakpoint")),
            "data_breakpoint_format": runner
                .pointer("/session/data_breakpoint_format")
                .cloned()
                .unwrap_or_else(|| serde_json::json!("<local-name>")),
            "exception_filter_argument": runner
                .pointer("/session/exception_filter_argument")
                .cloned()
                .unwrap_or_else(|| serde_json::json!("--exception-filter")),
            "exception_filter_format": runner
                .pointer("/session/exception_filter_format")
                .cloned()
                .unwrap_or_else(|| serde_json::json!("<orv.diagnostics|orv.runtime>")),
            "watch_expression_argument": runner
                .pointer("/session/watch_expression_argument")
                .cloned()
                .unwrap_or_else(|| serde_json::json!("--watch-expression")),
            "watch_expression_format": runner
                .pointer("/session/watch_expression_format")
                .cloned()
                .unwrap_or_else(|| serde_json::json!("<expression>")),
            "result_path": runner
                .pointer("/result/path")
                .cloned()
                .unwrap_or_else(|| serde_json::json!(EDITOR_DEBUG_SESSION_RESULT_PATH)),
            "result_kind": runner
                .pointer("/result/kind")
                .cloned()
                .unwrap_or_else(|| serde_json::json!("orv.editor.debug.runner.result")),
            "result_artifact": result_artifact,
            "breakpoint_count": breakpoint_count,
            "function_breakpoint_count": function_breakpoint_count,
            "data_breakpoint_count": data_breakpoint_count,
            "exception_filter_count": exception_filter_count,
            "reuse_session": runner
                .pointer("/session/reuse_session")
                .cloned()
                .unwrap_or(serde_json::json!(true)),
        },
        "runtime": runtime,
        "production": production,
        "trace": trace,
        "host": {
            "schema_version": 1,
            "kind": "orv.editor.native_host.local_bridge",
            "shell": "index.html",
            "bridge_script": EDITOR_NATIVE_HOST_BRIDGE_JS_PATH,
            "desktop_package": EDITOR_NATIVE_HOST_DESKTOP_PACKAGE_PATH,
            "desktop_launcher": EDITOR_NATIVE_HOST_DESKTOP_LAUNCHER_PATH,
            "desktop_platform_matrix": editor_native_host_desktop_platform_matrix_json(),
            "desktop_app": editor_native_host_desktop_app_contract_json(),
            "desktop_packaging": editor_native_host_desktop_packaging_json(),
            "action_endpoint": "/__orv/native-host/action",
            "command_format": [
                "orv",
                "editor",
                "host",
                "<export-dir>",
                "--listen",
                "<host:port>",
            ],
        },
        "panels": panels,
        "capabilities": {
            "project_graph": true,
            "runtime_inspection": true,
            "dap_controls": controls > 0,
            "dap_sources": source_count > 0,
            "dap_production_context": debug_production_context,
            "production_adapters": production_adapters,
            "production_preflight": production_preflight,
            "production_route_policies": production_route_policies,
            "production_graph_contract": production_graph_contract,
            "client_bundles": client_bundles,
            "trace_navigation": trace_enabled,
            "trace_reveal_actions": trace_reveal_actions,
            "native_host_bridge": true,
            "native_host_local_bridge": true,
            "native_host_desktop_package": true,
            "native_host_desktop_app": true,
            "native_host_desktop_packaging": true,
            "native_host_desktop_platform_matrix": true,
        },
    })
}

pub(crate) fn editor_native_host_panel_inventory_json(
    debug_result_artifact: &serde_json::Value,
    runtime: &serde_json::Value,
    production: &serde_json::Value,
    trace: &serde_json::Value,
) -> Vec<serde_json::Value> {
    let mut panels = Vec::new();
    panels.push(editor_native_host_panel_inventory_entry_json(
        "debug_result",
        "Debug Result",
        "debug",
        debug_result_artifact
            .get("path")
            .and_then(serde_json::Value::as_str),
        debug_result_artifact
            .get("kind")
            .and_then(serde_json::Value::as_str),
        debug_result_artifact
            .get("media_type")
            .and_then(serde_json::Value::as_str),
        debug_result_artifact.get("panel_contract"),
    ));
    panels.push(editor_native_host_panel_inventory_entry_json(
        "runtime",
        "Runtime",
        "runtime",
        runtime
            .pointer("/panel_artifact/path")
            .and_then(serde_json::Value::as_str),
        runtime
            .pointer("/panel_artifact/kind")
            .and_then(serde_json::Value::as_str),
        runtime
            .pointer("/panel_artifact/media_type")
            .and_then(serde_json::Value::as_str),
        runtime.get("panel_contract"),
    ));
    if !production.is_null() {
        panels.push(editor_native_host_panel_inventory_entry_json(
            "production",
            "Production",
            "production",
            production
                .pointer("/panel_artifact/path")
                .and_then(serde_json::Value::as_str),
            production
                .pointer("/panel_artifact/kind")
                .and_then(serde_json::Value::as_str),
            production
                .pointer("/panel_artifact/media_type")
                .and_then(serde_json::Value::as_str),
            production.get("panel_contract"),
        ));
    }
    if !trace.is_null() {
        panels.push(editor_native_host_panel_inventory_entry_json(
            "trace",
            "Trace",
            "trace",
            trace
                .pointer("/panel_artifact/path")
                .and_then(serde_json::Value::as_str),
            trace
                .pointer("/panel_artifact/kind")
                .and_then(serde_json::Value::as_str),
            trace
                .pointer("/panel_artifact/media_type")
                .and_then(serde_json::Value::as_str),
            trace.get("panel_contract"),
        ));
        panels.push(editor_native_host_panel_inventory_entry_json(
            "trace_action_result",
            "Trace Action Result",
            "trace_action",
            trace
                .pointer("/action_result_artifact/path")
                .and_then(serde_json::Value::as_str),
            trace
                .pointer("/action_result_artifact/kind")
                .and_then(serde_json::Value::as_str),
            trace
                .pointer("/action_result_artifact/media_type")
                .and_then(serde_json::Value::as_str),
            trace.pointer("/action_result_artifact/panel_contract"),
        ));
    }
    panels
}

pub(crate) fn editor_native_host_panel_inventory_entry_json(
    name: &str,
    title: &str,
    root: &str,
    path: Option<&str>,
    kind: Option<&str>,
    media_type: Option<&str>,
    panel_contract: Option<&serde_json::Value>,
) -> serde_json::Value {
    serde_json::json!({
        "name": name,
        "title": title,
        "root": root,
        "artifact": {
            "path": path.unwrap_or(""),
            "kind": kind.unwrap_or(""),
            "media_type": media_type.unwrap_or(""),
        },
        "panel_contract": panel_contract.cloned().unwrap_or(serde_json::Value::Null),
    })
}

pub(crate) fn editor_native_host_runtime_json(state: &serde_json::Value) -> serde_json::Value {
    let runtime_state = state.get("runtime").unwrap_or(&serde_json::Value::Null);
    let runtime = runtime_state
        .get("runtime")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));
    let frames = runtime_state
        .get("frames")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    let panel = runtime_state
        .pointer("/panels/runtime")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));
    serde_json::json!({
        "schema_version": 1,
        "status": runtime
            .get("status")
            .cloned()
            .unwrap_or_else(|| serde_json::json!("unknown")),
        "stdout": runtime
            .get("stdout")
            .cloned()
            .unwrap_or_else(|| serde_json::json!("")),
        "error": runtime
            .get("error")
            .cloned()
            .unwrap_or_else(|| serde_json::json!("")),
        "async": runtime
            .get("async")
            .cloned()
            .unwrap_or(serde_json::Value::Null),
        "frame_count": frames.len(),
        "frames": frames,
        "panel": panel,
        "panel_html_path": EDITOR_RUNTIME_PANEL_HTML_PATH,
        "panel_artifact": editor_runtime_panel_artifact_json(),
        "panel_contract": editor_native_host_runtime_panel_contract_json(),
    })
}

pub(crate) fn editor_native_host_runtime_panel_contract_json() -> serde_json::Value {
    serde_json::json!({
        "schema_version": 1,
        "root": "runtime",
        "sections": [
            {
                "name": "panel",
                "path": "runtime.panel",
                "kind": "object",
            },
            {
                "name": "frames",
                "path": "runtime.frames",
                "kind": "array",
            },
            {
                "name": "async",
                "path": "runtime.async",
                "kind": "object",
            },
            {
                "name": "panel_artifact",
                "path": "runtime.panel_artifact",
                "kind": "object",
            },
        ],
    })
}

pub(crate) fn editor_native_host_debug_panel_contract_json() -> serde_json::Value {
    serde_json::json!({
        "schema_version": 1,
        "root": "debug",
        "sections": [
            {
                "name": "adapter",
                "path": "debug.adapter_command",
                "kind": "array",
            },
            {
                "name": "capabilities",
                "path": "debug.capabilities",
                "kind": "object",
            },
            {
                "name": "configurations",
                "path": "debug.configurations",
                "kind": "array",
            },
            {
                "name": "source_inventory",
                "path": "debug.source_inventory",
                "kind": "object",
            },
            {
                "name": "production_context",
                "path": "debug.production_context",
                "kind": "object",
            },
            {
                "name": "control_commands",
                "path": "debug.control_commands",
                "kind": "array",
            },
            {
                "name": "breakpoint_commands",
                "path": "debug.breakpoint_commands",
                "kind": "array",
            },
            {
                "name": "function_breakpoint_commands",
                "path": "debug.function_breakpoint_commands",
                "kind": "array",
            },
            {
                "name": "data_breakpoint_commands",
                "path": "debug.data_breakpoint_commands",
                "kind": "array",
            },
            {
                "name": "exception_filter_commands",
                "path": "debug.exception_filter_commands",
                "kind": "array",
            },
            {
                "name": "function_breakpoint_argument",
                "path": "debug.function_breakpoint_argument",
                "kind": "string",
            },
            {
                "name": "data_breakpoint_argument",
                "path": "debug.data_breakpoint_argument",
                "kind": "string",
            },
            {
                "name": "exception_filter_argument",
                "path": "debug.exception_filter_argument",
                "kind": "string",
            },
            {
                "name": "watch_expression_argument",
                "path": "debug.watch_expression_argument",
                "kind": "string",
            },
            {
                "name": "result_artifact",
                "path": "debug.result_artifact",
                "kind": "object",
            },
        ],
    })
}

pub(crate) fn editor_native_host_breakpoint_commands_json(
    debug: &serde_json::Value,
) -> Vec<serde_json::Value> {
    let mut commands = Vec::new();
    let Some(sources) = debug
        .get("breakpoint_sources")
        .and_then(serde_json::Value::as_array)
    else {
        return commands;
    };
    for source_group in sources {
        let source = source_group
            .get("source")
            .cloned()
            .unwrap_or_else(|| serde_json::json!({}));
        let Some(breakpoints) = source_group
            .get("breakpoints")
            .and_then(serde_json::Value::as_array)
        else {
            continue;
        };
        for breakpoint in breakpoints {
            commands.push(serde_json::json!({
                "source": source,
                "line": breakpoint.get("line").cloned().unwrap_or(serde_json::Value::Null),
                "request": breakpoint
                    .get("request")
                    .cloned()
                    .unwrap_or_else(|| serde_json::json!({})),
                "command": breakpoint
                    .get("runner_command")
                    .cloned()
                    .unwrap_or_else(|| serde_json::json!([])),
            }));
        }
    }
    commands
}

pub(crate) fn editor_native_host_function_breakpoint_commands_json(
    debug: &serde_json::Value,
) -> Vec<serde_json::Value> {
    let Some(function_breakpoints) = debug
        .get("function_breakpoints")
        .and_then(serde_json::Value::as_array)
    else {
        return Vec::new();
    };
    function_breakpoints
        .iter()
        .map(|breakpoint| {
            serde_json::json!({
                "name": breakpoint
                    .get("name")
                    .cloned()
                    .unwrap_or_else(|| serde_json::json!("")),
                "kind": breakpoint
                    .get("kind")
                    .cloned()
                    .unwrap_or_else(|| serde_json::json!("function")),
                "source": breakpoint
                    .get("source")
                    .cloned()
                    .unwrap_or_else(|| serde_json::json!({})),
                "request": breakpoint
                    .get("request")
                    .cloned()
                    .unwrap_or_else(|| serde_json::json!({})),
                "command": breakpoint
                    .get("runner_command")
                    .cloned()
                    .unwrap_or_else(|| serde_json::json!([])),
            })
        })
        .collect()
}

pub(crate) fn editor_native_host_data_breakpoint_commands_json(
    debug: &serde_json::Value,
) -> Vec<serde_json::Value> {
    let Some(data_breakpoints) = debug
        .get("data_breakpoints")
        .and_then(serde_json::Value::as_array)
    else {
        return Vec::new();
    };
    data_breakpoints
        .iter()
        .map(|breakpoint| {
            serde_json::json!({
                "name": breakpoint
                    .get("name")
                    .cloned()
                    .unwrap_or_else(|| serde_json::json!("")),
                "data_id": breakpoint
                    .get("data_id")
                    .cloned()
                    .unwrap_or_else(|| serde_json::json!("")),
                "source": breakpoint
                    .get("source")
                    .cloned()
                    .unwrap_or_else(|| serde_json::json!({})),
                "info_request": breakpoint
                    .get("info_request")
                    .cloned()
                    .unwrap_or_else(|| serde_json::json!({})),
                "request": breakpoint
                    .get("request")
                    .cloned()
                    .unwrap_or_else(|| serde_json::json!({})),
                "command": breakpoint
                    .get("runner_command")
                    .cloned()
                    .unwrap_or_else(|| serde_json::json!([])),
            })
        })
        .collect()
}

pub(crate) fn editor_native_host_exception_filter_commands_json(
    debug: &serde_json::Value,
) -> Vec<serde_json::Value> {
    let Some(exception_filters) = debug
        .get("exception_filters")
        .and_then(serde_json::Value::as_array)
    else {
        return Vec::new();
    };
    exception_filters
        .iter()
        .map(|filter| {
            serde_json::json!({
                "filter": filter
                    .get("filter")
                    .cloned()
                    .unwrap_or_else(|| serde_json::json!("")),
                "label": filter
                    .get("label")
                    .cloned()
                    .unwrap_or_else(|| serde_json::json!("")),
                "request": filter
                    .get("request")
                    .cloned()
                    .unwrap_or_else(|| serde_json::json!({})),
                "command": filter
                    .get("runner_command")
                    .cloned()
                    .unwrap_or_else(|| serde_json::json!([])),
            })
        })
        .collect()
}

pub(crate) fn editor_native_host_trace_json(state: &serde_json::Value) -> serde_json::Value {
    let Some(trace) = state.get("trace") else {
        return serde_json::Value::Null;
    };
    let build_dir = trace
        .get("build_dir")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("");
    let live_refresh = trace
        .get("live_refresh")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));
    let stream_runner = trace
        .get("stream_runner")
        .cloned()
        .unwrap_or_else(|| editor_trace_stream_runner_json(Path::new(""), &live_refresh));
    let frames = editor_native_host_trace_frames_json(trace, build_dir);
    let actions = editor_native_host_trace_actions_json(&frames);
    let action_count = actions.len();
    serde_json::json!({
        "schema_version": 1,
        "kind": "orv.editor.native_host.trace",
        "build_dir": build_dir,
        "trace_path": trace.pointer("/trace/path").cloned().unwrap_or_else(|| serde_json::json!("")),
        "frame_count": trace.pointer("/trace/frame_count").cloned().unwrap_or_else(|| serde_json::json!(0)),
        "status_counts": trace.pointer("/trace/status_counts").cloned().unwrap_or_else(|| serde_json::json!({})),
        "summary": editor_native_host_trace_summary_json(trace),
        "status_filters": editor_native_host_trace_status_filters_json(trace),
        "frames": frames,
        "actions": actions,
        "action_count": action_count,
        "live_refresh": live_refresh,
        "transport": trace.pointer("/live_refresh/transport").cloned().unwrap_or(serde_json::Value::Null),
        "stream_runner": stream_runner,
        "action_runner": editor_trace_action_runner_json(),
        "action_result_artifact": editor_trace_action_result_artifact_json(),
        "panel_html_path": EDITOR_TRACE_PANEL_HTML_PATH,
        "panel_artifact": editor_trace_panel_artifact_json(),
        "panel_contract": editor_native_host_trace_panel_contract_json(),
    })
}

pub(crate) fn editor_native_host_run_action_json(
    host: &Path,
    action_id: &str,
    frame_index: Option<u64>,
    slot: Option<&str>,
) -> anyhow::Result<serde_json::Value> {
    let input = editor_native_host_action_input_path(host);
    let host_value = read_json_value(&input)?;
    validate_editor_native_host_manifest_root(&host_value)?;
    let action = editor_native_host_select_action(&host_value, action_id, frame_index, slot)?;
    let command = action
        .get("command")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| anyhow::anyhow!("native-host action missing command array"))?;
    let (build_dir, origin_id) = editor_native_host_reveal_command_parts(command)?;
    let navigation = editor_reveal_json(Path::new(build_dir), origin_id)?;
    let source = navigation
        .get("source")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let production = navigation
        .get("production")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let result_artifact = editor_trace_action_result_artifact_json();
    let panel = serde_json::json!({
        "schema_version": 1,
        "summary": {
            "status": "passed",
            "action": action.get("action").cloned().unwrap_or_else(|| serde_json::json!(action_id)),
            "slot": action.get("slot").cloned().unwrap_or(serde_json::Value::Null),
            "frame_index": action.get("frame_index").cloned().unwrap_or(serde_json::Value::Null),
            "origin_id": origin_id,
            "target_panel": action.get("target_panel").cloned().unwrap_or(serde_json::Value::Null),
            "source_path": source.get("path").cloned().unwrap_or(serde_json::Value::Null),
            "source_line": source.pointer("/location/line").cloned().unwrap_or(serde_json::Value::Null),
        },
        "action": action,
        "command": command,
        "navigation": navigation,
        "source": source,
        "production": production,
        "result_artifact": result_artifact,
    });
    Ok(serde_json::json!({
        "schema_version": 1,
        "kind": "orv.editor.native_host.action.result",
        "input": input.display().to_string(),
        "execution": {
            "kind": "orv.editor.native_host.action.execution",
            "allowlist": "orv.editor.reveal",
            "status": "passed",
        },
        "action": panel["action"].clone(),
        "command": panel["command"].clone(),
        "navigation": panel["navigation"].clone(),
        "result_artifact": result_artifact,
        "panels": {
            "trace_action": panel,
        },
    }))
}

pub(crate) fn editor_native_host_action_input_path(host: &Path) -> PathBuf {
    if host.is_dir() {
        host.join(EDITOR_NATIVE_HOST_MANIFEST_PATH)
    } else {
        host.to_path_buf()
    }
}

pub(crate) fn validate_editor_native_host_manifest_root(
    host: &serde_json::Value,
) -> anyhow::Result<()> {
    match host.get("kind").and_then(serde_json::Value::as_str) {
        Some("orv.editor.native_host") => {
            if host
                .get("schema_version")
                .and_then(serde_json::Value::as_u64)
                != Some(1)
            {
                anyhow::bail!("native-host manifest schema_version must be 1");
            }
            verify_json_object_keys_exact(
                host,
                &[
                    "schema_version",
                    "kind",
                    "entry",
                    "artifacts",
                    "debug",
                    "runtime",
                    "production",
                    "trace",
                    "host",
                    "panels",
                    "capabilities",
                ],
                "native-host manifest",
            )?;
        }
        Some("orv.editor.native_host.reveal_action") | None => {}
        Some(kind) => anyhow::bail!("native-host manifest kind is invalid: {kind}"),
    }
    Ok(())
}

pub(crate) fn editor_native_host_select_action(
    host: &serde_json::Value,
    action_id: &str,
    frame_index: Option<u64>,
    slot: Option<&str>,
) -> anyhow::Result<serde_json::Value> {
    if host.get("kind").and_then(serde_json::Value::as_str)
        == Some("orv.editor.native_host.reveal_action")
    {
        validate_editor_native_host_reveal_action(host)?;
        return Ok(host.clone());
    }
    let actions = host
        .pointer("/trace/actions")
        .or_else(|| host.get("actions"))
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| anyhow::anyhow!("native-host action input missing trace actions"))?;
    actions
        .iter()
        .find(|action| {
            action.get("action").and_then(serde_json::Value::as_str) == Some(action_id)
                && frame_index.is_none_or(|index| {
                    action
                        .get("frame_index")
                        .and_then(serde_json::Value::as_u64)
                        == Some(index)
                })
                && slot.is_none_or(|slot| {
                    action.get("slot").and_then(serde_json::Value::as_str) == Some(slot)
                })
        })
        .ok_or_else(|| {
            anyhow::anyhow!(
                "native-host action `{action_id}` not found for frame {frame_index:?} slot {slot:?}"
            )
        })
        .and_then(|action| {
            validate_editor_native_host_reveal_action(action)?;
            Ok(action.clone())
        })
}

pub(crate) fn validate_editor_native_host_reveal_action(
    action: &serde_json::Value,
) -> anyhow::Result<()> {
    if action.get("kind").and_then(serde_json::Value::as_str)
        != Some("orv.editor.native_host.reveal_action")
    {
        anyhow::bail!(
            "native-host reveal action kind must be orv.editor.native_host.reveal_action"
        );
    }
    if action
        .get("schema_version")
        .and_then(serde_json::Value::as_u64)
        != Some(1)
    {
        anyhow::bail!("native-host reveal action schema_version must be 1");
    }
    verify_json_object_keys_exact(
        action,
        &[
            "schema_version",
            "kind",
            "action",
            "slot",
            "label",
            "frame_index",
            "origin_id",
            "command",
            "runner_command",
            "focus",
            "target_panel",
            "source",
            "source_path",
            "source_line",
            "production",
            "navigation",
        ],
        "native-host reveal action",
    )?;
    Ok(())
}

pub(crate) fn editor_native_host_reveal_command_parts(
    command: &[serde_json::Value],
) -> anyhow::Result<(&str, &str)> {
    let parts = command
        .iter()
        .map(serde_json::Value::as_str)
        .collect::<Option<Vec<_>>>()
        .ok_or_else(|| anyhow::anyhow!("native-host action command must contain only strings"))?;
    match parts.as_slice() {
        ["orv", "editor", "reveal", build_dir, origin_id] => Ok((build_dir, origin_id)),
        _ => Err(anyhow::anyhow!(
            "unsupported native-host action command; only `orv editor reveal <build-dir> <origin-id>` is allowed"
        )),
    }
}

pub(crate) fn editor_native_host_trace_panel_contract_json() -> serde_json::Value {
    serde_json::json!({
        "schema_version": 1,
        "root": "trace",
        "sections": [
            {
                "name": "summary",
                "path": "trace.summary",
                "kind": "object",
            },
            {
                "name": "status_filters",
                "path": "trace.status_filters",
                "kind": "array",
            },
            {
                "name": "frames",
                "path": "trace.frames",
                "kind": "array",
            },
            {
                "name": "actions",
                "path": "trace.actions",
                "kind": "array",
            },
            {
                "name": "transport",
                "path": "trace.transport",
                "kind": "object",
            },
            {
                "name": "stream_runner",
                "path": "trace.stream_runner",
                "kind": "object",
            },
            {
                "name": "action_runner",
                "path": "trace.action_runner",
                "kind": "object",
            },
            {
                "name": "action_result_artifact",
                "path": "trace.action_result_artifact",
                "kind": "object",
            },
            {
                "name": "panel_artifact",
                "path": "trace.panel_artifact",
                "kind": "object",
            },
        ],
    })
}

pub(crate) fn editor_native_host_trace_summary_json(
    trace: &serde_json::Value,
) -> serde_json::Value {
    let frames = trace
        .get("frames")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    let first_request = frames
        .first()
        .and_then(|frame| frame.get("summary"))
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let last_request = frames
        .last()
        .and_then(|frame| frame.get("summary"))
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    serde_json::json!({
        "schema_version": 1,
        "build_dir": trace
            .get("build_dir")
            .cloned()
            .unwrap_or_else(|| serde_json::json!("")),
        "trace_path": trace
            .pointer("/trace/path")
            .cloned()
            .unwrap_or_else(|| serde_json::json!("")),
        "frame_count": trace
            .pointer("/trace/frame_count")
            .cloned()
            .unwrap_or_else(|| serde_json::json!(frames.len())),
        "status_counts": trace
            .pointer("/trace/status_counts")
            .cloned()
            .unwrap_or_else(|| serde_json::json!({})),
        "first_request": first_request,
        "last_request": last_request,
    })
}

pub(crate) fn editor_native_host_trace_status_filters_json(
    trace: &serde_json::Value,
) -> Vec<serde_json::Value> {
    let counts = trace
        .pointer("/trace/status_counts")
        .unwrap_or(&serde_json::Value::Null);
    [
        ("all", "All", "total"),
        ("ok", "OK", "ok"),
        ("redirect", "3xx", "redirect"),
        ("client_error", "4xx", "client_error"),
        ("server_error", "5xx", "server_error"),
        ("other", "Other", "other"),
    ]
    .into_iter()
    .map(|(name, label, field)| {
        serde_json::json!({
            "name": name,
            "label": label,
            "count": json_usize_field(counts, field),
        })
    })
    .collect()
}

pub(crate) fn editor_native_host_trace_frames_json(
    trace: &serde_json::Value,
    build_dir: &str,
) -> Vec<serde_json::Value> {
    trace
        .get("frames")
        .and_then(serde_json::Value::as_array)
        .map(|frames| {
            frames
                .iter()
                .map(|frame| {
                    let navigation = frame
                        .get("navigation")
                        .cloned()
                        .unwrap_or(serde_json::Value::Null);
                    let response_navigation = frame
                        .get("response_navigation")
                        .cloned()
                        .unwrap_or(serde_json::Value::Null);
                    let db_navigation = frame
                        .get("db_navigation")
                        .cloned()
                        .unwrap_or(serde_json::Value::Null);
                    let commerce_navigation = frame
                        .get("commerce_navigation")
                        .cloned()
                        .unwrap_or(serde_json::Value::Null);
                    let origin_id = frame.get("origin_id").and_then(serde_json::Value::as_str);
                    let response_origin_id = frame
                        .get("response_origin_id")
                        .and_then(serde_json::Value::as_str);
                    let db_operation_origin_id = frame
                        .get("db_operation_origin_id")
                        .and_then(serde_json::Value::as_str);
                    let commerce_adapter_origin_id = frame
                        .get("commerce_adapter_origin_id")
                        .and_then(serde_json::Value::as_str);
                    let index = frame.get("index").cloned().unwrap_or(serde_json::Value::Null);
                    let reveal_command =
                        editor_trace_frame_reveal_command_json(build_dir, origin_id);
                    let response_reveal_command =
                        editor_trace_frame_reveal_command_json(build_dir, response_origin_id);
                    let db_reveal_command =
                        editor_trace_frame_reveal_command_json(build_dir, db_operation_origin_id);
                    let commerce_reveal_command = editor_trace_frame_reveal_command_json(
                        build_dir,
                        commerce_adapter_origin_id,
                    );
                    let actions = editor_native_host_trace_frame_actions_json(
                        &index,
                        build_dir,
                        [
                            (
                                "route",
                                "Reveal route source",
                                origin_id,
                                &navigation,
                                &reveal_command,
                            ),
                            (
                                "response",
                                "Reveal response source",
                                response_origin_id,
                                &response_navigation,
                                &response_reveal_command,
                            ),
                            (
                                "db",
                                "Reveal DB operation source",
                                db_operation_origin_id,
                                &db_navigation,
                                &db_reveal_command,
                            ),
                            (
                                "commerce",
                                "Reveal commerce adapter source",
                                commerce_adapter_origin_id,
                                &commerce_navigation,
                                &commerce_reveal_command,
                            ),
                        ],
                    );
                    serde_json::json!({
                        "index": index,
                        "origin_id": frame.get("origin_id").cloned().unwrap_or(serde_json::Value::Null),
                        "response_origin_id": frame.get("response_origin_id").cloned().unwrap_or(serde_json::Value::Null),
                        "db_operation_origin_id": frame.get("db_operation_origin_id").cloned().unwrap_or(serde_json::Value::Null),
                        "commerce_adapter_origin_id": frame.get("commerce_adapter_origin_id").cloned().unwrap_or(serde_json::Value::Null),
                        "request": frame.get("request").cloned().unwrap_or_else(|| serde_json::json!({})),
                        "summary": frame.get("summary").cloned().unwrap_or_else(|| serde_json::json!({})),
                        "source": navigation
                            .get("source")
                            .cloned()
                            .unwrap_or(serde_json::Value::Null),
                        "production": navigation
                            .get("production")
                            .cloned()
                            .unwrap_or(serde_json::Value::Null),
                        "response_source": response_navigation
                            .get("source")
                            .cloned()
                            .unwrap_or(serde_json::Value::Null),
                        "response_production": response_navigation
                            .get("production")
                            .cloned()
                            .unwrap_or(serde_json::Value::Null),
                        "db_source": db_navigation
                            .get("source")
                            .cloned()
                            .unwrap_or(serde_json::Value::Null),
                        "db_production": db_navigation
                            .get("production")
                            .cloned()
                            .unwrap_or(serde_json::Value::Null),
                        "commerce_source": commerce_navigation
                            .get("source")
                            .cloned()
                            .unwrap_or(serde_json::Value::Null),
                        "commerce_production": commerce_navigation
                            .get("production")
                            .cloned()
                            .unwrap_or(serde_json::Value::Null),
                        "reveal_command": reveal_command,
                        "response_reveal_command": response_reveal_command,
                        "db_reveal_command": db_reveal_command,
                        "commerce_reveal_command": commerce_reveal_command,
                        "actions": actions,
                        "navigation": navigation,
                        "response_navigation": response_navigation,
                        "db_navigation": db_navigation,
                        "commerce_navigation": commerce_navigation,
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

pub(crate) fn editor_native_host_trace_actions_json(
    frames: &[serde_json::Value],
) -> Vec<serde_json::Value> {
    frames
        .iter()
        .flat_map(|frame| {
            frame
                .get("actions")
                .and_then(serde_json::Value::as_array)
                .into_iter()
                .flatten()
                .cloned()
        })
        .collect()
}

pub(crate) fn editor_native_host_trace_frame_actions_json<'a>(
    frame_index: &serde_json::Value,
    build_dir: &str,
    candidates: impl IntoIterator<
        Item = (
            &'a str,
            &'a str,
            Option<&'a str>,
            &'a serde_json::Value,
            &'a serde_json::Value,
        ),
    >,
) -> Vec<serde_json::Value> {
    candidates
        .into_iter()
        .filter_map(|(slot, label, origin_id, navigation, command)| {
            let origin_id = origin_id?;
            if build_dir.is_empty() || navigation.is_null() || command.is_null() {
                return None;
            }
            let action = format!("trace.{slot}.reveal");
            Some(serde_json::json!({
                "schema_version": 1,
                "kind": "orv.editor.native_host.reveal_action",
                "action": action,
                "slot": slot,
                "label": label,
                "frame_index": frame_index,
                "origin_id": origin_id,
                "command": command,
                "runner_command": editor_trace_action_runner_command_json(
                    frame_index,
                    &action,
                    slot,
                ),
                "focus": navigation
                    .get("focus")
                    .cloned()
                    .unwrap_or(serde_json::Value::Null),
                "target_panel": navigation
                    .pointer("/focus/panel")
                    .cloned()
                    .unwrap_or(serde_json::Value::Null),
                "source": navigation
                    .get("source")
                    .cloned()
                    .unwrap_or(serde_json::Value::Null),
                "source_path": navigation
                    .pointer("/source/path")
                    .cloned()
                    .unwrap_or(serde_json::Value::Null),
                "source_line": navigation
                    .pointer("/source/location/line")
                    .cloned()
                    .unwrap_or(serde_json::Value::Null),
                "production": navigation
                    .get("production")
                    .cloned()
                    .unwrap_or(serde_json::Value::Null),
                "navigation": navigation,
            }))
        })
        .collect()
}
