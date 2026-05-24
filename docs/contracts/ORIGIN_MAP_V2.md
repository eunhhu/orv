# OriginMap v2 Contract

Producer:

- `orv origins <file>`
- `orv graph <file>` inside `semantic.origin_map`
- production build artifacts as `origin-map.json`

Current regression coverage:

- `crates/orv-cli/tests/origin_map_contract.rs`
- `crates/orv-compiler/src/tests.rs::origin_map_json_contract_freezes_public_object_keys_and_types`
- `crates/orv-cli/src/tests.rs::verify_build_rejects_origin_map_edge_from_missing_entry`
- `crates/orv-cli/src/tests.rs::verify_build_rejects_origin_map_edge_to_missing_entry`
- `crates/orv-cli/src/tests.rs::verify_build_rejects_server_response_origin_drift`
- `crates/orv-cli/src/tests.rs::verify_build_rejects_project_graph_origin_link_drift`

## Root

```json
{
  "version": 2,
  "entries": [],
  "edges": []
}
```

Rules:

- `version` is currently `2`.
- Root keys are contract keys. New public keys require a changelog entry,
  regression update, and this file update.
- Arrays preserve HIR traversal order.

## Entries

Each `entries[]` item has exactly:

```json
{
  "id": "ori_...",
  "kind": "route",
  "name": "GET /ping",
  "span": { "file": 0, "start": 0, "end": 0 },
  "fingerprint": "..."
}
```

Rules:

- `id` is a stable origin id derived from `kind`, `name`, and `span`.
- `kind` is an executable-origin class such as `domain`, `route`, `function`,
  `call`, or another compiler-owned origin class.
- `name` is the human-readable source/execution label.
- `span.file`, `span.start`, and `span.end` are unsigned integers.
- `fingerprint` is a compact span fingerprint used by production artifacts and
  reveal surfaces.

## Edges

Each `edges[]` item has exactly:

```json
{
  "from": "ori_...",
  "to": "ori_...",
  "kind": "contains"
}
```

Rules:

- `from` and `to` reference `entries[].id`.
- `kind` is currently `contains` or `calls`.
- `contains` edges describe executable nesting discovered from HIR traversal.
- `calls` edges connect call origins to resolved function origins.

## Version Policy

OriginMap v2 is the public source-to-production executable-origin schema. Any
breaking key/type change requires a version bump. Backward-compatible additions
must keep existing keys stable and must update the contract regression before
changing generated artifacts.
