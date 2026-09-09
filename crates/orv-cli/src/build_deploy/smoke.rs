use super::*;

pub(super) fn deploy_smoke_source_bundle_hash(dir: &Path) -> anyhow::Result<String> {
    let source_bundle = read_json_value(&dir.join(SOURCE_BUNDLE_PATH))?;
    stable_json_hash(&source_bundle)
}

pub(super) fn deploy_smoke_dap_source_bundle_panel_hash_check(hash: &str) -> String {
    format!(r#"orv_smoke_dap_summary_contains "dap source bundle panel hash" '"hash": "{hash}"'"#)
}

pub(crate) fn deploy_smoke_base_url(listen: Option<&orv_compiler::ServerListenArtifact>) -> String {
    let port = deploy_listen_url_port(listen);
    format!("http://127.0.0.1:{port}")
}

pub(crate) fn deploy_smoke_origin_var_name(method: &str, path: &str) -> String {
    let mut suffix = String::new();
    let mut wrote = false;
    for ch in path.trim_matches('/').chars() {
        if ch.is_ascii_alphanumeric() {
            suffix.push(ch.to_ascii_uppercase());
            wrote = true;
        } else if wrote && !suffix.ends_with('_') {
            suffix.push('_');
        }
    }
    while suffix.ends_with('_') {
        suffix.pop();
    }
    if !wrote {
        suffix.push_str("ROOT");
    }
    format!(
        "ORV_SMOKE_ORIGIN_{}_{}",
        method.to_ascii_uppercase(),
        suffix
    )
}

pub(crate) fn deploy_smoke_origin_var_ref(method: &str, path: &str) -> String {
    format!("${}", deploy_smoke_origin_var_name(method, path))
}

pub(crate) fn deploy_smoke_response_origin_var_name(method: &str, path: &str) -> String {
    deploy_smoke_origin_var_name(method, path).replacen(
        "ORV_SMOKE_ORIGIN_",
        "ORV_SMOKE_RESPONSE_ORIGIN_",
        1,
    )
}

pub(crate) fn deploy_smoke_response_origin_var_ref(method: &str, path: &str) -> String {
    format!("${}", deploy_smoke_response_origin_var_name(method, path))
}

pub(crate) fn deploy_smoke_unique_response_origin(
    route: &orv_compiler::ServerRouteArtifact,
) -> Option<&str> {
    match route.response_origin_ids.as_slice() {
        [origin_id] => Some(origin_id.as_str()),
        _ => None,
    }
}

pub(crate) fn deploy_smoke_has_commerce_record(
    persistence: &DeployPersistence,
    kind: &str,
    record_path: &str,
) -> bool {
    persistence
        .commerce_adapters
        .iter()
        .any(|adapter| adapter.kind == kind && adapter.record_path.as_deref() == Some(record_path))
}

pub(crate) fn deploy_smoke_commerce_record_origin(
    persistence: &DeployPersistence,
    kind: &str,
    record_path: &str,
) -> String {
    persistence
        .commerce_adapters
        .iter()
        .find(|adapter| adapter.kind == kind && adapter.record_path.as_deref() == Some(record_path))
        .and_then(|adapter| adapter.source_origin_ids.first())
        .cloned()
        .unwrap_or_default()
}

pub(crate) fn deploy_smoke_ready_path(
    artifact: &orv_compiler::ServerRuntimeArtifact,
) -> Option<&str> {
    artifact
        .routes
        .iter()
        .find(|route| route.method == "GET" && route.path == "/health")
        .or_else(|| {
            artifact
                .routes
                .iter()
                .find(|route| route.method == "GET" && !route.path.contains(':'))
        })
        .map(|route| route.path.as_str())
}
pub(crate) const DEPLOY_SMOKE_TEST_PATH: &str = "deploy/smoke-test.sh";
pub(crate) const DEPLOY_SMOKE_OUTPUT_PATH: &str = "deploy/smoke-output.txt";

pub(crate) fn deploy_smoke_output_contract_value(
    artifacts: &DeployRunbookArtifacts<'_>,
) -> serde_json::Value {
    smoke_output_contract_value(artifacts.smoke_output)
}

pub(crate) fn smoke_output_contract_value(smoke_output: &str) -> serde_json::Value {
    serde_json::json!({
        "output": smoke_output,
        "required_markers": deploy_benchmark::smoke_required_markers_value(),
    })
}

pub(crate) fn write_prod_smoke_test_artifact(
    out: &Path,
    path: &str,
    server_artifact: &orv_compiler::ServerRuntimeArtifact,
    origin_map: &orv_compiler::OriginMap,
    persistence: &DeployPersistence,
    client: &serde_json::Value,
) -> anyhow::Result<()> {
    let mut script = format!(
        r#"#!/usr/bin/env sh
set -eu
ORV_SMOKE_SCRIPT_DIR=$(CDPATH= cd "$(dirname "$0")" && pwd)
ORV_SMOKE_BUILD_DIR=$(CDPATH= cd "$ORV_SMOKE_SCRIPT_DIR/.." && pwd)
cd "$ORV_SMOKE_BUILD_DIR"
BASE_URL="${{ORV_BASE_URL:-{}}}"
ORV_BIN="${{ORV_BIN:-orv}}"
ORV_SMOKE_OUTPUT="${{ORV_SMOKE_OUTPUT:-{}}}"
ORV_SMOKE_DAP_SUMMARY_OUTPUT=""

if ! command -v curl >/dev/null 2>&1; then
  printf 'orv deploy smoke test requires curl\n' >&2
  exit 127
fi

if ! command -v "$ORV_BIN" >/dev/null 2>&1; then
  printf 'orv deploy smoke test requires orv; set ORV_BIN to the local binary path\n' >&2
  exit 127
fi

orv_smoke_reveal_contains() {{
  label="$1"
  origin_id="$2"
  pattern="$3"
  output_path="$(mktemp)"
  if ! "$ORV_BIN" reveal . "$origin_id" > "$output_path"; then
    rm -f "$output_path"
    printf 'orv deploy smoke test failed: %s reveal command\n' "$label" >&2
    exit 1
  fi
  if ! grep -F "$pattern" "$output_path" >/dev/null; then
    rm -f "$output_path"
    printf 'orv deploy smoke test failed: %s reveal missing %s\n' "$label" "$pattern" >&2
    exit 1
  fi
  rm -f "$output_path"
}}

orv_smoke_editor_reveal_contains() {{
  label="$1"
  origin_id="$2"
  pattern="$3"
  output_path="$(mktemp)"
  if ! "$ORV_BIN" editor reveal . "$origin_id" > "$output_path"; then
    rm -f "$output_path"
    printf 'orv deploy smoke test failed: %s editor reveal command\n' "$label" >&2
    exit 1
  fi
  if ! grep -F "$pattern" "$output_path" >/dev/null; then
    rm -f "$output_path"
    printf 'orv deploy smoke test failed: %s editor reveal missing %s\n' "$label" "$pattern" >&2
    exit 1
  fi
  rm -f "$output_path"
}}

orv_smoke_lsp_reveal_contains() {{
  label="$1"
  origin_id="$2"
  pattern="$3"
  output_path="$(mktemp)"
  if ! "$ORV_BIN" lsp reveal . "$origin_id" > "$output_path"; then
    rm -f "$output_path"
    printf 'orv deploy smoke test failed: %s lsp reveal command\n' "$label" >&2
    exit 1
  fi
  if ! grep -F "$pattern" "$output_path" >/dev/null; then
    rm -f "$output_path"
    printf 'orv deploy smoke test failed: %s lsp reveal missing %s\n' "$label" "$pattern" >&2
    exit 1
  fi
  rm -f "$output_path"
}}

orv_smoke_dap_summary_capture() {{
  if [ -n "$ORV_SMOKE_DAP_SUMMARY_OUTPUT" ] && [ -f "$ORV_SMOKE_DAP_SUMMARY_OUTPUT" ]; then
    return 0
  fi
  output_path="$(mktemp)"
  if ! "$ORV_BIN" editor run-debug . --control next > "$output_path"; then
    rm -f "$output_path"
    printf 'orv deploy smoke test failed: DAP editor run-debug command\n' >&2
    exit 1
  fi
  ORV_SMOKE_DAP_SUMMARY_OUTPUT="$output_path"
}}

orv_smoke_dap_summary_contains() {{
  label="$1"
  pattern="$2"
  orv_smoke_dap_summary_capture
  if ! grep -F "$pattern" "$ORV_SMOKE_DAP_SUMMARY_OUTPUT" >/dev/null; then
    printf 'orv deploy smoke test failed: %s editor run-debug missing %s\n' "$label" "$pattern" >&2
    exit 1
  fi
}}

orv_smoke_dap_summary_cleanup() {{
  if [ -n "$ORV_SMOKE_DAP_SUMMARY_OUTPUT" ]; then
    rm -f "$ORV_SMOKE_DAP_SUMMARY_OUTPUT"
    ORV_SMOKE_DAP_SUMMARY_OUTPUT=""
  fi
}}

orv_smoke_trace_stream() {{
  if [ "${{ORV_SMOKE_TRACE_STREAM:-0}}" != "1" ]; then
    return 0
  fi
  events_path="${{ORV_SMOKE_TRACE_EVENTS:-deploy/trace-events.sse}}"
  output_path="$(mktemp)"
  rm -f "$events_path"
  if ! curl -fsS --max-time "${{ORV_SMOKE_TRACE_TIMEOUT:-2}}" "$BASE_URL/__orv/trace/events" > "$events_path" 2>/dev/null; then
    if ! grep -F 'event: orv:trace' "$events_path" >/dev/null 2>&1; then
      rm -f "$output_path"
      printf 'orv deploy smoke test failed: live trace stream unavailable; start server with --trace deploy/request-trace.json\n' >&2
      exit 1
    fi
  fi
  for pattern in 'event: orv:trace' 'orv.production.trace' 'event: orv:trace.frame' '"kind":"orv.production.trace.frame"' '"index":0' '"frame":{{'; do
    if ! grep -F "$pattern" "$events_path" >/dev/null; then
      rm -f "$output_path"
      printf 'orv deploy smoke test failed: live trace stream missing %s\n' "$pattern" >&2
      exit 1
    fi
  done
  if ! "$ORV_BIN" editor trace-stream . --events "$events_path" > "$output_path"; then
    rm -f "$output_path"
    printf 'orv deploy smoke test failed: editor trace-stream command\n' >&2
    exit 1
  fi
  for pattern in '"kind": "orv.editor.trace.stream"' '"strategy": "event-source-snapshot"' '"trace_frame_event_count":' '"response_navigation"'; do
    if ! grep -F "$pattern" "$output_path" >/dev/null; then
      rm -f "$output_path"
      printf 'orv deploy smoke test failed: editor trace-stream missing %s\n' "$pattern" >&2
      exit 1
    fi
  done
  rm -f "$output_path"
}}

orv_smoke_curl() {{
  label="$1"
  shift
  if ! curl -fsS "$@" >/dev/null; then
    printf 'orv deploy smoke test failed: %s\n' "$label" >&2
    exit 1
  fi
}}

orv_smoke_origin_header() {{
  label="$1"
  headers_path="$2"
  expected_origin="$3"
  actual_origin="$(tr -d '\r' < "$headers_path" | awk '
    {{
      lower = tolower($0)
      if (index(lower, "x-orv-origin-id:") == 1) {{
        value = substr($0, index($0, ":") + 1)
        sub(/^[[:space:]]*/, "", value)
        sub(/[[:space:]]*$/, "", value)
        print value
        exit
      }}
    }}
  ')"
  if [ -z "$actual_origin" ]; then
    printf 'orv deploy smoke test failed: %s missing x-orv-origin-id\n' "$label" >&2
    exit 1
  fi
  if [ "$actual_origin" != "$expected_origin" ]; then
    printf 'orv deploy smoke test failed: %s wrong x-orv-origin-id expected %s got %s\n' "$label" "$expected_origin" "$actual_origin" >&2
    exit 1
  fi
}}

orv_smoke_response_origin_header() {{
  label="$1"
  headers_path="$2"
  expected_response_origin="$3"
  actual_response_origin="$(tr -d '\r' < "$headers_path" | awk '
    {{
      lower = tolower($0)
      if (index(lower, "x-orv-response-origin-id:") == 1) {{
        value = substr($0, index($0, ":") + 1)
        sub(/^[[:space:]]*/, "", value)
        sub(/[[:space:]]*$/, "", value)
        print value
        exit
      }}
    }}
  ')"
  if [ -z "$actual_response_origin" ]; then
    printf 'orv deploy smoke test failed: %s missing x-orv-response-origin-id\n' "$label" >&2
    exit 1
  fi
  if [ "$actual_response_origin" != "$expected_response_origin" ]; then
    printf 'orv deploy smoke test failed: %s wrong x-orv-response-origin-id expected %s got %s\n' "$label" "$expected_response_origin" "$actual_response_origin" >&2
    exit 1
  fi
}}

orv_smoke_curl_origin() {{
  label="$1"
  expected_origin="$2"
  shift 2
  orv_smoke_tmp_headers="$(mktemp)"
  if ! curl -fsS -D "$orv_smoke_tmp_headers" "$@" >/dev/null; then
    rm -f "$orv_smoke_tmp_headers"
    printf 'orv deploy smoke test failed: %s\n' "$label" >&2
    exit 1
  fi
  orv_smoke_origin_header "$label" "$orv_smoke_tmp_headers" "$expected_origin"
  rm -f "$orv_smoke_tmp_headers"
}}

orv_smoke_curl_origin_response() {{
  label="$1"
  expected_origin="$2"
  expected_response_origin="$3"
  shift 3
  orv_smoke_tmp_headers="$(mktemp)"
  if ! curl -fsS -D "$orv_smoke_tmp_headers" "$@" >/dev/null; then
    rm -f "$orv_smoke_tmp_headers"
    printf 'orv deploy smoke test failed: %s\n' "$label" >&2
    exit 1
  fi
  orv_smoke_origin_header "$label" "$orv_smoke_tmp_headers" "$expected_origin"
  orv_smoke_response_origin_header "$label" "$orv_smoke_tmp_headers" "$expected_response_origin"
  rm -f "$orv_smoke_tmp_headers"
}}

orv_smoke_curl_capture_origin() {{
  label="$1"
  headers_path="$2"
  expected_origin="$3"
  shift 3
  if ! curl -fsS -D "$headers_path" "$@" >/dev/null; then
    printf 'orv deploy smoke test failed: %s\n' "$label" >&2
    exit 1
  fi
  orv_smoke_origin_header "$label" "$headers_path" "$expected_origin"
}}

orv_smoke_fetch() {{
  label="$1"
  output_path="$2"
  shift 2
  if ! curl -fsS "$@" > "$output_path"; then
    printf 'orv deploy smoke test failed: %s\n' "$label" >&2
    exit 1
  fi
}}

orv_smoke_fetch_origin() {{
  label="$1"
  output_path="$2"
  expected_origin="$3"
  shift 3
  orv_smoke_tmp_headers="$(mktemp)"
  if ! curl -fsS -D "$orv_smoke_tmp_headers" "$@" > "$output_path"; then
    rm -f "$orv_smoke_tmp_headers"
    printf 'orv deploy smoke test failed: %s\n' "$label" >&2
    exit 1
  fi
  orv_smoke_origin_header "$label" "$orv_smoke_tmp_headers" "$expected_origin"
  rm -f "$orv_smoke_tmp_headers"
}}

orv_smoke_fetch_capture_origin() {{
  label="$1"
  output_path="$2"
  headers_path="$3"
  expected_origin="$4"
  shift 4
  if ! curl -fsS -D "$headers_path" "$@" > "$output_path"; then
    printf 'orv deploy smoke test failed: %s\n' "$label" >&2
    exit 1
  fi
  orv_smoke_origin_header "$label" "$headers_path" "$expected_origin"
}}

orv_smoke_body_contains() {{
  label="$1"
  body_path="$2"
  pattern="$3"
  if ! grep -F "$pattern" "$body_path" >/dev/null; then
    printf 'orv deploy smoke test failed: %s\n' "$label" >&2
    exit 1
  fi
}}

orv_smoke_file() {{
  path="$1"
  if [ ! -f "$path" ]; then
    printf 'orv deploy smoke test missing file: %s\n' "$path" >&2
    exit 1
  fi
}}

orv_smoke_grep() {{
  label="$1"
  path="$2"
  pattern="$3"
  if ! grep -F "$pattern" "$path" >/dev/null; then
    printf 'orv deploy smoke test failed: %s\n' "$label" >&2
    exit 1
  fi
}}

orv_smoke_graph_contract() {{
  for path in source-bundle.json project-graph.json origin-map.json build-manifest.json; do
    orv_smoke_file "$path"
  done
  orv_smoke_grep "source bundle schema" "source-bundle.json" '"schema_version": 1'
  orv_smoke_grep "source bundle files" "source-bundle.json" '"files"'
  orv_smoke_grep "project graph semantic origin map" "project-graph.json" '"origin_map"'
  orv_smoke_grep "project graph origin links" "project-graph.json" '"origin_links"'
  orv_smoke_grep "origin map entries" "origin-map.json" '"entries"'
  if ! "$ORV_BIN" verify-build . >/dev/null; then
    printf 'orv deploy smoke test failed: verify-build graph contract\n' >&2
    exit 1
  fi
}}

orv_smoke_db_bridge_schema() {{
  label="$1"
  endpoint="$2"
  provider="$3"
  adapter_url="$4"
  auth_token="$5"
  if [ -z "$endpoint" ]; then
    printf 'orv deploy smoke test failed: %s missing endpoint\n' "$label" >&2
    exit 1
  fi
  if [ -n "$auth_token" ]; then
    if ! curl -fsS -H 'content-type: application/json' -H 'accept: application/json' -H "authorization: Bearer ${{auth_token}}" --data "{{\"kind\":\"orv.db.adapter\",\"contract\":\"http-json-v1\",\"provider\":\"${{provider}}\",\"url\":\"${{adapter_url}}\",\"method\":\"schema\",\"args\":[]}}" "$endpoint" >/dev/null; then
      printf 'orv deploy smoke test failed: %s\n' "$label" >&2
      exit 1
    fi
    return 0
  fi
  if ! curl -fsS -H 'content-type: application/json' -H 'accept: application/json' --data "{{\"kind\":\"orv.db.adapter\",\"contract\":\"http-json-v1\",\"provider\":\"${{provider}}\",\"url\":\"${{adapter_url}}\",\"method\":\"schema\",\"args\":[]}}" "$endpoint" >/dev/null; then
    printf 'orv deploy smoke test failed: %s\n' "$label" >&2
    exit 1
  fi
}}

orv_smoke_cookie_from_headers() {{
  cookie_name="$1"
  headers_path="$2"
  tr -d '\r' < "$headers_path" | awk -v cookie_name="$cookie_name" '
    {{
      lower = tolower($0)
      if (index(lower, "set-cookie:") == 1) {{
        line = substr($0, length("set-cookie:") + 1)
        sub(/^[[:space:]]*/, "", line)
        split(line, parts, ";")
        split(parts[1], kv, "=")
        if (kv[1] == cookie_name) {{
          print parts[1]
          exit
        }}
      }}
    }}
  '
}}

"#,
        deploy_smoke_base_url(server_artifact.listen.as_ref()),
        DEPLOY_SMOKE_OUTPUT_PATH
    );
    for route in &server_artifact.routes {
        let _ = writeln!(
            script,
            r#"{}="{}""#,
            deploy_smoke_origin_var_name(&route.method, &route.path),
            route.origin_id
        );
        if let Some(response_origin_id) = deploy_smoke_unique_response_origin(route) {
            let _ = writeln!(
                script,
                r#"{}="{}""#,
                deploy_smoke_response_origin_var_name(&route.method, &route.path),
                response_origin_id
            );
        }
    }
    if !client.is_null() {
        let client_origin_id = deploy_smoke_client_reveal_origin(origin_map)
            .ok_or_else(|| anyhow::anyhow!("client bundle smoke requires a revealable origin"))?;
        let _ = writeln!(script, r#"ORV_SMOKE_CLIENT_ORIGIN="{client_origin_id}""#);
    }
    if !server_artifact.routes.is_empty() {
        script.push('\n');
    }
    script.push_str("orv_smoke_graph_contract\n");
    let source_bundle_file_count = server_artifact.source_bundle.files.len();
    let source_bundle_hash = deploy_smoke_source_bundle_hash(out)?;
    let graph_contract_count = deploy_graph_contract_count(out)?;
    let project_graph_node_count = deploy_project_graph_node_count(out)?;
    let origin_entry_count = origin_map.entries.len();
    let source_bundle_panel_hash =
        deploy_smoke_dap_source_bundle_panel_hash_check(&source_bundle_hash);
    let _ = write!(
        script,
        r#"orv_smoke_dap_summary_contains "dap graph summary" '"graph_contract_count": {graph_contract_count}'
orv_smoke_dap_summary_contains "dap source bundle summary" '"source_bundle_file_count": {source_bundle_file_count}'
orv_smoke_dap_summary_contains "dap project graph summary" '"project_graph_node_count": {project_graph_node_count}'
orv_smoke_dap_summary_contains "dap origin summary" '"origin_entry_count": {origin_entry_count}'
orv_smoke_dap_summary_contains "dap source bundle panel" '"source_bundle": {{'
orv_smoke_dap_summary_contains "dap source bundle panel path" '"path": "./source-bundle.json"'
orv_smoke_dap_summary_contains "dap source bundle panel file count" '"fileCount": {source_bundle_file_count}'
{source_bundle_panel_hash}
orv_smoke_dap_summary_contains "dap smoke required markers" '"smoke_test_required_markers": ['
orv_smoke_dap_summary_contains "dap smoke summary required markers" '"required_markers": ['
orv_smoke_dap_summary_contains "dap smoke marker dap source bundle" '"dap_source_bundle"'
"#,
    );
    if !server_artifact.routes.is_empty() {
        let native_summary = deploy_native_server_summary_counts(out)?;
        let native_target_count = native_summary.targets;
        let native_route_count = native_summary.routes;
        let _ = writeln!(
            script,
            r#"orv_smoke_dap_summary_contains "dap native target summary" '"native_server_target_count": {native_target_count}'
orv_smoke_dap_summary_contains "dap native route summary" '"native_server_route_count": {native_route_count}'"#
        );
    }
    if let Some(route) = server_artifact.routes.first() {
        let origin_ref = deploy_smoke_origin_var_ref(&route.method, &route.path);
        script.push_str(&deploy_smoke_reveal_marker_contract_section(&origin_ref));
    }
    script.push_str(&deploy_smoke_client_contract_section(client));
    script.push_str(&deploy_smoke_client_reveal_section(out, client)?);
    script.push_str("orv_smoke_dap_summary_cleanup\n");
    script.push_str(&deploy_smoke_db_adapter_contract_section(persistence));
    script.push_str(&deploy_smoke_output_function_section(
        server_artifact.routes.len(),
        client,
    ));
    if let Some(ready_path) = deploy_smoke_ready_path(server_artifact) {
        let _ = writeln!(
            script,
            r#"READY_PATH="{ready_path}"
for attempt in 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20 21 22 23 24 25 26 27 28 29 30; do
  if curl -fsS "$BASE_URL$READY_PATH" >/dev/null; then
    break
  fi
  if [ "$attempt" = "30" ]; then
    printf 'orv deploy smoke test failed waiting for %s%s\n' "$BASE_URL" "$READY_PATH" >&2
    exit 1
  fi
  sleep 1
done
"#
        );
    }
    for route in server_artifact.routes.iter().filter(|route| {
        route.method == "GET"
            && !route.path.contains(':')
            && !route.path.starts_with("/admin")
            && route.path != "/account/sessions"
    }) {
        let origin_ref = deploy_smoke_origin_var_ref(&route.method, &route.path);
        if deploy_smoke_unique_response_origin(route).is_some() {
            let response_origin_ref =
                deploy_smoke_response_origin_var_ref(&route.method, &route.path);
            let _ = writeln!(
                script,
                r#"orv_smoke_curl_origin_response "GET {}" "{}" "{}" "$BASE_URL{}""#,
                route.path, origin_ref, response_origin_ref, route.path
            );
            let _ = writeln!(
                script,
                r#"orv_smoke_reveal_contains "reveal GET {} response source" "{}" '@respond'"#,
                route.path, response_origin_ref
            );
            let _ = writeln!(
                script,
                r#"orv_smoke_reveal_contains "reveal GET {} response production" "{}" '"response_origin_dispatch": true'"#,
                route.path, response_origin_ref
            );
            let _ = writeln!(
                script,
                r#"orv_smoke_editor_reveal_contains "editor reveal GET {} response source" "{}" '@respond'"#,
                route.path, response_origin_ref
            );
            let _ = writeln!(
                script,
                r#"orv_smoke_editor_reveal_contains "editor reveal GET {} response production" "{}" '"response_origin_dispatch": true'"#,
                route.path, response_origin_ref
            );
            let _ = writeln!(
                script,
                r#"orv_smoke_lsp_reveal_contains "lsp reveal GET {} response origin" "{}" '"name": "respond"'"#,
                route.path, response_origin_ref
            );
            let _ = writeln!(
                script,
                r#"orv_smoke_lsp_reveal_contains "lsp reveal GET {} response production" "{}" '"response_origin_dispatch": true'"#,
                route.path, response_origin_ref
            );
        } else {
            let _ = writeln!(
                script,
                r#"orv_smoke_curl_origin "GET {}" "{}" "$BASE_URL{}""#,
                route.path, origin_ref, route.path
            );
        }
        let summary =
            deploy_route_reveal_summary_counts(out, &route.origin_id, origin_map, server_artifact)?;
        for requirement in
            deploy_route_reveal_summary_requirements(&route.path, &origin_ref, summary)
        {
            script.push_str(&requirement);
            script.push('\n');
        }
    }
    if deploy_routes_include(server_artifact, "POST", "/checkout") {
        let root_origin = deploy_smoke_origin_var_ref("GET", "/");
        let products_origin = deploy_smoke_origin_var_ref("POST", "/products");
        let members_origin = deploy_smoke_origin_var_ref("POST", "/members");
        let login_origin = deploy_smoke_origin_var_ref("POST", "/members/login");
        let account_origin = deploy_smoke_origin_var_ref("GET", "/account/sessions");
        let cart_items_origin = deploy_smoke_origin_var_ref("POST", "/cart/items");
        let catalog_origin = deploy_smoke_origin_var_ref("GET", "/catalog");
        let cart_origin = deploy_smoke_origin_var_ref("GET", "/cart");
        let checkout_origin = deploy_smoke_origin_var_ref("POST", "/checkout");
        let admin_origin = deploy_smoke_origin_var_ref("GET", "/admin");
        let admin_summary_origin = deploy_smoke_origin_var_ref("GET", "/admin/summary");
        let admin_catalog_origin = deploy_smoke_origin_var_ref("GET", "/admin/catalog");
        let admin_orders_origin = deploy_smoke_origin_var_ref("GET", "/admin/orders");
        let admin_payments_origin = deploy_smoke_origin_var_ref("GET", "/admin/payments");
        let admin_shipments_origin = deploy_smoke_origin_var_ref("GET", "/admin/shipments");
        let admin_webhooks_origin = deploy_smoke_origin_var_ref("GET", "/admin/webhooks");
        let admin_audit_origin = deploy_smoke_origin_var_ref("GET", "/admin/audit");
        let db_connect_origin = origin_map
            .entries
            .iter()
            .find(|entry| entry.kind == "call" && entry.name == "@db.connect")
            .map(|entry| entry.id.clone())
            .unwrap_or_default();
        let payment_connect_origin =
            deploy_smoke_commerce_record_origin(persistence, "payment", "data/payments.jsonl");
        let shipping_connect_origin =
            deploy_smoke_commerce_record_origin(persistence, "shipping", "data/shipments.jsonl");
        let shop_smoke = r#"
SMOKE_ID="${ORV_SMOKE_ID:-$(date +%s)}"
SMOKE_SKU="orv-smoke-sku-${SMOKE_ID}"
SMOKE_SKU_SECOND="orv-smoke-sku-${SMOKE_ID}-2"
SMOKE_SKU_THIRD="orv-smoke-sku-${SMOKE_ID}-3"
SMOKE_BADGE="orv-smoke-badge-${SMOKE_ID}"
SMOKE_BADGE_SECOND="orv-smoke-badge-${SMOKE_ID}-2"
SMOKE_BADGE_THIRD="orv-smoke-badge-${SMOKE_ID}-3"
SMOKE_HANDLE="orv-smoke-${SMOKE_ID}"
SMOKE_EMAIL="${SMOKE_HANDLE}@example.invalid"
SMOKE_PASSWORD="orv-smoke-password-${SMOKE_ID}"
ORV_SMOKE_DB_CONNECT_ORIGIN="__DB_CONNECT_ORIGIN__"
ORV_SMOKE_PAYMENT_CONNECT_ORIGIN="__PAYMENT_CONNECT_ORIGIN__"
ORV_SMOKE_SHIPPING_CONNECT_ORIGIN="__SHIPPING_CONNECT_ORIGIN__"
SMOKE_HEADERS="$(mktemp)"
SMOKE_MEMBER_HEADERS="$(mktemp)"
SMOKE_ADMIN_HEADERS="$(mktemp)"
SMOKE_HOME_BODY="$(mktemp)"
SMOKE_CATALOG_BODY="$(mktemp)"
SMOKE_CART_BODY="$(mktemp)"
SMOKE_ACCOUNT_BODY="$(mktemp)"
SMOKE_CHECKOUT_BODY="$(mktemp)"
SMOKE_ADMIN_BODY="$(mktemp)"
SMOKE_ADMIN_SUMMARY_BODY="$(mktemp)"
SMOKE_ADMIN_CATALOG_BODY="$(mktemp)"
SMOKE_ADMIN_ORDERS_BODY="$(mktemp)"
SMOKE_ADMIN_PAYMENTS_BODY="$(mktemp)"
SMOKE_ADMIN_SHIPMENTS_BODY="$(mktemp)"
SMOKE_ADMIN_WEBHOOKS_BODY="$(mktemp)"
SMOKE_ADMIN_AUDIT_BODY="$(mktemp)"
trap 'rm -f "$SMOKE_HEADERS" "$SMOKE_MEMBER_HEADERS" "$SMOKE_ADMIN_HEADERS" "$SMOKE_HOME_BODY" "$SMOKE_CATALOG_BODY" "$SMOKE_CART_BODY" "$SMOKE_ACCOUNT_BODY" "$SMOKE_CHECKOUT_BODY" "$SMOKE_ADMIN_BODY" "$SMOKE_ADMIN_SUMMARY_BODY" "$SMOKE_ADMIN_CATALOG_BODY" "$SMOKE_ADMIN_ORDERS_BODY" "$SMOKE_ADMIN_PAYMENTS_BODY" "$SMOKE_ADMIN_SHIPMENTS_BODY" "$SMOKE_ADMIN_WEBHOOKS_BODY" "$SMOKE_ADMIN_AUDIT_BODY"' EXIT

orv_smoke_fetch_capture_origin "GET / home" "$SMOKE_HOME_BODY" "$SMOKE_HEADERS" "__ROOT_ORIGIN__" "$BASE_URL/"
orv_smoke_body_contains "home title" "$SMOKE_HOME_BODY" 'Miol Shop'
orv_smoke_body_contains "home copy" "$SMOKE_HOME_BODY" 'Catalog, member signup, payment capture, and shipment booking are ready.'
orv_smoke_body_contains "home theme surface" "$SMOKE_HOME_BODY" 'background-color: #f8fafc'
orv_smoke_body_contains "home theme typography" "$SMOKE_HOME_BODY" 'font-family: Inter, system-ui, sans-serif'
orv_smoke_reveal_contains "reveal GET / source" "__ROOT_ORIGIN__" '@route GET /'
orv_smoke_reveal_contains "reveal GET / production" "__ROOT_ORIGIN__" '"path": "/"'
orv_smoke_editor_reveal_contains "editor reveal GET / source" "__ROOT_ORIGIN__" '@route GET /'
orv_smoke_editor_reveal_contains "editor reveal GET / production" "__ROOT_ORIGIN__" '"path": "/"'
orv_smoke_lsp_reveal_contains "lsp reveal GET / origin" "__ROOT_ORIGIN__" '"name": "GET /"'
orv_smoke_lsp_reveal_contains "lsp reveal GET / production" "__ROOT_ORIGIN__" '"path": "/"'
if [ -n "$ORV_SMOKE_DB_CONNECT_ORIGIN" ]; then
  orv_smoke_reveal_contains "reveal DB source" "$ORV_SMOKE_DB_CONNECT_ORIGIN" '@db.connect'
  orv_smoke_reveal_contains "reveal DB preflight" "$ORV_SMOKE_DB_CONNECT_ORIGIN" '"preflight"'
  orv_smoke_reveal_contains "reveal DB smoke summary" "$ORV_SMOKE_DB_CONNECT_ORIGIN" '"smoke_test_summary"'
  orv_smoke_reveal_contains "reveal DB smoke summary count" "$ORV_SMOKE_DB_CONNECT_ORIGIN" '"preflight_smoke_summary_missing_count"'
  orv_smoke_reveal_contains "reveal DB sqlite path" "$ORV_SMOKE_DB_CONNECT_ORIGIN" 'sqlite://data/shop.sqlite'
  orv_smoke_editor_reveal_contains "editor reveal DB source" "$ORV_SMOKE_DB_CONNECT_ORIGIN" '@db.connect'
  orv_smoke_editor_reveal_contains "editor reveal DB preflight" "$ORV_SMOKE_DB_CONNECT_ORIGIN" '"preflight"'
  orv_smoke_editor_reveal_contains "editor reveal DB smoke summary" "$ORV_SMOKE_DB_CONNECT_ORIGIN" '"smoke_test_summary"'
  orv_smoke_editor_reveal_contains "editor reveal DB smoke summary count" "$ORV_SMOKE_DB_CONNECT_ORIGIN" '"preflight_smoke_summary_missing_count"'
  orv_smoke_lsp_reveal_contains "lsp reveal DB origin" "$ORV_SMOKE_DB_CONNECT_ORIGIN" '@db.connect'
  orv_smoke_lsp_reveal_contains "lsp reveal DB preflight" "$ORV_SMOKE_DB_CONNECT_ORIGIN" '"preflight"'
  orv_smoke_lsp_reveal_contains "lsp reveal DB smoke summary" "$ORV_SMOKE_DB_CONNECT_ORIGIN" '"smoke_test_summary"'
  orv_smoke_lsp_reveal_contains "lsp reveal DB smoke summary count" "$ORV_SMOKE_DB_CONNECT_ORIGIN" '"preflight_smoke_summary_missing_count"'
fi
if [ -n "$ORV_SMOKE_PAYMENT_CONNECT_ORIGIN" ]; then
  orv_smoke_reveal_contains "reveal payment source" "$ORV_SMOKE_PAYMENT_CONNECT_ORIGIN" '@payment.connect'
  orv_smoke_reveal_contains "reveal payment match" "$ORV_SMOKE_PAYMENT_CONNECT_ORIGIN" '"matched": true'
  orv_smoke_reveal_contains "reveal payment record path" "$ORV_SMOKE_PAYMENT_CONNECT_ORIGIN" 'file://data/payments.jsonl'
  orv_smoke_reveal_contains "reveal payment request kind" "$ORV_SMOKE_PAYMENT_CONNECT_ORIGIN" 'payment.capture'
  orv_smoke_editor_reveal_contains "editor reveal payment source" "$ORV_SMOKE_PAYMENT_CONNECT_ORIGIN" '@payment.connect'
  orv_smoke_editor_reveal_contains "editor reveal payment match" "$ORV_SMOKE_PAYMENT_CONNECT_ORIGIN" '"matched": true'
  orv_smoke_lsp_reveal_contains "lsp reveal payment origin" "$ORV_SMOKE_PAYMENT_CONNECT_ORIGIN" '@payment.connect'
  orv_smoke_lsp_reveal_contains "lsp reveal payment match" "$ORV_SMOKE_PAYMENT_CONNECT_ORIGIN" '"matched": true'
fi
if [ -n "$ORV_SMOKE_SHIPPING_CONNECT_ORIGIN" ]; then
  orv_smoke_reveal_contains "reveal shipping source" "$ORV_SMOKE_SHIPPING_CONNECT_ORIGIN" '@shipping.connect'
  orv_smoke_reveal_contains "reveal shipping match" "$ORV_SMOKE_SHIPPING_CONNECT_ORIGIN" '"matched": true'
  orv_smoke_reveal_contains "reveal shipping record path" "$ORV_SMOKE_SHIPPING_CONNECT_ORIGIN" 'file://data/shipments.jsonl'
  orv_smoke_reveal_contains "reveal shipping request kind" "$ORV_SMOKE_SHIPPING_CONNECT_ORIGIN" 'shipping.booking'
  orv_smoke_editor_reveal_contains "editor reveal shipping source" "$ORV_SMOKE_SHIPPING_CONNECT_ORIGIN" '@shipping.connect'
  orv_smoke_editor_reveal_contains "editor reveal shipping match" "$ORV_SMOKE_SHIPPING_CONNECT_ORIGIN" '"matched": true'
  orv_smoke_lsp_reveal_contains "lsp reveal shipping origin" "$ORV_SMOKE_SHIPPING_CONNECT_ORIGIN" '@shipping.connect'
  orv_smoke_lsp_reveal_contains "lsp reveal shipping match" "$ORV_SMOKE_SHIPPING_CONNECT_ORIGIN" '"matched": true'
fi
CSRF_COOKIE="$(orv_smoke_cookie_from_headers orv_csrf "$SMOKE_HEADERS")"
if [ -z "$CSRF_COOKIE" ]; then
  printf 'orv deploy smoke test failed: missing orv_csrf cookie\n' >&2
  exit 1
fi
CSRF_TOKEN="${CSRF_COOKIE#orv_csrf=}"

orv_smoke_curl_origin "POST /products" "__PRODUCTS_ORIGIN__" -X POST "$BASE_URL/products" -H 'content-type: application/json' -H "cookie: ${CSRF_COOKIE}" -H "x-csrf-token: ${CSRF_TOKEN}" --data "{\"sku\":\"${SMOKE_SKU}\",\"name\":\"ORV Smoke Product\",\"badge\":\"${SMOKE_BADGE}\",\"price\":1000,\"stock\":5}"
orv_smoke_curl_origin "POST /products second" "__PRODUCTS_ORIGIN__" -X POST "$BASE_URL/products" -H 'content-type: application/json' -H "cookie: ${CSRF_COOKIE}" -H "x-csrf-token: ${CSRF_TOKEN}" --data "{\"sku\":\"${SMOKE_SKU_SECOND}\",\"name\":\"ORV Smoke Product 2\",\"badge\":\"${SMOKE_BADGE_SECOND}\",\"price\":1200,\"stock\":4}"
orv_smoke_curl_origin "POST /products third" "__PRODUCTS_ORIGIN__" -X POST "$BASE_URL/products" -H 'content-type: application/json' -H "cookie: ${CSRF_COOKIE}" -H "x-csrf-token: ${CSRF_TOKEN}" --data "{\"sku\":\"${SMOKE_SKU_THIRD}\",\"name\":\"ORV Smoke Product 3\",\"badge\":\"${SMOKE_BADGE_THIRD}\",\"price\":1300,\"stock\":3}"
orv_smoke_curl_origin "POST /members" "__MEMBERS_ORIGIN__" -X POST "$BASE_URL/members" -H 'content-type: application/json' -H "cookie: ${CSRF_COOKIE}" -H "x-csrf-token: ${CSRF_TOKEN}" --data "{\"handle\":\"${SMOKE_HANDLE}\",\"name\":\"ORV Smoke Member\",\"email\":\"${SMOKE_EMAIL}\",\"password\":\"${SMOKE_PASSWORD}\"}"
orv_smoke_curl_capture_origin "POST /members/login smoke" "$SMOKE_MEMBER_HEADERS" "__LOGIN_ORIGIN__" -X POST "$BASE_URL/members/login" -H 'content-type: application/json' -H "cookie: ${CSRF_COOKIE}" -H "x-csrf-token: ${CSRF_TOKEN}" --data "{\"handle\":\"${SMOKE_HANDLE}\",\"email\":\"${SMOKE_EMAIL}\",\"password\":\"${SMOKE_PASSWORD}\"}"
MEMBER_SESSION_COOKIE="$(orv_smoke_cookie_from_headers orv_session "$SMOKE_MEMBER_HEADERS")"
if [ -z "$MEMBER_SESSION_COOKIE" ]; then
  printf 'orv deploy smoke test failed: missing member session cookie\n' >&2
  exit 1
fi
orv_smoke_curl_origin "GET /account/sessions" "__ACCOUNT_ORIGIN__" -H "cookie: ${MEMBER_SESSION_COOKIE}" "$BASE_URL/account/sessions"
orv_smoke_fetch_origin "GET /account/sessions content" "$SMOKE_ACCOUNT_BODY" "__ACCOUNT_ORIGIN__" -H "cookie: ${MEMBER_SESSION_COOKIE}" "$BASE_URL/account/sessions"
orv_smoke_body_contains "account smoke session" "$SMOKE_ACCOUNT_BODY" "$SMOKE_HANDLE"
orv_smoke_curl_origin "POST /cart/items" "__CART_ITEMS_ORIGIN__" -X POST "$BASE_URL/cart/items" -H 'content-type: application/json' -H "cookie: ${CSRF_COOKIE}" -H "x-csrf-token: ${CSRF_TOKEN}" --data "{\"handle\":\"${SMOKE_HANDLE}\",\"sku\":\"${SMOKE_SKU}\",\"quantity\":1}"
orv_smoke_fetch_origin "GET /catalog content" "$SMOKE_CATALOG_BODY" "__CATALOG_ORIGIN__" "$BASE_URL/catalog"
orv_smoke_body_contains "catalog smoke product" "$SMOKE_CATALOG_BODY" "$SMOKE_SKU"
orv_smoke_body_contains "catalog second smoke product" "$SMOKE_CATALOG_BODY" "$SMOKE_SKU_SECOND"
orv_smoke_body_contains "catalog third smoke product" "$SMOKE_CATALOG_BODY" "$SMOKE_SKU_THIRD"
orv_smoke_body_contains "catalog smoke product field" "$SMOKE_CATALOG_BODY" "$SMOKE_BADGE"
orv_smoke_body_contains "catalog second smoke product field" "$SMOKE_CATALOG_BODY" "$SMOKE_BADGE_SECOND"
orv_smoke_body_contains "catalog third smoke product field" "$SMOKE_CATALOG_BODY" "$SMOKE_BADGE_THIRD"
orv_smoke_fetch_origin "GET /cart content" "$SMOKE_CART_BODY" "__CART_ORIGIN__" "$BASE_URL/cart"
orv_smoke_body_contains "cart smoke item" "$SMOKE_CART_BODY" "$SMOKE_SKU"
orv_smoke_fetch_origin "POST /checkout" "$SMOKE_CHECKOUT_BODY" "__CHECKOUT_ORIGIN__" -X POST "$BASE_URL/checkout" -H 'content-type: application/json' -H "cookie: ${CSRF_COOKIE}" -H "x-csrf-token: ${CSRF_TOKEN}" --data "{\"handle\":\"${SMOKE_HANDLE}\",\"sku\":\"${SMOKE_SKU}\",\"quantity\":1,\"total\":1000,\"method\":\"card\",\"carrier\":\"post\",\"address\":\"ORV smoke address\"}"
orv_smoke_body_contains "checkout shipped order" "$SMOKE_CHECKOUT_BODY" '"status":"shipped"'
orv_smoke_body_contains "checkout captured payment" "$SMOKE_CHECKOUT_BODY" '"status":"captured"'
orv_smoke_body_contains "checkout shipment tracking" "$SMOKE_CHECKOUT_BODY" 'TRK-LOCAL'
orv_smoke_curl_capture_origin "POST /members/login admin" "$SMOKE_ADMIN_HEADERS" "__LOGIN_ORIGIN__" -X POST "$BASE_URL/members/login" -H 'content-type: application/json' -H "cookie: ${CSRF_COOKIE}" -H "x-csrf-token: ${CSRF_TOKEN}" --data "{\"handle\":\"admin\",\"email\":\"admin@example.test\",\"password\":\"admin-reference-password\"}"
ADMIN_SESSION_COOKIE="$(orv_smoke_cookie_from_headers orv_session "$SMOKE_ADMIN_HEADERS")"
ADMIN_ROLE_COOKIE="$(orv_smoke_cookie_from_headers orv_session_role "$SMOKE_ADMIN_HEADERS")"
if [ -z "$ADMIN_SESSION_COOKIE" ] || [ -z "$ADMIN_ROLE_COOKIE" ]; then
  printf 'orv deploy smoke test failed: missing admin session cookies\n' >&2
  exit 1
fi
orv_smoke_fetch_origin "GET /admin dashboard content" "$SMOKE_ADMIN_BODY" "__ADMIN_ORIGIN__" -H "cookie: ${ADMIN_SESSION_COOKIE}; ${ADMIN_ROLE_COOKIE}" "$BASE_URL/admin"
orv_smoke_body_contains "admin dashboard title" "$SMOKE_ADMIN_BODY" 'Miol Shop Admin'
orv_smoke_body_contains "admin dashboard summary link" "$SMOKE_ADMIN_BODY" '/admin/summary'
orv_smoke_body_contains "admin dashboard webhook link" "$SMOKE_ADMIN_BODY" '/admin/webhooks'
orv_smoke_body_contains "admin dashboard audit link" "$SMOKE_ADMIN_BODY" '/admin/audit'
orv_smoke_body_contains "admin dashboard sqlite storage" "$SMOKE_ADMIN_BODY" 'data/shop.sqlite'
orv_smoke_body_contains "admin dashboard payment storage" "$SMOKE_ADMIN_BODY" 'data/payments.jsonl'
orv_smoke_body_contains "admin dashboard shipment storage" "$SMOKE_ADMIN_BODY" 'data/shipments.jsonl'
orv_smoke_fetch_origin "GET /admin/summary content" "$SMOKE_ADMIN_SUMMARY_BODY" "__ADMIN_SUMMARY_ORIGIN__" -H "cookie: ${ADMIN_SESSION_COOKIE}; ${ADMIN_ROLE_COOKIE}" "$BASE_URL/admin/summary"
orv_smoke_body_contains "admin summary orders" "$SMOKE_ADMIN_SUMMARY_BODY" '"orders"'
orv_smoke_body_contains "admin summary payments" "$SMOKE_ADMIN_SUMMARY_BODY" '"payments"'
orv_smoke_body_contains "admin summary webhook events" "$SMOKE_ADMIN_SUMMARY_BODY" '"webhookEvents"'
orv_smoke_body_contains "admin summary audit events" "$SMOKE_ADMIN_SUMMARY_BODY" '"auditEvents"'
orv_smoke_fetch_origin "GET /admin/catalog content" "$SMOKE_ADMIN_CATALOG_BODY" "__ADMIN_CATALOG_ORIGIN__" -H "cookie: ${ADMIN_SESSION_COOKIE}; ${ADMIN_ROLE_COOKIE}" "$BASE_URL/admin/catalog"
orv_smoke_body_contains "admin catalog smoke product" "$SMOKE_ADMIN_CATALOG_BODY" "$SMOKE_SKU"
orv_smoke_body_contains "admin catalog second smoke product" "$SMOKE_ADMIN_CATALOG_BODY" "$SMOKE_SKU_SECOND"
orv_smoke_body_contains "admin catalog third smoke product" "$SMOKE_ADMIN_CATALOG_BODY" "$SMOKE_SKU_THIRD"
orv_smoke_body_contains "admin catalog smoke product field" "$SMOKE_ADMIN_CATALOG_BODY" "$SMOKE_BADGE"
orv_smoke_body_contains "admin catalog second smoke product field" "$SMOKE_ADMIN_CATALOG_BODY" "$SMOKE_BADGE_SECOND"
orv_smoke_body_contains "admin catalog third smoke product field" "$SMOKE_ADMIN_CATALOG_BODY" "$SMOKE_BADGE_THIRD"
orv_smoke_fetch_origin "GET /admin/orders content" "$SMOKE_ADMIN_ORDERS_BODY" "__ADMIN_ORDERS_ORIGIN__" -H "cookie: ${ADMIN_SESSION_COOKIE}; ${ADMIN_ROLE_COOKIE}" "$BASE_URL/admin/orders"
orv_smoke_body_contains "admin orders smoke member" "$SMOKE_ADMIN_ORDERS_BODY" "$SMOKE_HANDLE"
orv_smoke_body_contains "admin orders shipped" "$SMOKE_ADMIN_ORDERS_BODY" 'shipped'
orv_smoke_fetch_origin "GET /admin/payments content" "$SMOKE_ADMIN_PAYMENTS_BODY" "__ADMIN_PAYMENTS_ORIGIN__" -H "cookie: ${ADMIN_SESSION_COOKIE}; ${ADMIN_ROLE_COOKIE}" "$BASE_URL/admin/payments"
orv_smoke_body_contains "admin payments captured" "$SMOKE_ADMIN_PAYMENTS_BODY" 'captured'
orv_smoke_fetch_origin "GET /admin/shipments content" "$SMOKE_ADMIN_SHIPMENTS_BODY" "__ADMIN_SHIPMENTS_ORIGIN__" -H "cookie: ${ADMIN_SESSION_COOKIE}; ${ADMIN_ROLE_COOKIE}" "$BASE_URL/admin/shipments"
orv_smoke_body_contains "admin shipments tracking" "$SMOKE_ADMIN_SHIPMENTS_BODY" 'TRK-LOCAL'
orv_smoke_fetch_origin "GET /admin/webhooks content" "$SMOKE_ADMIN_WEBHOOKS_BODY" "__ADMIN_WEBHOOKS_ORIGIN__" -H "cookie: ${ADMIN_SESSION_COOKIE}; ${ADMIN_ROLE_COOKIE}" "$BASE_URL/admin/webhooks"
orv_smoke_body_contains "admin webhooks title" "$SMOKE_ADMIN_WEBHOOKS_BODY" 'Webhooks'
orv_smoke_fetch_origin "GET /admin/audit content" "$SMOKE_ADMIN_AUDIT_BODY" "__ADMIN_AUDIT_ORIGIN__" -H "cookie: ${ADMIN_SESSION_COOKIE}; ${ADMIN_ROLE_COOKIE}" "$BASE_URL/admin/audit"
orv_smoke_body_contains "admin audit checkout" "$SMOKE_ADMIN_AUDIT_BODY" 'checkout.complete'
orv_smoke_body_contains "admin audit payment" "$SMOKE_ADMIN_AUDIT_BODY" 'payment.capture'
orv_smoke_body_contains "admin audit shipment" "$SMOKE_ADMIN_AUDIT_BODY" 'shipment.book'
"#
        .replace("__ROOT_ORIGIN__", &root_origin)
        .replace("__DB_CONNECT_ORIGIN__", &db_connect_origin)
        .replace("__PAYMENT_CONNECT_ORIGIN__", &payment_connect_origin)
        .replace("__SHIPPING_CONNECT_ORIGIN__", &shipping_connect_origin)
        .replace("__PRODUCTS_ORIGIN__", &products_origin)
        .replace("__MEMBERS_ORIGIN__", &members_origin)
        .replace("__LOGIN_ORIGIN__", &login_origin)
        .replace("__ACCOUNT_ORIGIN__", &account_origin)
        .replace("__CART_ITEMS_ORIGIN__", &cart_items_origin)
        .replace("__CATALOG_ORIGIN__", &catalog_origin)
        .replace("__CART_ORIGIN__", &cart_origin)
        .replace("__CHECKOUT_ORIGIN__", &checkout_origin)
        .replace("__ADMIN_ORIGIN__", &admin_origin)
        .replace("__ADMIN_SUMMARY_ORIGIN__", &admin_summary_origin)
        .replace("__ADMIN_CATALOG_ORIGIN__", &admin_catalog_origin)
        .replace("__ADMIN_ORDERS_ORIGIN__", &admin_orders_origin)
        .replace("__ADMIN_PAYMENTS_ORIGIN__", &admin_payments_origin)
        .replace("__ADMIN_SHIPMENTS_ORIGIN__", &admin_shipments_origin)
        .replace("__ADMIN_WEBHOOKS_ORIGIN__", &admin_webhooks_origin)
        .replace("__ADMIN_AUDIT_ORIGIN__", &admin_audit_origin);
        script.push_str(&shop_smoke);
        for route in server_artifact.routes.iter().filter(|route| {
            route.method == "GET" && !route.path.contains(':') && route.path.starts_with("/admin")
        }) {
            let origin_ref = deploy_smoke_origin_var_ref(&route.method, &route.path);
            let _ = writeln!(
                script,
                r#"orv_smoke_curl_origin "GET {}" "{}" -H "cookie: ${{ADMIN_SESSION_COOKIE}}; ${{ADMIN_ROLE_COOKIE}}" "$BASE_URL{}""#,
                route.path, origin_ref, route.path
            );
        }
    }
    script.push_str("orv_smoke_trace_stream\n");
    script.push_str("orv_smoke_write_output\nprintf 'orv deploy smoke test passed\\n'\n");
    let target = out.join(path);
    write_text(&target, &script)?;
    set_executable_if_supported(&target)
}

pub(crate) fn deploy_smoke_output_function_section(
    route_count: usize,
    client: &serde_json::Value,
) -> String {
    let mut out = format!(
        r#"orv_smoke_write_output() {{
  {{
    printf 'orv deploy smoke test passed\n'
    printf 'build_dir=%s\n' "$ORV_SMOKE_BUILD_DIR"
    printf 'base_url=%s\n' "$BASE_URL"
    printf 'graph_contract=verified\n'
    printf 'dap_summary=verified\n'
    printf 'dap_source_bundle=verified\n'
    printf 'server_routes={route_count}\n'
    printf 'trace_stream_requested=%s\n' "${{ORV_SMOKE_TRACE_STREAM:-0}}"
"#,
    );
    if !client.is_null() {
        let manifest = json_str_or_empty(client, "manifest");
        let reactive_plan = json_str_or_empty(client, "reactive_plan");
        let page = json_str_or_empty(client, "page");
        let loader = json_str_or_empty(client, "loader");
        let wasm = json_str_or_empty(client, "wasm");
        for line in [
            format!("    printf 'client_manifest={manifest}\\n'\n"),
            format!("    printf 'client_reactive_plan={reactive_plan}\\n'\n"),
            format!("    printf 'client_page={page}\\n'\n"),
            format!("    printf 'client_loader={loader}\\n'\n"),
            format!("    printf 'client_wasm={wasm}\\n'\n"),
        ] {
            out.push_str(&line);
        }
    }
    out.push_str(
        r#"  } > "$ORV_SMOKE_OUTPUT"
}

"#,
    );
    out
}

pub(crate) fn deploy_smoke_client_contract_section(client: &serde_json::Value) -> String {
    if client.is_null() {
        return String::new();
    }
    let manifest = json_str_or_empty(client, "manifest");
    let reactive_plan = json_str_or_empty(client, "reactive_plan");
    let page = json_str_or_empty(client, "page");
    let loader = json_str_or_empty(client, "loader");
    let wasm = json_str_or_empty(client, "wasm");
    format!(
        r#"orv_smoke_file "{manifest}"
orv_smoke_file "{reactive_plan}"
orv_smoke_file "{page}"
orv_smoke_file "{loader}"
orv_smoke_file "{wasm}"
orv_smoke_grep "client page marker" "{page}" 'data-orv-client="wasm"'
orv_smoke_grep "client loader reference" "{page}" 'app.js'
orv_smoke_grep "client manifest reactive plan path" "{manifest}" '"reactive_plan": "{reactive_plan}"'
orv_smoke_grep "client manifest reactive plan hash" "{manifest}" '"reactive_plan_hash"'
orv_smoke_grep "client manifest loader hash" "{manifest}" '"loader_hash"'
orv_smoke_grep "client manifest wasm hash" "{manifest}" '"wasm_hash"'
orv_smoke_grep "client manifest source bundle" "{manifest}" '"source_bundle": "source-bundle.json"'
orv_smoke_grep "client manifest runtime" "{manifest}" '"runtime": "client_wasm"'
orv_smoke_grep "client manifest capabilities" "{manifest}" '"capabilities"'
orv_smoke_grep "client manifest capability surfaces" "{manifest}" '"surfaces"'
orv_smoke_grep "client manifest event actions" "{manifest}" '"event_actions"'
orv_smoke_grep "client reactive plan kind" "{reactive_plan}" '"kind": "orv.client.reactive_plan"'
orv_smoke_grep "client reactive plan source bundle" "{reactive_plan}" '"source_bundle": "source-bundle.json"'
orv_smoke_grep "client reactive plan blocked_by" "{reactive_plan}" '"blocked_by"'
orv_smoke_grep "client loader bootstrap" "{loader}" 'ORV_CLIENT_BOOTSTRAP'
orv_smoke_grep "client loader embedded reactive plan" "{loader}" 'embeddedReactivePlan'
orv_smoke_grep "client loader embedded reactive plan hash" "{loader}" 'embeddedReactivePlanHash'
orv_smoke_grep "client loader source bundle hash" "{loader}" 'sourceBundleHash'
orv_smoke_grep "client loader wasm reference" "{loader}" 'app.wasm'
orv_smoke_grep "client loader signal setter" "{loader}" '__ORV_SET_SIGNAL__'

"#
    )
}

pub(crate) fn deploy_smoke_db_adapter_contract_section(persistence: &DeployPersistence) -> String {
    if persistence.db_adapters.is_empty() {
        return String::new();
    }
    let mut out = String::from(
        r#"orv_smoke_file "deploy/db-adapters.json"
orv_smoke_grep "db adapter artifact kind" "deploy/db-adapters.json" '"orv.deploy.db_adapters"'
orv_smoke_grep "db adapter bridge contract" "deploy/db-adapters.json" '"contract": "http-json-v1"'
orv_smoke_grep "db adapter bridge retry" "deploy/db-adapters.json" '"retry"'
"#,
    );
    for adapter in &persistence.db_adapters {
        let Some(endpoint_env) = adapter
            .bridge_env
            .iter()
            .find(|env| env.purpose == "bridge_endpoint")
        else {
            continue;
        };
        let Some(endpoint) = &adapter.endpoint else {
            continue;
        };
        let auth_env = adapter
            .bridge_env
            .iter()
            .find(|env| env.purpose == "bridge_auth_token")
            .map(|env| env.env.as_str())
            .unwrap_or("");
        let endpoint_expr = format!("${{{}:-${{ORV_DB_ADAPTER_ENDPOINT:-}}}}", endpoint_env.env);
        let auth_expr = format!("${{{auth_env}:-${{ORV_DB_ADAPTER_AUTH_TOKEN:-}}}}");
        let _ = writeln!(
            out,
            r#"orv_smoke_db_bridge_schema "{} bridge" "{}" "{}" "{}" "{}""#,
            adapter.provider, endpoint_expr, adapter.provider, endpoint, auth_expr
        );
    }
    out.push('\n');
    out
}
