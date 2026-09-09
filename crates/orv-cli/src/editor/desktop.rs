#![allow(clippy::redundant_pub_crate, clippy::wildcard_imports)]

use super::*;

pub(crate) fn cmd_editor_desktop_shell(
    package: &Path,
    listen: &str,
    write_session: bool,
) -> anyhow::Result<()> {
    let value = editor_native_host_desktop_shell_json(package, listen)?;
    if write_session {
        write_editor_native_host_desktop_session_if_configured(package, &value)?;
    }
    println!("{}", serde_json::to_string_pretty(&value)?);
    Ok(())
}

pub(crate) fn cmd_editor_desktop_run(
    session: &Path,
    listen: &str,
    probe: bool,
    open: bool,
) -> anyhow::Result<()> {
    let mut run = editor_native_host_desktop_spawn_run(session, listen, open)?;
    println!("{}", serde_json::to_string_pretty(&run.value)?);
    std::io::Write::flush(&mut std::io::stdout())?;
    if probe {
        let _ = run.child.kill();
        let _ = run.child.wait();
        return Ok(());
    }
    let status = run.child.wait()?;
    eprintln!("editor desktop host exited: {status}");
    Ok(())
}

pub(crate) fn editor_native_host_desktop_package_json(
    entry: &Path,
    state: &serde_json::Value,
) -> serde_json::Value {
    let trace_enabled = state.get("trace").is_some();
    let mut allowed_commands = vec![
        serde_json::json!({
            "name": "host_server",
            "argv_prefix": ["orv", "editor", "host", "."],
            "working_directory": ".",
            "purpose": "serve the exported shell and local native-host bridge",
        }),
        serde_json::json!({
            "name": "debug_runner",
            "argv_prefix": ["orv", "editor", "run-debug", EDITOR_DEBUG_SESSION_RUNNER_PATH],
            "working_directory": ".",
            "result": {
                "json": EDITOR_DEBUG_SESSION_RESULT_PATH,
                "html": EDITOR_DEBUG_SESSION_RESULT_HTML_PATH,
            },
            "purpose": "execute exported DAP controls and refresh the debug result panel",
        }),
    ];
    if trace_enabled {
        allowed_commands.push(serde_json::json!({
            "name": "trace_reveal_action",
            "endpoint": "/__orv/native-host/action",
            "argv_prefix": [
                "orv",
                "editor",
                "run-action",
                EDITOR_NATIVE_HOST_MANIFEST_PATH,
            ],
            "working_directory": ".",
            "result": {
                "json": EDITOR_TRACE_ACTION_RESULT_PATH,
                "html": EDITOR_TRACE_ACTION_RESULT_HTML_PATH,
            },
            "purpose": "execute allowlisted trace reveal actions and refresh the trace action panel",
        }));
    }
    if let Some(stream_command) = state.pointer("/trace/stream_runner/command").cloned() {
        allowed_commands.push(serde_json::json!({
            "name": "trace_stream_runner",
            "argv": stream_command,
            "working_directory": ".",
            "result": {
                "json": "orv editor trace-stream stdout",
            },
            "purpose": "normalize captured EventSource trace bodies",
        }));
    }
    serde_json::json!({
        "schema_version": 1,
        "kind": "orv.editor.native_host.desktop_package",
        "runtime": "local-http-bridge",
        "entry": entry.display().to_string(),
        "export_root": ".",
        "artifacts": {
            "shell": "index.html",
            "state": "state.json",
            "manifest": EDITOR_NATIVE_HOST_MANIFEST_PATH,
            "bridge_script": EDITOR_NATIVE_HOST_BRIDGE_JS_PATH,
            "launcher": EDITOR_NATIVE_HOST_DESKTOP_LAUNCHER_PATH,
            "desktop_packaging": EDITOR_NATIVE_HOST_DESKTOP_PACKAGING_PATH,
            "desktop_package_script": EDITOR_NATIVE_HOST_DESKTOP_PACKAGE_SCRIPT_PATH,
            "desktop_app_package": EDITOR_NATIVE_HOST_DESKTOP_APP_PACKAGE_PATH,
            "desktop_app_info_plist": EDITOR_NATIVE_HOST_DESKTOP_APP_INFO_PLIST_PATH,
            "desktop_app_entitlements": EDITOR_NATIVE_HOST_DESKTOP_APP_ENTITLEMENTS_PATH,
            "desktop_app_main": EDITOR_NATIVE_HOST_DESKTOP_APP_MAIN_PATH,
        },
        "platform_matrix": editor_native_host_desktop_platform_matrix_json(),
        "desktop_app": editor_native_host_desktop_app_contract_json(),
        "packaging": editor_native_host_desktop_packaging_json(),
        "lifecycle": {
            "spawn": {
                "command": ["orv", "editor", "host", ".", "--listen", "127.0.0.1:0"],
                "stdout_kind": "orv.editor.native_host.server",
                "url_field": "url",
            },
            "webview": {
                "initial_url_template": "{url}index.html",
                "reload_policy": "reload-panel-artifacts-after-refresh-event",
            },
            "shutdown": {
                "strategy": "terminate-host-process",
            },
        },
        "process_policy": {
            "spawn_model": "local-child-process",
            "deny_unknown_commands": true,
            "allowed_commands": allowed_commands,
        },
        "refresh": {
            "events": editor_native_host_desktop_refresh_events_json(trace_enabled),
        },
        "source_permissions": editor_native_host_desktop_source_permissions_json(entry, state),
    })
}

