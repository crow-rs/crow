/// Imports
use crate::{id::*, ty::HirTy};
use crow_ast::atom::{BinOp, Lit, UnOp};
use crow_common::{LocalId, span::Span};
use crow_resolving::table::{Res};

/// Defines hir expression
#[derive(Debug, Clone)]
pub struct HirExpr {
    pub id: ExprId,
    pub span: Span,
    pub kind: HirExprKind,
}

/// Defines hir expression kind
#[derive(Debug, Clone)]
pub enum HirExprKind {
    Lit(Lit),
    Var(Res),
    Unary(ExprId, UnOp),
    Bin(ExprId, ExprId, BinOp),
    Assign(ExprId, ExprId),
    If(ExprId, ExprId, Option<ExprId>),
    Field(ExprId, String),
    Call(ExprId, Vec<ExprId>),
    Lambda {
        params: Vec<HirParam>,
        body: ExprId,
    },
    Match {
        subjects: Vec<ExprId>,
        arms: Vec<HirArm>,
    },
    Block(Vec<StmtId>),
    Diverge(DivergeKind, Option<ExprId>),
    Cast {
        expr: ExprId, 
        ty: HirTy
    },
    Rec {
        res: Res,
        fields: Vec<(String, ExprId)>,
    },
}

/// Defines diverge kind
#[derive(Debug, Clone, Copy)]
pub enum DivergeKind {
    Todo,
}

/// Defines hir parameter
#[derive(Debug, Clone)]
pub struct HirParam {
    pub span: Span,
    pub local_id: LocalId,
    pub name: String,
    pub ty: HirTy,
}

/// Defines hir arm
#[derive(Debug, Clone)]
pub struct HirArm {
    pub span: Span,
    pub pats: Vec<PatId>,
    pub body: ExprId,
}
