# Compiler Plugin Boundary v1 Contract

Producer:

- `orv_hir::domain_boundary_descriptor`
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
while full compiler plugin hooks remain planned. It does not make web, data,
security, design, jobs, payment, shipping, Stripe, carrier, or shop checkout
compiler core intrinsics. It only freezes how the current scaffold labels those
surfaces so artifacts and reveal payloads cannot silently drift.

The published golden fixture is
`docs/samples/compiler-plugin-boundary-v1.golden.json`.

## Descriptor Root

The normalized contract inventory has exactly:

```json
{
  "schema_version": 1,
  "kind": "orv.compiler_plugin_boundary.v1",
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

## Domain Descriptors

Each `domain_descriptors[]` entry has exactly:

```json
{
  "domain": "server",
  "surface": "first_party_compiler_plugin",
  "owner_package": "orv-web"
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

## Origin-Call Descriptors

Each valid `origin_call_descriptors[]` entry has exactly:

```json
{
  "call": "@payment.capture",
  "domain": "payment",
  "method": "capture",
  "surface": "library_provider_package",
  "owner_package": "orv-commerce"
}
```

Malformed display names keep the same keys with `null` values for `domain`,
`method`, `surface`, and `owner_package`.

## Non-Goals

This v1 contract is a boundary scaffold. It does not freeze a complete plugin
registry, plugin loading protocol, out-of-core lowering ABI, sandbox model, or
third-party package resolution story. Those remain future contract work.
