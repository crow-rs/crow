/// Imports
use crate::{
    atom::{BinOp, Lit, Param, TypeHint, UnOp},
    stmt::Stmt,
};
use crow_lex::token::Span;

/// Represents pattern
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Pat {
    pub span: Span,
    pub hint: TypeHint,
    pub bind: Option<String>,
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

    /// Index acesss
    Index(Box<Expr>, Box<Expr>),

    /// Represents call expression
    Call(Box<Expr>, Vec<Expr>),

    /// Represents anonymous function expression
    Function(Vec<Param>, Box<Expr>),

    /// Represents match expression
    Match(Vec<Expr>, Vec<Case>),

    /// Block expression
    Block(Vec<Stmt>),

    /// Represents todo expression (e.g `todo as "simple todo"`)
    Todo(Option<Box<Expr>>),

    /// Represents panic expression (e.g `panic as "simple panic"`)
    Panic(Option<Box<Expr>>),

    /// Alloc expression
    Alloc(Box<Expr>),
}

/// Represents expression
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Expr {
    pub span: Span,
    pub kind: ExprKind,
}
