/// Imports
use crate::def;
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
    Module(Id<def::Module>),
    Variant(Id<def::Enum>, usize),
    Struct(Id<def::Struct>),
    Enum(Id<def::Enum>),
    Effect(Id<def::Effect>),
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub enum IntBitness {
    I8,
    I16,
    I32,
    I64
}

/// Defines a type in the type system
#[derive(Clone, PartialEq, Eq, Hash)]
pub enum Typ {
    /// Primitive types
    Int(IntBitness),
    Float,
    Str,
    Bool,

    /// Function types
    FunRef(Box<Typ>, Vec<Typ>, EffectRow),
    Fun(Id<def::Function>, Vec<Typ>, EffectRow),

    /// Algebraic data types
    Struct(Id<def::Struct>, Vec<Typ>),
    Enum(Id<def::Enum>, Vec<Typ>),

    /// Inference variables
    Var(Id<Var>),
    Generic(String, usize),
    Meta(Meta),

    /// Unit type `()`
    Unit,

    /// An guaranteed error type
    Error,

    // infered primitives
    InferInt(Id<Var>)
}

/// Defines an effect
#[derive(Clone, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum Effect {
    Total,
    Exn,
    Div,
    Ndet,
    Console,
    IO,
    UserDefined(Id<def::Effect>),
}

/// Defines effects row
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct EffectRow {
    pub known: Vec<Effect>,
    pub tail: Option<Id<Var>>,
}
