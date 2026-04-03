# Server Domain

[← Back to Index](./index.md)

---

## Basic Server

The `@server` block defines an HTTP server with routes, middleware, and request handling.

```orv
@server {
  @listen 8080

  @route GET / {
    @serve ./public
  }
}
```

## Routes

```orv
@server {
  @listen 8080

  // Token order is flexible — method and path are parsed by keyword
  @route GET /api/users {
    return @response 200 {
      "users": []
    }
  }

  @route POST /api/users {
    let { name, email } = @body
    let user = await db.createUser(name, email)
    return @response 201 { "user": user }
  }

  // Wildcard
  @route * {
    @serve htmlString
  }
}
```

## Nested Routes

Routes nest naturally. Child routes inherit the parent's path prefix and middleware:

```orv
@server {
  @listen 8080

  @route /api {

    @before {
      @io.out "API request: {@method} {@path}"
    }

    @route GET /users {
      // handles GET /api/users
      let skip = @query "skip"
      let limit = @query "limit"
      let users = await db.findUsers(skip, limit)
      return @response 200 { "users": users }
    }

    @route GET /users/:id {
      // handles GET /api/users/:id
      let id = @param "id"
      let user = await db.findUser(id)
      return @response 200 { "user": user }
    }

    @route POST /users {
      // handles POST /api/users
      let { name, email } = @body
      let user = await db.createUser(name, email)
      return @response 201 { "user": user }
    }
  }
}
```

## Request Accessors

| Accessor | Returns | Description |
|----------|---------|-------------|
| `@body` | parsed body | Request body (JSON parsed) |
| `@param "key"` | `string?` | URL path parameter (`:id` in `/users/:id`) |
| `@query "key"` | `string?` | Query string parameter (`?skip=10`) |
| `@header "key"` | `string?` | Request header value |
| `@method` | `string` | HTTP method |
| `@path` | `string` | Request path |
| `@context "key"` | any | Value set by `@before` middleware |

```orv
// @param — path parameters from the route pattern
@route GET /users/:id {
  let id = @param "id"        // from /users/42 → "42"
}

// @query — query string parameters
@route GET /users {
  let skip = @query "skip"    // from /users?skip=10 → "10"
  let limit = @query "limit"  // from /users?limit=20 → "20"
}

// @body — parsed request body
@route POST /users {
  let { name, email } = @body // JSON body parsed
}

// @header — request headers
@route * {
  let token = @header "Authorization"
  let contentType = @header "Content-Type"
}
```

## Response

Responses are returned with `return @response`:

```orv
// Simple
return @response 200 { "message": "OK" }

// With headers
return @response 200 %header={
  "Content-Type": "application/json"
  "X-Custom": "value"
} {
  "data": result
}

// Early return (guard clause)
if !authorized {
  return @response 401 { "error": "Unauthorized" }
}

// Empty body
return @response 204 {}
```

`@response` is always used with `return` — it terminates the route handler and sends the HTTP response.

At the transport boundary:

- `Vec<T>` payloads become JSON arrays
- plain `{}` object payloads become JSON objects with fixed named fields
- `HashMap<string, T>` payloads also serialize as JSON objects, but remain map values in the language rather than plain record/object values

## Middleware

```orv
@route /api {

  // Runs before every child route
  @before {
    let token = @header "Authorization"
    let verified = await jwt.verify(token, SECRET)
    if !verified {
      return @response 401 { "error": "Unauthorized" }
    }
    // Pass data to route handlers via @context
    return @context {
      userId: verified.sub
    }
  }

  // Runs after every child route
  @after {
    @io.out "Request completed"
  }

  @route GET /profile {
    let userId = @context "userId"
    let user = await db.findUser(userId)
    return @response 200 { "user": user }
  }
}
```

## Serving Static Files & HTML

```orv
@route GET / {
  @serve ./public             // static directory
}

@route GET /app {
  @serve htmlString           // orv html node
}

@route GET /js {
  @serve ./public/bundle.js   // specific file
}
```

## Routes as Variables — Fullstack RPC

Routes assigned to variables become **callable endpoints** from the UI domain. This is orv's built-in fullstack RPC — no separate API client, no manual fetch URLs, no code generation step.

