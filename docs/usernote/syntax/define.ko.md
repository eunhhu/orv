# 커스텀 노드 & 도메인 블록 — `define`

[← 목차로 돌아가기](./index.ko.md)

---

## `define`의 역할

`define`은 재사용 가능한 `@node` 또는 도메인 블록을 선언합니다.

이것은 **클래스 시스템이 아니고**, **함수형 빌더 API도 아닙니다**.

구분은 이렇게 잡습니다:

- 재사용 가능한 로직과 값 계산: `function`
- 재사용 가능한 node/domain 구조: `define`

`@html`, `@route`, `@design` 같은 도메인 루트를 선언하는 `define`은 반드시 노드 문법으로 사용해야 합니다:

```orv
@Name %key=value custom-token {
  // child block
}
```

절대 이렇게 쓰지 않습니다:

```orv
Name(...)
```

## 기본 문법

```orv
define Name(params...) -> @domain {
  // body
}
```

- `Name`: 보통 PascalCase
- `params`: `%key=value`로 전달되는 named input
- `@domain`: 이 define이 확장되는 node/domain 루트
- body: 함수 반환 본문이 아니라 node/domain body
- 선언 헤더는 `-> @domain`까지만 두고, 토큰/속성은 호출 위치나 body 안의 노드에 둡니다

## 호출 규약

define은 노드처럼 호출합니다.

```orv
define Button(label: string, variant: string?) -> @button {
  if variant == "primary" {
    @text "Primary"
  }

  @text label
}

@Button %label="Save" %variant="primary"
@Button %label="Cancel"
```

호출 위치에서는:

- `%key=value`가 named parameter를 전달하고
- `@Name` 뒤의 일반 단어가 custom token이 되며
- 선택적인 `{ ... }` 블록이 `@children`이 됩니다

## `@token`을 이용한 커스텀 토큰

define은 호출 줄의 위치 토큰을 읽을 수 있습니다.

```orv
define Notice(message: string) -> @div {
  if @token warning {
    @text "Warning"
  } else if @token error {
    @text "Error"
  } else {
    @text "Info"
  }

  @text message
}

@Notice warning %message="Check your input"
@Notice error %message="Something failed"
@Notice %message="FYI"
```

이렇게 하면 define을 함수처럼 만들지 않고도 읽기 쉬운 토큰 기반 호출을 유지할 수 있습니다.

## `@children`

호출 블록 안의 자식 노드는 `@children`으로 노출됩니다.

```orv
define Card(title: string) -> @div {
  @div rounded-lg shadow-md p-4 {
    @h2 font-bold text-lg {
      @text title
    }

    @div mt-2 {
      @children
    }
  }
}

@Card %title="Profile" {
  @text "Card content"
  @button "Action"
}
```

define에 루트 토큰이나 `%` 속성이 필요하면 `define` 헤더가 아니라 body 안의 노드에 배치하세요.

## 커스텀 도메인 블록

도메인 지향 define도 같은 규칙을 따릅니다. 여전히 `@Name ...` 형태로만 사용하며 함수 호출형으로 쓰지 않습니다.

```orv
define AuthRoute(authRequired: bool?) -> @route {
  if authRequired ?? false {
    @before {
      let token = @header "Authorization"
      if !token {
        @respond 401 { "error": "Unauthorized" }
      }
    }
  }

  @children
}

@AuthRoute GET /profile %authRequired={true} {
  @respond 200 { "ok": true }
}
```

중요한 것은 호출 모양입니다:

```orv
@AuthRoute GET /profile %authRequired={true} {
  ...
}
```

이렇게 쓰지 않습니다:

```orv
AuthRoute("GET", "/profile", ...)
```

## 설계 규칙

재사용 가능한 node/domain define은 항상 이 순서로 생각하면 됩니다:

1. `@Name`
2. `%named=value`
3. custom token
4. 선택적 child block

이것이 커스텀 UI 노드와 커스텀 도메인 프리미티브의 안정적인 기본 모델입니다.

## 함수형 추상화가 필요할 때

추상화가 본질적으로 값 지향이거나 callable이라면 `function`을 사용하세요.

```orv
function buildUserUrl(id: string) -> string {
  "/api/users/{id}"
}

function clamp(value: i32, min: i32, max: i32) -> i32 {
  if value < min {
    min
  } else if value > max {
    max
  } else {
    value
  }
}
```

`function`은 계산용입니다.
`define`은 재사용 가능한 node/domain 구조용입니다.

## 요약

| 목표 | 사용 |
|------|------|
| 재사용 가능한 계산 | `function` |
| 선언 형태 | `define X(...) -> @domain { ... }` |
| 호출 형태 | `@X %key=value token { ... }` |
| named input | `%key=value` |
| 가벼운 modifier | custom token + `@token` |
| 중첩 콘텐츠 | `@children` |
