/// Imports
use crate::{id::*, ty::HirTy};
use crow_ast::atom::Mutability;
use crow_common::span::Span;
use crow_resolving::table::LocalId;

/// Defines hir statement
#[derive(Debug, Clone)]
pub struct HirStmt {
    pub id: StmtId,
    pub span: Span,
    pub kind: HirStmtKind,
}

/// Defines hir statement knid
#[derive(Debug, Clone)]
pub enum HirStmtKind {
    Binding {
        local_id: LocalId,
        name: String,
        ty: HirTy,
        init: ExprId,
        mutability: Mutability,
    },
    Wildcard {
        ty: HirTy,
        init: ExprId,
    },
    Expr(ExprId),
}
