# Shop Template v1 Contract

Producer:

- `orv init <dir> --template shop`
- `orv init <dir> --name <name> --template shop`

The published golden fixture is `docs/samples/shop-template-v1.golden.json`. It
freezes generated file presence, manifest markers, source scaffold markers,
security/checkout/webhook counts, README handoff markers, and `orv check .`
success.

Current regression coverage:

- `docs/samples/shop-template-v1.golden.json`
- `crates/orv-cli/tests/shop_template_contract.rs::shop_template_v1_freezes_scaffold_contract`
- `crates/orv-cli/tests/shop_acceptance_contract.rs::shop_acceptance_artifacts_expose_human_pass_gate_and_failure_classification`
- `crates/orv-cli/src/tests.rs::init_shop_template_*`

This contract freezes the generated starter project surface: manifest, source
scaffold, README/operator handoff, and parseability through `orv check .`.
Template-to-running-shop live smoke and recorded human 5-hour benchmark evidence
are separate acceptance layers.

The shop template is a first-party benchmark/template surface. It does not make
cart/order/checkout/payment/shipping, Stripe-style webhooks, or carrier booking
compiler core intrinsics. Those features are interpreted through the library/provider
boundary described in [Platform Boundary](../PLATFORM_BOUNDARY.md).

## Generated Files

The shop template writes:

- `orv.toml`
- `src/main.orv`
- `README.md`

`orv.toml` must contain a `[project]` table with:

- `name` from `--name`, or the target directory name when `--name` is omitted
- `version = "0.1.0"`
- `entry = "src/main.orv"`

`src/main.orv` must pass `orv check .` from the generated project root.

## Source Scaffold

`src/main.orv` is the reference shopping starter. Its public template/library
surface includes:

- `@listen 8080`
- a SQLite-defaulted `SHOP_DATABASE_URL` DB adapter:
  `@db.connect(@env.SHOP_DATABASE_URL ?? "sqlite://data/shop.sqlite")`
- editable `@design` tokens using `@colors`, `@spacing`, and `@typography`
- home shell use of `@design.colors.surface`, `@design.spacing.lg`, and
  `@design.typography.fontFamily`
- `ProductInput.badge` through form input, `POST /products`, persisted rows,
  customer catalog, admin catalog, and generated smoke checks
- browser routes for `/`, `/catalog`, `/cart`, and `/account/sessions`
- admin read-model routes for `/admin`, `/admin/summary`, `/admin/catalog`,
  `/admin/orders`, `/admin/payments`, `/admin/shipments`, `/admin/webhooks`,
  and `/admin/audit`
- `@Auth required role="admin"` on admin read-model routes
- `@session required` on the account session route
- CSRF hidden-token forms and `@csrf` on browser mutation routes
- typed `@body: ...Input` bindings for product/member/login/cart/order/checkout/
  payment/shipment mutation routes
- password hashing with `hash.password` and login verification with
  `hash.verify`
- checkout stock reservation and order creation in a `shopdb.transaction(...)`
  boundary before library/provider capture/booking
- local-file-defaulted payment and shipping adapters:
  `@payment.connect(@env.PAYMENT_ADAPTER_URL ?? "file://data/payments.jsonl")`
  and
  `@shipping.connect(@env.SHIPPING_ADAPTER_URL ?? "file://data/shipments.jsonl")`
  as commerce reference package handles, not compiler core intrinsics
- shipment-failure compensation path that updates orders to
  `payment_captured_pending_shipment` and records
  `checkout.compensation_required`
- Stripe-style webhook endpoint at `POST /webhooks/stripe`, using
  `stripe-signature`, `payments.verifyWebhook`, duplicate event handling, and
  audit/event persistence

## README Handoff

`README.md` must document the starter commands:

- `orv check .`
- `orv build . --prod --out dist`
- `orv verify-build dist`
- `orv deploy-env-check dist`
- `orv run-build dist`
- `sh dist/deploy/smoke-test.sh`
- `orv benchmark-prepare dist --participants 2`
- `orv benchmark-report dist`
- `orv benchmark-report dist --require-pass`

It must also name the core starter surfaces: `ProductInput.badge`, admin auth,
session cookies, CSRF, password hashing, generated smoke output, native server
artifacts, admin audit route, and Stripe-style provider webhook route.

## Acceptance Handoff

The generated project is expected to feed the later acceptance path:

```text
orv init --template shop
-> orv check .
-> orv build . --prod --out dist
-> orv verify-build dist
-> orv deploy-env-check dist
-> orv run-build dist
-> sh dist/deploy/smoke-test.sh
-> orv benchmark-prepare dist --participants 2
-> orv benchmark-report dist
```

Shop Template v1 only freezes the starter project and its handoff contract.
Passing the generated smoke script and collecting human benchmark evidence are
tracked by the template-to-running-shop and benchmark contracts.

## Version Policy

Breaking changes to generated file names, manifest keys, required source
scaffold markers, README command handoff, or the `orv check .` parseability gate
require a contract update, changelog entry, matrix update, and regression update.
