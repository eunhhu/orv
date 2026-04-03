# Phase 1: Lexer — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Convert orv source text into a stable token stream with spans, handling all documented syntax including `@nodes`, `%properties`, string interpolation, comments, and line-oriented structure.

**Architecture:** A new `orv-syntax` crate contains the token enum and a hand-written lexer. The lexer is a single-pass byte scanner that produces `Spanned<Token>` values, emitting diagnostics for malformed input via `DiagnosticBag`. Newlines are significant tokens (orv is line-oriented). Comments are stripped but their spans are tracked. String interpolation (`"Hello {name}"`) is tokenized into segments.

**Tech Stack:** Rust (edition 2024, nightly), `orv-span` (FileId, Span, Spanned), `orv-diagnostics` (DiagnosticBag, Diagnostic, Label), `pretty_assertions` for tests.

---

## File Structure

```text
crates/
  orv-syntax/
    Cargo.toml
    src/
      lib.rs            — crate root, re-exports
      token.rs          — Token enum, TokenKind, keyword lookup
      lexer.rs          — Lexer struct, tokenization logic
```

---

### Task 1: Create `orv-syntax` crate with Token types

**Files:**
- Create: `crates/orv-syntax/Cargo.toml`
- Create: `crates/orv-syntax/src/lib.rs`
- Create: `crates/orv-syntax/src/token.rs`
- Modify: `Cargo.toml` (workspace deps)

- [ ] **Step 1: Create `crates/orv-syntax/Cargo.toml`**

```toml
[package]
name = "orv-syntax"
description = "Lexer, parser, and AST for the orv language"
version.workspace = true
authors.workspace = true
edition.workspace = true
license.workspace = true
repository.workspace = true
rust-version.workspace = true

[dependencies]
orv-span = { workspace = true }
orv-diagnostics = { workspace = true }

[dev-dependencies]
pretty_assertions = { workspace = true }

[lints]
workspace = true
```

- [ ] **Step 2: Add `orv-syntax` to workspace deps in root `Cargo.toml`**

Add `orv-syntax = { path = "crates/orv-syntax" }` to `[workspace.dependencies]`.

- [ ] **Step 3: Create `src/lib.rs`**

```rust
pub mod token;
```

- [ ] **Step 4: Create `src/token.rs` with `TokenKind` enum**

The token kinds cover the full orv language surface from the docs:

```rust
/// All distinct token kinds the orv lexer can produce.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenKind {
    // ── Structural ──────────────────────────────────────
    /// `@` prefix for node declarations
    At,
    /// `%` prefix for property bindings
    Percent,

    // ── Delimiters ──────────────────────────────────────
    /// `{`
    LBrace,
    /// `}`
    RBrace,
    /// `(`
    LParen,
    /// `)`
    RParen,
    /// `[`
    LBracket,
    /// `]`
    RBracket,

    // ── Punctuation ─────────────────────────────────────
    /// `,`
    Comma,
    /// `.`
    Dot,
    /// `:`
    Colon,
    /// `::`
    ColonColon,
    /// `=`
    Eq,
    /// `==`
    EqEq,
    /// `!=`
    BangEq,
    /// `!`
    Bang,
    /// `<`
    Lt,
    /// `<=`
    LtEq,
    /// `>`
    Gt,
    /// `>=`
    GtEq,
    /// `+`
    Plus,
    /// `+=`
    PlusEq,
    /// `-`
    Minus,
    /// `-=`
    MinusEq,
    /// `->`
    Arrow,
    /// `*`
    Star,
    /// `/`
    Slash,
    /// `|`
    Pipe,
    /// `|>`
    PipeGt,
    /// `||`
    PipePipe,
    /// `&&`
    AmpAmp,
    /// `&`
    Amp,
    /// `?`
    Question,
    /// `#`
    Hash,
    /// `..`
    DotDot,
    /// `..=`
    DotDotEq,
    /// `...`
    Ellipsis,
    /// `$`
    Dollar,

    // ── Literals ────────────────────────────────────────
    /// Integer literal (e.g., `42`, `0xFF`)
    IntLiteral(i64),
    /// Float literal (e.g., `3.14`)
    FloatLiteral(f64),

    /// A plain string literal with no interpolation: `"hello"`
    StringLiteral(String),
    /// Start of an interpolated string: `"Hello {`
    StringInterpStart(String),
    /// Middle segment between interpolation holes: `} and {`
    StringInterpMiddle(String),
    /// End segment after last interpolation hole: `} world"`
    StringInterpEnd(String),

    // ── Identifiers & Keywords ──────────────────────────
    /// A plain identifier (variable, function, type name)
    Ident(String),

    // ─── Keywords ───────────────────────────────────────
    Let,
    Mut,
    Sig,
    Const,
    Function,
    Async,
    Await,
    Return,
    If,
    Else,
    For,
    Of,
    While,
    When,
    Import,
    Pub,
    Define,
    Struct,
    Enum,
    Type,
    True,
    False,
    Void,
    Try,
    Catch,

    // ── Whitespace & Structure ──────────────────────────
    /// A newline (`\n` or `\r\n`)
    Newline,

    // ── Special ─────────────────────────────────────────
    /// End of file
    Eof,
    /// An unrecognized or invalid token
    Error,
}

