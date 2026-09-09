#![allow(clippy::redundant_pub_crate, clippy::wildcard_imports)]

use super::*;

pub(crate) fn cmd_editor_export(path: &Path, out: &Path) -> anyhow::Result<()> {
    cmd_editor_export_with_options(path, out, None, None)
}

pub(crate) fn cmd_editor_export_with_options(
    path: &Path,
    out: &Path,
    build: Option<&Path>,
    trace: Option<&Path>,
) -> anyhow::Result<()> {
    let entry = project_entry_path(path)?;
    let state = editor_export_state_json_with_trace(&entry, build, trace)?;
    write_json(&out.join("state.json"), &state)?;
    let runner = state
        .pointer("/debug/session_runner")
        .ok_or_else(|| anyhow::anyhow!("editor export state missing debug.session_runner"))?;
    write_json(&out.join(EDITOR_DEBUG_SESSION_RUNNER_PATH), runner)?;
    write_json(
        &out.join(EDITOR_NATIVE_HOST_MANIFEST_PATH),
        &editor_native_host_manifest_json(&entry, &state),
    )?;
    write_text(
        &out.join(EDITOR_NATIVE_HOST_BRIDGE_JS_PATH),
        editor_native_host_bridge_js(),
    )?;
    write_json(
        &out.join(EDITOR_NATIVE_HOST_DESKTOP_PACKAGE_PATH),
        &editor_native_host_desktop_package_json(&entry, &state),
    )?;
    write_editor_native_host_desktop_launcher(out)?;
    write_editor_native_host_desktop_app(out)?;
    write_editor_native_host_desktop_packaging(out)?;
    write_text(&out.join("index.html"), &editor_export_html(&state)?)?;
    let runtime_panel_written = write_editor_runtime_panel_html_if_configured(out, &state)?;
    let production_panel_written = write_editor_production_panel_html_if_configured(out, &state)?;
    let trace_panel_written = write_editor_trace_panel_html_if_configured(out, &state)?;
    let mut files = vec![
        "index.html",
        "state.json",
        EDITOR_DEBUG_SESSION_RUNNER_PATH,
        EDITOR_NATIVE_HOST_MANIFEST_PATH,
        EDITOR_NATIVE_HOST_BRIDGE_JS_PATH,
        EDITOR_NATIVE_HOST_DESKTOP_PACKAGE_PATH,
        EDITOR_NATIVE_HOST_DESKTOP_LAUNCHER_PATH,
        EDITOR_NATIVE_HOST_DESKTOP_PACKAGING_PATH,
        EDITOR_NATIVE_HOST_DESKTOP_PACKAGE_SCRIPT_PATH,
        EDITOR_NATIVE_HOST_DESKTOP_APP_PACKAGE_PATH,
        EDITOR_NATIVE_HOST_DESKTOP_APP_INFO_PLIST_PATH,
        EDITOR_NATIVE_HOST_DESKTOP_APP_ENTITLEMENTS_PATH,
        EDITOR_NATIVE_HOST_DESKTOP_APP_MAIN_PATH,
    ];
    if runtime_panel_written {
        files.push(EDITOR_RUNTIME_PANEL_HTML_PATH);
    }
    if production_panel_written {
        files.push(EDITOR_PRODUCTION_PANEL_HTML_PATH);
    }
    if trace_panel_written {
        files.push(EDITOR_TRACE_PANEL_HTML_PATH);
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "schema_version": 1,
            "kind": "orv.editor.export",
            "entry": entry.display().to_string(),
            "out": out.display().to_string(),
            "files": files,
        }))?
    );
    Ok(())
}

pub(crate) fn verify_editor_export_state_contract_keys(
    state: &serde_json::Value,
) -> anyhow::Result<()> {
    verify_json_object_keys_allowing_optional(
        state,
        &["schema_version", "kind", "snapshot", "runtime", "debug"],
        &["production", "trace"],
        "editor export state",
    )?;
    if state.get("trace").is_some() && state.get("production").is_none() {
        anyhow::bail!("editor export state trace requires production context");
    }
    verify_editor_export_debug_contract_keys(
        state
            .get("debug")
            .ok_or_else(|| anyhow::anyhow!("editor export state debug must be an object"))?,
    )
}

pub(crate) fn editor_export_state_json(path: &Path) -> anyhow::Result<serde_json::Value> {
    Ok(serde_json::json!({
        "schema_version": 1,
        "kind": "orv.editor.export",
        "snapshot": editor_snapshot_json(path)?,
        "runtime": editor_runtime_json(path)?,
        "debug": editor_debug_json(path)?,
    }))
}

pub(crate) fn write_editor_runtime_panel_html_if_configured(
    out: &Path,
    state: &serde_json::Value,
) -> anyhow::Result<bool> {
    if state.get("runtime").is_none() {
        return Ok(false);
    }
    let runtime = editor_native_host_runtime_json(state);
    let html = editor_runtime_panel_html(&runtime)?;
    write_text(&out.join(EDITOR_RUNTIME_PANEL_HTML_PATH), &html)?;
    Ok(true)
}

