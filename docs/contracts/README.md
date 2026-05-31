# Public Artifact Contracts

These files describe JSON shapes that are intentionally consumed across CLI,
editor, native-host, smoke, and reveal surfaces. They are narrower than the
language spec: the goal is to freeze artifact keys, primitive types, ordering
rules, and version policy so generated output can drift only through an explicit
contract change.

Shop/commerce contracts in this directory freeze benchmark and library/provider
surfaces. They do not promote payment, shipping, Stripe, carrier, or shop
business policy to compiler core intrinsics; that boundary is defined in
[`../PLATFORM_BOUNDARY.md`](../PLATFORM_BOUNDARY.md).

Current contracts:

- [Core Spine v1](CORE_SPINE_V1.md)
- [ProjectGraph v1](PROJECT_GRAPH_V1.md)
- [OriginMap v2](ORIGIN_MAP_V2.md)
- [Check CLI v1](CHECK_CLI_V1.md)
- [Compiler Pipeline v1](COMPILER_PIPELINE_V1.md)
- [Compiler Plugin Boundary v1](COMPILER_PLUGIN_BOUNDARY_V1.md)
- [Build Artifacts v1](BUILD_ARTIFACTS_V1.md)
- [Deploy Artifacts v1](DEPLOY_ARTIFACTS_V1.md)
- [HTTP Server v1](HTTP_SERVER_V1.md)
- [Runtime Trace v1](RUNTIME_TRACE_V1.md)
- [Runtime CLI v1](RUNTIME_CLI_V1.md)
- [Request State v1](REQUEST_STATE_V1.md)
- [Request Bindings v1](REQUEST_BINDINGS_V1.md)
- [Validation Error Response v1](VALIDATION_ERROR_RESPONSE_V1.md)
- [DB Persistence v1](DB_PERSISTENCE_V1.md)
- [DB Adapters v1](DB_ADAPTERS_V1.md)
- [Route Origin Headers v1](ROUTE_ORIGIN_HEADERS_V1.md)
- [HTML Render v1](HTML_RENDER_V1.md)
- [Shop Template v1](SHOP_TEMPLATE_V1.md)
- [Shop Acceptance Smoke v1](SHOP_ACCEPTANCE_SMOKE_V1.md)
- [Shop Security Boundaries v1](SHOP_SECURITY_BOUNDARIES_V1.md)
- [Shop Checkout Resilience v1](SHOP_CHECKOUT_RESILIENCE_V1.md)
- [Commerce Adapters v1](COMMERCE_ADAPTERS_V1.md)
- [Commerce Provider Hardening v1](COMMERCE_PROVIDER_HARDENING_V1.md)
- [Native Host Desktop v1](NATIVE_HOST_DESKTOP_V1.md)
- [Client Bundle v1](CLIENT_BUNDLE_V1.md)
- [Native Server Plan v1](NATIVE_SERVER_PLAN_V1.md)
- [DAP Debug Session v1](DAP_DEBUG_SESSION_V1.md)
- [Reveal Payload v1](REVEAL_PAYLOAD_V1.md)
- [LSP Bootstrap v1](LSP_BOOTSTRAP_V1.md)
- [Test Runner v1](TEST_RUNNER_V1.md)
- [Editor Snapshot/Export v1](EDITOR_SNAPSHOT_EXPORT_V1.md)
- [Editor Trace v1](EDITOR_TRACE_V1.md)
