# Shop Checkout Resilience v1 Contract

Producers:

- `orv init <dir> --template shop`
- `fixtures/e2e/shopping_mall.orv`
- reference runtime `POST /checkout`
- payment and shipping commerce adapters
- SQLite/file-backed shop persistence

Current regression coverage:

- `docs/samples/shop-checkout-resilience-v1.golden.json`
- `crates/orv-runtime/src/server/tests.rs::fixture_shopping_mall_records_checkout_compensation_when_shipping_fails`
- `crates/orv-cli/tests/shop_security_boundary_contract.rs::shop_template_keeps_checkout_and_webhook_side_effect_boundaries_ordered`
- `crates/orv-cli/tests/shop_template_contract.rs`

This contract freezes the reference checkout resilience boundary for the shop
MVP. It does not claim full provider-grade compensation workflows; provider SDK
matrices and operator runbooks remain later hardening work.

This is a shop benchmark/library contract, not compiler core semantics.
Compiler-visible reuse is limited to transaction, adapter call, idempotency,
secret/redaction, origin/reveal, and deploy metadata as described in
[Platform Boundary](../PLATFORM_BOUNDARY.md).

## Checkout Order

`POST /checkout` must preserve this side-effect order:

```text
member/product lookup
-> stock guard
-> shopdb.transaction(stock decrement, reserved order create)
-> payment capture
-> payment row create
-> order status paid
-> shipment booking and shipment row create
-> shipped order + checkout.complete audit
```

The stock decrement and initial order creation are the transaction boundary.
Library/provider calls occur after that boundary so a payment capture cannot happen
before the reserved order exists.

## Shipment Failure Boundary

If payment capture succeeds but shipment booking or shipment persistence fails,
`POST /checkout` must:

- return HTTP 202
- update the order status to `payment_captured_pending_shipment`
- keep the payment row with captured status
- leave shipment as `null`/absent
- record an `AuditEvent` with `kind: "checkout.compensation_required"`
- include `compensation.required: true` in the response
- not record `checkout.complete`

The reserved stock decrement remains applied. The pending order is visible to
operator/admin read models and persisted SQLite state.

The published golden normalizes this runtime path into stable public evidence:
HTTP status, response shape, provider retry/idempotency markers, persisted
order/payment/shipment/audit counts, and payment-record secret redaction.

## Adapter Failure Boundary

Provider package or HTTP shipping adapter failures after payment capture are
part of the same compensation path. Provider-mode carrier calls must keep stable
idempotency keys across retries:

```text
carrier.shipment.create:<orderId>
```

Provider secrets may be sent to the adapter endpoint as bearer credentials but
must not appear in checkout responses, persisted records, generated artifacts,
or env-check output.

## Version Policy

Breaking changes to checkout side-effect order, transaction placement, shipment
failure HTTP status, pending order status string, compensation response shape,
audit event kind, shipment absence behavior, provider retry/idempotency coupling,
or secret redaction expectations require a contract update, changelog entry,
matrix update, and regression update.
