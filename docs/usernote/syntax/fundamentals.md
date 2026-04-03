# Syntax Fundamentals

[← Back to Index](./index.md)

---

## Node Declaration (`@`)

The `@` prefix declares a structural node. Nodes are the universal building block of orv — they represent UI elements, server routes, design tokens, and custom abstractions alike.

```orv
@identifier param1 param2 ... {
  // children, properties, and executable statements
}
```

Nodes accept **positional tokens** (parsed by keyword, order-independent where applicable), **inline properties** with `%`, and a **body block** `{ }` for children and logic.

## Property Binding (`%`)

The `%` prefix attaches a property to the nearest parent node.

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

Inline `%` properties may appear on the same line as the node or on immediately following indented continuation lines. `%` properties inside a `{ }` node body are inner properties and configure that parent node.

## Three Roles in a Block

Inside any `{ }` block, every line falls into exactly one of three categories:

| Prefix | Role | Example |
|--------|------|---------|
| `@` | Structure — child node | `@text "Hello"` |
| `%` | Configuration — property of the parent | `%onClick={handler()}` |
| *(none)* | Execution — runs when the scope is entered | `let x = 1` |

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

## Comments

```orv
// Single-line comment

/* 
  Multi-line
  comment 
*/

/// Documentation comment (attached to the next declaration)
/// Supports markdown formatting.
define Button(label: string) -> @button label rounded-md
```

## Semicolons

Semicolons are **not allowed**. Line breaks terminate statements by default. The main exceptions are continuation lines that extend the previous node declaration and expressions already enclosed by grouping delimiters such as `{ }`, `( )`, or `[ ]`.

```orv
let a = 1
let b = 2
let c = 3
```

---

## Type System

### Primitive Types

orv's primitive numeric and boolean types follow Rust-style naming and intent. The language surface keeps Rust-like integer/float families, but text is intentionally simplified to a single `string` type.

| Type | Description |
|------|-------------|
| `u8` | 8-bit unsigned integer |
| `u16` | 16-bit unsigned integer |
| `u32` | 32-bit unsigned integer |
| `u64` | 64-bit unsigned integer |
| `usize` | pointer-sized unsigned integer |
| `i8` | 8-bit signed integer |
| `i16` | 16-bit signed integer |
| `i32` | 32-bit signed integer |
| `i64` | 64-bit signed integer |
| `isize` | pointer-sized signed integer |
| `f32` | 32-bit float |
| `f64` | 64-bit float |
| `string` | The single UTF-8 text type |
| `bool` | Boolean |
| `void` | No value / no return value |

At the language surface, there is only one string type: `string`. orv does not expose separate `str`, `String`, or `char` types the way Rust does.

When compiled to WASM, numeric types map to their true WASM-friendly machine representations where applicable (`i32` is a real 32-bit integer). When compiled to native binary, they map to the platform's native representations.

### Type Inference

Types are inferred when the right-hand side is unambiguous:

```orv
let x = 42          // inferred as i32
let y = 3.14        // inferred as f64
let name = "orv"   // inferred as string
let flag = true     // inferred as bool
```

Explicit annotation is required when the compiler cannot infer:

```orv
let mut items: Vec<string> = []
```

### Built-in Data Shapes

Beyond primitive types, three data shapes matter immediately in day-to-day orv code:

| Shape | Literal | Semantics |
|------|---------|-----------|
| `Vec<T>` | `[]` | Ordered dynamic vector, closest to a JavaScript array |
| plain object / record | `{}` | Fixed named fields, used for struct-shaped data and JSON object literals |
| `HashMap<K, V>` | `#{}` | Dynamic key/value map, distinct from plain objects |

`Vec<T>` is the sequence type used throughout the language. In JSON-shaped contexts, a `Vec<T>` is treated like an array and serializes as a JSON array.

Plain object values and `HashMap` values may both cross a JSON boundary as JSON objects, but they are not the same thing in the type system:

- plain object / record values use fixed named fields known from the source shape
- `HashMap` is a true dictionary/map abstraction for dynamic keys

### Union Types

```orv
type Number = i32 | f64
type Result = string | Error
type Nullable<T> = T?
```

### Nullable Types

Append `?` to make any type nullable:

```orv
let name: string? = void    // nullable string, void means "no value"
let count: i32? = 42        // nullable but has a value
```

`void` serves as both the return type for functions that return nothing and the literal value representing "no value" for nullable types (similar to `null` in other languages).

### Enums

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

### Structs

Structs are **headless data shapes** — similar to TypeScript interfaces. They describe the shape of a literal object. Structs have no methods, no constructors, no inheritance. They are purely structural types.

orv has no `class`. If you need stateful objects with methods, use [`define` with nested defines](./define.md) instead — it's more explicit, more composable, and avoids the complexity of `this` binding, prototype chains, and inheritance hierarchies.

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

Struct values are created with regular object literals, not `Type { ... }` constructor syntax. Use a variable annotation, parameter type, or return type to provide the struct type.

Struct-shaped values are plain object / record values built with `{}` literals. They are not `HashMap` values and they are not interchangeable with `#{}`.

### Braces `{}` — Plain Object vs Code Block

orv uses `{}` for both plain object / record literals and code blocks. The compiler distinguishes them by inspecting the first statement inside the braces:

| First line pattern | Interpretation |
|-------------------|----------------|
| `key: value`, `key: value`, ... | **Plain object / record literal** — named fields separated by commas or line breaks |
| `let`, `if`, `for`, `@`, `%`, expression, ... | **Code block** — executable statements |

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

The distinction is unambiguous: a bare `identifier: expression` with no keyword prefix is always a plain object / record field, while any keyword (`let`, `const`, `if`, `for`, etc.) or prefix (`@`, `%`) signals a code block. In multi-line object literals, commas are optional because the line breaks already delimit fields.

### Generics

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

### Function Types

Function types use the arrow syntax:

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

### Tuples

```orv
let pair: (i32, string) = (42, "hello")
let (x, y) = pair     // destructuring

function divmod(a: i32, b: i32): (i32, i32) -> {
  (a / b, a % b)
}
let (quotient, remainder) = divmod(10, 3)
```

---

## Variables & Mutability

orv follows Rust's immutability-by-default philosophy:

```orv
let x = 10          // immutable
let mut y = 20      // mutable
let sig z = 30      // reactive signal (mutable, triggers UI updates)
const PI = 3.14159  // compile-time constant
```

| Keyword | Mutable | Reactive | Scope |
|---------|---------|----------|-------|
| `let` | No | No | Block |
| `let mut` | Yes | No | Block |
| `let sig` | Yes | Yes | Block (tracked by reactivity system) |
| `const` | No | No | Module |

### Destructuring

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
