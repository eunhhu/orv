# Custom Nodes & Domain Blocks — `define`

[← Back to Index](./index.md)

---

## What `define` Is For

`define` declares a reusable `@node` or domain block.

It is **not** a class system, and it is **not** a function-style builder API.

Use:

- `function` for reusable logic and value computation
- `define` for reusable node/domain structure invoked with `@Name ...`

If a `define` declares a domain root such as `@html`, `@route`, `@design`, or another custom node/domain, it must be used as node syntax:

```orv
@Name %key=value custom-token {
  // child block
}
```

Never:

```orv
Name(...)
```

## Basic Syntax

```orv
define Name(params...) -> @domain {
  // body
}
```

- `Name`: usually PascalCase
- `params`: named inputs passed from `%key=value`
- `@domain`: the node/domain root this define expands into
- body: node/domain body, not a function return body
- keep the declaration header at `-> @domain`; put invocation tokens/properties at the call site or on nodes inside the body

## Invocation Contract

Defines are invoked as nodes.

```orv
define Button(label: string, variant: string?) -> @button {
  if variant == "primary" {
    @text "Primary"
  }

  @text label
}

@Button %label="Save" %variant="primary"
@Button %label="Cancel"
```

At the call site:

- `%key=value` passes named parameters
- bare words after `@Name` are custom tokens
- an optional `{ ... }` block becomes `@children`

## Custom Tokens with `@token`

Defines can read positional tokens from the invocation line.

```orv
define Notice(message: string) -> @div {
  if @token warning {
    @text "Warning"
  } else if @token error {
    @text "Error"
  } else {
    @text "Info"
  }

  @text message
}

@Notice warning %message="Check your input"
@Notice error %message="Something failed"
@Notice %message="FYI"
```

This keeps invocation in node form while still allowing lightweight, readable tokens.

## Children with `@children`

Child nodes passed in the invocation block are exposed through `@children`.

```orv
define Card(title: string) -> @div {
  @div rounded-lg shadow-md p-4 {
    @h2 font-bold text-lg {
      @text title
    }

    @div mt-2 {
      @children
    }
  }
}

@Card %title="Profile" {
  @text "Card content"
  @button "Action"
}
```

If a `define` needs root-level tokens or `%` properties, place them on nodes inside the body rather than in the `define` header.

## Custom Domain Blocks

Domain-oriented defines follow the same rule: they are still invoked as `@Name ...`, never as function calls.

```orv
define AuthRoute(authRequired: bool?) -> @route {
  if authRequired ?? false {
    @before {
      let token = @header "Authorization"
      if !token {
        @respond 401 { "error": "Unauthorized" }
      }
    }
  }

  @children
}

@AuthRoute GET /profile %authRequired={true} {
  @respond 200 { "ok": true }
}
```

The important part is the shape of the invocation:

```orv
@AuthRoute GET /profile %authRequired={true} {
  ...
}
```

Not:

```orv
AuthRoute("GET", "/profile", ...)
```

## Design Rule

When a `define` declares a reusable node/domain, think in this order:

1. `@Name`
2. `%named=value`
3. bare custom tokens
4. optional child block

That is the stable mental model for custom UI nodes and custom domain primitives.

## What to Use Instead of Function-style `define`

If the abstraction is fundamentally value-oriented or callable, use `function`.

```orv
function buildUserUrl(id: string) -> string {
  "/api/users/{id}"
}

function clamp(value: i32, min: i32, max: i32) -> i32 {
  if value < min {
    min
  } else if value > max {
    max
  } else {
    value
  }
}
```

`function` is for computation.
`define` is for reusable node/domain structure.

## Summary

| Goal | Use |
|------|-----|
| Reusable calculation | `function` |
| Declaration shape | `define X(...) -> @domain { ... }` |
| Invocation shape | `@X %key=value token { ... }` |
| Named inputs | `%key=value` |
| Lightweight modifiers | custom tokens + `@token` |
| Nested content | `@children` |
