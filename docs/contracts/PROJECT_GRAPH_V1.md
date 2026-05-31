# ProjectGraph v1 Contract

Producer:

- `orv graph <file>`
- `orv graph <file> --view --out <dir>` as `graph.json`
- production build graph mirrors that embed the same source/semantic spine

Current regression coverage:

- `docs/samples/project-graph-v1.golden.json`
- `crates/orv-cli/src/tests.rs::graph_json_contract_freezes_public_object_keys_and_types`
- `crates/orv-cli/src/tests.rs::verify_build_rejects_project_graph_origin_link_drift`
- `crates/orv-cli/src/tests.rs::verify_build_rejects_project_graph_stat_content_drift`
- `crates/orv-cli/tests/project_graph_contract.rs`

## Root

```json
{
  "schema_version": 1,
  "stats": {},
  "nodes": [],
  "edges": [],
  "semantic": {}
}
```

Rules:

- `schema_version` is the ProjectGraph artifact schema version. It is currently
  `1`.
- Root keys are contract keys. New public keys require a changelog entry,
  regression update, and this file update.
- Arrays preserve producer order. Consumers must not sort before resolving ids.

## Stats

```json
{
  "node_count": 0,
  "edge_count": 0,
  "file_count": 0,
  "import_count": 0,
  "declaration_count": 0,
  "domain_count": 0,
  "max_source_contains_depth": 0,
  "semantic_origin_count": 0,
  "semantic_edge_count": 0,
  "semantic_call_edge_count": 0,
  "max_semantic_contains_depth": 0
}
```

All stat values are unsigned integers.
The CLI/view ProjectGraph v1 contract also requires these published stat values to match the actual `nodes`, `edges`, and `semantic.origin_map` content.
In production build verification, all stat values must match the graph content:
source node kind counts, source contains depth, semantic origin/edge/call counts,
and semantic contains depth are recomputed before reveal or smoke tooling trusts
the graph.

## Nodes

Each `nodes[]` entry has exactly:

```json
{
  "id": 0,
  "kind": "file",
  "name": "app.orv",
  "file": 0,
  "span": { "file": 0, "start": 0, "end": 0 }
}
```

Rules:

- `id` is a numeric ProjectGraph node id.
- The published golden fixture normalizes the file node `name` to
  `<workspace>/fixtures/e2e/hello.orv`; producer output uses the local absolute
  path for the selected source file.
- `kind` is one of `file`, `import`, `struct`, `enum`, `type_alias`,
  `function`, `define`, or `domain`.
- `file` and `span.file` are numeric file ids from the loaded project.
- In production build verification, `file` and `span.file` must match and
  reference a file present in `source-bundle.json`.
- `span.start` and `span.end` are byte offsets.
- `span.start <= span.end`, and `span.end` must not exceed the referenced
  source-bundle file byte length.

## Edges

Each `edges[]` entry has exactly:

```json
{
  "from": 0,
  "to": 1,
  "kind": "contains"
}
```

Rules:

- `from` and `to` reference `nodes[].id`.
- `kind` is `contains` or `imports`.

## Semantic

```json
{
  "origin_map": {},
  "origin_edges": [],
  "origin_links": []
}
```

Rules:

- `semantic.origin_map` follows [OriginMap v2](ORIGIN_MAP_V2.md).
- `semantic.origin_edges[]` mirrors origin-map edges as objects with exactly
  `kind`, `from`, and `to`, all strings.
- `semantic.origin_links[]` ties source graph nodes to executable origins.

Each `origin_links[]` entry has exactly:

```json
{
  "kind": "source_node",
  "origin_id": "ori_...",
  "node_id": 0
}
```

Rules:

- `origin_id` references `semantic.origin_map.entries[].id`.
- `node_id` references `nodes[].id`.
- `kind` is currently `source_node`.

## Version Policy

ProjectGraph v1 may add internal producers, but public JSON shape changes require
a schema version bump or an explicitly backward-compatible addition documented
here. Drift checks must keep build-time `project-graph.json`, `source-bundle`,
and `origin-map.json` aligned before reveal or smoke tooling trusts production
metadata.
