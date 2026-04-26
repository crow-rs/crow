use crate::{ctxt::check::InferCtxt, typ::Typ};
use crow_ast::expr::{Case, PatKind};

/// Implementation of exhaustiveness checking
impl<'tx> InferCtxt<'tx> {
    /// Performs match exhaustiveness checking
    pub fn is_exhaustive(values: &[Typ], cases: &[Case]) -> bool {
        todo!()
    }
    
    /// Performs single value exhaustiveness checking 

    /// Checks case contains default pattern
    fn has_default_pat(&mut self, case: &Case) -> bool {
        case.pats
            .iter()
            .any(|p| matches!(&p.kind, PatKind::Wildcard))
    }
}
