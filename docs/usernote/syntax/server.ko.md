# 서버 도메인

[← 목차로 돌아가기](./index.ko.md)

---

## 기본 서버

`@server` 블록은 라우트, 미들웨어, 요청 처리를 포함한 HTTP 서버를 정의합니다.

```orv
@server {
  @listen 8080

  @route GET / {
    @serve ./public
  }
}
```

## 라우트

```orv
@server {
  @listen 8080

  // 토큰 순서는 유연함 — 메서드와 경로는 키워드로 파싱됨
  @route GET /api/users {
    @respond 200 {
      "users": []
    }
  }

  @route POST /api/users {
    let { name, email } = @body
    let user = await db.createUser(name, email)
    @respond 201 { "user": user }
  }

  // 와일드카드
  @route * {
    @serve htmlString
  }
}
```

## 중첩 라우트

라우트는 자연스럽게 중첩됩니다. 자식 라우트는 부모의 경로 접두사와 미들웨어를 상속합니다:

```orv
@server {
  @listen 8080

  @route /api {

    @before {
      @io.out "API request: {@method} {@path}"
    }

    @route GET /users {
      // GET /api/users를 처리
      let skip = @query "skip"
      let limit = @query "limit"
      let users = await db.findUsers(skip, limit)
      @respond 200 { "users": users }
    }

    @route GET /users/:id {
      // GET /api/users/:id를 처리
      let id = @param "id"
      let user = await db.findUser(id)
      @respond 200 { "user": user }
    }

    @route POST /users {
      // POST /api/users를 처리
      let { name, email } = @body
      let user = await db.createUser(name, email)
      @respond 201 { "user": user }
    }
  }
}
```

## 요청 접근자

| 접근자 | 반환값 | 설명 |
|----------|---------|-------------|
| `@body` | 파싱된 본문 | 요청 본문 (JSON 파싱됨) |
| `@param "key"` | `string?` | URL 경로 파라미터 (`/users/:id`에서 `:id`) |
| `@query "key"` | `string?` | 쿼리 스트링 파라미터 (`?skip=10`) |
| `@header "key"` | `string?` | 요청 헤더 값 |
| `@method` | `string` | HTTP 메서드 |
| `@path` | `string` | 요청 경로 |
| `@context "key"` | any | `@before` 미들웨어에서 설정한 값 |

```orv
// @param — 라우트 패턴의 경로 파라미터
@route GET /users/:id {
  let id = @param "id"        // /users/42에서 → "42"
}

// @query — 쿼리 스트링 파라미터
@route GET /users {
  let skip = @query "skip"    // /users?skip=10에서 → "10"
  let limit = @query "limit"  // /users?limit=20에서 → "20"
}

// @body — 파싱된 요청 본문
@route POST /users {
  let { name, email } = @body // JSON 본문 파싱됨
}

// @header — 요청 헤더
@route * {
  let token = @header "Authorization"
  let contentType = @header "Content-Type"
}
```

## 응답

라우트 핸들러는 `@respond`로 HTTP 응답을 보냅니다:

```orv
// 단순
@respond 200 { "message": "OK" }

// 헤더 포함
@respond 200 %header={
  "Content-Type": "application/json"
  "X-Custom": "value"
} {
  "data": result
}

// 조기 반환 (가드 절)
if !authorized {
  @respond 401 { "error": "Unauthorized" }
}

// 빈 본문
@respond 204 {}
```

`@respond`는 종단 라우트 액션입니다. 앞에 `return`을 쓰지 않습니다. 실행되면 현재 라우트 핸들러를 끝내고 HTTP 응답을 전송합니다.
`return`은 함수 스타일 값 반환에만 쓰며, `@route`, `@before`, `@after` 같은 도메인 블록에는 쓰지 않습니다.

`@respond`는 라우트가 orv 코드 안에서 응답을 구성할 때 사용합니다:

- 상태 코드
- 선택적 헤더
- JSON 형태 페이로드 또는 기타 응답 본문

