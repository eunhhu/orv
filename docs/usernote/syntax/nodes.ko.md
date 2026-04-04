# 노드 시스템 & 반응성

[← 목차로 돌아가기](./index.ko.md)

---

## 노드 시스템

`@` / `%` 시스템은 orv의 핵심 추상화입니다. 모든 도메인(UI, 서버, 디자인)은 동일한 노드 문법으로 표현됩니다.

### `@` — 구조적 노드

```orv
@identifier tokens... {
  // 자식 노드와 로직
}
```

노드는 다음을 포함할 수 있습니다:
- **위치 토큰**: 키워드로 파싱됨 (해당하는 경우 순서 무관)
- **문자열 리터럴**: `"text content"`
- **Tailwind 클래스**: `rounded-md flex items-center` (UI 컨텍스트에서)
- **인라인 `%` 속성**: `%key=value`를 같은 줄에 작성하거나, 바로 다음 들여쓰기된 연속 줄에 작성

### `%` — 속성

속성은 자신이 속한 노드를 설정합니다:

```orv
// 인라인 — 같은 줄에 작성
@button "Click" %onClick=handler() %disabled=false

// 인라인 연속 — 여전히 같은 노드 구문의 일부
@input flex-1 rounded-md
  %type="text"
  %placeholder="Search"
  %value={query}

// 내부 — 블록 안에 작성하며, 부모에 적용
@div {
  %class="container"
  %style={
    display: "flex"
    gap: "1rem"
  }
  @text "Content"
}

// 여러 줄 속성 값 — 값이 여러 구문에 걸칠 때 { }를 사용
%onClick={
  counter += 1
  @io.out "clicked"
}
```

인라인 `%` 속성은 개념적으로 노드 선언 자체에 속할 때 사용합니다. 내부 `%` 속성은 노드에 이미 본문이 있고, 해당 블록 안에 설정을 유지하는 것이 더 명확할 때 사용합니다.

### `@io` — 표준 입출력

```orv
@io.out "Hello, world"        // stdout
@io.err "Something went wrong" // stderr
```

### `@env` — 환경 변수

`@env`는 환경 변수를 읽으며, 일반적인 타입 추론에 참여합니다. 주변 컨텍스트가 구체적인 타입을 기대하면, 컴파일러는 해당 타입에 맞게 env 값을 강제 변환하고 검증합니다. 타입 컨텍스트가 없으면 `string`으로 기본 설정됩니다.

```orv
let port: i32 = @env PORT      // 어노테이션에서 i32로 추론/강제 변환
let secret = @env JWT_SECRET   // string (더 강한 타입 컨텍스트 없음)

// 인라인 사용
@listen @env PORT              // @listen이 기대하는 타입에서 추론
```

---

## 반응성 & 시그널

### 시그널 선언

```orv
let sig count: i32 = 0
```

`sig` 변수는 **변경 가능**하고 **반응적**입니다. 값이 변경되면, 이를 의존하는 모든 UI 노드나 파생 시그널이 자동으로 업데이트됩니다.

### 읽기 & 쓰기

시그널은 일반 변수처럼 읽고 쓸 수 있습니다 — 특별한 접근자가 필요 없습니다:

```orv
let sig count: i32 = 0

// 읽기
@text "Count: {count}"

// 쓰기
count += 1
count = 0
```

### 파생 시그널

초기값이 다른 `sig`를 참조하는 `sig`는 자동으로 파생됩니다:

```orv
let sig count: i32 = 0
let sig doubled: i32 = count * 2      // 자동 파생
let sig label: string = "Count: {count}"  // 자동 파생

// doubled와 label은 count가 변경될 때마다 업데이트됨
```

### 세밀한 업데이트

orv의 반응성은 세밀합니다: `count`가 변경되면, `count`를 참조하는 특정 DOM 노드만 업데이트됩니다 — 전체 컴포넌트 트리가 아닙니다.

```orv
define Counter() -> @div {
  let sig count: i32 = 0

  // count가 변경될 때 이 텍스트 노드만 다시 렌더링됨
  @text "Count: {count}"

  // 이 텍스트 노드는 절대 다시 렌더링되지 않음
  @text "This is static"

  @button "+" %onClick={count += 1}
}
```

### 컬렉션의 시그널

```orv
let sig items: Vec<string> = ["a", "b", "c"]

// 컬렉션 변경 시 업데이트가 트리거됨
items.push("d")
items.pop()

// 컬렉션에서 파생
let sig itemCount = items.len()
```
