# orv Implementation Matrix

이 문서는 구현 중인 orv의 **상태 + 계약 레벨 + 검증 기준 + 담당 crate**를 한 번에 보여준다. 단순히 "구현됨"인지보다 "제품 표면으로 안정 계약을 걸 수 있는지"를 구분하는 것이 목적이다.

언어 의미론은 [SPEC.md](SPEC.md), 현재 MVP 경계는 [MVP.md](MVP.md), 구현 구조는 [ARCHITECTURE.md](ARCHITECTURE.md), 운영 command/method 세부는 [OPERATIONAL_SURFACES.md](OPERATIONAL_SURFACES.md)를 따른다.

## Core Spine

현재 안정화의 중심축은 다음 네 가지다.

```text
ProjectGraph + HIR Origin + Reference Runtime + Trace/Reveal
```

이 축이 깨지면 editor, deploy, native optimizer, shop scaffold 모두 신뢰를 잃는다. 따라서 feature 추가보다 먼저 다음 연결을 안정 계약으로 올린다.

- `Span -> AST node -> HIR node -> runtime event -> origin id`
- `orv graph`, `orv origins`, `x-orv-origin-id`, `x-orv-response-origin-id`, trace JSON의 origin schema 정합성
- route, DB query, HTML node, function call, domain invocation의 동일 reveal 모델
- first-party editor 없이도 CLI/static graph view만으로 production output에서 source로 돌아가는 경로

## Promotion Priorities

[IMPLEMENTATION_GAP_REPORT.md](IMPLEMENTATION_GAP_REPORT.md)는 matrix의 파생 분석이다. 상태/계약의 authoritative source는 이 문서이고, gap report는 다음 작업 순서를 제안한다.

| Priority | 기준 | 승격 조건 |
|----------|------|-----------|
| P0 | core spine schema | ProjectGraph/origin-map/trace/build/deploy schema가 fixture와 drift gate로 고정됨 |
| P1 | shop acceptance | generated shop smoke와 benchmark report가 fresh project에서 재현되고 human evidence가 기록됨 |
| P2 | production boundary | DB/provider adapter와 checkout transaction/idempotency/security boundary가 명확해짐 |
| P3 | reveal product value | route/html/db/function/domain/trace reveal이 같은 origin schema로 검증됨 |
| P4 | advanced domains | shop benchmark나 security gap을 직접 줄일 때만 MVP로 승격 |

## Status Terms

| Status | 의미 |
|--------|------|
| implemented | 현재 코드 경로가 동작하고 검증 대상으로 볼 수 있음 |
| reference stub | 레퍼런스 런타임/scaffold에서 제한적으로 동작함 |
| artifact only | 실행 기능보다 산출물/계약/manifest가 먼저 고정됨 |
| planned | 설계 방향은 있으나 구현 경로가 아직 없음 |
| not started | 문서상 아이디어 수준 |

## Contract Terms

| Contract | 의미 |
|----------|------|
| stable | 외부 사용자와 문서가 의존해도 되는 계약. 변경 시 migration/release note 필요 |
| stable-ish | MVP 내부 기준으로 안정화 중. 이름/JSON shape 변경 가능성 낮음 |
| experimental | 구현은 있으나 edge case와 문서 계약이 아직 흔들릴 수 있음 |
| reference | production provider가 아니라 reference/runtime/scaffold 기준 계약 |
| unstable | 개발 중인 내부 surface. 사용자는 직접 의존하지 않는 것이 좋음 |
| non-binding | 로드맵/디자인 방향. 구현 의무 없음 |

## Milestone Terms

| Milestone | 목적 |
|-----------|------|
| M0 | compiler/runtime foundation: parse, resolve, analyze, HIR, graph, origin, reference runtime, basic CLI |
| M1 | web app foundation: `@server`, `@route`, `@html`, form/body parse, schema validation, SQLite reference adapter, static serve, smoke test |
| M2 | shop foundation: auth/session, cart, order, mock payment, mock shipping, admin page, deploy artifact |
| M3 | reveal/editor foundation: graph view, origin reveal, runtime trace, LSP/DAP/bootstrap, editor protocol |
| M4+ | native optimizer, custom DB engine, advanced editor, production providers, advanced deploy |

