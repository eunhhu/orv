# Phase 0: Foundations and Fixtures — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Create the span, diagnostics, source-loading, and fixture infrastructure required before lexer work begins.

**Architecture:** Two new library crates (`orv-span`, `orv-diagnostics`) provide the error-reporting foundation. `orv-span` owns file identity and byte-range tracking. `orv-diagnostics` owns structured error types and terminal rendering. A fixture harness in the workspace root enables golden-output snapshot testing for all future compiler phases. `orv-core` becomes the pipeline orchestrator that re-exports span/diagnostics and provides the source-file loader.

**Tech Stack:** Rust (edition 2024, nightly toolchain), workspace crates, `pretty_assertions` for test diffing, `codespan-reporting` for diagnostic rendering.

---

## File Structure

```text
crates/
  orv-span/
    Cargo.toml
    src/lib.rs              — FileId, Span, Spanned<T>, LineIndex, SourceMap
  orv-diagnostics/
    Cargo.toml
    src/lib.rs              — Severity, Label, Diagnostic, re-exports
    src/render.rs           — terminal rendering via codespan-reporting
  orv-core/
    Cargo.toml              — (modify) add orv-span, orv-diagnostics deps
    src/lib.rs              — (modify) re-export span/diagnostics, add source loader
    src/source.rs           — SourceFile, SourceLoader (file system abstraction)
fixtures/
  lexer/.gitkeep
  parser/.gitkeep
  analyzer/.gitkeep
  runtime/.gitkeep
  ok/hello.orv
  ok/counter.orv
  ok/server-basic.orv
  err/domain-html-in-server.orv
  err/domain-route-in-ui.orv
  err/empty.orv
tests/
  fixture_harness.rs        — golden-output snapshot test helper (integration test)
```

---

### Task 1: Add `orv-span` crate with `FileId` and `Span`

**Files:**
- Create: `crates/orv-span/Cargo.toml`
- Create: `crates/orv-span/src/lib.rs`
- Modify: `Cargo.toml` (workspace members + dependency)

- [ ] **Step 1: Create `orv-span/Cargo.toml`**

```toml
[package]
name = "orv-span"
description = "File identity and byte-span tracking for the orv compiler"
version.workspace = true
authors.workspace = true
edition.workspace = true
license.workspace = true
repository.workspace = true
rust-version.workspace = true

[dependencies]

[dev-dependencies]
pretty_assertions = { workspace = true }

[lints]
workspace = true
```

- [ ] **Step 2: Add `orv-span` to workspace**

In root `Cargo.toml`, add `orv-span = { path = "crates/orv-span" }` to `[workspace.dependencies]`.

- [ ] **Step 3: Write failing tests for `FileId`, `Span`, `Spanned<T>`**

In `crates/orv-span/src/lib.rs`:

```rust
/// Identifies a source file within the compilation session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FileId(u32);

impl FileId {
    pub fn new(raw: u32) -> Self {
        Self(raw)
    }

    pub fn raw(self) -> u32 {
        self.0
    }
}

/// A byte-offset range within a single source file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub file: FileId,
    pub start: u32,
    pub end: u32,
}

impl Span {
    pub fn new(file: FileId, start: u32, end: u32) -> Self {
        Self { file, start, end }
    }

    /// Length in bytes.
    pub fn len(&self) -> u32 {
        self.end - self.start
    }

    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }

    /// Merge two spans into one that covers both. Panics if files differ.
    pub fn merge(self, other: Span) -> Span {
        assert_eq!(self.file, other.file, "cannot merge spans from different files");
        Span {
            file: self.file,
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }
}

/// Wraps a value with its source location.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Spanned<T> {
    pub node: T,
    pub span: Span,
}

impl<T> Spanned<T> {
    pub fn new(node: T, span: Span) -> Self {
        Self { node, span }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_id_roundtrip() {
        let id = FileId::new(7);
        assert_eq!(id.raw(), 7);
    }

    #[test]
    fn span_len_and_empty() {
        let f = FileId::new(0);
        let s = Span::new(f, 10, 20);
        assert_eq!(s.len(), 10);
        assert!(!s.is_empty());

        let empty = Span::new(f, 5, 5);
        assert_eq!(empty.len(), 0);
        assert!(empty.is_empty());
    }

    #[test]
    fn span_merge() {
        let f = FileId::new(0);
        let a = Span::new(f, 5, 10);
        let b = Span::new(f, 20, 30);
        let merged = a.merge(b);
        assert_eq!(merged.start, 5);
        assert_eq!(merged.end, 30);
    }

    #[test]
    #[should_panic(expected = "cannot merge spans from different files")]
    fn span_merge_different_files_panics() {
        let a = Span::new(FileId::new(0), 0, 5);
        let b = Span::new(FileId::new(1), 0, 5);
        a.merge(b);
    }

    #[test]
    fn spanned_wraps_value() {
        let f = FileId::new(0);
        let s = Spanned::new("hello", Span::new(f, 0, 5));
        assert_eq!(s.node, "hello");
        assert_eq!(s.span.start, 0);
    }
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p orv-span`
Expected: all 5 tests pass.

