/// Imports
use crate::typ::Typ;
use crow_ast::atom::Publicity;
use id_arena::Id;

/// Represents struct field
pub struct Field {
    pub name: String,
    pub typ: Typ,
}

/// Defines a structure in the type system
pub struct Struct {
    pub name: String,
    pub generics: Vec<String>,
    pub fields: Vec<Field>,
}

/// Represents enum variant
pub struct Variant {
    pub name: String,
    pub fields: Vec<Typ>,
}

/// Defines a structure in the type system
pub struct Enum {
    pub name: String,
    pub generics: Vec<String>,
    pub fields: Vec<Variant>,
}

/// Defines an algebraic data type
pub enum Adt {
    Enum(Enum),
    Struct(Struct),
}

/// Defines a function in the type system
pub struct Function {
    pub name: String,
    pub params: Vec<Typ>,
    pub ret: Typ,
}

/// Represents definition
pub enum Def {
    Adt(Id<Adt>),
    Function(Id<Function>),
    Const(Typ),
}

/// Represents module definition
pub type ModDef = (Publicity, Def);

/// Defines a module in the type system
pub struct Module {
    pub name: String,
    pub defs: Vec<ModDef>,
}