pub(crate) fn editor_native_host_desktop_platform_matrix_json() -> serde_json::Value {
    serde_json::json!({
        "schema_version": 1,
        "kind": "orv.editor.native_host.desktop_platform_matrix",
        "default_platform": "macos",
        "implemented_count": 1,
        "planned_count": 2,
        "targets": [
            {
                "platform": "macos",
                "status": "implemented",
                "container": "SwiftPM AppKit/WKWebView",
                "package": EDITOR_NATIVE_HOST_DESKTOP_APP_PACKAGE_PATH,
                "main": EDITOR_NATIVE_HOST_DESKTOP_APP_MAIN_PATH,
                "session_artifact": EDITOR_NATIVE_HOST_DESKTOP_SESSION_PATH,
                "packaging": {
                    "script": EDITOR_NATIVE_HOST_DESKTOP_PACKAGE_SCRIPT_PATH,
                    "bundle": "native-host/dist/OrvEditorDesktop.app",
                    "codesign": "ad-hoc-or-developer-id",
                    "notarization": "optional",
                },
                "capabilities": {
                    "webview": "WKWebView",
                    "process_supervision": "Foundation.Process",
                    "source_permission_prompt": "NSAlert",
                    "source_permission_denied_mode": "open-read-only",
                    "local_http_bridge": true,
                },
                "verification": [
                    "swift build --package-path native-host/desktop-app -c release",
                    "native-host/package-desktop-app.sh",
                    "codesign --verify --deep --strict native-host/dist/OrvEditorDesktop.app",
                ],
            },
            {
                "platform": "windows",
                "status": "planned",
                "container": "WebView2",
                "blocked_by": [
                    {
                        "id": "windows-webview2-container",
                        "reason": "needs native container implementation that consumes desktop-session.json and the same local HTTP bridge contract",
                    },
                    {
                        "id": "windows-signing-release-profile",
                        "reason": "needs Authenticode/MSIX signing and installer policy before release claim",
                    },
                ],
                "shared_contracts": [
                    EDITOR_NATIVE_HOST_DESKTOP_PACKAGE_PATH,
                    EDITOR_NATIVE_HOST_DESKTOP_SESSION_PATH,
                    EDITOR_NATIVE_HOST_BRIDGE_JS_PATH,
                ],
            },
            {
                "platform": "linux",
                "status": "planned",
                "container": "WebKitGTK or Tauri/WebView runtime",
                "blocked_by": [
                    {
                        "id": "linux-webview-container",
                        "reason": "needs native container implementation that consumes desktop-session.json and the same local HTTP bridge contract",
                    },
                    {
                        "id": "linux-packaging-release-profile",
                        "reason": "needs AppImage/Flatpak/deb packaging and sandbox/source permission policy before release claim",
                    },
                ],
                "shared_contracts": [
                    EDITOR_NATIVE_HOST_DESKTOP_PACKAGE_PATH,
                    EDITOR_NATIVE_HOST_DESKTOP_SESSION_PATH,
                    EDITOR_NATIVE_HOST_BRIDGE_JS_PATH,
                ],
            },
        ],
    })
}

pub(crate) fn editor_native_host_desktop_app_contract_json() -> serde_json::Value {
    serde_json::json!({
        "schema_version": 1,
        "kind": "orv.editor.native_host.desktop_app.swiftpm",
        "platform": "macos",
        "package": EDITOR_NATIVE_HOST_DESKTOP_APP_PACKAGE_PATH,
        "main": EDITOR_NATIVE_HOST_DESKTOP_APP_MAIN_PATH,
        "info_plist": EDITOR_NATIVE_HOST_DESKTOP_APP_INFO_PLIST_PATH,
        "entitlements": EDITOR_NATIVE_HOST_DESKTOP_APP_ENTITLEMENTS_PATH,
        "product": "OrvEditorDesktop",
        "run_command": [
            "swift",
            "run",
            "--package-path",
            "native-host/desktop-app",
            "OrvEditorDesktop",
            EDITOR_NATIVE_HOST_DESKTOP_SESSION_PATH,
        ],
        "capabilities": {
            "webview": "WKWebView",
            "process_supervision": "Foundation.Process",
            "source_permission_prompt": "NSAlert",
            "source_permission_state": "WKUserScript",
            "source_permission_denied_mode": "open-read-only",
        },
        "packaging": editor_native_host_desktop_packaging_json(),
    })
}

pub(crate) fn editor_native_host_desktop_packaging_json() -> serde_json::Value {
    serde_json::json!({
        "schema_version": 1,
        "kind": "orv.editor.native_host.desktop_app.packaging",
        "platform": "macos",
        "bundle": {
            "path": "native-host/dist/OrvEditorDesktop.app",
            "identifier": "dev.orv.editor.desktop",
            "executable": "Contents/MacOS/OrvEditorDesktop",
            "info_plist": EDITOR_NATIVE_HOST_DESKTOP_APP_INFO_PLIST_PATH,
            "entitlements": EDITOR_NATIVE_HOST_DESKTOP_APP_ENTITLEMENTS_PATH,
        },
        "script": EDITOR_NATIVE_HOST_DESKTOP_PACKAGE_SCRIPT_PATH,
        "build_command": [
            "swift",
            "build",
            "--package-path",
            "native-host/desktop-app",
            "-c",
            "release",
        ],
        "codesign": {
            "identity_env": "ORV_EDITOR_CODESIGN_IDENTITY",
            "default": "ad-hoc",
            "hardened_runtime": true,
            "developer_id_required_for_notarization": true,
            "entitlements": EDITOR_NATIVE_HOST_DESKTOP_APP_ENTITLEMENTS_PATH,
        },
        "notarization": {
            "status": "optional",
            "enable_env": "ORV_EDITOR_NOTARIZE",
            "profile_env": "ORV_EDITOR_NOTARY_PROFILE",
            "apple_id_env": "ORV_EDITOR_NOTARY_APPLE_ID",
            "password_env": "ORV_EDITOR_NOTARY_PASSWORD",
            "team_id_env": "ORV_EDITOR_NOTARY_TEAM_ID",
            "zip_path": "native-host/dist/OrvEditorDesktop.zip",
            "requires_developer_id_identity": true,
            "requires_notarytool_credentials": true,
            "staple": true,
        },
        "validation": {
            "bundle_structure": [
                "Contents/Info.plist",
                "Contents/MacOS/OrvEditorDesktop",
            ],
            "local_commands": [
                "codesign --verify --deep --strict native-host/dist/OrvEditorDesktop.app",
            ],
            "distribution_commands": [
                "xcrun stapler validate native-host/dist/OrvEditorDesktop.app",
                "spctl --assess --type execute native-host/dist/OrvEditorDesktop.app",
            ],
        },
    })
}

pub(crate) fn editor_native_host_desktop_refresh_events_json(
    trace_enabled: bool,
) -> Vec<serde_json::Value> {
    let mut events = vec![serde_json::json!({
        "event": "orv:debug-session-result",
        "panel": "debug_result",
        "json": EDITOR_DEBUG_SESSION_RESULT_PATH,
        "html": EDITOR_DEBUG_SESSION_RESULT_HTML_PATH,
    })];
    if trace_enabled {
        events.push(serde_json::json!({
            "event": "orv:trace-action-result",
            "panel": "trace_action_result",
            "json": EDITOR_TRACE_ACTION_RESULT_PATH,
            "html": EDITOR_TRACE_ACTION_RESULT_HTML_PATH,
        }));
    }
    events
}

