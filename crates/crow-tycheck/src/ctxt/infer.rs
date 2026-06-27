/// Imports
use crate::{ctxt::typ::TypesCtxt, resolve::Resolver, typ::EffectRow};
use crow_ast::item::Module;

/// Defines general inference context
/// used during module type-inference and type-checking
pub struct InferCtxt<'tx> {
    /// Types context
    pub(crate) tx: &'tx mut TypesCtxt,

    /// Definitions resolver
    pub resolver: Resolver,

    /// Does checker has error?
    pub(crate) has_error: bool,

    /// Current effects
    pub(crate) current_effects: EffectRow,
}

/// Implementation
impl<'tx> InferCtxt<'tx> {
    /// Creates new check context
    pub fn new(tx: &'tx mut TypesCtxt) -> Self {
        Self {
            tx,
            resolver: Resolver::default(),
            has_error: false,
            current_effects: EffectRow {
                known: vec![],
                tail: None,
            },
        }
    }

    /// Performs type check
    pub fn solve(&mut self, module: &Module) {
        self.early_pass(module);
        self.mid_pass(module);
        self.late_pass(module);
    }
}
