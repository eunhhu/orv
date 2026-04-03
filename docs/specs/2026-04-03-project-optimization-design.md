# orv Project Optimization — Design Spec

> Date: 2026-04-03
> Status: Draft
> Author: sunwoo + Claude

---

## Overview

orv의 핵심 차별점: **컴파일러가 전체 프로젝트(UI, 서버, 디자인, 데이터)를 한눈에 보고, 프로젝트 목적에 맞게 전 레이어를 최적화한다.** 개발자는 코드만 작성하면 나머지는 컴파일러와 런타임이 전부 처리한다.

### Design Decisions

| 결정 항목 | 선택 |
|-----------|------|
| 최적화 대상 | 통신 + 번들 + 렌더링 + 데이터 전 레이어 |
| 시점 | 컴파일 타임(정적 분석) + 런타임 어댑티브 |
| 통신 호환성 | 하이브리드 — 내부 RPC 최적화, 외부 API는 HTTP/JSON |
| 번들 단위 | 명령어 단위 (바이트 레벨 dead code elimination) |
| 런타임 범위 | 인프라 + 렌더링 전략 + 데이터 레이어 |
| 개발자 제어 | 자동 + `@hint` 디렉티브로 오버라이드 |
| 컴파일러 구조 | 단일 패스 (Analyzer → Optimizer → CodeGen → AdaptiveRuntime) |

### Guiding Principle

> 개발자가 "이런 것까지 알아서 다 해준다고?" 싶을 정도로 컴파일러가 떠먹여준다.

---

## 1. Compiler Pipeline

```
orv source → Parse → Analyze → Optimize → CodeGen
                         │
                    ProjectGraph
```

### 1.1 ProjectGraph

모든 최적화의 기반이 되는 전체 프로젝트 분석 결과.

| 카테고리 | 수집 내용 |
|---------|----------|
| 도메인 사용 | 어떤 도메인(`@html`, `@server`, `@design`)이 존재하는지 |
| 노드 사용 | 실제 사용된 노드, 프로퍼티, 함수, 타입 전체 목록 |
| 도메인 간 참조 | 서버가 어떤 html을 serve하는지, RPC 변수 호출 위치 |
| 데이터 흐름 | `@body` → 변수 → `@response` 타입 체인 |
| 시그널 의존성 | 어떤 sig가 어떤 UI 노드에 바인딩되어 있는지 |
| 라우트 맵 | 전체 라우트 트리, 미들웨어 체인, 파라미터 타입 |
| fetch 그래프 | 모든 `.fetch()` 호출과 DB 쿼리의 의존/독립 관계 |

### 1.2 Optimize 서브시스템

ProjectGraph를 입력으로 받아 4개 서브시스템이 각각 최적화 결정을 내린다:

- **ProtocolOptimizer** — 통신 프로토콜, 직렬화, HTTP 메타데이터
- **BundleOptimizer** — dead code elimination, 번들 분리, 코드 스플리팅
- **RenderOptimizer** — 페이지별 SSR/CSR/SSG 전략
- **DataOptimizer** — 캐싱, 프리페칭, 배칭, 병렬화

### 1.3 CodeGen

최적화 결정에 따라:
- 타겟별(WASM/네이티브) 코드 생성
- 런타임 어댑터 코드 emit
- JS 브릿지, CSS, HTML 쉘 생성

---

## 2. Protocol Optimization (ProtocolOptimizer)

### 2.1 라우트 분류

```
let x = @route ...  →  변수에 할당됨  →  "내부 RPC"
@route GET /api/... →  변수 미할당     →  "외부 API"
```

| 분류 | 직렬화 | 전송 | 클라이언트 접근 |
|------|--------|------|----------------|
| 내부 RPC | 바이너리 (스키마 기반) | 멀티플렉싱, 단일 커넥션 | orv 클라이언트만 (`.fetch()`) |
| 외부 API | JSON | 표준 HTTP | 어디서든 (curl, 브라우저 등) |

### 2.2 내부 RPC 바이너리 직렬화

컴파일러가 `@response`의 타입 구조를 알고 있으므로, 필드 이름 없이 순서 기반 바이너리 인코딩을 자동 생성한다.

```orv
let getUsers = @route GET /api/users {
  return @response 200 { "users": await db.findAll() }
}
```

컴파일러가 생성하는 것:
- **서버 측**: `Vec<User>` → 바이너리 인코더
- **클라이언트 측**: 바이너리 → `Vec<User>` 디코더
- 양쪽 스키마가 컴파일 타임에 일치하므로 버전 불일치 불가능

### 2.3 라우트 그룹 Facade

