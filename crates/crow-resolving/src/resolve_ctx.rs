use std::collections::HashMap;

use crow_lex::token::Span;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct DefId(pub u32);


#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct LocalId(pub u32);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DefKind {
    Struct,
    Enum,
    Variant { enum_def: DefId, index: u32 },
    Fun,
    NativeFun,
    Const, 
    Module,
    BuiltinType
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Res {
    Def(DefKind, DefId),

    Local(LocalId),

    Err,
}

#[derive(Debug, Clone)]
pub struct FieldDef {
    pub name: String,
    pub index: u32,
    pub parent: DefId,  
}

#[derive(Debug, Clone)]
pub struct VariantDef {
    pub name: String,
    pub index: u32,
    pub parent: DefId,
    pub arity: usize,
}

#[derive(Debug, Default)]
pub struct ResolveCtxt {
    pub resolutions: HashMap<Span, Res>,

    pub def_kinds: HashMap<DefId, DefKind>,

    pub def_spans: HashMap<DefId, Span>,

    pub def_names: HashMap<DefId, String>,

    pub struct_fields: HashMap<DefId, Vec<FieldDef>>,

    pub enum_variants: HashMap<DefId, Vec<VariantDef>>,

    pub variant_by_name: HashMap<(DefId, String), DefId>,

    pub local_spans: HashMap<LocalId, Span>,

    pub local_names: HashMap<LocalId, String>,

    pub use_resolutions: HashMap<Span, Vec<(String, DefId)>>,

    pub module_exports: HashMap<DefId, HashMap<String, DefId>>,
}