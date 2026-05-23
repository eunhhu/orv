# Request Bindings v1

This contract freezes declarative request binding behavior for the reference
HTTP runtime.

It covers:

- `@query: T`
- `@body: T`
- `@form: T`
- successful normalization back into `@query`, `@body`, and `@form`
- validation failure response handoff to Validation Error Response v1

It does not freeze every validator constraint, multipart forms, streaming
bodies, custom coercion hooks, or production-native lowering coverage. Those
remain implementation-level or covered by narrower contracts.

## Success

Route-level request bindings parse the current request value using the named
schema or type:

```orv
struct SearchQuery {
  page: int(min=1)
  q: string(trim, lower, min=1)
}

@route GET /search {
  @query: SearchQuery
  @respond 200 { page: @query.page, q: @query.q }
}
```

For:

```text
GET /search?page=2&q=%20HELLO%20
```

the handler observes normalized values:

```json
{
  "page": 2,
  "q": "hello"
}
```

Rules:

- `@query: T` validates the decoded query object;
- `@body: T` validates the parsed JSON body object;
- `@form: T` validates the parsed `application/x-www-form-urlencoded` object;
- successful bindings replace the corresponding runtime request-state object
  with normalized typed values before the rest of the route body runs;
- integer fields that pass validation are visible as JSON numbers in responses;
- string transforms such as `trim` and `lower` are visible in subsequent
  `@query`, `@body`, or `@form` reads.

## Failure

If validation fails, the remaining route body does not run. The runtime returns
HTTP `400` with a Validation Error Response v1 payload:

```json
{
  "schema_version": 1,
  "kind": "orv.validation.error",
  "error": "validation_failed",
  "fields": []
}
```

The exact field error shape and ordering are owned by
[Validation Error Response v1](VALIDATION_ERROR_RESPONSE_V1.md).

## Version Policy

- Changing `@query: T`, `@body: T`, or `@form: T` binding names requires a new
  contract file and migration note.
- Changing successful normalization visibility through `@query`, `@body`, or
  `@form` requires a new contract file and migration note.
- Changing validation failure payload shape requires a Validation Error Response
  contract update.
- Adding new request binding domains is backward-compatible if these bindings
  remain stable.

## Regression Coverage

- `crates/orv-runtime/src/server/tests.rs::declarative_request_bindings_validate_body_query_and_form`
  starts the reference HTTP runtime and verifies `@query: T`, `@body: T`, and
  `@form: T` success normalization plus Validation Error Response v1 failure
  envelopes.
