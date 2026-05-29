# Compiler Pipeline v1 Contract

Compiler Pipeline v1 freezes the public, observable behavior of the M0
load/resolve/analyze/lower pipeline. It does not expose the internal resolver
scope map or HIR data structures as public JSON. Instead, it fixes the behavior
that downstream users observe through CLI commands and origin artifacts.

The published golden fixture is
`docs/samples/compiler-pipeline-v1.golden.json`. It normalizes local entry paths
while freezing the successful check/run/origin-call-edge inventory and the
resolver/analyzer failure classes.

## Producers

- `orv check <entry>`
- `orv run <entry>`
- `orv origins <entry>`
- `orv build <entry> --out <dir>`

## Scope

The pipeline runs in this order:

1. project load/import merge
2. lex/parse to AST
3. name resolution
4. semantic analysis and HIR lowering
5. origin-map/build/runtime consumption of lowered HIR

Project loading, check CLI envelopes, runtime CLI envelopes, build artifact
roots, ProjectGraph, and OriginMap have their own contract files. This contract
ties the resolver and HIR analysis invariants between those surfaces.

## Name Resolution

Resolution must support:

- hoisted function declarations, including forward calls;
- lexical block shadowing;
- function parameter bindings;
- loop and catch bindings scoped to their bodies;
- imported names after project merge;
- built-in names without user declarations.

References outside their lexical scope fail before runtime. The public failure
surface is `orv check` diagnostics on stderr. The diagnostic must include the
undefined identifier and a source snippet for the offending reference.

## Semantic Analysis And HIR Lowering

HIR lowering must preserve source spans for executable origins and feed those
origins into OriginMap v2.

The analysis pass must reject these cases before runtime:

- obvious annotation mismatches such as assigning a string literal to `int`;
- function call arity mismatches;
- function call argument type mismatches when parameter types are known;
- return type mismatches when a function annotation is known.

The public failure surface is the shared `orv check` diagnostic envelope.

## Runtime Observable Binding

A resolved program with shadowing must execute using the selected lexical
binding. For example:

```orv
function twice(x: int): int -> add(x, x)
function add(a: int, b: int): int -> a + b
let x: int = 4
{ let x: int = 10
@out twice(x) }
@out twice(x)
```

`orv run` prints:

```text
20
8
```

## Origin Observable Binding

Calls that resolve to user functions must emit OriginMap v2 `calls` edges from
the call origin to the function origin. The origin IDs may change with spans,
but the edge relationship must remain present.

## Version Policy

Compiler Pipeline v1 is a public behavioral contract. Breaking the observable
binding behavior, moving resolver/analyzer failures out of `orv check`, or
removing HIR-derived function call origin edges requires a new contract version
or an explicit compatibility bridge.

## Regression Coverage

- `docs/samples/compiler-pipeline-v1.golden.json`
- `crates/orv-cli/tests/compiler_pipeline_contract.rs` is a CLI black-box
  regression. It runs the built `orv` binary and compares normalized public
  pipeline inventories against the published golden while checking resolver
  success, lexical shadowing runtime behavior, resolver failure diagnostics, HIR
  analysis failure diagnostics, and OriginMap call edges.