pub(crate) fn editor_runtime_panel_html(runtime: &serde_json::Value) -> anyhow::Result<String> {
    let status = html_escape_text(
        runtime
            .get("status")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("unknown"),
    );
    let stdout = html_escape_text(
        runtime
            .get("stdout")
            .and_then(serde_json::Value::as_str)
            .unwrap_or(""),
    );
    let error = html_escape_text(
        runtime
            .get("error")
            .and_then(serde_json::Value::as_str)
            .unwrap_or(""),
    );
    let frame_count = json_usize_field(runtime, "frame_count");
    let async_json = html_escape_text(&serde_json::to_string_pretty(
        runtime.get("async").unwrap_or(&serde_json::Value::Null),
    )?);
    let panel_json = html_escape_text(&serde_json::to_string_pretty(
        runtime.get("panel").unwrap_or(&serde_json::Value::Null),
    )?);
    let panel_contract_json = html_escape_text(&serde_json::to_string_pretty(
        runtime
            .get("panel_contract")
            .unwrap_or(&serde_json::Value::Null),
    )?);
    let runtime_json = html_script_json(&serde_json::to_string_pretty(runtime)?);
    let mut html = String::new();
    html.push_str(
        "<!doctype html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n<title>orv Runtime Panel</title>\n<style>\n:root{color-scheme:light dark;--bg:#f8f7f3;--fg:#151713;--muted:#697067;--panel:#fff;--line:#d8d9d2;--accent:#375f94;--accent-weak:#dde8f5;--bad:#a43737;}\n@media (prefers-color-scheme: dark){:root{--bg:#11130f;--fg:#eef0ea;--muted:#a8aea2;--panel:#191c17;--line:#30362d;--accent:#8ab8f0;--accent-weak:#1e314a;--bad:#ff9d9d;}}\n*{box-sizing:border-box}body{margin:0;background:var(--bg);color:var(--fg);font:14px/1.45 ui-sans-serif,system-ui,-apple-system,BlinkMacSystemFont,\"Segoe UI\",sans-serif;}header{padding:24px 28px 12px;border-bottom:1px solid var(--line);}h1{font-size:24px;margin:0 0 12px}h2{font-size:13px;text-transform:uppercase;letter-spacing:.08em;color:var(--muted);margin:0 0 12px}.summary{display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:10px}.metric{border:1px solid var(--line);border-radius:6px;padding:10px;background:var(--panel)}.metric b{display:block;font-size:22px;line-height:1.1}.ok{color:var(--accent)}.err{color:var(--bad)}main{display:grid;grid-template-columns:minmax(280px,380px) minmax(0,1fr);gap:16px;padding:16px 28px 28px}.panel{background:var(--panel);border:1px solid var(--line);border-radius:8px;padding:16px}.list{list-style:none;margin:0;padding:0;display:grid;gap:8px}.list li{border:1px solid var(--line);border-radius:6px;padding:10px;cursor:pointer;background:var(--bg)}.list li:focus,.list li:hover{outline:2px solid var(--accent);outline-offset:1px}pre{margin:0;white-space:pre-wrap;overflow:auto;font:12px/1.45 ui-monospace,SFMono-Regular,Menlo,Consolas,monospace}.detail-grid{display:grid;grid-template-columns:1fr 1fr;gap:16px}.wide{grid-column:1/-1}@media (max-width:900px){main,.summary,.detail-grid{grid-template-columns:1fr}main{padding:14px}header{padding:18px 14px 8px}}\n</style>\n</head>\n<body>\n",
    );
    writeln!(
        &mut html,
        "<header><h1>Runtime Panel</h1><section class=\"summary\"><div class=\"metric\"><span>Status</span><b class=\"{}\">{status}</b></div><div class=\"metric\"><span>Frames</span><b>{frame_count}</b></div></section></header>",
        if status == "ok" { "ok" } else { "err" }
    )?;
    html.push_str("<main><section class=\"panel\"><h2>Frames</h2><ul id=\"runtime-frame-list\" class=\"list\"></ul></section><section class=\"panel\"><h2>Selected Frame</h2><pre id=\"runtime-frame-detail\">No runtime frame selected.</pre></section><section class=\"detail-grid\">\n");
    writeln!(
        &mut html,
        "<section class=\"panel\"><h2>Stdout</h2><pre>{stdout}</pre></section><section class=\"panel\"><h2>Error</h2><pre>{error}</pre></section><section class=\"panel\"><h2>Async Runtime</h2><pre>{async_json}</pre></section><section class=\"panel\"><h2>Runtime Panel</h2><pre>{panel_json}</pre></section><section class=\"panel wide\"><h2>Panel Contract</h2><pre>{panel_contract_json}</pre></section></section></main>"
    )?;
    writeln!(
        &mut html,
        "<script id=\"orv-runtime\" type=\"application/json\">{runtime_json}</script>"
    )?;
    html.push_str(
        "<script>\nconst runtime = JSON.parse(document.getElementById('orv-runtime').textContent);\nconst frames = Array.isArray(runtime.frames) ? runtime.frames : [];\nconst list = document.getElementById('runtime-frame-list');\nconst detail = document.getElementById('runtime-frame-detail');\nfunction frameLabel(frame){\n  const source = frame?.source || {};\n  const label = source.name || source.path || 'frame';\n  const line = frame?.line ? `:${frame.line}` : '';\n  return `#${(frame?.index ?? 0) + 1} ${label}${line}`;\n}\nfunction renderDetail(frame){\n  if (!frame) { detail.textContent = 'No runtime frame selected.'; return; }\n  const source = frame.source || {};\n  const locals = (frame.locals || []).map(local => `  ${local.name}: ${local.value}${local.type ? ` (${local.type})` : ''}`);\n  const stack = (frame.stack || []).map(call => `  ${call.name || 'frame'} ${call.source?.name || call.source?.path || ''}:${call.line || ''}`.trim());\n  const lines = [\n    frameLabel(frame),\n    source.path ? `source ${source.path}${frame.line ? `:${frame.line}` : ''}` : '',\n    frame.output ? `output ${String(frame.output).trimEnd()}` : '',\n    locals.length ? `locals\\n${locals.join('\\n')}` : '',\n    stack.length ? `stack\\n${stack.join('\\n')}` : ''\n  ].filter(Boolean);\n  detail.textContent = lines.join('\\n');\n}\nfor (const frame of frames) {\n  const row = document.createElement('li');\n  row.textContent = frameLabel(frame);\n  row.tabIndex = 0;\n  row.addEventListener('click', () => renderDetail(frame));\n  row.addEventListener('keydown', event => {\n    if (event.key === 'Enter' || event.key === ' ') { event.preventDefault(); renderDetail(frame); }\n  });\n  list.appendChild(row);\n}\nrenderDetail(frames[0]);\n</script>\n</body>\n</html>\n",
    );
    Ok(html)
}

