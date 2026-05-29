# Validation Error Response v1 Contract

Producer:

- reference HTTP request bindings: `@body: T`, `@query: T`, `@form: T`
- runtime struct validation surfaced as route validation failure responses

Current regression coverage:

- `crates/orv-runtime/tests/validation_response_contract.rs::validation_error_response_contract_freezes_public_object_keys_and_types`
- `crates/orv-runtime/tests/validation_response_contract.rs::validation_error_response_contract_preserves_multi_error_order_and_null_actuals`
- `crates/orv-runtime/tests/validation_response_contract.rs::validation_error_response_contract_distinguishes_constraint_mismatch`
- `crates/orv-runtime/tests/validation_response_contract.rs::validation_error_response_contract_covers_query_and_form_binding_producers`
- request binding runtime tests in `crates/orv-runtime/src/server/tests.rs`

## HTTP Response

Validation failures return HTTP `400` with a JSON payload that has exactly:

```json
{
  "schema_version": 1,
  "kind": "orv.validation.error",
  "error": "validation_failed",
  "fields": []
}
```

Rules:

- `schema_version` is currently `1`.
- `kind` is exactly `orv.validation.error`.
- `error` is exactly `validation_failed`.
- `fields[]` preserves validation reporting order.

## Field Error

Each `fields[]` item has exactly:

```json
{
  "path": "$.quantity",
  "code": "type_mismatch",
  "message": "constraint mismatch: 0 does not satisfy `min=1`",
  "expected": "int(min=1)",
  "actual": "0"
}
```

Rules:

- `path` is a JSONPath-like field path rooted at `$`.
- `code` is currently one of `missing_required`, `unknown_property`,
  `type_mismatch`, or `constraint_mismatch`.
- `message` is a human-readable diagnostic string, not a stable machine key.
- `expected` is the expected type/constraint display string.
- `actual` is the request value converted to JSON. Missing required values use
  `null`.

## Ordering

Struct field errors are reported in schema declaration order. Unknown input
properties are reported after known struct fields, in request object order.

## Version Policy

Validation Error Response v1 is the public HTTP validation error schema. Breaking
key/type changes require a schema version bump and updates to SPEC, runtime
contract tests, and this file.
