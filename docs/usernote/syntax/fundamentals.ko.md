# 구문 기초

[← 목차로 돌아가기](./index.ko.md)

---

## 노드 선언 (`@`)

`@` 접두사는 구조적 노드를 선언합니다. 노드는 orv의 보편적 빌딩 블록으로, UI 요소, 서버 라우트, 디자인 토큰, 커스텀 추상화를 동일하게 표현합니다.

```orv
@identifier param1 param2 ... {
  // children, properties, and executable statements
}
```

노드는 **위치 기반 토큰**(키워드로 파싱되며, 해당하는 경우 순서 무관), `%`를 사용한 **인라인 속성**, 그리고 자식과 로직을 위한 **본문 블록** `{ }`을 받습니다.

## 속성 바인딩 (`%`)

`%` 접두사는 가장 가까운 부모 노드에 속성을 연결합니다.

```orv
// Inline (on the same line as the node)
@button "Submit" %onClick={submit()} %disabled={!isValid}

// Inline continuation (next indented lines still belong to the same node statement)
@input flex-1
  %type="text"
  %placeholder="Email"
  %value={email}

// Inner (inside the node body, applies to the parent node)
@div {
  %style={
    backgroundColor: "red"
  }
  @text "Hello"
}
```

인라인 `%` 속성은 노드와 같은 줄이나 바로 다음에 이어지는 들여쓰기된 연속 줄에 나타날 수 있습니다. `{ }` 노드 본문 내부의 `%` 속성은 내부 속성이며 해당 부모 노드를 구성합니다.

## 블록 내 세 가지 역할

모든 `{ }` 블록 내에서 각 줄은 정확히 세 가지 범주 중 하나에 해당합니다:

| 접두사 | 역할 | 예시 |
|--------|------|------|
| `@` | 구조 — 자식 노드 | `@text "Hello"` |
| `%` | 구성 — 부모의 속성 | `%onClick={handler()}` |
| *(없음)* | 실행 — 스코프 진입 시 실행 | `let x = 1` |

```orv
@div {
  // @ — structure
  @h1 "Title"

  // % — configuration (applies to parent @div)
  %style={ padding: "1rem" }

  // bare — execution (runs on mount in UI context)
  let sig count: i32 = 0
  @io.out "div mounted"
}
```

## 주석

```orv
// Single-line comment

/* 
  Multi-line
  comment 
*/

/// Documentation comment (attached to the next declaration)
/// Supports markdown formatting.
define Button(label: string) -> @button {
  @text label
}
```

## 세미콜론

세미콜론은 **허용되지 않습니다**. 줄 바꿈이 기본적으로 문장을 종료합니다. 주요 예외는 이전 노드 선언을 확장하는 연속 줄과 `{ }`, `( )`, `[ ]` 같은 그룹 구분자로 이미 감싸진 표현식입니다.

```orv
let a = 1
let b = 2
let c = 3
```

---

## 타입 시스템

### 원시 타입

orv의 원시 숫자 및 불리언 타입은 Rust 스타일의 명명 규칙과 의도를 따릅니다. 언어 표면에서는 Rust와 유사한 정수/부동소수점 계열을 유지하지만, 텍스트는 의도적으로 단일 `string` 타입으로 단순화되었습니다.

| 타입 | 설명 |
|------|------|
| `u8` | 8비트 부호 없는 정수 |
| `u16` | 16비트 부호 없는 정수 |
| `u32` | 32비트 부호 없는 정수 |
| `u64` | 64비트 부호 없는 정수 |
| `usize` | 포인터 크기 부호 없는 정수 |
| `i8` | 8비트 부호 있는 정수 |
| `i16` | 16비트 부호 있는 정수 |
| `i32` | 32비트 부호 있는 정수 |
| `i64` | 64비트 부호 있는 정수 |
| `isize` | 포인터 크기 부호 있는 정수 |
| `f32` | 32비트 부동소수점 |
| `f64` | 64비트 부동소수점 |
| `string` | 단일 UTF-8 텍스트 타입 |
| `bool` | 불리언 |
| `void` | 값 없음 / 반환 값 없음 |

