/// Imports
use crate::def::{Enum, Function, Module, Struct};
use id_arena::Id;

/// Defines a type variable
pub enum Var {
    /// Unbound type variable
    Unbound,

    /// Solved/bound type variable
    Bound(Typ),
}

/// Defines meta type
#[derive(Clone, PartialEq, Eq, Hash)]
pub enum Meta {
    Module(Id<Module>),
    Variant(Id<Enum>, usize),
    Struct(Id<Struct>),
    Enum(Id<Enum>),
}

/// Defines a type in the type system
#[derive(Clone, PartialEq, Eq, Hash)]
pub enum Typ {
    /// Primitive types
    Int,
    Float,
    Str,
    Bool,

    /// Function types
    FunRef(Box<Typ>, Vec<Typ>),
    Fun(Id<Function>, Vec<Typ>),

    /// Algebraic data types
    Struct(Id<Struct>, Vec<Typ>),
    Enum(Id<Enum>, Vec<Typ>),

    /// Inference variables
    Var(Id<Var>),
    Generic(String, usize),
    Meta(Meta),

    /// Unit type `()`
    Unit,

    /// An guaranteed error type
    Error,
}
