# 함수 & 제어 흐름

[← 목차로 돌아가기](./index.ko.md)

---

## 함수

### 선언

```orv
function add(a: i32, b: i32): i32 -> {
  a + b  // implicit return (last expression)
}

function greet(name: string): string -> {
  return "Hello, {name}"  // explicit return also works
}

// Single-expression shorthand
function double(x: i32): i32 -> x * 2
```

### 명명된 인수

함수는 `name=value` 구문을 사용하여 명명된 인수로 호출할 수 있으며, 순서는 상관없습니다:

```orv
function add(a: i32, b: i32): i32 -> {
  a + b
}

add(b=10, a=30)    // 40 — order doesn't matter
add(1, 2)          // 3  — positional also works
add(1, b=2)        // 3  — mixed: positional first, then named
```

명명된 인수는 매개변수가 많거나 선택적 값이 있는 함수에 특히 유용합니다:

```orv
function createUser(name: string, age: i32, email: string?): User -> { ... }

createUser(name="Kim", age=22)
createUser(age=25, name="Lee", email="lee@example.com")
```

### 콜백 & 클로저

```orv
// Named parameter
vec.map(x: i32 -> x * 2)

// Multi-line closure
vec.filter(x: i32 -> {
  let threshold = 10
  x > threshold && x.isEven()
})

// $N shorthand — when any $N token appears, the expression 
// is automatically wrapped as a callback
vec.map($0 * 2)           // same as: x -> x * 2
vec.filter($0 > 10)       // same as: x -> x > 10
items.sort($0.age - $1.age)  // same as: (a, b) -> a.age - b.age
```

**모범 사례:** 간단한 단일 표현식 변환에는 `$0`을 사용하세요. 여러 줄 클로저나 의미가 명확하지 않을 때는 명명된 매개변수를 사용하세요.

### 파이프 연산자

파이프 연산자 `|>`는 왼쪽 값을 오른쪽 함수의 첫 번째 인수로 전달합니다:

```orv
let result = value |> transform |> validate |> format

// Equivalent to:
let result = format(validate(transform(value)))

// Also works with type-attached methods
let nan = x |> f64.isNaN
```

### 비동기 함수

```orv
async function fetchUser(id: i32): User -> {
  let response = await http.get("/api/users/{id}")
  response.json()
}

// top-level await — no async wrapper needed
let config = await loadConfig()
let db = await Database.connect(config.dbUrl)
```

---

## 제어 흐름

### If / Else

```orv
if condition {
  doSomething()
} else if otherCondition {
  doOther()
} else {
  fallback()
}
```

### 삼항 연산자

```orv
let label = isActive ? "On" : "Off"
```

### For 루프

```orv
// Range iteration (0 to 9)
for i of 0..10 {
  @io.out "{i}"
}

// Inclusive range (0 to 10)
for i of 0..=10 {
  @io.out "{i}"
}

// Collection iteration
for item of items {
  @io.out item.name
}

// With index (enumerated)
for (i, item) of items.enumerate() {
  @io.out "{i}: {item.name}"
}
```

### While

```orv
while condition {
  // ...
}
```

---

## 패턴 매칭

`when`은 orv의 완전 패턴 매칭 구문으로, Kotlin의 `when`에서 영감을 받았으며 Rust 수준의 표현력을 갖습니다. `when`은 문(부수 효과용)과 표현식(값 반환) 모두로 사용할 수 있습니다.

### 값 매칭

```orv
when status {
  200 -> @io.out "OK"
  404 -> @io.out "Not Found"
  500 -> @io.out "Server Error"
  _ -> @io.out "Unknown: {status}"
}
```

### 범위 매칭

```orv
when score {
  90..=100 -> "A"
  80..90   -> "B"
  70..80   -> "C"
  ..70     -> "F"
  _        -> "Invalid"
}
```

### Or 패턴

```orv
when x {
  1 | 2 | 3 -> @io.out "small"
  _ -> @io.out "other"
}
```

### 구조체 구조 분해

```orv
let point: Point = { x: 1, y: 2 }

when point {
  Point { x: 0, y }    -> @io.out "on y-axis at {y}"
  Point { x, y: 0 }    -> @io.out "on x-axis at {x}"
  Point { x, y } if x == y -> @io.out "on diagonal"
  _                     -> @io.out "somewhere else"
}
```

`when` 암에서 `Type { ... }`은 구조 분해 패턴이지, 값 생성자가 아닙니다. 구조체 값은 `let point: Point = { x: 1, y: 2 }`와 같은 타입이 지정된 객체 리터럴로 생성합니다.

### 열거형 구조 분해

```orv
when result {
  Status.Ok(code)    -> @io.out "Success: {code}"
  Status.Error(msg)  -> @io.out "Failed: {msg}"
}
```

**모범 사례:** 완전성을 보장하기 위해 항상 `_`(와일드카드)를 처리하세요. 컴파일러는 열거형에 대해 패턴이 완전하지 않으면 경고합니다.
