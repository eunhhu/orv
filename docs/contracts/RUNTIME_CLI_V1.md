# Runtime CLI v1

This contract freezes the foreground reference runtime CLI envelope for
non-server source execution.

It covers:

- `orv run <entry>` for a source file or project entry path
- stdout produced by top-level `@out`
- process exit behavior for successful foreground execution
- process exit and stderr prefix for runtime failures

The published golden fixture is `docs/samples/runtime-cli-v1.golden.json`. It
freezes the foreground success stdout/newline envelope and the runtime failure
stdout/stderr marker inventory.

It does not freeze long-running HTTP server lifecycle, request/response
semantics, route origin headers, runtime trace files, EventSource streams, or
build-artifact runners. Those surfaces are covered by narrower contracts such as
Runtime Trace v1 and Route Origin Headers v1, or remain implementation-level
until separately promoted.

## Success

`orv run <entry>` loads, resolves, lowers, and executes the program through the
reference tree-walking runtime.

For foreground programs that terminate, the process exits with code `0`.
Top-level `@out` writes one line per call to stdout:

```text
hello Ada
3
```

Rules:

- stdout preserves program order.
- each `@out` call appends one trailing newline.
- stderr is empty for successful foreground execution.
- non-string primitive output uses the runtime display representation, for
  example integers as decimal text and booleans as `true` or `false`.

## Runtime Failure

If execution reaches a runtime failure, the process exits non-zero and stderr
starts with:

```text
error:
```

The runtime error message follows the prefix. Exact runtime diagnostic wording is
owned by the runtime error model and is not separately versioned here, but the
message must include the runtime failure reason.

The CLI may emit compile/load diagnostics for programs that cannot be lowered.
Those diagnostics are outside this runtime execution contract and follow the
diagnostics/source-map contracts.

When `try/catch` handles an error from a function or lambda, execution resumes
with the caller's bindings, HTML output buffer, return state, and loop state
restored. This applies to user `throw` values and native runtime errors and
to errors in recursive calls. Function debug frames are also unwound on error.
`crates/orv-runtime/src/interp/tests/recovery.rs` covers caller bindings,
recursive catches, and HTML rendering after a caught failure.

## Version Policy

- Changing the success stdout newline rule requires a new contract file and
  migration note.
- Changing successful foreground stderr from empty to non-empty requires a new
  contract file and migration note.
- Changing the runtime failure stderr prefix requires a new contract file and
  migration note.
- Rich machine-readable runtime result JSON should be introduced as an additive
  flag or new contract, not by changing the default human CLI envelope.

## Regression Coverage

- `docs/samples/runtime-cli-v1.golden.json`
- `crates/orv-cli/tests/runtime_cli_contract.rs` is a CLI black-box regression.
  It runs the built `orv` binary and compares normalized success/failure
  inventories against the published golden, freezing stdout/stderr/exit
  behavior plus runtime failure prefix and reason markers.