pub(crate) fn html_script_json(value: &str) -> String {
    value.replace('&', "\\u0026").replace('<', "\\u003c")
}

pub(crate) struct EditorGraphPanel {
    pub(crate) node_count: usize,
    pub(crate) edge_count: usize,
    pub(crate) source_depth: usize,
    pub(crate) semantic_depth: usize,
    pub(crate) svg: String,
}

pub(crate) fn editor_graph_panel_from_state(state: &serde_json::Value) -> EditorGraphPanel {
    let graph_stats = state
        .pointer("/snapshot/project_graph/stats")
        .unwrap_or(&serde_json::Value::Null);
    let graph_nodes = state
        .pointer("/snapshot/project_graph/nodes")
        .and_then(serde_json::Value::as_array)
        .map_or(&[][..], Vec::as_slice);
    let graph_edges = state
        .pointer("/snapshot/project_graph/edges")
        .and_then(serde_json::Value::as_array)
        .map_or(&[][..], Vec::as_slice);
    EditorGraphPanel {
        node_count: json_usize_field(graph_stats, "node_count"),
        edge_count: json_usize_field(graph_stats, "edge_count"),
        source_depth: json_usize_field(graph_stats, "max_source_contains_depth"),
        semantic_depth: json_usize_field(graph_stats, "max_semantic_contains_depth"),
        svg: project_graph_view_svg(graph_nodes, graph_edges),
    }
}

pub(crate) fn write_editor_graph_panel_html(
    html: &mut String,
    graph: &EditorGraphPanel,
) -> anyhow::Result<()> {
    write!(
        html,
        "<section class=\"panel graph-panel\"><h2>Project Graph</h2><div class=\"metric\">{}</div><p class=\"muted\">{} edges, source depth {}, semantic depth {}.</p><div id=\"editor-graph-view\" class=\"graph-view\">{}</div></section>",
        graph.node_count, graph.edge_count, graph.source_depth, graph.semantic_depth, graph.svg
    )?;
    Ok(())
}