impl TokenKind {
    /// Returns `true` if this token is a keyword.
    pub fn is_keyword(&self) -> bool {
        matches!(
            self,
            Self::Let
                | Self::Mut
                | Self::Sig
                | Self::Const
                | Self::Function
                | Self::Async
                | Self::Await
                | Self::Return
                | Self::If
                | Self::Else
                | Self::For
                | Self::Of
                | Self::While
                | Self::When
                | Self::Import
                | Self::Pub
                | Self::Define
                | Self::Struct
                | Self::Enum
                | Self::Type
                | Self::True
                | Self::False
                | Self::Void
                | Self::Try
                | Self::Catch
        )
    }
}

/// Look up whether an identifier string is a keyword.
pub fn lookup_keyword(ident: &str) -> Option<TokenKind> {
    match ident {
        "let" => Some(TokenKind::Let),
        "mut" => Some(TokenKind::Mut),
        "sig" => Some(TokenKind::Sig),
        "const" => Some(TokenKind::Const),
        "function" => Some(TokenKind::Function),
        "async" => Some(TokenKind::Async),
        "await" => Some(TokenKind::Await),
        "return" => Some(TokenKind::Return),
        "if" => Some(TokenKind::If),
        "else" => Some(TokenKind::Else),
        "for" => Some(TokenKind::For),
        "of" => Some(TokenKind::Of),
        "while" => Some(TokenKind::While),
        "when" => Some(TokenKind::When),
        "import" => Some(TokenKind::Import),
        "pub" => Some(TokenKind::Pub),
        "define" => Some(TokenKind::Define),
        "struct" => Some(TokenKind::Struct),
        "enum" => Some(TokenKind::Enum),
        "type" => Some(TokenKind::Type),
        "true" => Some(TokenKind::True),
        "false" => Some(TokenKind::False),
        "void" => Some(TokenKind::Void),
        "try" => Some(TokenKind::Try),
        "catch" => Some(TokenKind::Catch),
        _ => None,
    }
}
```

- [ ] **Step 5: Add unit tests for `lookup_keyword` and `is_keyword`**

In `token.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keyword_lookup_hits() {
        assert_eq!(lookup_keyword("let"), Some(TokenKind::Let));
        assert_eq!(lookup_keyword("define"), Some(TokenKind::Define));
        assert_eq!(lookup_keyword("void"), Some(TokenKind::Void));
    }

    #[test]
    fn keyword_lookup_misses() {
        assert_eq!(lookup_keyword("foo"), None);
        assert_eq!(lookup_keyword("Let"), None);
        assert_eq!(lookup_keyword(""), None);
    }

    #[test]
    fn is_keyword_check() {
        assert!(TokenKind::Let.is_keyword());
        assert!(TokenKind::Define.is_keyword());
        assert!(!TokenKind::Ident("foo".into()).is_keyword());
        assert!(!TokenKind::At.is_keyword());
    }
}
```

- [ ] **Step 6: Verify**

Run: `cargo test -p orv-syntax`
Expected: 3 tests pass.

Run: `cargo clippy -p orv-syntax`
Expected: clean.

- [ ] **Step 7: Commit**

```bash
git add crates/orv-syntax/ Cargo.toml Cargo.lock
git commit -m "feat(syntax): add orv-syntax crate with TokenKind enum and keyword lookup"
```

---

### Task 2: Lexer skeleton — single-character tokens and whitespace

**Files:**
- Create: `crates/orv-syntax/src/lexer.rs`
- Modify: `crates/orv-syntax/src/lib.rs`

- [ ] **Step 1: Create the `Lexer` struct in `src/lexer.rs`**

```rust
use orv_diagnostics::{Diagnostic, DiagnosticBag, Label};
use orv_span::{FileId, Span, Spanned};

use crate::token::{lookup_keyword, TokenKind};

/// A lexer that converts source text into a stream of spanned tokens.
pub struct Lexer<'src> {
    source: &'src [u8],
    file: FileId,
    pos: u32,
    diagnostics: DiagnosticBag,
}

impl<'src> Lexer<'src> {
    /// Create a new lexer for the given source text and file id.
    pub fn new(source: &'src str, file: FileId) -> Self {
        Self {
            source: source.as_bytes(),
            file,
            pos: 0,
            diagnostics: DiagnosticBag::new(),
        }
    }

    /// Tokenize the entire source, returning all tokens and accumulated diagnostics.
    pub fn tokenize(mut self) -> (Vec<Spanned<TokenKind>>, DiagnosticBag) {
        let mut tokens = Vec::new();
        loop {
            let tok = self.next_token();
            let is_eof = tok.node() == &TokenKind::Eof;
            tokens.push(tok);
            if is_eof {
                break;
            }
        }
        (tokens, self.diagnostics)
    }

