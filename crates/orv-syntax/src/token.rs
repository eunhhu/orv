/// All distinct token kinds the orv lexer can produce.
#[derive(Debug, Clone, PartialEq)]
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

/// `f64` does not implement `Eq`, but for token comparison purposes two float
/// literals are considered equal when their bit patterns are identical.  NaN
/// cannot appear as a source literal, so this is safe in practice.
impl Eq for TokenKind {}

impl TokenKind {
    /// Returns `true` if this token is a keyword.
    pub const fn is_keyword(&self) -> bool {
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
