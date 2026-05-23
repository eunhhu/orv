# DB Adapters v1 Contract

Producers:

- `@db.connect(...)` in source
- reference runtime DB handles and external bridge calls
- `orv build . --prod --out dist`
- `orv verify-build`, `orv deploy-env-check`, and generated smoke scripts
- `orv reveal`, `orv editor reveal`, and `orv lsp reveal`

Current regression coverage:

- `crates/orv-cli/tests/db_adapters_contract.rs::db_adapters_v1_freezes_external_bridge_artifacts`
- `crates/orv-runtime/src/interp.rs::tests::db_connect_external_adapter_reports_status_and_rejects_queries`
- `crates/orv-runtime/src/interp.rs::tests::db_connect_external_adapter_bridge_posts_checked_json`
- `crates/orv-runtime/src/interp.rs::tests::db_connect_external_adapter_bridge_retries_transient_errors`
- `crates/orv-cli/tests/provider_secret_redaction_contract.rs`
- DB adapter deploy/reveal/verify-build regressions in `crates/orv-cli/src/tests.rs`

This contract freezes the reference DB adapter boundary for local DB adapters
and external PostgreSQL/MySQL HTTP bridge adapters. It does not make direct
PostgreSQL/MySQL drivers production-complete; direct provider drivers remain a
later M4+ contract.

## Runtime Boundary

`@db.connect(url)` returns a DB handle.

Supported reference URL schemes:

- `memory://...` uses the in-memory reference DB
- `file://...` uses the reference WAL/snapshot path
- `sqlite://...` uses the SQLite row JSON adapter
- `postgres://...` creates an external PostgreSQL adapter handle
- `mysql://...` creates an external MySQL adapter handle

Without a configured bridge endpoint, external PostgreSQL/MySQL handles expose
`adapterStatus: "unsupported_runtime"` and reject query methods with a runtime
error. They must not silently fall back to in-memory behavior.

With a bridge endpoint configured, external handles expose
`adapterStatus: "bridge_configured"`, `runtime.status: "bridge_configured"`,
and `runtime.contract: "http-json-v1"`.

## HTTP JSON Bridge

External DB bridge calls POST checked JSON to the configured bridge endpoint:

```json
{
  "kind": "orv.db.adapter",
  "contract": "http-json-v1",
  "provider": "postgres",
  "url": "postgres://host/db",
  "method": "create",
  "args": []
}
```

Provider-specific endpoint envs are preferred:

- `ORV_DB_ADAPTER_POSTGRES_ENDPOINT`
- `ORV_DB_ADAPTER_MYSQL_ENDPOINT`

The shared fallback endpoint is:

- `ORV_DB_ADAPTER_ENDPOINT`

Provider-specific auth token envs are preferred:

- `ORV_DB_ADAPTER_POSTGRES_AUTH_TOKEN`
- `ORV_DB_ADAPTER_MYSQL_AUTH_TOKEN`

The shared fallback token is:

- `ORV_DB_ADAPTER_AUTH_TOKEN`

When an auth token is configured, runtime bridge requests use a bearer
`Authorization` header. Secret values must not be printed into generated deploy
artifacts, env-check output, reveal payloads, or logs.

Bridge requests retry bounded transient failures. The public artifact contract
advertises three attempts for `5xx`, connect, read, and timeout failures.

## Production Artifact Boundary

Production builds that contain external DB adapters must write
`deploy/db-adapters.json` and link it from `deploy/manifest.json` as
`server.db_adapters`.

The artifact root contains:

- `schema_version: 1`
- `artifact: "server/app.orv-runtime.json"`
- `adapters: [...]`

Each external adapter entry exposes:

- `kind: "db"`
- `mode: "external"`
- `provider`: `postgres` or `mysql`
- `env`: source env override name or `null`
- `default`: source fallback URL or `null`
- `endpoint`: source DB URL
- `adapter_status: "unsupported_runtime"`
- `runtime.status: "unsupported_runtime"`
- `runtime.query_methods`: `create`, `find`, `update`, `delete`,
  `transaction`
- `bridge.contract: "http-json-v1"`
- `bridge.method: "POST"`
- `bridge.content_type: "application/json"`
- `bridge.query_methods`: supported bridge method names including `schema`
- `bridge.body`: the documented bridge payload shape
- `bridge.retry.attempts: 3`
- `bridge.env`: provider-specific endpoint/token envs plus shared fallback envs
- `source_origin_id`: the primary `origin-map.json` call id for `@db.connect`
- `source_origin_ids`: all source call ids merged into this adapter entry

External DB adapter builds also expose:

- `server.persistence.db_endpoints` in `deploy/manifest.json`
- `server.persistence.db_env` env/default pairs when source uses `@env`
- matching container persistence metadata
- generated Compose env defaults for source DB URL and bridge endpoint envs
- generated `deploy/env.example` placeholders for source DB URL and bridge
  endpoint envs
- generated runbook lines for DB endpoints, source DB envs, and bridge envs
- generated preflight `required_env` entries for provider-specific bridge
  endpoints
- generated smoke checks for `deploy/db-adapters.json`, the `http-json-v1`
  contract marker, and safe bridge `schema` probes

## Reveal Boundary

Reveal payloads for a DB adapter origin must include the generated
`deploy/db-adapters.json` target and a matched adapter entry. Matched external
adapter entries include provider, endpoint, selected origin, match kind, and
`bridge.contract: "http-json-v1"` metadata.

## Version Policy

Breaking changes to runtime supported schemes, unsupported-runtime behavior,
bridge request JSON shape, retry metadata, bridge env names, auth-token
redaction, `deploy/db-adapters.json` root keys, adapter entry keys,
source-origin linkage, preflight env requirements, smoke bridge probes, or
reveal matched-adapter fields require a contract update, changelog entry, matrix
update, and regression update.
