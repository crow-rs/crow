/// Imports
use crate::typ::Typ;
use crow_ast::atom::Publicity;
use id_arena::Id;
use std::collections::HashMap;

/// Defines a structure in the type system
pub struct Struct {
    pub name: String,
    pub generics: Vec<String>,
    pub fields: HashMap<String, Typ>,
}

/// Defines a structure in the type system
pub struct Enum {
    pub name: String,
    pub generics: Vec<String>,
    pub variants: HashMap<String, Vec<Typ>>,
}

/// Defines a function in the type system
pub struct Function {
    pub name: String,
    pub generics: Vec<String>,
    pub params: Vec<Typ>,
    pub ret: Typ,
}

/// Represents definition
#[derive(Clone)]
pub enum Def {
    Struct(Id<Struct>),
    Enum(Id<Enum>),
    Function(Id<Function>),
    Const(Typ),
    Variant(Id<Enum>, String),
}

/// Represents module definition
pub type ModDef = (Publicity, Def);

/// Defines a module in the type system
pub struct Module {
    pub name: String,
    pub defs: HashMap<String, ModDef>,
}
