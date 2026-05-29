# Editor Snapshot/Export v1

This contract freezes the first-party editor bootstrap artifacts that are shared
by the static editor shell, native-host handoff, smoke checks, and follow-on DAP
or reveal actions.

The published snapshot golden fixture is
`docs/samples/editor-snapshot-v1.golden.json`. It normalizes only entry
path/URI values and the path-derived project graph hash.
The published export command-output golden fixture is
`docs/samples/editor-export-output-v1.golden.json`. It normalizes only the
local `entry` and `out` paths.

It covers:

- `orv editor snapshot <entry>`
- `orv editor export <entry> --out <dir>`
- `orv editor export <entry> --out <dir> --build <build-dir>`
- `state.json`
- `native-host.json`
- the static shell and runtime/production panel artifact envelope

It does not replace these narrower contracts:

- DAP debug runner/result bodies: `DAP_DEBUG_SESSION_V1.md`
- native desktop package/session: `NATIVE_HOST_DESKTOP_V1.md`
- source reveal navigation payloads: `REVEAL_PAYLOAD_V1.md`

## Snapshot

`orv editor snapshot <entry>` returns:

```json
{
  "schema_version": 1,
  "entry": {},
  "diagnostics": [],
  "project_graph": {},
  "live_refresh": {},
  "panels": {}
}
```

`entry` keys are `path` and `uri`.

`live_refresh` keys are `strategy`, `project_graph_hash`, and `watch`.
`strategy` is `source-hash`. `watch.sources[*]` keys are `file`, `path`,
`uri`, and `content_hash`.

`panels` keys are:

- `files`
- `routes`
- `schema`
- `domains`

Panel item keys:

- `files[*]`: `file`, `name`, `path`, `uri`, `node_id`
- `routes[*]`: `origin_id`, `method`, `path`, `name`, `location`
- `schema[*]`: `node_id`, `kind`, `name`, `location`
- `domains[*]`: `node_id`, `kind`, `name`, `location`

## Export Command Output

`orv editor export` prints:

```json
{
  "schema_version": 1,
  "kind": "orv.editor.export",
  "entry": "path/to/entry.orv",
  "out": "path/to/export",
  "files": []
}
```

`files` lists the generated paths relative to the export directory. The stable
base set includes `index.html`, `state.json`, `debug/session-runner.json`,
`native-host.json`, `native-host/bridge.js`, and native desktop package files.
When production build metadata is supplied, `production/panel.html` is also
listed.

## State Root

`state.json` root keys:

| Key | Type | Notes |
|-----|------|-------|
| `schema_version` | number | Always `1` |
| `kind` | string | `orv.editor.export` |
| `snapshot` | object | Same shape as snapshot output |
| `runtime` | object | Runtime inspection payload |
| `debug` | object | DAP runner bootstrap metadata |
| `production` | object | Present when `--build` is supplied |
| `trace` | object | Present when `--trace` is supplied |

The v1 production export contract freezes the build-backed `production` object
as the same production summary shape used by Reveal Payload v1: `graph_contract`,
`client`, `native_server`, `static`, `preflight`, `db_adapters`,
`commerce_adapters`, and `summary`.

## Native Host Manifest

`native-host.json` root keys:

- `schema_version`
- `kind`
- `entry`
- `artifacts`
- `debug`
- `runtime`
- `production`
- `trace`
- `host`
- `panels`
- `capabilities`

`artifacts` names generated files. The production export contract requires
`production_panel_html` when `--build` is supplied and always requires
`runtime_panel_html`, `debug_session_runner`, `native_host_bridge_js`,
`native_host_desktop_package`, and shell/state paths.

`panels[*]` keys are `name`, `title`, `root`, `artifact`, and `panel_contract`.
The base panel inventory includes `debug_result` and `runtime`; build-backed
exports also include `production`.

`host` keys are `schema_version`, `kind`, `shell`, `bridge_script`,
`desktop_package`, `desktop_launcher`, `desktop_platform_matrix`,
`desktop_app`, `desktop_packaging`, `action_endpoint`, and `command_format`.

`capabilities` exposes booleans for graph, runtime, DAP, production, trace, and
native-host handoff surfaces. New optional capability flags may be appended in
schema version 1.

Native-host action execution rejects unknown `native-host.json` root keys before
selecting a trace reveal action, so drifted handoff manifests are not accepted
silently.

## Static Artifacts

Required files for a production-backed export:

- `index.html`
- `state.json`
- `debug/session-runner.json`
- `native-host.json`
- `native-host/bridge.js`
- `runtime/panel.html`
- `production/panel.html`

`index.html` must embed the static editor shell root and load
`native-host/bridge.js`.

## Version Policy

- `schema_version: 1` is append-only for optional fields.
- Removing or renaming any key listed here requires a new contract file and a
  migration note.
- Nested DAP debug result, desktop package/session, and reveal payload changes
  follow their own contract files.
- `trace` is optional in v1 and will be promoted separately when trace panel
  payloads are frozen as their own product contract.

## Regression Coverage

- `crates/orv-cli/tests/editor_snapshot_export_contract.rs` is a CLI black-box
  regression. It runs snapshot/export commands, compares the normalized
  snapshot and export command-output payloads against the published golden
  fixtures, freezes public root and nested envelope keys, verifies the
  production panel handoff, checks native desktop package files are listed in
  the export output and native-host artifact map, and checks required static
  artifacts are written.
- `crates/orv-cli/tests/editor_trace_contract.rs::editor_run_action_rejects_extra_native_host_manifest_root_key`
  covers native-host action input rejection for drifted manifest roots.