- [ ] **Step 5: Commit**

```bash
git add crates/orv-span/ Cargo.toml Cargo.lock
git commit -m "feat(span): add orv-span crate with FileId, Span, Spanned<T>"
```

---

### Task 2: Add `LineIndex` to `orv-span`

**Files:**
- Modify: `crates/orv-span/src/lib.rs`

- [ ] **Step 1: Write failing tests for `LineIndex`**

Append to `crates/orv-span/src/lib.rs`:

```rust
/// Pre-computed line-start offsets for fast line/column lookup.
#[derive(Debug, Clone)]
pub struct LineIndex {
    /// Byte offsets where each line begins. `line_starts[0]` is always 0.
    line_starts: Vec<u32>,
}

impl LineIndex {
    /// Build a line index from source text.
    pub fn new(source: &str) -> Self {
        let mut line_starts = vec![0u32];
        for (i, b) in source.bytes().enumerate() {
            if b == b'\n' {
                line_starts.push(u32::try_from(i + 1).expect("source file too large"));
            }
        }
        Self { line_starts }
    }

    /// Returns (0-based line, 0-based column) for a byte offset.
    /// Returns `None` if offset is out of range.
    pub fn line_col(&self, offset: u32) -> Option<(u32, u32)> {
        let line = match self.line_starts.binary_search(&offset) {
            Ok(exact) => exact,
            Err(next) => next - 1,
        };
        let col = offset - self.line_starts[line];
        Some((u32::try_from(line).ok()?, col))
    }

    /// Total number of lines.
    pub fn line_count(&self) -> usize {
        self.line_starts.len()
    }
}
```

Add tests in the `tests` module:

```rust
    #[test]
    fn line_index_single_line() {
        let idx = LineIndex::new("hello");
        assert_eq!(idx.line_count(), 1);
        assert_eq!(idx.line_col(0), Some((0, 0)));
        assert_eq!(idx.line_col(4), Some((0, 4)));
    }

    #[test]
    fn line_index_multi_line() {
        let idx = LineIndex::new("ab\ncd\nef");
        assert_eq!(idx.line_count(), 3);
        assert_eq!(idx.line_col(0), Some((0, 0)));  // 'a'
        assert_eq!(idx.line_col(2), Some((0, 2)));  // '\n'
        assert_eq!(idx.line_col(3), Some((1, 0)));  // 'c'
        assert_eq!(idx.line_col(6), Some((2, 0)));  // 'e'
    }

    #[test]
    fn line_index_empty_source() {
        let idx = LineIndex::new("");
        assert_eq!(idx.line_count(), 1);
        assert_eq!(idx.line_col(0), Some((0, 0)));
    }

    #[test]
    fn line_index_trailing_newline() {
        let idx = LineIndex::new("abc\n");
        assert_eq!(idx.line_count(), 2);
        assert_eq!(idx.line_col(3), Some((0, 3)));  // '\n'
        assert_eq!(idx.line_col(4), Some((1, 0)));  // past newline
    }

    #[test]
    fn line_index_unicode() {
        // "안녕" is 6 bytes in UTF-8, then newline, then "hi"
        let idx = LineIndex::new("안녕\nhi");
        assert_eq!(idx.line_count(), 2);
        assert_eq!(idx.line_col(0), Some((0, 0)));
        assert_eq!(idx.line_col(7), Some((1, 0)));  // 'h'
    }
```

- [ ] **Step 2: Run tests**

Run: `cargo test -p orv-span`
Expected: all 10 tests pass.

- [ ] **Step 3: Commit**

```bash
git add crates/orv-span/src/lib.rs
git commit -m "feat(span): add LineIndex for line/column lookup"
```

---

### Task 3: Add `SourceMap` to `orv-span`

**Files:**
- Modify: `crates/orv-span/src/lib.rs`

- [ ] **Step 1: Write `SourceMap` with tests**

Append to `crates/orv-span/src/lib.rs`:

```rust
/// Stores all source files loaded during a compilation session.
#[derive(Debug, Default)]
pub struct SourceMap {
    files: Vec<SourceEntry>,
}

#[derive(Debug)]
struct SourceEntry {
    name: String,
    source: String,
    line_index: LineIndex,
}

impl SourceMap {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a source file. Returns the assigned `FileId`.
    pub fn add(&mut self, name: impl Into<String>, source: impl Into<String>) -> FileId {
        let source = source.into();
        let line_index = LineIndex::new(&source);
        let id = FileId::new(u32::try_from(self.files.len()).expect("too many source files"));
        self.files.push(SourceEntry {
            name: name.into(),
            source,
            line_index,
        });
        id
    }

    pub fn name(&self, id: FileId) -> &str {
        &self.files[id.raw() as usize].name
    }

    pub fn source(&self, id: FileId) -> &str {
        &self.files[id.raw() as usize].source
    }

    pub fn line_index(&self, id: FileId) -> &LineIndex {
        &self.files[id.raw() as usize].line_index
    }

    /// Resolve a `Span` to (filename, line, column).
    pub fn resolve(&self, span: Span) -> (&str, u32, u32) {
        let name = self.name(span.file);
        let (line, col) = self
            .line_index(span.file)
            .line_col(span.start)
            .expect("span offset out of range");
        (name, line, col)
    }

    pub fn file_count(&self) -> usize {
        self.files.len()
    }
}
```

