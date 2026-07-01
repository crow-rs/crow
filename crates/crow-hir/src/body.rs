/// Imports
use crate::{expr::HirExpr, id::*, pat::HirPat, stmt::HirStmt};
use crow_fresh::FreshenVec;

/// Defines hir body
#[derive(Debug, Clone)]
pub struct HirBody {
    /// An id
    pub id: BodyId,

    /// Root expression id
    pub root_expr: ExprId,

    /// Expressions mapping: ExprId -> HirExpr
    pub exprs: FreshenVec<u32, HirExpr>,

    /// Statements mapping: StmtId -> HirStmt
    pub stmts: FreshenVec<u32, HirStmt>,

    /// Patterns mapping: PatId -> HirPat
    pub pats: FreshenVec<u32, HirPat>,
}

/// Body implementation
impl HirBody {
    /// Returns expression by id
    pub fn expr(&self, id: ExprId) -> &HirExpr {
        &self.exprs.item_at(id.0)
    }

    /// Returns statement by id
    pub fn stmt(&self, id: StmtId) -> &HirStmt {
        &self.stmts.item_at(id.0)
    }

    /// Returns pattern by id
    pub fn pat(&self, id: PatId) -> &HirPat {
        &self.pats.item_at(id.0)
    }
}