라우트 그룹을 변수에 할당하면, 클라이언트에서 상대 경로로 하위 라우트에 접근한다.

```orv
@server {
  let api = @route /api {
    @route GET /user {
      return @response 200 { user: await db.find() }
    }
    @route GET /posts {
      return @response 200 { posts: await db.findPosts() }
    }
  }

  @route GET * {
    return @html {
      @body {
        let sig data = await api.fetch("/user")
        @text "{data}"
      }
    }
  }
}
```

| 패턴 | 용도 |
|------|------|
| `let x = @route GET /path` | 단일 엔드포인트 RPC — `x.fetch()` |
| `let api = @route /group { ... }` | 그룹 facade — `api.fetch("/sub")` 상대 경로 |

컴파일러가 그룹 facade에서:
- `api.fetch("/user")` → `/api/user` GET 라우트 존재 검증
- 존재하지 않는 경로 → **컴파일 에러**
- 응답 타입은 하위 라우트의 `@response`에서 추론
- 내부 RPC 최적화 동일 적용

### 2.4 HTTP 메타데이터 자동 결정

컴파일러가 `@response`와 `@serve`의 반환 타입을 분석해서 Content-Type, Content-Length 등을 컴파일 타임에 확정한다. 런타임에 MIME 스니핑이나 헤더 조립이 없다.

| 반환 표현 | 컴파일러가 결정하는 헤더 |
|-----------|----------------------|
| `@response 200 { "users": users }` | `Content-Type: application/json` 고정 |
| `@serve ./public` | 확장자 → MIME 매핑 테이블 빌드 타임 생성 |
| `@serve ./image.png` | `Content-Type: image/png`, `Content-Length` 파일 크기로 확정 |
| `@serve htmlNode` | `Content-Type: text/html; charset=utf-8` 고정 |
| `@response 204 {}` | `Content-Length: 0`, body 인코더 제거 |

추가 최적화:
- 바이트 배열로 미리 직렬화된 헤더를 emit (런타임 조립 없음)
- `@before`가 없는 라우트는 미들웨어 디스패치 자체를 건너뜀

### 2.5 오버라이드

```orv
// 내부 RPC지만 강제로 JSON
let getUsers = @route GET /api/users @hint protocol=json { ... }

// 외부 API지만 바이너리도 지원 (content negotiation)
@route GET /api/public/data @hint protocol=hybrid { ... }
```

---

## 3. Bundle Optimization (BundleOptimizer)

### 3.1 Dead Code Elimination

엔트리 포인트(main.orv)에서 도달 가능한(reachable) 코드만 남기고 전부 제거.

```
엔트리 포인트 (main.orv)
  → import 트리 추적
    → 각 모듈에서 실제 호출된 함수/노드/타입만 마킹
      → 마킹 안 된 것 전부 제거
```

| 레벨 | 예시 |
|------|------|
| 도메인 런타임 | `@html` 없음 → UI 런타임, 시그널, DOM 조작 전부 제거 |
| 기능 런타임 | `sig` 없음 → 리액티비티 추적기 제거. `when` 없음 → 패턴매칭 코드젠 생략 |
| 노드 런타임 | `@vstack` 미사용 → 레이아웃 헬퍼 제거 |
| 라이브러리 | `import @std.collections.Vec` → `HashMap` 런타임 미포함 |
| 함수/타입 | 정의됐지만 어디서도 사용 안 되면 제거 |

### 3.2 서버/클라이언트 번들 분리

단일 `.orv` 파일에 `@server`와 `@html`이 공존할 때 자동으로 두 번들 생성:

```
dist/
├── server                      (네이티브 바이너리 또는 server.wasm)
├── public/
│   ├── index.html              (HTML 쉘 — @head에서 생성)
│   ├── app.[hash].js           (JS 브릿지 — WASM 로더 + DOM 바인딩)
│   ├── app.[hash].wasm         (클라이언트 로직)
│   ├── style.[hash].css        (@design 토큰 + Tailwind → 컴파일된 CSS)
│   ├── chunk-home.[hash].js    (페이지별 청크)
│   ├── chunk-home.[hash].wasm
│   └── assets/                 (정적 파일)
```

**핵심 규칙**: 서버 번들에 UI 코드 없음, 클라이언트 번들에 DB/미들웨어 코드 없음. RPC 타입 스키마만 양쪽에 공유.

### 3.3 클라이언트 빌드 출력

