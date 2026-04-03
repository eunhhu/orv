# Getting Started

[← Back to User Docs](./README.md)

---

This repository is still early-stage. The documentation is organized so that product direction, language reference, and implementation specs are separated instead of mixed together.

## What Exists Today

- The repository contains the core crates for `miol`, including `miol-cli`, `miol-core`, and `miol-macros`.
- The current CLI surface in code is minimal. The implemented command path is version output.
- The language reference is already modularized under [`syntax/`](./syntax/).

## Recommended First Steps

1. Read [Why miol](./why-miol.md) to understand the project goal.
2. Read [Language Reference](./syntax.md) for the current syntax surface.
3. Read [Design Specs](../specs/README.md) for optimization and compiler/runtime proposals.

## Current CLI Snapshot

```bash
cargo run -p miol-cli
cargo run -p miol-cli -- version
```

As more CLI commands are implemented, this page should grow into task-oriented setup and workflow guidance.
