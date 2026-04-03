# Custom Nodes — `define`

[← Back to Index](./index.md)

---

## Why `define`, Not `class`

orv has no `class` keyword. There is no `new`, no `this`, no inheritance, no prototypes. This is intentional.

`define` replaces every role that `class` traditionally fills:

| Traditional OOP | orv equivalent |
|----------------|-----------------|
| Class with methods | `define` with nested `define`s |
| Constructor | `define` parameters |
| Instance state | `let` / `let mut` / `let sig` inside `define` |
| Encapsulation | Closure scope (inner variables are private by default) |
| Polymorphism | `define` returning different `@` nodes based on params |
| Composition | `@children` + nested `define` |
| Singleton | Top-level `define` called once |

The reasoning: orv is a **node-oriented language**. Everything is either a node (`@`), a property (`%`), or a statement. Classes introduce a parallel object system that competes with the node tree. `define` keeps everything in one unified model.

## Basic Syntax

```orv
define Name(params...) -> returnNode {
  // body
}
```

- **`Name`**: PascalCase by convention for UI components, camelCase for utilities
- **`params`**: typed parameters, received as `%` properties at call site
- **`-> returnNode`**: the root node or value this define produces
- **body**: children, properties, logic — same three-role rules as any `{ }` block

## Simple Component

```orv
define Button(label: string, variant: string?) -> @button label rounded-md {
  when variant {
    "primary"   -> %class="bg-blue-500 text-white"
    "danger"    -> %class="bg-red-500 text-white"
    _           -> %class="bg-gray-200 text-gray-800"
  }
}

// Usage — invoked as a node with @
@Button %label="Submit" %variant="primary"
@Button %label="Cancel"
```

## Positional Tokens with `@token`

`define` can inspect positional tokens (bare words) from the invocation line. `@token` checks if a specific token is present:

```orv
define Alert(message: string) -> @div p-4 rounded-md {
  if @token warning {
    %class="bg-yellow-100 text-yellow-800"
  } else if @token error {
    %class="bg-red-100 text-red-800"
  } else {
    %class="bg-blue-100 text-blue-800"
  }
  @text message
}

// Usage — tokens are bare words after @Identifier
@Alert warning %message="Check your input"
@Alert error %message="Something failed"
@Alert %message="Just so you know"
```

`@token` with a regex pattern matches dynamic tokens:

```orv
define Listen() -> {
  port = @token \d+    // captures the first numeric token
}

// Usage
@Listen 8080           // port = 8080
```

## Children with `@children`

Any nodes placed inside the invocation block are available as `@children` inside the define:

```orv
define Card(title: string) -> @div rounded-lg shadow-md p-4 {
  @h2 font-bold text-lg "{title}"
  @div mt-2 {
    @children
  }
}

// Usage — block contents become @children
@Card %title="Settings" {
  @text "Card body content"
  @button "Save"
}

// No children — @children renders nothing
@Card %title="Empty Card"
```

## Inner State

Variables declared inside `define` are **private to that instance**. Each invocation gets its own closure:

```orv
define Counter(initial: i32?) -> @div {
  let sig count: i32 = initial ?? 0

  @text "Count: {count}"
  @hstack gap-2 {
    @button "-" %onClick={count -= 1}
    @button "+" %onClick={count += 1}
    @button "Reset" %onClick={count = initial ?? 0}
  }
}

// Each instance has independent state
@Counter %initial={0}     // its own count
@Counter %initial={100}   // its own count, starts at 100
```

---

## Advanced Patterns

### Nested `define` — The `class` Killer

`define` blocks can contain nested `define`s, creating inner APIs. This is how orv replaces classes with methods:

```orv
define createServer() -> {
  let sig port: i32 = 8000
  let mut routes: Vec<Route> = []
  let server_instance = @io.serve(port)

  define listen(p: i32) -> {
    port = p
  }

  define route(method: string, path: string, handler: _ -> void) -> {
    let nextRoute: Route = { method, path, handler }
    routes.push(nextRoute)
  }

  define start() -> {
    @io.out "Server listening on port {port}"
    for r of routes {
      server_instance.register(r)
    }
  }

  // Return an interface — callers see listen, route, start
  // but not port, routes, or server_instance
  return { listen, route, start }
}

// Usage — looks like a class instance, but it's just closures
let app = createServer()
app.listen(3000)
app.route("GET", "/", _ -> return @response 200 { "ok": true })
app.start()
```

