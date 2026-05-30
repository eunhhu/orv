# Commerce Adapters v1 Contract

Producers:

- `@payment.connect(...)` and `@shipping.connect(...)` in source
- reference runtime `capture`, `book`, and provider webhook methods
- `orv build . --prod --out dist`
- `orv verify-build` and `orv deploy-env-check`
- `orv reveal`, `orv editor reveal`, and `orv lsp reveal`

Current regression coverage:

- `docs/samples/commerce-adapters-v1.golden.json`
- `docs/samples/provider-secret-redaction-v1.golden.json`
- `crates/orv-cli/tests/commerce_adapters_contract.rs::commerce_adapters_v1_freezes_http_adapter_artifacts`
- `crates/orv-runtime/src/interp.rs::tests::payment_and_shipping_http_adapters_post_json_payloads`
- `crates/orv-runtime/src/interp.rs::tests::payment_and_shipping_file_adapters_append_records`
- `crates/orv-runtime/src/interp.rs::tests::payment_and_shipping_provider_adapters_support_shop_flow`
- `crates/orv-cli/tests/commerce_provider_hardening_contract.rs`
- `crates/orv-cli/tests/provider_secret_redaction_contract.rs`
- commerce adapter deploy/reveal/verify-build regressions in `crates/orv-cli/src/tests.rs`

This contract freezes the reference commerce adapter boundary for local files,
HTTP JSON bridge adapters, and provider-mode reference handles. Commerce is a
first-party library/template surface, not a compiler core intrinsic. The reusable
platform contract is generic adapter/secret/idempotency/origin/deploy metadata as
defined in [Platform Boundary](../PLATFORM_BOUNDARY.md). It does not make Stripe
or carrier SDK integrations production-complete; provider SDK hardening remains
a later M4+ package/provider contract.

The published golden fixture is
`docs/samples/commerce-adapters-v1.golden.json`. It freezes normalized HTTP
payment/shipping adapter artifacts, source-origin linkage without generated
origin ids, deploy/container/Compose/runbook handoff markers, and reveal matched
adapter/source-command shape.

The cross-contract redaction fixture
`docs/samples/provider-secret-redaction-v1.golden.json` also freezes that
provider-mode commerce deploy artifacts and satisfied env-check output omit
configured secret values.

## Runtime Boundary

`@payment.connect(url)` returns a payment adapter handle. `@shipping.connect(url)`
returns a shipping adapter handle. These names belong to the reference commerce
library surface. Compiler/build/reveal code must treat them as adapter metadata,
not special payment/shipping semantics.

Supported reference URL schemes:

- `test://...` and `local://...` return deterministic local reference values
- `file://...` appends JSONL records under the runtime working directory
- `http://...` sends checked JSON POST requests and returns the JSON response
- `stripe://...` enables the reference payment provider path
- `carrier://...` enables the reference shipping provider path

Unsupported schemes fail in the reference runtime with a native runtime error
instead of silently falling back.

## HTTP JSON Bridge

HTTP payment capture sends one POST request to the adapter endpoint with:

```json
{
  "kind": "payment.capture",
  "payload": {}
}
```

HTTP shipping booking sends one POST request to the adapter endpoint with:

```json
{
  "kind": "shipping.booking",
  "payload": {}
}
```

The request content type is `application/json`. The adapter response must be
JSON; the reference runtime parses the response and returns it as an ORV value.
Non-JSON HTTP responses are runtime errors.

## File Adapter Boundary

`file://...` adapters append JSONL reference records. Production builds expose
relative file adapter paths as persistent `record_paths`, and generated Compose
mounts the parent record directory. File adapter paths are not exposed as HTTP
commerce endpoints.

## Provider Boundary

Provider handles expose stable reference metadata without leaking configured
secret values:

- `stripe://...` payment capture uses provider-mode payment metadata and
  supports Stripe-style webhook verification
- `carrier://...` shipping booking uses provider-mode shipment metadata
- provider credential and webhook-secret env contracts are emitted into deploy
  artifacts
- generated env checks report missing or configured secret state without
  printing secret values
- retry/idempotency, previous-secret webhook rotation, and provider env gates
  are documented in
  [Commerce Provider Hardening v1](COMMERCE_PROVIDER_HARDENING_V1.md)

## Production Artifact Boundary

Production builds that contain commerce adapters must write
`deploy/commerce-adapters.json` and link it from `deploy/manifest.json` as
`server.commerce_adapters`.

The artifact root contains:

- `schema_version: 1`
- `kind: "orv.deploy.commerce_adapters"`
- `artifact: "server/app.orv-runtime.json"`
- `adapters: [...]`

Each adapter entry exposes:

- `kind`: `payment` or `shipping`
- `mode`: `file`, `http`, or `provider`
- `env`: env override name or `null`
- `default`: source fallback URL or `null`
- `endpoint`: HTTP/provider endpoint or `null`
- `record_path`: JSONL record path or `null`
- `request.method: "POST"`
- `request.content_type: "application/json"`
- `request.kind`: `payment.capture` or `shipping.booking`
- `request.body.kind`: same value as `request.kind`
- `source_origin_id`: the primary `origin-map.json` call id for the
  corresponding `@payment.connect` or `@shipping.connect`
- `source_origin_ids`: all source call ids merged into this adapter entry

HTTP adapter builds also expose:

- `server.persistence.commerce_endpoints` in `deploy/manifest.json`
- `server.persistence.commerce_env` env/default pairs when source uses `@env`
- matching `container.persistence.commerce_env`
- generated Compose env defaults
- generated runbook lines for adapter env/defaults

## Reveal Boundary

Reveal payloads for a commerce adapter origin must include the generated
`deploy/commerce-adapters.json` target and a matched adapter entry. HTTP matched
adapter entries include endpoint and request metadata so editor/LSP/native
surfaces can reveal the source origin and the generated adapter call contract.

## Version Policy

Breaking changes to runtime supported schemes, HTTP request kind names, response
JSON parsing behavior, `deploy/commerce-adapters.json` root keys, adapter entry
keys, nested request/body/provider-env keys, source-origin linkage, or reveal
matched-adapter fields require a contract update, changelog entry, matrix
update, and regression update.
