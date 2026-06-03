# orv Implementation Status

이 문서는 상태 용어와 빠른 요약만 둔다. 기능별 정확한 판정은 [IMPLEMENTATION_MATRIX.md](IMPLEMENTATION_MATRIX.md)가 기준이다.

언어 의미론은 [SPEC.md](SPEC.md), 구현 구조는 [ARCHITECTURE.md](ARCHITECTURE.md), 운영 surface 세부는 [OPERATIONAL_SURFACES.md](OPERATIONAL_SURFACES.md)를 따른다.

## Status Terms

| Status | 의미 |
|--------|------|
| implemented | 현재 코드 경로가 동작하고 검증 대상으로 볼 수 있음 |
| reference stub | 레퍼런스 런타임 또는 scaffold에서 제한적으로 동작함 |
| artifact only | 실행 기능보다 산출물/계약/manifest가 먼저 고정됨 |
| planned | 설계 방향은 있으나 구현 경로가 아직 없음 |
| not started | 문서상 아이디어 수준 |

## Quick Summary

현재 orv는 구현 중인 Rust workspace MVP다. `.orv` source load/lex/parse, import/project loading, name resolution, semantic analysis, HIR lowering, reference tree-walking runtime, HTTP/1.1 `@server`, reference DB, library/template surface로 취급하는 reference commerce adapter, build/deploy artifacts, origin/reveal, LSP/DAP/editor bootstrap 일부가 있다.

Native optimizer, production editor reveal UI, custom DB engine, provider SDK matrix, CRDT, `@gpu`, `@net`, broad FFI는 아직 안정 제품 계약이 아니다. 도메인별 추상화는 [PLATFORM_BOUNDARY.md](PLATFORM_BOUNDARY.md)에 따라 compiler core intrinsic이 아니라 compiler plugin surface이고, payment/shipping/Stripe/carrier는 library/provider package surface다.

## Current Gap Snapshot

[IMPLEMENTATION_GAP_REPORT.md](IMPLEMENTATION_GAP_REPORT.md)의 현재 판정은 다음과 같다.

- M0-M3 reference MVP와 artifact contract는 많이 구현됐다.
- 5시간 쇼핑몰 경로는 automated template smoke 기준으로 강하지만, 실제 비개발자 benchmark evidence가 아직 필요하다.
- production-grade platform claim은 direct DB/provider adapters, full native/server-client codegen, first-party editor UI가 닫히기 전까지 보류한다.
- advanced domains는 `IMPLEMENTATION_MATRIX.md`와 [ADVANCED_DOMAINS.md](ADVANCED_DOMAINS.md)에서 `reference stub`, `artifact only`, `planned`, `non-binding`으로 명시된 한 MVP 진행률에 포함하지 않는다.

현재 실행 초점은 기능 폭 확대보다 계약 안정화, shop benchmark evidence, compiler/library/provider boundary 결정이다.

## Current Execution Focus (2026-06-01)

ULW 기준 G001-G010 evidence slice는 완료된 계약 안정화 작업으로 취급한다. G001-G006은 automated/pre-human MVP evidence gate, G007은 provider-like domain과 benchmark evidence hardening, G008은 consumer artifact boundary evidence, G009는 DAP/editor source-bundle evidence, G010은 reference MVP proof gate drift guard를 닫았다. 상위 goal은 "ORV MVP 완성"이 아니라 "M0-M3 reference MVP contract를 제품 claim 가능한 수준으로 좁혀 증명"으로 해석한다. 실제 실행 목표는 작고 검증 가능한 하위 goal로만 진행한다.

현재 실행 goal은 `G011: M0-M3 reference MVP proof gate lock`이다.

G011은 코드로 닫을 수 있는 proof gate만 포함한다. 실제 비개발자 5시간 benchmark run처럼 사람 참여나 외부 provider 계정이 필요한 작업은 별도 acceptance gate로 남긴다.

- Core spine은 `source-bundle.json`, `project-graph.json`, `origin-map.json`, runtime trace, editor trace, DAP production summary, reveal payload가 같은 source/origin/build identity를 보존한다.
- Imported-source path는 entry/import source map, diagnostics, source-bundle rehydration, DAP/editor source lookup까지 원본 파일 identity를 잃지 않는다.
- Shop acceptance는 fresh `orv init --template shop -> check -> build --prod -> verify-build -> deploy-env-check -> run-build -> smoke-test -> benchmark-prepare -> benchmark-report` 경로가 자동으로 재현된다.
- Benchmark evidence gate는 recorded deploy verifier와 `benchmark-report --require-pass`가 같은 human evidence contract를 강제한다: non-empty reviewer/notes, strict UTC `reviewed_at`, participant completion 이후 review, required review booleans, retained raw notes identity/hash parity.
- Platform boundary는 compiler core가 provider/shop semantics를 소유하지 않도록 유지한다. `@server`, `@route`, `@html`, `@db`, `@design`, `@Auth`는 first-party compiler plugin surface이고, `@payment`, `@shipping`, Stripe, carrier, shop checkout은 library/template/provider package surface다.
- 검증은 touched surface 중심의 focused DAP/editor/verify-build/benchmark/boundary contract test를 우선하고, 작업 후 evidence 기록, 커밋, clean worktree를 유지한다.

