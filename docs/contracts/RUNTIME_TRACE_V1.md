# Runtime Trace v1 Contract

Producer:

- `orv run-build <dir> --trace deploy/request-trace.json`
- attached runtime trace file writer
- `/__orv/trace/events` EventSource stream

Current regression coverage:

- `crates/orv-runtime/tests/request_trace_contract.rs::request_trace_json_contract_freezes_public_object_keys_and_types`
- `crates/orv-cli/src/tests.rs::editor_trace_rejects_invalid_trace_frame_status_type`
- `crates/orv-cli/src/tests.rs::editor_trace_rejects_invalid_trace_frame_params_type`
- `crates/orv-cli/src/tests.rs::editor_trace_stream_rejects_unwrapped_trace_frame_event`
- `crates/orv-runtime/src/server/tests.rs::request_trace_events_endpoint_emits_per_frame_events`
- generated smoke trace-stream gates for production builds

## Trace File Root

`orv.production.trace` has exactly:

```json
{
  "schema_version": 1,
  "kind": "orv.production.trace",
  "frame_count": 1,
  "frames": []
}
```

Rules:

- `schema_version` is currently `1`.
- `kind` is exactly `orv.production.trace`.
- `frame_count` equals `frames.length`.
- `frames[]` preserves capture order.

## Request Frame

Each `frames[]` item has exactly:

```json
{
  "method": "GET",
  "path": "/ping",
  "status": 200,
  "route_method": "GET",
  "route_path": "/ping",
  "route_origin_id": "ori_...",
  "response_origin_id": "ori_...",
  "params": {},
  "query": {},
  "body": ""
}
```

Rules:

- `method`, `path`, and `body` are strings.
- `status` is an unsigned integer HTTP status.
- `route_method`, `route_path`, `route_origin_id`, and `response_origin_id`
  are strings when known and `null` when unavailable.
- `params` and `query` are objects containing string values.
- `response_origin_id` is the executed `@respond` origin when known.
- Editor trace consumers reject malformed primitive types and non-string
  `params`/`query` values before building reveal navigation.

## EventSource Snapshot

`/__orv/trace/events` starts with:

```text
event: orv:trace
data: {"schema_version":1,"kind":"orv.production.trace",...}
```

The `data` payload follows the trace file root contract above.

## EventSource Frame

Each per-frame event uses:

```text
event: orv:trace.frame
data: {"schema_version":1,"kind":"orv.production.trace.frame","index":0,"frame":{...}}
```

The JSON data has exactly:

```json
{
  "schema_version": 1,
  "kind": "orv.production.trace.frame",
  "index": 0,
  "frame": {}
}
```

Rules:

- `index` is the zero-based capture index.
- `frame` follows the request-frame contract above.
- Raw unwrapped request frames are rejected for `orv:trace.frame`; frame events
  must use this wrapper.
- Existing subscribers receive new frame events after the initial snapshot.

## Version Policy

Runtime trace v1 is the public production request trace schema. Breaking key or
type changes require a schema version bump and updates to file, EventSource,
editor trace, and generated smoke regressions.
