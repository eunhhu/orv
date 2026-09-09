# Editor Trace v1

This contract freezes the editor trace payloads that connect production request
events to source reveal, native-host trace panels, and bounded trace reveal
actions.

It covers:

- `orv editor trace <build-dir> --trace <trace.json>`
- `orv editor trace-stream <build-dir> --events <trace-events.sse>`
- trace-enabled `native-host.json` trace envelope
- `orv editor run-action` trace reveal action result envelope

It builds on:

- Runtime trace input shape: `RUNTIME_TRACE_V1.md`
- Reveal navigation payloads: `REVEAL_PAYLOAD_V1.md`
- Editor export envelope: `EDITOR_SNAPSHOT_EXPORT_V1.md`

Editor trace consumers reject unknown runtime trace root keys, request-frame
keys, and `orv.production.trace.frame` event wrapper keys before building source
navigation. Request frames may include the editor/source navigation extension
keys `db_operation_origin_id` and `commerce_adapter_origin_id`. Consumers also
reject malformed request-frame primitive types and non-string `params`/`query`
values.

`commerce_adapter_origin_id` points at a commerce library/provider adapter
surface. It follows the same generic origin/reveal model as other adapter
targets and does not imply compiler-core-intrinsic payment or shipping semantics.

## Editor Trace Root

`orv editor trace` returns:

```json
{
  "schema_version": 1,
  "kind": "orv.editor.trace",
  "build_dir": "dist",
  "trace": {},
  "live_refresh": {},
  "stream_runner": {},
  "actions": [],
  "action_count": 0,
  "frames": []
}
```

`trace` keys are `path`, `kind`, `frame_count`, and `status_counts`.
`status_counts` keys are `total`, `ok`, `redirect`, `client_error`,
`server_error`, and `other`. `total` must equal `trace.frame_count`.

`live_refresh` keys are `strategy`, `watch`, and, when the build has a stable
listen endpoint, `transport`. File traces use `strategy: "trace-file-hash"`.
Trace streams use `strategy: "event-source-snapshot"`.

`stream_runner` keys are `schema_version`, `kind`, `event_stream`, `command`,
and `transport`.

## Frames

`frames[*]` keys are:

- `index`
- `origin_id`
- `response_origin_id`
- `db_operation_origin_id`
- `commerce_adapter_origin_id`
- `request`
- `summary`
- `reveal_command`
- `response_reveal_command`
- `db_reveal_command`
- `commerce_reveal_command`
- `actions`
- `navigation`
- `response_navigation`
- `db_navigation`
- `commerce_navigation`

`summary` keys are `label`, `route`, `status`, `status_class`, `origin_id`,
`response_origin_id`, `db_operation_origin_id`, and
`commerce_adapter_origin_id`.

Each non-null navigation field is an Editor Reveal payload from
`REVEAL_PAYLOAD_V1.md`.

## Actions

`actions[*]` and `frames[*].actions[*]` keys are:

- `schema_version`
- `kind`
- `action`
- `slot`
- `label`
- `frame_index`
- `origin_id`
- `command`
- `runner_command`
- `focus`
- `target_panel`
- `source`
- `source_path`
- `source_line`
- `production`
- `navigation`

`action` is one of:

- `trace.route.reveal`
- `trace.response.reveal`
- `trace.db.reveal`
- `trace.commerce.reveal`

`schema_version` must be `1`. Stale or unversioned direct reveal action inputs
and selected native-host trace actions are rejected before the allowlisted reveal
command runs.

`command` is the allowlisted reveal command:

```json
["orv", "editor", "reveal", "<build-dir>", "<origin-id>"]
```

`runner_command` is the native-host action runner command:

```json
[
  "orv",
  "editor",
  "run-action",
  "native-host.json",
  "--action",
  "<trace.*.reveal>",
  "--frame-index",
  0,
  "--slot",
  "<route|response|db|commerce>"
]
```

## Trace Stream

`orv editor trace-stream` returns:

```json
{
  "schema_version": 1,
  "kind": "orv.editor.trace.stream",
  "build_dir": "dist",
  "event_stream": {},
  "latest": {},
  "events": []
}
```

`event_stream` keys are `path`, `content_type`, `content_hash`, `event_count`,
`trace_event_count`, and `trace_frame_event_count`.

