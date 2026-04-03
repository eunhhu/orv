# orv Implementation Roadmap

> Date: 2026-04-03
> Status: Draft
> Audience: compiler/runtime implementation planning

---

## 1. Purpose

This document turns the current orv language docs and optimization spec into an execution-grade implementation roadmap.

It is intended to answer five practical questions:

1. What do we build first?
2. What is explicitly in the first usable MVP?
3. Which crates and files should exist after each phase?
4. What must work for users at each milestone?
5. Which edge cases and failure modes must be handled before moving on?

This roadmap now also assumes that compiler internals should be built so a future orv-native built-in editor can consume them directly.

---

## 2. Current State Snapshot

The current repository only contains the shell of the project:

- [Cargo.toml](/Users/sunwoo/work/miol/Cargo.toml): workspace definition
- [crates/orv-cli/src/main.rs](/Users/sunwoo/work/miol/crates/orv-cli/src/main.rs): CLI with only `version`
- [crates/orv-core/src/lib.rs](/Users/sunwoo/work/miol/crates/orv-core/src/lib.rs): version export and macro re-export
- [crates/orv-macros/src/lib.rs](/Users/sunwoo/work/miol/crates/orv-macros/src/lib.rs): placeholder proc macro
- [docs/usernote/syntax/index.md](/Users/sunwoo/work/miol/docs/usernote/syntax/index.md): language surface
- [docs/specs/2026-04-03-project-optimization-design.md](/Users/sunwoo/work/miol/docs/specs/2026-04-03-project-optimization-design.md): optimization direction

That means the roadmap must cover everything from lexical analysis to a usable runtime.

---

## 3. Delivery Philosophy

### 3.1 Order of Work

- Correctness before optimization
- Diagnostics before codegen
- Reference execution before optimized backend
- Stable IR before transport/runtime specialization
- Conservative semantics before adaptive behavior
- Editor-grade syntax data before editor shell work

### 3.2 Roadmap Rule

Every feature graduates through the same ladder:

1. syntax accepted
2. semantic model defined
3. diagnostics added
4. reference behavior implemented
5. fixture tests added
6. optimization added later

### 3.3 Implementation Rule

No phase is considered done unless it includes:

- source fixtures
- expected diagnostics or output snapshots
- CLI surface for inspection or execution
- a clear rollback boundary if the next phase slips

---

## 4. MVP Freeze

### 4.1 MVP Includes

- multi-file module loading
- lexer, parser, AST
- diagnostics with file/line/column spans
- name resolution and lexical scope
- type inference for documented core types
- domain validation for `@html`, `@server`, `@design`
- typed object literals and pattern matching
- reference runtime for:
  - html tree evaluation
  - signals
  - routes
  - request accessors
  - responses
  - `@serve`
  - `@env`
- CLI commands:
  - `orv version`
  - `orv check`
  - `orv dump ast`
  - `orv dump hir`
  - `orv run`
  - `orv build`
- JSON-based fullstack RPC v1 for `.fetch()`
- static html build and dev server build targets

### 4.2 MVP Excludes

- binary RPC transport
- adaptive runtime
- aggressive dead code elimination
- production-grade chunk splitting
- WASM backend
- native optimized backend
- general external-IDE parity
- formatter
- incremental compilation cache

### 4.3 MVP Success Definition

A small multi-file orv app from the docs must be able to:

1. parse
2. type-check
3. reject invalid domain usage
4. run through the reference runtime
5. serve basic routes
6. render a basic page
7. call a route via `.fetch()`

---

## 5. Target Crate Architecture

The current 3-crate workspace is too small for a real compiler/runtime pipeline. The recommended structure is:

| Crate | Purpose | Initial Phase |
|------|---------|---------------|
| `orv-cli` | command-line orchestration | existing |
| `orv-core` | public facade crate | existing |
| `orv-macros` | proc-macro facade, optional later integration | existing |
| `orv-span` | file ids, spans, source text indexing | Phase 0 |
| `orv-diagnostics` | errors, warnings, reports, printing | Phase 0 |
| `orv-syntax` | lexer, parser, AST | Phase 1-2 |
| `orv-hir` | lowered semantic IR | Phase 3-5 |
| `orv-analyzer` | scope, types, domain validation | Phase 3-5 |
| `orv-project` | module graph and ProjectGraph | Phase 5-6 |
| `orv-runtime` | reference runtime and interpreter | Phase 7-9 |
| `orv-compiler` | build pipeline, emit, backends | Phase 8-12 |
| `orv-ide` | editor-facing queries, completion, hover, steppers | post-MVP foundation |

