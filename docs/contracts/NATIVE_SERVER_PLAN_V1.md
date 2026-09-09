# Native Server Plan v1 Contract

Producer:

- `orv build <file-or-project> --out <dir>` for server entries
- `orv build <file-or-project> --prod --out <dir>` mirrors the same native
  plan through deploy, reveal, editor, LSP, DAP, and native-host production
  summaries

Current regression coverage:

- `crates/orv-cli/tests/native_server_contract.rs::native_server_plan_and_runtime_image_contract_freezes_public_shape`
- `crates/orv-cli/src/tests/build.rs::build_writes_manifest_origin_map_and_project_graph`
- `crates/orv-cli/src/tests/native.rs::build_writes_native_runtime_image_plan_contract`
- `crates/orv-cli/src/tests/native.rs::build_writes_native_server_routes_source_contract`
- `crates/orv-cli/src/tests/native.rs::build_writes_native_server_router_source_contract`
- `crates/orv-cli/src/tests/native.rs::build_writes_native_server_handler_source_contract`
- `crates/orv-cli/src/tests/native.rs::build_uses_reference_native_launcher_for_dynamic_handlers`
- `crates/orv-cli/src/tests/verify_native.rs::verify_native_artifact_cases` (case `native_runtime_image_dockerfile_mismatch`)
- `crates/orv-cli/src/tests/verify_native.rs` also retains independent command,
  routes source and launcher package mismatch tests.

This contract covers the public native server plan, runtime image plan, and
generated Rust launcher/source file surface, including build-manifest and
bundle-plan links to every native plan/image/source/package artifact. The final
optimized native server runtime is still planned; v1 freezes the current
artifact contract so editor, reveal, smoke, and deploy tooling can consume it
without inferring shape.

## Native Server Plan Root

`server/native-server.json` has exactly:

```json
{
  "schema_version": 1,
  "kind": "native_server_plan",
  "status": "direct_http",
  "runtime": "reference-interpreter",
  "runtime_features": [],
  "artifact": "server/app.orv-runtime.json",
  "launcher": "server/launch.json",
  "source": "server/native/main.rs",
  "routes_source": "server/native/routes.rs",
  "router_source": "server/native/router.rs",
  "handlers_source": "server/native/handlers.rs",
  "package": "server/native/Cargo.toml",
  "runtime_image_plan": "server/runtime-image.json",
  "target": {},
  "commands": {},
  "blocked_by": [],
  "listen": {},
  "routes": []
}
```

Rules:

- `schema_version` is currently `1`.
- `kind` is `native_server_plan`.
- `status` is `direct_http` when all routes are directly lowered by the
  generated native launcher. Dynamic fallback artifacts use `planned`.
- `artifact`, `launcher`, `source`, route/router/handler source paths,
  `package`, and `runtime_image_plan` are build-directory relative paths.
- `runtime_features`, `listen`, and `routes` mirror the server runtime artifact.
- `blocked_by` is empty for direct-lowered launchers. Dynamic fallback launchers
  keep `native-codegen` and `native-runtime-image`.

## Target

`target` has exactly:

```json
{
  "kind": "server_binary",
  "path": "server/app",
  "protocol": "http1"
}
```

Rules:

- `path` is the planned final native binary path. The generated Rust package
  currently builds `server/native/target/release/orv-native-server`; runtime
  image packaging copies that binary to `server/app`.

## Commands

`commands` has exactly:

```json
{
  "build": [
    "cargo",
    "build",
    "--manifest-path",
    "server/native/Cargo.toml",
    "--release"
  ],
  "run": {
    "env": {
      "ORV_BUILD_DIR": "."
    },
    "command": [
      "./server/native/target/release/orv-native-server"
    ]
  }
}
```

Rules:

- `build` is the generated native launcher package build argv.
- `run.env.ORV_BUILD_DIR` points at the build directory. The launcher can also
  infer the build directory when run from the generated package target path.
- `orv verify-build` rejects build/run command drift.