Add tests:

```rust
    #[test]
    fn source_map_add_and_resolve() {
        let mut map = SourceMap::new();
        let id = map.add("main.orv", "let x = 1\nlet y = 2");
        assert_eq!(map.name(id), "main.orv");
        assert_eq!(map.source(id), "let x = 1\nlet y = 2");

        let span = Span::new(id, 11, 20); // "let y = 2"
        let (name, line, col) = map.resolve(span);
        assert_eq!(name, "main.orv");
        assert_eq!(line, 1);
        assert_eq!(col, 0);
    }

    #[test]
    fn source_map_multiple_files() {
        let mut map = SourceMap::new();
        let a = map.add("a.orv", "aaa");
        let b = map.add("b.orv", "bbb");
        assert_eq!(map.file_count(), 2);
        assert_eq!(map.name(a), "a.orv");
        assert_eq!(map.name(b), "b.orv");
    }
```

- [ ] **Step 2: Run tests**

Run: `cargo test -p orv-span`
Expected: all 12 tests pass.

- [ ] **Step 3: Commit**

```bash
git add crates/orv-span/src/lib.rs
git commit -m "feat(span): add SourceMap for multi-file source tracking"
```

---

### Task 4: Add `orv-diagnostics` crate

**Files:**
- Create: `crates/orv-diagnostics/Cargo.toml`
- Create: `crates/orv-diagnostics/src/lib.rs`
- Create: `crates/orv-diagnostics/src/render.rs`
- Modify: `Cargo.toml` (workspace deps)

- [ ] **Step 1: Add `codespan-reporting` to workspace deps**

In root `Cargo.toml` `[workspace.dependencies]`:

```toml
codespan-reporting = "0.11"
```

And add `orv-diagnostics = { path = "crates/orv-diagnostics" }`.

- [ ] **Step 2: Create `orv-diagnostics/Cargo.toml`**

```toml
[package]
name = "orv-diagnostics"
description = "Structured diagnostics for the orv compiler"
version.workspace = true
authors.workspace = true
edition.workspace = true
license.workspace = true
repository.workspace = true
rust-version.workspace = true

[dependencies]
orv-span = { workspace = true }
codespan-reporting = { workspace = true }

[dev-dependencies]
pretty_assertions = { workspace = true }

[lints]
workspace = true
```

- [ ] **Step 3: Write `src/lib.rs` with `Severity`, `Label`, `Diagnostic`**

```rust
mod render;

pub use render::render_diagnostics;

use orv_span::Span;

/// How serious a diagnostic is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Error,
    Warning,
    Help,
}

/// A labeled source span inside a diagnostic.
#[derive(Debug, Clone)]
pub struct Label {
    pub span: Span,
    pub message: String,
    pub is_primary: bool,
}

impl Label {
    pub fn primary(span: Span, message: impl Into<String>) -> Self {
        Self {
            span,
            message: message.into(),
            is_primary: true,
        }
    }

    pub fn secondary(span: Span, message: impl Into<String>) -> Self {
        Self {
            span,
            message: message.into(),
            is_primary: false,
        }
    }
}

/// A single compiler diagnostic.
#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub severity: Severity,
    pub message: String,
    pub labels: Vec<Label>,
    pub notes: Vec<String>,
}

impl Diagnostic {
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Error,
            message: message.into(),
            labels: Vec::new(),
            notes: Vec::new(),
        }
    }

    pub fn warning(message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Warning,
            message: message.into(),
            labels: Vec::new(),
            notes: Vec::new(),
        }
    }

    pub fn with_label(mut self, label: Label) -> Self {
        self.labels.push(label);
        self
    }

    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }

    pub fn is_error(&self) -> bool {
        self.severity == Severity::Error
    }
}

/// Collection of diagnostics from a compilation phase.
#[derive(Debug, Default)]
pub struct DiagnosticBag {
    diagnostics: Vec<Diagnostic>,
}

impl DiagnosticBag {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, diag: Diagnostic) {
        self.diagnostics.push(diag);
    }

    pub fn has_errors(&self) -> bool {
        self.diagnostics.iter().any(Diagnostic::is_error)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Diagnostic> {
        self.diagnostics.iter()
    }

    pub fn len(&self) -> usize {
        self.diagnostics.len()
    }

    pub fn is_empty(&self) -> bool {
        self.diagnostics.is_empty()
    }

    pub fn into_vec(self) -> Vec<Diagnostic> {
        self.diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use orv_span::{FileId, Span};

    #[test]
    fn diagnostic_builder() {
        let span = Span::new(FileId::new(0), 0, 5);
        let diag = Diagnostic::error("unexpected token")
            .with_label(Label::primary(span, "here"))
            .with_note("expected identifier");

        assert!(diag.is_error());
        assert_eq!(diag.labels.len(), 1);
        assert_eq!(diag.notes.len(), 1);
        assert_eq!(diag.message, "unexpected token");
    }

    #[test]
    fn diagnostic_bag_tracks_errors() {
        let mut bag = DiagnosticBag::new();
        assert!(!bag.has_errors());

        bag.push(Diagnostic::warning("unused variable"));
        assert!(!bag.has_errors());

        bag.push(Diagnostic::error("type mismatch"));
        assert!(bag.has_errors());
        assert_eq!(bag.len(), 2);
    }

    #[test]
    fn diagnostic_with_multiple_labels() {
        let f = FileId::new(0);
        let diag = Diagnostic::error("type mismatch")
            .with_label(Label::primary(Span::new(f, 10, 15), "expected i32"))
            .with_label(Label::secondary(Span::new(f, 20, 25), "found string"));

        assert_eq!(diag.labels.len(), 2);
        assert!(diag.labels[0].is_primary);
        assert!(!diag.labels[1].is_primary);
    }
}
```