| 출력 | 생성 소스 | 역할 |
|------|----------|------|
| `index.html` | `@html > @head` | WASM/JS/CSS 로드하는 최소 HTML 쉘 |
| `app.[hash].js` | 컴파일러 자동 생성 | WASM 초기화, DOM API 브릿지, 이벤트 바인딩 |
| `app.[hash].wasm` | `@html > @body` UI 로직 | 시그널, 렌더링, 상태 관리 |
| `style.[hash].css` | `@design` + 사용된 Tailwind | 사용된 클래스만 포함된 최소 CSS |

**JS 브릿지**:
- WASM → DOM 호출 바인딩 (실제 사용된 DOM API만 포함)
- 이벤트 → WASM 콜백 전달
- 청크 lazy loading (라우트 전환 시)

**CSS 생성**:
- `@design` 토큰 → CSS 커스텀 프로퍼티 (`@color primary #1a1a1a` → `--color-primary: #1a1a1a`)
- `@theme light/dark` → `@media (prefers-color-scheme)` 또는 class 스코프
- Tailwind 클래스는 컴파일러가 직접 추출 (별도 PurgeCSS 불필요)

### 3.4 코드 스플리팅

라우트 기반 자동 분리:
- `@route GET /` → `chunk-home.[hash].wasm`
- `@route GET /about` → `chunk-about.[hash].wasm`
- 두 페이지가 같은 `define` 사용 → `chunk-shared.[hash].wasm`으로 추출

### 3.5 서버의 dist serve

```orv
@server {
  @route GET / {
    @serve HomePage()
  }
}
```

컴파일러가 자동 처리:
1. `index.html` 응답 (Content-Type 컴파일 타임 확정)
2. `<script src="/app.[hash].js">` 자동 삽입
3. `/public/*` 정적 파일 라우트 자동 등록

개발자는 `dist/` 구조를 인지할 필요 없이 `@serve`만 작성.

### 3.6 오버라이드

```orv
// tree-shake 방지
@hint keep
import libs.analytics

// 별도 청크로 강제 분리
@route GET /admin @hint chunk=separate { ... }
```

---

## 4. Render Optimization (RenderOptimizer)

### 4.1 자동 전략 결정

개발자는 SSR/CSR/SSG를 선택하지 않는다. 컴파일러가 코드 패턴에서 자동 결정.

| 코드 패턴 | 컴파일러 판단 | 전략 |
|-----------|-------------|------|
| `sig` 없음, `await` 없음, 정적 노드만 | 절대 안 바뀜 | **SSG** — 빌드 시 HTML 완성 |
| `sig` 있지만 서버 데이터 의존 없음 | 초기 렌더에 서버 불필요 | **CSR** — 빈 쉘 + WASM |
| `await`로 서버 데이터 가져와서 초기 UI 구성 | 첫 페인트에 데이터 필요 | **SSR** — 서버 HTML + hydration |
| SSR인데 데이터가 거의 안 바뀜 | 매번 렌더링 낭비 | **SSR+캐시** — TTL 기반 캐싱 |

### 4.2 코드 예시 — 동일 문법, 다른 전략

```orv
// → SSG (정적 콘텐츠만)
pub define AboutPage() -> @html {
  @body {
    @h1 "About Us"
    @p "We build things."
  }
}

// → CSR (클라이언트 상태만)
pub define CounterPage() -> @html {
  @body {
    let sig count: i32 = 0
    @text "{count}"
    @button "+" %onClick={count += 1}
  }
}

// → SSR (서버 데이터 의존)
pub define DashboardPage() -> @html {
  @body {
    let sig users = await getUsers.fetch()
    for user of users {
      @text "{user.name}"
    }
  }
}
```

개발자는 세 페이지를 같은 문법으로 작성. SSR/CSR/SSG라는 단어가 코드에 등장하지 않는다.

### 4.3 런타임 어댑티브 전환

| 관찰 | 전환 |
|------|------|
| SSR 페이지 응답이 1시간 동안 동일 | SSR → SSR+캐시 (자동 TTL) |
| SSR+캐시 데이터가 자주 바뀌기 시작 | TTL 축소 또는 SSR 복귀 |
| CSR 페이지 FCP가 느림 | 빌드 리포트로 SSR 승격 제안 |

**범위**: 같은 전략 계열 안에서만 자동 전환 (SSR ↔ SSR+캐시). CSR ↔ SSR 같은 근본 전환은 재빌드 필요 → 리포트로 제안.

### 4.4 오버라이드

```orv
// 강제 SSR
pub define AlwaysSSRPage() -> @html @hint render=ssr { ... }

// 강제 SSG (빌드 시 데이터 fetch)
pub define BlogPost() -> @html @hint render=ssg {
  @body {
    let post = await getPost.fetch()  // 빌드 타임에 실행됨
    @h1 "{post.title}"
  }
}
```

---

