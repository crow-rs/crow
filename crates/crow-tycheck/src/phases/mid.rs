/// Imports
use crate::{
    ctxt::check::CheckCtxt,
    def::{Def, Field, Function, Variant},
};
use crow_ast::{
    atom::Publicity,
    item::{Enum, Fun, ItemKind, Module, NativeFun, Struct},
};

/// Implementation of middle check phase
impl<'tx> CheckCtxt<'tx> {
    /// Performs mid analysis of struct:
    /// - infers types of all the fields
    /// - prepares all the fields
    pub fn mid_analyze_struct(&mut self, s: &Struct) {
        // Getting struct def
        self.resolver.enter_generics(s.generics.clone());
        match self
            .resolver
            .resolve_mod_def(&s.name)
            .expect("struct should exists after early analysis")
        {
            // Updating definition
            Def::Struct(id) => {
                self.tx.get_struct_mut(id).fields = s
                    .fields
                    .iter()
                    .map(|f| Field {
                        name: f.name.clone(),
                        typ: self.infer_type_hint(f.hint.clone()),
                    })
                    .collect()
            }
            _ => unreachable!(),
        }
        self.resolver.exit_generics();
    }

    /// Performs mid analysis of struct:
    /// - infers types of all the fields in all the variants
    /// - prepares all the variants
    pub fn mid_analyze_enum(&mut self, e: &Enum) {
        // Getting enum def
        self.resolver.enter_generics(e.generics.clone());
        match self
            .resolver
            .resolve_mod_def(&e.name)
            .expect("enum should exists after early analysis")
        {
            // Updating definition
            Def::Enum(id) => {
                self.tx.get_enum_mut(id).variants = e
                    .variants
                    .iter()
                    .map(|v| Variant {
                        name: v.name.clone(),
                        fields: v
                            .fields
                            .iter()
                            .map(|hint| self.infer_type_hint(hint.clone()))
                            .collect(),
                    })
                    .collect()
            }
            _ => unreachable!(),
        }
        self.resolver.exit_generics();
    }

    /// Performs mid analysis of function:
    /// - infers types of all the params
    /// - prepares function definition
    pub fn mid_analyze_fun(&mut self, p: Publicity, f: &Fun) {
        // Preparing function definition
        self.resolver.enter_generics(f.generics.clone());
        let def = Function {
            name: f.name.clone(),
            params: f
                .params
                .iter()
                .map(|param| self.infer_type_hint(param.hint.clone()))
                .collect(),
            ret: self.infer_type_hint(f.ret.clone()),
        };
        self.resolver.exit_generics();
        // Declaring function
        self.resolver
            .declare_mod_def(&f.name, (p, Def::Function(self.tx.insert_function(def))));
    }

    /// Performs mid analysis of native function:
    /// - infers types of all the params
    /// - prepares function definition
    pub fn mid_analyze_native_fun(&mut self, p: Publicity, f: &NativeFun) {
        // Preparing function definition
        self.resolver.enter_generics(f.generics.clone());
        let def = Function {
            name: f.name.clone(),
            params: f
                .params
                .iter()
                .map(|param| self.infer_type_hint(param.hint.clone()))
                .collect(),
            ret: self.infer_type_hint(f.ret.clone()),
        };
        self.resolver.exit_generics();
        // Declaring function
        self.resolver
            .declare_mod_def(&f.name, (p, Def::Function(self.tx.insert_function(def))));
    }

    /// Performs mid analysis of module
    pub fn mid_phase(&mut self, module: &Module) {
        // Iterating over module items
        for item in &module.items {
            // Matching item kind
            match &item.kind {
                // Processing struct and enum
                ItemKind::Struct(s) => self.mid_analyze_struct(s),
                ItemKind::Enum(e) => self.mid_analyze_enum(e),
                // Processing functions
                ItemKind::Fun(f) => self.mid_analyze_fun(item.publicity, f),
                ItemKind::Native(n) => self.mid_analyze_native_fun(item.publicity, n),
                // Skipping constants for now
                ItemKind::Const(_) => {}
            }
        }
    }
}
