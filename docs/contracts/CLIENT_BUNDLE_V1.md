# Client Bundle v1 Contract

Producer:

- `orv build <file-or-project> --out <dir>` for interactive entries that use
  `let sig` or client-bound HTML
- production builds mirror the same files through `deploy/manifest.json`,
  reveal/editor/LSP payloads, and generated deploy smoke checks

Current regression coverage:

- `crates/orv-cli/src/tests.rs::client_bundle_contract_freezes_public_object_keys_and_types`
- `crates/orv-cli/src/tests.rs::build_writes_client_wasm_for_signal_html_entry`
- `crates/orv-cli/src/tests.rs::verify_build_rejects_client_manifest_*`
- `crates/orv-cli/src/tests.rs::verify_build_rejects_client_reactive_plan_*`
- `crates/orv-cli/src/tests.rs::verify_build_rejects_deploy_client_*`

## Manifest Root

`client/manifest.json` has exactly:

```json
{
  "schema_version": 1,
  "kind": "orv.client.bundle",
  "entry": "page.orv",
  "page": "pages/index.html",
  "reactive_plan": "client/reactive-plan.json",
  "reactive_plan_hash": "fnv1a64:...",
  "loader": "client/app.js",
  "loader_hash": "fnv1a64:...",
  "wasm": "client/app.wasm",
  "wasm_hash": "fnv1a64:...",
  "source_bundle": "source-bundle.json",
  "source_bundle_hash": "fnv1a64:...",
  "exports": {},
  "initial_render": {},
  "runtime_features": [],
  "capabilities": {},
  "blocked_by": [],
  "blockers": []
}
```

Rules:

- `schema_version` is currently `1`.
- `kind` is `orv.client.bundle`.
- `page`, `reactive_plan`, `loader`, `wasm`, and `source_bundle` are checked
  paths in the build directory.
- Hash fields are stable JSON/file hashes checked by `orv verify-build`.
- `blocked_by` keeps the stable blocker id list; `blockers[]` carries structured
  blocker details.

## Exports

`exports` has exactly:

```json
{
  "start": "orv_start",
  "render_ptr": "orv_render_ptr",
  "render_len": "orv_render_len",
  "memory": "memory"
}
```

These are the current WASM ABI symbols. `orv verify-build` rejects drift between
the manifest and `client/app.wasm`.

## Initial Render

`initial_render` has exactly:

```json
{
  "content_type": "text/html",
  "encoding": "utf-8",
  "html_hash": "fnv1a64:...",
  "byte_length": 0
}
```

Rules:

- The manifest initial-render metadata must match the WASM `orv.client` custom
  section and the generated page shell.

## Capabilities

`capabilities` has exactly:

```json
{
  "schema_version": 1,
  "runtime": "client_wasm",
  "source": "client/reactive-plan.json",
  "signals": 1,
  "bindings": {},
  "surfaces": [],
  "event_actions": []
}
```

`capabilities.bindings` has exactly:

```json
{
  "initial_render": 1,
  "signal_state": 1,
  "signal_text": 1,
  "signal_attr": 0,
  "signal_event": 1,
  "total": 4
}
```

Rules:

- Capabilities are derived from `client/reactive-plan.json`.
- `orv verify-build` rejects manifest capability drift from the reactive plan.
- Deploy and reveal payloads mirror this object for client origins.

## Reactive Plan Root

`client/reactive-plan.json` has exactly:

```json
{
  "schema_version": 1,
  "kind": "orv.client.reactive_plan",
  "entry": "page.orv",
  "source_bundle": "source-bundle.json",
  "source_bundle_hash": "fnv1a64:...",
  "runtime_features": [],
  "signals": [],
  "bindings": [],
  "blocked_by": [],
  "blockers": []
}
```

Rules:

- `kind` is `orv.client.reactive_plan`.
- `source_bundle` and `source_bundle_hash` must match the build-level source
  bundle.
- `blocked_by` currently includes `reactive-dom-diff` until full DOM diff
  codegen exists.

## Signals

Each `signals[]` entry has exactly:

```json
{
  "origin_id": "ori_...",
  "name": "count",
  "state_key": "count",
  "initial_value": 0,
  "span": { "file": 0, "start": 0, "end": 0 }
}
```

Rules:

- `origin_id` links back to OriginMap v2 and ProjectGraph v1.
- `state_key` is the key used by the JS loader state object.
- `span` uses byte offsets from the source bundle file id.

## Bindings

Common binding variants have these key sets.

`initial_render`:

```json
{
  "kind": "initial_render",
  "source": "client/app.wasm",
  "target": "pages/index.html",
  "html_hash": "fnv1a64:...",
  "byte_length": 0
}
```

`signal_state`:

```json
{
  "kind": "signal_state",
  "source": "ori_...",
  "target": "client/app.js",
  "state_key": "count"
}
```

`signal_text`:

```json
{
  "kind": "signal_text",
  "source": "ori_...",
  "target": "pages/index.html",
  "selector": "p",
  "state_key": "count",
  "span": { "file": 0, "start": 0, "end": 0 }
}
```

`signal_event`:

```json
{
  "kind": "signal_event",
  "source": "ori_...",
  "target": "pages/index.html",
  "selector": "button",
  "state_key": "count",
  "span": { "file": 0, "start": 0, "end": 0 },
  "event": "click",
  "action": {}
}
```

`signal_event.action` currently has at least:

```json
{
  "kind": "assign_add",
  "value": { "kind": "int", "value": "1" }
}
```

Additional binding/action variants are allowed only with matching
`orv verify-build` validation, changelog, contract doc, and regression updates.

## Blockers

Each `blockers[]` entry has exactly:

```json
{
  "id": "dynamic-client-codegen",
  "artifact": "client/manifest.json",
  "reason": "..."
}
```

Rules:

- `blockers[].id` must correspond to a `blocked_by[]` entry.
- Manifest blocker ids currently include `dynamic-client-codegen`.
- Reactive-plan blocker ids currently include `reactive-dom-diff`.

## Loader/WASM Coupling

`client/app.js` must verify manifest path/hash/export metadata, embedded or
fetched reactive-plan metadata, source-bundle hash/count metadata, WASM hash,
initial-render hash/length, and signal binding counts before mounting.

`client/app.wasm` must:

- be a valid WASM module,
- carry an `orv.client` custom section,
- export `orv_start`, `orv_render_ptr`, `orv_render_len`, and `memory`,
- carry source-bundle and initial-render metadata matching the manifest.

## Version Policy

Client Bundle v1 is a public build/deploy/reveal contract. Breaking key/type
changes require a schema version bump or documented compatibility bridge plus
updates to this file, changelog, `orv verify-build`, generated smoke checks, and
contract regression.
