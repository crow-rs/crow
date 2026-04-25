use crow_ast::item::Module;

/// Imports
use crate::{ctxt::typ::TypCtxt, resolve::Resolver};

/// Module checking context used during module typechecking
pub struct CheckCtxt<'tx> {
    /// Types context
    pub(crate) tx: &'tx mut TypCtxt,

    /// Definitions env
    pub(crate) resolver: Resolver,

    /// Does checker has error?
    pub(crate) has_error: bool,
}

/// Implementation
impl<'tx> CheckCtxt<'tx> {
    /// Creates new check context
    pub fn new(tx: &'tx mut TypCtxt) -> Self {
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