`events[*]` for `orv:trace.frame` keys are `index`, `event`, `data_bytes`, and
`frame`. `events[*]` for `orv:trace` keys are `index`, `event`, `data_bytes`,
and `trace`.

`orv:trace.frame` event data must use the Runtime Trace v1 frame-event wrapper
with `schema_version`, `kind`, `index`, and `frame`; raw unwrapped request
frames are rejected.

When a trace stream contains a snapshot followed by frame events, `latest`
merges the observed frame events into the current trace view instead of
returning the stale snapshot. Replayed frame events for already-snapshotted
indices must match the snapshot frame payload; new frame events must continue at
the next zero-based index.

`latest` is either `null` or an Editor Trace root payload.

## Native Host Trace

Trace-enabled `native-host.json` includes a `trace` object with keys:

- `schema_version`
- `kind`
- `build_dir`
- `trace_path`
- `frame_count`
- `status_counts`
- `summary`
- `status_filters`
- `frames`
- `actions`
- `action_count`
- `live_refresh`
- `transport`
- `stream_runner`
- `action_runner`
- `action_result_artifact`
- `panel_html_path`
- `panel_artifact`
- `panel_contract`

`trace/panel.html` must exist when trace is enabled. The native host panel
inventory must include `trace` and `trace_action_result`.

## Action Result

`orv editor run-action` returns:

```json
{
  "schema_version": 1,
  "kind": "orv.editor.native_host.action.result",
  "input": "...",
  "execution": {},
  "action": {},
  "command": [],
  "navigation": {},
  "result_artifact": {},
  "panels": {}
}
```

`execution` keys are `kind`, `allowlist`, and `status`. `allowlist` is
`orv.editor.reveal`.

`panels.trace_action` keys are `schema_version`, `summary`, `action`, `command`,
`navigation`, `source`, `production`, and `result_artifact`.

When the input is an export directory, the runner writes:

- `trace/action-result.json`
- `trace/action-result.html`

## Version Policy

- `schema_version: 1` is append-only for optional fields.
- Removing or renaming any key listed here requires a new contract file and
  migration note.
- Action execution remains allowlist-bound to `orv editor reveal`; broader
  command execution requires a new contract.

## Regression Coverage

- `docs/samples/editor-trace-inventory-v1.golden.json` freezes the normalized
  editor trace, trace-stream, native-host trace, and action-result inventory.
- `crates/orv-cli/src/tests/editor_trace.rs::editor_trace_rejects_extra_trace_root_key`
- `crates/orv-cli/src/tests/editor_trace.rs::editor_trace_rejects_missing_trace_frame_count`
- `crates/orv-cli/src/tests/editor_trace.rs::editor_trace_rejects_trace_root_version_and_kind_drift`
- `crates/orv-cli/src/tests/editor_trace.rs::editor_trace_rejects_missing_trace_frame_base_key`
- `crates/orv-cli/src/tests/editor_trace.rs::editor_trace_rejects_extra_trace_frame_key`
- `crates/orv-cli/src/tests/editor_trace.rs::editor_trace_rejects_invalid_trace_frame_origin_id_types`
- `crates/orv-cli/src/tests/editor_trace.rs::editor_trace_stream_rejects_extra_trace_frame_event_key`
- `crates/orv-cli/src/tests/editor_trace.rs::editor_trace_stream_rejects_trace_frame_event_version_and_kind_drift`
- `crates/orv-cli/src/tests/editor_trace.rs::editor_trace_stream_applies_frame_events_after_snapshot_to_latest`
- `crates/orv-cli/src/tests/editor_trace.rs::editor_trace_stream_appends_live_frame_after_snapshot_replay`
- `crates/orv-cli/src/tests/editor_trace.rs::editor_trace_stream_rejects_snapshot_replay_frame_drift`
- `crates/orv-cli/src/tests/editor_trace.rs::editor_trace_stream_rejects_live_frame_gap_after_snapshot_replay`

- `crates/orv-cli/tests/editor_trace_contract.rs` is a CLI black-box regression.
  It builds a production fixture, compares the published inventory golden,
  freezes editor trace and trace-stream key surfaces, verifies native-host trace
  panel artifacts, runs an allowlisted trace reveal action, and checks the
  action result artifact envelope.