This pattern gives you:
- **Encapsulation**: `port`, `routes`, `server_instance` are not accessible outside
- **State**: each `createServer()` call gets its own isolated state
- **Methods**: `listen`, `route`, `start` are just functions that close over the shared state
- **No `this`**: no binding confusion, no `this` in callbacks

### Builder Pattern

```orv
define createQuery(table: string) -> {
  let mut conditions: Vec<string> = []
  let mut limit_val: i32? = void
  let mut order_by: string? = void

  define where(condition: string) -> {
    conditions.push(condition)
  }

  define limit(n: i32) -> {
    limit_val = n
  }

  define orderBy(field: string) -> {
    order_by = field
  }

  define build(): string -> {
    let mut sql = "SELECT * FROM {table}"
    if conditions.len() > 0 {
      sql = sql + " WHERE " + conditions.join(" AND ")
    }
    if order_by != void {
      sql = sql + " ORDER BY {order_by}"
    }
    if limit_val != void {
      sql = sql + " LIMIT {limit_val}"
    }
    sql
  }

  return { where, limit, orderBy, build }
}

let q = createQuery("users")
q.where("age > 18")
q.where("active = true")
q.orderBy("name")
q.limit(10)
let sql = q.build()
// → "SELECT * FROM users WHERE age > 18 AND active = true ORDER BY name LIMIT 10"
```

### Domain Primitives

The built-in `@server`, `@route`, etc. are conceptually `define` blocks with domain context. You can create your own domain primitives:

```orv
define ApiGroup(prefix: string) -> {

  define get(path: string, handler: _ -> void) -> {
    @route GET {prefix}{path} {
      try {
        handler()
      } catch e {
        return @response 500 { "error": e.message }
      }
    }
  }

  define post(path: string, handler: _ -> void) -> {
    @route POST {prefix}{path} {
      try {
        handler()
      } catch e {
        return @response 500 { "error": e.message }
      }
    }
  }

  return { get, post }
}

// Usage
@server {
  @listen 8080

  let users = ApiGroup("/api/users")

  users.get("/", _ -> {
    let all = await db.findAllUsers()
    return @response 200 { "users": all }
  })

  users.post("/", _ -> {
    let { name, email } = @body
    let user = await db.createUser(name, email)
    return @response 201 { "user": user }
  })
}
```

### State Machine

```orv
define createFetcher<T>(fetchFn: _ -> T) -> {
  let sig state: string = "idle"
  let sig data: T? = void
  let sig error: string? = void

  define execute() -> {
    state = "loading"
    data = void
    error = void
    try {
      data = await fetchFn()
      state = "success"
    } catch e {
      error = e.message
      state = "error"
    }
  }

  define reset() -> {
    state = "idle"
    data = void
    error = void
  }

  return { state, data, error, execute, reset }
}

// Usage in UI
define UserProfile(userId: i32) -> @div {
  let fetcher = createFetcher(_ -> http.get("/api/users/{userId}"))

  %onMount={
    fetcher.execute()
  }

  when fetcher.state {
    "idle"    -> @text "Ready"
    "loading" -> @text "Loading..."
    "success" -> {
      @h1 "{fetcher.data.name}"
      @p "{fetcher.data.email}"
    }
    "error"   -> @text text-red-500 "Error: {fetcher.error}"
    _         -> @text "Unknown state"
  }
}
```

### Generic Types

```orv
define List<T>(items: Vec<T>, renderItem: T -> void) -> @ul {
  for item of items {
    @li {
      renderItem(item)
    }
  }
}

// Usage
@List<User> %items={users} %renderItem={user: User -> {
  @text "{user.name} ({user.email})"
}}

```

### Exported Definitions

```orv
// components/Button.orv
pub define PrimaryButton(label: string) -> @button label {
  %class="bg-blue-500 text-white px-4 py-2 rounded-md hover:bg-blue-600"
}

pub define DangerButton(label: string) -> @button label {
  %class="bg-red-500 text-white px-4 py-2 rounded-md hover:bg-red-600"
}

// Private — not accessible outside this file
define baseButtonStyles() -> "px-4 py-2 rounded-md font-medium"
```

---

## Summary: `define` Capabilities

| Capability | Pattern |
|-----------|---------|
| UI Component | `define Name() -> @div { ... }` |
| Utility Function | `define helper() -> { return value }` |
| Stateful Object | `define create() -> { let state; return { methods } }` |
| Builder | `define builder() -> { return { chain, build } }` |
| State Machine | `define machine() -> { let sig state; return { state, transitions } }` |
| Domain Primitive | `define group() -> { define innerRoute(); return api }` |
| Higher-Order | `define hoc<T>(component: T) -> @div { ... }` |
