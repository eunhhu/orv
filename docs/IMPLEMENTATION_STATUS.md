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

ULW 기준 G001-G008 evidence slice는 완료된 계약 안정화 작업으로 취급한다. G001-G006은 automated/pre-human MVP evidence gate, G007은 provider-like domain과 benchmark evidence hardening, G008은 consumer artifact boundary evidence를 닫았다. 상위 goal은 "ORV MVP 완성"이 아니라 "M0-M3 reference MVP contract를 제품 claim 가능한 수준으로 좁혀 증명"으로 해석한다. 실제 실행 목표는 작고 검증 가능한 하위 goal로만 진행한다.

현재 실행 goal은 `G009: DAP/editor source-bundle evidence`다.

- DAP stdio `launch`가 `sourceBundle`만으로 entry source와 imported source를 재수화해야 한다. 원본 `.orv` 파일이 사라진 뒤에도 `loadedSources`와 `source` 응답은 file count, source reference, checksum, exact source text를 유지한다.
- `orv editor run-debug`와 editor/debug panel payload는 build directory에서 source-bundle path/hash/file count, loaded source checksums, source snapshot count, ProjectGraph node count, origin entry count를 같은 production summary로 보존한다.
- `orv verify-build`와 generated deploy smoke path는 stale source-bundle checksum, missing source-bundle marker, wrong file count, missing DAP/production-summary marker를 조용히 통과시키면 안 된다.
- 실제 비개발자 5시간 benchmark run, first-party native editor UI polish, full DAP method expansion, provider SDK production packages, native optimizer는 이 goal 밖에 둔다.
- 검증은 touched surface 중심의 focused DAP/editor/verify-build contract test를 우선하고, 작업 후 evidence 기록, 커밋, clean worktree를 유지한다.

### Concrete Goal Contract

상위 goal은 Codex goal tool에 active 상태로 남아 있지만, 실행 단위는 다음 완료 조건으로 관리한다.

| Scope | 완료 조건 |
|-------|-----------|
| DAP source rehydration | imported two-file fixture를 build한 뒤 원본 entry/import 파일을 삭제해도 DAP `loadedSources`와 `source`가 source-bundle의 정확한 내용을 반환한다 |
| Editor production summary | `editor run-debug` payload와 panel summary가 source-bundle path/hash/file count, loaded source count, source snapshot count, ProjectGraph/origin counters를 같은 값으로 보존한다 |
| Artifact drift rejection | imported source-bundle checksum drift나 DAP/production-summary marker drift는 `verify-build` 또는 generated smoke gate에서 field/path-oriented error로 실패한다 |
| Work discipline | 각 하위 goal은 focused test, 필요한 subagent scan, artifact/evidence 기록, commit, clean worktree로 닫는다 |

## Status Update Rule

When implementation changes, update [IMPLEMENTATION_MATRIX.md](IMPLEMENTATION_MATRIX.md) first, then adjust this summary if the user-facing story changed.
