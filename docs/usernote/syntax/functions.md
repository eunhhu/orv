# Functions & Control Flow

[← Back to Index](./index.md)

---

## Functions

### Declaration

```miol
function add(a: i32, b: i32): i32 -> {
  a + b  // implicit return (last expression)
}

function greet(name: string): string -> {
  return "Hello, {name}"  // explicit return also works
}

// Single-expression shorthand
function double(x: i32): i32 -> x * 2
```

### Named Arguments

Functions can be called with named arguments using `name=value` syntax, in any order:

```miol
function add(a: i32, b: i32): i32 -> {
  a + b
}

add(b=10, a=30)    // 40 — order doesn't matter
add(1, 2)          // 3  — positional also works
add(1, b=2)        // 3  — mixed: positional first, then named
```

Named arguments are especially useful for functions with many parameters or optional values:

```miol
function createUser(name: string, age: i32, email: string?): User -> { ... }

createUser(name="Kim", age=22)
createUser(age=25, name="Lee", email="lee@example.com")
```

### Callbacks & Closures

```miol
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

**Best Practice:** Use `$0` for simple one-expression transforms. Use named parameters for multi-line closures or when the meaning isn't obvious.

### Pipe Operator

The pipe operator `|>` passes the left-hand value as the first argument to the right-hand function:

```miol
let result = value |> transform |> validate |> format

// Equivalent to:
let result = format(validate(transform(value)))

// Also works with type-attached methods
let nan = x |> f64.isNaN
```

### Async Functions

```miol
async function fetchUser(id: i32): User -> {
  let response = await http.get("/api/users/{id}")
  response.json()
}

// top-level await — no async wrapper needed
let config = await loadConfig()
let db = await Database.connect(config.dbUrl)
```

---

## Control Flow

### If / Else

```miol
if condition {
  doSomething()
} else if otherCondition {
  doOther()
} else {
  fallback()
}
```

### Ternary

```miol
let label = isActive ? "On" : "Off"
```

### For Loops

```miol
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

```miol
while condition {
  // ...
}
```

---

## Pattern Matching

`when` is miol's exhaustive pattern matching construct, inspired by Kotlin's `when` with Rust-level expressiveness. `when` can be used both as a statement (for side effects) and as an expression (returning a value).

### Value Matching

```miol
when status {
  200 -> @io.out "OK"
  404 -> @io.out "Not Found"
  500 -> @io.out "Server Error"
  _ -> @io.out "Unknown: {status}"
}
```

### Range Matching

```miol
when score {
  90..=100 -> "A"
  80..90   -> "B"
  70..80   -> "C"
  ..70     -> "F"
  _        -> "Invalid"
}
```

### Or Patterns

```miol
when x {
  1 | 2 | 3 -> @io.out "small"
  _ -> @io.out "other"
}
```

### Struct Destructuring

```miol
let point: Point = { x: 1, y: 2 }

when point {
  Point { x: 0, y }    -> @io.out "on y-axis at {y}"
  Point { x, y: 0 }    -> @io.out "on x-axis at {x}"
  Point { x, y } if x == y -> @io.out "on diagonal"
  _                     -> @io.out "somewhere else"
}
```

`Type { ... }` in a `when` arm is a destructuring pattern, not a value constructor. Create struct values with typed object literals such as `let point: Point = { x: 1, y: 2 }`.

### Enum Destructuring

```miol
when result {
  Status.Ok(code)    -> @io.out "Success: {code}"
  Status.Error(msg)  -> @io.out "Failed: {msg}"
}
```

**Best Practice:** Always handle `_` (wildcard) to ensure exhaustiveness. The compiler will warn if patterns are not exhaustive for enums.
