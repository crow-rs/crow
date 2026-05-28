/// Imports
use crate::{
    atom::{Mutability, TypeHint},
    expr::Expr,
};
use crow_lex::token::Span;

/// Defines statement kind
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum StmtKind {
    /// Let definition
    Let {
        name: String,
        mutability: Mutability,
        hint: TypeHint,
        expr: Expr,
    },

    /// Drop statement
    Drop(Expr),

    /// An expression
    Expr(Expr),
}

/// Represents statement
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Stmt {
    pub span: Span,
    pub kind: StmtKind,
}