- [ ] **Step 4: Write `src/render.rs` — terminal rendering via `codespan-reporting`**

```rust
use codespan_reporting::diagnostic as cs;
use codespan_reporting::files::SimpleFiles;
use codespan_reporting::term;
use codespan_reporting::term::termcolor::{ColorChoice, StandardStream};

use orv_span::SourceMap;

use crate::{Diagnostic, Label, Severity};

/// Render diagnostics to stderr using codespan-reporting.
pub fn render_diagnostics(source_map: &SourceMap, diagnostics: &[Diagnostic]) {
    let writer = StandardStream::stderr(ColorChoice::Auto);
    let config = term::Config::default();

    let mut files = SimpleFiles::new();
    let mut file_ids = Vec::new();

    for i in 0..source_map.file_count() {
        let fid = orv_span::FileId::new(u32::try_from(i).unwrap());
        let id = files.add(source_map.name(fid), source_map.source(fid));
        file_ids.push(id);
    }

    for diag in diagnostics {
        let severity = match diag.severity {
            Severity::Error => cs::Severity::Error,
            Severity::Warning => cs::Severity::Warning,
            Severity::Help => cs::Severity::Help,
        };

        let labels: Vec<cs::Label<usize>> = diag
            .labels
            .iter()
            .map(|label| {
                let cs_file_id = file_ids[label.span.file.raw() as usize];
                if label.is_primary {
                    cs::Label::primary(cs_file_id, label.span.start as usize..label.span.end as usize)
                        .with_message(&label.message)
                } else {
                    cs::Label::secondary(cs_file_id, label.span.start as usize..label.span.end as usize)
                        .with_message(&label.message)
                }
            })
            .collect();

        let cs_diag = cs::Diagnostic::new(severity)
            .with_message(&diag.message)
            .with_labels(labels)
            .with_notes(diag.notes.clone());

        let _ = term::emit(&mut writer.lock(), &config, &files, &cs_diag);
    }
}

/// Render diagnostics to a string (for testing / snapshot comparison).
pub fn render_diagnostics_to_string(source_map: &SourceMap, diagnostics: &[Diagnostic]) -> String {
    use codespan_reporting::term::termcolor::Buffer;

    let config = term::Config::default();

    let mut files = SimpleFiles::new();
    let mut file_ids = Vec::new();

    for i in 0..source_map.file_count() {
        let fid = orv_span::FileId::new(u32::try_from(i).unwrap());
        let id = files.add(source_map.name(fid), source_map.source(fid));
        file_ids.push(id);
    }

    let mut buffer = Buffer::no_color();

    for diag in diagnostics {
        let severity = match diag.severity {
            Severity::Error => cs::Severity::Error,
            Severity::Warning => cs::Severity::Warning,
            Severity::Help => cs::Severity::Help,
        };

        let labels: Vec<cs::Label<usize>> = diag
            .labels
            .iter()
            .map(|label| {
                let cs_file_id = file_ids[label.span.file.raw() as usize];
                if label.is_primary {
                    cs::Label::primary(cs_file_id, label.span.start as usize..label.span.end as usize)
                        .with_message(&label.message)
                } else {
                    cs::Label::secondary(cs_file_id, label.span.start as usize..label.span.end as usize)
                        .with_message(&label.message)
                }
            })
            .collect();

        let cs_diag = cs::Diagnostic::new(severity)
            .with_message(&diag.message)
            .with_labels(labels)
            .with_notes(diag.notes.clone());

        let _ = term::emit(&mut buffer, &config, &files, &cs_diag);
    }

    String::from_utf8_lossy(buffer.as_slice()).into_owned()
}
```