`@serve`는 이미 존재하는 대상을 서버 런타임에 직접 넘길 때 사용합니다:

- 정적 디렉토리
- 특정 파일 경로
- `@html` 페이지 또는 노드

| 노드 | 의미 | 일반적인 출력 |
|------|------|----------------|
| `@respond` | 라우트 데이터로 HTTP 응답을 구성해서 전송 | JSON/본문 응답 |
| `@serve` | 기존 파일 또는 HTML 대상을 직접 서빙 | 정적 자산 또는 HTML 문서 |

전송 경계에서:

- `Vec<T>` 페이로드는 JSON 배열이 됩니다
- 일반 `{}` 객체 페이로드는 고정된 명명 필드를 가진 JSON 객체가 됩니다
- `HashMap<string, T>` 페이로드도 JSON 객체로 직렬화되지만, 언어 내에서는 일반 레코드/객체 값이 아닌 맵 값으로 유지됩니다

## 미들웨어

```orv
@route /api {

  // 모든 자식 라우트 전에 실행
  @before {
    let token = @header "Authorization"
    let verified = await jwt.verify(token, SECRET)
    if !verified {
      @respond 401 { "error": "Unauthorized" }
    }
    // @context를 통해 라우트 핸들러에 데이터 전달
    @context {
      userId: verified.sub
    }
  }

  // 모든 자식 라우트 후에 실행
  @after {
    @io.out "Request completed"
  }

  @route GET /profile {
    let userId = @context "userId"
    let user = await db.findUser(userId)
    @respond 200 { "user": user }
  }
}
```

## 정적 파일 & HTML 서빙

`@serve`는 JSON 페이로드를 만드는 노드가 아닙니다. 대상을 직접 서빙하고, 구조화된 응답 데이터를 보내는 쪽은 `@respond`입니다.

```orv
@route GET / {
  @serve ./public             // 정적 디렉토리
}

@route GET /app {
  @serve htmlString           // orv html 노드
}

@route GET /js {
  @serve ./public/bundle.js   // 특정 파일
}
```

## 변수로서의 라우트 — 풀스택 RPC

변수에 할당된 라우트는 UI 도메인에서 **호출 가능한 엔드포인트**가 됩니다. 이것이 orv의 내장 풀스택 RPC입니다 — 별도의 API 클라이언트, 수동 fetch URL, 코드 생성 단계가 없습니다.

라우트 참조는 일반 렉시컬 스코프 규칙을 따릅니다. `.fetch()`를 호출하는 UI는 라우트 참조와 동일한 스코프에서 정의되거나, 해당 라우트 참조를 명시적으로 전달받아야 합니다.

```orv
@server {
  @listen 8000

  let userService = @route GET /api/user {
    let users = await db.findAll()
    @respond 200 { "users": users }
  }

  let createUser = @route POST /api/user {
    let { name, email } = @body
    let user = await db.create(name, email)
    @respond 201 { "user": user }
  }

  @route GET / {
    let page = @html {
      @body {
        let sig data = await userService.fetch()

        @div {
          if data != void {
            for user of data.users {
              @text "{user.name}"
            }
          } else {
            @text "Loading..."
          }
        }

        @button "Add User" %onClick={
          await createUser.fetch(body={
            name: "Kim"
            email: "kim@example.com"
          })
          data = await userService.fetch()
        }
      }
    }

    @serve page
  }
}
```

**작동 방식:**

| 개념 | 설명 |
|---------|-------------|
| `let x = @route ...` | 라우트를 변수에 할당하여 호출 가능한 참조로 만듦 |
| `x.fetch()` | 클라이언트에서 라우트를 호출 — 올바른 URL과 메서드로 `fetch()`로 컴파일됨 |
| `x.fetch(body={...})` | 요청 본문 전송 (POST/PUT/PATCH용) |
| `x.fetch(query={...})` | 쿼리 파라미터 추가 |
| `x.fetch(header={...})` | 커스텀 헤더 추가 |
| `x.fetch(param={...})` | 경로 파라미터 (`/users/:id`에서 `:id`) |

