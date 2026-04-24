/// Imports
use crate::{atom::TypeHint, expr::Expr};
use crow_lex::token::Span;

/// Defines statement kind
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum StmtKind {
    /// Let definition
    Let(String, TypeHint, Expr),

    /// An expression
    Expr(Expr),
}

/// Represents statement
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Stmt {
    pub span: Span,
    pub kind: StmtKind,
}
