# Shop Security Boundaries v1 Contract

Producers:

- `orv init <dir> --template shop`
- `orv build . --prod --out dist` for a shop-template project
- reference `orv run` / `orv run-build dist` server execution

Current regression coverage:

- `crates/orv-cli/tests/shop_template_contract.rs::shop_template_v1_freezes_scaffold_contract`
- `crates/orv-cli/tests/shop_security_boundary_contract.rs::shop_template_keeps_checkout_and_webhook_side_effect_boundaries_ordered`
- `crates/orv-cli/src/tests.rs::init_shop_template_*`
- `crates/orv-runtime/src/server/tests.rs::checkout_route_has_reference_rate_limit`
- `crates/orv-runtime/src/server/tests.rs::session_required_route_checks_reference_session_cookie`
- `crates/orv-runtime/src/server/tests.rs::csrf_route_checks_reference_cookie_and_token`
- generated shop runtime-flow regressions in `crates/orv-runtime/src/server/tests.rs`

The black-box contract regression freezes source ordering plus generated
production artifact exposure. This contract freezes the reference shop security
boundary that the generated starter, production artifacts, and reference runtime
must expose. It is a reference/scaffold contract, not a claim that production
identity providers, payment SDKs, carrier SDKs, or vault-backed secret rotation
are complete.

## Scope

The v1 boundary covers:

- session cookie issuance and `@session required` route gates
- admin role gates through `@Auth required role="admin"`
- browser mutation CSRF checks through `@csrf`
- default route rate-limit policy exposure and enforcement
- checkout transaction, provider-call, and compensation ordering
- Stripe-style webhook signature verification, duplicate handling, and audit
  ordering
- generated build/server/deploy/native policy artifact exposure

It intentionally leaves these production hardening areas outside v1:

- production identity provider integration
- password reset, email verification, and account recovery flows
- provider SDK-specific Stripe/carrier adapters
- vault-backed secret rotation and operational runbooks
- authorization policy models beyond the reference admin role gate

## Source Boundary

The generated `src/main.orv` must keep these source-level security markers:

- account session read models use `@session required`
- admin read models use `@Auth required role="admin"`
- browser mutation forms include `_csrf` hidden fields
- browser mutation routes use `@csrf`
- checkout, login, and webhook hotspots carry rate-limit policy through the
  reference defaults or explicit `@rateLimit` descriptors
- `POST /webhooks/stripe` verifies `stripe-signature` through
  `payments.verifyWebhook` before duplicate lookup, event creation, or audit
  persistence

The checkout source must preserve this side-effect order:

```text
stock guard
-> shopdb.transaction(...)
-> Product stock update
-> Order create
-> payment capture
-> shipping booking
-> shipment-failure pending/compensation path or checkout.complete audit
```

The webhook source must preserve this side-effect order:

```text
payments.verifyWebhook
-> WebhookEvent duplicate lookup
-> duplicate audit branch
-> WebhookEvent create
-> webhook.received audit
```

## Runtime Boundary

The reference runtime must expose these observable results:

- successful login emits `orv_session` and, when applicable,
  `orv_session_role` cookies
- session cookies use `Path=/`, `Max-Age=86400`, `HttpOnly`, `SameSite=Lax`,
  and `Secure`
- HTML GET routes that render browser mutation forms mint
  `orv_csrf=orv-reference-csrf` with `Path=/`, `Max-Age=86400`, and
  `SameSite=Lax`
- missing `@session required` cookie returns HTTP 401 with
  `{"err":"session_required"}`
- missing `@Auth required role="admin"` cookie returns HTTP 401 with
  `{"err":"auth_required"}`
- wrong admin role returns HTTP 403 with `{"err":"role_required"}`
- missing or invalid `@csrf` token returns HTTP 403 with
  `{"err":"csrf_token_required"}`
- exceeded route rate limits return HTTP 429 and mention that the route rate
  limit was exceeded
- valid shop checkout returns a shipped order with captured payment and local
  shipment tracking in the reference adapter path
- shipment failure after payment capture leaves the order in
  `payment_captured_pending_shipment` and records
  `checkout.compensation_required`
- valid Stripe-style webhook signatures create verified webhook records, and
  replayed event ids are handled as duplicates

## Artifact Boundary

Production build artifacts for the shop template must expose the same boundary
to downstream tooling:

- runtime feature lists include `auth_roles`, `session_cookies`,
  `csrf_protection`, and `rate_limit`
- route policy descriptors include `auth`, `session`, `csrf`, and
  `rate_limit` where those gates apply
- policy descriptors carry source-backed `origin_id` values where the policy is
  source-authored
- generated deploy preflight and native route tables mirror the policy
  descriptors used by the reference server
- generated smoke tests exercise CSRF/session/admin cookies, checkout response
  markers, webhook/admin read models, and audit markers

## Version Policy

Breaking changes to cookie names, reference rejection error names, CSRF token
handoff, checkout/webhook side-effect ordering, route-policy artifact keys, or
runtime feature names require a contract update, changelog entry, matrix update,
and regression update.
