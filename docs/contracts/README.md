# Public Artifact Contracts

These files describe JSON shapes that are intentionally consumed across CLI,
editor, native-host, smoke, and reveal surfaces. They are narrower than the
language spec: the goal is to freeze artifact keys, primitive types, ordering
rules, and version policy so generated output can drift only through an explicit
contract change.

Current contracts:

- [ProjectGraph v1](PROJECT_GRAPH_V1.md)
- [OriginMap v2](ORIGIN_MAP_V2.md)
- [Check CLI v1](CHECK_CLI_V1.md)
- [Deploy Artifacts v1](DEPLOY_ARTIFACTS_V1.md)
- [Runtime Trace v1](RUNTIME_TRACE_V1.md)
- [Runtime CLI v1](RUNTIME_CLI_V1.md)
- [Request State v1](REQUEST_STATE_V1.md)
- [Validation Error Response v1](VALIDATION_ERROR_RESPONSE_V1.md)
- [Route Origin Headers v1](ROUTE_ORIGIN_HEADERS_V1.md)
- [Native Host Desktop v1](NATIVE_HOST_DESKTOP_V1.md)
- [Client Bundle v1](CLIENT_BUNDLE_V1.md)
- [Native Server Plan v1](NATIVE_SERVER_PLAN_V1.md)
- [DAP Debug Session v1](DAP_DEBUG_SESSION_V1.md)
- [Reveal Payload v1](REVEAL_PAYLOAD_V1.md)
- [LSP Bootstrap v1](LSP_BOOTSTRAP_V1.md)
- [Test Runner v1](TEST_RUNNER_V1.md)
- [Editor Snapshot/Export v1](EDITOR_SNAPSHOT_EXPORT_V1.md)
- [Editor Trace v1](EDITOR_TRACE_V1.md)
