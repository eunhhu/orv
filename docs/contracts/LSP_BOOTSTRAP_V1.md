# LSP Bootstrap v1

This contract freezes the LSP bootstrap surfaces that external editors and
first-party editor smoke paths may rely on before the full LSP method matrix is
declared stable.

It covers:

- `orv lsp snapshot <entry>`
- `orv lsp serve --stdio` `initialize` response shape
- a common stdio method inventory for `documentSymbol`, `completion`, `hover`,
  `formatting`, `semanticTokens/full`, and `foldingRange`
- method families advertised by the initialize capability object

Published golden fixtures:

- `docs/samples/lsp-snapshot-v1.golden.json` normalizes only the local entry
  path while freezing diagnostics count, document symbols, and ProjectGraph
  payload.
- `docs/samples/lsp-initialize-capabilities-v1.golden.json` freezes the full
  initialize capabilities payload.
- `docs/samples/lsp-method-inventory-v1.golden.json` freezes the common method
  response inventory for document symbols, completion labels/details, hover
  markdown, formatting edits, semantic-token data, and folding ranges.

It does not freeze every response body for navigation, rename, workspace, or
advanced editing requests. Those methods remain implementation-covered bootstrap
features until promoted by narrower contracts.

## Snapshot

`orv lsp snapshot <entry>` returns:

```json
{
  "schema_version": 1,
  "uri": "path/to/entry.orv",
  "diagnostics": [],
  "project_graph": {},
  "document_symbols": []
}
```

Root keys:

| Key | Type | Notes |
|-----|------|-------|
| `schema_version` | number | Always `1` for this contract |
| `uri` | string | Entry path as accepted by the CLI |
| `diagnostics` | array | LSP-style diagnostics from load/resolve/analyze |
| `project_graph` | object | ProjectGraph v1 projection |
| `document_symbols` | array | Graph-backed document symbol summaries |

`document_symbols[*]` keys are `name`, `kind`, `range`, `selectionRange`, and
`source_node`. `kind` is the stable orv string kind used by snapshot consumers,
not the numeric LSP protocol code. `range` and `selectionRange` are LSP range
objects with `start` and `end` positions.

## Initialize Response

`orv lsp serve --stdio` responds to `initialize` with one Content-Length framed
JSON-RPC response:

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "serverInfo": {
      "name": "orv-lsp",
      "version": "..."
    },
    "capabilities": {}
  }
}
```

`serverInfo.name` is `orv-lsp`. `serverInfo.version` follows the CLI crate
version.

## Capability Keys

The v1 initialize response advertises these capability keys:

- `textDocumentSync`
- `documentSymbolProvider`
- `codeLensProvider`
- `codeActionProvider`
- `executeCommandProvider`
- `documentLinkProvider`
- `foldingRangeProvider`
- `selectionRangeProvider`
- `semanticTokensProvider`
- `workspaceSymbolProvider`
- `definitionProvider`
- `declarationProvider`
- `typeDefinitionProvider`
- `implementationProvider`
- `monikerProvider`
- `callHierarchyProvider`
- `typeHierarchyProvider`
- `colorProvider`
- `linkedEditingRangeProvider`
- `referencesProvider`
- `documentHighlightProvider`
- `renameProvider`
- `hoverProvider`
- `signatureHelpProvider`
- `inlayHintProvider`
- `documentFormattingProvider`
- `documentRangeFormattingProvider`
- `documentOnTypeFormattingProvider`
- `completionProvider`
- `diagnosticProvider`

Nested stable capability keys:

- `textDocumentSync`: `openClose`, `change`, `save.includeText`
- `codeLensProvider`: `resolveProvider`
- `codeActionProvider`: `codeActionKinds`
- `executeCommandProvider`: `commands`
- `documentLinkProvider`: `resolveProvider`
- `semanticTokensProvider`: `legend`, `full`, `range`
- `semanticTokensProvider.legend`: `tokenTypes`, `tokenModifiers`
- `renameProvider`: `prepareProvider`
- `signatureHelpProvider`: `triggerCharacters`
- `documentOnTypeFormattingProvider`: `firstTriggerCharacter`,
  `moreTriggerCharacter`
- `completionProvider`: `triggerCharacters`
- `diagnosticProvider`: `interFileDependencies`, `workspaceDiagnostics`

`executeCommandProvider.commands` contains `orv.revealSourceNode` and
`orv.revealDiagnostic`.

## Version Policy

- `schema_version: 1` is append-only for optional snapshot fields.
- Removing or renaming any root key or capability key listed here requires a new
  contract file and migration note.
- Method result bodies outside snapshot and initialize remain bootstrap-level
  unless covered by the common method inventory fixture above.
- `Content-Length` framing is part of the stdio contract.

## Regression Coverage

- `crates/orv-cli/tests/lsp_bootstrap_contract.rs` is a CLI black-box
  regression. It runs `orv lsp snapshot` and `orv lsp serve --stdio`, then
  freezes snapshot root/document-symbol keys, the normalized snapshot payload,
  initialize root/result keys, public capability key surfaces, and the full
  initialize capabilities payload against the published golden fixtures. It also
  freezes a stdio common-method inventory covering `textDocument/documentSymbol`,
  `textDocument/completion`, `textDocument/hover`,
  `textDocument/formatting`, `textDocument/semanticTokens/full`, and
  `textDocument/foldingRange`. Its snapshot fixture covers every public
  `document_symbols[*].kind` emitted by the graph-backed snapshot contract:
  `Struct`, `Enum`, `TypeAlias`, `Function`, and `Event`.
