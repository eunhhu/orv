# UI & 디자인 도메인

[← 목차로 돌아가기](./index.ko.md)

---

## UI 도메인

UI 도메인은 `@html`, `@body`, 그리고 `@div`, `@vstack`, `@hstack` 같은 UI 전용 노드 내부에서 활성화됩니다.

### HTML 구조

```orv
let page: html = @html {
  @head {
    @title "My Application"
    @meta description "A orv app"
    @meta viewport "width=device-width, initial-scale=1"
  }

  @body {
    @div flex flex-col min-h-screen {
      @Header
      @main flex-1 {
        @Outlet
      }
      @Footer
    }
  }
}
```

### 요소 & Tailwind

HTML 요소는 노드입니다. Tailwind 클래스는 위치 토큰으로 사용됩니다 — `class=`가 필요 없습니다:

```orv
@div flex flex-col gap-4 p-6 {
  @h1 text-2xl font-bold "Welcome"
  @p text-gray-500 "This is a paragraph"
  @button bg-blue-500 text-white rounded-md "Click me"
}
```

Tailwind 클래스와 문자열 리터럴은 한 줄에서 위치 토큰으로 공존할 수 있습니다. 가독성을 유지하세요 — 줄이 너무 길어지면 `define`으로 추출하는 것을 고려하세요.

### 레이아웃 단축 표기

```orv
@vstack gap-4 {       // 수직 스택 (flex flex-col)
  @text "Top"
  @text "Bottom"
}

@hstack gap-2 {       // 수평 스택 (flex flex-row)
  @text "Left"
  @text "Right"
}
```

### 이벤트 처리

```orv
// 인라인
@button "Click" %onClick={count += 1}

// 인라인 연속
@input flex-1 rounded-md
  %type="text"
  %value={query}
  %onInput={query = $0.target.value}

// 블록
@button "Submit" {
  %onClick={
    let result = await submitForm()
    if result.ok {
      navigate("/success")
    }
  }
}
```

### 조건부 렌더링

```orv
@div {
  if isLoggedIn {
    @text "Welcome, {username}"
    @button "Logout" %onClick={logout()}
  } else {
    @button "Login" %onClick={showLogin()}
  }
}
```

### 리스트 렌더링

```orv
@ul {
  for item of items {
    @li "{item.name} — {item.description}"
  }
}

// 인덱스와 함께
@ol {
  for (i, task) of tasks.enumerate() {
    @li "#{i + 1}: {task.title}"
  }
}
```

### 자식 노드

컴포넌트는 `@children`을 통해 자식 노드를 받습니다:

```orv
define Card(title: string) -> @div rounded-lg shadow-md p-4 {
  @h2 font-bold text-lg "{title}"
  @div mt-2 {
    @children
  }
}

// 사용법
@Card %title="Profile" {
  @text "Card content goes here"
  @button "Action"
}
```

### 생명주기

생명주기 훅은 `%` 속성입니다:

```orv
define Timer() -> @div {
  let sig seconds: i32 = 0
  let mut interval: Interval? = void

  %onMount={
    interval = @io.interval 1000 {
      seconds += 1
    }
  }

  %onUnmount={
    interval?.clear()
  }

  @text "{seconds}s elapsed"
}
```

| 훅 | 트리거 시점 |
|------|---------|
| `%onMount` | 노드가 DOM에 추가될 때 |
| `%onUnmount` | 노드가 DOM에서 제거될 때 |

### 인라인 스타일

```orv
@div {
  %style={
    backgroundColor: "red",     // camelCase
    // "background-color": "red", // kebab-case도 사용 가능
    padding: "1rem"
  }
  @text "Styled div"
}
```

**모범 사례:** 스타일링에는 Tailwind 클래스를 선호하세요. `%style`은 시그널이나 계산된 상태에 의존하는 동적 값에만 사용하세요.

### 템플릿의 문자열 보간

```orv
let sig name: string = "World"

@text "Hello, {name}"          // 반응적 — name이 변경되면 업데이트됨
@h1 "Page {currentPage} of {totalPages}"
```

---

## 디자인 도메인

`@design` 블록은 디자인 토큰 — 색상, 크기, 폰트, 테마 — 을 정의합니다.

```orv
@design {
  // 테마별 토큰
  @theme light {
    @color primary #1a1a1a
    @color foreground #ffffff
    @color background #f5f5f5
  }

  @theme dark {
    @color primary #ffffff
    @color foreground #1a1a1a
    @color background #0a0a0a
  }

  // 전역 토큰 (테마 독립적)
  @color accent #3b82f6
  @color error #ef4444

  @size base 16px
  @size sm 14px
  @size lg 20px
  @size radius 8px

  @font sans "Inter, system-ui, sans-serif" 16px weight-400 line-1.5
  @font mono "JetBrains Mono, monospace" 14px weight-400 line-1.6
}
```

### 디자인 토큰 사용

디자인 토큰은 UI 노드에서 Tailwind 스타일의 클래스로 참조됩니다:

```orv
@h1 text-primary bg-background font-sans "Hello"
@p text-foreground text-base "Body text"
@span text-error text-sm "Error message"
```

**모범 사례:** 모든 색상을 `@design`에서 토큰으로 정의하세요. UI 노드에서 하드코딩된 hex 값을 절대 사용하지 마세요.
