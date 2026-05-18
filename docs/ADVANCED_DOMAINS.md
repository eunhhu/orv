# Advanced Domain Boundary

This document is the promotion gate for advanced domains. The current MVP does
not expand because a domain exists in `SPEC.md` or a pressure fixture.

## Non-Binding Set

These stay outside the MVP unless promoted by the rule below:

- CRDT and `@sync`
- `@gpu`
- `@media`
- raw `@net` and transport domains
- `@plugin`
- broad FFI and `@unsafe` ecosystem work
- custom storage engines beyond the current reference DB path
- provider SDK matrices beyond checked reference adapters
- first-party native editor UI beyond current artifact/native-host contracts

## Fixture Contract

Pressure fixtures must declare their contract at the top of the file:

```text
status: reference-only | planned-only
contract: reference | non-binding
milestone: M4+
purpose: syntax pressure | design pressure | reference runtime
```

`fixtures/default-syntax.orv` and `fixtures/plan/*.orv` are design pressure by
default. `fixtures/e2e/domains.orv` is a reference runtime fixture, not a
production surface claim.

## Promotion Rule

An advanced domain moves toward MVP only when the change directly improves
`BENCHMARK_SHOP_5H.md` or closes a documented production safety gap in
`SECURITY_MODEL.md`.

Promotion requires all of this in the same work item:

- update `IMPLEMENTATION_MATRIX.md` from `non-binding` or `planned`
- add or narrow a fixture that exercises the promoted path
- add focused regression coverage for the new contract
- update `MVP.md` only if the promoted path becomes an MVP acceptance condition

If those conditions are not met, the domain remains roadmap/reference material
even when parser, analyzer, or reference runtime stubs exist.
