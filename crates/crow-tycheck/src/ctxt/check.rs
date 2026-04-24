/// Imports
use crate::{ctxt::typ::TypCtxt, resolve::Resolver};

/// Module checking context used during module typechecking
pub struct CheckCtxt<'tx> {
    /// Types context
    pub(crate) tx: &'tx mut TypCtxt,

    /// Definitions resolver
    pub(crate) resolver: Resolver,
}