- [ ] **Step 5: Run tests**

Run: `cargo test -p orv-diagnostics`
Expected: all 3 tests pass.

- [ ] **Step 6: Commit**

```bash
git add crates/orv-diagnostics/ Cargo.toml Cargo.lock
git commit -m "feat(diagnostics): add orv-diagnostics crate with Severity, Label, Diagnostic, rendering"
```

---

### Task 5: Add diagnostic rendering snapshot test

**Files:**
- Modify: `crates/orv-diagnostics/src/lib.rs` (add integration-style test)

- [ ] **Step 1: Add snapshot test for rendered output**

Append to the `tests` module in `crates/orv-diagnostics/src/lib.rs`:

```rust
    #[test]
    fn render_snapshot_single_error() {
        use crate::render::render_diagnostics_to_string;

        let mut source_map = orv_span::SourceMap::new();
        let fid = source_map.add("test.orv", "let x = @badnode\nlet y = 2");

        let diag = Diagnostic::error("unknown node `@badnode`")
            .with_label(Label::primary(
                Span::new(fid, 8, 16),
                "not a valid node",
            ))
            .with_note("valid nodes include @div, @text, @button");

        let output = render_diagnostics_to_string(&source_map, &[diag]);

        // Verify key parts are present (deterministic rendering)
        assert!(output.contains("error"), "should contain 'error'");
        assert!(output.contains("unknown node `@badnode`"), "should contain message");
        assert!(output.contains("not a valid node"), "should contain label");
        assert!(output.contains("test.orv"), "should contain filename");
        assert!(
            output.contains("valid nodes include @div, @text, @button"),
            "should contain note"
        );
    }

    #[test]
    fn render_snapshot_cross_file() {
        use crate::render::render_diagnostics_to_string;

        let mut source_map = orv_span::SourceMap::new();
        let a = source_map.add("main.orv", "import components.Missing");
        let b = source_map.add("components.orv", "pub define Button() -> @button \"ok\"");

        let diag = Diagnostic::error("unresolved import `Missing`")
            .with_label(Label::primary(Span::new(a, 18, 25), "not found in module"))
            .with_label(Label::secondary(
                Span::new(b, 0, 35),
                "module defined here",
            ))
            .with_note("available exports: Button");

        let output = render_diagnostics_to_string(&source_map, &[diag]);
        assert!(output.contains("main.orv"), "should reference main.orv");
        assert!(output.contains("components.orv"), "should reference components.orv");
        assert!(output.contains("unresolved import `Missing`"));
    }
```

- [ ] **Step 2: Run tests**

Run: `cargo test -p orv-diagnostics`
Expected: all 5 tests pass.

- [ ] **Step 3: Commit**

```bash
git add crates/orv-diagnostics/src/lib.rs
git commit -m "test(diagnostics): add rendering snapshot tests for single and cross-file errors"
```

---

### Task 6: Wire `orv-span` and `orv-diagnostics` into `orv-core`

**Files:**
- Modify: `crates/orv-core/Cargo.toml`
- Modify: `crates/orv-core/src/lib.rs`

- [ ] **Step 1: Add dependencies to `orv-core/Cargo.toml`**

Add under `[dependencies]`:

```toml
orv-span = { workspace = true }
orv-diagnostics = { workspace = true }
```

- [ ] **Step 2: Re-export from `orv-core/src/lib.rs`**

Replace `crates/orv-core/src/lib.rs` with:

```rust
pub use orv_macros::orv;

// Re-export foundation crates
pub use orv_diagnostics as diagnostics;
pub use orv_span as span;

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert_eq!(version(), "0.1.0");
    }

    #[test]
    fn test_orv_macro() {
        let _result = orv! {
            hello world
        };
    }

    #[test]
    fn test_span_reexport() {
        let id = span::FileId::new(0);
        let s = span::Span::new(id, 0, 5);
        assert_eq!(s.len(), 5);
    }

    #[test]
    fn test_diagnostics_reexport() {
        let diag = diagnostics::Diagnostic::error("test");
        assert!(diag.is_error());
    }
}
```

- [ ] **Step 3: Run tests**

Run: `cargo test -p orv-core`
Expected: all 4 tests pass.

- [ ] **Step 4: Commit**

```bash
git add crates/orv-core/Cargo.toml crates/orv-core/src/lib.rs
git commit -m "feat(core): wire orv-span and orv-diagnostics into orv-core"
```

---

### Task 7: Add source-file loader to `orv-core`

**Files:**
- Create: `crates/orv-core/src/source.rs`
- Modify: `crates/orv-core/src/lib.rs`

- [ ] **Step 1: Write `source.rs` with `SourceLoader`**

Create `crates/orv-core/src/source.rs`:

