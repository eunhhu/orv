# UI & Design Domain

[← Back to Index](./index.md)

---

## UI Domain

The UI domain is active inside `@html`, `@body`, and UI-specific nodes like `@div`, `@vstack`, `@hstack`.

### HTML Structure

```miol
let page: html = @html {
  @head {
    @title "My Application"
    @meta description "A miol app"
    @meta viewport "width=device-width, initial-scale=1"
  }

  @body {
    @div flex flex-col min-h-screen {
      @Header
      @main flex-1 {
        @Outlet
      }
      @Footer
    }
  }
}
```

### Elements & Tailwind

HTML elements are nodes. Tailwind classes are positional tokens — no `class=` needed:

```miol
@div flex flex-col gap-4 p-6 {
  @h1 text-2xl font-bold "Welcome"
  @p text-gray-500 "This is a paragraph"
  @button bg-blue-500 text-white rounded-md "Click me"
}
```

Tailwind classes and a string literal can coexist on one line as positional tokens. Keep it readable — if the line gets too long, consider extracting a `define`.

### Layout Shorthands

```miol
@vstack gap-4 {       // vertical stack (flex flex-col)
  @text "Top"
  @text "Bottom"
}

@hstack gap-2 {       // horizontal stack (flex flex-row)
  @text "Left"
  @text "Right"
}
```

### Event Handling

```miol
// Inline
@button "Click" %onClick={count += 1}

// Inline continuation
@input flex-1 rounded-md
  %type="text"
  %value={query}
  %onInput={query = $0.target.value}

// Block
@button "Submit" {
  %onClick={
    let result = await submitForm()
    if result.ok {
      navigate("/success")
    }
  }
}
```

### Conditional Rendering

```miol
@div {
  if isLoggedIn {
    @text "Welcome, {username}"
    @button "Logout" %onClick={logout()}
  } else {
    @button "Login" %onClick={showLogin()}
  }
}
```

### List Rendering

```miol
@ul {
  for item of items {
    @li "{item.name} — {item.description}"
  }
}

// With index
@ol {
  for (i, task) of tasks.enumerate() {
    @li "#{i + 1}: {task.title}"
  }
}
```

### Children

Components receive children via `@children`:

```miol
define Card(title: string) -> @div rounded-lg shadow-md p-4 {
  @h2 font-bold text-lg "{title}"
  @div mt-2 {
    @children
  }
}

// Usage
@Card %title="Profile" {
  @text "Card content goes here"
  @button "Action"
}
```

### Lifecycle

Lifecycle hooks are `%` properties:

```miol
define Timer() -> @div {
  let sig seconds: i32 = 0
  let mut interval: Interval? = void

  %onMount={
    interval = @io.interval 1000 {
      seconds += 1
    }
  }

  %onUnmount={
    interval?.clear()
  }

  @text "{seconds}s elapsed"
}
```

| Hook | Trigger |
|------|---------|
| `%onMount` | Node is added to the DOM |
| `%onUnmount` | Node is removed from the DOM |

### Inline Styles

```miol
@div {
  %style={
    backgroundColor: "red",     // camelCase
    // "background-color": "red", // kebab-case also works
    padding: "1rem"
  }
  @text "Styled div"
}
```

**Best Practice:** Prefer Tailwind classes for styling. Use `%style` only for dynamic values that depend on signals or computed state.

### String Interpolation in Templates

```miol
let sig name: string = "World"

@text "Hello, {name}"          // reactive — updates when name changes
@h1 "Page {currentPage} of {totalPages}"
```

---

## Design Domain

The `@design` block defines design tokens — colors, sizes, fonts, and themes.

```miol
@design {
  // Theme-specific tokens
  @theme light {
    @color primary #1a1a1a
    @color foreground #ffffff
    @color background #f5f5f5
  }

  @theme dark {
    @color primary #ffffff
    @color foreground #1a1a1a
    @color background #0a0a0a
  }

  // Global tokens (theme-independent)
  @color accent #3b82f6
  @color error #ef4444

  @size base 16px
  @size sm 14px
  @size lg 20px
  @size radius 8px

  @font sans "Inter, system-ui, sans-serif" 16px weight-400 line-1.5
  @font mono "JetBrains Mono, monospace" 14px weight-400 line-1.6
}
```

### Using Design Tokens

Design tokens are referenced as Tailwind-style classes in UI nodes:

```miol
@h1 text-primary bg-background font-sans "Hello"
@p text-foreground text-base "Body text"
@span text-error text-sm "Error message"
```

**Best Practice:** Define all colors as tokens in `@design`. Never use hardcoded hex values in UI nodes.
