/// Imports
use crow_lex::token::Span;
use std::collections::HashMap;

/// Definition id
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct DefId(pub u32);

/// Local variable id
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct LocalId(pub u32);

/// Definition kind
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DefKind {
    Struct,
    Enum,
    Variant { enum_def: DefId, index: u32 },
    Fun,
    NativeFun,
    Const,
    Module,
    BuiltinType,
}

/// Defines resolution
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Res {
    /// Definition
    Def(DefKind, DefId),

    /// Local variable
    Local(LocalId),

    /// Something other..?
    Err,
}

/// Struct field definition
#[derive(Debug, Clone)]
pub struct FieldDef {
    pub name: String,
    pub index: u32,
    pub parent: DefId,
}

/// Enum variant definition
#[derive(Debug, Clone)]
pub struct VariantDef {
    pub name: String,
    pub index: u32,
    pub parent: DefId,
    pub arity: usize,
}

/// Resolve table
#[derive(Debug, Default)]
pub struct ResolveTable {
    /// Resolutions mapping: Span -> Res
    pub resolutions: HashMap<Span, Res>,

    /// Definition kinds
    pub def_kinds: HashMap<DefId, DefKind>,

    /// Definition spans
    pub def_spans: HashMap<DefId, Span>,

    /// Definition names
    pub def_names: HashMap<DefId, String>,

    /// Struct fields
    pub struct_fields: HashMap<DefId, Vec<FieldDef>>,

    /// Enum variants
    pub enum_variants: HashMap<DefId, Vec<VariantDef>>,

    /// Variant mapping: (enum def id, name) -> variant def id
    pub variant_by_name: HashMap<(DefId, String), DefId>,

    /// Local spanbs
    pub local_spans: HashMap<LocalId, Span>,

    /// Local names
    pub local_names: HashMap<LocalId, String>,

    /// Use resolutions
    pub use_resolutions: HashMap<Span, Vec<(String, DefId)>>,

    /// Module exports mapping: module def id -> map (name -> def id)
    pub module_exports: HashMap<DefId, HashMap<String, DefId>>,

    /// Modules mapping: module name -> def id
    pub module_by_name: HashMap<String, DefId>,
}
