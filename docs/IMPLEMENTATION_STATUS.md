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

## Current Execution Focus (2026-09-09)

G001-G011 evidence slice는 완료된 계약 안정화 작업으로 취급한다. G011의 A-F
proof track은 focused regression으로 닫혔고, 2026-07-13 재정립에서 전체
workspace 테스트 스위트 완주(green), deny-level clippy 게이트 복구, CI
lint/test workflow(`.github/workflows/ci.yml`)와 로컬
`scripts/preflight.sh` 게이트 도입으로 work discipline이 사람 규율에서
기계 게이트로 승격됐다. 과거 ULW/외부 goal tool 운영 잔재는 이 문서 기준으로
종료한다.

현재 실행 goal은 `G012: 재정립(renewal) 사이클`이다.

2026-09-03 maintenance baseline에서 floating nightly를 pinned stable
toolchain으로 교체하고 선언 MSRV를 별도 CI job으로 검증하도록 복구했다. DAP
integration contract의 공용 process/tempdir/timeout harness도 도입했다. 이는
G012-B의 테스트 인프라 선행 slice다. 2026-09-09에는 대형 CLI 구현을
`editor/`, `lsp/`, `dap/`, `build_deploy/`로, 테스트를 기능별 `tests/`
모듈로 분리했다. 파일 이동은 visibility/include 경로/포매팅 차이를
제외한 Rust item 비교로 확인한다. 같은 패치의 기능 수정은 SQLite
transaction rollback, 예외 후 호출 상태 복원, 컨테이너 bind 주소,
LSP UTF-16 좌표이며 각각 별도 회귀 테스트로 검증한다.

후속 리팩터에서는 JSON key 검사 중복과 bundle target 검증의 반복 파일
읽기/파싱을 제거했다. CLI 통합 테스트 실행 대상 39개를 하나로 합치고,
산출물 오류 사례 138개를 13개 fixture 공유 묶음으로 전환했다. 공개 계약의
중복 단위 테스트 2개와 golden 비교를 반복하는 수동 검사를 정리했으며,
병렬 테스트의 임시 경로 충돌도 수정했다. 현재 전체 결과는 1,465 tests /
17 suites 통과다. 테스트 함수 수 감소는 사례 묶음 전환과 중복 제거에 따른
것이며, 운영 smoke나 언어·런타임 회귀 테스트를 제외하지 않는다.

G012는 기능 폭 확대가 아니라 검증 가능한 제품 증거와 구조 부채 해소에
집중한다. 우선순위 순서:

| Track | 내용 | 완료 조건 |
|-------|------|-----------|
| G012-A Human benchmark evidence — 대기 | 실제 비개발자 최소 2명(목표 3명)이 5시간 shop benchmark를 수행한다. 1명 실행은 pilot이며 통과 cohort가 아니다 | `benchmark-report --require-pass`가 실데이터로 `passed`를 반환하고 raw notes/smoke output 아티팩트가 보존된다. 기존 실측 기록 없음; [실행 안내](HUMAN_BENCHMARK_RUNBOOK.md) 준비 |
| G012-B `orv-cli` 구조 분할·중복 정리 — 완료 | editor/LSP/DAP와 build/deploy/verify 모듈 분리, JSON 검사 공유, 검증 내 artifact 재사용, 통합 테스트 실행 대상 및 반복 fixture 통합 | 최초 이동 item 1,869개 동등성 확인; 후속 리팩터 포함 full suite 1,465 tests / 17 suites 통과. 테스트 구조는 [TESTING.md](TESTING.md) 참고 |
| G012-C `libs/std` boundary 결정 — 결정 완료 | std wrapper 후보와 `orv-design` 첫 in-process hook 범위 결정 | `PLATFORM_BOUNDARY.md`에 seed 유지, 실제 추출의 명시 보류와 승격 조건 기록 |
| G012-Z Work discipline | 모든 커밋은 `scripts/preflight.sh`(fmt+clippy)를 통과하고, 계약 변경은 full suite로 검증한다 | CI workflow green 유지, main 상시 green |

### Out Of G012

- first-party native editor product polish
- full DAP/LSP method expansion beyond current bootstrap contracts
- production Stripe/carrier provider SDK packages
- direct PostgreSQL/MySQL drivers and custom DB engine
- native optimizer, dynamic client DOM diff/codegen, CRDT/GPU/media/network/FFI advanced domains

## Status Update Rule

When implementation changes, update [IMPLEMENTATION_MATRIX.md](IMPLEMENTATION_MATRIX.md) first, then adjust this summary if the user-facing story changed.
