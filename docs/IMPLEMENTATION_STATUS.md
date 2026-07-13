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

## Current Execution Focus (2026-07-13)

G001-G011 evidence slice는 완료된 계약 안정화 작업으로 취급한다. G011의 A-F
proof track은 focused regression으로 닫혔고, 2026-07-13 재정립에서 전체
workspace 테스트 스위트 완주(green), deny-level clippy 게이트 복구, CI
lint/test workflow(`.github/workflows/ci.yml`)와 로컬
`scripts/preflight.sh` 게이트 도입으로 work discipline이 사람 규율에서
기계 게이트로 승격됐다. 과거 ULW/외부 goal tool 운영 잔재는 이 문서 기준으로
종료한다.

현재 실행 goal은 `G012: 재정립(renewal) 사이클`이다.

G012는 기능 폭 확대가 아니라 검증 가능한 제품 증거와 구조 부채 해소에
집중한다. 우선순위 순서:

| Track | 내용 | 완료 조건 |
|-------|------|-----------|
| G012-A Human benchmark evidence | 실제 비개발자 1명 이상이 5시간 shop benchmark를 수행하고 recorded evidence가 verifier를 통과한다 | `benchmark-report --require-pass`가 실데이터로 `passed`를 반환하고 raw notes/smoke output 아티팩트가 보존된다 |
| G012-B `orv-cli` 구조 분할 | `editor_lsp_dap.rs` → `editor/`/`lsp/`/`dap/`, `tests.rs` → 표면별 모듈, `build_deploy.rs` → deploy/benchmark/verify 분리 | 행동 변경 0의 순수 이동 + full suite green + clippy clean |
| G012-C `libs/std` boundary 결정 | runtime intrinsic(`@out`/`@fs`/`@fetch` 등) 중 stdlib 이동 후보와 compiler plugin protocol 첫 실물 범위 결정 | `PLATFORM_BOUNDARY.md` 개정과 seed → 실구현 전환 또는 명시 보류 기록 |
| G012-Z Work discipline | 모든 커밋은 `scripts/preflight.sh`(fmt+clippy)를 통과하고, 계약 변경은 full suite로 검증한다 | CI workflow green 유지, main 상시 green |

### Out Of G012

- first-party native editor product polish
- full DAP/LSP method expansion beyond current bootstrap contracts
- production Stripe/carrier provider SDK packages
- direct PostgreSQL/MySQL drivers and custom DB engine
- native optimizer, dynamic client DOM diff/codegen, CRDT/GPU/media/network/FFI advanced domains

## Status Update Rule

When implementation changes, update [IMPLEMENTATION_MATRIX.md](IMPLEMENTATION_MATRIX.md) first, then adjust this summary if the user-facing story changed.
