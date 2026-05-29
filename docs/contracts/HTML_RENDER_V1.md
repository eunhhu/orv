# HTML Render v1 Contract

HTML Render v1 freezes the public zero-runtime `@html` path that can be
rendered directly by the CLI and emitted as a static build artifact.

The published golden fixture is `docs/samples/html-render-v1.golden.json`. It
freezes the compact static HTML, static bundle marker, absence of a server
runtime bundle, and `run-build` stdout envelope.

## Scope

This contract covers:

- `@out @html { ... }` entries that do not require client or server runtime
  features.
- `orv build <entry> --out <dir>` static page output.
- `orv run-build <dir>` execution for a zero-runtime static page build.
- the `bundle-plan.json` static bundle marker used by deploy and verify
  surfaces.

Client reactive entries with signals, event bindings, or WASM bootstrap are
covered by [Client Bundle v1](CLIENT_BUNDLE_V1.md). Server route rendering is
covered by [HTTP Server v1](HTTP_SERVER_V1.md).

## Source Example

```orv
@out @html { @body { @h1 "Home" @p "<script>alert(1)</script>&" @a title="<img src=x onerror=\"alert(1)\" data-note='x'>&" "safe" } }
```

## Static HTML

The example above renders this exact HTML:

```html
<html><body><h1>Home</h1><p>&lt;script&gt;alert(1)&lt;/script&gt;&amp;</p><a title="&lt;img src=x onerror=&quot;alert(1)&quot; data-note=&#39;x&#39;&gt;&amp;">safe</a></body></html>
```

The renderer preserves the current compact HTML envelope for static nodes:

- root `@html` emits `<html>...</html>`
- `@body` emits `<body>...</body>`
- `@h1` emits `<h1>...</h1>`
- `@p` emits `<p>...</p>`
- text children escape `&`, `<`, and `>` by default
- quoted attribute values escape `&`, `<`, `>`, `"`, and `'` by default

## Build Artifact

`orv build <entry> --out <dir>` writes the static page to:

```text
pages/index.html
```

`bundle-plan.json` must include a static page bundle:

```json
{
  "kind": "static_page",
  "path": "pages/index.html",
  "runtime_features": []
}
```

For a zero-runtime static page, the bundle plan must not include a
`server_runtime` bundle.

## Run-Build Output

`orv run-build <dir>` prints the static HTML bytes to stdout and writes nothing
to stderr on success. The command must select the `static_page` target from
`bundle-plan.json` instead of requiring a server launcher.

## Version Policy

HTML Render v1 is a public build/runtime contract. Breaking the static HTML
envelope, static bundle marker, target path, or `run-build` stdout behavior
requires a new contract version.

## Regression Coverage

- `docs/samples/html-render-v1.golden.json`
- `crates/orv-cli/tests/html_render_contract.rs` freezes the public black-box
  `orv build` and `orv run-build` behavior for a zero-runtime static page, and
  compares the normalized inventory against the published golden fixture.
- `crates/orv-cli/src/tests.rs` keeps focused internal coverage for static
  page artifact emission, stale server launcher avoidance, and dev bootstrap.
