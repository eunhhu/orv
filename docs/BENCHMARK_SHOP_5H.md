# 5-Hour Shop Benchmark

This benchmark is the product test for orv's north star: a non-developer can build and deploy a small shop without AI assistance in under 5 hours.

## Participant

- HTML/CSS/JS experience: 0 to 1 year.
- No professional backend, DB, deployment, or payment integration experience.
- Can read official orv docs and use built-in editor/help.
- Cannot use Copilot, Cursor, ChatGPT, or other AI assistance during the run.

## Starting Point

```bash
orv init my-shop --template shop
cd my-shop
orv dev
```

The primary benchmark uses local reference adapters:

- SQLite-backed shop DB via `SHOP_DATABASE_URL` default.
- Mock/local payment capture.
- Mock/local shipping booking.
- Local deploy/preflight artifacts and generated smoke-test.

Provider-backed Stripe/carrier runs are separate advanced variants.

## Acceptance Before Human Runs

Before recruiting participants, the generated shop template must pass an automated template-to-running-shop smoke path:

```bash
orv init my-shop --template shop
cd my-shop
orv check .
orv build . --prod --out dist
orv verify-build dist
orv deploy-env-check dist
orv run-build dist
sh dist/deploy/smoke-test.sh
orv benchmark-report dist
```

`orv run-build dist` keeps the reference server in the foreground. Keep that command running and execute the generated smoke test from a second terminal, or use the generated Docker Compose runbook for a detached server.

This gate proves the implementation path first. Human 5-hour runs then measure authoring UX, not whether the scaffold can boot.

Production builds mirror this benchmark contract into `deploy/preflight.json` under `benchmark`, and the checked preflight command list includes both `orv benchmark-report .` and `orv benchmark-report . --require-pass`. They also emit `deploy/benchmark-evidence.json`, a checked evidence template keyed to the same preflight hash, and generated smoke tests write `deploy/smoke-output.txt` on success with the checked graph/client/route/trace summary. The evidence artifact carries the automated gate, success criteria, time budget, and data-to-record fields so benchmark reports stay tied to the same deploy preflight that `orv verify-build` checks.

After a human run, fill the recorded fields in `deploy/benchmark-evidence.json` and run `orv benchmark-report dist --require-pass` to turn elapsed task time, required observation data, generated smoke output, participant-run metadata, failure classification, and the 5-hour limit into a checked JSON report. The report parses `deploy/smoke-output.txt`, requires the generated pass, graph-contract, route-count, and trace-request markers instead of trusting a manually typed "passed" string, keeps the report incomplete until the minimum participant-run evidence is recorded, requires each recorded `raw_notes_artifact` to point to a retained non-empty relative file under the build directory, emits `participant_raw_notes_artifacts[]` with path safety/check/retained/non-empty status for reviewer handoff, and requires `failure_classification.primary` whenever a task or participant run failed.

The report also requires `ai_assistance_used: false`,
`generated_artifact_edits: false`, and
`manual_undocumented_security_steps: false`. Missing values keep the report
incomplete, and any `true` value fails the benchmark.

Use only the checked benchmark status values in task and participant rows:
`not_recorded`, `missing`, `todo`, `incomplete`, `recorded`, `passed`, `pass`,
`failed`, `fail`, or `blocked`. Unknown statuses are not counted as recorded
evidence. Recorded/non-missing task rows must also include non-empty task
notes; blank notes keep the report incomplete.

The participant count target is part of the benchmark contract:
`recommended_participant_count.minimum` stays `2` and `target` stays `3`.
Recorded evidence can add participant runs, but must not lower the minimum.
Every participant run for the primary benchmark must keep
`participant_profile: "non_developer"`; developer runs can be retained as
separate notes, but they do not count as passing 5-hour shop evidence.

Use [samples/shop-benchmark-evidence.sample.json](samples/shop-benchmark-evidence.sample.json) as a field-level example only. Real evidence must be recorded in the generated `deploy/benchmark-evidence.json`, preserve its generated preflight hash, keep recorded `run_id` and `participant_id` values unique, keep `started_at`/`completed_at` as strict UTC timestamps like `2026-05-18T09:00:00Z` with completion not earlier than start, keep `raw_notes_artifact` paths non-empty and forward-slash relative to `dist` without absolute paths, Windows drive paths, backslash paths, or `..` traversal, and set `recording_status` to `"recorded"` only after replacing sample participant data with retained raw notes/output.

## Success Criteria

The participant must finish all items:

- edit the home page copy and theme tokens
- create 3 products
- add one product field and show it in catalog/admin
- sign up and log in as a member
- add an item to cart
- complete checkout
- capture mock payment
- book mock shipping
- view order/payment/shipment rows in admin
- run prod build
- pass deploy env check
- pass generated smoke-test
- reveal route/html/db-related execution output back to source through origin artifacts

## Failure Criteria

The run fails if:

- total elapsed time exceeds 5 hours
- AI assistance is used
- checkout cannot create an order, payment record, and shipment record
- smoke-test fails
- the participant must edit generated runtime/build artifacts by hand
- a required security step is manual and undocumented

## Time Budget

| Task | Target |
|------|--------|
| Project creation and first run | 15 min |
| First page/theme edit | 30 min |
| Product data entry | 30 min |
| Product field addition | 45 min |
| Form validation update | 45 min |
| Auth/member flow check | 30 min |
| Checkout/payment/shipping config | 60 min |
| Admin verification | 30 min |
| Prod build and env check | 30 min |
| Smoke-test and issue fixing | 45 min |

## Data To Record

- non-negative elapsed time and non-empty notes per recorded task
- non-negative number of docs/help lookups
- non-negative number of compiler/runtime errors
- non-negative time from first error to fix
- whether any AI assistance was used
- whether generated runtime/build artifacts were edited by hand
- whether any required security step was manual and undocumented
- all manual config edits as non-empty string descriptions
- smoke-test output
- unique participant run metadata with ordered UTC timestamps and retained raw notes artifacts for the 2-3 person first benchmark set
- failure classification from the fixed category list: syntax, scaffold, compiler/runtime error, editor, documentation, deploy config, smoke contract, or other
- participant notes on confusing concepts

## Design Feedback Loop

Any step that repeatedly exceeds its time budget should produce one of:

- simpler App Authoring syntax
- better scaffold defaults
- better error message
- editor affordance
- documentation change
- removal from MVP scope