Route references follow normal lexical scope rules. The UI that calls `.fetch()` must be defined in the same scope as the route reference or receive that route reference explicitly.

```orv
@server {
  @listen 8000

  let userService = @route GET /api/user {
    let users = await db.findAll()
    return @response 200 { "users": users }
  }

  let createUser = @route POST /api/user {
    let { name, email } = @body
    let user = await db.create(name, email)
    return @response 201 { "user": user }
  }

  @route GET / {
    let page = @html {
      @body {
        let sig data = await userService.fetch()

        @div {
          if data != void {
            for user of data.users {
              @text "{user.name}"
            }
          } else {
            @text "Loading..."
          }
        }

        @button "Add User" %onClick={
          await createUser.fetch(body={
            name: "Kim"
            email: "kim@example.com"
          })
          data = await userService.fetch()
        }
      }
    }

    @serve page
  }
}
```

**How it works:**

| Concept | Description |
|---------|-------------|
| `let x = @route ...` | Assigns a route to a variable, making it a callable reference |
| `x.fetch()` | Calls the route from the client — compiles to a `fetch()` with the correct URL and method |
| `x.fetch(body={...})` | Sends a request body (for POST/PUT/PATCH) |
| `x.fetch(query={...})` | Appends query parameters |
| `x.fetch(header={...})` | Adds custom headers |
| `x.fetch(param={...})` | Path parameters (`:id` in `/users/:id`) |

**Why this matters:**

- **Type safety across the boundary.** The compiler knows the response shape from `@response`, so `data.users` is type-checked at compile time.
- **No URL strings in UI code.** Route paths are an implementation detail — the UI references the variable, not the URL.
- **Refactoring safety.** Rename the route path, and all `.fetch()` calls still work because they reference the variable, not a hardcoded string.
- **Zero boilerplate.** No API client library, no OpenAPI spec, no codegen step. The connection between server and client is the variable binding.

### Multiple Route References

```orv
@server {
  @listen 8000

  let getUsers = @route GET /api/users {
    return @response 200 { "users": await db.findAll() }
  }

  let getUser = @route GET /api/users/:id {
    let id = @param "id"
    return @response 200 { "user": await db.findUser(id) }
  }

  let deleteUser = @route DELETE /api/users/:id {
    let id = @param "id"
    await db.deleteUser(id)
    return @response 204 {}
  }

  @route GET /dashboard {
    let page = @html {
      @body {
        let sig users = await getUsers.fetch()
        let sig profile = await getUser.fetch(param={ id: "42" })

        @button "Delete" %onClick={
          await deleteUser.fetch(param={ id: profile.user.id })
          users = await getUsers.fetch()
        }
      }
    }

    @serve page
  }
}
```

## Server as Function

Servers can be created dynamically:

```orv
function myServer(port: i32, root: string) -> @server {
  @listen port
  @route * {
    @serve root
  }
}

myServer(8080, "./public")
myServer(3000, "./admin")
```

---

## Domain Contexts & Validation

orv enforces **compile-time domain validation**. Each top-level block (`@html`, `@server`, `@design`) defines a context that restricts which `@` nodes are valid inside it.

Because the compiler sees all domains together, it can optimize across domain boundaries. When `@server` serves an `@html` page, the compiler knows both sides — it can optimize the communication between them, inline what can be inlined, and produce output tailored to the project's specific domain relationships.

```orv
// Valid — each node belongs to its correct domain
@server {
  @listen 8080
  @route / { @serve page }
}

@html {
  @body {
    @div { @text "Hello" }
  }
}

@design {
  @theme dark {
    @color primary #fff
  }
}
```

```orv
// Compile errors — domain mismatch
@server {
  @div { ... }           // ERROR: @div is not valid in server context
}

@html {
  @body {
    @listen 8080         // ERROR: @listen is not valid in UI context
    @route / { ... }     // ERROR: @route is not valid in UI context
  }
}

@design {
  @route / { ... }       // ERROR: @route is not valid in design context
}
```

### Cross-Domain References

Use variables to bridge domains:

```orv
let page = @html {
  @body {
    @div { @text "Hello" }
  }
}

@server {
  @listen 8080
  @route / {
    @serve page   // reference, not inline — keeps domains separate
  }
}
```
