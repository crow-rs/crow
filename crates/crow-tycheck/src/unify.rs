/// Imports
use crate::{
    ctxt::infer::InferCtxt,
    errors::TypeckError,
    typ::{Effect, EffectRow, Meta, Typ, Var},
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
            Typ::Fun(id, inner_args, effects) => Typ::Fun(
                id,
                inner_args
                    .into_iter()
                    .map(|a| self.subst(a, args))
                    .collect(),
                effects,
            ),
            other => other,
        }
    }

    /// Applies all the substitutions by replacing
    /// type variables with concrete types from `TypCtxt`
    pub fn apply(&mut self, typ: Typ) -> Typ {
        match typ {
            Typ::Fun(id, args, eff) => {
                let args =
                    args.into_iter().map(|a| self.apply(a)).collect();
                let eff = self.apply_effect_row(eff);
                Typ::Fun(id, args, eff)
            }
            Typ::FunRef(ret, params, eff) => {
                let ret = self.apply(*ret);
                let params =
                    params.into_iter().map(|a| self.apply(a)).collect();
                let eff = self.apply_effect_row(eff);
                Typ::FunRef(Box::new(ret), params, eff)
            }
            Typ::Struct(id, args) => {
                let args =
                    args.into_iter().map(|a| self.apply(a)).collect();
                Typ::Struct(id, args)
            }
            Typ::Enum(id, args) => {
                let args =
                    args.into_iter().map(|a| self.apply(a)).collect();
                Typ::Enum(id, args)
            }
            Typ::Var(id) => match self.tx.get_var(id) {
                Var::Unbound => Typ::Var(id),
                Var::Bound(typ) => self.apply(typ.clone()),
            },
            other => other,
        }
    }

    /// Applies effects row substitutions
    fn apply_effect_row(&mut self, row: EffectRow) -> EffectRow {
        match row.tail {
            Some(var) => match self.tx.get_effect_row(var) {
                Some(bound) => {
                    let bound = bound.clone();
                    let mut known = row.known;
                    known.extend(bound.known);
                    known.sort();
                    known.dedup();
                    self.apply_effect_row(EffectRow {
                        known,
                        tail: bound.tail,
                    })
                }
                None => row,
            },
            None => row,
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
                        expected: self.pretty_type(&a),
                        got: self.pretty_type(&b),
                    }
                ),
                UnifyError::Occurs => emit!(
                    self,
                    TypeckError::RecursiveType {
                        src: span.0.clone(),
                        span: span.1.clone().into(),
                        t: self.pretty_type(&a),
                    }
                ),
            }
        }
    }

    /// Unifies effect rows
    pub fn unify_effects(
        &mut self,
        a: EffectRow,
        b: EffectRow,
    ) -> Result<(), UnifyError> {
        // Resolving effect rows
        let a = self.resolve_effect_row(a);
        let b = self.resolve_effect_row(b);

        // Resolving known effects
        let only_a: Vec<_> = a
            .known
            .iter()
            .filter(|e| !b.known.contains(e))
            .cloned()
            .collect();
        let only_b: Vec<_> = b
            .known
            .iter()
            .filter(|e| !a.known.contains(e))
            .cloned()
            .collect();

        // Unifying tails
        match (a.tail, b.tail) {
            (None, None) => {
                if only_a.is_empty() && only_b.is_empty() {
                    Ok(())
                } else {
                    Err(UnifyError::Mismatch)
                }
            }
            (Some(var), None) => {
                if !only_a.is_empty() {
                    return Err(UnifyError::Mismatch);
                }
                self.bind_effect_row(
                    var,
                    EffectRow {
                        known: only_b,
                        tail: None,
                    },
                );
                Ok(())
            }
            (None, Some(var)) => {
                if !only_b.is_empty() {
                    return Err(UnifyError::Mismatch);
                }
                self.bind_effect_row(
                    var,
                    EffectRow {
                        known: only_a,
                        tail: None,
                    },
                );
                Ok(())
            }
            (Some(var_a), Some(var_b)) => {
                let fresh = self.fresh();
                self.bind_effect_row(
                    var_a,
                    EffectRow {
                        known: only_b,
                        tail: Some(fresh),
                    },
                );
                self.bind_effect_row(
                    var_b,
                    EffectRow {
                        known: only_a,
                        tail: Some(fresh),
                    },
                );
                Ok(())
            }
        }
    }

    fn bind_effect_row(&mut self, id: Id<Var>, row: EffectRow) {
        self.tx.bind_effect_row(id, row);
    }

    fn resolve_effect_row(&self, row: EffectRow) -> EffectRow {
        match row.tail {
            Some(var) => match self.tx.get_var(var) {
                Var::Bound(Typ::Var(next)) => {
                    // chain - follow the variable
                    self.resolve_effect_row(EffectRow {
                        known: row.known,
                        tail: Some(*next),
                    })
                }
                _ => row,
            },
            None => row,
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
            (Typ::Fun(id1, args1, eff1), Typ::Fun(id2, args2, eff2))
                if id1 == id2 =>
            {
                for (a, b) in args1.into_iter().zip(args2) {
                    self.unify(a, b)?;
                }
                self.unify_effects(eff1, eff2)?;
                Ok(())
            }

            // Unifying function references
            (
                Typ::FunRef(ret1, params1, eff1),
                Typ::FunRef(ret2, params2, eff2),
            ) => {
                self.unify(*ret1, *ret2)?;
                for (a, b) in params1.into_iter().zip(params2) {
                    self.unify(a, b)?;
                }
                self.unify_effects(eff1, eff2)?;
                Ok(())
            }

            // Unifying function references
            (
                Typ::Fun(id, args, eff1),
                Typ::FunRef(ret1, params1, eff2),
            ) => {
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
                self.unify_effects(eff1, eff2)?;
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
            Typ::Struct(_, args) | Typ::Enum(_, args) => {
                args.iter().any(|a| self.occurs(id, a))
            }
            Typ::Fun(_, args, eff) => {
                args.iter().any(|a| self.occurs(id, a))
                    || eff.tail.map_or(false, |v| v == id)
            }
            Typ::FunRef(ret, params, eff) => {
                params.iter().any(|a| self.occurs(id, a))
                    || self.occurs(id, ret)
                    || eff.tail.map_or(false, |v| v == id)
            }
            _ => false,
        }
    }

    /// Pretty prints effect
    pub fn pretty_effect(&self, eff: &Effect) -> String {
        match eff {
            Effect::Exn => "Exn",
            Effect::Div => "Div",
            Effect::IO => "IO",
            Effect::Console => "Console",
            Effect::Ndet => "Ndet",
            Effect::Total => "Total",
            Effect::UserDefined(id) => &self.tx.get_eff(*id).name,
        }
        .to_string()
    }

    /// Pretty prints effects row
    pub fn pretty_effect_row(&self, eff: &EffectRow) -> String {
        // Pretty printing buffer
        let mut buffer = String::new();

        // Pretty printing effects
        buffer.push_str(
            &eff.known
                .iter()
                .map(|eff| self.pretty_effect(eff))
                .collect::<Vec<_>>()
                .join(","),
        );

        // If tail presented
        if let Some(_) = eff.tail {
            buffer.push_str(":e");
        }

        // If effects empty
        if buffer.is_empty() {
            "Total".to_string()
        } else {
            format!("<{buffer}>")
        }
    }

    /// Returns human-readable type representation
    pub fn pretty_type(&self, ty: &Typ) -> String {
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
                        .map(|a| self.pretty_type(a))
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
                        .map(|a| self.pretty_type(a))
                        .collect::<Vec<_>>()
                        .join(", ");
                    format!("{name}[{args}]")
                }
            }
            Typ::Fun(id, args, eff) => {
                let def = self.tx.get_function(*id);
                let params = def
                    .params
                    .iter()
                    .map(|p| {
                        self.pretty_type(&self.subst(p.clone(), args))
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                let ret = self.pretty_type(&def.ret);
                let effects = self.pretty_effect_row(&eff);
                format!("fun({params}): {effects} -> {ret}")
            }
            Typ::FunRef(ret, params, eff) => {
                let params = params
                    .iter()
                    .map(|p| self.pretty_type(p))
                    .collect::<Vec<_>>()
                    .join(", ");
                let ret = self.pretty_type(ret);
                let effects = self.pretty_effect_row(&eff);
                format!("fun({params}): {effects} -> {ret}")
            }
            Typ::Meta(meta) => match meta {
                Meta::Module(id) => {
                    format!("Meta(Module({}))", self.tx.get_mod(*id).name)
                }
                Meta::Struct(id) => {
                    format!(
                        "Meta(Struct({}))",
                        self.tx.get_struct(*id).name
                    )
                }
                Meta::Enum(id) => {
                    format!("Meta(Enum({}))", self.tx.get_enum(*id).name)
                }
                Meta::Effect(id) => {
                    format!("Meta(Effect({}))", self.tx.get_eff(*id).name)
                }
                Meta::Variant(_, _) => "Meta(Variant)".to_string(),
            },
            Typ::Error => "Error".to_string(),
        }
    }
}
