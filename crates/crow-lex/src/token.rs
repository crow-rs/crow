/// Imports
use crow_common::span::Span;
use std::fmt::Debug;

/// Represents token kind
#[derive(Debug, PartialEq, Copy, Clone, Eq)]
pub enum TokenKind {
    Use,         // `use` keyword
    Enum,        // `enum` keyword
    Rec,         // adt rec keywword
    Alt,         // adt sum type keyword
    Val,         // `val` keyword
    Var,         // `var` keyword
    If,          // `if` keyword
    Else,        // `else` keyword
    Fun,         // `fun` keyword
    Match,       // `match` keyword
    Pub,         // `pub` keyword
    Native,      // 'native' keyword
    As,          // `as` keyword
    For,         // `for` keyword
    None,        // `none` keyword
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
