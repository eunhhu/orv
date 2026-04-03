# Compiler Hints

[← Back to Index](./index.md)

---

`@hint` lets you override compiler optimization decisions when the default behavior is not what your project needs.

## Syntax

```orv
// Route-level hints
let getUsers = @route GET /api/users @hint protocol=json { ... }
@route GET /admin @hint chunk=separate { ... }

// HTML render strategy
pub define DashboardPage() -> @html @hint render=ssr { ... }

// Fetch and data-layer behavior
let livePrice = await getPrice.fetch() @hint cache=never
let report = await getReport.fetch() @hint prefetch=never

// Import preservation
@hint keep
import libs.analytics
```

## Supported Hints

| Directive | Target | Values |
|-----------|--------|--------|
| `@hint protocol=` | route | `json`, `binary`, `hybrid` |
| `@hint render=` | `@html` page / define | `ssr`, `csr`, `ssg` |
| `@hint cache=` | fetch / query | `never`, `immutable`, TTL value such as `60s` |
| `@hint prefetch=` | fetch | `never`, `eager` |
| `@hint chunk=` | route / module | `separate`, `inline` |
| `@hint keep` | import | prevent tree-shaking |

## Guidance

- Prefer the compiler defaults first. Add `@hint` only when you need a specific override.
- Keep hints close to the declaration they affect so the optimization intent is obvious.
- Treat `@hint` as an override, not a substitute for clear domain structure.

For the full optimization model and rationale, see [Project Optimization Design](../../specs/2026-04-03-project-optimization-design.md).
