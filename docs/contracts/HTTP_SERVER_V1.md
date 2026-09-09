# HTTP Server v1

This contract freezes the minimal reference HTTP server route-dispatch envelope.

It covers:

- `@server` with `@listen`
- HTTP/1.1 request dispatch to `@route METHOD /path`
- `@respond <status> <object>` JSON responses
- default unmatched-route response behavior

It does not freeze long-running process supervision, TLS, HTTP/2, HTTP/3,
WebSocket/WebTransport/WebRTC, middleware ordering, static file serving,
request-state domains, validation bindings, trace payloads, or route origin
headers. Those are covered by narrower contracts or remain implementation-level.

## Listen Address

`@listen` sets the port. `ORV_HOST` optionally sets the bind IP address and
accepts an IPv4 or IPv6 literal. With no override, direct `orv run` and
`orv run-build` execution listens on `127.0.0.1`. Invalid values fail before the
listener starts. Generated container images and Compose environments default
to `ORV_HOST=0.0.0.0` so published ports can reach the listener.

`scripts/container_smoke.sh` builds and runs the generated reference Dockerfile
and checks a route through a host-published port.

## JSON Route Response

The published golden fixture is `docs/samples/http-server-v1.golden.json`.

Given:

```orv
@server {
  @listen 0
  @route GET /ping {
    @respond 200 { ok: true, msg: "pong" }
  }
}
```

`GET /ping` returns:

```text
HTTP/1.1 200
content-type: application/json
```

with body:

```json
{
  "ok": true,
  "msg": "pong"
}
```

Rules:

- route method and path must both match;
- object response payloads serialize as JSON;
- JSON response content type is `application/json`;
- runtime JSON serialization preserves object field values and primitive types.

## Unmatched Routes

When no route matches and no catch-all route handles the request, the reference
server returns:

```text
HTTP/1.1 404
content-type: text/plain; charset=utf-8

Not Found
```

Catch-all routes such as `@route GET *` may override the default 404 behavior;
that catch-all policy is outside this minimal contract.

## Version Policy

- Changing default unmatched-route status or body behavior requires a new
  contract file and migration note.
- Changing JSON response content type or object serialization requires a new
  contract file and migration note.
- Adding new route matching features is backward-compatible if this minimal
  dispatch behavior remains stable.

## Regression Coverage

- `docs/samples/http-server-v1.golden.json`
- `crates/orv-runtime/src/server/tests.rs::http_server_v1_contract_covers_json_route_and_default_404`
  starts the reference HTTP runtime, verifies JSON route response status,
  content type, payload, default unmatched-route 404 behavior, and compares the
  response envelope against the published golden fixture.
