/// Implementation
use crate::{
    ctxt::check::InferCtxt,
    def::{self, Def},
    errors::TypeckError,
};
use crow_ast::{
    atom::Publicity,
    item::{Enum, ItemKind, Module, Struct},
};
use crow_lex::token::Span;
use crow_macros::bail;

/// Implementation of early check phase
impl<'tx> InferCtxt<'tx> {
    /// Performs early analysis of struct
    /// - Defines struct with name and generics, ignoring fields
    pub fn early_analyze_struct(
        &mut self,
        span: &Span,
        p: Publicity,
        s: &Struct,
    ) {
        let def = (
            p,
            Def::Struct(self.tx.insert_struct(def::Struct {
                name: s.name.clone(),
                generics: s.generics.clone(),
                fields: Vec::new(),
            })),
        );
        if !self.resolver.declare_mod_def(&s.name, def) {
            bail!(TypeckError::ModDefRedefinition {
                src: span.0.clone(),
                span: span.1.clone().into(),
                name: s.name.to_string()
            })
        };
    }

    /// Performs early analysis of enum
    /// - Defines enum with name and generics, ignoring variants
    pub fn early_analyze_enum(
        &mut self,
        span: &Span,
        p: Publicity,
        e: &Enum,
    ) {
        let def = (
            p,
            Def::Enum(self.tx.insert_enum(def::Enum {
                name: e.name.clone(),
                generics: e.generics.clone(),
                variants: Vec::new(),
            })),
        );
        if !self.resolver.declare_mod_def(&e.name, def) {
            bail!(TypeckError::ModDefRedefinition {
                src: span.0.clone(),
                span: span.1.clone().into(),
                name: e.name.to_string()
            })
        };
    }

    /// Performs early analysis of module
    pub fn early_pass(&mut self, module: &Module) {
        // Iterating over module items
        for item in &module.items {
            // Matching item kind
            match &item.kind {
                // Processing struct and enum
                ItemKind::Struct(s) => self.early_analyze_struct(
                    &item.span,
                    item.publicity,
                    s,
                ),
                ItemKind::Enum(e) => {
                    self.early_analyze_enum(&item.span, item.publicity, e)
                }
                // Skipping functions, natives and consts for early phase
                ItemKind::Fun(_)
                | ItemKind::Native(_)
                | ItemKind::Const(_) => {}
            }
        }
    }
}
