# Public Artifact Contracts

These files describe JSON shapes that are intentionally consumed across CLI,
editor, native-host, smoke, and reveal surfaces. They are narrower than the
language spec: the goal is to freeze artifact keys, primitive types, ordering
rules, and version policy so generated output can drift only through an explicit
contract change.

Current contracts:

- [ProjectGraph v1](PROJECT_GRAPH_V1.md)
- [OriginMap v2](ORIGIN_MAP_V2.md)
- [Deploy Artifacts v1](DEPLOY_ARTIFACTS_V1.md)
- [Runtime Trace v1](RUNTIME_TRACE_V1.md)
- [Validation Error Response v1](VALIDATION_ERROR_RESPONSE_V1.md)

