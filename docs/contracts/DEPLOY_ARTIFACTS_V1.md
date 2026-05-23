# Deploy Artifacts v1 Contract

Producer:

- `orv build <file-or-dir> --prod --out <dir>`
- shop template production builds under `dist/`

Current regression coverage:

- `crates/orv-cli/tests/deploy_schema_contract.rs::prod_build_deploy_and_benchmark_json_contracts_freeze_public_shape`
- `crates/orv-cli/src/tests.rs::verify_build_rejects_deploy_preflight_*`
- `crates/orv-cli/src/tests.rs::verify_build_rejects_deploy_benchmark_evidence_*`
- `crates/orv-cli/src/tests.rs::benchmark_report_*`
- the same CLI contract regression runs `orv deploy-env-check` against the
  generated production artifact set.

This contract covers the public deploy/preflight/benchmark JSON roots that
external smoke, editor, and native-host tooling consume. Common `orv build`
artifacts are covered by [Build Artifacts v1](BUILD_ARTIFACTS_V1.md). This file
keeps summary copies of those roots because production deploy artifacts link
them directly. It does not document every nested runtime/server route field;
those nested fields are owned by the server runtime artifact and linked
contracts.

## Build Manifest

`build-manifest.json` has exactly:

```json
{
  "schema_version": 1,
  "entry": "app.orv",
  "runtime": "reference",
  "artifacts": [],
  "capabilities": {}
}
```

Rules:

- `schema_version` is currently `1`.
- `artifacts` is an array of generated build artifact descriptors.
- `capabilities` is an object consumed by editor/build surfaces.

## Source Bundle

`source-bundle.json` has exactly:

```json
{
  "schema_version": 1,
  "entry": "app.orv",
  "files": []
}
```

Rules:

- `files[]` preserves source-bundle producer order.
- Build verification compares server/deploy source snapshots and ProjectGraph
  file nodes against this bundle before reveal tooling trusts production output.

## Bundle Plan

`bundle-plan.json` has exactly:

```json
{
  "schema_version": 1,
  "bundles": []
}
```

Rules:

- `bundles[]` describes generated server/static/client targets.
- Static deploy targets must match the bundle-plan `static_page` target.

## Deploy Manifest

`deploy/manifest.json` has exactly:

```json
{
  "schema_version": 1,
  "profile": "prod",
  "entry": "app.orv",
  "runtime": "reference",
  "runtime_features": {},
  "source_bundle": "source-bundle.json",
  "server": {},
  "static": null,
  "client": null
}
```

When `server` is present, its public keys are:

```json
{
  "runtime": "reference",
  "runtime_features": {},
  "artifact": "server/app.orv-runtime.json",
  "entrypoint": "deploy/server.sh",
  "routes_artifact": "deploy/routes.json",
  "native_plan": "server/native-server.json",
  "native_runtime_image_plan": "server/runtime-image.json",
  "native_routes_source": "server/native/routes.rs",
  "native_router_source": "server/native/router.rs",
  "native_handlers_source": "server/native/handlers.rs",
  "container": "deploy/container.json",
  "dockerfile": "deploy/Dockerfile",
  "compose": "deploy/compose.yaml",
  "env_example": "deploy/env.example",
  "db_adapters": "deploy/db-adapters.json",
  "commerce_adapters": "deploy/commerce-adapters.json",
  "smoke_test": "deploy/smoke-test.sh",
  "smoke_output": "deploy/smoke-output.txt",
  "preflight": "deploy/preflight.json",
  "benchmark_evidence": "deploy/benchmark-evidence.json",
  "runbook": "deploy/README.md",
  "runtime_image": "server/runtime-image.json",
  "protocol": "http/1.1",
  "listen": {},
  "routes": [],
  "persistence": {}
}
```

## Deploy Preflight

`deploy/preflight.json` has exactly:

```json
{
  "schema_version": 1,
  "kind": "orv.deploy.preflight",
  "artifact": "server/app.orv-runtime.json",
  "runtime": "reference",
  "runtime_features": {},
  "security_features": {},
  "listen": {},
  "routes": [],
  "persistence": {},
  "required_env": [],
  "optional_env": [],
  "commands": {},
  "artifacts": {},
  "smoke_output_contract": {},
  "benchmark": {},
  "client": null
}
```

`commands` has exactly:

```json
{
  "verify_build": "orv verify-build .",
  "env_check": "orv deploy-env-check .",
  "run_build": "orv run-build .",
  "smoke_test": "./deploy/smoke-test.sh",
  "editor_run_debug": "orv editor run-debug . --control next",
  "benchmark_report": "orv benchmark-report .",
  "benchmark_report_require_pass": "orv benchmark-report . --require-pass",
  "compose_up": "docker compose -f deploy/compose.yaml up --build -d",
  "trace": "./deploy/server.sh --trace deploy/request-trace.json",
  "trace_run_build": "orv run-build . --trace deploy/request-trace.json",
  "editor_trace": "orv editor trace . --trace deploy/request-trace.json",
  "trace_stream_smoke": "ORV_SMOKE_TRACE_STREAM=1 ./deploy/smoke-test.sh"
}
```