### 5.1 Recommended File Skeleton

The implementation should converge toward this minimum file layout:

```text
crates/
  orv-span/src/lib.rs
  orv-diagnostics/src/{lib.rs,render.rs}
  orv-syntax/src/{lib.rs,token.rs,lexer.rs,parser.rs,ast.rs}
  orv-hir/src/{lib.rs,hir.rs,ids.rs}
  orv-analyzer/src/{lib.rs,scope.rs,types.rs,domains.rs,patterns.rs}
  orv-project/src/{lib.rs,module_graph.rs,project_graph.rs}
  orv-runtime/src/{lib.rs,value.rs,signals.rs,html.rs,server.rs,rpc.rs,env.rs}
  orv-compiler/src/{lib.rs,pipeline.rs,build.rs,emit.rs}
  orv-ide/src/{lib.rs,context.rs,complete.rs,hover.rs,step.rs}
```

---

## 6. Milestone Plan

### Phase 0. Foundations and Fixtures

**Goal**

Create the support systems required to build the language without repainting the architecture later.

**Deliverables**

- `orv-span`
- `orv-diagnostics`
- fixture harness
- snapshot test convention
- source-file loader abstraction

**Tickets**

1. Add `orv-span` with `FileId`, `Span`, `Spanned<T>`, line index lookup.
2. Add `orv-diagnostics` with `Severity`, `Diagnostic`, labels, notes, help text.
3. Add fixture directories:
   - `fixtures/lexer`
   - `fixtures/parser`
   - `fixtures/analyzer`
   - `fixtures/runtime`
4. Add test helpers for golden output comparison.
5. Add `orv-cli` hidden debug commands for fixture-driven development.

**Primary Scenarios**

- A malformed source file reports exact location.
- Multiple files in a project can be loaded and attributed by `FileId`.
- Diagnostics can point to primary and secondary spans.

**Edge Cases**

- file with only comments
- empty file
- file without trailing newline
- very long line
- unicode in string literal
- cross-file import error with note pointing to both files

**Exit Criteria**

- diagnostics render deterministically in tests
- span math is validated by unit tests
- fixture runner exists before lexer implementation starts

### Phase 1. Lexer

**Goal**

Convert source text into a stable token stream that preserves enough structure for line-oriented parsing.

**Deliverables**

- token enum
- lexer with span emission
- line break and indentation handling
- comment stripping and trivia policy

**Tickets**

1. Define token types for identifiers, keywords, punctuation, literals, operators, interpolation markers.
2. Decide newline handling strategy:
   - emit `Newline`
   - preserve indentation count
   - allow continuation lines for inline `%` properties
3. Implement string literal support with interpolation segments.
4. Implement path/method/attribute tokenization for route and hint syntax.
5. Emit diagnostics for unclosed constructs and invalid characters.

**Primary Scenarios**

- `@button "Click" %onClick={count += 1}`
- continued property lines after a node declaration
- route paths such as `/users/:id`
- `@hint protocol=json`
- `"Hello {name}"`

**Edge Cases**

- unterminated string
- unmatched `{` inside interpolation
- `%` at top level with no attachable parent
- tabs mixed with spaces in continuation lines
- stray `@` token
- regex-like token in `@token \d+`

**Exit Criteria**

- all parser fixtures start from stable token snapshots
- lexer produces no panic on fuzzed small inputs

### Phase 2. Parser and AST

**Goal**

Parse the language surface documented in the syntax reference into a recoverable AST.

**Deliverables**

- AST definitions
- parser with recovery
- AST dump command

**Tickets**

1. Define module, item, statement, expression, type, pattern, node, property AST.
2. Parse top-level items:
   - `import`
   - `type`
   - `struct`
   - `enum`
   - `function`
   - `define`
   - top-level `let`/`const`
3. Parse node syntax:
   - positional tokens
   - inline properties
   - continuation properties
   - body blocks
