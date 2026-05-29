# orv Changelog

Implementation deltas live here, not in [SPEC.md](SPEC.md). Keep entries factual and dated.

## 2026-05-18

- Added a published DB Adapters v1 golden fixture for normalized
  MySQL/PostgreSQL external bridge artifacts, deploy/env/smoke handoff, and
  reveal matched-adapter shape without auth token values.
- Added a published Commerce Provider Hardening v1 golden fixture for
  normalized provider adapter artifacts, deploy/env gates, and compose/runbook
  marker inventory without secret values.
- Added a published Commerce Adapters v1 golden fixture for normalized HTTP
  payment/shipping adapter artifacts, deploy handoff, source-origin linkage, and
  reveal matched-adapter shape.
- Added a published Shop Benchmark Report passed-evidence golden fixture for
  normalized task counts, smoke markers, participant summaries, and retained
  raw-notes artifact checks.
- Added a published Reveal Coverage v1 golden fixture for normalized
  route/html/db/commerce/trace, function/domain, and static graph-view
  origin-spine inventories.
- Added a published Shop Acceptance Smoke v1 runner inventory golden for runner
  env knobs, command order, lifecycle cleanup, stdout handoff labels, generated
  smoke markers, and generated benchmark evidence/preflight mirroring.
- Added a published Editor Trace v1 inventory golden fixture covering normalized
  trace, trace-stream, native-host trace, and action-result payloads.
- Added a published LSP Bootstrap v1 editor-action inventory golden fixture for
  diagnostics, imported links, rename/highlight/reference flows, workspace
  search/diagnostics, and reveal code-lens commands.
- Added a published Check CLI v1 golden fixture for success stdout and
  imported-file diagnostic source routing.
- Added a published Compiler Pipeline v1 golden fixture for check/run behavior,
  resolver/analyzer failure classes, and OriginMap call edges.
- Added a published Runtime CLI v1 golden fixture for foreground `orv run`
  success output and runtime failure markers.
- Added a published HTML Render v1 golden fixture for compact static HTML,
  static bundle planning, and `run-build` stdout.
- Added a published Client Bundle v1 golden fixture for the manifest/reactive
  plan artifact graph, capability inventory, loader markers, and WASM marker.
- Added a published DB Persistence v1 golden fixture for local file-WAL/SQLite
  runtime/deploy/container/preflight handoff markers.
- Added a published Shop Template v1 golden fixture for generated manifest,
  source scaffold, README handoff, and `orv check .` gate markers.
- Added a Core Spine v1 integration contract and golden fixture that freezes the
  ProjectGraph/OriginMap/runtime trace/editor trace route-origin chain.
- Added a published Test Runner v1 discovery golden fixture, normalizing the
  temporary fixture root while freezing deterministic test order, path, span,
  and range metadata.
- Added a published LSP Bootstrap v1 common method inventory golden fixture for
  stdio `documentSymbol`, `completion`, `hover`, `formatting`,
  `semanticTokens/full`, and `foldingRange` responses.
- Added a published DAP Debug Session v1 stdio source-bundle launch golden
  fixture, covering source rehydration after the original file is removed.
- Added a published DAP Debug Session v1 stdio launch/step golden fixture for
  the initialize, launch, source inventory, source content, `next`, stack/scope,
  locals, and watch-evaluate frame inventory.
- Added a published DAP Debug Session v1 runner-result inventory golden fixture,
  normalizing local build/source-bundle paths while freezing DAP frame/control
  order, watch/local values, source inventory counts, panel sections, and
  production summary counts.
- Added a published DAP Debug Session v1 stdio initialize golden fixture for the
  initialize response plus initialized event frame.
- Added a published LSP Bootstrap v1 snapshot golden fixture, normalizing only
  the local entry path while freezing document symbols and ProjectGraph payload.
- Added a published LSP Bootstrap v1 initialize capabilities golden fixture, so
  advertised editor capability drift is checked by exact payload comparison.
- Added a published Editor State inventory v1 golden fixture, normalizing local
  entry, source-bundle, and build-directory paths while freezing snapshot/runtime
  counts, DAP adapter capabilities, control/configuration inventory, debug
  runner metadata, and production summary counts.
- Added a published Editor Native Host inventory v1 golden fixture, normalizing
  only the local production build directory while freezing export artifact
  names, capability flags, host handoff fields, panel sections, and production
  summary counts.
- Added a published Editor Export command-output v1 golden fixture, normalizing
  only local `entry` and `out` paths while freezing generated artifact names and
  order.
- Added a published Editor Snapshot v1 golden fixture, normalizing only local
  entry paths and the path-derived project graph hash while freezing the panel,
  ProjectGraph, and origin-map payload shape.