### Concrete Goal Contract

상위 goal은 Codex goal tool에 active 상태로 남아 있지만, 실행 단위는 다음 완료 조건으로 관리한다. Goal tool objective를 직접 수정할 수 없을 때는 이 절이 operational source of truth다. G011은 "남은 MVP 전체 구현"이 아니라, 코드와 자동화로 닫을 수 있는 M0-M3 proof gate만 닫는다. G011이 닫히면 다음 goal은 실제 human benchmark evidence 수집 또는 M4+ product hardening 중 하나로 별도 승격한다.

Operational goal string:

> G011은 M0-M3 reference MVP contract를 제품 claim 가능한 pre-human 증거 수준으로 lock한다. 완료 조건은 fresh shop template smoke, benchmark pre-human report, source-bundle/ProjectGraph/origin-map/runtime trace/reveal/editor/DAP source identity, imported-file diagnostics/source rehydration, recorded deploy/report human-evidence verifier parity, compiler/provider/platform boundary guard가 각각 focused regression과 커밋으로 증명되고 worktree가 clean인 것이다.

| Track | Required proof | Next code-owned check |
|-------|----------------|-----------------------|
| G011-A Core graph spine | `verify-build`가 source-bundle, ProjectGraph, OriginMap, route/listen/response origin, graph edge/origin-link, trace version/kind drift를 field/path-oriented error로 거부한다 | touched surface가 아니면 추가 확장보다 기존 focused contract를 유지한다 |
| G011-B Imported diagnostics/source rehydration | imported two-file fixture의 diagnostics가 `span.file` 기준 파일/라인을 보고하고, build 후 원본 entry/import 파일을 삭제해도 DAP `loadedSources`와 `source`가 source-bundle의 정확한 text/path/checksum을 반환한다 | source-bundle/DAP/editor path를 건드릴 때 checksum/text/path 삼중 parity를 재검증한다 |
| G011-C Production summary parity | editor/DAP/reveal production summary가 source-bundle path/hash/file count, loaded source count, source snapshot count, ProjectGraph/origin counters를 같은 값으로 보존한다 | `dap_editor_source_bundle_summary_parity_contract`가 editor run-debug/reveal production summary의 shared public fields 전체 parity를 검증하고, summary key 추가 시 CLI/editor/DAP/reveal public key drift guard를 함께 갱신한다 |
| G011-D Shop pre-human smoke | fresh shop template에서 generated smoke가 graph/source-bundle/origin-map, route origin headers, reveal, client/native/DAP summary, smoke-output markers를 검증하고 `benchmark-report`는 human evidence 전 `incomplete`를 유지한다 | `shop_acceptance_smoke_contract`가 `scripts/shop_acceptance_smoke.sh` 자체를 fresh temp shop workspace에서 실행하고 pre-human `incomplete` report/artifact handoff를 검증한다 |
| G011-E Benchmark human evidence verifier | recorded deploy verifier와 benchmark report가 reviewer/notes, UTC review timestamp, participant completion ordering, required review booleans, retained raw-notes path/identity/hash parity를 같은 기준으로 강제한다 | `benchmark_report_rejects_blank_human_review_text_fields`가 report-side blank `human_evidence_review.reviewer`/`notes` rejection을 deploy verifier parity와 같이 고정한다 |
| G011-F Platform boundary guard | provider-named calls cannot promote to core intrinsic, first-party compiler plugin, commerce adapter artifact, adapter runtime feature, or `orv-compiler` production dependency leakage | 현재 obvious gap 없음; 새 provider alias나 adapter feature가 생길 때 boundary inventory test를 먼저 갱신한다 |
| G011-Z Work discipline | 각 하위 goal은 focused test, 필요한 subagent scan, evidence note, commit, clean worktree로 닫는다 | broad full-suite보다 touched-surface unit/smoke를 우선하되 계약 변경은 문서와 같이 커밋한다 |

Immediate G011 queue:

1. Add a focused `check-artifact` imported-source rehydration regression for cross-file secondary labels.
2. Continue Core graph spine and Platform boundary guard audit before adding new scope.

### Out Of G011

- 실제 비개발자 5시간 benchmark run과 recorded human evidence collection
- first-party native editor product polish
- full DAP/LSP method expansion beyond current bootstrap contracts
- production Stripe/carrier provider SDK packages
- direct PostgreSQL/MySQL drivers and custom DB engine
- native optimizer, dynamic client DOM diff/codegen, CRDT/GPU/media/network/FFI advanced domains

## Status Update Rule

When implementation changes, update [IMPLEMENTATION_MATRIX.md](IMPLEMENTATION_MATRIX.md) first, then adjust this summary if the user-facing story changed.
