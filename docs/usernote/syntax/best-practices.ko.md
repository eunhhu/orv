# 모범 사례 & 예제

[← 목차로 돌아가기](./index.ko.md)

---

## 1. 파일 구성

```
project/
├── main.orv                // 진입점: 서버 + 배선
├── design.orv              // @design 토큰
├── components/
│   ├── Button.orv          // pub define Button
│   ├── Card.orv
│   ├── Input.orv
│   └── Layout.orv
├── pages/
│   ├── Home.orv
│   ├── About.orv
│   └── Dashboard.orv
├── server/
│   ├── routes.orv          // 라우트 정의
│   ├── middleware.orv       // @before / @after 블록
│   └── db.orv              // 데이터베이스 헬퍼
└── libs/
    ├── auth.orv
    └── validation.orv
```

## 2. 시그널 위생

```orv
// 좋음 — UI 업데이트를 구동하는 값에만 시그널 사용
let sig count: i32 = 0
let sig username: string = ""

// 나쁨 — 비반응적 데이터에 sig 사용
let sig API_URL: string = "https://api.example.com"  // 대신 const 사용
let sig tempCalc: i32 = someExpensiveCalc()           // 대신 let 사용
```

## 3. `define` 블록을 집중적으로 유지

```orv
// 좋음 — define당 하나의 책임
define UserAvatar(url: string, size: i32) -> @img {
  @img rounded-full %src={url} {
    %style={
      width: "{size}px"
      height: "{size}px"
    }
  }
}

define UserCard(user: User) -> @div {
  @div flex items-center gap-3 {
    @UserAvatar %url={user.avatarUrl} %size={48}
    @div {
      @text font-bold "{user.name}"
      @text text-gray-500 text-sm "{user.email}"
    }
  }
}

// 나쁨 — 하나의 define에서 너무 많은 것을 처리
define UserSection(users: Vec<User>) -> @div {
  // 가져오기, 필터링, 렌더링, 페이지네이션... 모두 하나의 블록에
}
```

## 4. 서버 라우트의 에러 처리

```orv
// 좋음 — 라우트에서 항상 에러 처리
@route POST /api/users {
  try {
    let { name, email } = @body
    let user = await db.createUser(name, email)
    @respond 201 { "user": user }
  } catch e: ValidationError {
    @respond 400 { "error": e.message }
  } catch e {
    @io.err "Unexpected: {e.message}"
    @respond 500 { "error": "Internal server error" }
  }
}

// 나쁨 — 처리되지 않은 에러가 서버를 크래시시킴
@route POST /api/users {
  let { name, email } = @body       // 본문이 잘못되면 throw
  let user = await db.createUser(name, email)  // DB 에러 시 throw
  @respond 201 { "user": user }
}
```

## 5. 하드코딩 값 대신 디자인 토큰 사용

```orv
// 좋음
@design {
  @color primary #3b82f6
  @color text-muted #6b7280
  @size radius-md 8px
}

@button bg-primary text-white "Submit"
@p text-text-muted "Helper text"

// 나쁨
@button %style={ backgroundColor: "#3b82f6", color: "#ffffff" } "Submit"
@p %style={ color: "#6b7280" } "Helper text"
```

## 6. 복잡성보다 합성을 선호

```orv
// 좋음 — 작은 define을 합성
define IconButton(icon: string, label: string) -> @button {
  @Icon %name={icon}
  @text "{label}"
}

define DangerButton(label: string) -> @button {
  @text "{label}"
}

// 좋음 — 반복 패턴에 define 사용
define ApiRoute(method: string, path: string) -> @route {
  @before {
    let token = @header "Authorization"
    if !token {
      @respond 401 { "error": "Unauthorized" }
    }
  }
  @children
}
```

## 7. 비동기 모범 사례

```orv
// 좋음 — 병렬 가져오기
let (users, posts) = await (fetchUsers(), fetchPosts())

// 나쁨 — 병렬이 가능한데 순차 실행
let users = await fetchUsers()
let posts = await fetchPosts()  // users가 완료될 때까지 대기

// 좋음 — 비동기에서의 에러 처리
let user: User = try await fetchUser(id) catch {
  @io.err "Failed to fetch user {id}"
  { name: "unknown", age: 0 }
}
```

## 8. 도메인 분리

```orv
// 좋음 — 각 도메인을 자체 파일에 또는 명확하게 분리
// design.orv
@design {
  @theme light { ... }
  @theme dark { ... }
}

// pages/Home.orv
pub define HomePage() -> @html {
  @body { ... }
}

// main.orv
import design.*
import pages.Home.HomePage

@server {
  @listen 8080
  @route GET / {
    @serve @html {
      @HomePage
    }
  }
}

// 나쁨 — 모든 것을 도메인이 뒤섞인 하나의 거대한 파일에
```

---

## 전체 예제: Todo 애플리케이션

```orv
// design.orv
@design {
  @theme light {
    @color primary #1a1a1a
    @color surface #ffffff
    @color border #e5e7eb
    @color text-muted #6b7280
  }

  @theme dark {
    @color primary #f5f5f5
    @color surface #1f2937
    @color border #374151
    @color text-muted #9ca3af
  }

  @font sans "Inter, sans-serif" 16px weight-400 line-1.5
}

// components/TodoItem.orv
import @std.io

pub define TodoItem(todo: Todo) -> @li {
  @input %type="checkbox" %checked={todo.done} %onChange={
    todo.done = !todo.done
  }

  if todo.done {
    @span text-text-muted line-through "{todo.title}"
  } else {
    @span text-primary "{todo.title}"
  }

  @button text-red-500 hover:text-red-700 "x" %onClick={
    todo.deleted = true
  }
}

// pages/Home.orv
import components.TodoItem

struct Todo {
  title: string
  done: bool
  deleted: bool
}

pub define HomePage() -> @html {
  @head {
    @title "orv Todo"
    @meta viewport "width=device-width, initial-scale=1"
  }

  @body font-sans bg-surface text-primary {
    @div max-w-md mx-auto py-8 {
      @h1 text-2xl font-bold mb-4 "orv Todo"

      let sig todos: Vec<Todo> = []
      let sig input: string = ""

      // 파생
      let sig remaining: i32 = todos.filter($0.done == false).len()

      @div flex gap-2 mb-4 {
        // 들여쓰기된 줄에서 인라인 연속 % 속성 허용
        @input flex-1 border border-border rounded-md px-3 py-2
          %type="text"
          %placeholder="What needs to be done?"
          %value={input}
          %onInput={input = $0.target.value}
          %onKeyDown={
            if $0.key == "Enter" && input.len() > 0 {
              let nextTodo: Todo = {
                title: input
                done: false
                deleted: false
              }
              todos.push(nextTodo)
              input = ""
            }
          }

        @button bg-primary text-surface px-4 py-2 rounded-md "Add" %onClick={
          if input.len() > 0 {
            let nextTodo: Todo = {
              title: input
              done: false
              deleted: false
            }
            todos.push(nextTodo)
            input = ""
          }
        }
      }

      @ul {
        for todo of todos {
          if !todo.deleted {
            @TodoItem %todo={todo}
          }
        }
      }

      @p text-text-muted text-sm mt-4 "{remaining} items remaining"
    }
  }
}

// main.orv
import pages.Home.HomePage

let port: i32 = @env PORT

@server {
  @listen port

  @route GET / {
    @serve @html {
      @HomePage
    }
  }

  @route GET /static {
    @serve ./public
  }
}
```