- Added a published Reveal Payload v1 production summary golden fixture, normalizing only the build directory path while freezing route/graph/preflight/adapter/native count rollups.
- Added a published Request Bindings v1 golden fixture for `@query: T`, `@body: T`, and `@form: T` success normalization.
- Added a published Request State v1 golden fixture for path params, decoded UTF-8 query values, headers, JSON body values, and raw-body preservation.
- Added a published HTTP Server v1 golden fixture for the minimal JSON route response and default 404 envelope.
- Added a Route Origin Headers v1 server-route golden fixture that freezes the route origin id, response origin id, and generated response metadata used by production smoke header checks.
- Added published Shop Acceptance Smoke v1 smoke-output and benchmark-parser summary fixtures, with regression coverage for the generated marker handoff shape.
- Added Build Artifacts v1 golden fixtures for `build-manifest.json`, `bundle-plan.json`, and `source-bundle.json`, with contract coverage that normalizes only fixture-local absolute source paths.
- Added published Deploy Preflight v1 and generated Benchmark Evidence v1 golden fixtures, and compared production builds against them in the deploy schema contract regression.
- Added published ProjectGraph v1 and OriginMap v2 golden fixtures for the `fixtures/e2e/hello.orv` source spine, with regressions that compare the CLI producers against the fixture while normalizing only the local workspace path.
- Added a published Runtime Trace v1 golden fixture and regression so the stable trace root/frame payload emitted by the runtime producer is checked against the documented sample.
- Added a published Validation Error Response v1 golden fixture and regression, normalizing only diagnostic prose while freezing the stable payload schema, ordering, null actuals, and unknown-property actuals.
- Extended Validation Error Response v1 contract coverage to lock `@query: T` and `@form: T` producers to the same 400 payload envelope as `@body: T`.
- Fixed editor trace-stream latest-state merging so frame events observed after a snapshot update `latest`, while replayed frame events must match the snapshotted frame payload.
- Tightened generated deploy trace-stream smoke gates to verify Runtime Trace v1 frame-event wrappers and editor trace-frame event counts before benchmark evidence can record `trace_stream_requested=1`.
- Clarified Runtime Trace v1 producer base-frame keys versus editor navigation extension origin ids, and added a drift gate for invalid trace origin-id primitive types.
- Added the HTML Render v1 public contract doc and a CLI black-box regression that freezes zero-runtime `@html` static build output, bundle-plan target shape, and `orv run-build` stdout behavior.
- Added the Build Artifacts v1 public contract doc and a CLI black-box regression that freezes common `orv build` artifact roots plus `orv verify-build` acceptance.
- Extended the Native Server Plan v1 contract regression to freeze build-manifest and bundle-plan native artifact linkage, and promoted the native server plan/source matrix row to stable-ish while keeping the final native optimizer planned.
- Added the Shop Template v1 public contract doc and a CLI black-box regression that freezes generated `orv init --template shop` manifest, source scaffold, README handoff, and `orv check .` parseability; promoted the shop template matrix row to stable-ish.
- Added the Shop Acceptance Smoke v1 public contract doc and extended the shop acceptance regression to freeze runner command order, generated preflight smoke/benchmark commands, smoke-output required markers, generated smoke script markers, and benchmark evidence marker mirroring; promoted the template-to-running-shop smoke path matrix row to stable-ish.
- Added the Shop Security Boundaries v1 public contract doc and extended its CLI black-box regression to freeze generated shop session/auth/CSRF/rate-limit/checkout/webhook source ordering plus production artifact and smoke-script exposure.
- Added the Shop Checkout Resilience v1 public contract doc and a runtime fixture regression that freezes payment-captured shipment-failure compensation: HTTP 202, pending order status, captured payment persistence, no shipment row, `checkout.compensation_required` audit, and stable carrier idempotency keys.
- Added the Commerce Adapters v1 public contract doc and a CLI black-box regression that freezes HTTP payment/shipping adapter deploy artifacts, env/default handoff, source-origin linkage, verify-build acceptance, and reveal matched-adapter metadata.
- Added the Commerce Provider Hardening v1 public contract doc and CLI black-box regressions that freeze provider-mode Stripe/carrier env gates, artifact handoff, secret redaction expectations, retry behavior, and stable provider idempotency keys.
- Added the DB Adapters v1 public contract doc and a CLI black-box regression that freezes external PostgreSQL/MySQL bridge deploy artifacts, env/preflight/smoke handoff, source-origin linkage, verify-build acceptance, and reveal matched-adapter metadata.
- Added the DB Persistence v1 public contract doc and a CLI black-box regression that freezes local file-WAL and SQLite deploy persistence handoff, env/default propagation, Compose volume mapping, runtime feature exposure, `verify-build`, and `deploy-env-check` acceptance.
- Added a Client Bundle v1 CLI black-box regression that freezes client manifest/reactive-plan/page/JS/WASM artifact linkage, stable 16-hex hash fields, capabilities, blockers, bindings, and loader/WASM markers; aligned the contract doc with the emitted hash format and promoted the client reactive bundle matrix row to stable-ish.
- Extended the Deploy Artifacts v1 contract regression to run `orv deploy-env-check` against the generated production artifact set and promoted the deploy artifacts matrix row to stable-ish.
- Extended the DAP Debug Session v1 public contract with the `orv dap serve --stdio` initialize capability envelope and initialized event, backed by a CLI black-box regression, and promoted the DAP bootstrap matrix row to stable-ish.
- Added the Compiler Pipeline v1 public contract doc and a CLI black-box regression for resolver hoisting/shadowing, resolver diagnostics, HIR analysis diagnostics, runtime binding behavior, and HIR-derived OriginMap call edges; promoted name resolution and HIR lowering matrix rows to stable-ish.
- Added native-host trace reveal action inventories so route, response, DB operation, and commerce adapter trace frames expose one-loop `orv editor reveal` actions with source/production targets.
- Wired trace reveal actions into the exported editor shell so selected trace frames render route/response/db/commerce reveal actions and dispatch `orv:trace-reveal-action` for native host execution.
- Added `orv editor run-action` as the native-host trace reveal action runner, writing `trace/action-result.json` and `trace/action-result.html` after executing the allowlisted `orv editor reveal` action.
- Exported a `native-host/bridge.js` helper that maps trace reveal UI actions to native WebView postMessage payloads with `orv editor run-action` argv and trace action result refresh metadata.
- Added `orv editor host <export-dir>` as a local native-host bridge server: it serves exported editor artifacts, accepts `POST /__orv/native-host/action`, runs the allowlisted trace reveal action, writes result JSON/HTML, and returns refresh metadata for the shell.
- Added exported native-host desktop package artifacts (`native-host/desktop-package.json` and `native-host/run-desktop-host.sh`) so desktop containers get a checked local bridge lifecycle, allowed process-spawn policy, refresh event map, and source permission roots.
- Added `orv editor desktop-shell <export-dir|desktop-package.json>` to consume the exported desktop package, verify artifact readiness, normalize host spawn/WebView/process supervision/source permission plans, and optionally write `native-host/desktop-session.json`.
- Added `orv editor desktop-run <export-dir|desktop-package.json|desktop-session.json>` to execute the desktop session plan by spawning the host process, reading its ready JSON, deriving the WebView URL, and supporting a terminating `--probe` mode for automation.
- Exported a SwiftPM macOS desktop container scaffold under `native-host/desktop-app` that uses AppKit/WKWebView, spawns the host process from `desktop-session.json`, reads the host ready JSON, prompts before source reveal access, and terminates the host process with the app.
- Added macOS desktop bundle packaging artifacts (`native-host/desktop-packaging.json`, `native-host/package-desktop-app.sh`, `Info.plist`, and entitlements) so exported editor desktop containers can build a local `.app`, ad-hoc sign it by default, and opt into Developer ID hardened-runtime signing via `ORV_EDITOR_CODESIGN_IDENTITY`.
- Added optional macOS editor desktop notarization packaging: `ORV_EDITOR_NOTARIZE=1` zips the app, submits it with `xcrun notarytool` using either `ORV_EDITOR_NOTARY_PROFILE` or Apple ID/password/team env vars, staples the accepted ticket, and reports notarization status in the package result JSON.
- Added richer native editor source-permission UX: desktop sessions now carry source/root counts, prompt labels, read-only denied mode, WebView permission-state injection, and bridge-side blocking for source reveal actions when access is denied.
- Added a desktop platform matrix to native-host desktop package/session contracts, marking macOS SwiftPM/AppKit/WKWebView packaging as implemented and Windows WebView2 plus Linux WebKitGTK/Tauri containers as planned with explicit blockers and shared contracts.
- Added the Native Host Desktop v1 public contract doc and a regression that freezes desktop package, platform matrix, source-permission, and desktop-shell public JSON key/type surfaces.
- Added the Client Bundle v1 public contract doc and a regression that freezes `client/manifest.json`, `client/reactive-plan.json`, capability, blocker, signal, and core binding key/type surfaces.
- Added the Native Server Plan v1 public contract doc and a regression that freezes `server/native-server.json`, `server/runtime-image.json`, generated native source markers, package metadata, and Dockerfile key surfaces.
- Added the DAP Debug Session v1 public contract doc and a CLI black-box regression that freezes `orv editor run-debug <build-dir>` result, runner, production context, raw debug session, panel, source-bundle, and result artifact key surfaces.
- Added the Reveal Payload v1 public contract doc and a CLI black-box regression that freezes `orv reveal`, `orv editor reveal`, and `orv lsp reveal` root/source/focus/location/production key surfaces.
- Added the LSP Bootstrap v1 public contract doc and a CLI black-box regression that freezes `orv lsp snapshot` root/document-symbol keys plus `orv lsp serve --stdio` initialize capability keys.
- Added the Test Runner v1 public contract doc and a CLI black-box regression that freezes `orv test --list` discovery JSON, filter behavior, success summary output, and failure envelope.
- Added the Runtime CLI v1 public contract doc and a CLI black-box regression that freezes foreground `orv run` stdout/stderr/exit behavior and runtime failure envelope.
- Added the Editor Snapshot/Export v1 public contract doc and a CLI black-box regression that freezes `orv editor snapshot`, export stdout, `state.json`, `native-host.json`, and runtime/production panel artifact envelopes.
- Added the Editor Trace v1 public contract doc and a CLI black-box regression that freezes editor trace, trace-stream, native-host trace panel, trace reveal action, and action-result artifact envelopes.
- Added a CLI black-box reveal coverage regression that builds one production fixture and verifies route, route-local HTML, DB adapter, commerce adapter, and response-trace origins across `orv reveal`, `orv editor reveal`, `orv lsp reveal`, and `orv editor trace`.
- Extended reveal route matching through function `calls` edges, and added CLI regression coverage for function origins plus domain origins inside called functions returning to the containing production route.
- Extended editor trace frames with optional `db_operation_origin_id` and `commerce_adapter_origin_id` reveal navigation, with black-box coverage for DB operation source and commerce adapter source reveal.
- Added `ADVANCED_DOMAINS.md` as the M4+ promotion gate and marked advanced-domain pressure fixtures with explicit non-MVP contract badges.
- Added a CLI graph-view regression that verifies `orv graph --view` writes the same semantic origin-map, origin-edge, and origin-link spine into `graph.json` and renders it in the static graph HTML.
- Added CLI integration regressions that assert Stripe/carrier provider secret values and DB bridge auth token values do not leak into generated deploy artifacts or `orv deploy-env-check` output.
- Wrapped generated shop checkout stock reservation and order creation in the captured DB handle transaction boundary, with source-order and runtime shop flow regressions.
- Added an audit-visible generated shop checkout pending path for shipment failures after payment capture: orders move to `payment_captured_pending_shipment`, emit `checkout.compensation_required`, and return 202 with payment/order context.
- Hardened Stripe-style webhook verification so `t=...,v1=...` signatures require timestamp freshness within `STRIPE_WEBHOOK_TOLERANCE_SECONDS` or the 300-second default.
- Hardened `orv benchmark-report --require-pass` so shop benchmark evidence stays incomplete without the minimum participant-run metadata and fails on failed participant runs.
- Hardened `orv benchmark-report --require-pass` so recorded shop participant runs must retain their referenced raw notes artifact under the build directory.
- Aligned reveal/editor/DAP production benchmark summaries with the same recording-status, failure-classification, and retained raw-notes gates used by `orv benchmark-report --require-pass`.
- Made deploy benchmark evidence reject unsafe `raw_notes_artifact` paths before `verify-build` accepts recorded human-run metadata.
- Locked benchmark raw-notes artifact paths to forward-slash relative paths so Windows drive/backslash forms cannot bypass the retained-file evidence gate.
- Added per-run `participant_raw_notes_artifacts` status to benchmark reports so retained human-run notes are visible in report JSON, not only through missing-data failures.
- Mirrored `participant_raw_notes_artifacts` into reveal/editor/DAP production benchmark summaries so native/editor surfaces show the same retained-notes evidence status.
- Made recorded participant raw-notes artifacts require non-empty files and report `non_empty`/`size_bytes` evidence status.
- Restricted benchmark task and participant statuses to the checked status taxonomy so unknown strings cannot count as recorded evidence.
- Fixed the shop benchmark participant-count contract at minimum 2 and target 3 so recorded evidence cannot lower the human-run gate.
- Fixed the shop benchmark participant-profile contract at `non_developer` so developer-run metadata cannot count as primary 5-hour shop evidence.
- Added an `ai_assistance_used` benchmark evidence gate so primary 5-hour shop reports are incomplete until AI-use status is recorded and fail when AI assistance was used.
- Added generated-artifact-edit and manual-undocumented-security-step benchmark evidence gates so primary 5-hour shop reports fail when either failure criterion is recorded.
- Made benchmark observation counts non-negative so docs/help lookup and compiler/runtime error evidence cannot pass with negative or non-integer values.
- Made benchmark elapsed-time evidence non-negative so task timing and first-error-to-fix values cannot pass with negative durations.
- Made benchmark smoke-output `base_url` evidence require an HTTP(S) URL instead of any non-empty marker value.
- Made benchmark smoke-output trace evidence require `trace_stream_requested=1` so normal smoke output cannot satisfy the trace-stream gate.
- Made benchmark smoke-output `build_dir` evidence require an absolute path instead of any non-empty marker value.
- Made duplicate benchmark smoke-output marker fields keep the duplicated marker incomplete instead of trusting the last value.
- Made benchmark smoke-output `server_routes` evidence match the generated deploy/preflight route count before passing.
- Made benchmark smoke-output `build_dir` evidence match the report target build directory before passing.
- Made benchmark reports reject copied `smoke_test_output` evidence when it differs from the retained generated smoke-output artifact.
- Made manual benchmark config-edit evidence require non-empty string entries so blank or typed placeholders cannot pass reports.
- Added participant timestamp format/order gates so recorded human-run evidence requires UTC `started_at`/`completed_at` values with completion not earlier than start.
- Added participant `run_id`/`participant_id` uniqueness gates so duplicated rows cannot satisfy the benchmark participant minimum.
- Made benchmark reports reject failure-classification category drift instead of trusting evidence-provided category lists.
- Made benchmark reports require `failure_classification.primary` whenever recorded task evidence or participant-run evidence fails.
- Made `failure_classification.primary: "other"` require explanatory notes so out-of-taxonomy failures stay reviewable.
- Made benchmark reports require `recording_status: "recorded"` before passing, and added a sample-only shop benchmark evidence field example.
- Made recorded benchmark task rows require non-empty notes so elapsed-time/status stubs cannot count as complete human-run evidence.
- Added `scripts/shop_acceptance_smoke.sh` as a fresh shop CI-style acceptance runner and extended the shop benchmark contract/evidence template with the human `benchmark-report --require-pass` gate plus participant-run and failure-classification slots.
- Split the reference runtime HTTP server into facade, request/response/routing/state, rate-limit, attached runtime, serve-loop, trace, and test-support modules while preserving the public server trace and attached-server APIs.
- Added a prod build/deploy schema contract regression covering build manifest, source bundle, bundle plan, deploy manifest, deploy preflight, and benchmark evidence public JSON shapes.
- Added public contract docs for ProjectGraph v1 and OriginMap v2 JSON shapes, version policy, and regression coverage.
- Added a ProjectGraph v1 CLI black-box regression that freezes `orv graph` JSON shape and `orv graph --view` `graph.json` mirroring.
- Added an OriginMap v2 CLI black-box regression that freezes `orv origins` JSON shape, entry/edge references, and `orv graph` embedded origin-map equality.
- Added the Check CLI v1 public contract doc and a CLI black-box regression that freezes `orv check` success/failure envelope plus imported-file diagnostic source routing.
- Added a Deploy Artifacts v1 public contract doc for build manifest, source bundle, bundle plan, deploy manifest, preflight, smoke-output contract, and benchmark evidence JSON shapes.
- Added Runtime Trace v1 and Validation Error Response v1 public contract docs for trace file/EventSource payloads and 400 validation error JSON shapes.
- Added a Request State v1 public contract doc and runtime regression covering `@param`, decoded `@query`, `@header`, parsed JSON `@body`, and `@request.rawBody`.
- Added a Request Bindings v1 public contract doc for `@query: T`, `@body: T`, and `@form: T` success normalization plus validation-failure handoff to Validation Error Response v1.
- Added an HTTP Server v1 public contract doc and runtime regression covering minimal route dispatch, JSON response content type/payload, and default unmatched-route 404 behavior.
- Added a Route Origin Headers v1 public contract doc for `x-orv-origin-id`, branch-specific `x-orv-response-origin-id`, server artifact coupling, generated smoke checks, and trace coupling.
- Promoted the Route Origin Headers matrix row to stable-ish now that runtime, generated smoke, trace, and verify-build coverage are tied to the public v1 contract.
- Added generated smoke regression coverage that multi-response routes do not force an ambiguous `x-orv-response-origin-id` expectation.
- Added integration regressions that freeze runtime request trace JSON root/frame keys and generated deploy smoke checks for exact `x-orv-origin-id` and `x-orv-response-origin-id` contracts against server artifacts.
- Added EventSource trace-frame contract coverage for `orv.production.trace.frame` stream event keys and nested request-frame primitive values.
- Added an `orv-compiler` origin-map JSON contract regression that freezes public object keys and primitive field types for the root map, entries, spans, and edges.
- Added a runtime integration regression that freezes the public 400 `orv.validation.error` response root keys, field keys, and primitive values for declarative request validation.
- Extended the validation response contract to freeze multi-error ordering, missing-field null actuals, and unknown-property actual values.
- Added runtime regression coverage that `x-orv-response-origin-id` follows the executed `@respond` branch when a route has multiple response sites.
- Made `orv verify-build` reject generated smoke drift from per-route CLI/editor/LSP reveal production summary counters.
- Added exact generated deploy smoke gates for DAP native server target counts from the actual production summary instead of a one-target literal.
- Added exact generated deploy smoke gates for client bundle target, manifest, and capability-surface counts across CLI/editor/LSP reveal and DAP production summaries.
- Added exact generated deploy smoke gates for DAP project graph node and origin-map entry summary counts, tying the graph/source-bundle/origin-map production summary back to build artifacts.
- Fixed generated deploy smoke DAP source-bundle count checks to use the actual build source-bundle file count, so imported multi-file projects do not fail against a one-file expectation.
- Fixed generated deploy smoke DAP native route summary checks to use the actual server route count, so shop-scale builds with many routes do not fail against a one-route expectation.