    fn next_token(&mut self) -> Spanned<TokenKind> {
        self.skip_horizontal_whitespace();

        let start = self.pos;

        let Some(&byte) = self.peek() else {
            return self.make_token(start, TokenKind::Eof);
        };

        match byte {
            b'\n' => {
                self.advance();
                // Handle \r\n as a single newline
                self.make_token(start, TokenKind::Newline)
            }
            b'\r' => {
                self.advance();
                if self.peek() == Some(&b'\n') {
                    self.advance();
                }
                self.make_token(start, TokenKind::Newline)
            }
            b'@' => { self.advance(); self.make_token(start, TokenKind::At) }
            b'%' => { self.advance(); self.make_token(start, TokenKind::Percent) }
            b'{' => { self.advance(); self.make_token(start, TokenKind::LBrace) }
            b'}' => { self.advance(); self.make_token(start, TokenKind::RBrace) }
            b'(' => { self.advance(); self.make_token(start, TokenKind::LParen) }
            b')' => { self.advance(); self.make_token(start, TokenKind::RParen) }
            b'[' => { self.advance(); self.make_token(start, TokenKind::LBracket) }
            b']' => { self.advance(); self.make_token(start, TokenKind::RBracket) }
            b',' => { self.advance(); self.make_token(start, TokenKind::Comma) }
            b':' => {
                self.advance();
                if self.peek() == Some(&b':') {
                    self.advance();
                    self.make_token(start, TokenKind::ColonColon)
                } else {
                    self.make_token(start, TokenKind::Colon)
                }
            }
            b'?' => { self.advance(); self.make_token(start, TokenKind::Question) }
            b'#' => {
                self.advance();
                self.make_token(start, TokenKind::Hash)
            }
            b'$' => { self.advance(); self.make_token(start, TokenKind::Dollar) }
            b'&' => {
                self.advance();
                if self.peek() == Some(&b'&') {
                    self.advance();
                    self.make_token(start, TokenKind::AmpAmp)
                } else {
                    self.make_token(start, TokenKind::Amp)
                }
            }
            _ => {
                self.advance();
                self.diagnostics.push(
                    Diagnostic::error(format!("unexpected character `{}`", byte as char))
                        .with_label(Label::primary(
                            Span::new(self.file, start, self.pos),
                            "unexpected character",
                        )),
                );
                self.make_token(start, TokenKind::Error)
            }
        }
    }

    // ── Helpers ────────────────────────────────────────

    fn peek(&self) -> Option<&u8> {
        self.source.get(self.pos as usize)
    }

    fn peek_at(&self, offset: u32) -> Option<u8> {
        self.source.get((self.pos + offset) as usize).copied()
    }

    fn advance(&mut self) -> u8 {
        let byte = self.source[self.pos as usize];
        self.pos += 1;
        byte
    }

    fn skip_horizontal_whitespace(&mut self) {
        while let Some(&b) = self.peek() {
            if b == b' ' || b == b'\t' {
                self.advance();
            } else {
                break;
            }
        }
    }

