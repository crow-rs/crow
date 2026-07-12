pub mod macros;
pub mod span;

/// Definition id
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct DefId(pub u32);

/// Local variable id
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct LocalId(pub u32);