pub(crate) fn editor_native_host_desktop_source_permissions_json(
    entry: &Path,
    state: &serde_json::Value,
) -> serde_json::Value {
    let mut roots = BTreeSet::new();
    if let Some(parent) = entry.parent() {
        roots.insert(parent.display().to_string());
    }
    if let Some(build_dir) = state
        .pointer("/production/build_dir")
        .and_then(serde_json::Value::as_str)
    {
        if !build_dir.trim().is_empty() {
            roots.insert(build_dir.to_string());
        }
    }
    for source in state
        .pointer("/snapshot/live_refresh/watch/sources")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
    {
        if let Some(path) = source.get("path").and_then(serde_json::Value::as_str) {
            if let Some(parent) = Path::new(path).parent() {
                roots.insert(parent.display().to_string());
            }
        }
    }
    let source_hashes = state
        .pointer("/snapshot/live_refresh/watch/sources")
        .cloned()
        .unwrap_or_else(|| serde_json::json!([]));
    let source_count = source_hashes.as_array().map_or(0, |sources| sources.len());
    let allowed_roots = roots.into_iter().collect::<Vec<_>>();
    serde_json::json!({
        "mode": "prompt-before-source-reveal",
        "default": "prompt-before-open",
        "denied_mode": "open-read-only",
        "reveal_requires_origin_id": true,
        "webview_injection": "orvNativeHostSourcePermissions",
        "decision_event": "orv:source-permission",
        "blocked_event": "orv:source-permission-blocked",
        "root_count": allowed_roots.len(),
        "source_count": source_count,
        "allowed_roots": allowed_roots,
        "source_hashes": source_hashes,
        "prompt": {
            "title": "Allow orv source reveal access?",
            "allow_label": "Allow Source Reveal",
            "read_only_label": "Open Read-Only",
            "quit_label": "Quit",
        },
    })
}

pub(crate) fn write_editor_native_host_desktop_launcher(out: &Path) -> anyhow::Result<()> {
    let path = out.join(EDITOR_NATIVE_HOST_DESKTOP_LAUNCHER_PATH);
    write_text(&path, editor_native_host_desktop_launcher_sh())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = std::fs::metadata(&path)?.permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(&path, permissions)?;
    }
    Ok(())
}

pub(crate) fn write_editor_native_host_desktop_app(out: &Path) -> anyhow::Result<()> {
    write_text(
        &out.join(EDITOR_NATIVE_HOST_DESKTOP_APP_PACKAGE_PATH),
        editor_native_host_desktop_app_package_swift(),
    )?;
    write_text(
        &out.join(EDITOR_NATIVE_HOST_DESKTOP_APP_INFO_PLIST_PATH),
        editor_native_host_desktop_app_info_plist(),
    )?;
    write_text(
        &out.join(EDITOR_NATIVE_HOST_DESKTOP_APP_ENTITLEMENTS_PATH),
        editor_native_host_desktop_app_entitlements_plist(),
    )?;
    write_text(
        &out.join(EDITOR_NATIVE_HOST_DESKTOP_APP_MAIN_PATH),
        editor_native_host_desktop_app_main_swift(),
    )?;
    Ok(())
}

pub(crate) fn write_editor_native_host_desktop_packaging(out: &Path) -> anyhow::Result<()> {
    write_json(
        &out.join(EDITOR_NATIVE_HOST_DESKTOP_PACKAGING_PATH),
        &editor_native_host_desktop_packaging_json(),
    )?;
    let path = out.join(EDITOR_NATIVE_HOST_DESKTOP_PACKAGE_SCRIPT_PATH);
    write_text(&path, editor_native_host_desktop_package_script_sh())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = std::fs::metadata(&path)?.permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(&path, permissions)?;
    }
    Ok(())
}

pub(crate) fn editor_native_host_desktop_launcher_sh() -> &'static str {
    r#"#!/usr/bin/env sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
LISTEN="${ORV_EDITOR_HOST_LISTEN:-127.0.0.1:0}"

exec orv editor host "$ROOT" --listen "$LISTEN"
"#
}

pub(crate) fn editor_native_host_desktop_package_script_sh() -> &'static str {
    r#"#!/usr/bin/env sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
APP="${ORV_EDITOR_APP_OUT:-$ROOT/native-host/dist/OrvEditorDesktop.app}"
PACKAGE="$ROOT/native-host/desktop-app"
EXECUTABLE="$PACKAGE/.build/release/OrvEditorDesktop"
IDENTITY="${ORV_EDITOR_CODESIGN_IDENTITY:-}"
NOTARIZE="${ORV_EDITOR_NOTARIZE:-0}"
NOTARY_PROFILE="${ORV_EDITOR_NOTARY_PROFILE:-}"
NOTARY_APPLE_ID="${ORV_EDITOR_NOTARY_APPLE_ID:-}"
NOTARY_PASSWORD="${ORV_EDITOR_NOTARY_PASSWORD:-}"
NOTARY_TEAM_ID="${ORV_EDITOR_NOTARY_TEAM_ID:-}"
ZIP="${ORV_EDITOR_APP_ZIP:-$ROOT/native-host/dist/OrvEditorDesktop.zip}"
SIGNED=false
NOTARIZED=false

swift build --package-path "$PACKAGE" -c release
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp "$PACKAGE/Info.plist" "$APP/Contents/Info.plist"
cp "$EXECUTABLE" "$APP/Contents/MacOS/OrvEditorDesktop"
chmod 755 "$APP/Contents/MacOS/OrvEditorDesktop"

if command -v codesign >/dev/null 2>&1; then
  if [ -n "$IDENTITY" ]; then
    codesign --force --options runtime --entitlements "$PACKAGE/OrvEditorDesktop.entitlements" --sign "$IDENTITY" "$APP"
    SIGNED=true
  else
    codesign --force --sign - "$APP"
  fi
fi