    fn make_token(&self, start: u32, kind: TokenKind) -> Spanned<TokenKind> {
        Spanned::new(kind, Span::new(self.file, start, self.pos))
    }
}
```

- [ ] **Step 2: Register module in `lib.rs`**

```rust
pub mod lexer;
pub mod token;
```

- [ ] **Step 3: Add tests for single-char tokens and whitespace**

In `lexer.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    fn lex(source: &str) -> Vec<TokenKind> {
        let lexer = Lexer::new(source, FileId::new(0));
        let (tokens, _) = lexer.tokenize();
        tokens.into_iter().map(|t| t.node().clone()).collect()
    }

    fn lex_with_diags(source: &str) -> (Vec<TokenKind>, DiagnosticBag) {
        let lexer = Lexer::new(source, FileId::new(0));
        let (tokens, diags) = lexer.tokenize();
        (tokens.into_iter().map(|t| t.node().clone()).collect(), diags)
    }

    #[test]
    fn empty_source() {
        assert_eq!(lex(""), vec![TokenKind::Eof]);
    }

    #[test]
    fn whitespace_only() {
        assert_eq!(lex("   \t  "), vec![TokenKind::Eof]);
    }

    #[test]
    fn single_char_tokens() {
        assert_eq!(
            lex("@%{}()[],:?#$"),
            vec![
                TokenKind::At,
                TokenKind::Percent,
                TokenKind::LBrace,
                TokenKind::RBrace,
                TokenKind::LParen,
                TokenKind::RParen,
                TokenKind::LBracket,
                TokenKind::RBracket,
                TokenKind::Comma,
                TokenKind::Colon,
                TokenKind::Question,
                TokenKind::Hash,
                TokenKind::Dollar,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn multi_char_tokens() {
        assert_eq!(
            lex(":: &&"),
            vec![TokenKind::ColonColon, TokenKind::AmpAmp, TokenKind::Eof]
        );
    }

    #[test]
    fn newlines() {
        assert_eq!(
            lex("@\n%"),
            vec![TokenKind::At, TokenKind::Newline, TokenKind::Percent, TokenKind::Eof]
        );
    }

    #[test]
    fn crlf_newline() {
        assert_eq!(
            lex("@\r\n%"),
            vec![TokenKind::At, TokenKind::Newline, TokenKind::Percent, TokenKind::Eof]
        );
    }

    #[test]
    fn unknown_char_emits_error() {
        let (tokens, diags) = lex_with_diags("~");
        assert_eq!(tokens, vec![TokenKind::Error, TokenKind::Eof]);
        assert!(diags.has_errors());
    }
}
```

- [ ] **Step 4: Verify**

Run: `cargo test -p orv-syntax`
Expected: 10 tests pass (3 from token.rs + 7 from lexer.rs).

- [ ] **Step 5: Commit**

```bash
git add crates/orv-syntax/src/
git commit -m "feat(syntax): add lexer skeleton with single-char tokens and whitespace handling"
```

---

### Task 3: Operators and multi-character punctuation

**Files:**
- Modify: `crates/orv-syntax/src/lexer.rs`

- [ ] **Step 1: Add operator arms to `next_token` match**

Add these arms to the existing match in `next_token`, before the `_ =>` fallback:

```rust
            b'=' => {
                self.advance();
                if self.peek() == Some(&b'=') {
                    self.advance();
                    self.make_token(start, TokenKind::EqEq)
                } else {
                    self.make_token(start, TokenKind::Eq)
                }
            }
            b'!' => {
                self.advance();
                if self.peek() == Some(&b'=') {
                    self.advance();
                    self.make_token(start, TokenKind::BangEq)
                } else {
                    self.make_token(start, TokenKind::Bang)
                }
            }
            b'<' => {
                self.advance();
                if self.peek() == Some(&b'=') {
                    self.advance();
                    self.make_token(start, TokenKind::LtEq)
                } else {
                    self.make_token(start, TokenKind::Lt)
                }
            }
            b'>' => {
                self.advance();
                if self.peek() == Some(&b'=') {
                    self.advance();
                    self.make_token(start, TokenKind::GtEq)
                } else {
                    self.make_token(start, TokenKind::Gt)
                }
            }
            b'+' => {
                self.advance();
                if self.peek() == Some(&b'=') {
                    self.advance();
                    self.make_token(start, TokenKind::PlusEq)
                } else {
                    self.make_token(start, TokenKind::Plus)
                }
            }
            b'-' => {
                self.advance();
                match self.peek() {
                    Some(&b'=') => {
                        self.advance();
                        self.make_token(start, TokenKind::MinusEq)
                    }
                    Some(&b'>') => {
                        self.advance();
                        self.make_token(start, TokenKind::Arrow)
                    }
                    _ => self.make_token(start, TokenKind::Minus),
                }
            }
            b'*' => { self.advance(); self.make_token(start, TokenKind::Star) }
            b'|' => {
                self.advance();
                match self.peek() {
                    Some(&b'>') => {
                        self.advance();
                        self.make_token(start, TokenKind::PipeGt)
                    }
                    Some(&b'|') => {
                        self.advance();
                        self.make_token(start, TokenKind::PipePipe)
                    }
                    _ => self.make_token(start, TokenKind::Pipe),
                }
            }
            b'.' => {
                self.advance();
                if self.peek() == Some(&b'.') {
                    self.advance();
                    if self.peek() == Some(&b'=') {
                        self.advance();
                        self.make_token(start, TokenKind::DotDotEq)
                    } else if self.peek() == Some(&b'.') {
                        self.advance();
                        self.make_token(start, TokenKind::Ellipsis)
                    } else {
                        self.make_token(start, TokenKind::DotDot)
                    }
                } else {
                    self.make_token(start, TokenKind::Dot)
                }
            }
```

- [ ] **Step 2: Add tests for operators**

```rust
    #[test]
    fn comparison_operators() {
        assert_eq!(
            lex("== != < <= > >="),
            vec![
                TokenKind::EqEq, TokenKind::BangEq,
                TokenKind::Lt, TokenKind::LtEq,
                TokenKind::Gt, TokenKind::GtEq,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn assignment_operators() {
        assert_eq!(
            lex("= += -="),
            vec![TokenKind::Eq, TokenKind::PlusEq, TokenKind::MinusEq, TokenKind::Eof]
        );
    }

    #[test]
    fn arithmetic_operators() {
        assert_eq!(
            lex("+ - * /"),
            vec![TokenKind::Plus, TokenKind::Minus, TokenKind::Star, TokenKind::Slash, TokenKind::Eof]
        );
    }

    #[test]
    fn arrow_and_pipe() {
        assert_eq!(
            lex("-> |> || | !"),
            vec![
                TokenKind::Arrow, TokenKind::PipeGt, TokenKind::PipePipe,
                TokenKind::Pipe, TokenKind::Bang, TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn dot_variants() {
        assert_eq!(
            lex(". .. ..= ..."),
            vec![
                TokenKind::Dot, TokenKind::DotDot, TokenKind::DotDotEq,
                TokenKind::Ellipsis, TokenKind::Eof,
            ]
        );
    }
```

- [ ] **Step 3: Verify**

Run: `cargo test -p orv-syntax`
Expected: 15 tests pass.

- [ ] **Step 4: Commit**

```bash
git add crates/orv-syntax/src/lexer.rs
git commit -m "feat(syntax): add operator and multi-char punctuation tokenization"
```

---

### Task 4: Comments (line and block)

**Files:**
- Modify: `crates/orv-syntax/src/lexer.rs`

- [ ] **Step 1: Handle `/` as comment-or-slash in `next_token`**

Replace the handling for `b'/'` — instead of a simple slash arm, add logic for `//`, `///`, and `/* */`:

Add this arm to the match, before `_ =>`:

```rust
            b'/' => {
                self.advance();
                match self.peek() {
                    Some(&b'/') => {
                        // Line comment — consume until end of line
                        self.advance();
                        while let Some(&b) = self.peek() {
                            if b == b'\n' {
                                break;
                            }
                            self.advance();
                        }
                        // Don't emit a token for comments; recurse to get next real token
                        return self.next_token();
                    }
                    Some(&b'*') => {
                        // Block comment — consume until */
                        self.advance();
                        let mut depth: u32 = 1;
                        while depth > 0 {
                            match self.peek() {
                                None => {
                                    self.diagnostics.push(
                                        Diagnostic::error("unterminated block comment")
                                            .with_label(Label::primary(
                                                Span::new(self.file, start, self.pos),
                                                "comment starts here",
                                            )),
                                    );
                                    return self.make_token(start, TokenKind::Error);
                                }
                                Some(&b'*') => {
                                    self.advance();
                                    if self.peek() == Some(&b'/') {
                                        self.advance();
                                        depth -= 1;
                                    }
                                }
                                Some(&b'/') => {
                                    self.advance();
                                    if self.peek() == Some(&b'*') {
                                        self.advance();
                                        depth += 1;
                                    }
                                }
                                _ => { self.advance(); }
                            }
                        }
                        return self.next_token();
                    }
                    _ => self.make_token(start, TokenKind::Slash),
                }
            }
```

- [ ] **Step 2: Add tests**

```rust
    #[test]
    fn line_comment_stripped() {
        assert_eq!(
            lex("@\n// this is a comment\n%"),
            vec![TokenKind::At, TokenKind::Newline, TokenKind::Newline, TokenKind::Percent, TokenKind::Eof]
        );
    }

    #[test]
    fn doc_comment_stripped() {
        assert_eq!(
            lex("/// doc comment\n@"),
            vec![TokenKind::Newline, TokenKind::At, TokenKind::Eof]
        );
    }

    #[test]
    fn block_comment_stripped() {
        assert_eq!(
            lex("@ /* comment */ %"),
            vec![TokenKind::At, TokenKind::Percent, TokenKind::Eof]
        );
    }

    #[test]
    fn nested_block_comments() {
        assert_eq!(
            lex("@ /* outer /* inner */ still comment */ %"),
            vec![TokenKind::At, TokenKind::Percent, TokenKind::Eof]
        );
    }

    #[test]
    fn unterminated_block_comment() {
        let (tokens, diags) = lex_with_diags("@ /* never closed");
        assert_eq!(tokens, vec![TokenKind::At, TokenKind::Error, TokenKind::Eof]);
        assert!(diags.has_errors());
    }

    #[test]
    fn slash_alone_is_operator() {
        assert_eq!(lex("4 / 2"), vec![TokenKind::Eof]); // will be updated once numbers are added
        // For now just verify the slash token
        let tokens = lex("/");
        assert_eq!(tokens, vec![TokenKind::Slash, TokenKind::Eof]);
    }
```

- [ ] **Step 3: Verify**

Run: `cargo test -p orv-syntax`
Expected: 21 tests pass.

- [ ] **Step 4: Commit**

```bash
git add crates/orv-syntax/src/lexer.rs
git commit -m "feat(syntax): add line and block comment handling with nesting support"
```

---

### Task 5: Identifiers and keywords

**Files:**
- Modify: `crates/orv-syntax/src/lexer.rs`

- [ ] **Step 1: Add identifier/keyword scanning to `next_token`**

Add this arm before `_ =>`:

```rust
            b'a'..=b'z' | b'A'..=b'Z' | b'_' => {
                self.advance();
                while let Some(&b) = self.peek() {
                    if b.is_ascii_alphanumeric() || b == b'_' {
                        self.advance();
                    } else {
                        break;
                    }
                }
                let text = std::str::from_utf8(&self.source[start as usize..self.pos as usize])
                    .expect("identifier should be valid ASCII");
                let kind = lookup_keyword(text)
                    .unwrap_or_else(|| TokenKind::Ident(text.to_owned()));
                self.make_token(start, kind)
            }
```

- [ ] **Step 2: Add tests**

```rust
    #[test]
    fn identifiers() {
        assert_eq!(
            lex("foo bar_baz _x A1"),
            vec![
                TokenKind::Ident("foo".into()),
                TokenKind::Ident("bar_baz".into()),
                TokenKind::Ident("_x".into()),
                TokenKind::Ident("A1".into()),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn keywords_recognized() {
        assert_eq!(
            lex("let mut sig const function define"),
            vec![
                TokenKind::Let, TokenKind::Mut, TokenKind::Sig,
                TokenKind::Const, TokenKind::Function, TokenKind::Define,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn keyword_prefix_is_ident() {
        // "lettuce" is not the keyword "let"
        assert_eq!(
            lex("lettuce define2"),
            vec![
                TokenKind::Ident("lettuce".into()),
                TokenKind::Ident("define2".into()),
                TokenKind::Eof,
            ]
        );
    }
```

- [ ] **Step 3: Verify**

Run: `cargo test -p orv-syntax`
Expected: 24 tests pass.

- [ ] **Step 4: Commit**

```bash
git add crates/orv-syntax/src/lexer.rs
git commit -m "feat(syntax): add identifier and keyword tokenization"
```

---

### Task 6: Number literals (integers and floats)

**Files:**
- Modify: `crates/orv-syntax/src/lexer.rs`

- [ ] **Step 1: Add number scanning to `next_token`**

Add before `_ =>`:

```rust
            b'0'..=b'9' => {
                self.lex_number(start)
            }
```

Add the `lex_number` method:

```rust
    fn lex_number(&mut self, start: u32) -> Spanned<TokenKind> {
        // Consume leading digits
        while let Some(&b) = self.peek() {
            if b.is_ascii_digit() {
                self.advance();
            } else {
                break;
            }
        }

        // Check for float (dot followed by digit)
        if self.peek() == Some(&b'.') && self.peek_at(1).is_some_and(|b| b.is_ascii_digit()) {
            self.advance(); // consume '.'
            while let Some(&b) = self.peek() {
                if b.is_ascii_digit() {
                    self.advance();
                } else {
                    break;
                }
            }
            let text = std::str::from_utf8(&self.source[start as usize..self.pos as usize])
                .expect("number should be ASCII");
            let value: f64 = text.parse().unwrap_or_else(|_| {
                self.diagnostics.push(
                    Diagnostic::error(format!("invalid float literal `{text}`"))
                        .with_label(Label::primary(
                            Span::new(self.file, start, self.pos),
                            "here",
                        )),
                );
                0.0
            });
            self.make_token(start, TokenKind::FloatLiteral(value))
        } else {
            let text = std::str::from_utf8(&self.source[start as usize..self.pos as usize])
                .expect("number should be ASCII");
            let value: i64 = text.parse().unwrap_or_else(|_| {
                self.diagnostics.push(
                    Diagnostic::error(format!("invalid integer literal `{text}`"))
                        .with_label(Label::primary(
                            Span::new(self.file, start, self.pos),
                            "here",
                        )),
                );
                0
            });
            self.make_token(start, TokenKind::IntLiteral(value))
        }
    }
```

- [ ] **Step 2: Add tests**

```rust
    #[test]
    fn integer_literals() {
        assert_eq!(
            lex("0 42 12345"),
            vec![
                TokenKind::IntLiteral(0),
                TokenKind::IntLiteral(42),
                TokenKind::IntLiteral(12345),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn float_literals() {
        assert_eq!(
            lex("3.14 0.5"),
            vec![
                TokenKind::FloatLiteral(3.14),
                TokenKind::FloatLiteral(0.5),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn dot_after_int_is_not_float() {
        // "42.." should be int 42, then DotDot
        assert_eq!(
            lex("42..100"),
            vec![
                TokenKind::IntLiteral(42),
                TokenKind::DotDot,
                TokenKind::IntLiteral(100),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn number_before_dot_method() {
        // "42.len" should be int 42, dot, ident
        assert_eq!(
            lex("42.len"),
            vec![
                TokenKind::IntLiteral(42),
                TokenKind::Dot,
                TokenKind::Ident("len".into()),
                TokenKind::Eof,
            ]
        );
    }
```

- [ ] **Step 3: Verify**

Run: `cargo test -p orv-syntax`
Expected: 28 tests pass.

- [ ] **Step 4: Commit**

```bash
git add crates/orv-syntax/src/lexer.rs
git commit -m "feat(syntax): add integer and float literal tokenization"
```

---

### Task 7: String literals with interpolation

**Files:**
- Modify: `crates/orv-syntax/src/lexer.rs`

- [ ] **Step 1: Add string scanning to `next_token`**

Add before `_ =>`:

```rust
            b'"' => {
                self.lex_string(start)
            }
```

Add `lex_string` method:

```rust
    fn lex_string(&mut self, start: u32) -> Spanned<TokenKind> {
        self.advance(); // consume opening '"'
        let mut buf = String::new();
        loop {
            match self.peek() {
                None | Some(&b'\n') => {
                    self.diagnostics.push(
                        Diagnostic::error("unterminated string literal")
                            .with_label(Label::primary(
                                Span::new(self.file, start, self.pos),
                                "string starts here",
                            )),
                    );
                    return self.make_token(start, TokenKind::Error);
                }
                Some(&b'"') => {
                    self.advance();
                    return self.make_token(start, TokenKind::StringLiteral(buf));
                }
                Some(&b'{') => {
                    self.advance();
                    return self.make_token(start, TokenKind::StringInterpStart(buf));
                }
                Some(&b'\\') => {
                    self.advance();
                    match self.peek() {
                        Some(&b'n') => { self.advance(); buf.push('\n'); }
                        Some(&b't') => { self.advance(); buf.push('\t'); }
                        Some(&b'r') => { self.advance(); buf.push('\r'); }
                        Some(&b'\\') => { self.advance(); buf.push('\\'); }
                        Some(&b'"') => { self.advance(); buf.push('"'); }
                        Some(&b'{') => { self.advance(); buf.push('{'); }
                        Some(&b'}') => { self.advance(); buf.push('}'); }
                        _ => {
                            let esc_start = self.pos - 1;
                            if let Some(&b) = self.peek() {
                                self.advance();
                                self.diagnostics.push(
                                    Diagnostic::error(format!("unknown escape sequence `\\{}`", b as char))
                                        .with_label(Label::primary(
                                            Span::new(self.file, esc_start, self.pos),
                                            "unknown escape",
                                        )),
                                );
                                buf.push(b as char);
                            }
                        }
                    }
                }
                _ => {
                    // Regular character — could be multi-byte UTF-8
                    let byte = self.advance();
                    if byte.is_ascii() {
                        buf.push(byte as char);
                    } else {
                        // Read remaining UTF-8 continuation bytes
                        let start_byte = self.pos - 1;
                        let char_len = utf8_char_len(byte);
                        for _ in 1..char_len {
                            if self.peek().is_some() {
                                self.advance();
                            }
                        }
                        let s = std::str::from_utf8(
                            &self.source[start_byte as usize..self.pos as usize],
                        );
                        match s {
                            Ok(ch) => buf.push_str(ch),
                            Err(_) => buf.push(char::REPLACEMENT_CHARACTER),
                        }
                    }
                }
            }
        }
    }

    /// Resume scanning a string after an interpolation expression `}`.
    /// Call this when the parser encounters `}` inside a string interpolation.
    pub fn continue_string(&mut self) -> Spanned<TokenKind> {
        let start = self.pos;
        let mut buf = String::new();
        loop {
            match self.peek() {
                None | Some(&b'\n') => {
                    self.diagnostics.push(
                        Diagnostic::error("unterminated string literal")
                            .with_label(Label::primary(
                                Span::new(self.file, start, self.pos),
                                "string continues here",
                            )),
                    );
                    return self.make_token(start, TokenKind::Error);
                }
                Some(&b'"') => {
                    self.advance();
                    return self.make_token(start, TokenKind::StringInterpEnd(buf));
                }
                Some(&b'{') => {
                    self.advance();
                    return self.make_token(start, TokenKind::StringInterpMiddle(buf));
                }
                Some(&b'\\') => {
                    self.advance();
                    match self.peek() {
                        Some(&b'n') => { self.advance(); buf.push('\n'); }
                        Some(&b't') => { self.advance(); buf.push('\t'); }
                        Some(&b'\\') => { self.advance(); buf.push('\\'); }
                        Some(&b'"') => { self.advance(); buf.push('"'); }
                        Some(&b'{') => { self.advance(); buf.push('{'); }
                        Some(&b'}') => { self.advance(); buf.push('}'); }
                        _ => {
                            if let Some(&b) = self.peek() {
                                self.advance();
                                buf.push(b as char);
                            }
                        }
                    }
                }
                _ => {
                    let byte = self.advance();
                    if byte.is_ascii() {
                        buf.push(byte as char);
                    } else {
                        let start_byte = self.pos - 1;
                        let char_len = utf8_char_len(byte);
                        for _ in 1..char_len {
                            if self.peek().is_some() {
                                self.advance();
                            }
                        }
                        let s = std::str::from_utf8(
                            &self.source[start_byte as usize..self.pos as usize],
                        );
                        match s {
                            Ok(ch) => buf.push_str(ch),
                            Err(_) => buf.push(char::REPLACEMENT_CHARACTER),
                        }
                    }
                }
            }
        }
    }
```

Add helper function at module level (outside impl):

```rust
fn utf8_char_len(first_byte: u8) -> u32 {
    match first_byte {
        0..=0x7F => 1,
        0xC0..=0xDF => 2,
        0xE0..=0xEF => 3,
        0xF0..=0xFF => 4,
        _ => 1,
    }
}
```

- [ ] **Step 2: Add tests**

```rust
    #[test]
    fn simple_string() {
        assert_eq!(
            lex(r#""hello""#),
            vec![TokenKind::StringLiteral("hello".into()), TokenKind::Eof]
        );
    }

    #[test]
    fn string_with_escapes() {
        assert_eq!(
            lex(r#""a\nb""#),
            vec![TokenKind::StringLiteral("a\nb".into()), TokenKind::Eof]
        );
    }

    #[test]
    fn string_interpolation_start() {
        // "Hello {" starts interpolation
        assert_eq!(
            lex(r#""Hello {"#),
            vec![TokenKind::StringInterpStart("Hello ".into()), TokenKind::Eof]
        );
    }

    #[test]
    fn unterminated_string() {
        let (tokens, diags) = lex_with_diags("\"hello");
        assert_eq!(tokens, vec![TokenKind::Error, TokenKind::Eof]);
        assert!(diags.has_errors());
    }

    #[test]
    fn string_with_unicode() {
        assert_eq!(
            lex("\"안녕\""),
            vec![TokenKind::StringLiteral("안녕".into()), TokenKind::Eof]
        );
    }

    #[test]
    fn empty_string() {
        assert_eq!(
            lex(r#""""#),
            vec![TokenKind::StringLiteral(String::new()), TokenKind::Eof]
        );
    }

    #[test]
    fn escaped_braces_in_string() {
        assert_eq!(
            lex(r#""\{not interp\}""#),
            vec![TokenKind::StringLiteral("{not interp}".into()), TokenKind::Eof]
        );
    }
```

- [ ] **Step 3: Verify**

Run: `cargo test -p orv-syntax`
Expected: 35 tests pass.

- [ ] **Step 4: Commit**

```bash
git add crates/orv-syntax/src/lexer.rs
git commit -m "feat(syntax): add string literal and interpolation tokenization"
```

---

### Task 8: Lexer integration test with fixture files

**Files:**
- Create: `fixtures/lexer/hello.orv`
- Create: `fixtures/lexer/operators.orv`
- Create: `fixtures/lexer/string-interp.orv`
- Create: `crates/orv-syntax/tests/lexer_fixtures.rs`

- [ ] **Step 1: Create fixture files**

`fixtures/lexer/hello.orv`:
```orv
@io.out "Hello, orv!"
```

`fixtures/lexer/operators.orv`:
```orv
let x = 1 + 2
let y = x * 3
if x >= y {
  return x
}
```

`fixtures/lexer/string-interp.orv`:
```orv
let name = "World"
@text "Hello, {name}!"
```

- [ ] **Step 2: Create integration test**

`crates/orv-syntax/tests/lexer_fixtures.rs`:

```rust
use std::path::PathBuf;

use orv_span::FileId;
use orv_syntax::lexer::Lexer;
use orv_syntax::token::TokenKind;

fn fixtures_root() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("fixtures")
}

fn lex_fixture(name: &str) -> (Vec<TokenKind>, bool) {
    let path = fixtures_root().join("lexer").join(name);
    let source = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
    let lexer = Lexer::new(&source, FileId::new(0));
    let (tokens, diags) = lexer.tokenize();
    let kinds: Vec<TokenKind> = tokens.into_iter().map(|t| t.node().clone()).collect();
    (kinds, diags.has_errors())
}

#[test]
fn hello_fixture_lexes_cleanly() {
    let (tokens, has_errors) = lex_fixture("hello.orv");
    assert!(!has_errors, "hello.orv should lex without errors");
    // Should contain: At, Ident("io"), Dot, Ident("out"), StringLiteral, Newline, Eof
    assert!(tokens.contains(&TokenKind::At));
    assert!(tokens.contains(&TokenKind::StringLiteral("Hello, orv!".into())));
    assert!(tokens.last() == Some(&TokenKind::Eof));
}

#[test]
fn operators_fixture_lexes_cleanly() {
    let (tokens, has_errors) = lex_fixture("operators.orv");
    assert!(!has_errors, "operators.orv should lex without errors");
    assert!(tokens.contains(&TokenKind::Let));
    assert!(tokens.contains(&TokenKind::Plus));
    assert!(tokens.contains(&TokenKind::Star));
    assert!(tokens.contains(&TokenKind::GtEq));
    assert!(tokens.contains(&TokenKind::Return));
}

#[test]
fn string_interp_fixture_lexes_cleanly() {
    let (tokens, has_errors) = lex_fixture("string-interp.orv");
    assert!(!has_errors, "string-interp.orv should lex without errors");
    assert!(tokens.contains(&TokenKind::StringLiteral("World".into())));
    // "Hello, {" is an interpolation start
    assert!(tokens.contains(&TokenKind::StringInterpStart("Hello, ".into())));
}
```

- [ ] **Step 3: Verify**

Run: `cargo test -p orv-syntax --test lexer_fixtures`
Expected: 3 tests pass.

- [ ] **Step 4: Commit**

```bash
git add fixtures/lexer/ crates/orv-syntax/tests/
git commit -m "test(syntax): add lexer fixture files and integration tests"
```

---

### Task 9: Wire lexer into orv-cli `dump tokens` command

**Files:**
- Modify: `crates/orv-cli/Cargo.toml`
- Modify: `crates/orv-cli/src/main.rs`

- [ ] **Step 1: Add `orv-syntax` dependency to `orv-cli`**

In `crates/orv-cli/Cargo.toml` `[dependencies]`:

```toml
orv-syntax = { workspace = true }
```

- [ ] **Step 2: Add `Tokens` variant to `DumpTarget` and implement**

Add to `DumpTarget` enum:

```rust
    /// Dump token stream for a source file
    Tokens {
        /// Path to the .orv source file
        file: PathBuf,
    },
```

Add match arm in the dump handler:

```rust
            DumpTarget::Tokens { file } => {
                cmd_dump_tokens(&file)?;
            }
```

Add the implementation function:

```rust
fn cmd_dump_tokens(file: &PathBuf) -> anyhow::Result<()> {
    let (source_map, file_id, _) = load_source(file)?;

    let source = source_map.source(file_id);
    let lexer = orv_syntax::lexer::Lexer::new(source, file_id);
    let (tokens, diags) = lexer.tokenize();

    if diags.has_errors() {
        let diag_vec: Vec<_> = diags.into_vec();
        orv_diagnostics::render_diagnostics(&source_map, &diag_vec);
    }

    for token in &tokens {
        let span = token.span();
        let (_, line, col) = source_map.resolve(span);
        println!(
            "{:>4}:{:<3} {:?}",
            line + 1,
            col,
            token.node()
        );
    }

    Ok(())
}
```

(Note: `load_source` is the existing helper that returns `(SourceMap, FileId, ...)`. Adjust to match the actual signature — read the current main.rs to verify the exact helper name and signature.)

- [ ] **Step 3: Test CLI**

Run: `cargo run -p orv-cli -- dump tokens fixtures/lexer/hello.orv`
Expected: prints each token with line:col and token kind.

Run: `cargo run -p orv-cli -- dump tokens fixtures/ok/counter.orv`
Expected: prints token stream without errors.

- [ ] **Step 4: Commit**

```bash
git add crates/orv-cli/
git commit -m "feat(cli): add dump tokens command for lexer inspection"
```

---

### Task 10: Full workspace validation

**Files:** None (validation only)

- [ ] **Step 1: Run all workspace tests**

Run: `cargo test --workspace`
Expected: all tests pass.

- [ ] **Step 2: Run clippy**

Run: `cargo clippy --workspace --all-targets`
Expected: clean.

- [ ] **Step 3: Run fmt check**

Run: `cargo fmt --all -- --check`
Expected: clean.

- [ ] **Step 4: Test CLI end-to-end**

```bash
cargo run -p orv-cli -- dump tokens fixtures/ok/hello.orv
cargo run -p orv-cli -- dump tokens fixtures/ok/counter.orv
cargo run -p orv-cli -- dump tokens fixtures/ok/server-basic.orv
cargo run -p orv-cli -- dump tokens fixtures/lexer/string-interp.orv
```

Expected: all fixtures tokenize without errors, output looks reasonable.

- [ ] **Step 5: Commit if any fixes were needed**

---

## Phase 1 Exit Criteria (from roadmap)

- [ ] All parser fixtures start from stable token snapshots
- [ ] Lexer produces no panic on fuzzed small inputs
- [ ] Token stream covers: `@`, `%`, keywords, identifiers, numbers, strings with interpolation, operators, comments, newlines
- [ ] Diagnostics emitted for: unterminated strings, unterminated block comments, unknown characters
