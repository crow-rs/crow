/// Modules
mod ribs;

/// Imports
use crate::{
    def::{Def, ModDef, Module},
    resolve::ribs::RibsStack,
};
use id_arena::Id;
use std::collections::HashMap;

/// Defines a resolver
pub struct Resolver {
    /// Scopes (ribs) stack
    ribs: RibsStack,

    /// Top-level defs
    defs: HashMap<String, ModDef>,

    /// Imported modules
    imported_mods: HashMap<String, Id<Module>>,

    /// Imported definitions
    imported_defs: HashMap<String, Def>,
}