## 2026-05-17

- Aligned `clippy.toml` with the workspace Rust MSRV and replaced the remaining newer API use in DAP hit conditions so `cargo clippy -- -D warnings` is warning-free on the declared toolchain floor.
- Added a checked `smoke_output_contract` to generated preflight and benchmark evidence artifacts so `deploy/preflight.json`, benchmark reports, reveal surfaces, and runbooks share the same required marker list.
- Added generated deploy smoke gates for the smoke-output required-marker contract across CLI/editor/LSP reveal payloads and DAP production context, and cached the DAP run-debug output so smoke does not rerun it for every grep.
- Added the generated smoke-output artifact and required marker list to the shop starter README so the starter guide matches generated deploy runbooks.
- Added the required smoke-output marker list to the generated deploy runbook and made verify-build reject runbook drift from the benchmark smoke marker contract.
- Mirrored benchmark smoke required-marker contracts into reveal/editor/native production preflight payloads.
- Split the large CLI and compiler implementation files into focused modules while keeping public command/artifact behavior unchanged.
- Made generated benchmark evidence record the required smoke-output marker list, including `dap_source_bundle`, and made `orv verify-build` reject evidence drift from that marker contract.
- Exposed the same required smoke-output marker list in benchmark report data and parsed smoke summaries so reveal/editor consumers can see the expected smoke contract alongside missing markers.

