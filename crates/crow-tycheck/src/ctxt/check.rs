/// Imports
use crate::{ctxt::typ::TypesCtxt, resolve::Resolver};
use crow_ast::item::Module;

/// Module checking context used during module typechecking
pub struct InferCtxt<'tx> {
    /// Types context
    pub(crate) tx: &'tx mut TypesCtxt,

    /// Definitions env
    pub(crate) resolver: Resolver,

    /// Does checker has error?
    pub(crate) has_error: bool,
}

/// Implementation
impl<'tx> InferCtxt<'tx> {
    /// Creates new check context
    pub fn new(tx: &'tx mut TypesCtxt) -> Self {
        Self {
            tx,
            resolver: Resolver::default(),
            has_error: false,
        }
    }

    /// Performs type check
    pub fn solve(&mut self, module: Module) {
        self.early_pass(&module);
        self.mid_pass(&module);
        self.late_pass(module);
    }
}
