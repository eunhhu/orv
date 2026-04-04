# 컬렉션, 에러 처리 & 비동기

[← 목차로 돌아가기](./index.ko.md)

---

## 컬렉션

### Vec (동적 배열)

`Vec<T>`는 orv의 표준 순서 시퀀스 타입입니다. 의미적으로 JavaScript 배열에 가장 가깝지만, 언어 내에서는 여전히 타입이 지정된 벡터입니다.

```orv
let mut numbers: Vec<i32> = []
numbers.push(1)
numbers.push(2)
numbers.pop()         // returns i32?
numbers.len()         // i32
numbers.clear()

// Functional operations
let doubled = numbers.map($0 * 2)
let evens = numbers.filter($0 % 2 == 0)
let sum = numbers.reduce(0, $0 + $1)

// Literal initialization
let primes = [2, 3, 5, 7, 11]
```

`@response` 페이로드와 같은 JSON 형태 컨텍스트에서 `Vec<T>`는 배열로 취급되며 JSON 배열로 직렬화됩니다.

### 일반 객체 / 레코드 (`{}`)

일반 객체 / 레코드 값은 `{}`를 사용하며 고정된 명명 필드를 나타냅니다:

```orv
let user = {
  name: "Kim"
  age: 22
}

let point: Point = {
  x: 10
  y: 20
}
```

필드 이름이 프로그램의 구조적 형태의 일부일 때 `{}`를 사용하세요.

### HashMap

`HashMap<K, V>`는 진정한 맵/딕셔너리 타입입니다. `#{}` 리터럴을 사용하며, `{}`로 만든 일반 객체 / 레코드 값과는 구별됩니다.

```orv
let mut scores: HashMap<string, i32> = #{}
scores.insert("alice", 100)
scores.insert("bob", 85)
scores.get("alice")       // i32?
scores.remove("bob")
scores.clear()
scores.len()
scores.keys()             // Vec<string>
scores.values()           // Vec<i32>

// Literal initialization
let config = #{
  "host": "localhost"
  "port": "8080"
}
```

키가 동적 맵 항목일 때 `HashMap`을 사용하세요. 명명 필드가 구조적으로 의미 있을 때는 일반 `{}` 객체를 사용하세요.

JSON 경계에서 일반 객체와 `HashMap<string, T>` 모두 JSON 객체로 직렬화할 수 있지만, orv 내부에서는 의미적으로 다른 타입으로 유지됩니다.

### 반복

```orv
for (key, value) of scores {
  @io.out "{key}: {value}"
}

for item of vec {
  @io.out item
}
```

---

## 에러 처리

orv는 에러 처리를 위해 `try` / `catch` 블록을 사용합니다:

```orv
try {
  let data = await fetchData()
  process(data)
} catch e {
  @io.out "Error: {e.message}"
}
```

### 타입이 지정된 Catch

```orv
try {
  let user = await db.findUser(id)
} catch e: NotFoundError {
  @io.out "User not found"
} catch e: DatabaseError {
  @io.out "DB error: {e.message}"
} catch e {
  @io.out "Unknown error: {e.message}"
}
```

### 표현식에서의 Try

```orv
let user: User = try db.findUser(id) catch {
  { name: "anonymous", age: 0 }
}
```

**모범 사례:** 일반적인 catch 절보다 구체적인 catch 절을 선호하세요. 서버 라우트에서는 항상 에러를 잡아서 적절한 HTTP 상태 코드를 반환하세요.

---

## Async / Await

### 기본

I/O나 네트워크 작업을 수행하는 함수는 `async`로 선언합니다:

```orv
async function fetchUser(id: i32): User -> {
  let res = await http.get("/api/users/{id}")
  res.json()
}
```

### 최상위 Await

`await`는 **어디서든** 작동합니다 — 최상위에서 `async` 래퍼가 필요 없습니다:

```orv
let config = await loadConfig()
let db = await Database.connect(config.dbUrl)

@server {
  @listen config.port
}
```

### 동시 실행

```orv
// Parallel fetch
let (users, posts) = await (fetchUsers(), fetchPosts())

// Or explicitly
let usersFuture = fetchUsers()
let postsFuture = fetchPosts()
let users = await usersFuture
let posts = await postsFuture
```

**모범 사례:** 병렬로 실행할 수 있는 독립적인 비동기 작업에는 동시 튜플 await를 사용하세요.
