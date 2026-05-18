# orv Ardex Workflow

이 문서는 Codex 작업 상태를 Ardex 기준으로 이어가기 위한 운영 메모다. 제품 상태와 우선순위의 기준은 `docs/IMPLEMENTATION_GAP_REPORT.md`, `docs/ROADMAP.md`, `docs/MVP.md`를 따른다.

## Current Ardex Project

- Project: `p_042`
- Name: `miol`
- Path: `/Users/sunwoo/work/miol`
- Workflow: `sdd_vdd`
- Session: `s_001`
- Session goal: `Complete orv document-driven final MVP: freeze P0 contracts, finish P1 shop acceptance path, then harden P2 production boundaries.`

Codex hook state is installed outside this repository in `/Users/sunwoo/.codex/hooks.json`. The active hooks should point to `/Users/sunwoo/.ardex/hooks/*`; stale temporary Ardex hook paths must not remain.

## Backlog Order

The migrated Ardex backlog mirrors the current roadmap execution order:

| Ardex task | Priority | Gate | Scope |
|------------|----------|------|-------|
| `t_002` | 2 | `none` | P0 freeze origin-map JSON contract |
| `t_007` | 3 | `none` | P0 freeze runtime trace JSON and origin header contracts |
| `t_008` | 4 | `test` | P0 freeze build deploy preflight benchmark evidence schemas |
| `t_009` | 5 | `review` | Refactor runtime server module below Ardex scale threshold before source edits |
| `t_003` | 6 | `test` | P1 close shop acceptance smoke and benchmark evidence path |
| `t_004` | 7 | `test` | P2 harden production DB and commerce boundary decisions |
| `t_005` | 8 | `test` | P3 lock reveal coverage across route/html/db/domain traces |
| `t_006` | 9 | `spec` | P4 keep advanced domains non-binding unless benchmark-critical |

`t_001` is the completed setup migration task. It recorded accepted evidence for Codex hook JSON validation, current project resolution, and Codex metadata migration.

## Operating Rules

- Start each implementation turn with `ardex -p p_042 statement --json`.
- Claim exactly one task before editing: `ardex -p p_042 task <id> claim --json`.
- Run `ardex -p p_042 scale check --path <path> --json` before broad edits.
- Prefer narrow verification commands, for example `rtk cargo test -p <crate> <test-name>` or `rtk cargo check -p <crate> --tests`.
- Do not duplicate Codex command/file logs in Ardex evidence. Use evidence for user decisions, external URLs, manual QA notes, deploy links, or artifacts Codex cannot reconstruct.
- Run `ardex -p p_042 task <id> checklist --json` before `task done`.
- Keep advanced domains outside MVP unless they improve the 5-hour shop benchmark or close a documented production safety gap.
