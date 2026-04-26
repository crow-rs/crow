/// Imports
use crate::{
    ctxt::check::InferCtxt,
    def::{Def, Enum, Struct},
    errors::TypeckError,
    typ::Typ,
};
use crow_ast::atom::{Publicity, TypeHint};
use crow_lex::token::Span;
use crow_macros::{bail, emit};
use id_arena::Id;

/// Implementation of type-hint inference
impl<'tx> InferCtxt<'tx> {
    /// Checks generics arity
    fn check_generics_arity(
        &self,
        span: &Span,
        expected: usize,
        got: usize,
    ) {
        if expected != got {
            bail!(TypeckError::ArityMissmatch {
                src: span.0.clone(),
                span: span.1.clone().into(),
                expected,
                got
            })
        }
    }

    /// Ensures no generics given
    fn ensure_no_generics<F>(
        &mut self,
        span: &Span,
        got: usize,
        typ: F,
    ) -> Typ
    where
        F: FnOnce() -> Typ,
    {
        self.check_generics_arity(span, 0, got);
        typ()
    }

    /// Makes enum with given args
    fn make_enum(
        &mut self,
        span: &Span,
        e: Id<Enum>,
        args: &[TypeHint],
    ) -> Typ {
        // Checking arity
        let params = self.tx.get_enum(e).generics.clone();
        self.check_generics_arity(span, params.len(), args.len());

        // Making fresh args
        let substs = args
            .into_iter()
            .map(|a| self.infer_type_hint(a))
            .collect::<Vec<Typ>>();

        // Done
        Typ::Enum(e, substs)
    }

    /// Makes struct with given args
    fn make_struct(
        &mut self,
        span: &Span,
        s: Id<Struct>,
        args: &[TypeHint],
    ) -> Typ {
        // Checking arity
        let params = self.tx.get_struct(s).generics.clone();
        self.check_generics_arity(span, params.len(), args.len());

        // Making fresh args
        let substs = args
            .into_iter()
            .map(|a| self.infer_type_hint(a))
            .collect::<Vec<Typ>>();

        // Done
        Typ::Struct(s, substs)
    }

    /// Infers local type hint
    fn infer_local_type_hint(
        &mut self,
        span: &Span,
        name: &str,
        args: &[TypeHint],
    ) -> Typ {
        match name {
            // Primitive types
            "int" => {
                self.ensure_no_generics(&span, args.len(), || Typ::Int)
            }
            "float" => {
                self.ensure_no_generics(&span, args.len(), || Typ::Float)
            }
            "bool" => {
                self.ensure_no_generics(&span, args.len(), || Typ::Bool)
            }
            "str" => {
                self.ensure_no_generics(&span, args.len(), || Typ::Str)
            }

            // User-defined types
            _ => {
                // Trying to resolve generic
                match self.resolver.resolve_generic(&name) {
                    Some(idx) => {
                        self.ensure_no_generics(&span, args.len(), || {
                            Typ::Generic(name.to_owned(), idx)
                        })
                    }
                    // If no generic found, trying to resolve module definition
                    None => match self.resolver.resolve_mod_def(&name) {
                        Some(Def::Enum(e)) => {
                            self.make_enum(span, e, args)
                        }
                        Some(Def::Struct(s)) => {
                            self.make_struct(span, s, args)
                        }
                        _ => bail!(TypeckError::UndefinedType {
                            src: span.0.clone(),
                            span: span.1.clone().into(),
                            name: name.to_owned(),
                        }),
                    },
                }
            }
        }
    }

    /// Infers module type hint
    fn infer_mod_type_hint(
        &mut self,
        span: &Span,
        module: &str,
        name: &str,
        args: &[TypeHint],
    ) -> Typ {
        // Getting module
        let module = match self.resolver.resolve_mod(module) {
            Some(m) => m,
            None => bail!(TypeckError::UndefinedMod {
                src: span.0.clone(),
                span: span.1.clone().into(),
                name: module.to_owned()
            }),
        };

        // Getting definition
        match self.tx.get_mod(module).defs.get(name) {
            // If publicity is private
            Some((Publicity::Priv, Def::Enum(_) | Def::Struct(_))) => {
                emit!(
                    self,
                    TypeckError::PrivateType {
                        src: span.0.clone(),
                        span: span.1.clone().into(),
                        name: name.to_owned(),
                    }
                );
                Typ::Error
            }
            // Otherwise
            Some((_, Def::Enum(e))) => self.make_enum(span, *e, args),
            Some((_, Def::Struct(s))) => self.make_struct(span, *s, args),
            // If type is undefined
            _ => bail!(TypeckError::UndefinedType {
                src: span.0.clone(),
                span: span.1.clone().into(),
                name: name.to_owned(),
            }),
        }
    }

    /// Infers function type hint
    fn infer_fun_type_hint(
        &mut self,
        params: &[TypeHint],
        ret: &TypeHint,
    ) -> Typ {
        let params = params
            .into_iter()
            .map(|p| self.infer_type_hint(p))
            .collect();
        let ret = self.infer_type_hint(ret);

        Typ::FunRef(Box::new(ret), params)
    }

    /// Infers type hint
    pub fn infer_type_hint(&mut self, hint: &TypeHint) -> Typ {
        match hint {
            TypeHint::Local { span, name, args } => {
                self.infer_local_type_hint(span, name, args)
            }
            TypeHint::Mod {
                span,
                module,
                name,
                args,
            } => self.infer_mod_type_hint(span, module, name, args),
            TypeHint::Fun { params, ret, .. } => {
                self.infer_fun_type_hint(params, ret)
            }
            TypeHint::Unit(_) => Typ::Unit,
            TypeHint::Infer => Typ::Var(self.fresh()),
        }
    }
}