pub(crate) fn editor_export_html(state: &serde_json::Value) -> anyhow::Result<String> {
    let entry = state
        .pointer("/snapshot/entry/path")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("app.orv");
    let file_count = json_array_count(state.pointer("/snapshot/panels/files"));
    let route_count = json_array_count(state.pointer("/snapshot/panels/routes"));
    let schema_count = json_array_count(state.pointer("/snapshot/panels/schema"));
    let domain_count = json_array_count(state.pointer("/snapshot/panels/domains"));
    let diagnostic_count = json_array_count(state.pointer("/snapshot/diagnostics"));
    let graph_panel = editor_graph_panel_from_state(state);
    let runtime_frame_count = json_array_count(state.pointer("/runtime/frames"));
    let debug_config_count = json_array_count(state.pointer("/debug/configurations"));
    let debug_control_count = json_array_count(state.pointer("/debug/controls"));
    let debug_capability_count = editor_debug_capability_count_from_state(state);
    let debug_breakpoint_count = editor_debug_breakpoint_count_from_state(state);
    let debug_function_breakpoint_count = editor_debug_function_breakpoint_count_from_state(state);
    let debug_data_breakpoint_count = editor_debug_data_breakpoint_count_from_state(state);
    let debug_exception_filter_count = editor_debug_exception_filter_count_from_state(state);
    let production_client_target_count = json_array_count(state.pointer("/production/client"));
    let production_native_server_target_count =
        json_array_count(state.pointer("/production/native_server"));
    let production_static_target_count = json_array_count(state.pointer("/production/static"));
    let production_preflight_count = json_array_count(state.pointer("/production/preflight"));
    let production_db_adapter_count = json_array_count(state.pointer("/production/db_adapters"));
    let production_commerce_adapter_count =
        json_array_count(state.pointer("/production/commerce_adapters"));
    let production_summary = editor_production_summary_text(state);
    let trace_count = json_array_count(state.pointer("/trace/frames"));
    let trace_status_counts = editor_trace_status_counts_from_state(state);
    let runtime_status = state
        .pointer("/runtime/runtime/status")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("unknown");
    let stdout = state
        .pointer("/runtime/runtime/stdout")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("");
    let state_json = serde_json::to_string(state)?.replace("</", "<\\/");
    let mut html = String::new();
    html.push_str("<!doctype html>\n<html lang=\"en\">\n<head>\n");
    html.push_str("<meta charset=\"utf-8\">\n");
    html.push_str("<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n");
    html.push_str("<title>orv editor</title>\n");
    html.push_str("<style>\n");
    html.push_str(":root{color-scheme:light;--bg:#f7f8fb;--ink:#18202f;--muted:#687386;--line:#d7dce5;--panel:#ffffff;--accent:#0f766e;--warn:#b45309;}\n");
    html.push_str("*{box-sizing:border-box}body{margin:0;background:var(--bg);color:var(--ink);font:14px/1.45 ui-sans-serif,system-ui,-apple-system,BlinkMacSystemFont,\"Segoe UI\",sans-serif}#orv-editor{min-height:100vh;display:grid;grid-template-columns:240px 1fr;grid-template-rows:auto 1fr}.sidebar{grid-row:1/3;border-right:1px solid var(--line);background:#111827;color:#f8fafc;padding:20px 16px}.brand{font-weight:700;font-size:18px;margin-bottom:18px}.nav{display:grid;gap:8px}.nav span{display:flex;justify-content:space-between;border:1px solid #334155;padding:8px 10px}.topbar{border-bottom:1px solid var(--line);background:var(--panel);padding:14px 20px}.topbar h1{font-size:18px;margin:0}.topbar p{margin:4px 0 0;color:var(--muted)}.workspace{padding:18px 20px;display:grid;gap:14px;grid-template-columns:repeat(2,minmax(0,1fr))}.panel{border:1px solid var(--line);background:var(--panel);border-radius:8px;padding:14px;min-height:132px}.panel h2{font-size:14px;margin:0 0 10px}.metric{font-size:28px;font-weight:700}.muted{color:var(--muted)}.list{list-style:none;margin:10px 0 0;padding:0;display:grid;gap:6px}.list li{border-top:1px solid var(--line);padding-top:6px;color:var(--muted);word-break:break-word;cursor:pointer}.list li:focus{outline:2px solid var(--accent);outline-offset:2px}.filterbar{display:flex;flex-wrap:wrap;gap:6px;margin:10px 0}.filterbar button{border:1px solid var(--line);background:#f8fafc;color:var(--ink);padding:5px 8px;font:inherit;cursor:pointer}.filterbar button[aria-pressed=\"true\"]{border-color:var(--accent);color:var(--accent);font-weight:700}.detail{min-height:120px}pre{white-space:pre-wrap;word-break:break-word;margin:0;max-height:240px;overflow:auto;background:#f1f5f9;border:1px solid var(--line);padding:10px}@media(max-width:760px){#orv-editor{display:block}.sidebar{border-right:0}.workspace{grid-template-columns:1fr}}\n");
    html.push_str(".graph-panel{grid-column:1/-1}.graph-view{overflow:auto;border:1px solid var(--line);background:#fff}.graph-view svg{display:block;min-width:900px}\n");
    html.push_str("</style>\n</head>\n<body>\n");
    html.push_str("<main id=\"orv-editor\">\n");
    html.push_str(
        "<aside class=\"sidebar\"><div class=\"brand\">orv editor</div><nav class=\"nav\">",
    );
    write!(&mut html, "<span>Files<b>{file_count}</b></span>")?;
    write!(&mut html, "<span>Routes<b>{route_count}</b></span>")?;
    write!(&mut html, "<span>Schema<b>{schema_count}</b></span>")?;
    write!(&mut html, "<span>Domains<b>{domain_count}</b></span>")?;
    write!(
        &mut html,
        "<span>Graph<b>{}</b></span>",
        graph_panel.node_count
    )?;
    write!(
        &mut html,
        "<span>Runtime Frames<b>{runtime_frame_count}</b></span>"
    )?;
    write!(&mut html, "<span>Debug<b>{debug_config_count}</b></span>")?;
    write!(
        &mut html,
        "<span>Debug Controls<b>{debug_control_count}</b></span>"
    )?;
    write!(
        &mut html,
        "<span>DAP Caps<b>{debug_capability_count}</b></span>"
    )?;
    write!(
        &mut html,
        "<span>Production<b>{}</b></span>",
        production_client_target_count
            + production_native_server_target_count
            + production_static_target_count
            + production_preflight_count
            + production_db_adapter_count
            + production_commerce_adapter_count
    )?;
    write!(&mut html, "<span>Trace<b>{trace_count}</b></span>")?;
    html.push_str("</nav></aside>\n");
    html.push_str("<header class=\"topbar\">");
    write!(
        &mut html,
        "<h1>{}</h1><p>First-party editor export backed by shared ProjectGraph.</p>",
        html_escape_text(entry)
    )?;
    html.push_str("</header>\n<section class=\"workspace\">\n");
    write!(
        &mut html,
        "<section class=\"panel\"><h2>Routes</h2><div class=\"metric\">{route_count}</div><p class=\"muted\">Graph-backed route panel entries.</p><ul id=\"routes-list\" class=\"list\"></ul></section>"
    )?;
    write!(
        &mut html,
        "<section class=\"panel\"><h2>Schema</h2><div class=\"metric\">{schema_count}</div><p class=\"muted\">Struct, enum, and type alias nodes.</p><ul id=\"schema-list\" class=\"list\"></ul></section>"
    )?;
    write!(
        &mut html,
        "<section class=\"panel\"><h2>Domains</h2><div class=\"metric\">{domain_count}</div><p class=\"muted\">Project domain and define nodes.</p><ul id=\"domains-list\" class=\"list\"></ul></section>"
    )?;
    write!(
        &mut html,
        "<section class=\"panel\"><h2>Diagnostics</h2><div class=\"metric\">{diagnostic_count}</div><p class=\"muted\">Project loader, resolver, and analyzer diagnostics.</p></section>"
    )?;
    write!(
        &mut html,
        "<section class=\"panel\"><h2>Debug</h2><div class=\"metric\">{debug_config_count}</div><p class=\"muted\">DAP launch and attach configurations.</p><ul id=\"debug-config-list\" class=\"list\"></ul></section>"
    )?;
    write!(
        &mut html,
        "<section class=\"panel\"><h2>Debug Controls</h2><div class=\"metric\">{debug_control_count}</div><p class=\"muted\">DAP live-control request payloads.</p><ul id=\"debug-control-list\" class=\"list\"></ul></section>"
    )?;
    write!(
        &mut html,
        "<section class=\"panel\"><h2>DAP Capabilities</h2><div class=\"metric\">{debug_capability_count}</div><p class=\"muted\">Adapter features exposed to native hosts.</p><ul id=\"debug-capability-list\" class=\"list\"></ul></section>"
    )?;
    write!(
        &mut html,
        "<section class=\"panel\"><h2>Breakpoints</h2><div class=\"metric\">{debug_breakpoint_count}</div><p class=\"muted\">Executable source lines for DAP setBreakpoints.</p><ul id=\"debug-breakpoint-list\" class=\"list\"></ul></section>"
    )?;
    write!(
        &mut html,
        "<section class=\"panel\"><h2>Function Breakpoints</h2><div class=\"metric\">{debug_function_breakpoint_count}</div><p class=\"muted\">Named functions for DAP setFunctionBreakpoints.</p><ul id=\"debug-function-breakpoint-list\" class=\"list\"></ul></section>"
    )?;
    write!(
        &mut html,
        "<section class=\"panel\"><h2>Data Breakpoints</h2><div class=\"metric\">{debug_data_breakpoint_count}</div><p class=\"muted\">Local variables for DAP setDataBreakpoints.</p><ul id=\"debug-data-breakpoint-list\" class=\"list\"></ul></section>"
    )?;
    write!(
        &mut html,
        "<section class=\"panel\"><h2>Exception Filters</h2><div class=\"metric\">{debug_exception_filter_count}</div><p class=\"muted\">DAP exception filter presets.</p><ul id=\"debug-exception-filter-list\" class=\"list\"></ul></section>"
    )?;
    write_editor_graph_panel_html(&mut html, &graph_panel)?;
    html.push_str("<section class=\"panel\"><h2>Debug Runner</h2><pre id=\"debug-runner-detail\" class=\"detail\"></pre></section>");
    html.push_str("<section class=\"panel\"><h2>Debug Result</h2><pre id=\"debug-result-detail\" class=\"detail\"></pre></section>");
    html.push_str("<section class=\"panel\"><h2>Runner Command</h2><pre id=\"debug-control-command\" class=\"detail\"></pre></section>");
    html.push_str("<section class=\"panel\"><h2>Selected Debug</h2><pre id=\"debug-detail\" class=\"detail\"></pre></section>");
    write!(
        &mut html,
        "<section class=\"panel\"><h2>Production</h2><div class=\"metric\">{}</div><p class=\"muted\">Client Bundles {production_client_target_count} · Preflight {production_preflight_count} · DB Adapters {production_db_adapter_count} · Commerce Adapters {production_commerce_adapter_count}</p><pre>{}</pre></section>",
        production_client_target_count
            + production_preflight_count
            + production_db_adapter_count
            + production_commerce_adapter_count,
        html_escape_text(&production_summary)
    )?;
    write_trace_panel_html(&mut html, trace_count, &trace_status_counts)?;
    html.push_str("<section class=\"panel\"><h2>Selected Trace</h2><pre id=\"trace-detail\" class=\"detail\"></pre></section>");
    html.push_str("<section class=\"panel\"><h2>Trace Reveal Actions</h2><ul id=\"trace-action-list\" class=\"list\"></ul><pre id=\"trace-action-detail\" class=\"detail\"></pre></section>");
    html.push_str("<section class=\"panel\"><h2>Trace Transport</h2><pre id=\"trace-transport-detail\" class=\"detail\"></pre></section>");
    html.push_str("<section class=\"panel\"><h2>Trace Stream Runner</h2><pre id=\"trace-stream-runner-detail\" class=\"detail\"></pre></section>");
    html.push_str("<section class=\"panel\"><h2>Runtime</h2>");
    write!(
        &mut html,
        "<div class=\"metric\">{}</div><p class=\"muted\">Reference runtime status.</p><pre>{}</pre>",
        html_escape_text(runtime_status),
        html_escape_text(stdout)
    )?;
    html.push_str("</section>\n");
    write!(
        &mut html,
        "<section class=\"panel\"><h2>Runtime Frames</h2><div class=\"metric\">{runtime_frame_count}</div><ul id=\"runtime-frame-list\" class=\"list\"></ul></section>"
    )?;
    html.push_str("<section class=\"panel\"><h2>Selected Runtime</h2><pre id=\"runtime-frame-detail\" class=\"detail\"></pre></section>");
    html.push_str("</section>\n");
    html.push_str("</main>\n");
    writeln!(
        &mut html,
        "<script src=\"{EDITOR_NATIVE_HOST_BRIDGE_JS_PATH}\"></script>"
    )?;
    html.push_str("<script id=\"orv-editor-state\" type=\"application/json\">");
    html.push_str(&state_json);
    html.push_str("</script>\n");
    html.push_str(
        "<script>\nfunction renderTraceDetail(frame){\n  const target = document.getElementById('trace-detail');\n  if (!target) return;\n  if (!frame) {\n    target.textContent = 'No trace frame selected.';\n    return;\n  }\n  const request = frame.request || {};\n  const summary = frame.summary || {};\n  const navigation = frame.navigation || {};\n  const source = navigation.source || {};\n  const location = source.location || {};\n  const params = request.params && Object.keys(request.params).length ? `params ${JSON.stringify(request.params)}` : '';\n  const query = request.query && Object.keys(request.query).length ? `query ${JSON.stringify(request.query)}` : '';\n  const body = request.body ? `body ${request.body}` : '';\n  const lines = [\n    summary.label || `${request.method || ''} ${request.path || ''}`.trim(),\n    summary.route ? `route ${summary.route}` : '',\n    summary.status_class ? `status ${summary.status_class}` : '',\n    frame.origin_id ? `origin ${frame.origin_id}` : '',\n    params,\n    query,\n    body,\n    source.path || location.uri || '',\n    source.snippet || ''\n  ].filter(Boolean);\n  target.textContent = lines.join('\\n');\n}\nfunction renderRuntimeDetail(frame){\n  const target = document.getElementById('runtime-frame-detail');\n  if (!target) return;\n  if (!frame) {\n    target.textContent = 'No runtime frame selected.';\n    return;\n  }\n  const source = frame.source || {};\n  const locals = (frame.locals || []).map(local => `  ${local.name}: ${local.value}${local.type ? ` (${local.type})` : ''}`);\n  const stack = (frame.stack || []).map(call => `  ${call.name || 'frame'} ${call.source?.name || call.source?.path || ''}:${call.line || ''}`.trim());\n  const output = frame.output ? `output ${String(frame.output).trimEnd()}` : '';\n  const lines = [\n    `frame #${(frame.index ?? 0) + 1}`,\n    source.path ? `source ${source.path}:${frame.line || ''}` : (frame.line ? `line ${frame.line}` : ''),\n    output,\n    locals.length ? `locals\\n${locals.join('\\n')}` : '',\n    stack.length ? `stack\\n${stack.join('\\n')}` : ''\n  ].filter(Boolean);\n  target.textContent = lines.join('\\n');\n}\nfunction renderDebugDetail(value){\n  const target = document.getElementById('debug-detail');\n  if (!target) return;\n  if (!value) {\n    target.textContent = 'No debug item selected.';\n    return;\n  }\n  target.textContent = JSON.stringify(value, null, 2);\n}\nfunction renderDebugRunner(runner){\n  const target = document.getElementById('debug-runner-detail');\n  if (!target) return;\n  target.textContent = runner ? JSON.stringify(runner, null, 2) : 'No debug runner.';\n}\nfunction renderDebugResultArtifact(result){\n  const target = document.getElementById('debug-result-detail');\n  if (!target) return;\n  if (!result) {\n    target.textContent = 'No debug result artifact.';\n    return;\n  }\n  const panels = Array.isArray(result.panels) ? result.panels.join(', ') : '';\n  target.textContent = [result.kind, result.path, result.media_type, panels ? `panels ${panels}` : ''].filter(Boolean).join('\\n');\n}\nfunction debugBreakpointRows(state){\n  const rows = [];\n  for (const group of state.debug?.breakpoint_sources || []) {\n    const breakpoints = group.breakpoints || (group.lines || []).map(line => ({line}));\n    for (const breakpoint of breakpoints) {\n      rows.push({...breakpoint, source: group.source || {}, line: breakpoint.line});\n    }\n  }\n  return rows;\n}\nfunction filterTraceFrames(frames, filter){\n  if (filter === 'all') return frames;\n  return frames.filter(frame => frame.summary?.status_class === filter);\n}\nfunction renderTraceTransport(state){\n  const target = document.getElementById('trace-transport-detail');\n  if (!target) return;\n  const transport = state.trace?.live_refresh?.transport;\n  if (!transport) {\n    target.textContent = 'No trace transport.';\n    return;\n  }\n  target.textContent = [transport.kind, transport.event, transport.url].filter(Boolean).join('\\n');\n}\nfunction renderTraceStreamRunner(state){\n  const target = document.getElementById('trace-stream-runner-detail');\n  if (!target) return;\n  const runner = state.trace?.stream_runner;\n  if (!runner) {\n    target.textContent = 'No trace stream runner.';\n    return;\n  }\n  const command = Array.isArray(runner.command) ? runner.command.join(' ') : '';\n  target.textContent = [runner.kind, runner.event_stream, command].filter(Boolean).join('\\n');\n}\nfunction renderEditorState(){\n  const state = JSON.parse(document.getElementById('orv-editor-state').textContent);\n  const put = (id, items, label, onPick) => {\n    const target = document.getElementById(id);\n    if (!target) return;\n    target.textContent = '';\n    for (const item of items || []) {\n      const row = document.createElement('li');\n      row.textContent = label(item);\n      if (onPick) {\n        row.tabIndex = 0;\n        row.addEventListener('click', () => onPick(item));\n        row.addEventListener('keydown', event => {\n          if (event.key === 'Enter' || event.key === ' ') {\n            event.preventDefault();\n            onPick(item);\n          }\n        });\n      }\n      target.appendChild(row);\n    }\n  };\n  put('routes-list', state.snapshot?.panels?.routes, item => `${item.method || ''} ${item.path || item.name || ''}`.trim() || item.origin_id || 'route');\n  put('schema-list', state.snapshot?.panels?.schema, item => item.name || item.kind || 'schema');\n  put('domains-list', state.snapshot?.panels?.domains, item => item.name || item.kind || 'domain');\n  const debugConfigs = state.debug?.configurations || [];\n  put('debug-config-list', debugConfigs, item => item.name || item.request || 'debug', renderDebugDetail);\n  const debugBreakpoints = debugBreakpointRows(state);\n  put('debug-breakpoint-list', debugBreakpoints, breakpoint => {\n    const source = breakpoint.source || {};\n    return `${source.name || source.path || 'source'}:${breakpoint.line}`;\n  }, breakpoint => {\n    const request = breakpoint.request || {\n      command: 'setBreakpoints',\n      arguments: {source: breakpoint.source, breakpoints: [{line: breakpoint.line}]}\n    };\n    renderDebugControlCommand({runner_command: breakpoint.runner_command || []});\n    renderDebugDetail({request, runner_command: breakpoint.runner_command || []});\n  });\n  renderDebugRunner(state.debug?.session_runner);\n  renderDebugResultArtifact(state.debug?.result_artifact || state.debug?.session_runner?.result);\n  renderDebugDetail(debugConfigs[0]);\n  const runtimeFrames = state.runtime?.frames || [];\n  put('runtime-frame-list', runtimeFrames, frame => {\n    const source = frame.source || {};\n    const label = source.name || source.path || 'frame';\n    const line = frame.line ? `:${frame.line}` : '';\n    return `#${(frame.index ?? 0) + 1} ${label}${line}`;\n  }, renderRuntimeDetail);\n  renderRuntimeDetail(runtimeFrames[0]);\n  const traceFrames = state.trace?.frames || [];\n  const traceButtons = Array.from(document.querySelectorAll('[data-trace-filter]'));\n  const renderTraceList = filter => {\n    const frames = filterTraceFrames(traceFrames, filter);\n    put('trace-list', frames, frame => frame.summary?.label || frame.origin_id || 'request', renderTraceDetail);\n    renderTraceDetail(frames[0]);\n  };\n  for (const button of traceButtons) {\n    button.addEventListener('click', () => {\n      for (const item of traceButtons) item.setAttribute('aria-pressed', 'false');\n      button.setAttribute('aria-pressed', 'true');\n      renderTraceList(button.dataset.traceFilter || 'all');\n    });\n  }\n  renderTraceList('all');\n  renderTraceTransport(state);\n  renderTraceStreamRunner(state);\n}\nrenderEditorState();\n</script>\n",
    );
    html.push_str(
        "<script>\nfunction traceRevealCommandText(action){\n  const command = action?.command || [];\n  return Array.isArray(command) ? command.join(' ') : JSON.stringify(command);\n}\nfunction traceRevealActionText(action){\n  if (!action) return 'No trace reveal action selected.';\n  const source = action.source || {};\n  const location = source.location || {};\n  const production = action.production || {};\n  return [\n    action.label || action.action || 'Reveal source',\n    action.slot ? `slot ${action.slot}` : '',\n    action.origin_id ? `origin ${action.origin_id}` : '',\n    traceRevealCommandText(action) ? `command ${traceRevealCommandText(action)}` : '',\n    action.target_panel ? `panel ${action.target_panel}` : '',\n    source.path ? `source ${source.path}${location.line ? `:${location.line}` : ''}` : '',\n    production.path ? `production ${production.path}` : '',\n    source.snippet || ''\n  ].filter(Boolean).join('\\n');\n}\nfunction runTraceRevealAction(action){\n  const detail = document.getElementById('trace-action-detail');\n  if (detail) detail.textContent = traceRevealActionText(action);\n  const payload = {action, command: action?.command || []};\n  if (window.orvNativeHost && typeof window.orvNativeHost.runAction === 'function') {\n    window.orvNativeHost.runAction(action);\n  }\n  window.dispatchEvent(new CustomEvent('orv:trace-reveal-action', {detail: payload}));\n}\nfunction renderTraceActions(frame){\n  const target = document.getElementById('trace-action-list');\n  const detail = document.getElementById('trace-action-detail');\n  if (!target) return;\n  target.textContent = '';\n  const actions = Array.isArray(frame?.actions) ? frame.actions : [];\n  if (!actions.length) {\n    if (detail) detail.textContent = 'No reveal action for selected trace frame.';\n    return;\n  }\n  for (const action of actions) {\n    const row = document.createElement('li');\n    row.textContent = `${action.label || action.action || 'Reveal'} ${action.origin_id || ''}`.trim();\n    row.tabIndex = 0;\n    row.dataset.traceRevealAction = action.action || '';\n    row.addEventListener('click', () => runTraceRevealAction(action));\n    row.addEventListener('keydown', event => {\n      if (event.key === 'Enter' || event.key === ' ') {\n        event.preventDefault();\n        runTraceRevealAction(action);\n      }\n    });\n    target.appendChild(row);\n  }\n  if (detail) detail.textContent = traceRevealActionText(actions[0]);\n}\nrenderTraceDetail = function(frame){\n  const target = document.getElementById('trace-detail');\n  if (!target) return;\n  if (!frame) {\n    target.textContent = 'No trace frame selected.';\n    renderTraceActions(null);\n    return;\n  }\n  const request = frame.request || {};\n  const summary = frame.summary || {};\n  const navigation = frame.navigation || {};\n  const source = navigation.source || frame.source || {};\n  const location = source.location || {};\n  const params = request.params && Object.keys(request.params).length ? `params ${JSON.stringify(request.params)}` : '';\n  const query = request.query && Object.keys(request.query).length ? `query ${JSON.stringify(request.query)}` : '';\n  const body = request.body ? `body ${request.body}` : '';\n  const actions = Array.isArray(frame.actions) ? frame.actions : [];\n  const lines = [\n    summary.label || `${request.method || ''} ${request.path || ''}`.trim(),\n    summary.route ? `route ${summary.route}` : '',\n    summary.status_class ? `status ${summary.status_class}` : '',\n    frame.origin_id ? `origin ${frame.origin_id}` : '',\n    actions.length ? `actions ${actions.length}` : '',\n    params,\n    query,\n    body,\n    source.path || location.uri || '',\n    source.snippet || ''\n  ].filter(Boolean);\n  target.textContent = lines.join('\\n');\n  renderTraceActions(frame);\n};\nrenderEditorState();\n</script>\n",
    );
    html.push_str(
        "<script>\nfunction renderDebugControlCommand(control){\n  const target = document.getElementById('debug-control-command');\n  if (!target) return;\n  const command = control?.runner_command || control?.command || [];\n  target.textContent = Array.isArray(command) ? command.join(' ') : JSON.stringify(command, null, 2);\n}\nfunction renderDebugControls(){\n  const stateNode = document.getElementById('orv-editor-state');\n  const target = document.getElementById('debug-control-list');\n  if (!stateNode || !target) return;\n  const state = JSON.parse(stateNode.textContent);\n  target.textContent = '';\n  const controls = state.debug?.controls || [];\n  for (const control of controls) {\n    const row = document.createElement('li');\n    row.textContent = control.name || control.request?.command || 'control';\n    row.tabIndex = 0;\n    const show = () => {\n      renderDebugControlCommand(control);\n      renderDebugDetail(control.request || control);\n    };\n    row.addEventListener('click', show);\n    row.addEventListener('keydown', event => {\n      if (event.key === 'Enter' || event.key === ' ') {\n        event.preventDefault();\n        show();\n      }\n    });\n    target.appendChild(row);\n  }\n  if (controls.length) renderDebugControlCommand(controls[0]);\n}\nrenderDebugControls();\n</script>\n",
    );
    html.push_str(
        "<script>\nfunction renderFunctionBreakpoints(){\n  const stateNode = document.getElementById('orv-editor-state');\n  const target = document.getElementById('debug-function-breakpoint-list');\n  if (!stateNode || !target) return;\n  const state = JSON.parse(stateNode.textContent);\n  target.textContent = '';\n  for (const breakpoint of state.debug?.function_breakpoints || []) {\n    const row = document.createElement('li');\n    const source = breakpoint.source || {};\n    row.textContent = `${breakpoint.name || 'function'}${source.line ? `:${source.line}` : ''}`;\n    row.tabIndex = 0;\n    const show = () => {\n      renderDebugControlCommand({runner_command: breakpoint.runner_command || []});\n      renderDebugDetail({request: breakpoint.request || {}, runner_command: breakpoint.runner_command || [], source});\n    };\n    row.addEventListener('click', show);\n    row.addEventListener('keydown', event => {\n      if (event.key === 'Enter' || event.key === ' ') {\n        event.preventDefault();\n        show();\n      }\n    });\n    target.appendChild(row);\n  }\n}\nrenderFunctionBreakpoints();\n</script>\n",
    );
    html.push_str(
        "<script>\nfunction renderDataBreakpoints(){\n  const stateNode = document.getElementById('orv-editor-state');\n  const target = document.getElementById('debug-data-breakpoint-list');\n  if (!stateNode || !target) return;\n  const state = JSON.parse(stateNode.textContent);\n  target.textContent = '';\n  for (const breakpoint of state.debug?.data_breakpoints || []) {\n    const row = document.createElement('li');\n    const source = breakpoint.source || {};\n    const line = source.line ? `:${source.line}` : '';\n    row.textContent = `${breakpoint.name || 'local'}${line}`;\n    row.tabIndex = 0;\n    const show = () => {\n      renderDebugControlCommand({runner_command: breakpoint.runner_command || []});\n      renderDebugDetail({info_request: breakpoint.info_request || {}, request: breakpoint.request || {}, runner_command: breakpoint.runner_command || [], source});\n    };\n    row.addEventListener('click', show);\n    row.addEventListener('keydown', event => {\n      if (event.key === 'Enter' || event.key === ' ') {\n        event.preventDefault();\n        show();\n      }\n    });\n    target.appendChild(row);\n  }\n}\nrenderDataBreakpoints();\n</script>\n",
    );
    html.push_str(
        "<script>\nfunction renderExceptionFilters(){\n  const stateNode = document.getElementById('orv-editor-state');\n  const target = document.getElementById('debug-exception-filter-list');\n  if (!stateNode || !target) return;\n  const state = JSON.parse(stateNode.textContent);\n  target.textContent = '';\n  for (const filter of state.debug?.exception_filters || []) {\n    const row = document.createElement('li');\n    row.textContent = filter.label || filter.filter || 'exception filter';\n    row.tabIndex = 0;\n    const show = () => {\n      renderDebugControlCommand({runner_command: filter.runner_command || []});\n      renderDebugDetail({request: filter.request || {}, runner_command: filter.runner_command || []});\n    };\n    row.addEventListener('click', show);\n    row.addEventListener('keydown', event => {\n      if (event.key === 'Enter' || event.key === ' ') {\n        event.preventDefault();\n        show();\n      }\n    });\n    target.appendChild(row);\n  }\n}\nrenderExceptionFilters();\n</script>\n",
    );
    html.push_str(
        "<script>\nfunction renderDebugCapabilities(){\n  const stateNode = document.getElementById('orv-editor-state');\n  const target = document.getElementById('debug-capability-list');\n  if (!stateNode || !target) return;\n  const state = JSON.parse(stateNode.textContent);\n  target.textContent = '';\n  for (const [name, value] of Object.entries(state.debug?.capabilities || {})) {\n    if (value !== true && !Array.isArray(value)) continue;\n    const row = document.createElement('li');\n    row.textContent = Array.isArray(value) ? `${name} (${value.length})` : name;\n    row.tabIndex = 0;\n    const show = () => renderDebugDetail({name, value});\n    row.addEventListener('click', show);\n    row.addEventListener('keydown', event => {\n      if (event.key === 'Enter' || event.key === ' ') {\n        event.preventDefault();\n        show();\n      }\n    });\n    target.appendChild(row);\n  }\n}\nrenderDebugCapabilities();\n</script>\n</body>\n</html>\n",
    );
    Ok(html)
}

pub(crate) fn html_escape_text(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