```rust
use std::path::{Path, PathBuf};

use orv_diagnostics::{Diagnostic, DiagnosticBag};
use orv_span::{FileId, SourceMap};

/// Loads source files from the filesystem into a `SourceMap`.
pub struct SourceLoader {
    source_map: SourceMap,
    diagnostics: DiagnosticBag,
    root: PathBuf,
}

impl SourceLoader {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            source_map: SourceMap::new(),
            diagnostics: DiagnosticBag::new(),
            root: root.into(),
        }
    }

    /// Load a file relative to the project root.
    pub fn load(&mut self, relative_path: &str) -> Option<FileId> {
        let full_path = self.root.join(relative_path);
        self.load_absolute(&full_path, relative_path)
    }

    /// Load a file from an absolute path, using the given display name.
    pub fn load_absolute(&mut self, path: &Path, display_name: &str) -> Option<FileId> {
        match std::fs::read_to_string(path) {
            Ok(source) => {
                let id = self.source_map.add(display_name, source);
                Some(id)
            }
            Err(e) => {
                self.diagnostics.push(
                    Diagnostic::error(format!("could not read `{display_name}`: {e}")),
                );
                None
            }
        }
    }

    /// Load source from a string (for testing or REPL).
    pub fn load_string(&mut self, name: &str, source: &str) -> FileId {
        self.source_map.add(name, source)
    }

    pub fn source_map(&self) -> &SourceMap {
        &self.source_map
    }

    pub fn into_parts(self) -> (SourceMap, DiagnosticBag) {
        (self.source_map, self.diagnostics)
    }

    pub fn has_errors(&self) -> bool {
        self.diagnostics.has_errors()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn load_string_works() {
        let mut loader = SourceLoader::new(".");
        let id = loader.load_string("test.orv", "let x = 1");
        assert_eq!(loader.source_map().source(id), "let x = 1");
        assert_eq!(loader.source_map().name(id), "test.orv");
        assert!(!loader.has_errors());
    }

    #[test]
    fn load_missing_file_produces_diagnostic() {
        let mut loader = SourceLoader::new("/nonexistent");
        let result = loader.load("missing.orv");
        assert!(result.is_none());
        assert!(loader.has_errors());
    }

    #[test]
    fn load_real_file() {
        let dir = std::env::temp_dir().join("orv-test-loader");
        std::fs::create_dir_all(&dir).unwrap();
        let file_path = dir.join("hello.orv");
        let mut f = std::fs::File::create(&file_path).unwrap();
        write!(f, "@io.out \"hello\"").unwrap();

        let mut loader = SourceLoader::new(&dir);
        let id = loader.load("hello.orv");
        assert!(id.is_some());
        assert_eq!(loader.source_map().source(id.unwrap()), "@io.out \"hello\"");

        // Cleanup
        std::fs::remove_dir_all(&dir).ok();
    }
}
```

- [ ] **Step 2: Register module in `lib.rs`**

Add to `crates/orv-core/src/lib.rs` after the re-exports:

```rust
pub mod source;
```

- [ ] **Step 3: Run tests**

Run: `cargo test -p orv-core`
Expected: all 7 tests pass.

- [ ] **Step 4: Commit**

```bash
git add crates/orv-core/src/source.rs crates/orv-core/src/lib.rs
git commit -m "feat(core): add SourceLoader for filesystem and string source loading"
```

---

### Task 8: Create fixture directories and initial fixture files

**Files:**
- Create: `fixtures/lexer/.gitkeep`
- Create: `fixtures/parser/.gitkeep`
- Create: `fixtures/analyzer/.gitkeep`
- Create: `fixtures/runtime/.gitkeep`
- Create: `fixtures/ok/hello.orv`
- Create: `fixtures/ok/counter.orv`
- Create: `fixtures/ok/server-basic.orv`
- Create: `fixtures/err/domain-html-in-server.orv`
- Create: `fixtures/err/domain-route-in-ui.orv`
- Create: `fixtures/err/empty.orv`

- [ ] **Step 1: Create fixture directories**

```bash
mkdir -p fixtures/{lexer,parser,analyzer,runtime,ok,err}
touch fixtures/lexer/.gitkeep fixtures/parser/.gitkeep fixtures/analyzer/.gitkeep fixtures/runtime/.gitkeep
```

- [ ] **Step 2: Create `fixtures/ok/hello.orv`**

```orv
@io.out "Hello, orv!"
```

- [ ] **Step 3: Create `fixtures/ok/counter.orv`**

```orv
pub define CounterPage() -> @html {
  @body {
    let sig count: i32 = 0
    @text "{count}"
    @button "+" %onClick={count += 1}
    @button "-" %onClick={count -= 1}
  }
}
```

- [ ] **Step 4: Create `fixtures/ok/server-basic.orv`**

```orv
@server {
  @listen 8080

  @route GET /api/health {
    return @response 200 { "status": "ok" }
  }

  @route GET / {
    @serve ./public
  }
}
```

- [ ] **Step 5: Create `fixtures/err/domain-html-in-server.orv`**

```orv
// ERROR: @div is not valid in server context
@server {
  @listen 8080
  @div {
    @text "this should not be here"
  }
}
```

