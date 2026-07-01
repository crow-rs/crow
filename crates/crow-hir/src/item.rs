/// Imports
use crate::{
    expr::HirParam,
    id::*,
    ty::{HirEffects, HirTy},
};
use crow_ast::atom::Publicity;
use crow_lex::token::Span;
use crow_resolving::table::DefId;

/// Defines hir item
#[derive(Debug, Clone)]
pub struct HirItem {
    pub id: ItemId,
    pub def_id: DefId,
    pub publicity: Publicity,
    pub span: Span,
    pub kind: HirItemKind,
}

/// Defines hir item kind
#[derive(Debug, Clone)]
pub enum HirItemKind {
    Struct(HirStructDef),
    Enum(HirEnumDef),
    Fun(HirFnDef),
    Native(HirNativeFnDef),
    Const(HirConstDef),
}

/// Defines hir struct def
#[derive(Debug, Clone)]
pub struct HirStructDef {
    pub name: String,
    pub fields: Vec<HirFieldDef>,
}

/// Defines hir struct field
#[derive(Debug, Clone)]
pub struct HirFieldDef {
    pub span: Span,
    pub name: String,
    pub index: u32,
    pub ty: HirTy,
}

/// Defines hir enum def
#[derive(Debug, Clone)]
pub struct HirEnumDef {
    pub name: String,
    pub variants: Vec<HirVariantDef>,
}

/// Defines hir variant def
#[derive(Debug, Clone)]
pub struct HirVariantDef {
    pub span: Span,
    pub def_id: DefId,
    pub name: String,
    pub index: u32,
    pub fields: Vec<HirTy>,
}

/// Defines hir function def
#[derive(Debug, Clone)]
pub struct HirFnDef {
    pub name: String,
    pub params: Vec<HirParam>,
    pub effects: HirEffects,
    pub ret: HirTy,
    pub body: BodyId,
}

/// Defines hir native function def
#[derive(Debug, Clone)]
pub struct HirNativeFnDef {
    pub name: String,
    pub params: Vec<HirParam>,
    pub ret: HirTy,
    pub native_body: String,
}

/// Defines hir const def
#[derive(Debug, Clone)]
pub struct HirConstDef {
    pub name: String,
    pub ty: HirTy,
    pub body: BodyId,
}