**이것이 중요한 이유:**

- **경계를 넘는 타입 안전성.** 컴파일러가 `@respond`에서 응답 형태를 알기 때문에, `data.users`는 컴파일 시점에 타입 체크됩니다.
- **UI 코드에 URL 문자열 없음.** 라우트 경로는 구현 세부사항입니다 — UI는 URL이 아닌 변수를 참조합니다.
- **리팩토링 안전성.** 라우트 경로를 변경해도, 모든 `.fetch()` 호출은 하드코딩된 문자열이 아닌 변수를 참조하므로 계속 작동합니다.
- **보일러플레이트 제로.** API 클라이언트 라이브러리, OpenAPI 스펙, 코드 생성 단계가 없습니다. 서버와 클라이언트 간의 연결은 변수 바인딩입니다.

### 다중 라우트 참조

```orv
@server {
  @listen 8000

  let getUsers = @route GET /api/users {
    @respond 200 { "users": await db.findAll() }
  }

  let getUser = @route GET /api/users/:id {
    let id = @param "id"
    @respond 200 { "user": await db.findUser(id) }
  }

  let deleteUser = @route DELETE /api/users/:id {
    let id = @param "id"
    await db.deleteUser(id)
    @respond 204 {}
  }

  @route GET /dashboard {
    let page = @html {
      @body {
        let sig users = await getUsers.fetch()
        let sig profile = await getUser.fetch(param={ id: "42" })

        @button "Delete" %onClick={
          await deleteUser.fetch(param={ id: profile.user.id })
          users = await getUsers.fetch()
        }
      }
    }

    @serve page
  }
}
```

## 함수로서의 서버

서버는 동적으로 생성할 수 있습니다:

```orv
function myServer(port: i32, root: string) -> @server {
  @listen port
  @route * {
    @serve root
  }
}

myServer(8080, "./public")
myServer(3000, "./admin")
```

---

## 도메인 컨텍스트 & 유효성 검사

orv는 **컴파일 타임 도메인 유효성 검사**를 시행합니다. 각 최상위 블록(`@html`, `@server`, `@design`)은 내부에서 어떤 `@` 노드가 유효한지를 제한하는 컨텍스트를 정의합니다.

컴파일러가 모든 도메인을 함께 보기 때문에, 도메인 경계를 넘어 최적화할 수 있습니다. `@server`가 `@html` 페이지를 서빙할 때, 컴파일러는 양쪽을 모두 알고 있으므로 — 둘 간의 통신을 최적화하고, 인라인할 수 있는 것은 인라인하며, 프로젝트의 특정 도메인 관계에 맞춤화된 출력을 생성할 수 있습니다.

```orv
// 유효 — 각 노드가 올바른 도메인에 속함
@server {
  @listen 8080
  @route / { @serve page }
}

@html {
  @body {
    @div { @text "Hello" }
  }
}

@design {
  @theme dark {
    @color primary #fff
  }
}
```

```orv
// 컴파일 에러 — 도메인 불일치
@server {
  @div { ... }           // 에러: @div는 서버 컨텍스트에서 유효하지 않음
}

@html {
  @body {
    @listen 8080         // 에러: @listen은 UI 컨텍스트에서 유효하지 않음
    @route / { ... }     // 에러: @route는 UI 컨텍스트에서 유효하지 않음
  }
}

@design {
  @route / { ... }       // 에러: @route는 디자인 컨텍스트에서 유효하지 않음
}
```

### 도메인 간 참조

변수를 사용하여 도메인을 연결합니다:

```orv
let page = @html {
  @body {
    @div { @text "Hello" }
  }
}

@server {
  @listen 8080
  @route / {
    @serve page   // 인라인이 아닌 참조 — 도메인을 분리하여 유지
  }
}
```
