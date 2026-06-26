/// Imports
use crate::typ::{Effects, Typ, Var};
use crow_ast::atom::Publicity;
use crow_lex::token::Span;
use id_arena::Id;
use std::collections::HashMap;

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct EffectRow {
    pub known: Vec<Effects>,
    pub tail: Option<Id<Var>>
}

/// Defines a field in the type system
#[derive(Clone)]
pub struct Field {
    pub span: Span,
    pub name: String,
    pub typ: Typ,
}

/// Defines a structure in the type system
pub struct Struct {
    pub name: String,
    pub generics: Vec<String>,
    pub fields: Vec<Field>,
}

/// Defines an enum variant in the type system
#[derive(Clone)]
pub struct Variant {
    pub span: Span,
    pub name: String,
    pub fields: Vec<Typ>,
}

/// Defines an enum in the type system
pub struct Enum {
    pub name: String,
    pub generics: Vec<String>,
    pub variants: Vec<Variant>,
}

/// Defines a function in the type system
pub struct Function {
    pub name: String,
    pub generics: Vec<String>,
    pub params: Vec<Typ>,
    pub ret: Typ,
    pub effects: EffectRow
}

/// Represents definition
#[derive(Clone)]
pub enum Def {
    Struct(Id<Struct>),
    Enum(Id<Enum>),
    Function(Id<Function>),
    Const(Typ),
    Variant(Id<Enum>, usize),
}

/// Represents module definition
pub type ModDef = (Publicity, Def);

/// Defines a module in the type system
pub struct Module {
    pub name: String,
    pub defs: HashMap<String, ModDef>,
}
