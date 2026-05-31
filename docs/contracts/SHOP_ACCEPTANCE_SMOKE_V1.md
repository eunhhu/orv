# Shop Acceptance Smoke v1 Contract

Producers:

- `scripts/shop_acceptance_smoke.sh`
- `orv build . --prod --out dist` for a shop-template project
- generated `dist/deploy/smoke-test.sh`
- generated `dist/deploy/smoke-output.txt`
- generated `dist/deploy/participant-notes-template.md`
- `orv benchmark-prepare dist --participants 2`
- `orv benchmark-report dist`

Current regression coverage:

- `docs/samples/shop-smoke-output-v1.golden.txt`
- `docs/samples/shop-smoke-output-summary-v1.golden.json`
- `docs/samples/shop-acceptance-runner-v1.golden.json`
- `docs/samples/shop-benchmark-report-passed-v1.golden.json`
- `crates/orv-cli/src/tests.rs::benchmark_smoke_output_matches_published_shop_smoke_output_fixture`
- `crates/orv-cli/src/tests.rs::benchmark_report_marks_recorded_evidence_passed`
- `crates/orv-cli/tests/shop_acceptance_contract.rs::shop_acceptance_artifacts_expose_human_pass_gate_and_failure_classification`
- `crates/orv-cli/tests/deploy_schema_contract.rs::prod_build_deploy_and_benchmark_json_contracts_freeze_public_shape`
- generated smoke/reveal/DAP/route-origin regressions in `crates/orv-cli/src/tests.rs`

This contract freezes the automated template-to-running-shop handoff. It covers
fresh shop project creation, check/build/verify/env-check command order,
generated smoke script handoff, smoke output markers, benchmark-report handoff,
and the generated participant notes template. It does not claim that human
5-hour benchmark evidence has been recorded; human evidence remains the
benchmark acceptance layer.

The runner/generated handoff inventory golden is
`docs/samples/shop-acceptance-runner-v1.golden.json`. It freezes runner env
knobs, command order, server lifecycle cleanup, stdout handoff labels,
generated preflight commands, generated smoke script markers, and the generated
benchmark evidence mirror against preflight, including the participant notes
template marker inventory and prepare-result handoff.

## Acceptance Runner

`scripts/shop_acceptance_smoke.sh` must run the following sequence against a
fresh `orv init --template shop` project:

```text
orv init "$SHOP_DIR" --template shop
orv check .
orv build . --prod --out dist
orv verify-build dist
orv deploy-env-check dist
orv run-build dist
sh dist/deploy/smoke-test.sh
orv benchmark-prepare dist --participants 2 > dist/deploy/benchmark-prepare.json
orv benchmark-report dist > dist/deploy/benchmark-report.json
grep -F '"status": "incomplete"' dist/deploy/benchmark-report.json
```

The runner must:

- allow `ORV_BIN` override
- create or reuse `ORV_SHOP_ACCEPTANCE_DIR`
- wait for `${ORV_BASE_URL:-http://127.0.0.1:8080}/`
- kill the foreground `orv run-build dist` process on exit
- fail if the generated benchmark report is not `incomplete` before human
  evidence is recorded
- print `shop acceptance smoke passed`
- print the generated `smoke_output`, `benchmark_prepare`, and
  `benchmark_report` paths

## Generated Preflight Handoff

For a fresh shop production build, `dist/deploy/preflight.json` must advertise:

```json
{
  "commands": {
    "run_build": "orv run-build .",
    "smoke_test": "./deploy/smoke-test.sh",
    "benchmark_prepare": "orv benchmark-prepare . --participants 2",
    "benchmark_report": "orv benchmark-report .",
    "benchmark_report_require_pass": "orv benchmark-report . --require-pass"
  },
  "smoke_output_contract": {
    "output": "deploy/smoke-output.txt",
    "required_markers": []
  },
  "artifacts": {
    "participant_notes_template": "deploy/participant-notes-template.md"
  }
}
```

`smoke_output_contract.required_markers` is exactly:

```json
[
  "pass_marker",
  "build_dir",
  "base_url",
  "graph_contract",
  "dap_summary",
  "dap_source_bundle",
  "server_routes",
  "trace_stream_requested"
]
```

## Generated Smoke Script

The published sample output is
`docs/samples/shop-smoke-output-v1.golden.txt`, and the benchmark parser summary
golden is `docs/samples/shop-smoke-output-summary-v1.golden.json`.

`dist/deploy/smoke-test.sh` must:

- write `deploy/smoke-output.txt` on success
- include `orv deploy smoke test passed`
- verify the build graph contract before live route checks
- run the DAP production-summary gate
- verify the DAP source-bundle panel gate
- record `server_routes=<count>`
- record `trace_stream_requested=<0-or-1>`
- keep route/reveal/client/native checks aligned with the generated build
  artifacts

## Benchmark Handoff

`dist/deploy/benchmark-evidence.json` must mirror the same
`smoke_output_contract` and `smoke_test_required_markers` as preflight. The
generated benchmark evidence starts incomplete until human task timing,
smoke-output content, participant metadata, failure classification, and notes
are recorded. If copied smoke-output evidence and the generated
`dist/deploy/smoke-output.txt` artifact are both present, benchmark reports
require their trimmed contents to match. Recorded participant runs must keep
`raw_notes_artifact` as a
non-empty forward-slash relative path under `dist` with no absolute path, Windows
drive path, backslash path, or `..` traversal, and the referenced file must exist
for `orv benchmark-report dist --require-pass` to pass. The referenced file must
also be filled beyond the generated template and its `participant_id`/`run_id`
lines must match the corresponding evidence row; unresolved timestamp/failure
classification placeholders, generated template instruction prose, or identity
mismatches keep the report incomplete.
`dist/deploy/participant-notes-template.md` is the generated capture template.
`orv benchmark-prepare dist --participants 2` copies it once per participant
under `dist/deploy/evidence/`, seeds participant rows, and sets each
`data.participant_runs[].raw_notes_artifact` value to that relative path. Its
JSON output includes `recording_handoff` with the evidence path, task fields,
participant fields, observation fields, benchmark `fields_to_record`, success
criteria, and the `orv benchmark-report . --require-pass` command. The
require-pass command is intentionally a hard gate only after those fields and
retained raw artifacts are filled.

The recorded-evidence pass inventory golden is
`docs/samples/shop-benchmark-report-passed-v1.golden.json`. It freezes the
normalized pass report status, task counts, smoke-output markers, participant
minimum, participant run summaries, and retained/non-empty/template-filled/
identity-matched raw-notes artifact checks for a synthetic recorded evidence
bundle. This fixture proves the accepted bundle shape; it is not real human
benchmark evidence.

## Version Policy

Breaking changes to runner command order, generated preflight command names,
smoke output marker names, generated smoke output path, participant notes
template path/markers, or benchmark handoff fields require a contract update,
changelog entry, matrix update, and regression update.
