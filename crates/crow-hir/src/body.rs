use crate::{expr::HirExpr, id::*, pat::HirPat, stmt::HirStmt};

#[derive(Debug, Clone)]
pub struct HirBody {
    pub id: BodyId,
    pub root_expr: ExprId,
    pub exprs: Vec<HirExpr>,
    pub stmts: Vec<HirStmt>,
    pub pats: Vec<HirPat>,
}

impl HirBody {
    pub fn expr(&self, id: ExprId) -> &HirExpr {
        &self.exprs[id.as_index()]
    }

    pub fn stmt(&self, id: StmtId) -> &HirStmt {
        &self.stmts[id.as_index()]
    }

    pub fn pat(&self, id: PatId) -> &HirPat {
        &self.pats[id.as_index()]
    }
}