`artifacts` has exactly:

```json
{
  "server": "server/app.orv-runtime.json",
  "routes": "deploy/routes.json",
  "source_bundle": "source-bundle.json",
  "project_graph": "project-graph.json",
  "origin_map": "origin-map.json",
  "build_manifest": "build-manifest.json",
  "bundle_plan": "bundle-plan.json",
  "env_example": "deploy/env.example",
  "db_adapters": "deploy/db-adapters.json",
  "commerce_adapters": "deploy/commerce-adapters.json",
  "smoke_test": "deploy/smoke-test.sh",
  "smoke_output": "deploy/smoke-output.txt",
  "preflight": "deploy/preflight.json",
  "benchmark_evidence": "deploy/benchmark-evidence.json",
  "runbook": "deploy/README.md"
}
```

`smoke_output_contract` has exactly:

```json
{
  "output": "deploy/smoke-output.txt",
  "required_markers": []
}
```

## Deploy Env Check

`orv deploy-env-check <dir>` consumes the generated production artifact set and
must accept a fresh build when all required env values are either absent from the
program or satisfied by generated defaults. Required env failures are covered by
feature-specific DB/commerce/provider regressions.

## Benchmark Evidence

`deploy/benchmark-evidence.json` has exactly:

```json
{
  "schema_version": 1,
  "kind": "orv.benchmark.shop_5h.evidence",
  "preflight": "deploy/preflight.json",
  "preflight_hash": "...",
  "benchmark": {},
  "commands": {},
  "artifacts": {},
  "smoke_output_contract": {},
  "recording_status": "not_recorded",
  "task_entries": [],
  "data": {}
}
```

Rules:

- `preflight_hash` is the stable JSON hash of the expected preflight artifact.
- `benchmark`, `commands`, `artifacts`, and `smoke_output_contract` must match
  the generated preflight contract.
- `task_entries[]` must match the 5-hour task budget and include
  `elapsed_minutes`, `status`, and `notes`.
- `task_entries[].elapsed_minutes` is either `null` while unrecorded or a
  non-negative number when recorded; negative elapsed time fails benchmark
  reports.
- `task_entries[].status` and `participant_runs[].status` must be one of
  `not_recorded`, `missing`, `todo`, `incomplete`, `recorded`, `passed`, `pass`,
  `failed`, `fail`, or `blocked`.
- `data` must include `elapsed_time_per_task`, `docs_help_lookups`,
  `compiler_runtime_errors`, `first_error_to_fix_minutes`,
  `ai_assistance_used`, `generated_artifact_edits`,
  `manual_undocumented_security_steps`, `manual_config_edits`,
  `smoke_test_output`, `smoke_test_required_markers`,
  `recommended_participant_count`, `participant_runs`, `failure_classification`,
  and `participant_notes`.
- `docs_help_lookups` and `compiler_runtime_errors` are either `null` while
  evidence is unrecorded or non-negative integers when recorded; negative or
  non-integer values fail benchmark reports.
- `first_error_to_fix_minutes` is either `null` or a non-negative number;
  negative values fail benchmark reports.
- `ai_assistance_used` is either `null` while evidence is unrecorded or `false`
  for passing primary benchmark evidence; `true` makes the report failed.
- `generated_artifact_edits` and `manual_undocumented_security_steps` follow
  the same `null`/`false`/`true` report gate: missing keeps the report
  incomplete, and `true` fails the primary benchmark.
- `recommended_participant_count` is fixed by the benchmark contract at
  `minimum: 2` and `target: 3`; recorded evidence may add participant runs but
  must not lower the minimum.
- `participant_runs[].participant_profile` is fixed to `non_developer` for the
  primary shop benchmark.
- `participant_runs[].started_at` and `participant_runs[].completed_at` are
  either `null` while unrecorded or strict UTC timestamps shaped like
  `2026-05-18T09:00:00Z`; completed timestamps must not be earlier than started
  timestamps.
- `participant_runs[].raw_notes_artifact` is either `null` or a non-empty
  forward-slash relative path under the build directory; absolute paths, Windows
  drive paths, backslash paths, and `..` traversal are invalid.
- `orv benchmark-report .` reports `data.participant_raw_notes_artifacts[]`
  with per-run `path`, `path_safe`, `checked`, `retained`, `non_empty`, and
  `size_bytes` status for the raw-notes files reviewers must inspect.
- `orv benchmark-report . --require-pass` stays incomplete until task timing,
  smoke markers, participant-run minimum, retained non-empty participant
  raw-notes artifacts, and required observation data are recorded. Failed
  participant runs make the report failed.
- Passing reports require `recording_status: "recorded"`. Sample files or
  generated templates that leave `recording_status` as `sample` or
  `not_recorded` remain incomplete.

## Version Policy

These artifacts are v1 public deploy contracts. Breaking key/type changes require
a schema version bump or a documented compatibility bridge. Backward-compatible
additions must update deploy schema regression tests, `verify-build` drift gates,
and this file in the same change.