if [ "$NOTARIZE" = "1" ]; then
  if [ -z "$IDENTITY" ]; then
    printf 'ORV_EDITOR_NOTARIZE=1 requires ORV_EDITOR_CODESIGN_IDENTITY\n' >&2
    exit 2
  fi
  if [ "$SIGNED" != "true" ]; then
    printf 'ORV_EDITOR_NOTARIZE=1 requires codesign with the Developer ID identity\n' >&2
    exit 2
  fi
  if ! command -v ditto >/dev/null 2>&1; then
    printf 'ORV_EDITOR_NOTARIZE=1 requires ditto\n' >&2
    exit 2
  fi
  if ! command -v xcrun >/dev/null 2>&1; then
    printf 'ORV_EDITOR_NOTARIZE=1 requires xcrun notarytool and stapler\n' >&2
    exit 2
  fi
  rm -f "$ZIP"
  ditto -c -k --keepParent "$APP" "$ZIP"
  if [ -n "$NOTARY_PROFILE" ]; then
    xcrun notarytool submit "$ZIP" --keychain-profile "$NOTARY_PROFILE" --wait
  else
    if [ -z "$NOTARY_APPLE_ID" ] || [ -z "$NOTARY_PASSWORD" ] || [ -z "$NOTARY_TEAM_ID" ]; then
      printf 'ORV_EDITOR_NOTARIZE=1 requires ORV_EDITOR_NOTARY_PROFILE or ORV_EDITOR_NOTARY_APPLE_ID/ORV_EDITOR_NOTARY_PASSWORD/ORV_EDITOR_NOTARY_TEAM_ID\n' >&2
      exit 2
    fi
    xcrun notarytool submit "$ZIP" --apple-id "$NOTARY_APPLE_ID" --password "$NOTARY_PASSWORD" --team-id "$NOTARY_TEAM_ID" --wait
  fi
  xcrun stapler staple "$APP"
  NOTARIZED=true
fi

printf '{"schema_version":1,"kind":"orv.editor.native_host.desktop_app.package_result","app":"%s","signed":%s,"notarized":%s,"zip":"%s"}\n' "$APP" "$SIGNED" "$NOTARIZED" "$ZIP"
"#
}

pub(crate) fn editor_native_host_desktop_app_package_swift() -> &'static str {
    r#"// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "OrvEditorDesktopHost",
    platforms: [.macOS(.v13)],
    products: [
        .executable(name: "OrvEditorDesktop", targets: ["OrvEditorDesktop"])
    ],
    targets: [
        .executableTarget(name: "OrvEditorDesktop")
    ]
)
"#
}

pub(crate) fn editor_native_host_desktop_app_info_plist() -> &'static str {
    r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleDevelopmentRegion</key>
  <string>en</string>
  <key>CFBundleExecutable</key>
  <string>OrvEditorDesktop</string>
  <key>CFBundleIdentifier</key>
  <string>dev.orv.editor.desktop</string>
  <key>CFBundleInfoDictionaryVersion</key>
  <string>6.0</string>
  <key>CFBundleName</key>
  <string>orv Editor</string>
  <key>CFBundlePackageType</key>
  <string>APPL</string>
  <key>CFBundleShortVersionString</key>
  <string>0.1.0</string>
  <key>CFBundleVersion</key>
  <string>1</string>
  <key>LSMinimumSystemVersion</key>
  <string>13.0</string>
  <key>NSHighResolutionCapable</key>
  <true/>
  <key>NSSupportsAutomaticGraphicsSwitching</key>
  <true/>
</dict>
</plist>
"#
}

pub(crate) fn editor_native_host_desktop_app_entitlements_plist() -> &'static str {
    r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict/>
</plist>
"#
}

pub(crate) fn editor_native_host_desktop_app_main_swift() -> &'static str {
    r#"import AppKit
import Foundation
import WebKit

enum DesktopError: Error {
    case invalidSession(String)
    case invalidReadyPayload
    case missingHostURL
}

func readJSON(_ url: URL) throws -> [String: Any] {
    let data = try Data(contentsOf: url)
    guard let value = try JSONSerialization.jsonObject(with: data) as? [String: Any] else {
        throw DesktopError.invalidSession("expected object in \(url.path)")
    }
    return value
}

func nested(_ value: [String: Any], _ path: [String]) -> Any? {
    var current: Any = value
    for part in path {
        guard let object = current as? [String: Any], let next = object[part] else {
            return nil
        }
        current = next
    }
    return current
}

func stringArray(_ value: Any?) -> [String] {
    return value as? [String] ?? []
}

func jsonLiteral(_ value: [String: Any]) -> String {
    guard let data = try? JSONSerialization.data(withJSONObject: value, options: [.sortedKeys]),
          let text = String(data: data, encoding: .utf8) else {
        return "{}"
    }
    return text
}

func sourcePermissionScript(_ state: [String: Any]) -> String {
    let json = jsonLiteral(state)
    return """
window.orvNativeHostSourcePermissions = \(json);
window.dispatchEvent(new CustomEvent('orv:source-permission', { detail: window.orvNativeHostSourcePermissions }));
"""
}

func readReadyJSON(from handle: FileHandle) throws -> [String: Any] {
    var data = Data()
    var started = false
    var depth = 0
    var inString = false
    var escaped = false

    while true {
        let chunk = handle.readData(ofLength: 1)
        if chunk.isEmpty {
            break
        }
        let byte = chunk[chunk.startIndex]
        if !started {
            if byte == 10 || byte == 13 || byte == 32 || byte == 9 {
                continue
            }
            guard byte == 123 || byte == 91 else {
                throw DesktopError.invalidReadyPayload
            }
            started = true
            depth = 1
            data.append(byte)
            continue
        }
        data.append(byte)
        if inString {
            if escaped {
                escaped = false
            } else if byte == 92 {
                escaped = true
            } else if byte == 34 {
                inString = false
            }
            continue
        }
        if byte == 34 {
            inString = true
        } else if byte == 123 || byte == 91 {
            depth += 1
        } else if byte == 125 || byte == 93 {
            depth -= 1
            if depth == 0 {
                break
            }
        }
    }

    guard !data.isEmpty, depth == 0 else {
        throw DesktopError.invalidReadyPayload
    }
    guard let value = try JSONSerialization.jsonObject(with: data) as? [String: Any] else {
        throw DesktopError.invalidReadyPayload
    }
    return value
}

func webviewURL(session: [String: Any], ready: [String: Any]) throws -> URL {
    guard let base = ready["url"] as? String else {
        throw DesktopError.missingHostURL
    }
    let template = nested(session, ["webview", "initial_url_template"]) as? String ?? "{url}index.html"
    guard let url = URL(string: template.replacingOccurrences(of: "{url}", with: base)) else {
        throw DesktopError.missingHostURL
    }
    return url
}

