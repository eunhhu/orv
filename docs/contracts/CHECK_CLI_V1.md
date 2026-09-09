# Check CLI v1

This contract freezes the public `orv check` CLI envelope for local development
and editor smoke automation.

It covers:

- `orv check <entry>` successful foreground output
- diagnostic emission to stderr
- imported-file diagnostic source routing by diagnostic `span.file`
- `check-artifact` diagnostics after source-bundle rehydration
- failure exit behavior for check errors

The published golden fixture is `docs/samples/check-cli-v1.golden.json`. It
normalizes local paths while freezing the success stdout envelope and
imported-file diagnostic routing markers.

It does not freeze the internal resolver scope map or HIR lowering data model.
The public resolver/analyzer behavior visible through `orv check`, `orv run`,
and `orv origins` is covered by [Compiler Pipeline v1](COMPILER_PIPELINE_V1.md).
Exact diagnostic wording remains owned by the producing crate unless a narrower
contract says otherwise.

## Success

`orv check <entry>` loads the project, resolves names, lowers/analyzes the
program, and does not execute it.

For a checked program, stdout is a single line:

```text
check: path/to/entry.orv passed
```

The process exits with code `0`, and stderr is empty.

## Diagnostics

If load, resolve, or analysis emits an error, the process exits non-zero.
Diagnostics are rendered to stderr with source snippets and labels. The final
stderr line includes:

```text
error: aborting due to previous errors
```

Diagnostic source routing is part of this contract:

- a diagnostic whose span belongs to an imported file must render that imported
  file path and source line;
- it must not render an unrelated entry-file source line for that primary span;
- secondary labels are routed by their own `span.file`, so a diagnostic may
  render both entry and imported-file snippets when labels cross files;
- line/column rendering follows the shared diagnostics implementation.

The same routing applies after source-bundle rehydration for generated server
runtime artifacts. A diagnostic from an imported source bundled into
`check-artifact` must still render that imported bundle path/source, not the
entry source.

Exact diagnostic text remains owned by the producing crate, but the stderr
envelope and file/source routing are stable.

## Version Policy

- Changing the success stdout prefix or newline rule requires a new contract
  file and migration note.
- Moving diagnostics from stderr to stdout requires a new contract file and
  migration note.
- Regressing imported-file span routing is a contract break even if the process
  still exits non-zero.

## Regression Coverage

- `docs/samples/check-cli-v1.golden.json`
- `crates/orv-cli/tests/check_cli_contract.rs` is a CLI black-box regression. It
  runs the built `orv` binary, freezes success stdout/stderr/exit behavior, and
  compares normalized success, `orv check` imported-file diagnostic, and
  `check-artifact` imported-source-bundle diagnostic inventories against the
  published golden fixture. Imported-file diagnostics must render the imported
  file path and source line instead of the entry-file source line. It also
  mutates a generated server runtime artifact source bundle to verify
  imported-file routing survives `check-artifact` rehydration.
- `crates/orv-cli/src/tests/language.rs::rendered_diagnostics_use_secondary_span_file_source`
  freezes cross-file secondary-label routing in the shared renderer.