- [ ] **Step 6: Create `fixtures/err/domain-route-in-ui.orv`**

```orv
// ERROR: @route is not valid in UI context
@html {
  @body {
    @route GET /api/users {
      return @response 200 { "users": [] }
    }
  }
}
```

- [ ] **Step 7: Create `fixtures/err/empty.orv`**

Empty file (0 bytes).

- [ ] **Step 8: Commit**

```bash
git add fixtures/
git commit -m "chore: add fixture directories and initial orv fixture files"
```

---

### Task 9: Add golden-output snapshot test harness

**Files:**
- Create: `crates/orv-core/tests/fixture_harness.rs`

- [ ] **Step 1: Write the fixture harness as an integration test**

Create `crates/orv-core/tests/fixture_harness.rs`:

```rust
//! Golden-output fixture test harness.
//!
//! Convention:
//!   fixtures/ok/*.orv  — must load without diagnostics
//!   fixtures/err/*.orv — must produce at least one error diagnostic
//!
//! This harness validates that the source loader can read all fixtures
//! and that the ok/err classification holds. As the compiler grows,
//! this harness will expand to compare AST dumps and diagnostic snapshots.

use std::path::PathBuf;

fn fixtures_root() -> PathBuf {
    // Integration tests run from the crate directory; workspace root is two levels up.
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir.parent().unwrap().parent().unwrap().join("fixtures")
}

fn orv_files_in(dir: &std::path::Path) -> Vec<PathBuf> {
    if !dir.exists() {
        return Vec::new();
    }
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|ext| ext == "orv"))
        .collect();
    files.sort();
    files
}

#[test]
fn ok_fixtures_load_without_errors() {
    let ok_dir = fixtures_root().join("ok");
    let files = orv_files_in(&ok_dir);
    assert!(!files.is_empty(), "no .orv files found in fixtures/ok/");

    for path in &files {
        let source = std::fs::read_to_string(path).unwrap_or_else(|e| {
            panic!("failed to read {}: {e}", path.display());
        });
        let name = path.file_name().unwrap().to_string_lossy();

        let mut loader = orv_core::source::SourceLoader::new(ok_dir.clone());
        let _id = loader.load_string(&name, &source);

        // At the source-loading stage, ok fixtures should have no errors.
        assert!(
            !loader.has_errors(),
            "fixture {} produced load errors",
            path.display()
        );
    }
}

#[test]
fn err_fixtures_are_readable() {
    let err_dir = fixtures_root().join("err");
    let files = orv_files_in(&err_dir);
    assert!(!files.is_empty(), "no .orv files found in fixtures/err/");

    for path in &files {
        let source = std::fs::read_to_string(path).unwrap_or_else(|e| {
            panic!("failed to read {}: {e}", path.display());
        });
        let name = path.file_name().unwrap().to_string_lossy();

        // err fixtures should at least be readable (errors come from later phases).
        let mut loader = orv_core::source::SourceLoader::new(err_dir.clone());
        let id = loader.load_string(&name, &source);
        // Source loading itself should succeed (the file exists); errors are semantic.
        let source_text = loader.source_map().source(id);
        assert!(
            source_text.len() <= 10_000,
            "fixture {} unexpectedly large",
            path.display()
        );
    }
}

#[test]
fn empty_fixture_loads() {
    let err_dir = fixtures_root().join("err");
    let empty_path = err_dir.join("empty.orv");
    assert!(empty_path.exists(), "fixtures/err/empty.orv must exist");

    let source = std::fs::read_to_string(&empty_path).unwrap();
    assert!(source.is_empty(), "empty.orv should be empty");

    let mut loader = orv_core::source::SourceLoader::new(err_dir);
    let id = loader.load_string("empty.orv", &source);
    assert_eq!(loader.source_map().source(id), "");
}
```

- [ ] **Step 2: Run the harness**

Run: `cargo test -p orv-core --test fixture_harness`
Expected: all 3 tests pass.

- [ ] **Step 3: Commit**

```bash
git add crates/orv-core/tests/fixture_harness.rs
git commit -m "test: add golden-output fixture harness for ok/err fixture validation"
```

---

### Task 10: Add hidden debug CLI commands for fixture-driven development

**Files:**
- Modify: `crates/orv-cli/Cargo.toml`
- Modify: `crates/orv-cli/src/main.rs`

- [ ] **Step 1: Update `orv-cli/Cargo.toml`**

Add `orv-diagnostics` and `orv-span` under `[dependencies]`:

```toml
orv-span = { workspace = true }
orv-diagnostics = { workspace = true }
```

- [ ] **Step 2: Expand CLI with `check` and `dump` commands**

Replace `crates/orv-cli/src/main.rs` with:

