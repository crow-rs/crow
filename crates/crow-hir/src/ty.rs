use crow_lex::token::Span;
use crow_resolving::resolve_ctx::Res;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct HirTy {
    pub span: Span,
    pub kind: HirTyKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum HirTyKind {
    Resolved {
        res: Res,
        args: Vec<HirTy>,
    },

    ModPath {
        module_res: Res,
        name: String,
        args: Vec<HirTy>,
    },

    Fn {
        params: Vec<HirTy>,
        ret: Box<HirTy>,
        effects: HirEffects,
    },

    Unit,

    Infer,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct HirEffects {
    pub known: Vec<HirEffectRef>,
    pub tail: Option<char>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct HirEffectRef {
    pub span: Span,
    pub name: String,
}