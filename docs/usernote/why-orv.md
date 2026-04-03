# Why orv

[← Back to User Docs](./README.md)

---

## Problem

Modern software stacks are assembled from domain-specific systems that were optimized in isolation:

- databases optimize for storage and querying
- transport layers optimize for general interoperability
- UI frameworks optimize for rendering patterns inside their own runtime
- server frameworks optimize for request handling inside their own abstraction model

That separation brought portability, but it also means developers frequently pay for compatibility and generic tooling even when the project has a much narrower and more predictable shape.

## Observation

Most projects do not need maximum generality. A landing page, dashboard, internal tool, or game-adjacent web app each has a different set of constraints, yet many stacks still rely on the same broad protocols, bundling assumptions, and framework boundaries.

Even when teams adopt domain-specific tools, they still have to research and compose them manually. The project goal remains external to the stack.

## Core Idea

orv starts from a different premise:

> what if the framework and compiler were optimized for the project itself, not only for the domain?

If the compiler can see the structure of the whole project, it can make decisions across boundaries:

- UI and server can be optimized together
- data flow can inform render strategy
- route usage can inform transport shape
- actual project intent can drive bundling and dead code elimination

The result is a build artifact shaped around the application that is being made, rather than around a generic interoperability baseline.

## Trade-off

This approach intentionally gives up some generic compatibility in exchange for tighter integration and stronger optimization. That trade can be worthwhile when the ecosystem produces enough productivity and performance value to justify it.

orv is designed around that trade-off.

## Language Direction

The language is declarative and line-oriented on purpose:

- each line should carry a single clear intent
- domain relationships should remain easy to read
- syntax should stay understandable to both humans and AI systems performing structured analysis

That is why the language reference emphasizes explicit structure, lexical scope, and domain-aware validation.
