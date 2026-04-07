# Custom Domains & Node Contracts — `define`

[← Back to Index](./index.md)

---

## What `define` Is For

`define` is the abstraction that describes reusable orv surface grammar.

It is not a class, not a constructor system, and not a function-style builder API.

Use:

- `function` for reusable computation and value-oriented logic
- `define` for reusable `@domain` / `@node` surface structure

If something must be invoked with `@Name ...`, it belongs to `define`.

If something must be called as `name(...)`, it belongs to `function`.

## Two Shapes of `define`

### 1. Node / Domain Root Define

```orv
define Button(label: string) -> @button {
  @text label
}

@Button %label="Save"
```

This shape says: "the invocation surface is `@Button ...` and it expands into a node-like domain root."

### 2. Domain Family Define

```orv
define html() -> {
  let ssr: bool = @token exist "ssr"

  define head() -> {
    @children
  }

  define body() -> {
    @children
  }
}

@html ssr {
  @head {}
  @body {}
}
```

This shape says: "the invocation surface is `@html ...`, and nested `define` declarations describe the legal subdomains that appear inside that block."

This is the mental model used by [`project-e2e`](/Users/sunwoo/work/miol/fixtures/project-e2e).

## Surface Grammar

The canonical node/domain surface is space-structured:

```orv
@domain subtoken subtoken2 %key=value data {
  // nested block content
}
```

Spaces matter. In a node head, spaces act as the primary structural splitter.

After the leading `@domain`, each space-delimited segment belongs to one of four roles:

| Role | Shape | Meaning |
|------|-------|---------|
| domain root | `@route`, `@html`, `@div` | selects the active node/domain |
| subtoken | `GET`, `/api`, `ssr`, `1m` | lightweight semantic modifier interpreted by the domain |
| property | `%key=value` | named configuration |
| data | `port`, `page`, `"description"`, `200` | ordinary value payload carried in the head |

An optional `{ ... }` block after the head carries nested slot content.

## Subtokens

Subtokens are bare space-separated units after the domain root.

Examples:

```orv
@route GET /users
@html ssr
@RateLimit 1000 1m
```

Subtokens are not "arguments" in the function-call sense. They are domain-specific surface markers.

Typical interpretations:

- HTTP verbs and paths in `@route`
- render modifiers such as `ssr`
- duration or size shorthands such as `1m`
- style/domain modifiers in UI nodes

Use `@token` inside `define` to inspect them:

```orv
let has_ssr: bool = @token exist "ssr"
let duration: string = @token match "\\d+[smhd]"
```

## Properties

Named configuration still uses `%key=value`.

```orv
@Card %title="Profile"
@Button %variant="primary" %disabled={isLocked}
```

Properties are the stable place for named inputs and configuration that should not depend on positional order.

## Data in the Head

Head data is not limited to `{ ... }` blocks.

Any normal data expression may appear in the node head:

```orv
@listen port
@serve page
@respond 200 result
@meta "description" "My App description"
```

`data` means "ordinary payload value attached to the head," not "must be an inner block."

Depending on the domain, head data may be:

- an identifier
- a literal
- a path-like token
- a computed value
- an HTML/page reference

## Nested Slot Content and `@children`

`@children` is the syntax that projects nested block content passed into a `define`.

Although the keyword is `@children`, the concept is not DOM-specific. It is the general slot-content operator for any domain.

```orv
define Card(title: string) -> @div {
  @div {
    @text title
    @children
  }
}

@Card %title="Profile" {
  @text "Body"
}
```

Read it as:

- invocation block content
- nested slot content
- projected through `@children`

Do not think of it as a UI-only API.

## `define` as a Contract

A `define` establishes a contract over four axes:

1. legal subtokens
2. legal `%key=value` properties
3. legal head data values
4. legal nested slot content

That contract may be implicit or explicit in the body:

```orv
define html() -> {
  let ssr: bool = @token exist "ssr"

  define head() -> {
    @children
  }

  define body() -> {
    let font_token: string = @token match "text-[(sm)(base)(lg)]"
    @children
  }
}
```

The docs standardize on this model even where explicit schema syntax is still evolving.

In other words: future constraint syntax should refine this contract model, not replace it.

## Naming Guidance

- lower-case names are appropriate when defining a domain root such as `html`, `head`, `body`
- PascalCase names are appropriate for reusable component-like or application-specific defines such as `Button`, `Card`, `RateLimit`

The important distinction is invocation form, not letter case:

- `@html ...`
- `@Button ...`
- never `html(...)`
- never `Button(...)`

## Function vs `define`

Use `function` when the abstraction is fundamentally callable and returns a value:

```orv
function buildUserUrl(id: string) -> string {
  "/api/users/{id}"
}
```

Use `define` when the abstraction defines a surface grammar for an `@domain` block:

```orv
define RateLimit(max: i32?, time: string?) -> @after {
  let parsed_time = time ?? @token match "\\d+[smhd]" ?? "1m"
  let parsed_max = max ?? @token match "\\d+" ?? 1000
  @children
}
```

## Summary

| Goal | Use |
|------|-----|
| reusable computation | `function` |
| reusable `@domain` surface | `define` |
| domain root declaration | `define X(...) -> @domain { ... }` or `define x() -> { ... }` |
| invocation shape | `@X subtoken %key=value data { ... }` |
| subtoken inspection | `@token exist`, `@token match` |
| nested slot projection | `@children` |
