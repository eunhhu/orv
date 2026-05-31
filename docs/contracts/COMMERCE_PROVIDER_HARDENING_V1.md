# Commerce Provider Hardening v1 Contract

Producers:

- provider-mode `@payment.connect("stripe://...")`
- provider-mode `@shipping.connect("carrier://...")`
- reference runtime provider capture/booking/webhook paths
- `orv build . --prod --out dist`
- `orv verify-build`, `orv deploy-env-check`, generated Compose/env/runbook
  artifacts

Current regression coverage:

- `docs/samples/commerce-provider-hardening-v1.golden.json`
- `docs/samples/commerce-provider-runtime-v1.golden.json`
- `docs/samples/provider-secret-redaction-v1.golden.json`
- `crates/orv-cli/tests/commerce_provider_hardening_contract.rs::commerce_provider_hardening_v1_freezes_deploy_and_env_gate`
- `crates/orv-cli/tests/commerce_provider_hardening_contract.rs::verify_build_rejects_wrong_commerce_provider_package`
- `crates/orv-cli/tests/commerce_provider_hardening_contract.rs::commerce_provider_hardening_v1_retries_with_stable_idempotency_keys`
- `crates/orv-cli/tests/commerce_provider_hardening_contract.rs::commerce_provider_hardening_v1_freezes_previous_secret_webhook_runtime`
- `crates/orv-runtime/src/interp.rs::tests::provider_adapters_retry_transient_endpoint_errors_with_idempotency_keys`
- `crates/orv-runtime/src/interp.rs::tests::stripe_provider_adapter_accepts_previous_webhook_secret_for_rotation`
- `crates/orv-runtime/src/interp.rs::tests::stripe_provider_adapter_rejects_stale_webhook_timestamp`
- `crates/orv-cli/tests/provider_secret_redaction_contract.rs`

This contract freezes the reference hardening boundary for provider-mode
commerce adapters. Provider-mode commerce is a package/template layer over the
generic adapter/secret/idempotency/origin/deploy boundary described in
[Platform Boundary](../PLATFORM_BOUNDARY.md). It does not make Stripe or carrier
SDK integrations production-complete; provider SDK matrices remain M4+ package
work.

The published golden fixture is
`docs/samples/commerce-provider-hardening-v1.golden.json`. It freezes
normalized provider adapter artifacts, provider credential env gates,
deploy/container handoff, Compose/env.example/runbook markers, and
source-origin presence without generated origin ids or secret values.

The runtime golden fixture is
`docs/samples/commerce-provider-runtime-v1.golden.json`. It freezes normalized
provider retry/idempotency-key behavior and previous-secret webhook rotation
metadata without storing configured secret values.

The cross-contract redaction fixture
`docs/samples/provider-secret-redaction-v1.golden.json` also freezes that
commerce provider deploy artifacts and satisfied env-check output omit
configured Stripe/carrier secret values.

## Runtime Boundary

Stripe provider captures use:

```json
{
  "kind": "stripe.payment_intent.create",
  "payload": {}
}
```

Carrier provider bookings use:

```json
{
  "kind": "carrier.shipment.create",
  "payload": {}
}
```

When `STRIPE_API_ENDPOINT` or `CARRIER_API_ENDPOINT` is configured, runtime
calls POST checked JSON to that endpoint. `STRIPE_SECRET_KEY` and
`CARRIER_API_KEY` are sent as bearer credentials and must not appear in stdout,
stderr, generated artifacts, reveal payloads, or env-check output.

Provider HTTP calls retry transient `5xx`, connect, read, and timeout failures
up to three attempts. Retry attempts must reuse the same stable idempotency key:

- `stripe.payment_intent.create:<orderId>`
- `carrier.shipment.create:<orderId>`

The order id is read from the capture/booking payload `orderId` field. Missing
order ids use `unknown`.

## Webhook Boundary

Stripe webhook verification supports Stripe-style `t=...,v1=...` signatures.
The signed payload is `<timestamp>.<payload>`, verified by HMAC-SHA256.

The default timestamp tolerance is 300 seconds and can be overridden by
`STRIPE_WEBHOOK_TOLERANCE_SECONDS`. Stale or future timestamps outside the
tolerance return an invalid webhook status.

Webhook secret rotation is reference-supported by checking both:

- `STRIPE_WEBHOOK_SECRET`
- `STRIPE_WEBHOOK_SECRET_PREVIOUS`

Verification result metadata may report `webhookSecretStatus: "configured"` and
`webhookSecretMatch: "primary" | "previous" | "none"`, but must not expose
secret values.

## Production Artifact Boundary

Provider-mode commerce adapters still use `deploy/commerce-adapters.json`.
Each provider entry exposes:

- `kind`: `payment` or `shipping`
- `surface: "library_provider_package"`
- `package: "orv-commerce"`
- `provider_package`: `orv-stripe` for `stripe`, `orv-carrier` for `carrier`
- `mode: "provider"`
- `provider`: `stripe` or `carrier`
- `env`: source adapter env override name or `null`
- `default`: source fallback provider URL or `null`
- `endpoint: null`
- `record_path: null`
- `provider_env`: env contract entries with `env`, `required`, and `purpose`
- `request.method: "POST"`
- `request.content_type: "application/json"`
- `request.kind`: `payment.capture` or `shipping.booking`
- `source_origin_id` and `source_origin_ids`

Stripe provider env contract:

- `STRIPE_API_ENDPOINT`, optional, `api_endpoint`
- `STRIPE_SECRET_KEY`, required, `api_secret`
- `STRIPE_WEBHOOK_SECRET`, optional, `webhook_signature`
- `STRIPE_WEBHOOK_SECRET_PREVIOUS`, optional,
  `webhook_signature_previous`

Carrier provider env contract:

- `CARRIER_API_ENDPOINT`, optional, `api_endpoint`
- `CARRIER_API_KEY`, required, `api_key`
- `CARRIER_WEBHOOK_SECRET`, optional, `webhook_signature`

Provider URLs are not HTTP commerce endpoints and do not create Compose volumes.

## Deploy Env Gate

Generated Compose and `deploy/env.example` artifacts must expose the provider
env names without values. `deploy/README.md` must list each provider env with
kind, provider, required/optional status, and purpose.

The generated runbook must also include operational hardening notes for secret
manager/vault sourcing, Stripe previous-secret rotation, the default
`STRIPE_WEBHOOK_TOLERANCE_SECONDS` replay window, and idempotency-key based
provider replay review. These notes are generated artifacts, not manual local
edits, so `orv verify-build` rejects stale runbook drift.

`orv deploy-env-check` must fail when required provider credentials are absent
and pass when required credentials are present. Optional endpoint and webhook
envs may remain unset. Env-check diagnostics must name missing env variables
without printing configured secret values.

## Version Policy

Breaking changes to provider request kinds, credential env names, idempotency
key format, retry attempt count, retryable error classes, webhook timestamp
tolerance behavior, previous-secret rotation behavior, `provider_env` artifact
shape, deploy-env required/optional classification, secret redaction, or
provider reveal fields require a contract update, changelog entry, matrix
update, and regression update.
