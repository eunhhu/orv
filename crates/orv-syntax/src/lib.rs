//! orv-syntax — 렉서와 파서.
//!
//! SPEC.md §2 어휘 구조부터 §9까지 구문을 처리한다. 현재는 렉서만 구현
//! 되어 있으며, 파서는 이후 단계에서 추가된다.

#![warn(missing_docs)]

mod cursor;
mod lexer;
mod token;

pub use lexer::{lex, LexResult};
pub use token::{Keyword, Token, TokenKind};