```rust
use std::path::PathBuf;

use clap::Parser;

#[derive(Parser)]
#[command(name = "orv", version, about = "Integrated Platform Development DSL")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(clap::Subcommand)]
enum Commands {
    /// Display version information
    Version,
    /// Check a source file for errors
    Check {
        /// Path to the .orv source file
        file: PathBuf,
    },
    /// Dump internal representations
    Dump {
        #[command(subcommand)]
        target: DumpTarget,
    },
}

#[derive(clap::Subcommand)]
enum DumpTarget {
    /// Dump source file metadata (file id, line count, spans)
    Source {
        /// Path to the .orv source file
        file: PathBuf,
    },
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Version) | None => {
            println!("orv {}", orv_core::version());
        }
        Some(Commands::Check { file }) => {
            cmd_check(&file)?;
        }
        Some(Commands::Dump { target }) => match target {
            DumpTarget::Source { file } => {
                cmd_dump_source(&file)?;
            }
        },
    }

    Ok(())
}

fn cmd_check(file: &PathBuf) -> anyhow::Result<()> {
    let parent = file.parent().unwrap_or(std::path::Path::new("."));
    let name = file
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| file.display().to_string());

    let mut loader = orv_core::source::SourceLoader::new(parent);
    let id = loader.load_absolute(file, &name);

    if loader.has_errors() {
        let (source_map, bag) = loader.into_parts();
        let diags: Vec<_> = bag.into_vec();
        orv_diagnostics::render_diagnostics(&source_map, &diags);
        std::process::exit(1);
    }

    let id = id.unwrap();
    let source = loader.source_map().source(id);
    let line_count = loader.source_map().line_index(id).line_count();

    println!("check: {name} — {line_count} lines, {} bytes, ok", source.len());
    Ok(())
}

fn cmd_dump_source(file: &PathBuf) -> anyhow::Result<()> {
    let parent = file.parent().unwrap_or(std::path::Path::new("."));
    let name = file
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| file.display().to_string());

    let mut loader = orv_core::source::SourceLoader::new(parent);
    let id = loader.load_absolute(file, &name);

    if loader.has_errors() {
        let (source_map, bag) = loader.into_parts();
        let diags: Vec<_> = bag.into_vec();
        orv_diagnostics::render_diagnostics(&source_map, &diags);
        std::process::exit(1);
    }

    let id = id.unwrap();
    let source_map = loader.source_map();
    let line_index = source_map.line_index(id);

    println!("file: {name}");
    println!("file_id: {}", id.raw());
    println!("bytes: {}", source_map.source(id).len());
    println!("lines: {}", line_index.line_count());

    Ok(())
}
```

- [ ] **Step 3: Build and test CLI**

Run: `cargo build -p orv-cli`
Expected: compiles successfully.

Run: `cargo run -p orv-cli -- version`
Expected: `orv 0.1.0`

Run: `cargo run -p orv-cli -- check fixtures/ok/hello.orv`
Expected: `check: hello.orv — 1 lines, 21 bytes, ok`

Run: `cargo run -p orv-cli -- dump source fixtures/ok/server-basic.orv`
Expected: prints file metadata.

Run: `cargo run -p orv-cli -- check nonexistent.orv`
Expected: error diagnostic and exit code 1.

- [ ] **Step 4: Commit**

```bash
git add crates/orv-cli/
git commit -m "feat(cli): add check and dump source commands for fixture-driven development"
```

---

### Task 11: Run full workspace validation

**Files:** None (validation only)

- [ ] **Step 1: Run all workspace tests**

Run: `cargo test --workspace`
Expected: all tests pass across orv-span, orv-diagnostics, orv-core, orv-macros.

- [ ] **Step 2: Run clippy**

Run: `cargo clippy --workspace --all-targets`
Expected: no errors (warnings acceptable from pedantic/nursery).

- [ ] **Step 3: Run fmt check**

Run: `cargo fmt --all -- --check`
Expected: no formatting issues.

- [ ] **Step 4: Verify CLI end-to-end**

```bash
cargo run -p orv-cli -- check fixtures/ok/hello.orv
cargo run -p orv-cli -- check fixtures/ok/counter.orv
cargo run -p orv-cli -- check fixtures/ok/server-basic.orv
cargo run -p orv-cli -- dump source fixtures/ok/hello.orv
cargo run -p orv-cli -- check fixtures/err/empty.orv
```

Expected: all ok fixtures report success, empty.orv reports 0 bytes.

- [ ] **Step 5: Final commit if any fixes were needed**

```bash
git add -A
git commit -m "chore: fix lint and format issues from Phase 0 validation"
```

---

## Phase 0 Exit Criteria Checklist

Per the roadmap:

- [ ] Diagnostics render deterministically in tests — verified by Task 5 snapshot tests
- [ ] Span math is validated by unit tests — verified by Task 1-2 unit tests
- [ ] Fixture runner exists before lexer implementation starts — verified by Task 9
- [ ] Multiple files can be loaded and attributed by FileId — verified by Task 3 SourceMap tests
- [ ] Malformed source file reports exact location — verified by Task 4-5 rendering tests
- [ ] Cross-file import error can point to both files — verified by Task 5 cross-file test
