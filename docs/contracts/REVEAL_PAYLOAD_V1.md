# Reveal Payload v1

`orv reveal`, `orv editor reveal`, and `orv lsp reveal` are the public
navigation bridge from generated artifacts back to source. This contract freezes
the JSON keys that editor, native-host, smoke, and LSP consumers may rely on.

## Producers

- `orv reveal <build-dir> <origin-id>`
- `orv editor reveal <build-dir> <origin-id>`
- `orv lsp reveal <build-dir> <origin-id>`
- editor trace/native-host reveal actions that execute `orv editor reveal`

All producers read the same build graph spine:

- `origin-map.json`
- `project-graph.json`
- `source-bundle.json` when present
- production artifact summaries under `server/`, `client/`, and `deploy/`

## CLI Reveal Root

`orv reveal` returns:

```json
{
  "schema_version": 1,
  "origin": {},
  "source": {},
  "project_graph": {},
  "production": {}
}
```

`origin` is an OriginMap v2 entry. `project_graph` is the ProjectGraph v1 node
linked to that origin when one exists, otherwise `null`.

`source` has these keys:

| Key | Type | Notes |
|-----|------|-------|
| `file` | number | Source file id from the origin span |
| `path` | string or null | Source path from project graph or source bundle |
| `start` | number | Byte start from the origin span |
| `end` | number | Byte end from the origin span |
| `snippet` | string or null | Source slice for the selected span |
| `content` | string or null | Full source file content when available |

## Editor Reveal Root

`orv editor reveal` returns:

```json
{
  "schema_version": 1,
  "origin": {},
  "focus": {},
  "source": {},
  "project_graph": {},
  "production": {}
}
```

`focus` has:

| Key | Type | Notes |
|-----|------|-------|
| `origin_id` | string | Requested origin id |
| `panel` | string | `routes`, `schema`, `domains`, or `source` |
| `node_id` | number or null | ProjectGraph node id for focused UI selection |

`source` has:

| Key | Type | Notes |
|-----|------|-------|
| `file` | number | Source file id from the origin span |
| `path` | string | Source path used by editor consumers |
| `snippet` | string or null | Source slice for the selected span |
| `location` | object | LSP-style `{ uri, range }` navigation target; `uri` is a `file://` URI |

## LSP Reveal Root

`orv lsp reveal` returns:

```json
{
  "schema_version": 1,
  "origin": {},
  "location": {},
  "project_graph": {},
  "production": {}
}
```

`location` has `uri` and `range`; `uri` is a `file://` URI. `range` has
`start` and `end`; each position has `line` and `character`.

## Production

The published production summary golden fixture is
`docs/samples/reveal-production-summary-v1.golden.json`. It normalizes
`production.summary.build_dir` to `<build-dir>`.

The published coverage golden fixture is
`docs/samples/reveal-coverage-v1.golden.json`. It freezes normalized
route/html/db/commerce/trace, function/domain call-chain, and static graph-view
origin-spine inventories without embedding temp paths or generated origin ids.

`commerce` reveal targets are library/provider-surface adapter targets. They do
not mean the compiler has payment, shipping, Stripe, or carrier intrinsic
semantics; reveal follows generic adapter/source-origin metadata.

All three reveal surfaces expose the same `production` object:

| Key | Type | Notes |
|-----|------|-------|
| `graph_contract` | array | Source bundle, ProjectGraph, and OriginMap target summaries |
| `routes` | array | Route targets matching the selected origin |
| `native_server` | array | Native server plan/source/runtime-image summaries |
| `preflight` | array | Deploy preflight, smoke, benchmark, and env summaries |
| `static` | array | Static page targets matching the selected origin |
| `db_adapters` | array | DB adapter artifact summaries and selected-origin matches |
| `commerce_adapters` | array | Commerce library adapter summaries and selected-origin matches |
| `client` | array | Client manifest/reactive-plan/page targets matching the selected origin |
| `summary` | object | Count-only rollup for smoke/editor panels |

`preflight[].benchmark_evidence` uses the same benchmark-report status rules as
`orv benchmark-report . --require-pass`, including `recording_status`,
failure-classification, smoke-marker, participant minimum, and retained raw-notes
artifact gates. It also carries `participant_raw_notes_artifacts[]` so reveal
surfaces show the same per-run raw-notes retained/non-empty/template-filled
status as benchmark reports. Smoke-output artifact parity is surfaced through
`smoke_test_output_source`, `smoke_test_output_artifact_path`, and
`smoke_test_output_artifact_match`.

`routes[*]` keys are `artifact`, `method`, `path`, `origin_id`, `match`,
`matched_origin_id`, and `policies`. `match` is `direct`, `contains`, or
`calls`. Route policies preserve server runtime policy descriptors, including
the `surface` value that distinguishes first-party compiler plugin policies
from shop-template and provider-package-template defaults.

`db_adapters[*]` and `commerce_adapters[*]` keys are `kind`, `path`, `exists`,
`selected_origin_id`, `matched`, `matched_adapter_count`, `artifact`,
`adapters`, `source_reveal_commands`, and `matched_adapters`. If the deploy
manifest references an adapter artifact that is missing on disk, the target
still preserves this full key set with `exists: false`, no matches,
`artifact: null`, and empty arrays.

`source_reveal_commands[*]` keys are `adapter_index`, `kind`, `provider`,
`env`, `endpoint`, `record_path`, `source_origin_id`, and `command`.

`graph_contract[]` contains one target for each build graph spine artifact:

- `kind: "source_bundle"` keys are `schema_version`, `kind`, `path`, `exists`,
  `entry`, `file_count`, `files`, and `artifact_hash`; `files[*]` keys are
  `path` and `content_hash`.
- `kind: "project_graph"` keys are `schema_version`, `kind`, `path`, `exists`,
  `stats`, `node_count`, `edge_count`, `semantic_origin_count`,
  `semantic_edge_count`, `semantic_origin_link_count`, and `artifact_hash`.
- `kind: "origin_map"` keys are `version`, `kind`, `path`, `exists`,
  `entry_count`, `edge_count`, `call_edge_count`, and `artifact_hash`.

## Version Policy

- `schema_version: 1` is append-only for optional fields.
- Removing or renaming any key listed here requires a new contract file and
  migration note.
- Primitive type changes require a new contract version.
- Array ordering follows generated artifact ordering and must remain stable
  for deterministic builds.

## Regression Coverage

- `docs/samples/reveal-production-summary-v1.golden.json`
- `docs/samples/reveal-coverage-v1.golden.json`
- `crates/orv-cli/tests/reveal_payload_contract.rs` freezes the public root,
  source/focus/location, graph-contract target, production summary, route,
  adapter, and reveal-command key surfaces across CLI, editor, and LSP
  producers. It also compares `production.summary` against the published golden
  fixture.
- `crates/orv-cli/tests/reveal_coverage_contract.rs` verifies route, HTML, DB,
  commerce, function, domain, graph-view, and trace reveal behavior over
  production builds, and compares normalized coverage inventories against the
  published golden fixture.