4. Parse object literal vs code block using first-statement rule.
5. Parse `when`, `try/catch`, `async`, tuple await, generics, route hints.
6. Add recovery around braces, parentheses, and node blocks.

**Primary Scenarios**

- every `orv` fenced example in the docs parses
- syntax dump is stable enough for snapshot testing
- partially broken files still produce a usable AST for editor tooling later

**Edge Cases**

- multiline object literals with and without commas
- `return @response 200 %header={...} { ... }`
- `pub define X() -> @html @hint render=ssr { ... }`
- nested `define`
- `Point { x, y }` in pattern position
- ambiguous `{}` after expression

**Exit Criteria**

- `orv dump ast` works on all happy-path fixtures
- syntax errors are recoverable enough to emit at least one downstream error

### Phase 3. Name Resolution and Module Graph

**Goal**

Resolve identifiers and imports under true lexical scope rules.

**Deliverables**

- module graph loader
- import resolver
- symbol tables
- lexical scope map

**Tickets**

1. Implement module path resolution from file layout.
2. Resolve exports and private declarations.
3. Resolve local bindings, patterns, function params, `define` params.
4. Resolve route references and same-scope `.fetch()` targets.
5. Detect duplicate symbol and unresolved symbol errors.
6. Detect direct import cycles.

**Primary Scenarios**

- imported `pub define`
- local symbol shadowing inside `if`/`for`
- `@children` and inner `define` names
- route reference used by sibling code in the same lexical scope

**Edge Cases**

- import alias collisions
- wildcard import collisions
- cyclic re-export
- use-before-declaration policy
- pattern bindings shadowing outer names

**Exit Criteria**

- every name in analyzer fixtures is either bound or clearly diagnosed
- route scope behavior matches the docs, not ad hoc runtime magic

### Phase 4. Type System

**Goal**

Implement the core type rules needed to make diagnostics meaningful and runtime lowering safe.

**Deliverables**

- type representation
- inference engine
- constraint solver
- type diagnostics

**Tickets**

1. Add primitive, nullable, tuple, function, struct, enum, generic, and `void` types.
2. Implement local inference for literals and expressions.
3. Implement typed object literal validation.
4. Implement pattern typing for `when`.
5. Implement callable typing for functions, closures, and `.fetch()`.
6. Implement `@env` contextual typing.

**Primary Scenarios**

- `let port: i32 = @env PORT`
- `let sig remaining = todos.len()`
- `when result { Status.Ok(code) -> ... }`
- `let user: User = { name: "x", age: 1 }`

**Edge Cases**

- empty `[]` without context
- empty `{}` used as object literal with no expected type
- nullable narrowing after `if x != void`
- mixed-type branches in `when`
- associated enum payload mismatch
- object field missing or extra

**Exit Criteria**

- docs’ fundamentals examples type-check or fail with deliberate diagnostics
- there is no implicit `any`

### Phase 5. Domain Validation

**Goal**

Enforce the language’s core promise that domain misuse is a compile-time error.

**Deliverables**

- domain context model
- node/property validity matrix
- domain-aware diagnostics

**Tickets**

1. Model domain contexts:
   - neutral
   - html
   - server
   - design
2. Define legal nodes and properties for each context.
3. Validate transitions and cross-domain references.
4. Validate event handlers and callback bodies under the correct domain assumptions.
5. Validate `@serve` rules for html/file/static targets.

**Primary Scenarios**

- `@server` containing only server nodes
- `@html` containing only html-safe nodes
- `@serve page` where `page` is html
- design token usage from UI

**Edge Cases**

- `define` returning domain-specific nodes
- domain mismatch hidden behind imported definitions
- `@children` inserted into a different domain than declared
- `@route` inside illegal callback scope

**Exit Criteria**

- invalid-domain fixtures are rejected deterministically
- false positives are below acceptable threshold for docs examples

### Phase 6. HIR and ProjectGraph

**Goal**

Lower analyzed AST into a semantic model the compiler and runtime can actually consume.

**Deliverables**

- HIR node ids
- resolved expression forms
- route map
- signal dependency map
- fetch graph
- project-level summary graph

**Tickets**

