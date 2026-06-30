use crate::id::*;
use crow_ast::atom::Lit;
use crow_lex::token::Span;
use crow_resolving::resolve_ctx::{LocalId, Res};

#[derive(Debug, Clone)]
pub struct HirPat {
    pub id: PatId,
    pub span: Span,
    pub kind: HirPatKind,
}

#[derive(Debug, Clone)]
pub enum HirPatKind {
    Lit(Lit),

    Wildcard,

    Bind(LocalId, String),

    Variant(Res),

    Unpack(Res, Vec<PatId>),

    Or(Vec<PatId>),
}