func sourcePermissionDecision(session: [String: Any]) -> (allowed: Bool, state: [String: Any], shouldQuit: Bool) {
    guard let prompt = session["source_permission_prompt"] as? [String: Any] else {
        return (
            true,
            [
                "kind": "orv.editor.native_host.source_permission",
                "allowed": true,
                "policy": "none",
                "reason": "no source permission prompt"
            ],
            false
        )
    }
    let policy = prompt["default"] as? String ?? "prompt-before-open"
    if policy != "prompt-before-open" {
        return (
            true,
            [
                "kind": "orv.editor.native_host.source_permission",
                "allowed": true,
                "policy": policy,
                "reason": "prompt not required"
            ],
            false
        )
    }
    let roots = stringArray(prompt["allowed_roots"])
    let sourceHashes = prompt["source_hashes"] as? [Any] ?? []
    let promptLabels = prompt["prompt"] as? [String: Any] ?? [:]
    let blockedEvent = prompt["blocked_event"] as? String ?? "orv:source-permission-blocked"
    let baseState: [String: Any] = [
        "kind": "orv.editor.native_host.source_permission",
        "policy": policy,
        "mode": prompt["mode"] as? String ?? "prompt-before-source-reveal",
        "denied_mode": prompt["denied_mode"] as? String ?? "open-read-only",
        "reveal_requires_origin_id": prompt["reveal_requires_origin_id"] as? Bool ?? true,
        "blocked_event": blockedEvent,
        "allowed_roots": roots,
        "root_count": roots.count,
        "source_count": sourceHashes.count,
        "source_hashes": sourceHashes
    ]
    let alert = NSAlert()
    alert.messageText = promptLabels["title"] as? String ?? "Allow orv source reveal access?"
    alert.informativeText = roots.isEmpty
        ? "This editor session has no declared source roots."
        : "Source roots: \(roots.count)\nTracked sources: \(sourceHashes.count)\n\n" + roots.joined(separator: "\n")
    alert.addButton(withTitle: promptLabels["allow_label"] as? String ?? "Allow Source Reveal")
    alert.addButton(withTitle: promptLabels["read_only_label"] as? String ?? "Open Read-Only")
    alert.addButton(withTitle: promptLabels["quit_label"] as? String ?? "Quit")
    let response = alert.runModal()
    if response == .alertFirstButtonReturn {
        var state = baseState
        state["allowed"] = true
        state["reason"] = "user_allowed"
        return (true, state, false)
    }
    if response == .alertSecondButtonReturn {
        var state = baseState
        state["allowed"] = false
        state["reason"] = "user_read_only"
        return (false, state, false)
    }
    var state = baseState
    state["allowed"] = false
    state["reason"] = "user_quit"
    return (false, state, true)
}

final class AppDelegate: NSObject, NSApplicationDelegate {
    var process: Process?
    var window: NSWindow?
    var webView: WKWebView?

    func applicationDidFinishLaunching(_ notification: Notification) {
        do {
            let sessionPath = CommandLine.arguments.dropFirst().first ?? "native-host/desktop-session.json"
            let sessionURL = URL(fileURLWithPath: sessionPath)
            let session = try readJSON(sessionURL)
            let command = stringArray(nested(session, ["lifecycle", "spawn", "command"]))
            guard !command.isEmpty else {
                throw DesktopError.invalidSession("missing lifecycle.spawn.command")
            }

            let pipe = Pipe()
            let task = Process()
            task.executableURL = URL(fileURLWithPath: "/usr/bin/env")
            task.arguments = command
            task.standardOutput = pipe
            try task.run()
            process = task

            let ready = try readReadyJSON(from: pipe.fileHandleForReading)
            let url = try webviewURL(session: session, ready: ready)
            let sourcePermission = sourcePermissionDecision(session: session)
            guard !sourcePermission.shouldQuit else {
                NSApplication.shared.terminate(nil)
                return
            }

            let configuration = WKWebViewConfiguration()
            let userContentController = WKUserContentController()
            userContentController.addUserScript(WKUserScript(
                source: sourcePermissionScript(sourcePermission.state),
                injectionTime: .atDocumentStart,
                forMainFrameOnly: false
            ))
            configuration.userContentController = userContentController
            let webView = WKWebView(frame: .zero, configuration: configuration)
            webView.load(URLRequest(url: url))
            let window = NSWindow(
                contentRect: NSRect(x: 0, y: 0, width: 1280, height: 820),
                styleMask: [.titled, .closable, .miniaturizable, .resizable],
                backing: .buffered,
                defer: false
            )
            window.title = "orv Editor"
            window.center()
            window.contentView = webView
            window.makeKeyAndOrderFront(nil)
            self.window = window
            self.webView = webView
        } catch {
            let alert = NSAlert(error: error)
            alert.runModal()
            NSApplication.shared.terminate(nil)
        }
    }

    func applicationWillTerminate(_ notification: Notification) {
        process?.terminate()
    }
}

let app = NSApplication.shared
let delegate = AppDelegate()
app.setActivationPolicy(.regular)
app.delegate = delegate
app.activate(ignoringOtherApps: true)
app.run()
"#
}

