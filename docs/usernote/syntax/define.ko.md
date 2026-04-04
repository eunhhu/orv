# 커스텀 노드 — `define`

[← 목차로 돌아가기](./index.ko.md)

---

## 왜 `class`가 아닌 `define`인가

orv에는 `class` 키워드가 없습니다. `new`, `this`, 상속, 프로토타입이 없습니다. 이는 의도적입니다.

`define`은 `class`가 전통적으로 수행하던 모든 역할을 대체합니다:

| 전통적 OOP | orv 대응 |
|----------------|-----------------|
| 메서드를 가진 클래스 | 중첩 `define`을 가진 `define` |
| 생성자 | `define` 파라미터 |
| 인스턴스 상태 | `define` 내부의 `let` / `let mut` / `let sig` |
| 캡슐화 | 클로저 스코프 (내부 변수는 기본적으로 비공개) |
| 다형성 | 파라미터에 따라 다른 `@` 노드를 반환하는 `define` |
| 합성 | `@children` + 중첩 `define` |
| 싱글톤 | 한 번만 호출되는 최상위 `define` |

그 이유는: orv는 **노드 지향 언어**입니다. 모든 것은 노드(`@`), 속성(`%`), 또는 구문입니다. 클래스는 노드 트리와 경쟁하는 병렬 객체 시스템을 도입합니다. `define`은 모든 것을 하나의 통합 모델로 유지합니다.

## 기본 문법

```orv
define Name(params...) -> returnNode {
  // 본문
}
```

- **`Name`**: UI 컴포넌트는 관례적으로 PascalCase, 유틸리티는 camelCase
- **`params`**: 타입이 지정된 파라미터, 호출 시 `%` 속성으로 전달됨
- **`-> returnNode`**: 이 define이 생성하는 루트 노드 또는 값
- **본문**: 자식 노드, 속성, 로직 — 모든 `{ }` 블록과 동일한 세 가지 역할 규칙

## 간단한 컴포넌트

```orv
define Button(label: string, variant: string?) -> @button label rounded-md {
  when variant {
    "primary"   -> %class="bg-blue-500 text-white"
    "danger"    -> %class="bg-red-500 text-white"
    _           -> %class="bg-gray-200 text-gray-800"
  }
}

// 사용법 — @로 노드처럼 호출
@Button %label="Submit" %variant="primary"
@Button %label="Cancel"
```

## `@token`을 사용한 위치 토큰

`define`은 호출 줄의 위치 토큰(일반 단어)을 검사할 수 있습니다. `@token`은 특정 토큰이 존재하는지 확인합니다:

```orv
define Alert(message: string) -> @div p-4 rounded-md {
  if @token warning {
    %class="bg-yellow-100 text-yellow-800"
  } else if @token error {
    %class="bg-red-100 text-red-800"
  } else {
    %class="bg-blue-100 text-blue-800"
  }
  @text message
}

// 사용법 — 토큰은 @Identifier 뒤의 일반 단어
@Alert warning %message="Check your input"
@Alert error %message="Something failed"
@Alert %message="Just so you know"
```

정규식 패턴을 사용한 `@token`은 동적 토큰을 매칭합니다:

```orv
define Listen() -> {
  port = @token \d+    // 첫 번째 숫자 토큰을 캡처
}

// 사용법
@Listen 8080           // port = 8080
```

## `@children`을 사용한 자식 노드

호출 블록 안에 배치된 모든 노드는 define 내부에서 `@children`으로 사용할 수 있습니다:

```orv
define Card(title: string) -> @div rounded-lg shadow-md p-4 {
  @h2 font-bold text-lg "{title}"
  @div mt-2 {
    @children
  }
}

// 사용법 — 블록 내용이 @children이 됨
@Card %title="Settings" {
  @text "Card body content"
  @button "Save"
}

// 자식 없음 — @children은 아무것도 렌더링하지 않음
@Card %title="Empty Card"
```

## 내부 상태

`define` 내부에서 선언된 변수는 **해당 인스턴스에 비공개**입니다. 각 호출은 자체 클로저를 가집니다:

```orv
define Counter(initial: i32?) -> @div {
  let sig count: i32 = initial ?? 0

  @text "Count: {count}"
  @hstack gap-2 {
    @button "-" %onClick={count -= 1}
    @button "+" %onClick={count += 1}
    @button "Reset" %onClick={count = initial ?? 0}
  }
}

// 각 인스턴스는 독립적인 상태를 가짐
@Counter %initial={0}     // 자체 count
@Counter %initial={100}   // 자체 count, 100에서 시작
```

---

## 고급 패턴

### 중첩 `define` — `class` 대체자

`define` 블록은 중첩된 `define`을 포함하여 내부 API를 만들 수 있습니다. 이것이 orv가 메서드를 가진 클래스를 대체하는 방법입니다:

```orv
define createServer() -> {
  let sig port: i32 = 8000
  let mut routes: Vec<Route> = []
  let server_instance = @io.serve(port)

  define listen(p: i32) -> {
    port = p
  }

  define route(method: string, path: string, handler: _ -> void) -> {
    let nextRoute: Route = { method, path, handler }
    routes.push(nextRoute)
  }

  define start() -> {
    @io.out "Server listening on port {port}"
    for r of routes {
      server_instance.register(r)
    }
  }

  // 인터페이스 반환 — 호출자는 listen, route, start를 볼 수 있지만
  // port, routes, server_instance는 볼 수 없음
  return { listen, route, start }
}

// 사용법 — 클래스 인스턴스처럼 보이지만, 실제로는 클로저
let app = createServer()
app.listen(3000)
app.route("GET", "/", _ -> return @response 200 { "ok": true })
app.start()
```

