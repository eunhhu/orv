# Test Runner v1

This contract freezes the public source test runner surfaces used by CLI smoke
automation and local development loops.

It covers:

- `orv test <path> --list`
- `orv test <path> --filter <substring> --list`
- `orv test <path>`
- failure exit and diagnostic prefix for a selected failing test block

It does not freeze async test isolation, richer machine-readable execution
reports, fixtures, snapshots, retries, or per-test timing. Those remain planned
test-runner extensions.

## Discovery JSON

`orv test <path> --list` emits pretty-printed JSON:

```json
{
  "schema_version": 1,
  "tests": [
    {
      "path": "path/to/source.orv",
      "name": "checkout shows cart",
      "line": 1,
      "column": 1,
      "span": {
        "start": 0,
        "end": 42
      },
      "range": {
        "start": {
          "line": 0,
          "character": 0
        },
        "end": {
          "line": 2,
          "character": 1
        }
      }
    }
  ]
}
```

The published discovery golden fixture is
`docs/samples/test-runner-list-v1.golden.json`. It normalizes the temporary
fixture root to `<fixture>` while freezing deterministic discovery order, test
names, source paths, line/column values, byte spans, and LSP ranges.

Root keys:

| Key | Type | Notes |
|-----|------|-------|
| `schema_version` | number | Always `1` for this contract |
| `tests` | array | Test cases selected by path and optional filter |

`tests[*]` keys:

| Key | Type | Notes |
|-----|------|-------|
| `path` | string | File path selected by the runner |
| `name` | string | The quoted `test "name"` block name |
| `line` | number | 1-based start line |
| `column` | number | 1-based start column |
| `span` | object | Byte range in the selected source file |
| `range` | object | 0-based LSP-style source range |

`span` keys are `start` and `end`. `range.start` and `range.end` keys are
`line` and `character`.

Ordering is deterministic: directory inputs are scanned recursively and sorted
by path before test blocks are emitted in source order.

`--filter <substring>` selects test names containing the substring. It does not
execute test bodies in list mode.

## Execution Summary

`orv test <path>` executes selected `test` blocks through the reference runtime.
On success, stdout is a single line:

```text
test: N passed
```

The process exits with code `0`. `N` is the selected passing test count after
path and optional filter selection.

On failure, the process exits non-zero and stderr starts with:

```text
error: test:
```

The error text includes the failing source path, failing test name, and runtime
error message. Exact runtime error wording follows the runtime diagnostic
surface and is not independently versioned by this contract.

## Version Policy

- `schema_version: 1` is append-only for optional discovery fields.
- Removing or renaming any root key or test-entry key listed here requires a new
  contract file and migration note.
- Changing the success summary prefix or failure stderr prefix requires a new
  contract file and migration note.
- Rich execution JSON should be introduced as an additive flag or a new
  contract version, not by changing the existing human summary line.

## Regression Coverage

- `crates/orv-cli/tests/test_runner_contract.rs` is a CLI black-box regression.
  It compares the published discovery golden fixture, freezes discovery
  root/test/span/range keys, filter semantics, success summary output, and
  failure exit/prefix behavior through the built `orv` binary.