pub(crate) fn editor_native_host_desktop_shell_json(
    package: &Path,
    listen: &str,
) -> anyhow::Result<serde_json::Value> {
    let package_path = editor_native_host_desktop_package_input_path(package);
    let root = editor_native_host_desktop_package_root(&package_path)?;
    let package_value = read_json_value(&package_path)?;
    if package_value
        .get("kind")
        .and_then(serde_json::Value::as_str)
        != Some("orv.editor.native_host.desktop_package")
    {
        anyhow::bail!(
            "desktop shell requires {} kind in {}",
            "orv.editor.native_host.desktop_package",
            package_path.display()
        );
    }
    verify_editor_native_host_desktop_package_contract_keys(&package_value)?;
    let artifact_checks = editor_native_host_desktop_artifact_checks_json(&root, &package_value);
    let artifacts_ready = artifact_checks
        .iter()
        .all(|check| check.get("exists").and_then(serde_json::Value::as_bool) == Some(true));
    let launch_command =
        editor_native_host_desktop_launch_command_json(&root, &package_value, listen)?;
    let allowed_commands = package_value
        .pointer("/process_policy/allowed_commands")
        .cloned()
        .unwrap_or_else(|| serde_json::json!([]));
    let refresh_events = package_value
        .pointer("/refresh/events")
        .cloned()
        .unwrap_or_else(|| serde_json::json!([]));
    let source_permissions = package_value
        .get("source_permissions")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));
    let platform_matrix = package_value
        .get("platform_matrix")
        .cloned()
        .unwrap_or_else(editor_native_host_desktop_platform_matrix_json);
    Ok(serde_json::json!({
        "schema_version": 1,
        "kind": "orv.editor.native_host.desktop_shell",
        "status": if artifacts_ready { "ready" } else { "incomplete" },
        "root": root.display().to_string(),
        "package": {
            "path": package_path.display().to_string(),
            "hash": stable_json_hash(&package_value)?,
        },
        "lifecycle": {
            "spawn": {
                "command": launch_command,
                "stdout_kind": package_value
                    .pointer("/lifecycle/spawn/stdout_kind")
                    .cloned()
                    .unwrap_or_else(|| serde_json::json!("orv.editor.native_host.server")),
                "url_field": package_value
                    .pointer("/lifecycle/spawn/url_field")
                    .cloned()
                    .unwrap_or_else(|| serde_json::json!("url")),
            },
            "webview": package_value
                .pointer("/lifecycle/webview")
                .cloned()
                .unwrap_or_else(|| serde_json::json!({})),
            "shutdown": package_value
                .pointer("/lifecycle/shutdown")
                .cloned()
                .unwrap_or_else(|| serde_json::json!({ "strategy": "terminate-host-process" })),
        },
        "process_supervision": {
            "mode": package_value
                .pointer("/process_policy/spawn_model")
                .cloned()
                .unwrap_or_else(|| serde_json::json!("local-child-process")),
            "deny_unknown_commands": package_value
                .pointer("/process_policy/deny_unknown_commands")
                .cloned()
                .unwrap_or(serde_json::Value::Bool(true)),
            "allowed_commands": allowed_commands,
            "expected_host": {
                "argv0": launch_command.get(0).cloned().unwrap_or(serde_json::Value::Null),
                "command": launch_command,
            },
        },
        "webview": {
            "initial_url_template": package_value
                .pointer("/lifecycle/webview/initial_url_template")
                .cloned()
                .unwrap_or_else(|| serde_json::json!("{url}index.html")),
            "pending_host_url": "{url}",
            "initial_url_preview": "{url}index.html",
            "reload_policy": package_value
                .pointer("/lifecycle/webview/reload_policy")
                .cloned()
                .unwrap_or_else(|| serde_json::json!("reload-panel-artifacts-after-refresh-event")),
        },
        "refresh": {
            "events": refresh_events,
        },
        "platform_matrix": platform_matrix,
        "source_permission_prompt": {
            "mode": source_permissions
                .get("mode")
                .cloned()
                .unwrap_or_else(|| serde_json::json!("prompt-before-source-reveal")),
            "default": source_permissions
                .get("default")
                .cloned()
                .unwrap_or_else(|| serde_json::json!("prompt-before-open")),
            "denied_mode": source_permissions
                .get("denied_mode")
                .cloned()
                .unwrap_or_else(|| serde_json::json!("open-read-only")),
            "reveal_requires_origin_id": source_permissions
                .get("reveal_requires_origin_id")
                .cloned()
                .unwrap_or(serde_json::Value::Bool(true)),
            "webview_injection": source_permissions
                .get("webview_injection")
                .cloned()
                .unwrap_or_else(|| serde_json::json!("orvNativeHostSourcePermissions")),
            "decision_event": source_permissions
                .get("decision_event")
                .cloned()
                .unwrap_or_else(|| serde_json::json!("orv:source-permission")),
            "blocked_event": source_permissions
                .get("blocked_event")
                .cloned()
                .unwrap_or_else(|| serde_json::json!("orv:source-permission-blocked")),
            "root_count": source_permissions
                .get("root_count")
                .cloned()
                .unwrap_or_else(|| serde_json::json!(0)),
            "source_count": source_permissions
                .get("source_count")
                .cloned()
                .unwrap_or_else(|| serde_json::json!(0)),
            "allowed_roots": source_permissions
                .get("allowed_roots")
                .cloned()
                .unwrap_or_else(|| serde_json::json!([])),
            "source_hashes": source_permissions
                .get("source_hashes")
                .cloned()
                .unwrap_or_else(|| serde_json::json!([])),
            "prompt": source_permissions
                .get("prompt")
                .cloned()
                .unwrap_or_else(|| serde_json::json!({})),
        },
        "artifact_checks": artifact_checks,
        "session_artifact": {
            "path": EDITOR_NATIVE_HOST_DESKTOP_SESSION_PATH,
            "kind": "orv.editor.native_host.desktop_shell",
        },
    }))
}

pub(crate) fn verify_editor_native_host_desktop_package_contract_keys(
    package: &serde_json::Value,
) -> anyhow::Result<()> {
    verify_json_object_keys_exact(
        package,
        &[
            "schema_version",
            "kind",
            "runtime",
            "entry",
            "export_root",
            "artifacts",
            "platform_matrix",
            "desktop_app",
            "packaging",
            "lifecycle",
            "process_policy",
            "refresh",
            "source_permissions",
        ],
        "desktop package",
    )?;
    if package
        .get("schema_version")
        .and_then(serde_json::Value::as_u64)
        != Some(1)
    {
        anyhow::bail!("desktop package schema_version must be 1");
    }
    if json_str(package, "kind", "desktop package")? != "orv.editor.native_host.desktop_package" {
        anyhow::bail!("desktop package kind must be orv.editor.native_host.desktop_package");
    }
    if json_str(package, "runtime", "desktop package")? != "local-http-bridge" {
        anyhow::bail!("desktop package runtime must be local-http-bridge");
    }
    verify_editor_native_host_desktop_platform_matrix_contract_keys(
        package
            .get("platform_matrix")
            .ok_or_else(|| anyhow::anyhow!("desktop package platform_matrix must be an object"))?,
    )?;
    verify_editor_native_host_desktop_source_permissions_contract_keys(
        package.get("source_permissions").ok_or_else(|| {
            anyhow::anyhow!("desktop package source_permissions must be an object")
        })?,
        "desktop source permissions",
    )
}