## Routes

Each `routes[]` entry reuses the server runtime artifact route descriptor. For a
simple static response route, the public keys are exactly:

```json
{
  "method": "GET",
  "path": "/ping",
  "origin_id": "ori_...",
  "response_origin_ids": [],
  "responses": []
}
```

Each `policies[]` entry is a server runtime policy descriptor. When present, it
includes `kind` and `surface`. Source-authored first-party compiler plugin
policies use `surface: "first_party_compiler_plugin"` and carry an `origin_id`;
shop/provider-template defaults use `shop_template` or
`provider_package_template` and may omit `origin_id`.

Each simple static response entry has exactly:

```json
{
  "origin_id": "ori_...",
  "status": 200,
  "body_kind": "static_json",
  "body_json": "{\"ok\":true}"
}
```

Rules:

- `origin_id` and `response_origin_ids[]` resolve through OriginMap v2.
- `responses[]` records source-backed `@respond` lowering. More complex
  response body variants may add the optional server runtime response fields;
  the route descriptor owner remains the server runtime artifact.
- `orv verify-build` rejects route/listen/response origin drift and native route
  source drift.

## Runtime Image Plan Root

`server/runtime-image.json` has exactly:

```json
{
  "schema_version": 1,
  "kind": "native_runtime_image_plan",
  "status": "image_planned",
  "runtime": "reference-interpreter",
  "runtime_features": [],
  "artifact": "server/app.orv-runtime.json",
  "native_plan": "server/native-server.json",
  "reference_image": "ghcr.io/orv-lang/orv-reference:latest",
  "target": {},
  "dockerfile": "server/native/Dockerfile",
  "commands": {},
  "blocked_by": [],
  "listen": {},
  "routes": []
}
```

Rules:

- `kind` is `native_runtime_image_plan`.
- `status` is `image_planned` for direct-lowered launchers and `planned` for
  dynamic fallback launchers.
- `listen` and `routes` must match `server/native-server.json`.
- `blocked_by` follows the native server plan blocker rule.

## Runtime Image Target And Commands

`target` has exactly:

```json
{
  "kind": "oci_image",
  "image": "orv-native-server:latest",
  "binary": "server/app",
  "protocol": "http1"
}
```

`commands` has exactly:

```json
{
  "build": [
    "docker",
    "build",
    "-f",
    "server/native/Dockerfile",
    "-t",
    "orv-native-server:latest",
    "."
  ]
}
```

Rules:

- The generated Dockerfile must build the Rust launcher package and copy the
  generated native launcher binary to `/app/server/app`.
- `orv verify-build` rejects runtime image plan and Dockerfile drift.

## Generated Source Files

The native source surface consists of:

- `server/native/Cargo.toml`
- `server/native/main.rs`
- `server/native/routes.rs`
- `server/native/router.rs`
- `server/native/handlers.rs`
- `server/native/Dockerfile`

Rules:

- `Cargo.toml` package name is `orv-native-server`; the binary path is
  `main.rs`.
- `main.rs` validates the build directory, native plan, and server runtime
  artifact before serving or falling back to the reference runner.
- `routes.rs` exposes typed route descriptors, policy descriptors, response
  origin ids, and `orv_native_match_route`.
- `router.rs` exposes dispatch structs and delegates matched routes to
  `handlers::orv_native_handle_route`.
- `handlers.rs` exposes handler descriptors and native response lowering for
  supported route slices.
- Direct-lowered launchers use a dependency-free HTTP/1 loop. Dynamic fallback
  launchers keep the reference `orv run-artifact` bridge.
- `orv verify-build` regenerates and compares source/package files against the
  server runtime artifact.

## Version Policy

Native Server Plan v1 is public to deploy artifacts, generated smoke tests,
reveal/editor/LSP production payloads, DAP production summaries, and native-host
production panels. Breaking key/type changes require a schema version bump or
documented compatibility bridge plus updates to this file, changelog, and
contract regression.
