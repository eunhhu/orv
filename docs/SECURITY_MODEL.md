# orv Security Model

Security is a default scaffold behavior, not an optional library checklist. The App Authoring surface should let a beginner build a shop without manually handling bearer token slicing, cookie flags, CSRF details, webhook replay logic, or provider idempotency.

Security primitives are compiler/platform concepts. Provider-specific payment, shipping, Stripe, carrier, or shop checkout semantics are library/template concerns. This boundary follows [PLATFORM_BOUNDARY.md](PLATFORM_BOUNDARY.md).

## Safe Defaults

| Area | Default expectation |
|------|---------------------|
| Sessions | HttpOnly, Secure in production, SameSite=Lax or Strict, rotation after login |
| Passwords | `hash.password` with approved parameters; no plaintext storage |
| CSRF | state-changing browser routes require CSRF token unless explicitly exempted |
| XSS | HTML text escapes by default; raw HTML requires an explicit unsafe escape hatch |
| Authz | admin routes require declarative role/policy checks |
| Rate limits | auth, checkout, webhook, and password reset routes get scaffolded limits |
| Secrets | `vault.get`/env contracts never expose values in runtime responses or artifacts |
| Webhooks | generic signature verification metadata, 300-second default timestamp tolerance, replay/idempotency key storage |
| External adapters | stable idempotency keys per external capability attempt |
| Audit | login, checkout, payment, shipping, admin mutation, and webhook events logged |
| Errors | route errors become safe 4xx/5xx responses without leaking secrets |

## App Authoring Surface

Beginner-facing code should prefer declarative security domains:

```orv
@route POST /checkout {
  @session required
  @csrf
  @rateLimit key=@session.userId limit=10 window="1m"
  @CheckoutPolicy
  @body: CheckoutForm

  // `@checkout` is a library/template surface, not compiler core intrinsic.
  @checkout.capture
}

@route GET /admin/orders {
  @Auth required role="admin"
  @respond 200 await @db.find Order
}

@route POST /webhooks/provider {
  @csrf exempt
  @respond 200 { ok: true }
}
```

Lower-level primitives such as `jwt.verify`, `hash.password`, `crypto.hmac`, and `vault.get` remain available for Systems Surface code, but scaffolds should not force beginners to wire them by hand. Provider packages should expose their security requirements through generic secret/env/idempotency/replay metadata instead of requiring compiler changes.

## Shop Scaffold Requirements

The shop template should provide:

- protected admin routes
- member session cookie defaults
- signup/login password hashing
- checkout CSRF and rate limit hooks
- external adapter idempotency keys
- provider-package webhook signature/replay protection in provider mode
- audit records for checkout, payment, shipping, and admin mutations
- deploy env checks for required provider secrets

## Current XSS Contract

HTML Render v1 now freezes the safe default escaping boundary: text children
escape `&`, `<`, and `>`, while quoted attribute values additionally escape
`"` and `'`. Raw HTML injection remains outside the MVP safe path until it has
an explicit unsafe escape hatch and review contract.

## Implementation Tracking

This file defines security expectations. Exact implementation/contract status is tracked in [IMPLEMENTATION_MATRIX.md](IMPLEMENTATION_MATRIX.md).
