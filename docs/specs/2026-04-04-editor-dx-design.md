# orv Built-in Editor and DX Design

> Date: 2026-04-04
> Status: Draft
> Audience: language, compiler, and editor implementation planning

---

## 1. Purpose

This document defines the editor experience orv is designed to enable.

The goal is not to bolt language tooling onto a generic external IDE after the language is done. The goal is to shape the language, parser, analyzer, and runtime metadata so a built-in editor can provide the best possible editing experience from the beginning.

---

## 2. Core Position

orv is **not** slash-command-first.

The language is built around `@node` and `%property`, so the editor should be built around those same triggers:

- `@` opens a node palette for the current domain
- `%` opens a property palette for the current node
- `Shift + Up` / `Shift + Down` steps semantic values instead of only moving text
- the current cursor location determines what is legal, visible, and suggested

The reference interaction target is the quality of products like layouts.dev, but adapted to orv's grammar rather than copied from slash-command editors.

---

## 3. Editor Principles

### 3.1 Built-in Editor First

The best orv editing experience should come from an orv-native editor, not from depending on third-party IDE extension constraints.

External IDE integration can exist later, but it is not the primary product shape.

### 3.2 Domain-Aware Suggestions

The editor must understand domain context:

- inside `@html`, show UI nodes and UI-safe properties
- inside `@server`, show server nodes such as `@listen`, `@route`, `@response`, `@serve`
- inside `@design`, show design tokens and token properties
- inside `@route`, show route-specific request/response helpers

Suggestions should not be generic string lists. They should be legal choices filtered by semantic context.

### 3.3 Cursor-Local Editing

The cursor location must drive the UI:

- on `@`, show legal nodes for the current context
- on `%`, show legal properties for the current node
- on a property value, show value suggestions based on that property's type or token space
- on a route method/path slot, show route-aware affordances instead of generic completion

### 3.4 Structural Editing over Text Tricks

The editor should operate on syntax and semantic structure, not only on raw text.

Examples:

- increment `text-2xl` to `text-3xl`
- decrement spacing scales
- cycle enum-like property values
- step numeric literals up or down
- step color or size tokens within the same token family

### 3.5 Tree-sitter-grade Detail and Speed

The language service target is tree-sitter-grade responsiveness and structural precision:

- incremental-friendly parsing
- stable byte spans
- cheap cursor-context queries
- minimal re-analysis for small edits
- syntax trees rich enough for editor operations even before full semantic analysis finishes

This does **not** require embedding tree-sitter itself. It does require equivalent quality in detail, latency, and robustness.

---

## 4. Required User Interactions

### 4.1 `@` Node Palette

Typing `@` should open a palette whose items are filtered by domain.

Examples:

- in `@html`: `@div`, `@text`, `@button`, `@input`, `@body`
- in `@server`: `@listen`, `@route`, `@before`, `@after`
- in `@route`: `@response`, `@serve`

Each item should include:

- short description
- allowed context
- snippet shape
- key properties

### 4.2 `%` Property Palette

Typing `%` should show legal properties for the current node.

Examples:

- `@button`: `%onClick`, `%class`, `%style`
- `@input`: `%value`, `%placeholder`, `%type`, `%onInput`
- `@response`: `%header`

The editor should distinguish:

- inline `%` properties
- indented continuation `%` properties
- inner `%` properties inside a node body

### 4.3 Semantic Steppers

When the cursor is on a step-capable token, `Shift + Up` / `Shift + Down` should transform the token semantically.

Examples:

- `text-sm -> text-base -> text-lg -> text-xl`
- `p-2 -> p-3 -> p-4`
- `200 -> 201 -> 202`
- `true <-> false`

Steppers must be property-aware. A size stepper should not activate on unrelated identifiers.

### 4.4 Route-aware Editing

Inside `@route`, the editor should understand:

- method slot
- path slot
- route body
- legal response/serve nodes

Possible UI behaviors:

- method dropdown
- path parameter visualization
- response helper snippets

### 4.5 Hint-aware Editing

`@hint` should expose legal keys and values by target context.

Examples:

- `@hint protocol=` -> `json`, `binary`, `hybrid`
- `@hint render=` -> `ssr`, `csr`, `ssg`
- `@hint chunk=` -> `separate`, `inline`

---

## 5. Compiler and Language-Service Requirements

To enable the editor above, the compiler stack must expose more than a final HIR dump.

### 5.1 Syntax Requirements

The syntax layer must provide:

- stable byte spans for all relevant nodes and tokens
- error-tolerant parse results
- node/property boundaries even in partially invalid code
- incremental reparse strategy for small edits
- enough structure to answer cursor queries without full compilation

### 5.2 Semantic Requirements

The analyzer must provide:

- current domain context at cursor position
- current enclosing node
- legal child nodes for that context
- legal properties for that node
- property value categories
- diagnostics that remain usable while code is incomplete

### 5.3 Registry Requirements

orv needs a machine-readable metadata registry for:

- node definitions
- legal domains
- legal child relationships
- legal properties
- property value kinds
- stepper behavior
- documentation strings

This metadata must be compiler-owned, not duplicated separately in the editor.

---

## 6. Proposed Architecture

### 6.1 New Editor-facing Layer

Add an editor-oriented crate after the core syntax/analyzer foundation is stable:

| Crate | Purpose |
|------|---------|
| `orv-ide` | cursor context, completion, hover, stepper, editor metadata API |

### 6.2 Responsibility Split

- `orv-syntax`: tokens, spans, parse tree, incremental-friendly syntax data
- `orv-analyzer`: domain/type legality, semantic context, symbol info
- `orv-ide`: editor queries and UX-oriented APIs
- built-in editor app: rendering, interaction, keybindings, command surfaces

### 6.3 First `orv-ide` APIs

The initial API set should include:

- `cursor_context(file, offset) -> Context`
- `complete_at(file, offset, trigger) -> Suggestions`
- `step_value(file, offset, direction) -> Edit`
- `hover_at(file, offset) -> Info`
- `outline(file) -> Tree`

`trigger` should at minimum support:

- `At`
- `Percent`
- `Manual`

---

## 7. Roadmap Impact

This changes the implementation priorities in two ways:

1. parser/analyzer work must preserve editor-grade structural information
2. the built-in editor is a planned product surface, not an afterthought

That means:

- no language change should ignore cursor-context implications
- no parser shortcut should make partial-code editing fragile
- domain/property metadata should be treated as first-class compiler output

---

## 8. Non-goals for the First Editor Pass

The first editor pass does **not** need:

- full general LSP parity
- every possible refactor
- collaborative editing
- AI-native code generation UX
- visual canvas editing

The first pass **does** need:

- fast `@` node completion
- fast `%` property completion
- reliable cursor-context detection
- semantic steppers
- stable diagnostics under incomplete code

---

## 9. Immediate Next Steps

1. Keep syntax and analyzer changes incremental-friendly.
2. Define machine-readable node/property/domain metadata.
3. Add cursor-context queries over source spans.
4. Add `@` and `%` completion APIs before building the actual editor shell.
5. Design the built-in editor around the compiler's metadata rather than duplicating logic in UI code.
