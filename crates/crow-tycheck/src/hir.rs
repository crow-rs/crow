use crow_ast::atom::{BinOp, UnOp};
use id_arena::Id;

use crate::{
    def::Enum,
    typ::{EffectRow, Typ},
};

pub struct HirModule {
    pub functions: Vec<HirFunction>,
    pub structs: Vec<HirStruct>,
    pub enums: Vec<HirEnum>,
    pub constants: Vec<HirConst>,
    pub natives: Vec<HirNativeFun>,
}

pub struct HirFunction {
    pub name: String,
    pub params: Vec<(String, Typ)>,
    pub ret: Typ,
    pub body: HirExpr,
    pub effects: EffectRow,
}

pub struct HirNativeFun {
    pub name: String,
    pub params: Vec<(String, Typ)>,
    pub ret: Typ,
    pub body: String,
    pub effects: EffectRow,
}

pub struct HirConst {
    pub name: String,
    pub ty: Typ,
    pub value: HirExpr,
}

pub struct HirStruct {
    pub name: String,
    pub fields: Vec<(String, Typ)>,
}

pub struct HirEnum {
    pub name: String,
    pub variants: Vec<HirVariant>,
}

pub struct HirVariant {
    pub name: String,
    pub fields: Vec<Typ>,
}

pub struct HirExpr {
    pub ty: Typ,
    pub kind: HirExprKind,
}

pub enum HirExprKind {
    Lit(HirLit),
    Var(String),
    BinOp(BinOp, Box<HirExpr>, Box<HirExpr>),
    UnOp(UnOp, Box<HirExpr>),
    Assign(Box<HirExpr>, Box<HirExpr>),
    If(Box<HirExpr>, Box<HirExpr>, Option<Box<HirExpr>>),
    Field(Box<HirExpr>, String, usize),
    Call(Box<HirExpr>, Vec<HirExpr>),
    Lambda(Vec<(String, Typ)>, Box<HirExpr>),
    Match(Vec<HirExpr>, Vec<HirCase>),
    Block(Vec<HirStmt>),
    Todo(Option<Box<HirExpr>>),
    Panic(Option<Box<HirExpr>>),
}

pub enum HirStmt {
    Let(String, Typ, HirExpr),
    Expr(HirExpr),
}

pub enum HirLit {
    Int(i64),
    Float(f64),
    Bool(bool),
    Str(String),
    Unit,
}

pub struct HirCase {
    pub pats: Vec<HirPat>,
    pub body: HirExpr,
}

pub enum HirPat {
    Lit(HirLit),
    Variant(Id<Enum>, usize),
    Unpack(Id<Enum>, usize, Vec<HirPat>),
    Bind(String, Typ),
    Wildcard,
    Or(Vec<HirPat>),
}
