/// Imports
use miette::NamedSource;
use std::{
    fmt::Debug,
    ops::{Add, Range},
    sync::Arc,
};

/// Represents token kind
#[derive(Debug, PartialEq, Copy, Clone, Eq)]
pub enum TokenKind {
    Use,         // `use` keyword
    Enum,        // `enum` keyword
    Struct,      // `struct` keywword
    Let,         // `let` keyword
    If,          // `if` keyword
    Else,        // `else` keyword
    Fun,         // `fun` keyword
    Match,       // `match` keyword
    Pub,         // `pub` keyword
    As,          // `as` keyword
    For,         // `for` keyword
    None,        // `none` keyword
    Pure,        // `pure` keyword
    Todo,        // `todo` keyword
    Panic,       // `panic` keyword
    Const,       // `const` keyword
    Comma,       // ,
    Dot,         // .
    Lparen,      // (
    Rparen,      // )
    Lbrace,      // {
    Rbrace,      // }
    Lbracket,    // [
    Rbracket,    // ]
    Plus,        // +
    Minus,       // -
    Star,        // *
    Slash,       // /
    Percent,     // %
    Caret,       // ^
    Ampersand,   // &
    Bang,        // !
    Bar,         // |
    Eq,          // =
    Ge,          // >=
    Le,          // <=
    Gt,          // >
    Lt,          // <
    Colon,       // :
    Arrow,       // ->
    DoubleEq,    // ==
    DoubleBar,   // ||
    DoubleAmp,   // &&
    BangEq,      // !=
    PlusEq,      // +=
    MinusEq,     // -=
    StarEq,      // *=
    SlashEq,     // /=
    CaretEq,     // ^=
    PercentEq,   // %=
    BarEq,       // |=
    AmpersandEq, // &=
    Wildcard,    // _
    Number,      // any number
    String,      // "quoted text"
    Id,          // identifier
    Bool,        // bool
}

/// Represents span
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Span(pub Arc<NamedSource<String>>, pub Range<usize>);

impl Span {
    pub fn zeroed() -> Self {
        Self(Arc::new(NamedSource::new(String::from("??"), String::from("??"))), Range::default())
    }
}

/// Debug implementation
impl Debug for Span {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Span").field(&self.1).finish()
    }
}

/// Add implementation
impl Add for Span {
    type Output = Span;

    fn add(self, rhs: Self) -> Self::Output {
        // Checking that files are same
        if self.0 != rhs.0 {
            panic!("attempt to perform `+` operation on two spans from different files.")
        }

        // Calculating new span range
        let start = self.1.start.min(rhs.1.start);
        let end = self.1.end.max(rhs.1.end);
        Span(self.0, start..end)
    }
}

/// Represents token
#[derive(Debug, PartialEq, Clone, Eq)]
pub struct Token {
    pub span: Span,
    pub kind: TokenKind,
    pub lexeme: String,
}

/// Implementation
impl Token {
    /// Creates new token
    pub fn new(span: Span, kind: TokenKind, lexeme: String) -> Self {
        Self { span, kind, lexeme }
    }
}
