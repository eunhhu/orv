# 컴파일러 힌트

[← 목차로 돌아가기](./index.ko.md)

---

`@hint`는 기본 동작이 프로젝트에 적합하지 않을 때 컴파일러 최적화 결정을 재정의할 수 있게 해줍니다.

## 문법

```orv
// 라우트 수준 힌트
let getUsers = @route GET /api/users @hint protocol=json { ... }
@route GET /admin @hint chunk=separate { ... }

// HTML 렌더 전략
pub define DashboardPage() -> @html @hint render=ssr { ... }

// Fetch 및 데이터 레이어 동작
let livePrice = await getPrice.fetch() @hint cache=never
let report = await getReport.fetch() @hint prefetch=never

// import 보존
@hint keep
import libs.analytics
```

## 지원되는 힌트

| 지시자 | 대상 | 값 |
|-----------|--------|--------|
| `@hint protocol=` | route | `json`, `binary`, `hybrid` |
| `@hint render=` | `@html` 페이지 / define | `ssr`, `csr`, `ssg` |
| `@hint cache=` | fetch / query | `never`, `immutable`, `60s` 같은 TTL 값 |
| `@hint prefetch=` | fetch | `never`, `eager` |
| `@hint chunk=` | route / module | `separate`, `inline` |
| `@hint keep` | import | 트리 셰이킹 방지 |

## 가이드

- 먼저 컴파일러 기본값을 선호하세요. 특정 재정의가 필요할 때만 `@hint`를 추가하세요.
- 최적화 의도가 명확하도록 힌트를 영향을 미치는 선언에 가까이 두세요.
- `@hint`는 명확한 도메인 구조의 대체가 아닌 재정의로 취급하세요.

전체 최적화 모델과 근거에 대해서는 [프로젝트 최적화 설계](../../specs/2026-04-03-project-optimization-design.md)를 참조하세요.