## 2026-05-16

- Added a `dap_source_bundle=verified` marker to generated smoke output and benchmark-report parsing so source-bundled DAP panel coverage is recorded as benchmark evidence.
- Added generated deploy smoke and verify-build gates for `panels.debug.source_bundle` path/file-count/hash metadata from source-bundled DAP runs.
- Mirrored raw DAP source-bundle launch metadata into editor run-debug result JSON, `panels.debug.source_bundle`, session summaries, and the rendered debug result panel.
- Exposed DAP source-bundle launch metadata in raw launch/restart responses and made `restart` preserve the previous build `source-bundle.json` path when no program override is supplied.
- Mirrored the DAP production-summary gate into generated deploy preflight, benchmark evidence, and runbook commands as `orv editor run-debug . --control next`, with verify-build drift checks.
- Advertised raw DAP `sourceBundle` launch support and added a direct DAP regression that launches from build `source-bundle.json` after the original source file is removed.
- Added a `dap_summary=verified` marker to generated `deploy/smoke-output.txt` and `orv benchmark-report` parsing, so benchmark evidence records whether the source-bundled DAP production-summary gate passed.
- Added generated deploy smoke DAP gates: smoke tests now run `orv editor run-debug . --control next` from the build dir and assert graph/source-bundle, native, and client production summary counters.
- Let `orv editor run-debug <build-dir>` synthesize a DAP runner from `source-bundle.json`, so build-backed debug sessions can run and render production summaries even after the original source file is unavailable.
- Added client and static positive gates for `panels.debug.production_summary`, so DAP runner result tests now cover native, client bundle, and zero-runtime static production counters.
- Split build-backed DAP runner production context into a checked `panels.debug.production_summary` section and rendered debug-result metrics, so native/static/client/smoke summary counters stay visible in `orv editor run-debug` outputs.
- Extended generated deploy smoke tests so client-bundle builds assert CLI/editor/LSP client-origin reveal payloads carry client target, manifest, and capability summary counters.
- Tightened static production verification so `deploy/manifest.json` static targets must match the bundle-plan `static_page` target, and LSP/editor reveal tests now assert static summary counters.
- Extended generated deploy smoke tests so route-origin CLI/editor/LSP reveal payloads must carry native-server target and route summary counters.
- Added native-server and static-page production target summaries to reveal/editor/native-host production payloads, including native route/blocker counts, static verification counts, and Production panel sections.
- Threaded build-backed production graph and summary context into editor debug metadata, standalone DAP runner artifacts, native-host debug metadata, run-debug results, and debug result panels.
- Added graph-contract and production summary counters to `orv reveal`, `orv editor reveal`, and `orv lsp reveal` production payloads, and made generated deploy smoke checks assert the smoke-evidence summary counter is present across all three reveal surfaces.
- Added state/native-host/editor Production panel counters for preflight smoke evidence summaries, including present, missing, and missing-marker gap counts.
- Added reference-runtime `x-orv-response-origin-id` headers and request trace `response_origin_id` fields for executed `@respond` nodes, and wired editor/native-host trace payloads to expose separate response reveal navigation alongside route navigation.
- Extended generated deploy smoke tests to verify exact `x-orv-response-origin-id` headers for covered routes with one unambiguous response origin, and made verify-build reject response-origin smoke drift.
- Linked `@html` projection origins back to static page/client bundle artifacts and route-local HTML origins back to their containing route/native-server production targets in reveal/editor payloads.
- Linked generated DB adapter contracts back to the source `@db.connect` origin through `source_origin_id`, and made reveal production payloads expose `matched_adapters` for the selected origin.
- Linked generated commerce adapter contracts back to source `@payment.connect` and `@shipping.connect` origins through `source_origin_id`, with matching reveal `matched_adapters` payloads.
- Strengthened `orv verify-build` so DB and commerce adapter `source_origin_id(s)` must resolve to the expected connect call entries in `origin-map.json`.
- Added a reference HTTP bridge for PostgreSQL/MySQL `@db.connect` handles: configured `ORV_DB_ADAPTER_POSTGRES_ENDPOINT`, `ORV_DB_ADAPTER_MYSQL_ENDPOINT`, or `ORV_DB_ADAPTER_ENDPOINT` values turn external DB handles from explicit unsupported status into checked `http-json-v1` POST adapter calls, with optional bearer tokens from provider-specific or generic DB adapter auth envs.
- Made `@design` token lookup work inside HTML render attributes and added editable color/spacing/typography tokens to the shop starter home shell.
- Added an end-to-end editable product field path to the shop starter: `ProductInput.badge` now flows through the product form, `POST /products`, customer catalog, admin catalog, and generated smoke-test body checks.
- Surfaced PostgreSQL/MySQL DB bridge request shape, bounded transient retry policy, and provider-specific endpoint/auth env knobs in `deploy/db-adapters.json`, generated Compose/env.example, preflight envs, and the deploy runbook; production deploy env checks now require the provider-specific bridge endpoint before launch while keeping bridge auth tokens optional.
- Aligned deploy preflight and smoke tests with the runtime DB bridge fallback envs, so generic `ORV_DB_ADAPTER_ENDPOINT` and `ORV_DB_ADAPTER_AUTH_TOKEN` can satisfy shared bridge deployments when provider-specific values are unset.
- Extended generated deploy smoke tests so external DB bridge builds check `deploy/db-adapters.json` and POST a safe `schema` probe to each configured provider bridge endpoint.
- Strengthened generated production shop smoke tests so checkout/admin validation captures response bodies and checks checkout status, payment capture, shipment tracking, customer catalog/cart/session read models, and admin catalog/order/payment/shipment/audit read models.
- Added generated production smoke checks for `x-orv-origin-id` route headers so deployed route reachability also proves the ProjectGraph/HIR origin contract is exposed at runtime.
- Made `orv run-build <dir>` execute relative DB/WAL, `@serve`, `@fs`, and file-backed commerce adapter paths against the build directory so local deploy smoke runs do not leak persistence files into the caller's shell cwd.
- Strengthened `orv verify-build` so server route/listen/response origin ids must resolve through `origin-map.json` and server/deploy source snapshots must match `source-bundle.json`.
- Added `project-graph.json` verification for source-bundle file nodes, semantic origin-map mirrors, semantic origin edges, and origin-link drift.
- Made generated deploy smoke tests compare each `x-orv-origin-id` header against the exact route origin id from the server artifact instead of accepting any `ori_` value.
- Made DAP `setInstructionBreakpoints` verify `orv:frame:N` pseudo-instruction references after launch and stop `continue` on matching runtime frames.
- Surfaced DAP `loadedSources`/`source` request inventory and launch-time source snapshot responses through editor export, native-host debug metadata, and run-debug result panels, including imported source SHA256 checksums.
- Mirrored build graph contracts (`source-bundle.json`, `project-graph.json`, and `origin-map.json`) into editor production export/native-host/panel payloads with artifact hashes and source/origin counts.
- Made generated deploy smoke tests gate on the same build graph spine by checking `source-bundle.json`, `project-graph.json`, `origin-map.json`, and running `orv verify-build .` before live route checks.
- Mirrored graph artifacts into `deploy/preflight.json` so preflight, smoke, runbook, and verify-build all name the same source-bundle/project-graph/origin-map contract paths.
- Added a trace-enabled `orv run-build . --trace deploy/request-trace.json` preflight command and clearer trace-smoke failure guidance, aligning generated smoke with the runbook trace capture flow.
- Mirrored the 5-hour shop benchmark contract into `deploy/preflight.json`, including automated gate commands, success criteria, time budget, and data-to-record fields, with verify-build drift checks.
- Added checked `deploy/benchmark-evidence.json` generation so benchmark timing and observation records carry the same 5-hour shop contract, preflight hash, command list, linked artifacts, task budget, and data-to-record schema that `orv verify-build` validates.
- Added `orv benchmark-report <dir> [--require-pass]` to summarize recorded benchmark evidence as pass/fail/incomplete JSON and optionally fail CI when the human-run evidence is incomplete or over the 5-hour budget.
- Mirrored `orv benchmark-report .` and `orv benchmark-report . --require-pass` into generated deploy preflight/runbook contracts so benchmark reporting is a checked deploy gate instead of a standalone command.
- Added benchmark evidence report-status and missing-evidence counters to reveal/editor/native production preflight payloads, reusing the same pass/fail/incomplete calculation as `orv benchmark-report`.
- Added generated `deploy/smoke-output.txt` capture on successful smoke runs and let `orv benchmark-report` use it when benchmark evidence has not copied smoke output yet.
- Strengthened generated production shop smoke tests to fetch the admin dashboard and webhook read-model page, checking dashboard links/storage paths plus webhook/audit summary fields.
- Exposed CSRF, session cookie, auth role, and default route rate-limit requirements as shared `runtime_features` across build, server, deploy, and native plan artifacts.
- Added explicit reference `@rateLimit key=... limit=... window=...` route policies plus `@rateLimit exempt`, with runtime enforcement, server artifact descriptors, and native route table fields.
- Added source-backed `@csrf exempt` so intentional CSRF bypasses can execute without a token while still appearing in route policy artifacts.
- Added generated `deploy/preflight.json` so verify-build, deploy-env-check, run-build, smoke-test, runtime features, security features, persistence, env requirements, and linked deploy artifacts share one checked preflight contract.
- Exposed the checked deploy preflight contract through reveal/editor/LSP production payloads and the native editor production panel.
- Added per-route security policy descriptors for source-backed auth/session/csrf domains and built-in rate-limit defaults, with verify-build origin containment checks and reveal production payload exposure.
- Surfaced preflight route-policy counts and kind summaries in editor export/native-host production payloads and the generated production panel.
- Surfaced preflight command counts and checked benchmark-report commands through reveal/editor/native production payload tests and production panel summaries.
- Mirrored route security policy descriptors into generated native route table source so native artifacts carry the same source-backed policy contract.

