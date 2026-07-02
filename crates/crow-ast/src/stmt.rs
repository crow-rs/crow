/// Imports
use crate::{atom::TypeHint, expr::Expr};
use crow_lex::token::Span;

/// Defines statement kind
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum StmtKind {
    /// Var definition
    Variable(String, TypeHint, Expr, bool), //last param - immutability, true - mutable, false immutable

    WildcardAssign(TypeHint, Expr),

    /// An expression
    Expr(Expr),
}

/// Represents statement
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Stmt {
    pub span: Span,
    pub kind: StmtKind,
}
