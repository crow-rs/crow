/// Imports
use crate::{
    atom::{Mutability, TypeHint},
    expr::Expr,
};
use crow_common::span::Span;

/// Defines statement kind
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum StmtKind {
    /// Variable binding
    Binding(String, TypeHint, Mutability, Expr),

    /// Wilcard binding
    Wildcard(TypeHint, Expr),

    /// An expression
    Expr(Expr),
}

/// Represents statement
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Stmt {
    pub span: Span,
    pub kind: StmtKind,
}
