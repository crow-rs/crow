use crate::{expr::HirParam, id::*, ty::{HirEffects, HirTy}};
use crow_ast::atom::Publicity;
use crow_lex::token::Span;
use crow_resolving::resolve_ctx::DefId;

#[derive(Debug, Clone)]
pub struct HirItem {
    pub id: ItemId,
    pub def_id: DefId,
    pub publicity: Publicity,
    pub span: Span,
    pub kind: HirItemKind,
}

#[derive(Debug, Clone)]
pub enum HirItemKind {
    Struct(HirStructDef),
    Enum(HirEnumDef),
    Fun(HirFnDef),
    Native(HirNativeFnDef),
    Const(HirConstDef),
}

#[derive(Debug, Clone)]
pub struct HirStructDef {
    pub name: String,
    pub fields: Vec<HirFieldDef>,
}

#[derive(Debug, Clone)]
pub struct HirFieldDef {
    pub span: Span,
    pub name: String,
    pub index: u32,
    pub ty: HirTy,
}

#[derive(Debug, Clone)]
pub struct HirEnumDef {
    pub name: String,
    pub variants: Vec<HirVariantDef>,
}

#[derive(Debug, Clone)]
pub struct HirVariantDef {
    pub span: Span,
    pub def_id: DefId,
    pub name: String,
    pub index: u32,
    pub fields: Vec<HirTy>,
}

#[derive(Debug, Clone)]
pub struct HirFnDef {
    pub name: String,
    pub params: Vec<HirParam>,
    pub effects: HirEffects,
    pub ret: HirTy,
    pub body: BodyId,
}

#[derive(Debug, Clone)]
pub struct HirNativeFnDef {
    pub name: String,
    pub params: Vec<HirParam>,
    pub ret: HirTy,
    pub native_body: String,
}

#[derive(Debug, Clone)]
pub struct HirConstDef {
    pub name: String,
    pub ty: HirTy,
    pub body: BodyId,
}