언어 표면에서 문자열 타입은 `string` 하나뿐입니다. orv는 Rust처럼 별도의 `str`, `String`, `char` 타입을 노출하지 않습니다.

WASM으로 컴파일할 때 숫자 타입은 해당하는 경우 실제 WASM 친화적 머신 표현에 매핑됩니다(`i32`는 실제 32비트 정수). 네이티브 바이너리로 컴파일할 때는 플랫폼의 네이티브 표현에 매핑됩니다.

### 타입 추론

오른쪽이 명확할 때 타입이 추론됩니다:

```orv
let x = 42          // inferred as i32
let y = 3.14        // inferred as f64
let name = "orv"   // inferred as string
let flag = true     // inferred as bool
```

컴파일러가 추론할 수 없을 때는 명시적 어노테이션이 필요합니다:

```orv
let mut items: Vec<string> = []
```

### 내장 데이터 형태

원시 타입 외에, 일상적인 orv 코드에서 바로 중요한 세 가지 데이터 형태가 있습니다:

| 형태 | 리터럴 | 의미 |
|------|--------|------|
| `Vec<T>` | `[]` | 순서가 있는 동적 벡터, JavaScript 배열에 가장 가까움 |
| 일반 객체 / 레코드 | `{}` | 고정된 명명 필드, 구조체 형태 데이터와 JSON 객체 리터럴에 사용 |
| `HashMap<K, V>` | `#{}` | 동적 키/값 맵, 일반 객체와 구별됨 |

`Vec<T>`는 언어 전반에서 사용되는 시퀀스 타입입니다. JSON 형태 컨텍스트에서 `Vec<T>`는 배열로 취급되며 JSON 배열로 직렬화됩니다.

일반 객체 값과 `HashMap` 값 모두 JSON 경계에서 JSON 객체로 교차할 수 있지만, 타입 시스템에서 같은 것이 아닙니다:

- 일반 객체 / 레코드 값은 소스 형태에서 알려진 고정된 명명 필드를 사용합니다
- `HashMap`은 동적 키를 위한 진정한 딕셔너리/맵 추상화입니다

### 유니온 타입

```orv
type Number = i32 | f64
type Result = string | Error
type Nullable<T> = T?
```

### Nullable 타입

어떤 타입이든 `?`를 붙여 nullable로 만들 수 있습니다:

```orv
let name: string? = void    // nullable string, void means "no value"
let count: i32? = 42        // nullable but has a value
```

`void`는 아무것도 반환하지 않는 함수의 반환 타입이자, nullable 타입에서 "값 없음"을 나타내는 리터럴 값 역할을 합니다(다른 언어의 `null`과 유사).

### 열거형

```orv
enum Direction {
  Up
  Down
  Left
  Right
}

enum Status {
  Ok(i32)             // associated value
  Error(string)
}
```

### 구조체

구조체는 **헤드리스 데이터 형태**입니다 — TypeScript 인터페이스와 유사합니다. 리터럴 객체의 형태를 기술합니다. 구조체에는 메서드, 생성자, 상속이 없습니다. 순수한 구조적 타입입니다.

orv에는 `class`가 없습니다. 데이터는 `struct` / `enum`으로 모델링하고, 재사용 가능한 로직은 `function`에 두며, [`define`](./define.ko.md)은 재사용 가능한 `@node` / 도메인 구조에만 사용하세요.

```orv
struct Point {
  x: i32
  y: i32
}

struct User {
  name: string
  age: i32
  email: string?          // nullable field
  greet: void -> string   // function-typed field
  transform: i32 -> i32   // function-typed field
}

// Instantiation — typed literal object syntax
let origin: Point = { x: 0, y: 0 }
let user: User = {
  name: "Kim"
  age: 22
  email: void
  greet: _ -> "Hello, I'm {name}"
  transform: x -> x * 2
}
```

