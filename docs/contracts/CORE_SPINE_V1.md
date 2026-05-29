# Core Spine v1 Contract

Core Spine v1 freezes the minimal source-to-runtime chain that must stay
consistent across M0-M3 tooling:

```text
ProjectGraph source node -> OriginMap entry -> runtime HTTP event -> editor trace reveal
```

Covered producers and consumers:

- `orv origins <file>`
- `orv graph <file>`
- `orv build <file>`
- DAP in-process attached runtime request trace
- `orv editor trace <build-dir> --trace <trace.json>`

Regression coverage:

- `docs/samples/core-spine-v1.golden.json`
- `crates/orv-cli/tests/core_spine_contract.rs`

## Frozen Chain

The published golden fixture uses a single route:

```orv
@server { @listen 0 @route GET /ping { @respond 200 { ok: true } } }
```

The regression proves all of the following in one black-box flow:

- `orv origins` emits the route and response OriginMap entries.
- `orv graph` embeds the same OriginMap and links those origin ids back to
  ProjectGraph source nodes.
- build `origin-map.json` and `project-graph.json` preserve the same origin ids
  and graph embedding.
- a real in-process runtime request returns `x-orv-origin-id` and
  `x-orv-response-origin-id` matching the route and response entries.
- the runtime `orv.production.trace` frame carries the same route and response
  origin ids.
- `orv editor trace` resolves those ids back to route and response source
  navigation.

## Version Policy

Core Spine v1 is an integration contract, not a replacement for the individual
ProjectGraph, OriginMap, Runtime Trace, Route Origin Headers, or Editor Trace
contracts. Breaking the chain requires updating this file, the golden fixture,
the regression, and the narrower affected contract.
