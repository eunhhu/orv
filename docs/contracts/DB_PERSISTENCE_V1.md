# DB Persistence v1 Contract

Producers:

- `@db` in the reference runtime
- `@db.save/load`, `@db.wal(path)`, `@db.checkpoint()`,
  `@db.savepoint()`, and `@db.rollback(point)`
- local `@db.connect(...)` adapters using `memory://`, `file://`, and
  `sqlite://`
- `orv build . --prod --out dist`
- `orv verify-build`, `orv deploy-env-check`, `orv run-build`, and DB CLI
  migration/recovery/archive commands

Current regression coverage:

- `crates/orv-cli/tests/db_persistence_contract.rs::db_persistence_v1_freezes_local_wal_sqlite_deploy_handoff`
- `crates/orv-runtime/src/interp.rs::tests::db_connect_file_adapter_replays_and_persists_wal`
- `crates/orv-runtime/src/interp.rs::tests::db_connect_sqlite_adapter_replays_and_persists_rows`
- `crates/orv-runtime/src/interp.rs::tests::db_checkpoint_compacts_wal_and_preserves_row_ids`
- `crates/orv-runtime/src/interp.rs::tests::db_wal_savepoint_rollback_survives_replay`
- DB recovery/archive/crash-matrix coverage in
  `crates/orv-cli/tests/db_data_migration.rs`

This contract freezes the local reference persistence boundary. External
PostgreSQL/MySQL bridge behavior is covered separately by
[DB Adapters v1](DB_ADAPTERS_V1.md).

## Runtime Boundary

`@db` starts from an in-memory table map and supports the reference CRUD,
filter, sort, limit, and aggregation operations used by the shop MVP.

`@db.save(path)` writes a JSON snapshot. `@db.load(path)` restores that
snapshot. `@db.wal(path)` appends mutation records to a JSONL write-ahead log
and replays existing records when opened.

`@db.checkpoint()` compacts WAL-backed state while preserving row ids and
query-visible data. `@db.savepoint()` captures an in-memory rollback point.
`@db.rollback(point)` restores that state. WAL-backed savepoint rollback must
survive replay after reopening.

`@db.connect(url)` supports these local reference schemes:

- `memory://...`: in-memory reference handle
- `file://relative/path.jsonl`: WAL-backed local handle
- `sqlite://relative/path.sqlite`: SQLite row JSON handle

The SQLite adapter stores ORV table metadata plus row JSON in a real SQLite
file while preserving reference query semantics.

## Production Artifact Boundary

Production builds with local DB persistence must expose the same persistence
shape in all deploy-facing artifacts:

- `deploy/manifest.json` at `server.persistence`
- `deploy/container.json` at `persistence`
- `deploy/preflight.json` at `persistence`

The persistence object contains:

- `wal_paths`: relative WAL paths from `@db.wal(...)` and `file://` adapters
- `db_paths`: relative SQLite paths from `sqlite://` adapters
- `db_env`: source env/default pairs for env-configured local DB adapters
- `db_endpoints`: empty for local adapters
- `db_adapters`: empty for local adapters
- `volumes`: Compose volume descriptors derived from parent directories

For:

```orv
let waldb = @db.connect "file://data/app.wal.jsonl"
let shopdb = @db.connect(@env.SHOP_DATABASE_URL ?? "sqlite://data/app.sqlite")
```

the public persistence handoff is:

```json
{
  "wal_paths": ["data/app.wal.jsonl"],
  "db_paths": ["data/app.sqlite"],
  "db_env": [
    {
      "env": "SHOP_DATABASE_URL",
      "default": "sqlite://data/app.sqlite"
    }
  ],
  "db_endpoints": [],
  "db_adapters": [],
  "volumes": [
    {
      "host": "data",
      "container": "/app/data",
      "compose_mount": "../data:/app/data"
    }
  ]
}
```

Build, server runtime, deploy manifest, and deploy preflight artifacts must
include the `db_adapter` runtime feature when source uses `@db.connect`.

## Deploy Handoff

Generated Compose files must mount local persistence directories into the
container and preserve env/default handoff for env-configured SQLite/file
adapters:

```yaml
SHOP_DATABASE_URL: "${SHOP_DATABASE_URL:-sqlite://data/app.sqlite}"
```

`deploy/env.example` must contain the default assignment:

```text
SHOP_DATABASE_URL=sqlite://data/app.sqlite
```

`deploy/README.md` must document each WAL path, SQLite DB path, DB adapter env,
and Compose mount. `orv deploy-env-check` must accept env-configured local
adapters when a default is present.

`orv verify-build` must reject drift when deploy manifest, container, preflight,
runbook, Compose, or env-example persistence data stops matching source-bundled
reanalysis.

## DB CLI Boundary

The DB CLI is part of the same reference persistence surface:

- `orv db plan/verify/apply/migrate/rollback/squash` manage schema/data
  snapshot changes.
- `orv db backup/restore` read and write local snapshot artifacts.
- `orv db recover` replays WAL or archive manifests by record count, unix ms, or
  RFC3339 cutoff.
- `orv db archive` writes WAL manifests and supports local file, HTTP, and
  S3-compatible reference targets.
- `orv db crash-matrix` reports replay, torn EOF, corruption, checkpoint,
  savepoint rollback, PITR, and archive hash-mismatch scenarios.

These commands remain reference persistence tooling, not a custom production DB
engine.

## Version Policy

Breaking changes to local DB URL schemes, snapshot/WAL/checkpoint/savepoint
semantics, SQLite row JSON persistence, deploy `persistence` keys, runtime
feature names, Compose/env-example/runbook handoff, `deploy-env-check` default
handling, `verify-build` drift checks, or DB CLI recovery/archive semantics
require a contract update, changelog entry, matrix update, and regression
update.