pub(crate) fn verify_editor_native_host_desktop_platform_matrix_contract_keys(
    matrix: &serde_json::Value,
) -> anyhow::Result<()> {
    verify_json_object_keys_exact(
        matrix,
        &[
            "schema_version",
            "kind",
            "default_platform",
            "implemented_count",
            "planned_count",
            "targets",
        ],
        "desktop platform_matrix",
    )?;
    if matrix
        .get("schema_version")
        .and_then(serde_json::Value::as_u64)
        != Some(1)
    {
        anyhow::bail!("desktop platform_matrix schema_version must be 1");
    }
    if json_str(matrix, "kind", "desktop platform_matrix")?
        != "orv.editor.native_host.desktop_platform_matrix"
    {
        anyhow::bail!(
            "desktop platform_matrix kind must be orv.editor.native_host.desktop_platform_matrix"
        );
    }
    let targets = matrix
        .get("targets")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| anyhow::anyhow!("desktop platform_matrix targets must be an array"))?;
    for (index, target) in targets.iter().enumerate() {
        let status = target
            .get("status")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("");
        if status == "implemented" {
            verify_json_object_keys_exact(
                target,
                &[
                    "platform",
                    "status",
                    "container",
                    "package",
                    "main",
                    "session_artifact",
                    "packaging",
                    "capabilities",
                    "verification",
                ],
                &format!("desktop platform_matrix targets[{index}]"),
            )?;
        } else if status == "planned" {
            verify_json_object_keys_exact(
                target,
                &[
                    "platform",
                    "status",
                    "container",
                    "blocked_by",
                    "shared_contracts",
                ],
                &format!("desktop platform_matrix targets[{index}]"),
            )?;
            let blockers = target
                .get("blocked_by")
                .and_then(serde_json::Value::as_array)
                .ok_or_else(|| {
                    anyhow::anyhow!(
                        "desktop platform_matrix targets[{index}].blocked_by must be an array"
                    )
                })?;
            if blockers.is_empty() {
                anyhow::bail!(
                    "desktop platform_matrix targets[{index}].blocked_by must be non-empty for planned target"
                );
            }
            for (blocker_index, blocker) in blockers.iter().enumerate() {
                verify_json_object_keys_exact(
                    blocker,
                    &["id", "reason"],
                    &format!(
                        "desktop platform_matrix targets[{index}].blocked_by[{blocker_index}]"
                    ),
                )?;
                if json_str(blocker, "id", "desktop platform blocker")?
                    .trim()
                    .is_empty()
                    || json_str(blocker, "reason", "desktop platform blocker")?
                        .trim()
                        .is_empty()
                {
                    anyhow::bail!(
                        "desktop platform_matrix targets[{index}].blocked_by[{blocker_index}] id and reason must be non-empty strings"
                    );
                }
            }
            let shared_contracts = target
                .get("shared_contracts")
                .and_then(serde_json::Value::as_array)
                .ok_or_else(|| {
                    anyhow::anyhow!(
                        "desktop platform_matrix targets[{index}].shared_contracts must be an array"
                    )
                })?;
            if shared_contracts.is_empty() {
                anyhow::bail!(
                    "desktop platform_matrix targets[{index}].shared_contracts must be non-empty for planned target"
                );
            }
            for (contract_index, contract) in shared_contracts.iter().enumerate() {
                if contract
                    .as_str()
                    .is_none_or(|contract| contract.trim().is_empty())
                {
                    anyhow::bail!(
                        "desktop platform_matrix targets[{index}].shared_contracts[{contract_index}] must be a non-empty string"
                    );
                }
            }
        } else {
            anyhow::bail!(
                "desktop platform_matrix targets[{index}].status must be implemented or planned"
            );
        }
    }
    Ok(())
}

pub(crate) fn verify_editor_native_host_desktop_source_permissions_contract_keys(
    permissions: &serde_json::Value,
    context: &str,
) -> anyhow::Result<()> {
    verify_json_object_keys_exact(
        permissions,
        &[
            "mode",
            "default",
            "denied_mode",
            "reveal_requires_origin_id",
            "webview_injection",
            "decision_event",
            "blocked_event",
            "root_count",
            "source_count",
            "allowed_roots",
            "source_hashes",
            "prompt",
        ],
        context,
    )?;
    verify_json_object_keys_exact(
        permissions
            .get("prompt")
            .ok_or_else(|| anyhow::anyhow!("{context} prompt must be an object"))?,
        &["title", "allow_label", "read_only_label", "quit_label"],
        &format!("{context}.prompt"),
    )
}

pub(crate) fn verify_editor_native_host_desktop_shell_contract_keys(
    session: &serde_json::Value,
) -> anyhow::Result<()> {
    verify_json_object_keys_exact(
        session,
        &[
            "schema_version",
            "kind",
            "status",
            "root",
            "package",
            "lifecycle",
            "process_supervision",
            "webview",
            "refresh",
            "platform_matrix",
            "source_permission_prompt",
            "artifact_checks",
            "session_artifact",
        ],
        "desktop shell",
    )?;
    if session
        .get("schema_version")
        .and_then(serde_json::Value::as_u64)
        != Some(1)
    {
        anyhow::bail!("desktop shell schema_version must be 1");
    }
    if json_str(session, "kind", "desktop shell")? != "orv.editor.native_host.desktop_shell" {
        anyhow::bail!("desktop shell kind must be orv.editor.native_host.desktop_shell");
    }
    verify_editor_native_host_desktop_platform_matrix_contract_keys(
        session
            .get("platform_matrix")
            .ok_or_else(|| anyhow::anyhow!("desktop shell platform_matrix must be an object"))?,
    )?;
    verify_editor_native_host_desktop_source_permissions_contract_keys(
        session.get("source_permission_prompt").ok_or_else(|| {
            anyhow::anyhow!("desktop shell source_permission_prompt must be an object")
        })?,
        "desktop shell source_permission_prompt",
    )
}

pub(crate) fn editor_native_host_desktop_package_input_path(package: &Path) -> PathBuf {
    if package.is_dir() {
        package.join(EDITOR_NATIVE_HOST_DESKTOP_PACKAGE_PATH)
    } else {
        package.to_path_buf()
    }
}

pub(crate) fn editor_native_host_desktop_package_root(
    package_path: &Path,
) -> anyhow::Result<PathBuf> {
    let package_path = package_path.canonicalize().map_err(|e| {
        anyhow::anyhow!(
            "failed to canonicalize desktop package {}: {e}",
            package_path.display()
        )
    })?;
    if package_path.file_name().and_then(std::ffi::OsStr::to_str) == Some("desktop-package.json")
        && package_path
            .parent()
            .and_then(Path::file_name)
            .and_then(std::ffi::OsStr::to_str)
            == Some("native-host")
    {
        return package_path
            .parent()
            .and_then(Path::parent)
            .map(Path::to_path_buf)
            .ok_or_else(|| anyhow::anyhow!("desktop package has no export root"));
    }
    package_path
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| anyhow::anyhow!("desktop package has no parent directory"))
}