1. Define HIR ids and cross-reference system.
2. Lower parsed items into canonical HIR forms.
3. Record route metadata and nesting.
4. Record signal dependencies.
5. Record `.fetch()` target edges.
6. Build initial `ProjectGraph` output command.

**Primary Scenarios**

- nested route group path flattening
- signal dependency extraction from interpolated strings
- html page and server route linked in one project graph

**Edge Cases**

- unreachable route references
- recursive `define` graphs
- cyclic signal dependency
- conflicting route methods/paths

**Exit Criteria**

- `orv dump hir` and `orv build --emit project-graph` are useful for debugging

### Phase 7. Reference Runtime

**Goal**

Make the language executable before building optimized backends.

**Deliverables**

- runtime values
- expression evaluator
- signal store
- html node tree renderer
- server route dispatcher
- request/response model

**Tickets**

1. Define runtime `Value`.
2. Implement expression and statement evaluator.
3. Implement `sig` with dependency tracking.
4. Implement html tree evaluation.
5. Implement `@server`, `@route`, `@before`, `@after`, `@response`, `@serve`.
6. Implement `@env` at runtime with analyzer-validated coercion.

**Primary Scenarios**

- render a static page
- increment a signal in a button event handler
- serve GET and POST routes
- return JSON response
- read query/path/body/header accessors

**Edge Cases**

- missing env var
- env coercion failure
- recursive signal update loop
- middleware early return
- `@after` after error response
- route not found

**Exit Criteria**

- `orv run` can execute small apps through the reference runtime

### Phase 8. CLI Expansion

**Goal**

Expose compiler stages through stable commands.

**Deliverables**

- subcommands in `orv-cli`
- machine-readable and human-readable diagnostics

**Tickets**

1. Add `check`.
2. Add `dump ast`.
3. Add `dump hir`.
4. Add `run`.
5. Add `build`.
6. Add `--format json` for diagnostics.

**Primary Scenarios**

- `orv check app.orv`
- `orv dump ast app.orv`
- `orv run app.orv`
- `orv build app.orv --emit project-graph`

**Edge Cases**

- relative import roots
- multiple entry files
- missing file
- invalid UTF-8 file contents
- non-zero exit code consistency

**Exit Criteria**

- developer can inspect every major compiler phase without touching library internals

### Phase 9. Basic Build Backend

**Goal**

Produce usable application artifacts without yet doing aggressive optimization.

**Deliverables**

- build manifest
- output directory layout
- static html emitter
- dev server artifact generator

**Tickets**

1. Define build options and output directory conventions.
2. Emit static html for pure static pages.
3. Emit runtime-backed server app wrapper for `@server`.
4. Emit html shell and minimal client runtime bundle manifest.
5. Add asset copying for `@serve ./public` and similar paths.

**Primary Scenarios**

- static page app builds to `dist/index.html`
- server route app builds to executable/dev-runner artifact
- mixed project emits build manifest and asset tree

**Edge Cases**

- duplicate asset path
- route path collision
- missing static asset path
- build output overwrite

**Exit Criteria**

- `orv build` can produce a visible artifact for examples

### Phase 10. Fullstack RPC v1

**Goal**

Implement typed `.fetch()` over a safe default JSON transport.

**Deliverables**

- route reference lowering
- request argument mapping
- response shape typing
- runtime glue between UI and server

**Tickets**

1. Lower `let x = @route ...` to a first-class route reference.
2. Map `.fetch(body=..., query=..., param=..., header=...)` arguments.
3. Enforce compile-time validation of param names and transport legality.
4. Lower `.fetch()` to runtime transport calls.
5. Add typed result mapping from `@response`.

**Primary Scenarios**

- `getUsers.fetch()`
- `getUser.fetch(param={ id: "42" })`
- `createUser.fetch(body={ name: "Kim" })`

**Edge Cases**

- missing path param
- extra path param
- `body` on GET policy violation
- `.fetch()` on non-route symbol
- unresolved route reference across scope boundary

**Exit Criteria**

- docs’ fullstack route examples work over JSON transport

### Phase 11. UI Runtime v1

**Goal**

Support interactive docs-scale UI behavior.

**Deliverables**

- html node rendering
- event binding
- signal-driven rerender or patching
- `@design` token resolution

**Tickets**

