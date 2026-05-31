# Compiler Plugin Boundary v1 Contract

Producer:

- `orv_hir::domain_boundary_descriptor`
- `orv_hir::domain_plugin_registry`
- `orv_hir::origin_call_boundary_descriptor`
- build/deploy/reveal code that consumes domain boundary descriptors

Current regression coverage:

- `docs/samples/compiler-plugin-boundary-v1.golden.json`
- `crates/orv-cli/tests/compiler_plugin_boundary_contract.rs::compiler_plugin_boundary_v1_freezes_domain_descriptor_inventory`
- `crates/orv-hir/src/domain_boundary.rs` unit tests for local helper behavior
- commerce adapter and provider hardening contract regressions that consume the
  same descriptors for `@payment` and `@shipping`
- shop security boundary regressions that consume first-party compiler plugin
  policy surfaces

This contract freezes the minimal platform-boundary descriptor vocabulary used
while full compiler plugin hooks remain planned. The registry scaffold groups
current first-party and library/provider-owned domains by package, and
descriptors include generic capability, effect, and hook metadata so downstream
artifacts can reason about the class of compiler/runtime affordance without
knowing provider names. It does not make web, data, security, design, jobs,
payment, shipping, Stripe, carrier, or shop checkout compiler core intrinsics.
It only freezes how the current scaffold labels those surfaces so artifacts and
reveal payloads cannot silently drift.

The published golden fixture is
`docs/samples/compiler-plugin-boundary-v1.golden.json`.

## Descriptor Root

The normalized contract inventory has exactly:

```json
{
  "schema_version": 1,
  "kind": "orv.compiler_plugin_boundary.v1",
  "plugin_registry": [],
  "domain_descriptors": [],
  "origin_call_descriptors": []
}
```

Rules:

- `schema_version` is currently `1`.
- `kind` is `orv.compiler_plugin_boundary.v1`.
- Arrays preserve producer order in the published regression fixture.
- New public surface spellings, owner package names, or descriptor keys require
  this document, the golden fixture, and changelog to change together.

## Plugin Registry

Each `plugin_registry[]` entry has exactly:

```json
{
  "surface": "first_party_compiler_plugin",
  "owner_package": "orv-web",
  "domains": ["body", "form", "header", "html", "listen", "param", "query", "request", "respond", "route", "serve", "server"],
  "capabilities": ["http.route", "http.request", "http.response", "html.render"],
  "effects": ["network.listen", "http.respond"],
  "hooks": ["type.check", "hir.lower", "origin.emit", "artifact.emit"]
}
```

Rules:

- `plugin_registry` is an owned-domain registry scaffold, not a dynamic plugin
  loader.
- Each registered domain must appear in exactly one registry entry.
- Every registered domain descriptor must match its registry entry's surface,
  owner package, capabilities, effects, and hooks.
- Unknown extension domains are intentionally absent from the registry and
  resolve to `surface: "extension"` with owner `extension`.
- Provider-named domains such as `Stripe` and `carrier` must stay absent from
  the registry unless a future extension/package contract explicitly registers
  them.

## Domain Descriptors

Each `domain_descriptors[]` entry has exactly:

```json
{
  "domain": "server",
  "surface": "first_party_compiler_plugin",
  "owner_package": "orv-web",
  "capabilities": ["http.route", "http.request", "http.response", "html.render"],
  "effects": ["network.listen", "http.respond"],
  "hooks": ["type.check", "hir.lower", "origin.emit", "artifact.emit"]
}
```

Frozen surface spellings:

- `core_intrinsic`
- `first_party_compiler_plugin`
- `library_provider_package`
- `extension`

Frozen MVP descriptor ownership:

- `out` is `core_intrinsic` owned by `orv-core`.
- Web domains such as `server`, `route`, and `html` are
  `first_party_compiler_plugin` owned by `orv-web`.
- `db` is `first_party_compiler_plugin` owned by `orv-data`.
- `Auth`, `session`, `csrf`, and `rateLimit` are
  `first_party_compiler_plugin` owned by `orv-security`.
- `design` is `first_party_compiler_plugin` owned by `orv-design`.
- `cron` is `first_party_compiler_plugin` owned by `orv-jobs`.
- `payment` and `shipping` are `library_provider_package` owned by
  `orv-commerce`.
- Unknown domains are `extension` owned by `extension`.

Frozen metadata rules:

- `capabilities` are generic compiler/runtime capability labels, not
  provider-specific feature names.
- `effects` are generic side-effect labels used for analysis and deploy/reveal
  handoff.
- `hooks` are the compiler phases the surface may contribute to; these labels
  are a scaffold, not the complete plugin loading ABI.
- No descriptor metadata may encode provider-specific names such as Stripe,
  carrier products, or shop checkout workflow state.
- Unknown extension domains keep empty metadata arrays until a registered
  extension contract supplies its own descriptor.

## Origin-Call Descriptors

Each valid `origin_call_descriptors[]` entry has exactly:

```json
{
  "call": "@payment.capture",
  "domain": "payment",
  "method": "capture",
  "surface": "library_provider_package",
  "owner_package": "orv-commerce",
  "capabilities": ["adapter.bridge", "secret.env", "idempotency.key", "webhook.verify"],
  "effects": ["external.call", "secret.read"],
  "hooks": ["type.check", "hir.lower", "origin.emit", "artifact.emit"]
}
```

Malformed display names keep the same keys with `null` values for `domain`,
`method`, `surface`, `owner_package`, `capabilities`, `effects`, and `hooks`.

Frozen origin-call ownership rules:

- `@payment.capture`, `@payment.connect`, `@shipping.book`, and
  `@shipping.connect` remain `library_provider_package` owned by
  `orv-commerce`.
- Provider-named connect calls such as `@Stripe.connect` and
  `@carrier.connect` are `extension` owned by `extension`; they must not claim
  `core_intrinsic` or `first_party_compiler_plugin`.

## Non-Goals

This v1 contract is a boundary scaffold. It does not freeze a dynamic plugin
loading protocol, out-of-core lowering ABI, sandbox model, permission
negotiation model, provider SDK contract, or third-party package resolution
story. Those remain future contract work.
