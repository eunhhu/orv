# Native Host Desktop v1 Contract

Producer:

- `orv editor export <file> --out <dir>` as
  `native-host/desktop-package.json`
- `orv editor desktop-shell <export-dir|native-host/desktop-package.json>`
  as the normalized desktop session shape
- `native-host.json` under `host.desktop_platform_matrix`

Current regression coverage:

- `crates/orv-cli/src/tests.rs::native_host_desktop_contract_freezes_public_object_keys_and_types`
- `crates/orv-cli/src/tests.rs::native_host_desktop_shell_rejects_extra_package_root_key`
- `crates/orv-cli/src/tests.rs::native_host_desktop_shell_rejects_extra_platform_target_key`
- `crates/orv-cli/src/tests.rs::native_host_desktop_shell_rejects_empty_planned_platform_blockers`
- `crates/orv-cli/src/tests.rs::native_host_desktop_shell_rejects_empty_planned_platform_shared_contracts`
- `crates/orv-cli/src/tests.rs::native_host_desktop_shell_rejects_extra_source_permission_key`
- `crates/orv-cli/src/tests.rs::native_host_desktop_run_rejects_extra_session_root_key`
- `crates/orv-cli/src/tests.rs::editor_export_embeds_dap_debug_wiring`
- `crates/orv-cli/src/tests.rs::editor_desktop_run_probe_spawns_host_and_reads_ready_json`

## Desktop Package Root

`native-host/desktop-package.json` has exactly:

```json
{
  "schema_version": 1,
  "kind": "orv.editor.native_host.desktop_package",
  "runtime": "local-http-bridge",
  "entry": "app.orv",
  "export_root": ".",
  "artifacts": {},
  "platform_matrix": {},
  "desktop_app": {},
  "packaging": {},
  "lifecycle": {},
  "process_policy": {},
  "refresh": {},
  "source_permissions": {}
}
```

Rules:

- `schema_version` is currently `1`.
- `runtime` is currently `local-http-bridge`.
- `artifacts` contains the shell, state, native-host manifest, bridge script,
  desktop launcher, macOS SwiftPM source, packaging script, plist, and
  entitlements paths.
- New public root keys require a changelog entry, this file update, and the
  contract regression update.
- `orv editor desktop-shell` rejects unknown package root keys, platform matrix
  target keys, and source-permission keys before normalizing a session.

## Platform Matrix

`platform_matrix` has exactly:

```json
{
  "schema_version": 1,
  "kind": "orv.editor.native_host.desktop_platform_matrix",
  "default_platform": "macos",
  "implemented_count": 1,
  "planned_count": 2,
  "targets": []
}
```

Current targets:

- `macos`: `implemented`, SwiftPM AppKit/WKWebView container, generated
  package source, `desktop-session.json` input, packaging script, ad-hoc or
  Developer ID signing, optional notarization, and local verification commands.
- `windows`: `planned`, WebView2 container. It must consume the same
  `desktop-package.json`, `desktop-session.json`, and bridge script contracts
  before it can move out of planned status.
- `linux`: `planned`, WebKitGTK or Tauri/WebView runtime. It must consume the
  same desktop package/session/bridge contracts and define release packaging
  and sandbox/source-permission policy before it can move out of planned status.

The macOS target keys are exactly:

```json
{
  "platform": "macos",
  "status": "implemented",
  "container": "SwiftPM AppKit/WKWebView",
  "package": "native-host/desktop-app/Package.swift",
  "main": "native-host/desktop-app/Sources/OrvEditorDesktop/main.swift",
  "session_artifact": "native-host/desktop-session.json",
  "packaging": {},
  "capabilities": {},
  "verification": []
}
```

The Windows and Linux target keys are exactly:

```json
{
  "platform": "windows",
  "status": "planned",
  "container": "WebView2",
  "blocked_by": [],
  "shared_contracts": []
}
```

Rules:

- Planned targets must carry `blocked_by[]` entries with `id` and `reason`.
- Planned targets must name shared contracts they intend to consume.
- A target may move to `implemented` only when a generated container,
  packaging path, and automated verification exist.

## Source Permissions

`source_permissions` has exactly:

```json
{
  "mode": "prompt-before-source-reveal",
  "default": "prompt-before-open",
  "denied_mode": "open-read-only",
  "reveal_requires_origin_id": true,
  "webview_injection": "orvNativeHostSourcePermissions",
  "decision_event": "orv:source-permission",
  "blocked_event": "orv:source-permission-blocked",
  "root_count": 1,
  "source_count": 1,
  "allowed_roots": [],
  "source_hashes": [],
  "prompt": {}
}
```

`prompt` has exactly:

```json
{
  "title": "Allow orv source reveal access?",
  "allow_label": "Allow Source Reveal",
  "read_only_label": "Open Read-Only",
  "quit_label": "Quit"
}
```

Rules:

- Denied source permission opens the editor in read-only mode instead of
  quitting.
- Native/WebView bridges must block source reveal actions when injected
  `orvNativeHostSourcePermissions.allowed` is `false`.
- `source_hashes[]` mirrors the export live-refresh source hash inventory.

## Desktop Shell Session

`orv editor desktop-shell` normalizes the package into a session with exactly:

```json
{
  "schema_version": 1,
  "kind": "orv.editor.native_host.desktop_shell",
  "status": "ready",
  "root": "/absolute/export/root",
  "package": {},
  "lifecycle": {},
  "process_supervision": {},
  "webview": {},
  "refresh": {},
  "platform_matrix": {},
  "source_permission_prompt": {},
  "artifact_checks": [],
  "session_artifact": {}
}
```

Rules:

- `platform_matrix` must match the package `platform_matrix`.
- `source_permission_prompt` must carry the same denied-mode, WebView injection,
  decision-event, blocked-event, root count, source count, roots, hashes, and
  prompt metadata from `source_permissions`.
- `artifact_checks[]` records whether referenced package artifacts exist.
- `orv editor desktop-run` rejects unknown `desktop-session.json` root keys
  before spawning a native host process.

## Version Policy

Native Host Desktop v1 is public to editor containers and local native-host
bridges. Breaking key/type changes require a schema version bump or documented
compatibility bridge plus updates to this file, changelog, and contract
regression.