1. Render html nodes and text interpolation.
2. Bind event handlers.
3. Apply signal updates to affected nodes.
4. Implement list and conditional rendering.
5. Resolve design tokens to runtime style values.
6. Implement lifecycle hooks.

**Primary Scenarios**

- counter page
- card with children
- todo list add/delete
- theme token usage

**Edge Cases**

- event handler mutates collection during iteration
- signal updates inside derived expressions
- missing token reference
- lifecycle cleanup ordering

**Exit Criteria**

- docs’ UI examples run end-to-end in the reference runtime

### Phase 12. Optimization v1

**Goal**

Apply only the optimizations that are safe with the current semantic model.

**Deliverables**

- conservative dead code elimination
- static header/content-type inference
- route graph validation
- initial render classification

**Tickets**

1. Remove unreachable declarations from build graph.
2. Infer simple response metadata.
3. Classify obvious static pages as SSG.
4. Emit warnings or reports for obvious parallel fetch opportunities.
5. Respect `@hint keep`, `@hint render`, and `@hint protocol` at planning level even if backend behavior is partial.

**Primary Scenarios**

- unused helper removed
- static page emitted without runtime dependency
- route with explicit `@hint render=ssr` preserved in metadata

**Edge Cases**

- side-effect import accidentally removed
- hint conflicts
- false-positive static classification

**Exit Criteria**

- optimization never changes program meaning relative to the reference runtime

### Phase 13. Advanced Runtime and Transport

**Goal**

Implement the differentiated features that make orv distinct beyond the MVP.

**Deliverables**

- binary RPC
- cache/prefetch metadata execution
- chunk planning
- adaptive runtime hooks

**Tickets**

1. Add response schema serialization for binary RPC.
2. Add route privacy/public transport split.
3. Add simple cache and prefetch runtime policy.
4. Add chunk metadata model.
5. Add metrics collection and adaptive runtime stubs.

**Primary Scenarios**

- internal route compiled to binary transport
- public route forced to JSON
- repeated fetch served from cache

**Edge Cases**

- schema drift
- stale cache
- transport mismatch between client and server build
- adaptive runtime acting on incomplete metrics

**Exit Criteria**

- advanced features remain optional and do not block the core compiler/runtime

---

## 7. Cross-Cutting Test Strategy

### 7.1 Test Layers

| Layer | Purpose | Location |
|------|---------|----------|
| Unit | algorithm correctness | crate-local `src/*` and `tests/*` |
| Fixture snapshot | language surface stability | workspace `fixtures/*` |
| Integration | multi-crate pipeline behavior | `crates/orv-cli/tests` or workspace `tests` |
| Runtime scenario | executable language behavior | `fixtures/runtime` |
| Build artifact | emitted files and manifests | `fixtures/build` |

### 7.2 Fixture Categories

- `ok/hello.orv`
- `ok/counter.orv`
- `ok/server-basic.orv`
- `ok/fullstack-rpc.orv`
- `ok/env-inference.orv`
- `ok/design-theme.orv`
- `err/domain-html-in-server.orv`
- `err/domain-route-in-ui.orv`
- `err/object-vs-block-ambiguous.orv`
- `err/fetch-missing-param.orv`
- `err/env-coercion-fail.orv`
- `err/import-cycle.orv`

### 7.3 Non-Negotiable Assertions

- lexer and parser must never panic on malformed source
- analyzer must never infer hidden dynamic types
- route reference scope must remain lexical
- typed object literal construction must never accept `Type { ... }` as value syntax
- runtime behavior must be reproducible without optimization enabled

---

## 8. Functional Scenario Matrix

### Scenario A. Static HTML Page

**Source**

- one file with `@html`, `@head`, `@body`

**Expected Behavior**

- parses cleanly
- validates as html domain
- builds to static html
- no server runtime required

**Failure Modes**

- invalid node in html context
- missing required interpolation symbol
- static classifier incorrectly retaining runtime dependency

### Scenario B. Signal Counter

**Source**

- `let sig count = 0`
- text interpolation
- button event increments count

**Expected Behavior**

- signal storage created
- button event closure mutates signal
- only affected text node updates

**Failure Modes**

- event closure captures wrong binding
- signal graph creates self-loop
- full rerender instead of targeted update

### Scenario C. Basic Server

**Source**

