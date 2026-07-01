/// Imports
use crate::id::*;
use crow_ast::atom::Lit;
use crow_lex::token::Span;
use crow_resolving::table::{LocalId, Res};

/// Defines hir pattern
#[derive(Debug, Clone)]
pub struct HirPat {
    pub id: PatId,
    pub span: Span,
    pub kind: HirPatKind,
}

/// Defines hir pattern kind
#[derive(Debug, Clone)]
pub enum HirPatKind {
    Lit(Lit),
    Wildcard,
    Bind(LocalId, String),
    Variant(Res),
    Unpack(Res, Vec<PatId>),
    Or(Vec<PatId>),
}
