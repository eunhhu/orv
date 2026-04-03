# Collections, Error Handling & Async

[← Back to Index](./index.md)

---

## Collections

### Vec (Dynamic Array)

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

### HashMap

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

### Iteration

```orv
for (key, value) of scores {
  @io.out "{key}: {value}"
}

for item of vec {
  @io.out item
}
```

---

## Error Handling

orv uses `try` / `catch` blocks for error handling:

```orv
try {
  let data = await fetchData()
  process(data)
} catch e {
  @io.out "Error: {e.message}"
}
```

### Typed Catch

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

### Try in Expressions

```orv
let user: User = try db.findUser(id) catch {
  { name: "anonymous", age: 0 }
}
```

**Best Practice:** Prefer specific catch clauses over generic ones. In server routes, always catch errors and return appropriate HTTP status codes.

---

## Async / Await

### Basics

Functions that perform I/O or network operations are declared `async`:

```orv
async function fetchUser(id: i32): User -> {
  let res = await http.get("/api/users/{id}")
  res.json()
}
```

### Top-Level Await

`await` works **everywhere** — no `async` wrapper required at the top level:

```orv
let config = await loadConfig()
let db = await Database.connect(config.dbUrl)

@server {
  @listen config.port
}
```

### Concurrent Execution

```orv
// Parallel fetch
let (users, posts) = await (fetchUsers(), fetchPosts())

// Or explicitly
let usersFuture = fetchUsers()
let postsFuture = fetchPosts()
let users = await usersFuture
let posts = await postsFuture
```

**Best Practice:** Use concurrent tuple await for independent async operations that can run in parallel.