- `@server`
- `@listen`
- `@route GET /`
- `return @response 200 { ... }`

**Expected Behavior**

- route dispatch works
- JSON response emitted with inferred metadata

**Failure Modes**

- route collision
- invalid response payload shape
- request accessor used outside route scope

### Scenario D. Fullstack RPC

**Source**

- `let getUsers = @route GET /api/users { ... }`
- same-scope html calling `await getUsers.fetch()`

**Expected Behavior**

- analyzer resolves route symbol
- `.fetch()` arguments validated
- runtime calls route and returns typed value

**Failure Modes**

- route used outside lexical scope
- wrong `param` key
- `.fetch()` on symbol that is not a route reference

### Scenario E. `@env` Inference

**Source**

- `let port: i32 = @env PORT`
- `let secret = @env SECRET`

**Expected Behavior**

- `port` receives contextual coercion
- `secret` remains string

**Failure Modes**

- missing env var
- malformed integer string
- implicit coercion with no expected type

### Scenario F. `@hint` Round-Trip

**Source**

- route and page with `@hint`

**Expected Behavior**

- parser accepts hints
- analyzer validates allowed targets
- build metadata preserves hints even before advanced optimizer exists

**Failure Modes**

- hint attached to illegal target
- unsupported hint value silently ignored

---

## 9. Pre-Mortem

### Failure Scenario 1. Grammar Ambiguity Burns the Front-End

**Risk**

Object literals, code blocks, continuation lines, and node bodies conflict in subtle ways.

**Mitigation**

- freeze fixture corpus before parser work
- explicitly test ambiguous constructs
- avoid hidden parser heuristics that are not documented

### Failure Scenario 2. Runtime Semantics Drift from Future Backend

**Risk**

Reference runtime behaves differently from build output, making optimization unsafe.

**Mitigation**

- treat runtime as semantic oracle
- compare backend behavior to runtime fixtures
- do not optimize features before runtime behavior is stable

### Failure Scenario 3. Scope Rules Regress Under RPC Features

**Risk**

Route references accidentally become global or dynamic to “make examples work.”

**Mitigation**

- scope resolution tests for route references
- explicit analyzer rule that `.fetch()` uses lexical binding only
- reject undocumented scope lifting

---

## 10. First Two Weeks Execution Plan

### Week 1

**Day 1**

- create `orv-span`
- create `orv-diagnostics`
- add shared fixture harness

**Day 2**

- add source loader abstraction
- add diagnostic rendering snapshots
- create first 12 language fixtures

**Day 3**

- implement token enum
- implement punctuation, identifiers, keywords
- add lexer unit tests

**Day 4**

- implement strings, interpolation, comments, newlines
- implement lexer diagnostics

**Day 5**

- freeze lexer snapshot output
- add parser skeleton and AST shell types

### Week 2

**Day 6**

- parse top-level items
- parse node declarations and inline properties

**Day 7**

- parse continuation properties and block bodies
- parse object literal vs code block split

**Day 8**

- parse expressions, calls, closures, tuples, patterns

**Day 9**

- add parser recovery
- add `orv dump ast`

**Day 10**

- review all parser fixtures
- fix ambiguities before moving into name resolution

### Two-Week Exit Target

At the end of week 2, the project should have:

- stable span and diagnostics infrastructure
- a fixture-driven lexer
- a recoverable parser
- an AST dump CLI command

If that is not true, do not start type inference yet.

---

## 11. Decision Gates

Before starting the next phase, explicitly confirm:

### Gate A. Before Name Resolution

- AST is stable enough that fixture churn is low
- parser recovery is acceptable

### Gate B. Before Type System

- scope model is not being rewritten
- import resolution behavior is clear

### Gate C. Before Runtime

- type diagnostics are trustworthy
- domain validation is implemented

### Gate D. Before Build Backend

- reference runtime semantics are stable
- HIR and ProjectGraph are not moving underfoot

### Gate E. Before Advanced Optimization

- core app scenarios already run correctly without optimization

---

## 12. Immediate Next Action

The next implementation task should be:

1. add `orv-span`
2. add `orv-diagnostics`
3. add fixture infrastructure
4. add lexer token model

Do not start with proc-macro expansion, binary RPC, or optimizer work.
