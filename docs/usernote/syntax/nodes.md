# Node System & Reactivity

[← Back to Index](./index.md)

---

## Node System

The `@` / `%` system is the core abstraction of orv. Every domain (UI, server, design) is expressed through the same node grammar.

### `@` — Structural Nodes

```orv
@identifier tokens... {
  // children and logic
}
```

Nodes can carry:
- **Positional tokens**: parsed by keyword (order-independent where applicable)
- **String literals**: `"text content"`
- **Tailwind classes**: `rounded-md flex items-center` (in UI context)
- **Inline `%` properties**: `%key=value` on the same line or on immediately following indented continuation lines

### `%` — Properties

Properties configure the node they belong to:

```orv
// Inline — on the same line
@button "Click" %onClick=handler() %disabled=false

// Inline continuation — still part of the same node statement
@input flex-1 rounded-md
  %type="text"
  %placeholder="Search"
  %value={query}

// Inner — inside the block, applies to parent
@div {
  %class="container"
  %style={
    display: "flex"
    gap: "1rem"
  }
  @text "Content"
}

// Multi-line property value — use { } when the value spans multiple statements
%onClick={
  counter += 1
  @io.out "clicked"
}
```

Use inline `%` properties when they conceptually belong to the node declaration itself. Use inner `%` properties when the node already has a body and keeping the configuration inside that block is clearer.

### `@io` — Standard I/O

```orv
@io.out "Hello, world"        // stdout
@io.err "Something went wrong" // stderr
```

### `@env` — Environment Variables

`@env` reads an environment variable and participates in normal type inference. If the surrounding context expects a concrete type, the compiler coerces and validates the env value against that type. Without type context, it defaults to `string`.

```orv
let port: i32 = @env PORT      // inferred/coerced as i32 from the annotation
let secret = @env JWT_SECRET   // string (no stronger type context)

// Inline usage
@listen @env PORT              // inferred from @listen's expected type
```

---

## Reactivity & Signals

### Signal Declaration

```orv
let sig count: i32 = 0
```

A `sig` variable is **mutable** and **reactive**. When its value changes, any UI node or derived signal that depends on it is automatically updated.

### Reading & Writing

Signals are read and written like normal variables — no special accessor needed:

```orv
let sig count: i32 = 0

// Read
@text "Count: {count}"

// Write
count += 1
count = 0
```

### Derived Signals

Any `sig` whose initial value references another `sig` is automatically derived:

```orv
let sig count: i32 = 0
let sig doubled: i32 = count * 2      // auto-derived
let sig label: string = "Count: {count}"  // auto-derived

// doubled and label update whenever count changes
```

### Fine-Grained Updates

orv's reactivity is fine-grained: when `count` changes, only the specific DOM nodes that reference `count` are updated — not the entire component tree.

```orv
define Counter() -> @div {
  let sig count: i32 = 0

  // Only this text node re-renders when count changes
  @text "Count: {count}"

  // This text node never re-renders
  @text "This is static"

  @button "+" %onClick={count += 1}
}
```

### Signals in Collections

```orv
let sig items: Vec<string> = ["a", "b", "c"]

// Mutating the collection triggers updates
items.push("d")
items.pop()

// Derived from collection
let sig itemCount = items.len()
```