## 5. Data Layer Optimization (DataOptimizer)

### 5.1 컴파일 타임 정적 분석

| 분석 대상 | 수집 정보 | 최적화 |
|-----------|----------|--------|
| 같은 스코프 내 다중 fetch | 독립적인 fetch 2개 이상 | 자동 병렬화 (`await (a, b)` 튜플 코드젠) |
| 순차 의존 fetch | A의 결과가 B의 파라미터 | 의존 체인 보존, 프리페칭 힌트 생성 |
| 반복 fetch | for 루프 안에서 N번 호출 | 자동 배칭 (단일 batch 요청으로 치환) |
| 라우트 간 공유 데이터 | 여러 페이지가 같은 fetch 호출 | 공유 캐시 키, 라우트 전환 시 재사용 |
| 서버 DB 쿼리 | 독립적인 쿼리 2개 이상 | 자동 병렬 실행 |

### 5.2 자동 배칭 예시

```orv
// 개발자가 작성한 코드
@ul {
  for userId of userIds {
    let user = await getUser.fetch(param={ id: userId })
    @li "{user.name}"
  }
}

// 컴파일러 내부 변환 (개발자에게 보이지 않음):
// → getUserBatch([...userIds]) 단일 요청으로 치환
// → 서버 측에도 배치 핸들러 자동 생성
```

### 5.3 서버 측 병렬 실행

```orv
@route GET /api/dashboard {
  // 컴파일러가 두 쿼리의 독립성을 감지 → 자동 병렬
  let users = await db.findUsers()
  let stats = await db.getStats()

  return @response 200 {
    "users": users,
    "stats": stats
  }
}
```

개발자가 순차적으로 작성해도, 의존 관계가 없으면 병렬로 실행.

### 5.4 런타임 어댑티브

| 관찰 | 자동 조절 |
|------|----------|
| 동일 파라미터로 5초 내 재호출 | 인메모리 캐시 활성화, TTL 자동 설정 |
| 캐시 데이터가 stale | TTL 단축 또는 캐시 무효화 |
| `/users` → `/users/42` 이동 패턴 반복 | 상위 N개 상세 프리페치 |
| API 응답 시간 증가 | 커넥션 풀 확장, 타임아웃 조절 |
| 동일 WHERE 조건 쿼리 반복 | 쿼리 결과 캐시 활성화 |

### 5.5 오버라이드

```orv
// 캐시 금지 (실시간 데이터)
let livePrice = await getPrice.fetch() @hint cache=never

// 불변 데이터 (빌드 타임 고정)
let countries = await getCountries.fetch() @hint cache=immutable

// 프리페치 비활성화 (비용이 큰 API)
let report = await getReport.fetch() @hint prefetch=never
```

---

## 6. `@hint` Directive — Summary

모든 최적화는 자동이지만, `@hint`로 컴파일러 결정을 덮어쓸 수 있다.

| Directive | Target | Values |
|-----------|--------|--------|
| `@hint protocol=` | 라우트 | `json`, `binary`, `hybrid` |
| `@hint render=` | 페이지 (`@html`) | `ssr`, `csr`, `ssg` |
| `@hint cache=` | fetch / 쿼리 | `never`, `immutable`, TTL 값 (예: `cache=60s`) |
| `@hint prefetch=` | fetch | `never`, `eager` |
| `@hint chunk=` | 라우트 / 모듈 | `separate`, `inline` |
| `@hint keep` | import | tree-shake 방지 |

---

## 7. Adaptive Runtime

컴파일러가 emit하는 경량 런타임 레이어. 프로덕션에서 관찰 → 조절을 수행한다.

### 7.1 관찰 대상

- 요청/응답 시간, 에러율
- 캐시 히트/미스 비율
- 페이지별 렌더링 시간 (FCP, TTI)
- 쿼리 패턴, fetch 호출 빈도
- 커넥션 풀 사용률

### 7.2 자동 조절 범위

| 레이어 | 자동 가능 | 재빌드 필요 (리포트 제안) |
|--------|----------|------------------------|
| 인프라 | 커넥션 풀, 워커 수, 타임아웃 | — |
| 캐싱 | TTL 조절, 캐시 활성/비활성 | — |
| 렌더링 | SSR ↔ SSR+캐시 | CSR ↔ SSR 전환 |
| 데이터 | 프리페치 대상, 배치 크기 | — |

### 7.3 Non-goal

런타임 어댑터는 **관찰 + 파라미터 튜닝** 만 한다. 코드를 재작성하거나 전략을 근본적으로 바꾸지 않는다. 그런 수준의 변경은 빌드 리포트를 통해 개발자에게 제안.
