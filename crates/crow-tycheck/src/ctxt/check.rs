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