## Matrix

| Feature | Status | Contract | Milestone | Crate | Test / Fixture | CLI | Notes |
|---------|--------|----------|-----------|-------|----------------|-----|-------|
| Source load / import DFS | implemented | stable-ish | M0 | `orv-project` | `fixtures/e2e/hello.orv` | `orv check` | Merged program + source map |
| Lexer / parser / AST | implemented | stable-ish | M0 | `orv-syntax` | `fixtures/e2e/hello.orv` | `orv check` | Span-backed AST |
| Name resolution | implemented | experimental | M0 | `orv-resolve` | `fixtures/plan/models/*.orv` | `orv check` | Scope/binding map |
| Semantic analysis / HIR lowering | implemented | experimental | M0 | `orv-analyzer`, `orv-hir` | `fixtures/e2e/hello.orv` | `orv check`, `orv run` | Runtime/compiler consume HIR |
| Diagnostics | implemented | stable-ish | M0 | `orv-diagnostics` | compiler fixture suite | `orv check` | Span-backed structured diagnostics |
| AST ProjectGraph v1 | implemented | experimental | M0/M3 | `orv-project`, `orv-cli` | CLI graph tests | `orv graph` | File/import/declaration/domain graph |
| HIR origin map | implemented | experimental | M0/M3 | `orv-hir`, `orv-compiler` | origin-map JSON contract test, origin/graph CLI tests | `orv origins`, `orv graph` | Contains/calls semantic edges; public JSON root/entry/span/edge keys and primitive field types are regression-covered |
| Reference tree-walking runtime | implemented | experimental | M0 | `orv-runtime` | `fixtures/e2e/hello.orv` | `orv run` | Main execution path |
| Source test runner | implemented | stable-ish | M0 | `orv-cli`, `orv-runtime` | Test Runner v1 contract regression, test runner CLI tests | `orv test`, `orv test --list` | Discovers `test` blocks, filters by name, emits list JSON schema v1, and executes selected blocks through the reference runtime; public discovery JSON and success/failure CLI envelope are documented in `docs/contracts/TEST_RUNNER_V1.md` |
| HTTP/1.1 `@server` / `@route` | implemented | experimental | M1 | `orv-runtime` | `fixtures/e2e/hello.orv`, `fixtures/e2e/path_param.orv` | `orv run` | Hyper reference server |
| Route origin header | implemented | experimental | M1/M3 | `orv-runtime`, `orv-compiler`, `orv-cli` | origin runtime tests, generated smoke header contract test, route origin header contract doc | `orv run`, generated deploy smoke | Emits `x-orv-origin-id` and branch-specific `x-orv-response-origin-id`; generated smoke verifies exact header values against unambiguous server artifacts |
| Request body parsing | implemented | experimental | M1 | `orv-runtime` | `fixtures/e2e/shopping_mall.orv` | `orv run` | JSON/form-urlencoded into `@body`; raw body available |
| Typed body/form validation | implemented | experimental | M1 | `orv-syntax`, `orv-runtime` | request binding runtime tests, validation response contract test, `fixtures/e2e/shopping_mall.orv` | `orv run`, `orv init` | `@body: T`, `@query: T`, `@form: T` named-schema bindings use runtime validators, normalize request-state values, and return 400 `orv.validation.error` payloads with regression-covered root keys, field keys, `schema_version: 1`, `error: "validation_failed"`, and field errors |
| `@html` static render | implemented | experimental | M1 | `orv-runtime`, `orv-compiler` | `fixtures/e2e/shopping_mall.orv` | `orv run`, `orv build` | HTML page/static build path |
| Client reactive bundle | artifact only | unstable | M4+ | `orv-compiler`, `orv-cli` | build artifact tests, Client Bundle v1 contract regression | `orv build`, `orv verify-build` | Manifest/reactive plan/JS/WASM bootstrap; public manifest/reactive-plan key surfaces are documented in `docs/contracts/CLIENT_BUNDLE_V1.md`; full DOM diff roadmap |
| In-memory `@db` | implemented | reference | M1 | `orv-runtime` | `fixtures/e2e/shopping_mall.orv` | `orv run` | CRUD/filter/sort/limit/reference aggregation |
| DB snapshot/WAL/checkpoint | implemented | reference | M1 | `orv-runtime`, `orv-cli` | DB CLI/runtime tests | `orv db *` | Reference persistence/recovery path |
| SQLite row JSON adapter | implemented | reference | M1/M2 | `orv-runtime` | `fixtures/e2e/shopping_mall.orv` | `orv run` | SQLite file with ORV metadata + row JSON |
| PostgreSQL/MySQL adapters | reference stub | reference | M4+ | `orv-runtime`, `orv-cli` | external DB adapter bridge runtime/deploy artifact/env-check/smoke tests, DB bridge secret redaction integration test | `orv run`, `orv build --prod`, `orv deploy-env-check`, `deploy/smoke-test.sh` | Default handles expose explicit unsupported status/fail query methods; when `ORV_DB_ADAPTER_POSTGRES_ENDPOINT`, `ORV_DB_ADAPTER_MYSQL_ENDPOINT`, or `ORV_DB_ADAPTER_ENDPOINT` is configured, query methods POST checked `http-json-v1` requests to the external bridge with bounded transient retry and return its JSON response. Prod artifacts expose the bridge request/retry shape plus provider-specific and generic endpoint/auth env knobs, deploy env check requires a provider-specific or generic bridge endpoint before launch, generated smoke probes bridge `schema`, and integration coverage asserts DB bridge auth token values do not leak into deploy artifacts or env-check output. Direct provider drivers remain planned |
| Auth/member session scaffold | reference stub | reference | M2 | `orv-cli`, `orv-runtime` | `fixtures/e2e/shopping_mall.orv` | `orv init`, `orv run` | Member/session rows exist, signup stores Argon2 `passwordHash` through `hash.password`, login verifies with `hash.verify`, successful login emits `orv_session` plus role cookies with HttpOnly/SameSite/Secure defaults, `@session required` gates cookie-backed routes, and reference `@Auth required role="admin"` gates shop admin read models |
| CSRF/rate-limit/security defaults | partial | reference | M2 | `orv-runtime`, `orv-cli` | shopping fixture security assertions, rate-limit runtime test | `orv check`, `orv run` | Shop scaffold persists AuditEvent rows, emits reference login session cookies, gates account sessions with `@session required`, protects browser mutation routes with `@csrf`, reference server rate-limits login/checkout/webhook hotspots, and build/server/deploy/native artifacts expose matching `auth_roles`, `session_cookies`, `csrf_protection`, and `rate_limit` runtime features plus per-route `auth`/`session`/`csrf`/`rate_limit` policy descriptors; explicit `@csrf exempt`, `@rateLimit key=... limit=... window=...`, and `@rateLimit exempt` mark intentional exemptions/overrides |
| Payment/shipping local adapters | implemented | reference | M2 | `orv-runtime` | `fixtures/e2e/shopping_mall.orv` | `orv run` | Local/file capture and booking records |
| Payment/shipping HTTP adapters | reference stub | reference | M2/M4+ | `orv-runtime` | commerce adapter tests | `orv run` | Checked JSON POST contract |
| Stripe webhook verification | reference stub | reference | M2/M4+ | `orv-runtime`, `orv-cli` | shop scaffold tests, provider secret redaction integration test, webhook timestamp freshness runtime test | `orv run`, `orv deploy-env-check` | HMAC/idempotency reference path; Stripe-style `t=...,v1=...` signatures require a fresh timestamp within `STRIPE_WEBHOOK_TOLERANCE_SECONDS` or the 300-second default, provider env contracts expose secret names, and integration coverage asserts configured Stripe/carrier secret values do not leak into deploy artifacts or env-check output |
| Provider SDK matrix | planned | non-binding | M4+ | - | - | - | Production hardening later |
| `orv init <dir> --template shop` | implemented | experimental | M2 | `orv-cli` | `fixtures/e2e/shopping_mall.orv` | `orv init` | Catalog/cart/member/checkout/admin scaffold with editable `@design` color/spacing/typography tokens on the home shell, an end-to-end `ProductInput.badge` field path, a checkout stock-reservation/order-create transaction boundary before provider capture/booking, and an audit-visible `payment_captured_pending_shipment` compensation path when shipment booking fails after payment capture |
| Template-to-running-shop smoke path | implemented | experimental | M2 | `orv-cli`, `orv-runtime` | generated smoke-test | `orv init`, `orv build --prod`, smoke-test | First acceptance target before human 5h runs; generated prod smoke now gates on `source-bundle.json`/`project-graph.json`/`origin-map.json` through `orv verify-build .`, then checks route reachability, exact `x-orv-origin-id`/`x-orv-response-origin-id` headers, route/response/DB/commerce/client source reveal through CLI reveal, editor reveal, and LSP reveal, native-server and client-bundle summary counters on reveal payloads, cached DAP `orv editor run-debug . --control next` production summary counters and smoke required-marker contract from the build dir, home copy/theme token rendering, CSRF/session/admin cookies, three product creates, checkout response markers, editable product field propagation, admin dashboard links/storage paths, and customer/admin read-model body markers including webhook/audit surfaces |
| Build artifacts | implemented | experimental | M1/M3 | `orv-compiler`, `orv-cli` | build artifact tests, prod build/deploy schema contract test | `orv build`, `orv verify-build` | Manifest, bundle plan, origin map, graph, source bundle; build manifest/source-bundle/bundle-plan public JSON roots are regression-covered; verify-build rejects project-graph/source-bundle/origin-map drift plus server route/listen/response origin drift |
| Native server plan/source | artifact only | experimental | M4+ | `orv-compiler`, `orv-cli` | build artifact tests, Native Server Plan v1 contract regression | `orv build`, `orv verify-build` | Contract first; full native optimizer planned; public native plan/runtime image/generated source key surfaces are documented in `docs/contracts/NATIVE_SERVER_PLAN_V1.md`; reveal/editor production summaries expose native plan target, route, blocker, runtime image, route source, router source, and handler source metadata |
| Deploy artifacts | implemented | experimental | M2 | `orv-cli`, `orv-compiler` | deploy artifact tests, prod build/deploy schema contract test | `orv build --prod`, `orv deploy-env-check`, `orv benchmark-report` | Manifest/container/Compose/runbook/env/preflight/benchmark-evidence/smoke-test/smoke-output contracts; deploy manifest/preflight/benchmark-evidence public JSON roots plus command/artifact/smoke-output nested contracts are regression-covered; static deploy targets must match the bundle-plan `static_page` target; preflight names the same source-bundle/project-graph/origin-map graph artifacts that verify-build and generated smoke gate on, includes the 5-hour shop benchmark and checked `smoke_output_contract`, links a checked `deploy/benchmark-evidence.json` template keyed to the preflight hash, includes checked `orv editor run-debug . --control next`, `orv benchmark-report .`, and `orv benchmark-report . --require-pass` commands, exposes pass/fail/incomplete JSON for recorded task/smoke/participant evidence, enforces minimum participant-run metadata before pass, and lets benchmark-report consume generated `deploy/smoke-output.txt`, plus trace-enabled run-build command for trace-stream smoke |
| `orv reveal` / editor/LSP reveal payload | implemented | stable-ish | M3 | `orv-cli`, `orv-compiler` | Reveal Payload v1 contract regression, route/html/db/commerce/function/domain/trace black-box integration test | `orv reveal`, `orv editor reveal`, `orv lsp reveal` | Public root/source/focus/location/production key surfaces are documented in `docs/contracts/REVEAL_PAYLOAD_V1.md`; build origin to source/production payload includes graph-contract, native-server/static artifact, preflight smoke evidence summary counters, and route target matching through `contains` plus function `calls` edges |
| Runtime trace JSON / editor trace stream | implemented | stable-ish | M3 | `orv-runtime`, `orv-cli` | Runtime Trace v1 regression, Editor Trace v1 contract regression, editor trace tests, optional generated smoke trace-stream check | `orv editor trace`, `orv editor trace-stream`, `orv editor run-action` | Runtime trace file/EventSource input keys are documented in `docs/contracts/RUNTIME_TRACE_V1.md`; editor trace root/frame/action/trace-stream/native-host action result envelopes are documented in `docs/contracts/EDITOR_TRACE_V1.md`; optional DB operation and commerce adapter origin ids expand into source/production reveal navigation |
| LSP bootstrap | implemented | stable-ish | M3 | `orv-cli` | LSP Bootstrap v1 contract regression, LSP CLI tests | `orv lsp snapshot`, `orv lsp serve` | Snapshot root/document-symbol shape and stdio initialize capability envelope are documented in `docs/contracts/LSP_BOOTSTRAP_V1.md`; symbols/diagnostics/navigation/format/completion method bodies remain bootstrap-level implementation surfaces |
| DAP bootstrap | implemented | experimental | M3 | `orv-cli`, `orv-runtime` | DAP CLI tests, editor debug runner tests, DAP Debug Session v1 contract regression | `orv dap serve`, `orv editor debug`, `orv editor run-debug` | Runtime frame/locals/debug control subsets; launch-time `loadedSources`/`source` snapshots carry imported source checksums into editor/native-host debug payloads; build-backed editor exports and build-dir `run-debug` carry production graph/preflight/native/static/client/smoke summary context into standalone debug runners and result panels; public debug runner/result/panel JSON shapes are documented in `docs/contracts/DAP_DEBUG_SESSION_V1.md`; `orv:frame:N` instruction breakpoints verify against pseudo-instruction frames |
| Static editor export | implemented | stable-ish | M3 | `orv-cli` | Editor Snapshot/Export v1 contract regression, editor export tests, SwiftPM scaffold build probe, generated `.app` packaging/codesign verify probe | `orv editor snapshot`, `orv editor export`, `orv editor host`, `orv editor desktop-shell`, `orv editor desktop-run` | Public snapshot/export/state/native-host envelope keys are documented in `docs/contracts/EDITOR_SNAPSHOT_EXPORT_V1.md`; graph/panel/trace HTML artifacts; production export mirrors source-bundle/project-graph/origin-map graph contract, native-server/static target counters, and preflight smoke evidence summary counts into `state.json`, `native-host.json`, and `production/panel.html`; local native-host bridge server serves exports and executes trace reveal actions through `POST /__orv/native-host/action`; exported desktop package manifest/launcher fixes bridge lifecycle, spawn allowlist, refresh events, source permission roots/counts, read-only denied mode, WebView permission-state injection, source reveal blocking, desktop platform matrix for macOS implemented plus Windows/Linux planned blockers, SwiftPM AppKit/WKWebView desktop container source, and local macOS `.app` bundle packaging/ad-hoc signing plus optional Developer ID notarytool submission/stapling via env-gated packaging; `desktop-shell` consumes that package into a verified native-container session plan, and `desktop-run --probe` exercises host process spawn/readiness/WebView URL derivation |
| First-party native editor UI | planned | non-binding | M4+ | - | - | - | Native shell and production reveal UI later |
| `@gpu` / `@net` / CRDT / broad FFI | reference stub | non-binding | M4+ | `orv-runtime`, `orv-analyzer` | `fixtures/e2e/domains.orv`, `fixtures/default-syntax.orv`, `fixtures/plan/08-superapp-simulation.orv` | `orv run` | Syntax/design pressure and deterministic local stubs only; [ADVANCED_DOMAINS.md](ADVANCED_DOMAINS.md) is the promotion gate before any MVP claim |

## Update Rule

When implementation changes, update this matrix first. Then adjust [MVP.md](MVP.md), [ROADMAP.md](ROADMAP.md), [CHANGELOG.md](CHANGELOG.md), or [SPEC.md](SPEC.md) only if the product boundary, future plan, dated delta, or language contract changed.