## 2026-05-06

- Added shop scaffold coverage for persisted catalog, cart, member sessions, checkout, admin read models, payment records, shipment records, and webhook records.
- Added reference Stripe-style webhook verification with primary/previous secret handling, HMAC-SHA256 signature checks, duplicate event handling, and payment/order reconciliation hooks.
- Added DB archive upload/restore contracts for local file, HTTP, and S3-compatible targets, including hash/byte verification and bounded transient retries.
- Added DB crash-matrix verification for WAL replay, torn EOF recovery, corruption rejection, checkpoint replay, savepoint rollback, PITR cutoff, and archive hash mismatch.
- Added build/deploy artifacts for native server plan/source contracts, runtime image plan, generated Compose/runbook/env.example, DB adapter manifest, commerce adapter manifest, and smoke-test script.
- Added client bundle artifacts for static page, reactive plan, JS loader, WASM bootstrap, manifest capability inventory, blocker metadata, and verify-build checks.
- Expanded LSP/DAP bootstrap with source checksums, paging for stack/local windows, guarded request-domain references, debug runner commands, native-host export metadata, and trace transport payloads.

## Policy

- Date-stamped implementation notes go here.
- State/contract/crate/test tables go in [IMPLEMENTATION_MATRIX.md](IMPLEMENTATION_MATRIX.md).
- Future work goes in [ROADMAP.md](ROADMAP.md).
- Stable language behavior goes in [SPEC.md](SPEC.md).
