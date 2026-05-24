# Build Artifacts v1 Contract

Build Artifacts v1 freezes the common `orv build` artifact set that other
surfaces consume before deploy-specific artifacts are generated.

## Producer

```text
orv build <file-or-dir> --out <dir>
```

Production builds with `--prod` produce this same common artifact set and then
add deploy artifacts covered by [Deploy Artifacts v1](DEPLOY_ARTIFACTS_V1.md).

## Artifact Set

Every successful build writes these common artifacts:

- `build-manifest.json`
- `bundle-plan.json`
- `origin-map.json`
- `project-graph.json`
- `source-bundle.json`

Server, static page, client, and native-plan files are target-specific bundle
outputs recorded by `build-manifest.json` and `bundle-plan.json`.

## Build Manifest

`build-manifest.json` has exactly:

```json
{
  "schema_version": 1,
  "entry": "app.orv",
  "runtime": "reference-interpreter",
  "artifacts": [],
  "capabilities": {}
}
```

`capabilities` has exactly:

```json
{
  "has_server": true,
  "server_routes": 1,
  "client_wasm": false,
  "runtime_features": []
}
```

`capabilities` is derived from the OriginMap v2 content and must not drift from
the generated route/client/runtime feature inventory.

Each `artifacts[]` item has exactly:

```json
{
  "kind": "origin_map",
  "path": "origin-map.json"
}
```

Required common artifact descriptors:

- `origin_map` at `origin-map.json`
- `bundle_plan` at `bundle-plan.json`
- `project_graph` at `project-graph.json`
- `source_bundle` at `source-bundle.json`

When a build has server output, the manifest also records `server_runtime` at
`server/app.orv-runtime.json`. Static and client outputs are recorded by their
own contract docs.

## Bundle Plan

`bundle-plan.json` has exactly:

```json
{
  "schema_version": 1,
  "bundles": []
}
```

Each `bundles[]` item has exactly:

```json
{
  "kind": "server_runtime",
  "path": "server/app.orv-runtime.json",
  "runtime_features": []
}
```

Bundle targets are the authoritative list used by `orv verify-build` and
`orv run-build` to decide which runtime/static/client outputs must exist.
Each target's `runtime_features` must match the runtime artifact, static page,
or client WASM target contract for that target kind.
The full bundle plan must match the plan regenerated from the OriginMap-derived
build manifest, so paired manifest/plan drift is still rejected.

## Source Bundle

`source-bundle.json` has exactly:

```json
{
  "schema_version": 1,
  "entry": "app.orv",
  "files": []
}
```

Each `files[]` item has exactly:

```json
{
  "path": "app.orv",
  "content_hash": "fnv1a64:...",
  "source": "@server { ... }"
}
```

Rules:

- `content_hash` uses the `fnv1a64:` prefix.
- `files[]` preserves source-bundle producer order.
- `orv verify-build` compares this artifact against server and graph metadata
  before reveal or smoke tooling trusts production output.

## Graph And Origin Artifacts

- `origin-map.json` follows [OriginMap v2](ORIGIN_MAP_V2.md).
- `project-graph.json` follows [ProjectGraph v1](PROJECT_GRAPH_V1.md).
- `project-graph.json` embeds the same origin map under
  `semantic.origin_map`.

## Verification

`orv verify-build <dir>` must accept a fresh Build Artifacts v1 output and fail
if a required manifest artifact, bundle target, source bundle, graph, or origin
map drifts from the generated build.

## Version Policy

Build Artifacts v1 is a public build/reveal/deploy foundation contract. Breaking
key/type changes in the common artifact set require a new contract version or a
documented compatibility bridge.

## Regression Coverage

- `crates/orv-cli/tests/build_artifacts_contract.rs` freezes the public
  black-box `orv build` and `orv verify-build` behavior for the common artifact
  set.
- `crates/orv-cli/src/tests.rs::verify_build_rejects_build_manifest_extra_capability_key`
  and `::verify_build_rejects_bundle_plan_extra_root_key` cover nested
  capabilities/root-key drift rejection for the common artifact set.
- `crates/orv-cli/src/tests.rs::verify_build_rejects_build_manifest_capability_value_drift`
  and `::verify_build_rejects_bundle_target_runtime_features_drift` cover
  capability value drift and bundle target runtime feature drift.
- `crates/orv-cli/src/tests.rs::verify_build_rejects_bundle_plan_and_manifest_paired_drift`
  covers paired build manifest and bundle plan drift.
- `crates/orv-cli/src/tests.rs::verify_build_rejects_source_bundle_content_hash_drift`
  and `::verify_build_rejects_source_bundle_entry_drift` cover source bundle
  integrity and runtime/source-bundle entry linkage drift.
- ProjectGraph v1 and OriginMap v2 have dedicated contract regressions for their
  nested public shapes.
