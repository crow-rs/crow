// hir/expr.rs

use crate::{id::*, ty::HirTy};
use crow_ast::atom::{BinOp, Lit, UnOp};
use crow_lex::token::Span;
use crow_resolving::resolve_ctx::{LocalId, Res};

#[derive(Debug, Clone)]
pub struct HirExpr {
    pub id: ExprId,
    pub span: Span,
    pub kind: HirExprKind,
}

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
        scrutinees: Vec<ExprId>,
        arms: Vec<HirArm>,
    },

    Block(Vec<StmtId>),

    Diverge(DivergeKind, Option<ExprId>),
}

#[derive(Debug, Clone, Copy)]
pub enum DivergeKind {
    Todo,
    Panic,
}

#[derive(Debug, Clone)]
pub struct HirParam {
    pub span: Span,
    pub local_id: LocalId,
    pub name: String,
    pub ty: HirTy,
}

#[derive(Debug, Clone)]
pub struct HirArm {
    pub span: Span,
    pub pats: Vec<PatId>,
    pub body: ExprId,
}