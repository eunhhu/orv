# orv Language Specification

[← Back to User Docs](../README.md)

> **orv** — a universal, concise DSL for building fullstack applications.
> WASM-first web runtime, native binary compilation, fine-grained reactivity.

---

## Table of Contents

1. [Philosophy](#philosophy)
2. [Syntax Fundamentals](./fundamentals.md)
3. [Functions & Control Flow](./functions.md)
4. [Collections, Errors & Async](./collections.md)
5. [Modules & Imports](./modules.md)
6. [Node System & Reactivity](./nodes.md)
7. [UI & Design Domain](./ui.md)
8. [Server Domain](./server.md)
9. [Custom Nodes — `define`](./define.md)
10. [Compiler Hints](./hints.md)
11. [Best Practices & Examples](./best-practices.md)

---

## Philosophy

orv is built on six principles:

- **One syntax, every domain.** UI, server, design tokens, and general logic share a unified grammar. The `@node` / `%property` structure scales from a button to an HTTP server.
- **One reusable node abstraction: `define`.** There is no `class`, no `new`, no `this`, no inheritance. `define` is for reusable `@node` / domain structure, while `function` remains the callable abstraction for value-oriented logic.
- **Conciseness without magic.** Every abbreviation has a predictable expansion. `$0` is always the first callback parameter. `sig` is always a reactive signal. There are no hidden transforms.
- **Compile-time safety, runtime speed.** Types are inferred like Rust, checked at compile time, and compiled to WASM (web) or native binary. Domain contexts are validated at compile time — you cannot put `@div` inside `@server`.
- **Project-specific optimization.** The compiler analyzes the entire project — UI, server, design, and their relationships — to produce a bundle optimized for the project's actual purpose. Rather than relying on general-purpose protocols and formats, the output is tailored to the specific domains and communication patterns the project uses.
- **Declarative, line-oriented syntax.** Each line carries a single clear intent. The grammar is designed so that both humans and AI can read code line by line with unambiguous meaning. There are no semicolons, no multi-statement lines, and inline constructs stay on one line only when they remain readable.
- **Editor-native structure.** The language is intentionally shaped for an orv-native built-in editor. `@` should open legal node choices for the current domain, `%` should open legal properties for the current node, and semantic values should be step-able without falling back to generic text editing.