구조체 값은 일반 객체 리터럴로 생성되며, `Type { ... }` 생성자 구문이 아닙니다. 변수 어노테이션, 매개변수 타입, 또는 반환 타입을 사용하여 구조체 타입을 제공합니다.

구조체 형태 값은 `{}` 리터럴로 만든 일반 객체 / 레코드 값입니다. `HashMap` 값이 아니며 `#{}`과 호환되지 않습니다.

### 중괄호 `{}` — 일반 객체 vs 코드 블록

orv는 `{}`를 일반 객체 / 레코드 리터럴과 코드 블록 모두에 사용합니다. 컴파일러는 중괄호 내부의 첫 번째 문장을 검사하여 구별합니다:

| 첫 번째 줄 패턴 | 해석 |
|-----------------|------|
| `key: value`, `key: value`, ... | **일반 객체 / 레코드 리터럴** — 쉼표 또는 줄 바꿈으로 구분된 명명 필드 |
| `let`, `if`, `for`, `@`, `%`, 표현식, ... | **코드 블록** — 실행 가능한 문장 |

```orv
// Plain object / record — first line is `key: value`
let user = {
  name: "Kim"
  age: 22
}

// Commas are also allowed when you want them
let config = {
  host: "localhost",
  port: 8080,
}

// Code block — first line is a statement
let result = {
  let mut n = 0
  n += 10
  n * 2
}

// Plain object / record as function argument
createUser({
  name: "Lee"
  age: 25
  email: "lee@example.com"
})

// Code block as function body
function compute(): i32 -> {
  let x = heavy()
  x * 2
}
```

구별은 모호하지 않습니다: 키워드 접두사 없는 순수 `identifier: expression`은 항상 일반 객체 / 레코드 필드이고, 키워드(`let`, `const`, `if`, `for` 등)나 접두사(`@`, `%`)는 코드 블록을 나타냅니다. 여러 줄 객체 리터럴에서 쉼표는 선택 사항입니다. 줄 바꿈이 이미 필드를 구분하기 때문입니다.

### 제네릭

```orv
type Container<T> = T

struct Pair<A, B> {
  first: A
  second: B
}

struct Node<T> {
  value: T
  next: Node<T>?
}

function identity<T>(x: T): T -> x

let pair: Pair<i32, string> = { first: 1, second: "hello" }
```

### 함수 타입

함수 타입은 화살표 구문을 사용합니다:

```orv
// Single parameter
type Transform = i32 -> i32

// Multiple parameters
type Add = i32, i32 -> i32

// No parameters
type Factory = void -> string

// Nullable return
type MaybeFind = string -> i32?

// In struct fields
struct Config {
  validate: string -> bool
  onError: string, i32 -> void
}
```

### 튜플

```orv
let pair: (i32, string) = (42, "hello")
let (x, y) = pair     // destructuring

function divmod(a: i32, b: i32): (i32, i32) -> {
  (a / b, a % b)
}
let (quotient, remainder) = divmod(10, 3)
```

---

## 변수 & 가변성

orv는 Rust의 불변 기본 철학을 따릅니다:

```orv
let x = 10          // immutable
let mut y = 20      // mutable
let sig z = 30      // reactive signal (mutable, triggers UI updates)
const PI = 3.14159  // compile-time constant
```

| 키워드 | 가변 | 반응형 | 스코프 |
|--------|------|--------|--------|
| `let` | 아니오 | 아니오 | 블록 |
| `let mut` | 예 | 아니오 | 블록 |
| `let sig` | 예 | 예 | 블록 (반응성 시스템이 추적) |
| `const` | 아니오 | 아니오 | 모듈 |

### 구조 분해

```orv
// Array destructuring
let [first, second, ...rest] = [1, 2, 3, 4, 5]

// Struct destructuring
let point: Point = { x: 10, y: 20 }
let { x, y } = point

// Tuple destructuring
let (a, b) = (1, 2)

// Nested
let { x, y }: Point = { x: 1, y: 2 }
let [{ name }, ...others] = users
```
