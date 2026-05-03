/// Imports
use crate::{ctxt::check::InferCtxt, def::Def};
use crow_ast::{
    atom::Publicity,
    item::{Const, Fun, ItemKind, Module},
};

/// Implementation of middle check phase
impl<'tx> InferCtxt<'tx> {
    /// Performs late analysis of function:
    /// - infers body of the function
    pub fn late_analyze_fun(&mut self, f: &Fun) {
        // Getting function definition
        self.resolver.enter_generics(f.generics.clone());
        match self
            .resolver
            .resolve_mod_def(&f.name)
            .expect("enum should exists after early analysis")
        {
            // Checking function body
            Def::Function(id) => {
                // Getting function info
                let (params, ret) = {
                    let f = self.tx.get_function(id);
                    (f.params.clone(), f.ret.clone())
                };

                // Entering function scope
                self.resolver.enter_scope();

                // Defining params
                f.params.iter().zip(params.clone()).for_each(|(p, t)| {
                    self.resolver.declare_local_def(&p.name, t)
                });

                // Checking body
                let (block_span, block_typ) =
                    (f.block.span.clone(), self.infer_expr(&f.block));
                self.eq(&block_span, block_typ, ret);

                // Exiting function scope
                self.resolver.exit_scope();
            }
            _ => unreachable!(),
        }
        self.resolver.exit_generics();
    }

    /// Performs mid analysis of constant:
    /// - infers expression of the constant body
    pub fn late_analyze_const(&mut self, p: Publicity, c: &Const) {
        // Inferring constant value
        let value = self.infer_expr(&c.value);

        // Declaring constant
        self.resolver
            .declare_mod_def(&c.name, (p, Def::Const(value)));
    }

    /// Performs late analysis of module
    pub fn late_pass(&mut self, module: &Module) {
        // Iterating over module items
        for item in &module.items {
            // Matching item kind
            match &item.kind {
                // Processing functions and constants
                ItemKind::Fun(f) => self.late_analyze_fun(f),
                ItemKind::Const(c) => {
                    self.late_analyze_const(item.publicity, c)
                }
                // Skipping structs, enums and natives, because
                // we finished all the analysis of them in `mid` phase
                ItemKind::Struct(_)
                | ItemKind::Enum(_)
                | ItemKind::Native(_) => {}
            }
        }
    }
}
