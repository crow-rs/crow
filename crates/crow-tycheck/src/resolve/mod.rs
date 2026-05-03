/// Modules
mod generics;
mod ribs;

/// Imports
use crate::{
    def::{Def, ModDef, Module},
    resolve::{generics::GenericsStack, ribs::RibsStack},
    typ::Typ,
};
use id_arena::Id;
use std::collections::HashMap;

/// Defines a resolver
#[derive(Default)]
pub struct Resolver {
    /// Scopes (ribs) stack
    ribs: RibsStack,

    /// Generics stack
    generics: GenericsStack,

    /// Top-level defs
    defs: HashMap<String, ModDef>,

    /// Imported modules
    imported_mods: HashMap<String, Id<Module>>,

    /// Imported definitions
    imported_defs: HashMap<String, Def>,
}

/// Implementation
impl Resolver {
    /// Enters generics scope
    pub fn enter_generics(&mut self, g: Vec<String>) {
        self.generics.enter(g);
    }

    /// Exits generics scope
    pub fn exit_generics(&mut self) {
        self.generics.exit();
    }

    /// Enters ribs scope
    pub fn enter_scope(&mut self) {
        self.ribs.push();
    }

    /// Exits ribs scope
    pub fn exit_scope(&mut self) {
        self.ribs.pop();
    }

    /// Resolves module-level def
    pub fn resolve_mod_def(&mut self, name: &str) -> Option<Def> {
        match self.defs.get(name) {
            Some(def) => Some(def.1.clone()),
            None => self.imported_defs.get(name).map(|def| def.clone()),
        }
    }

    /// Resolves local-level def
    pub fn resolve_local_def(&mut self, name: &str) -> Option<Typ> {
        self.ribs.lookup(name).map(|t| t)
    }

    /// Resolves generic
    pub fn resolve_generic(&mut self, name: &str) -> Option<usize> {
        self.generics.lookup(name).map(|t| t)
    }

    /// Resolves imported module
    pub fn resolve_mod(&mut self, name: &str) -> Option<Id<Module>> {
        self.imported_mods.get(name).cloned()
    }

    /// Declares module-level def. Returns true on success
    pub fn declare_mod_def(&mut self, name: &str, def: ModDef) -> bool {
        if !self.defs.contains_key(name) {
            self.defs.insert(name.to_string(), def);
            true
        } else {
            false
        }
    }

    /// Declares local-level type
    pub fn declare_local_def(&mut self, name: &str, t: Typ) {
        self.ribs.define(name, t);
    }

    /// Declares module
    pub fn declare_mod(&mut self, name: &str, m: Id<Module>) {
        self.imported_mods.insert(name.to_string(), m);
    }
}
