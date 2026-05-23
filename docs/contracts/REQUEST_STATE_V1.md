# Request State v1

This contract freezes the reference HTTP request-state values that route
handlers can read through the runtime domains.

It covers:

- route path parameters through `@param.<name>`
- percent-decoded query parameters through `@query.<name>`
- request headers through `@header["name"]` and dotted header access where
  valid
- JSON and `application/x-www-form-urlencoded` request body parsing through
  `@body`
- raw request body preservation through `@request.rawBody`

It does not freeze the full HTTP server lifecycle, middleware ordering,
streaming bodies, multipart parsing, or typed validation failure payloads. Typed
`@body: T`, `@query: T`, and `@form: T` failures are covered by Validation Error
Response v1.

## Route Handler Values

Given:

```orv
@route POST /users/:id {
  @respond 201 {
    id: @param.id,
    q: @query.q,
    auth: @header["x-client-auth"],
    name: @body.name,
    raw: @request.rawBody
  }
}
```

A request to:

```text
POST /users/u-42?q=hello+world%20%EC%95%88%EB%85%95
x-client-auth: token-123

{"name":"Ada"}
```

exposes:

```json
{
  "id": "u-42",
  "q": "hello world 안녕",
  "auth": "token-123",
  "name": "Ada",
  "raw": "{\"name\":\"Ada\"}"
}
```

Rules:

- path parameters are strings captured from matching `:name` route segments;
- query parameter names and values are URL-decoded before handler evaluation;
- `+` in query values decodes as a space;
- JSON request bodies expose JSON objects, arrays, numbers, booleans, strings,
  and null through `@body`;
- form-urlencoded request bodies expose decoded field objects through `@body`;
- `@request.rawBody` is the byte body decoded as UTF-8 text before parsing;
- response JSON serialization preserves the runtime value types.

## Version Policy

- Changing path/query/header/body/raw-body domain names requires a new contract
  file and migration note.
- Changing query decoding, JSON body typing, or raw-body preservation requires a
  new contract file and migration note.
- New request-state domains may be added without changing this contract if the
  existing keys and behavior remain stable.

## Regression Coverage

- `crates/orv-runtime/src/server/tests.rs::request_state_v1_contract_covers_param_query_header_body_and_raw_body`
  is a reference HTTP runtime regression. It starts the runtime server, sends one
  HTTP request, and verifies `@param`, decoded `@query`, `@header`, parsed JSON
  `@body`, and `@request.rawBody` in the route response.
