/// Imports
use crate::{
    def::{Enum, Function, Module, Struct},
    typ::Var,
};
use crow_macros::bug;
use id_arena::{Arena, Id};

/// Represents types context that stores all the types info
pub struct TypesCtxt {
    /// Arenas for definitions
    pub structs: Arena<Struct>,
    pub enums: Arena<Enum>,
    pub functions: Arena<Function>,

    /// Modules arena
    pub mods: Arena<Module>,

    /// Type variables arena
    pub vars: Arena<Var>,
}

/// Implementation
impl Default for TypesCtxt {
    fn default() -> Self {
        Self::new()
    }
}

impl TypesCtxt {
    pub fn new() -> Self {
        Self {
            structs: Arena::new(),
            enums: Arena::new(),
            functions: Arena::new(),
            vars: Arena::new(),
            mods: Arena::new(),
        }
    }

    /// Inserts struct
    pub fn insert_struct(&mut self, s: Struct) -> Id<Struct> {
        self.structs.alloc(s)
    }

    /// Returns ref to struct
    pub fn get_struct(&self, id: Id<Struct>) -> &Struct {
        self.structs
            .get(id)
            .unwrap_or_else(|| bug!(format!("struct not found: {:?}", id)))
    }

    /// Returns mutable ref to struct
    pub fn get_struct_mut(&mut self, id: Id<Struct>) -> &mut Struct {
        self.structs
            .get_mut(id)
            .unwrap_or_else(|| bug!(format!("struct not found: {:?}", id)))
    }

    /// Inserts enum
    pub fn insert_enum(&mut self, e: Enum) -> Id<Enum> {
        self.enums.alloc(e)
    }

    /// Returns ref to enum
    pub fn get_enum(&self, id: Id<Enum>) -> &Enum {
        self.enums
            .get(id)
            .unwrap_or_else(|| bug!(format!("enum not found: {:?}", id)))
    }

    /// Returns mutable ref to struct
    pub fn get_enum_mut(&mut self, id: Id<Enum>) -> &mut Enum {
        self.enums
            .get_mut(id)
            .unwrap_or_else(|| bug!(format!("enum not found: {:?}", id)))
    }

    /// Inserts function
    pub fn insert_function(&mut self, f: Function) -> Id<Function> {
        self.functions.alloc(f)
    }

    /// Returns ref to function
    pub fn get_function(&self, id: Id<Function>) -> &Function {
        self.functions
            .get(id)
            .unwrap_or_else(|| bug!(format!("Function not found: {:?}", id)))
    }

    /// Returns mutable ref to function
    pub fn get_function_mut(&mut self, id: Id<Function>) -> &mut Function {
        self.functions
            .get_mut(id)
            .unwrap_or_else(|| bug!(format!("Function not found: {:?}", id)))
    }

    /// Inserts variable
    pub fn insert_var(&mut self, v: Var) -> Id<Var> {
        self.vars.alloc(v)
    }

    /// Returns ref to variable
    pub fn get_var(&self, id: Id<Var>) -> &Var {
        self.vars
            .get(id)
            .unwrap_or_else(|| bug!(format!("var not found: {:?}", id)))
    }

    /// Returns mutable ref to variable
    pub fn get_var_mut(&mut self, id: Id<Var>) -> &mut Var {
        self.vars
            .get_mut(id)
            .unwrap_or_else(|| bug!(format!("var not found: {:?}", id)))
    }

    /// Inserts module
    pub fn insert_mod(&mut self, s: Module) -> Id<Module> {
        self.mods.alloc(s)
    }

    /// Returns ref to module
    pub fn get_mod(&self, id: Id<Module>) -> &Module {
        self.mods
            .get(id)
            .unwrap_or_else(|| bug!(format!("module not found: {:?}", id)))
    }

    /// Returns mutable ref to struct
    pub fn get_mod_mut(&mut self, id: Id<Module>) -> &mut Module {
        self.mods
            .get_mut(id)
            .unwrap_or_else(|| bug!(format!("module not found: {:?}", id)))
    }
}
