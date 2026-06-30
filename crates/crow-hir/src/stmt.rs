use crate::{id::*, ty::HirTy};
use crow_lex::token::Span;
use crow_resolving::resolve_ctx::LocalId;

#[derive(Debug, Clone)]
pub struct HirStmt {
    pub id: StmtId,
    pub span: Span,
    pub kind: HirStmtKind,
}

#[derive(Debug, Clone)]
pub enum HirStmtKind {
    Let {
        local_id: LocalId,
        name: String,
        ty: HirTy,
        init: ExprId,
    },
    Expr(ExprId),
}