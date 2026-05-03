/// Imports
use crate::{
    ctxt::check::InferCtxt,
    errors::TypeckError,
    typ::{Meta, Typ, Var},
};
use crow_lex::token::Span;
use crow_macros::emit;
use id_arena::Id;

/// Represents unification error
#[derive(Clone)]
pub enum UnifyError {
    /// Types mismatch
    Mismatch,
    /// Occurs check failure
    Occurs,
}

/// Implementation of coercion solving
impl<'tx> InferCtxt<'tx> {
    /// Generates fresh unbound type variable
    pub fn fresh(&mut self) -> Id<Var> {
        self.tx.insert_var(Var::Unbound)
    }

    /// Returns fresh generics vector with given len
    pub fn fresh_generic_args(&mut self, len: usize) -> Vec<Typ> {
        (0..len).map(|_| Typ::Var(self.fresh())).collect()
    }

    /// Substitutes generic parameters with given
    /// concrete type arguments: `Typ::Generic(_, idx)` -> `args.get(idx)`
    pub fn subst(&self, ty: Typ, args: &[Typ]) -> Typ {
        match ty {
            Typ::Generic(_, idx) => args[idx].clone(),
            Typ::Struct(id, inner_args) => Typ::Struct(
                id,
                inner_args
                    .into_iter()
                    .map(|a| self.subst(a, args))
                    .collect(),
            ),
            Typ::Enum(id, inner_args) => Typ::Enum(
                id,
                inner_args
                    .into_iter()
                    .map(|a| self.subst(a, args))
                    .collect(),
            ),
            Typ::Fun(id, inner_args) => Typ::Fun(
                id,
                inner_args
                    .into_iter()
                    .map(|a| self.subst(a, args))
                    .collect(),
            ),
            other => other,
        }
    }

    /// Applies all the substitutions by replacing
    /// type variables with concrete types from `TypCtxt`
    pub fn apply(&mut self, typ: Typ) -> Typ {
        // Helper function for args mapping
        let mut map_args = |args: Vec<Typ>| {
            args.into_iter().map(|a| self.apply(a)).collect()
        };

        // Matching type for application
        match typ {
            Typ::Fun(id, args) => Typ::Fun(id, map_args(args)),
            Typ::Struct(id, args) => Typ::Struct(id, map_args(args)),
            Typ::Enum(id, args) => Typ::Enum(id, map_args(args)),
            Typ::Var(id) => match self.tx.get_var(id) {
                Var::Unbound => Typ::Var(id),
                Var::Bound(typ) => self.apply(typ.clone()),
            },
            other => other,
        }
    }

    /// Binds type variable `id` to `typ` if it is still unbound
    fn bind(&mut self, id: Id<Var>, typ: Typ) {
        let var = self.tx.get_var_mut(id);
        if let Var::Unbound = var {
            *var = Var::Bound(typ);
        }
    }

    /// Performs types equality coercion
    pub fn eq(&mut self, span: &Span, a: Typ, b: Typ) {
        if let Err(err) = self.unify(a.clone(), b.clone()) {
            match err {
                UnifyError::Mismatch => emit!(
                    self,
                    TypeckError::TypesMissmatch {
                        src: span.0.clone(),
                        span: span.1.clone().into(),
                        expected: self.pretty(&a),
                        got: self.pretty(&b),
                    }
                ),
                UnifyError::Occurs => emit!(
                    self,
                    TypeckError::RecursiveType {
                        src: span.0.clone(),
                        span: span.1.clone().into(),
                        t: self.pretty(&a),
                    }
                ),
            }
        }
    }