이 패턴은 다음을 제공합니다:
- **캡슐화**: `port`, `routes`, `server_instance`는 외부에서 접근 불가
- **상태**: 각 `createServer()` 호출은 자체 격리된 상태를 가짐
- **메서드**: `listen`, `route`, `start`는 공유 상태에 대한 클로저인 함수
- **`this` 없음**: 바인딩 혼란 없음, 콜백에서 `this` 문제 없음

### 빌더 패턴

```orv
define createQuery(table: string) -> {
  let mut conditions: Vec<string> = []
  let mut limit_val: i32? = void
  let mut order_by: string? = void

  define where(condition: string) -> {
    conditions.push(condition)
  }

  define limit(n: i32) -> {
    limit_val = n
  }

  define orderBy(field: string) -> {
    order_by = field
  }

  define build(): string -> {
    let mut sql = "SELECT * FROM {table}"
    if conditions.len() > 0 {
      sql = sql + " WHERE " + conditions.join(" AND ")
    }
    if order_by != void {
      sql = sql + " ORDER BY {order_by}"
    }
    if limit_val != void {
      sql = sql + " LIMIT {limit_val}"
    }
    sql
  }

  return { where, limit, orderBy, build }
}

let q = createQuery("users")
q.where("age > 18")
q.where("active = true")
q.orderBy("name")
q.limit(10)
let sql = q.build()
// → "SELECT * FROM users WHERE age > 18 AND active = true ORDER BY name LIMIT 10"
```

### 도메인 프리미티브

내장된 `@server`, `@route` 등은 개념적으로 도메인 컨텍스트를 가진 `define` 블록입니다. 자체 도메인 프리미티브를 만들 수 있습니다:

```orv
define ApiGroup(prefix: string) -> {

  define get(path: string, handler: _ -> void) -> {
    @route GET {prefix}{path} {
      try {
        handler()
      } catch e {
        return @response 500 { "error": e.message }
      }
    }
  }

  define post(path: string, handler: _ -> void) -> {
    @route POST {prefix}{path} {
      try {
        handler()
      } catch e {
        return @response 500 { "error": e.message }
      }
    }
  }

  return { get, post }
}

// 사용법
@server {
  @listen 8080

  let users = ApiGroup("/api/users")

  users.get("/", _ -> {
    let all = await db.findAllUsers()
    return @response 200 { "users": all }
  })

  users.post("/", _ -> {
    let { name, email } = @body
    let user = await db.createUser(name, email)
    return @response 201 { "user": user }
  })
}
```

### 상태 머신

```orv
define createFetcher<T>(fetchFn: _ -> T) -> {
  let sig state: string = "idle"
  let sig data: T? = void
  let sig error: string? = void

  define execute() -> {
    state = "loading"
    data = void
    error = void
    try {
      data = await fetchFn()
      state = "success"
    } catch e {
      error = e.message
      state = "error"
    }
  }

  define reset() -> {
    state = "idle"
    data = void
    error = void
  }

  return { state, data, error, execute, reset }
}

// UI에서 사용
define UserProfile(userId: i32) -> @div {
  let fetcher = createFetcher(_ -> http.get("/api/users/{userId}"))

  %onMount={
    fetcher.execute()
  }

  when fetcher.state {
    "idle"    -> @text "Ready"
    "loading" -> @text "Loading..."
    "success" -> {
      @h1 "{fetcher.data.name}"
      @p "{fetcher.data.email}"
    }
    "error"   -> @text text-red-500 "Error: {fetcher.error}"
    _         -> @text "Unknown state"
  }
}
```

### 제네릭 타입

```orv
define List<T>(items: Vec<T>, renderItem: T -> void) -> @ul {
  for item of items {
    @li {
      renderItem(item)
    }
  }
}

// 사용법
@List<User> %items={users} %renderItem={user: User -> {
  @text "{user.name} ({user.email})"
}}

```

### 내보낸 정의

```orv
// components/Button.orv
pub define PrimaryButton(label: string) -> @button label {
  %class="bg-blue-500 text-white px-4 py-2 rounded-md hover:bg-blue-600"
}

pub define DangerButton(label: string) -> @button label {
  %class="bg-red-500 text-white px-4 py-2 rounded-md hover:bg-red-600"
}

// 비공개 — 이 파일 외부에서 접근 불가
define baseButtonStyles() -> "px-4 py-2 rounded-md font-medium"
```

---

## 요약: `define` 기능

| 기능 | 패턴 |
|-----------|---------|
| UI 컴포넌트 | `define Name() -> @div { ... }` |
| 유틸리티 함수 | `define helper() -> { return value }` |
| 상태를 가진 객체 | `define create() -> { let state; return { methods } }` |
| 빌더 | `define builder() -> { return { chain, build } }` |
| 상태 머신 | `define machine() -> { let sig state; return { state, transitions } }` |
| 도메인 프리미티브 | `define group() -> { define innerRoute(); return api }` |
| 고차 함수 | `define hoc<T>(component: T) -> @div { ... }` |
