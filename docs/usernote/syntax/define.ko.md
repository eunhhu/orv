# 커스텀 도메인 & 노드 계약 — `define`

[← 목차로 돌아가기](./index.ko.md)

---

## `define`의 역할

`define`은 재사용 가능한 orv 표면 문법을 설명하는 추상화입니다.

이것은 클래스가 아니고, 생성자 시스템이 아니며, 함수형 빌더 API도 아닙니다.

구분은 이렇게 잡습니다:

- 재사용 가능한 계산과 값 로직: `function`
- 재사용 가능한 `@domain` / `@node` 표면 구조: `define`

무언가를 `@Name ...` 형태로 호출해야 한다면 `define`에 속합니다.

무언가를 `name(...)` 형태로 호출해야 한다면 `function`에 속합니다.

## `define`의 두 가지 형태

### 1. 노드 / 도메인 루트 define

```orv
define Button(label: string) -> @button {
  @text label
}

@Button %label="Save"
```

이 형태는 "`@Button ...`이 호출 표면이고, node-like 도메인 루트로 확장된다"는 뜻입니다.

### 2. 도메인 패밀리 define

```orv
define html() -> {
  let ssr: bool = @token exist "ssr"

  define head() -> {
    @children
  }

  define body() -> {
    @children
  }
}

@html ssr {
  @head {}
  @body {}
}
```

이 형태는 "`@html ...`이 호출 표면이고, 내부의 nested `define`이 이 블록 안에서 허용되는 하위 도메인을 설명한다"는 뜻입니다.

이것이 [`project-e2e`](/Users/sunwoo/work/miol/fixtures/project-e2e)에서 사용하는 기본 사고방식입니다.

## 표면 문법

정식 node/domain 표면은 공백 기반으로 구조화됩니다:

```orv
@domain subtoken subtoken2 %key=value data {
  // nested block content
}
```

공백은 중요합니다. node head에서 공백은 기본 구조 splitter 역할을 합니다.

선행 `@domain` 뒤의 각 공백 구분 세그먼트는 네 가지 역할 중 하나를 가집니다:

| 역할 | 형태 | 의미 |
|------|------|------|
| 도메인 루트 | `@route`, `@html`, `@div` | 활성 node/domain 선택 |
| subtoken | `GET`, `/api`, `ssr`, `1m` | 도메인이 해석하는 가벼운 의미 modifier |
| property | `%key=value` | named configuration |
| data | `port`, `page`, `"description"`, `200` | head에 실리는 일반 값 payload |

head 뒤의 선택적 `{ ... }` 블록은 nested slot content를 담습니다.

## Subtoken

subtoken은 도메인 루트 뒤에 오는 공백 구분 bare unit입니다.

예시:

```orv
@route GET /users
@html ssr
@RateLimit 1000 1m
```

subtoken은 함수 호출 인자의 의미가 아니라, 도메인 표면에서 쓰이는 의미 표식입니다.

대표적인 해석:

- `@route`의 HTTP method와 path
- `ssr` 같은 render modifier
- `1m` 같은 duration/size shorthand
- UI node의 style/domain modifier

`define` 내부에서는 `@token`으로 이를 읽습니다:

```orv
let has_ssr: bool = @token exist "ssr"
let duration: string = @token match "\\d+[smhd]"
```

## Property

named configuration은 여전히 `%key=value`를 사용합니다.

```orv
@Card %title="Profile"
@Button %variant="primary" %disabled={isLocked}
```

property는 순서보다 이름이 중요한 입력을 표현하는 기본 자리입니다.

## Head의 Data

head data는 `{ ... }` 블록으로 제한되지 않습니다.

일반적인 데이터 표현식은 모두 node head에 올 수 있습니다:

```orv
@listen port
@serve page
@respond 200 result
@meta "description" "My App description"
```

여기서 `data`는 "head에 붙는 일반 payload 값"을 뜻하며, "반드시 inner block이어야 한다"는 뜻이 아닙니다.

도메인에 따라 head data는 다음일 수 있습니다:

- identifier
- literal
- path-like token
- 계산된 값
- HTML/page reference

## Nested Slot Content와 `@children`

`@children`은 `define` 호출 블록으로 들어온 nested block content를 투영하는 문법입니다.

키워드는 `@children`이지만, 개념 자체는 DOM 전용이 아닙니다. 어떤 도메인에서든 쓰이는 일반 slot-content operator입니다.

```orv
define Card(title: string) -> @div {
  @div {
    @text title
    @children
  }
}

@Card %title="Profile" {
  @text "Body"
}
```

문서에서는 이것을 다음처럼 읽습니다:

- invocation block content
- nested slot content
- `@children`을 통한 projection

UI 전용 API처럼 생각하지 마세요.

## `define`은 계약이다

`define`은 네 축에 대한 계약을 세웁니다:

1. 허용되는 subtoken
2. 허용되는 `%key=value` property
3. 허용되는 head data 값
4. 허용되는 nested slot content

이 계약은 body 안에서 암묵적이거나 명시적으로 드러날 수 있습니다:

```orv
define html() -> {
  let ssr: bool = @token exist "ssr"

  define head() -> {
    @children
  }

  define body() -> {
    let font_token: string = @token match "text-[(sm)(base)(lg)]"
    @children
  }
}
```

명시적인 schema 문법은 앞으로 더 다듬어질 수 있지만, 문서 기준 모델은 이 네 축 계약입니다.

즉 앞으로 constraint 문법이 추가되더라도, 이 계약 모델을 더 정교하게 만드는 방향이어야 합니다.

## 이름 가이드

- `html`, `head`, `body`처럼 도메인 루트를 정의할 때는 lower-case가 자연스럽습니다
- `Button`, `Card`, `RateLimit`처럼 컴포넌트/앱 단위 define은 PascalCase가 자연스럽습니다

중요한 건 대소문자보다 호출 형태입니다:

- `@html ...`
- `@Button ...`
- `html(...)`는 아님
- `Button(...)`도 아님

## `function`과 `define`

추상화가 본질적으로 callable이고 값을 반환한다면 `function`을 사용하세요:

```orv
function buildUserUrl(id: string) -> string {
  "/api/users/{id}"
}
```

추상화가 `@domain` 블록의 표면 문법을 정의한다면 `define`을 사용하세요:

```orv
define RateLimit(max: i32?, time: string?) -> @after {
  let parsed_time = time ?? @token match "\\d+[smhd]" ?? "1m"
  let parsed_max = max ?? @token match "\\d+" ?? 1000
  @children
}
```

## 요약

| 목표 | 사용 |
|------|------|
| 재사용 가능한 계산 | `function` |
| 재사용 가능한 `@domain` 표면 | `define` |
| 도메인 루트 선언 | `define X(...) -> @domain { ... }` 또는 `define x() -> { ... }` |
| 호출 형태 | `@X subtoken %key=value data { ... }` |
| subtoken 검사 | `@token exist`, `@token match` |
| nested slot projection | `@children` |