    /// Performs types unification
    fn unify(&mut self, a: Typ, b: Typ) -> Result<(), UnifyError> {
        match (a, b) {
            // Skipping error types
            (Typ::Error, _) | (_, Typ::Error) => Ok(()),

            // Skipping primitive types
            (Typ::Int, Typ::Int)
            | (Typ::Float, Typ::Float)
            | (Typ::Bool, Typ::Bool)
            | (Typ::Str, Typ::Str)
            | (Typ::Unit, Typ::Unit) => Ok(()),

            // Skipping same generics
            (Typ::Generic(_, idx), Typ::Generic(_, idx2))
                if idx == idx2 =>
            {
                Ok(())
            }

            // Skipping same meta variables
            (Typ::Meta(a), Typ::Meta(b)) if a == b => Ok(()),

            // Unifying adt args
            (Typ::Struct(id1, args1), Typ::Struct(id2, args2))
                if id1 == id2 =>
            {
                for (a, b) in args1.into_iter().zip(args2) {
                    self.unify(a, b)?;
                }
                Ok(())
            }
            (Typ::Enum(id1, args1), Typ::Enum(id2, args2))
                if id1 == id2 =>
            {
                for (a, b) in args1.into_iter().zip(args2) {
                    self.unify(a, b)?;
                }
                Ok(())
            }

            // Unifying functions
            (Typ::Fun(id1, args1), Typ::Fun(id2, args2)) if id1 == id2 => {
                for (a, b) in args1.into_iter().zip(args2) {
                    self.unify(a, b)?;
                }
                Ok(())
            }

            // Unifying function references
            (Typ::FunRef(ret1, params1), Typ::FunRef(ret2, params2)) => {
                self.unify(*ret1, *ret2)?;
                for (a, b) in params1.into_iter().zip(params2) {
                    self.unify(a, b)?;
                }
                Ok(())
            }

            // Unifying function references
            (Typ::Fun(id, args), Typ::FunRef(ret1, params1)) => {
                let fun = self.tx.get_function(id);

                let (ret1, params1) = (
                    self.subst(*ret1, &args),
                    params1
                        .iter()
                        .map(|it| self.subst(it.clone(), &args))
                        .collect::<Vec<Typ>>(),
                );
                let (ret2, params2) =
                    (fun.ret.clone(), fun.params.clone());

                self.unify(ret1, ret2)?;
                for (a, b) in params1.into_iter().zip(params2) {
                    self.unify(a, b)?;
                }

                Ok(())
            }

            // Unifying type variables
            (Typ::Var(a), b) | (b, Typ::Var(a)) => self.unify_var(a, b),

            // Other, raising error
            (_, _) => Err(UnifyError::Mismatch),
        }
    }

    /// Unifies type variables
    fn unify_var(
        &mut self,
        id: Id<Var>,
        ty: Typ,
    ) -> Result<(), UnifyError> {
        match self.tx.get_var(id) {
            // Variable already bound, unifying
            Var::Bound(bound) => self.unify(bound.clone(), ty),

            // Unbound variable
            Var::Unbound => {
                // Performing occurs check: restricts infinite types like `T = Vec<T>`
                if self.occurs(id, &ty) {
                    Err(UnifyError::Occurs)
                } else {
                    self.bind(id, ty);
                    Ok(())
                }
            }
        }
    }

    /// Performs occurs check
    fn occurs(&self, id: Id<Var>, typ: &Typ) -> bool {
        match typ {
            Typ::Var(other) => {
                if *other == id {
                    return true;
                }
                match self.tx.get_var(*other) {
                    Var::Bound(inner) => self.occurs(id, inner),
                    _ => false,
                }
            }
            Typ::Struct(_, args)
            | Typ::Enum(_, args)
            | Typ::Fun(_, args) => args.iter().any(|a| self.occurs(id, a)),
            Typ::FunRef(ret, params) => {
                params.iter().any(|a| self.occurs(id, a))
                    || self.occurs(id, ret)
            }
            _ => false,
        }
    }

    /// Returns human-readable type representation
    pub fn pretty(&self, ty: &Typ) -> String {
        match ty {
            Typ::Int => "int".to_string(),
            Typ::Float => "float".to_string(),
            Typ::Bool => "bool".to_string(),
            Typ::Str => "str".to_string(),
            Typ::Unit => "()".to_string(),
            Typ::Var(_) => "_".to_string(),
            Typ::Generic(name, _) => name.to_string(),
            Typ::Struct(id, args) => {
                let name = self.tx.get_struct(*id).name.clone();
                if args.is_empty() {
                    name
                } else {
                    let args = args
                        .iter()
                        .map(|a| self.pretty(a))
                        .collect::<Vec<_>>()
                        .join(", ");
                    format!("{name}[{args}]")
                }
            }
            Typ::Enum(id, args) => {
                let name = self.tx.get_enum(*id).name.clone();
                if args.is_empty() {
                    name
                } else {
                    let args = args
                        .iter()
                        .map(|a| self.pretty(a))
                        .collect::<Vec<_>>()
                        .join(", ");
                    format!("{name}[{args}]")
                }
            }
            Typ::Fun(id, args) => {
                let def = self.tx.get_function(*id);
                let params = def
                    .params
                    .iter()
                    .map(|p| self.pretty(&self.subst(p.clone(), args)))
                    .collect::<Vec<_>>()
                    .join(", ");
                let ret = self.pretty(&def.ret);
                format!("fun({params}) -> {ret}")
            }
            Typ::FunRef(ret, params) => {
                let params = params
                    .iter()
                    .map(|p| self.pretty(p))
                    .collect::<Vec<_>>()
                    .join(", ");
                let ret = self.pretty(ret);
                format!("fun({params}) -> {ret}")
            }
            Typ::Meta(meta) => match meta {
                Meta::Module(_) => "Meta(Module)".to_string(),
                Meta::Struct(_) => "Meta(Struct)".to_string(),
                Meta::Enum(_) => "Meta(Enum)".to_string(),
                Meta::Variant(_, _) => "Meta(Variant)".to_string(),
            },
            Typ::Error => "Error".to_string(),
        }
    }
}
