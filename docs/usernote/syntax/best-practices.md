# Best Practices & Examples

[← Back to Index](./index.md)

---

## 1. File Organization

```
project/
├── main.orv                // entry: server + wiring
├── design.orv              // @design tokens
├── components/
│   ├── Button.orv          // pub define Button
│   ├── Card.orv
│   ├── Input.orv
│   └── Layout.orv
├── pages/
│   ├── Home.orv
│   ├── About.orv
│   └── Dashboard.orv
├── server/
│   ├── routes.orv          // route definitions
│   ├── middleware.orv       // @before / @after blocks
│   └── db.orv              // database helpers
└── libs/
    ├── auth.orv
    └── validation.orv
```

## 2. Signal Hygiene

```orv
// Good — signals only for values that drive UI updates
let sig count: i32 = 0
let sig username: string = ""

// Bad — using sig for non-reactive data
let sig API_URL: string = "https://api.example.com"  // use const instead
let sig tempCalc: i32 = someExpensiveCalc()           // use let instead
```

## 3. Keep `define` Blocks Focused

```orv
// Good — one responsibility per define
define UserAvatar(url: string, size: i32) -> @img %src={url} rounded-full {
  %style={
    width: "{size}px"
    height: "{size}px"
  }
}

define UserCard(user: User) -> @div flex items-center gap-3 {
  @UserAvatar %url={user.avatarUrl} %size={48}
  @div {
    @text font-bold "{user.name}"
    @text text-gray-500 text-sm "{user.email}"
  }
}

// Bad — doing too much in one define
define UserSection(users: Vec<User>) -> @div {
  // fetching, filtering, rendering, pagination... all in one block
}
```

## 4. Error Handling in Server Routes

```orv
// Good — always handle errors in routes
@route POST /api/users {
  try {
    let { name, email } = @body
    let user = await db.createUser(name, email)
    return @response 201 { "user": user }
  } catch e: ValidationError {
    return @response 400 { "error": e.message }
  } catch e {
    @io.err "Unexpected: {e.message}"
    return @response 500 { "error": "Internal server error" }
  }
}

// Bad — unhandled errors crash the server
@route POST /api/users {
  let { name, email } = @body       // throws if body is malformed
  let user = await db.createUser(name, email)  // throws on DB error
  return @response 201 { "user": user }
}
```

## 5. Use Design Tokens, Not Hardcoded Values

```orv
// Good
@design {
  @color primary #3b82f6
  @color text-muted #6b7280
  @size radius-md 8px
}

@button bg-primary text-white "Submit"
@p text-text-muted "Helper text"

// Bad
@button %style={ backgroundColor: "#3b82f6", color: "#ffffff" } "Submit"
@p %style={ color: "#6b7280" } "Helper text"
```

## 6. Prefer Composition Over Complexity

```orv
// Good — compose small defines
define IconButton(icon: string, label: string) -> @button flex items-center gap-2 {
  @Icon %name={icon}
  @text "{label}"
}

define DangerButton(label: string) -> @button bg-red-500 text-white rounded-md {
  @text "{label}"
}

// Good — use define for repeated patterns
define ApiRoute(method: string, path: string) -> @route {
  @before {
    let token = @header "Authorization"
    if !token {
      return @response 401 { "error": "Unauthorized" }
    }
  }
  @children
}
```

## 7. Async Best Practices

```orv
// Good — parallel fetching
let (users, posts) = await (fetchUsers(), fetchPosts())

// Bad — sequential when parallel is possible
let users = await fetchUsers()
let posts = await fetchPosts()  // waits for users to finish first

// Good — error handling on async
let user: User = try await fetchUser(id) catch {
  @io.err "Failed to fetch user {id}"
  { name: "unknown", age: 0 }
}
```

## 8. Domain Separation

```orv
// Good — each domain in its own file or clearly separated
// design.orv
@design {
  @theme light { ... }
  @theme dark { ... }
}

// pages/Home.orv
pub define HomePage() -> @html {
  @body { ... }
}

// main.orv
import design.*
import pages.Home.HomePage

@server {
  @listen 8080
  @route / { @serve HomePage() }
}

// Bad — everything in one massive file with domains interleaved
```

---

## Full Example: Todo Application

```orv
// design.orv
@design {
  @theme light {
    @color primary #1a1a1a
    @color surface #ffffff
    @color border #e5e7eb
    @color text-muted #6b7280
  }

  @theme dark {
    @color primary #f5f5f5
    @color surface #1f2937
    @color border #374151
    @color text-muted #9ca3af
  }

  @font sans "Inter, sans-serif" 16px weight-400 line-1.5
}

// components/TodoItem.orv
import @std.io

pub define TodoItem(todo: Todo) -> @li flex items-center gap-3 p-3 border-b border-border {
  @input %type="checkbox" %checked={todo.done} %onChange={
    todo.done = !todo.done
  }

  if todo.done {
    @span text-text-muted line-through "{todo.title}"
  } else {
    @span text-primary "{todo.title}"
  }

  @button text-red-500 hover:text-red-700 "x" %onClick={
    todo.deleted = true
  }
}

// pages/Home.orv
import components.TodoItem

struct Todo {
  title: string
  done: bool
  deleted: bool
}

pub define HomePage() -> @html {
  @head {
    @title "orv Todo"
    @meta viewport "width=device-width, initial-scale=1"
  }

  @body font-sans bg-surface text-primary {
    @div max-w-md mx-auto py-8 {
      @h1 text-2xl font-bold mb-4 "orv Todo"

      let sig todos: Vec<Todo> = []
      let sig input: string = ""

      // Derived
      let sig remaining: i32 = todos.filter($0.done == false).len()

      @div flex gap-2 mb-4 {
        // Continued inline % properties are allowed on indented lines
        @input flex-1 border border-border rounded-md px-3 py-2
          %type="text"
          %placeholder="What needs to be done?"
          %value={input}
          %onInput={input = $0.target.value}
          %onKeyDown={
            if $0.key == "Enter" && input.len() > 0 {
              let nextTodo: Todo = {
                title: input
                done: false
                deleted: false
              }
              todos.push(nextTodo)
              input = ""
            }
          }

        @button bg-primary text-surface px-4 py-2 rounded-md "Add" %onClick={
          if input.len() > 0 {
            let nextTodo: Todo = {
              title: input
              done: false
              deleted: false
            }
            todos.push(nextTodo)
            input = ""
          }
        }
      }

      @ul {
        for todo of todos {
          if !todo.deleted {
            @TodoItem %todo={todo}
          }
        }
      }

      @p text-text-muted text-sm mt-4 "{remaining} items remaining"
    }
  }
}

// main.orv
import pages.Home.HomePage

let port: i32 = @env PORT

@server {
  @listen port

  @route GET / {
    @serve HomePage()
  }

  @route GET /static {
    @serve ./public
  }
}
```
