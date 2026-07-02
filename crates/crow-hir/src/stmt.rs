/// Imports
use crate::{id::*, ty::HirTy};
use crow_lex::token::Span;
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
    Variable {
        local_id: LocalId,
        name: String,
        ty: HirTy,
        init: ExprId,
        mutable: bool
    },
    WildcardAssign {
        ty: HirTy,
        init: ExprId
    },
    Expr(ExprId),
}