pub(crate) fn editor_native_host_desktop_artifact_checks_json(
    root: &Path,
    package: &serde_json::Value,
) -> Vec<serde_json::Value> {
    let mut checks = Vec::new();
    if let Some(artifacts) = package
        .get("artifacts")
        .and_then(serde_json::Value::as_object)
    {
        for (name, path) in artifacts {
            let Some(path) = path.as_str() else {
                continue;
            };
            checks.push(serde_json::json!({
                "name": name,
                "path": path,
                "exists": root.join(path).is_file(),
            }));
        }
    }
    checks.sort_by(|left, right| {
        left.get("name")
            .and_then(serde_json::Value::as_str)
            .cmp(&right.get("name").and_then(serde_json::Value::as_str))
    });
    checks
}

pub(crate) fn editor_native_host_desktop_launch_command_json(
    root: &Path,
    package: &serde_json::Value,
    listen: &str,
) -> anyhow::Result<serde_json::Value> {
    let command = package
        .pointer("/lifecycle/spawn/command")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| anyhow::anyhow!("desktop package missing lifecycle.spawn.command"))?;
    let mut parts = Vec::with_capacity(command.len());
    let mut replace_listen = false;
    for value in command {
        let part = value
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("desktop lifecycle command must contain strings"))?;
        if replace_listen {
            parts.push(listen.to_string());
            replace_listen = false;
            continue;
        }
        if part == "--listen" {
            parts.push(part.to_string());
            replace_listen = true;
            continue;
        }
        if part == "." {
            parts.push(root.display().to_string());
        } else {
            parts.push(part.to_string());
        }
    }
    Ok(serde_json::json!(parts))
}

pub(crate) fn write_editor_native_host_desktop_session_if_configured(
    package: &Path,
    value: &serde_json::Value,
) -> anyhow::Result<bool> {
    let package_path = editor_native_host_desktop_package_input_path(package);
    let root = editor_native_host_desktop_package_root(&package_path)?;
    write_json(&root.join(EDITOR_NATIVE_HOST_DESKTOP_SESSION_PATH), value)?;
    Ok(true)
}

pub(crate) struct EditorDesktopRun {
    pub(crate) value: serde_json::Value,
    pub(crate) child: Child,
}

pub(crate) fn editor_native_host_desktop_spawn_run(
    input: &Path,
    listen: &str,
    open: bool,
) -> anyhow::Result<EditorDesktopRun> {
    let session = editor_native_host_desktop_run_session_json(input, listen)?;
    let command = session
        .pointer("/lifecycle/spawn/command")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| anyhow::anyhow!("desktop session missing lifecycle.spawn.command"))?;
    let parts = command
        .iter()
        .map(serde_json::Value::as_str)
        .collect::<Option<Vec<_>>>()
        .ok_or_else(|| anyhow::anyhow!("desktop session spawn command must contain strings"))?;
    let Some((program, args)) = parts.split_first() else {
        anyhow::bail!("desktop session spawn command is empty");
    };
    let mut child = ProcessCommand::new(program)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| anyhow::anyhow!("failed to spawn desktop host `{program}`: {e}"))?;
    let mut stdout = child
        .stdout
        .take()
        .ok_or_else(|| anyhow::anyhow!("desktop host stdout was not captured"))?;
    let host_ready = match read_first_json_value_from_reader(&mut stdout) {
        Ok(value) => value,
        Err(error) => {
            let _ = child.kill();
            let _ = child.wait();
            return Err(error);
        }
    };
    let webview_url = editor_native_host_desktop_webview_url(&session, &host_ready);
    let open_status = if open {
        Some(editor_native_host_open_url(&webview_url)?)
    } else {
        None
    };
    let child_id = child.id();
    Ok(EditorDesktopRun {
        value: serde_json::json!({
            "schema_version": 1,
            "kind": "orv.editor.native_host.desktop_run",
            "status": "ready",
            "probe_supported": true,
            "host": host_ready,
            "process": {
                "pid": child_id,
                "supervision": session
                    .get("process_supervision")
                    .cloned()
                    .unwrap_or_else(|| serde_json::json!({})),
            },
            "webview": {
                "url": webview_url,
                "open_requested": open,
                "open_status": open_status,
            },
            "source_permission_prompt": session
                .get("source_permission_prompt")
                .cloned()
                .unwrap_or_else(|| serde_json::json!({})),
            "refresh": session
                .get("refresh")
                .cloned()
                .unwrap_or_else(|| serde_json::json!({})),
            "session": session,
        }),
        child,
    })
}

#[cfg(test)]
pub(crate) fn editor_native_host_desktop_run_probe_json(
    input: &Path,
    listen: &str,
) -> anyhow::Result<serde_json::Value> {
    let mut run = editor_native_host_desktop_spawn_run(input, listen, false)?;
    let _ = run.child.kill();
    let _ = run.child.wait();
    run.value["status"] = serde_json::json!("probe_ready");
    Ok(run.value)
}

pub(crate) fn editor_native_host_desktop_run_session_json(
    input: &Path,
    listen: &str,
) -> anyhow::Result<serde_json::Value> {
    let input_path = if input.is_dir() {
        let session = input.join(EDITOR_NATIVE_HOST_DESKTOP_SESSION_PATH);
        if session.is_file() {
            session
        } else {
            return editor_native_host_desktop_shell_json(input, listen);
        }
    } else {
        input.to_path_buf()
    };
    let value = read_json_value(&input_path)?;
    match value.get("kind").and_then(serde_json::Value::as_str) {
        Some("orv.editor.native_host.desktop_shell") => {
            verify_editor_native_host_desktop_shell_contract_keys(&value)?;
            Ok(editor_native_host_desktop_session_with_listen(
                value, listen,
            ))
        }
        Some("orv.editor.native_host.desktop_package") => {
            editor_native_host_desktop_shell_json(&input_path, listen)
        }
        kind => Err(anyhow::anyhow!(
            "unsupported desktop run input kind {:?} in {}",
            kind,
            input_path.display()
        )),
    }
}

pub(crate) fn editor_native_host_desktop_session_with_listen(
    mut session: serde_json::Value,
    listen: &str,
) -> serde_json::Value {
    if let Some(command) = session
        .pointer_mut("/lifecycle/spawn/command")
        .and_then(serde_json::Value::as_array_mut)
    {
        let mut replace_listen = false;
        for part in command {
            if replace_listen {
                *part = serde_json::json!(listen);
                replace_listen = false;
                continue;
            }
            replace_listen = part.as_str() == Some("--listen");
        }
    }
    session
}

pub(crate) fn editor_native_host_desktop_webview_url(
    session: &serde_json::Value,
    host_ready: &serde_json::Value,
) -> String {
    let base_url = host_ready
        .get("url")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("{url}");
    session
        .pointer("/webview/initial_url_template")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("{url}index.html")
        .replace("{url}", base_url)
}
