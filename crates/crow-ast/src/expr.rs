/// Imports
use crate::{
    atom::{BinOp, Lit, Mutability, Param, UnOp},
    stmt::Stmt,
};
use crow_common::span::Span;

/// Defines pattern kind
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PatKind {
    /// Represents literal pattern, e.g `123`
    Lit(Lit),

    /// Represents just enum variant pattern
    Variant(Expr),

    /// Represents enum fields unpack pattern
    Unpack(Expr, Vec<Pat>),

    /// Represents bind pattern
    BindTo(Mutability, String),

    /// Represents wildcard pattern
    Wildcard,

    /// Represents or pattern
    Or(Vec<Pat>),
}

/// Represents pattern
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Pat {
    pub span: Span,
    pub kind: PatKind,
}

/// Represents case in pattern matching
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Case {
    pub span: Span,
    pub pats: Vec<Pat>,
    pub body: Expr,
}

/// Defines expression kind
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ExprKind {
    /// Literal expression
    Lit(Lit),

    /// Represents unary expression
    Unary(Box<Expr>, UnOp),

    /// Represents binary expression
    Bin(Box<Expr>, Box<Expr>, BinOp),

    /// Assignment expression
    Assign(Box<Expr>, Box<Expr>),

    /// Represents if expression (cond, then, else)
    If(Box<Expr>, Box<Expr>, Option<Box<Expr>>),

    /// Represents variable access
    Var(String),

    /// Represents field access
    Field(Box<Expr>, String),

    /// Represents call expression
    Call(Box<Expr>, Vec<Expr>),

    /// Represents anonymous function expression
    Function(Vec<Param>, Box<Expr>),

    /// Represents match expression
    Match(Vec<Expr>, Vec<Case>),

    /// Represents paren expression
    Paren(Box<Expr>),

    /// Block expression
    Block(Vec<Stmt>),

    /// Represents todo expression (e.g `todo as "simple todo"`)
    Todo(Option<Box<Expr>>),

    /// Represents panic expression (e.g `panic as "simple panic"`)
    Panic(Option<Box<Expr>>),
}

/// Represents expression
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Expr {
    pub span: Span,
    pub kind: ExprKind,
}
