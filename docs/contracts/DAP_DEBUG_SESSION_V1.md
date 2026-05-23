# DAP Debug Session v1 Contract

Producer:

- `orv editor debug <file>`
- `orv editor run-debug <state.json|debug/session-runner.json|build-dir>`
- `orv editor export <file> --out <dir>` as `debug/session-runner.json`
- `orv dap serve --stdio`
- build-backed editor exports and generated deploy smoke via
  `orv editor run-debug . --control next`

Current regression coverage:

- `crates/orv-cli/tests/dap_debug_contract.rs::dap_debug_runner_result_contract_freezes_public_shape`
- `crates/orv-cli/tests/dap_debug_contract.rs::dap_debug_session_v1_freezes_stdio_initialize_contract`
- `crates/orv-cli/tests/dap_debug_contract.rs::dap_debug_runner_rejects_extra_runner_root_key`
- `crates/orv-cli/tests/dap_debug_contract.rs::dap_debug_runner_rejects_extra_result_artifact_key`
- `crates/orv-cli/tests/dap_debug_contract.rs::dap_debug_runner_rejects_extra_export_state_root_key`
- `crates/orv-cli/tests/dap_debug_contract.rs::dap_debug_runner_rejects_extra_session_key`
- `crates/orv-cli/tests/dap_debug_contract.rs::dap_debug_runner_rejects_extra_control_key`
- `crates/orv-cli/tests/dap_debug_contract.rs::dap_debug_runner_rejects_extra_production_context_key`
- `crates/orv-cli/src/tests.rs::editor_run_debug_writes_native_debug_result_panel_contract`
- `crates/orv-cli/src/tests.rs::editor_run_debug_build_dir_rehydrates_source_bundle_when_original_source_is_missing`
- `crates/orv-cli/src/tests.rs::editor_run_debug_result_summarizes_*_production_targets`
- generated deploy smoke gates for DAP production summary and source-bundle
  markers

This contract covers the public debug runner/result JSON consumed by static
editor exports, native-host debug panels, generated deploy smoke, and build-dir
debug runs. It also freezes the `orv dap serve --stdio` bootstrap envelope that
debug clients use before launch. Other raw DAP frame details remain adapter
transport internals.

## DAP Stdio Bootstrap

`orv dap serve --stdio` uses DAP `Content-Length` framing. An `initialize`
request returns a response frame with exactly:

```json
{
  "seq": 1,
  "type": "response",
  "request_seq": 1,
  "success": true,
  "command": "initialize",
  "body": {}
}
```

The `body` object has exactly these boolean capabilities, all `true`:

- `supportsConfigurationDoneRequest`
- `supportsTerminateRequest`
- `supportsTerminateThreadsRequest`
- `supportsLoadedSourcesRequest`
- `supportsEvaluateForHovers`
- `supportsCompletionsRequest`
- `supportsBreakpointLocationsRequest`
- `supportsConditionalBreakpoints`
- `supportsHitConditionalBreakpoints`
- `supportsFunctionBreakpoints`
- `supportsDataBreakpoints`
- `supportsExceptionInfoRequest`
- `supportsRestartRequest`
- `supportsSetVariable`
- `supportsSetExpression`
- `supportsModulesRequest`
- `supportsGotoTargetsRequest`
- `supportsStepBack`
- `supportsStepInTargetsRequest`
- `supportsRestartFrame`
- `supportsPauseRequest`
- `supportsCancelRequest`
- `supportsInstructionBreakpoints`
- `supportsDisassembleRequest`
- `supportsReadMemoryRequest`
- `supportsOrvRuntimeAttach`
- `supportsOrvRuntimeTracePath`
- `supportsOrvSourceBundleLaunch`

The `body.exceptionBreakpointFilters[]` array contains two default-enabled
filters:

- `orv.diagnostics` labeled `ORV diagnostics`
- `orv.runtime` labeled `ORV runtime errors`

After the initialize response, the server emits an `initialized` event frame
with an empty object body.

## Runner Result Root

`orv editor run-debug ...` emits and may write `debug/session-result.json` with
exactly:

```json
{
  "schema_version": 1,
  "kind": "orv.editor.debug.runner.result",
  "state": "dist",
  "runner": {},
  "production_context": {},
  "debug": {},
  "panels": {}
}
```

Rules:

- `schema_version` is currently `1`.
- `kind` is `orv.editor.debug.runner.result`.
- `state` is the input path used for the run: export state, runner artifact, or
  build directory.
- `runner` is the normalized debug runner artifact used for execution.
- `production_context` mirrors `runner.production_context` when available.
- `debug` is the raw debug session summary.
- `panels.debug` is the stable native/editor panel payload.
- When `state` points at an exported `state.json`, the root `schema_version`
  must be `1`; stale or unversioned export states are rejected before launch.
- Unknown exported `state.json` root keys are rejected before launch, so DAP
  runs cannot silently accept drifted editor handoff artifacts.

## Build-Dir Runner

When the input is a build directory, `runner` has exactly:

```json
{
  "schema_version": 1,
  "kind": "orv.editor.debug.runner",
  "program": "app.orv",
  "source_bundle": "dist/source-bundle.json",
  "production_context": {},
  "result": {}
}
```

Rules:

- `source_bundle` points at the build `source-bundle.json`.
- `program` is rehydrated from the source bundle, so debug can run after the
  original source file is unavailable.
- `result.path` is `debug/session-result.json`.
- `result.html_path` is `debug/session-result.html`.
- `result.panel_contract.root` is `panels.debug`.
- `schema_version` must be `1`; stale or unversioned runner artifacts are
  rejected before launch.
- Unknown runner/result/production summary keys are rejected before launch, so
  `session-result.json` cannot echo drifted debug contract data.

## Exported Session Runner

`orv editor export <file> --out <dir>` writes `debug/session-runner.json`, and
`state.json.debug.session_runner` mirrors the same object. The export runner has
exactly:

```json
{
  "schema_version": 1,
  "kind": "orv.editor.debug.runner",
  "program": "app.orv",
  "transport": {},
  "command": [],
  "result": {},
  "session": {},
  "controls": []
}
```

Build-backed exports also include `source_bundle` and `production_context`.
Rules:

- `schema_version` must be `1`; stale or unversioned runner artifacts are
  rejected before launch.
- Unknown runner/result/production summary keys are rejected before launch, so
  exported editor/native-host runners keep the same public result shape.
- Unknown `session`, `controls[]`, and `production_context` keys are rejected
  before launch.
- `transport.protocol` is `dap`, and `transport.framing` is `content-length`.
- `command` is the default `orv editor run-debug debug/session-runner.json`
  command for the `next` debug control.
- `session` freezes launch/thread/breakpoint argument metadata consumed by
  static and native editor shells.
- `controls[]` lists stable debug controls and matching runner commands.

## Production Context

`production_context` has exactly:

```json
{
  "schema_version": 1,
  "kind": "orv.editor.debug.production_context",
  "build_dir": "dist",
  "source_bundle": "dist/source-bundle.json",
  "graph_contract": [],
  "preflight": [],
  "summary": {}
}
```

Rules:

- `graph_contract[]` carries source-bundle, ProjectGraph, and OriginMap
  contract targets when present.
- `preflight[]` carries production preflight targets when the build has server
  deploy artifacts. Nested `benchmark_evidence` summaries use the same hard
  evidence gates, smoke-output artifact parity status, and per-run raw-notes
  artifact retained/non-empty status as `orv benchmark-report . --require-pass`.
- `summary` uses the shared editor production summary shape.

`summary` has exactly:

```json
{
  "schema_version": 1,
  "build_dir": "dist",
  "graph_contract_count": 3,
  "source_bundle_file_count": 1,
  "project_graph_node_count": 1,
  "origin_entry_count": 1,
  "client_target_count": 0,
  "client_manifest_count": 0,
  "client_capability_surface_count": 0,
  "route_target_count": 0,
  "native_server_target_count": 0,
  "native_server_route_count": 0,
  "native_server_blocker_count": 0,
  "static_target_count": 0,
  "static_verified_count": 0,
  "preflight_target_count": 0,
  "preflight_command_count": 0,
  "preflight_route_count": 0,
  "preflight_required_env_count": 0,
  "preflight_optional_env_count": 0,
  "preflight_smoke_summary_present_count": 0,
  "preflight_smoke_summary_missing_count": 0,
  "preflight_smoke_summary_missing_marker_count": 0,
  "route_policy_count": 0,
  "route_policy_kind_counts": {},
  "db_target_count": 0,
  "commerce_target_count": 0,
  "db_adapter_count": 0,
  "commerce_adapter_count": 0,
  "adapter_count": 0,
  "missing_artifact_count": 0
}
```

Rules:

- Counts are derived from checked build artifacts, not inferred by the editor.
- Generated deploy smoke checks the same graph/source-bundle/native/client/smoke
  counters through `orv editor run-debug . --control next`.

## Debug Session

`debug` has exactly:

```json
{
  "schema_version": 1,
  "kind": "orv.editor.debug",
  "program": "app.orv",
  "adapter": {},
  "transport": {},
  "breakpoints": [],
  "function_breakpoints": [],
  "data_breakpoints": [],
  "exception_filters": [],
  "launch": {},
  "loaded_sources": {},
  "source_snapshots": [],
  "control": {},
  "controls": [],
  "watch_expressions": [],
  "stack": {},
  "scopes": {},
  "project_variables": [],
  "locals": [],
  "frames": []
}
```

Rules:

- `transport.protocol` is `dap`.
- `transport.framing` is `content-length`.
- `launch.body.sourceBundle` contains `path`, `entry`, `fileCount`, and `hash`
  when launched from a build source bundle.
- `loaded_sources` and `source_snapshots[]` carry source inventory for editor
  navigation.
- `controls[]`, breakpoint arrays, exception filters, and watch expressions
  record request/response summaries for the run.

## Debug Panel

`panels.debug` has exactly:

```json
{
  "schema_version": 1,
  "production_context": {},
  "production_summary": {},
  "session_summary": {},
  "source_bundle": {},
  "result_artifact": {},
  "selected_frame": {},
  "stack_frames": [],
  "source_navigation": {},
  "scopes": {},
  "project_variables": [],
  "locals": [],
  "control_count": 1,
  "breakpoint_count": 0,
  "function_breakpoint_count": 0,
  "data_breakpoint_count": 0,
  "exception_filter_count": 0,
  "watch_expression_count": 0,
  "loaded_source_count": 1,
  "source_snapshot_count": 1,
  "controls": [],
  "breakpoints": [],
  "function_breakpoints": [],
  "data_breakpoints": [],
  "exception_filters": [],
  "watch_expressions": [],
  "loaded_sources": {},
  "source_snapshots": [],
  "event_count": 1,
  "stopped_event_count": 1,
  "output_event_count": 0,
  "events": [],
  "stopped_events": [],
  "output_events": []
}
```

Rules:

- `session_summary.source_bundle` equals `source_bundle`.
- `production_summary` equals `production_context.summary` when production
  context exists.
- `source_navigation.selected` points at the selected stack frame source.
- `result_artifact.panel_contract.sections[]` names the stable panel sections
  exposed to native/editor UIs.

## Result HTML

When `runner.result.html_path` is present, `orv editor run-debug` writes
`debug/session-result.html`.

Rules:

- The HTML is a companion render of `debug/session-result.json`.
- The JSON file is the authoritative contract. HTML text/layout can evolve, but
  it must continue rendering selected frame, production summary, source bundle,
  controls, breakpoints, watch expressions, and event sections.

## Version Policy

DAP Debug Session v1 is public to generated deploy smoke, editor exports,
native-host debug panels, and standalone build-dir debug runs. Runner, result,
export-state root, production-context, and production-summary key drift is
rejected instead of being echoed into public results. Breaking key/type changes
require a schema version bump or documented compatibility bridge plus updates to
this file, changelog, and contract regression.
