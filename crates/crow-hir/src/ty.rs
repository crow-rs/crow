/// Imports
use crow_lex::token::Span;
use crow_resolving::table::Res;

/// Defines hir type
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct HirTy {
    pub span: Span,
    pub kind: HirTyKind,
}

/// Defines hir type kind
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum HirTyKind {
    /// Resolved type
    Res { res: Res, args: Vec<HirTy> },

    /// Function type
    Fn {
        params: Vec<HirTy>,
        ret: Box<HirTy>,
        effects: HirEffects,
    },

    /// Unit type
    Unit,

    /// Unknown type, should infer
    Infer,
}

/// Hir effects
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct HirEffects {
    pub known: Vec<HirEffectRow>,
    pub tail: Option<char>,
}

/// Hir effects row
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct HirEffectRow {
    pub span: Span,
    pub res: Res,
}
