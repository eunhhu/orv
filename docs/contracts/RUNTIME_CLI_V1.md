# Runtime CLI v1

This contract freezes the foreground reference runtime CLI envelope for
non-server source execution.

It covers:

- `orv run <entry>` for a source file or project entry path
- stdout produced by top-level `@out`
- process exit behavior for successful foreground execution
- process exit and stderr prefix for runtime failures

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

- `crates/orv-cli/tests/runtime_cli_contract.rs` is a CLI black-box regression.
  It runs the built `orv` binary and freezes success stdout/stderr/exit behavior
  plus runtime failure exit, stderr prefix, and failure reason.
