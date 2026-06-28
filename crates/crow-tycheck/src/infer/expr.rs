/// Imports
use crate::{
    ctxt::infer::InferCtxt, def::{DefKind, Enum, Function, Module, Struct, Variant}, errors::TypeckError, typ::{EffectRow, IntBitness, Meta, Typ},
};
use crow_ast::{
    atom::{BinOp, Lit, Param, Publicity, UnOp},
    expr::{Case, Expr, ExprKind, Pat, PatKind},
};
use crow_lex::token::Span;
use crow_macros::emit;
use id_arena::Id;

/// Implementation of expressions inference
impl<'tx> InferCtxt<'tx> {
    /// Infers literal
    fn infer_lit(&mut self, lit: &Lit) -> Typ {
        match lit {
            Lit::Int(_) => Typ::InferInt(self.fresh()), // todo - parse prefixes to resolve bitness
            Lit::Float(_) => Typ::Float,
            Lit::String(_) => Typ::Str,
            Lit::Bool(_) => Typ::Bool,
            Lit::None => Typ::Unit,
        }
    }

    /// Infers unary expression
    fn infer_unary(
        &mut self,
        span: Span,
        un_op: UnOp,
        expr: &Expr,
    ) -> Typ {
        let typ = self.infer_expr(expr);
        match (un_op, typ) {
            // Number neg operator
            (UnOp::Neg, Typ::Int(donor)) => Typ::Int(donor),
            (UnOp::Neg, Typ::InferInt(id)) => Typ::InferInt(id),
            (UnOp::Neg, Typ::Float) => Typ::Float,
            // Bool bang operator
            (UnOp::Bang, Typ::Bool) => Typ::Bool,
            (op, t) => {
                emit!(
                    self,
                    TypeckError::InvalidUnOp {
                        src: span.0,
                        span: span.1.into(),
                        t: self.pretty_type(&t),
                        op
                    }
                );
                Typ::Error
            }
        }
    }

    /// Infers binary expression
    fn infer_binary(
        &mut self,
        span: Span,
        bin_op: BinOp,
        lhs: &Expr,
        rhs: &Expr,
    ) -> Typ {
        let lhs = self.infer_expr(lhs);
        let rhs = self.infer_expr(rhs);

        match (bin_op, &lhs, &rhs) {
            (BinOp::Add, Typ::InferInt(a), Typ::InferInt(b))
            | (BinOp::Sub, Typ::InferInt(a), Typ::InferInt(b))
            | (BinOp::Mul, Typ::InferInt(a), Typ::InferInt(b))
            | (BinOp::Div, Typ::InferInt(a), Typ::InferInt(b))
            | (BinOp::Rem, Typ::InferInt(a), Typ::InferInt(b))
            | (BinOp::Xor, Typ::InferInt(a), Typ::InferInt(b)) => {
                self.unify(Typ::InferInt(a.clone()), Typ::InferInt(b.clone())).ok();
                Typ::InferInt(a.clone())
            }

            (BinOp::Add, Typ::Int(a), Typ::Int(b))
            | (BinOp::Sub, Typ::Int(a), Typ::Int(b))
            | (BinOp::Mul, Typ::Int(a), Typ::Int(b))
            | (BinOp::Div, Typ::Int(a), Typ::Int(b))
            | (BinOp::Rem, Typ::Int(a), Typ::Int(b))
            | (BinOp::Xor, Typ::Int(a), Typ::Int(b)) => {
                if a != b {
                    emit!(
                        self,
                        TypeckError::TypesMissmatch { 
                            src: span.0, 
                            span: span.1.into(), 
                            expected: self.pretty_type(&lhs.clone()), 
                            got: self.pretty_type(&rhs)
                        } 
                    );
                }
                Typ::Int(a.clone())
            }

            // Float operators
            (BinOp::Add, Typ::Float, Typ::Float)
            | (BinOp::Sub, Typ::Float, Typ::Float)
            | (BinOp::Mul, Typ::Float, Typ::Float)
            | (BinOp::Div, Typ::Float, Typ::Float)
            | (BinOp::Rem, Typ::Float, Typ::Float)
            | (BinOp::Xor, Typ::Float, Typ::Float) => Typ::Float,
            // Logical operators
            (BinOp::And, Typ::Bool, Typ::Bool)
            | (BinOp::Or, Typ::Bool, Typ::Bool)
            | (BinOp::BitAnd, Typ::Bool, Typ::Bool)
            | (BinOp::BitOr, Typ::Bool, Typ::Bool)
            | (BinOp::Xor, Typ::Bool, Typ::Bool) => Typ::Bool,
            // Comparison operators
            (BinOp::Gt, Typ::Int(_), Typ::Int(_))
            | (BinOp::Gt, Typ::Int(_), Typ::Float)
            | (BinOp::Ge, Typ::Int(_), Typ::Int(_))
            | (BinOp::Ge, Typ::Int(_), Typ::Float)
            | (BinOp::Lt, Typ::Int(_), Typ::Int(_))
            | (BinOp::Lt, Typ::Int(_), Typ::Float)
            | (BinOp::Le, Typ::Int(_), Typ::Int(_))
            | (BinOp::Le, Typ::Int(_), Typ::Float) => Typ::Bool,
            // Concat operator
            (BinOp::Concat, Typ::Str, Typ::Str) => Typ::Str,
            // Equality operators
            (BinOp::Eq, a, b) | (BinOp::Ne, a, b) => {
                self.eq(&span, a.clone(), b.clone());
                Typ::Bool
            }
            // Other
            (op, a, b) => {
                emit!(
                    self,
                    TypeckError::InvalidBinOp {
                        src: span.0,
                        span: span.1.into(),
                        a: self.pretty_type(&a),
                        b: self.pretty_type(&b),
                        op
                    }
                );
                Typ::Error
            }
        }
    }

    /// Infers assign expression
    fn infer_assign(&mut self, span: Span, what: &Expr, to: &Expr) -> Typ {
        let what = self.infer_expr(what);
        let to = self.infer_expr(to);

        self.eq(&span, what, to.clone());
        to
    }

    /// Infers if expression
    fn infer_if(
        &mut self,
        cond: &Expr,
        then: &Expr,
        else_: &Option<Box<Expr>>,
    ) -> Typ {
        // Checking that condition type is bool
        let (cond_span, cond_typ) =
            (cond.span.clone(), self.infer_expr(cond));
        self.eq(&cond_span, cond_typ, Typ::Bool);

        // Inferring then block
        let then_typ = self.infer_expr(then);

        // Inferring else block, is presented
        if let Some(else_) = else_ {
            let (else_span, else_typ) =
                (else_.span.clone(), self.infer_expr(else_));
            self.eq(&else_span, then_typ.clone(), else_typ.clone());
        }

        then_typ
    }

    /// Infers variable expression
    fn infer_var(&mut self, span: Span, name: &str) -> Typ {
        // Local variable
        match self.resolver.resolve_local_def(name) {
            Some(typ) => typ,
            // Module definition
            None => match self.resolver.resolve_mod_def(name) {
                Some(def) => match def.1 {
                    DefKind::Struct(id) => Typ::Meta(Meta::Struct(id)),
                    DefKind::Enum(id) => Typ::Meta(Meta::Enum(id)),
                    DefKind::Effect(id) => Typ::Meta(Meta::Effect(id)),
                    DefKind::Function(id) => {
                        let fun = self.tx.get_function(id);
                        let effects = fun.effects.clone();
                        Typ::Fun(
                            id,
                            self.fresh_generic_args(fun.generics.len()),
                            effects,
                        )
                    }
                    DefKind::Const(typ) => typ,
                    DefKind::Variant(id, idx) => {
                        let en = self.tx.get_enum(id);
                        let variant = en.variants[idx].clone();
                        if variant.fields.is_empty() {
                            Typ::Enum(
                                id,
                                self.fresh_generic_args(en.generics.len()),
                            )
                        } else {
                            Typ::Meta(Meta::Variant(id, idx))
                        }
                    }
                },
                // Module definition
                None => match self.resolver.resolve_mod(name) {
                    Some(m) => Typ::Meta(Meta::Module(m)),
                    None => {
                        emit!(
                            self,
                            TypeckError::UndefinedName {
                                src: span.0,
                                span: span.1.into(),
                                name: name.to_owned()
                            }
                        );
                        Typ::Error
                    }
                },
            },
        }
    }

    /// Infers struct field expression
    fn infer_struct_field(
        &mut self,
        span: Span,
        id: Id<Struct>,
        args: Vec<Typ>,
        name: &str,
    ) -> Typ {
        match self
            .tx
            .get_struct(id)
            .fields
            .iter()
            .find(|f| f.name == name)
        {
            Some(field) => self.subst(field.typ.clone(), &args),
            None => {
                emit!(
                    self,
                    TypeckError::UndefinedField {
                        src: span.0,
                        span: span.1.into(),
                        name: name.to_owned()
                    }
                );
                Typ::Error
            }
        }
    }

    /// Infers enum field expression
    fn infer_meta_enum_field(
        &mut self,
        span: Span,
        id: Id<Enum>,
        name: &str,
    ) -> Typ {
        let en = self.tx.get_enum(id);
        match en.variants.iter().enumerate().find(|(_, v)| v.name == name)
        {
            // Variant with some fields => meta variant type
            Some((idx, variant)) if !variant.fields.is_empty() => {
                Typ::Meta(Meta::Variant(id, idx))
            }
            // Variant without any fields => enum type
            Some((_, _)) => {
                Typ::Enum(id, self.fresh_generic_args(en.generics.len()))
            }
            // Other => undefined field
            None => {
                emit!(
                    self,
                    TypeckError::UndefinedField {
                        src: span.0,
                        span: span.1.into(),
                        name: name.to_owned()
                    }
                );
                Typ::Error
            }
        }
    }

    /// Infers module field expression
    fn infer_module_field(
        &mut self,
        span: Span,
        id: Id<Module>,
        name: &str,
    ) -> Typ {
        match self.tx.get_mod(id).defs.get(name).cloned() {
            Some((p, def)) => {
                let typ = match def {
                    DefKind::Struct(id) => Typ::Meta(Meta::Struct(id)),
                    DefKind::Enum(id) => Typ::Meta(Meta::Enum(id)),
                    DefKind::Effect(id) => Typ::Meta(Meta::Effect(id)),
                    DefKind::Function(id) => {
                        let fun = self.tx.get_function(id);
                        let effects = fun.effects.clone();
                        Typ::Fun(
                            id,
                            self.fresh_generic_args(fun.generics.len()),
                            effects,
                        )
                    }
                    DefKind::Const(typ) => typ.clone(),
                    DefKind::Variant(id, idx) => {
                        Typ::Meta(Meta::Variant(id, idx))
                    }
                };
                match p {
                    Publicity::Pub => typ,
                    Publicity::Priv => {
                        emit!(
                            self,
                            TypeckError::PrivateModField {
                                src: span.0,
                                span: span.1.into(),
                                name: name.to_owned(),
                                module: self.tx.get_mod(id).name.clone()
                            }
                        );
                        typ
                    }
                }
            }
            None => {
                emit!(
                    self,
                    TypeckError::UndefinedField {
                        src: span.0,
                        span: span.1.into(),
                        name: name.to_owned()
                    }
                );
                Typ::Error
            }
        }
    }

    /// Infers field expression
    fn infer_field(
        &mut self,
        span: Span,
        container: &Expr,
        name: &str,
    ) -> Typ {
        // Matching container
        let container = self.infer_expr(container);
        match container {
            // Struct field access
            Typ::Struct(id, args) => {
                self.infer_struct_field(span, id, args, name)
            }
            // Enum variant access
            Typ::Meta(Meta::Enum(id)) => {
                self.infer_meta_enum_field(span, id, name)
            }
            // Module field access
            Typ::Meta(Meta::Module(id)) => {
                self.infer_module_field(span, id, name)
            }
            // Undefined field
            _ => {
                emit!(
                    self,
                    TypeckError::UndefinedField {
                        src: span.0,
                        span: span.1.into(),
                        name: name.to_owned()
                    }
                );
                Typ::Error
            }
        }
    }

    /// Infers function call
    fn infer_fun_call(
        &mut self,
        span: Span,
        id: Id<Function>,
        generics: Vec<Typ>,
        args: Vec<Typ>,
        effects: EffectRow,
    ) -> Typ {
        // Getting function
        let fun = self.tx.get_function(id);
        let fun_effects = fun.effects.clone();

        // Instantiating return type and param types
        let ret = self.subst(fun.ret.clone(), &generics);
        let params = fun
            .params
            .iter()
            .map(|p| self.subst(p.clone(), &generics))
            .collect::<Vec<Typ>>();

        // Checking arity
        if params.len() == args.len() {
            // Checking params and args types equality
            params
                .into_iter()
                .zip(args)
                .for_each(|(p, a)| self.eq(&span, p, a));
        } else {
            emit!(
                self,
                TypeckError::ArityMissmatch {
                    src: span.clone().0,
                    span: span.clone().1.into(),
                    expected: params.len(),
                    got: args.len()
                }
            );
        }

        // Unifying effects
        if let Err(_) =
            self.unify_effects(fun_effects.clone(), effects.clone())
        {
            emit!(
                self,
                TypeckError::EffectsMismatch {
                    src: span.0,
                    span: span.1.into(),
                    expected: self.pretty_effect_row(&effects),
                    got: self.pretty_effect_row(&fun_effects)
                }
            );
        }

        ret
    }

    fn infer_fun_ref_call(
        &mut self,
        span: Span,
        ret: Typ,
        params: Vec<Typ>,
        args: Vec<Typ>,
        callee_effects: EffectRow,
        caller_effects: EffectRow,
    ) -> Typ {
        // Checking arity
        if params.len() == args.len() {
            params
                .iter()
                .zip(args)
                .for_each(|(p, a)| self.eq(&span, p.clone(), a));
        } else {
            emit!(
                self,
                TypeckError::ArityMissmatch {
                    src: span.0.clone(),
                    span: span.1.clone().into(),
                    expected: params.len(),
                    got: args.len()
                }
            );
        }

        // callee's effects must fit into caller's
        if let Err(_) = self
            .unify_effects(callee_effects.clone(), caller_effects.clone())
        {
            emit!(
                self,
                TypeckError::EffectsMismatch {
                    src: span.0,
                    span: span.1.into(),
                    expected: self.pretty_effect_row(&callee_effects),
                    got: self.pretty_effect_row(&caller_effects)
                }
            );
        }

        ret
    }

    /// Infers struct call
    fn infer_struct_call(
        &mut self,
        span: Span,
        id: Id<Struct>,
        args: Vec<Typ>,
    ) -> Typ {
        // Getting struct info
        let (generics_len, fields): (usize, Vec<_>) = {
            let s = self.tx.get_struct(id);
            (s.generics.len(), s.fields.clone())
        };

        // Preparing generic args
        let generic_args = self.fresh_generic_args(generics_len);

        // Instantiating field types
        let params = fields
            .into_iter()
            .map(|f| self.subst(f.typ, &generic_args))
            .collect::<Vec<Typ>>();

        // Checking arity
        if params.len() == args.len() {
            // Checking fields and args types equality
            params
                .into_iter()
                .zip(args)
                .for_each(|(p, a)| self.eq(&span, p, a));
        } else {
            emit!(
                self,
                TypeckError::ArityMissmatch {
                    src: span.0,
                    span: span.1.into(),
                    expected: params.len(),
                    got: args.len()
                }
            );
        }

        // Done!
        Typ::Struct(id, generic_args)
    }

    /// Infers variant call
    fn infer_variant_call(
        &mut self,
        span: Span,
        id: Id<Enum>,
        variant: usize,
        args: Vec<Typ>,
    ) -> Typ {
        // Getting enum and variant info
        let (generics_len, variant): (usize, Variant) = {
            let e = self.tx.get_enum(id);
            (e.generics.len(), e.variants.get(variant).cloned().unwrap())
        };

        // Preparing generic args
        let generic_args = self.fresh_generic_args(generics_len);

        // Instantiating variant fields
        let params = variant
            .fields
            .iter()
            .map(|f| self.subst(f.clone(), &generic_args))
            .collect::<Vec<Typ>>();

        // Checking arity
        if params.len() == args.len() {
            // Checking params and args types equality
            params
                .into_iter()
                .zip(args)
                .for_each(|(f, a)| self.eq(&span, f, a));
        } else {
            emit!(
                self,
                TypeckError::ArityMissmatch {
                    src: span.0,
                    span: span.1.into(),
                    expected: params.len(),
                    got: args.len()
                }
            );
        }

        // Done!
        Typ::Enum(id, generic_args)
    }

    /// Infers call expression
    fn infer_call(
        &mut self,
        span: Span,
        callee: &Expr,
        args: &[Expr],
    ) -> Typ {
        // Inferring callee and args
        let callee = self.infer_expr(callee);
        let args = args
            .iter()
            .map(|a| self.infer_expr(a))
            .collect::<Vec<Typ>>();
        // Matching callee
        match callee {
            // Function call
            Typ::Fun(id, generics, _effects) => self.infer_fun_call(
                span,
                id,
                generics,
                args,
                self.current_effects.clone(),
            ),
            Typ::FunRef(ret, params, fn_effects) => self
                .infer_fun_ref_call(
                    span,
                    *ret,
                    params,
                    args,
                    fn_effects,
                    self.current_effects.clone(),
                ),
            // Struct call
            Typ::Meta(Meta::Struct(id)) => {
                self.infer_struct_call(span, id, args)
            }
            // Enum variant call
            Typ::Meta(Meta::Variant(id, idx)) => {
                self.infer_variant_call(span, id, idx, args)
            }
            // Other
            other => {
                emit!(
                    self,
                    TypeckError::NonCallable {
                        src: span.0,
                        span: span.1.into(),
                        typ: self.pretty_type(&other)
                    }
                );
                Typ::Error
            }
        }
    }

    /// Infers function expression
    fn infer_fun(&mut self, params: &[Param], body: &Expr) -> Typ {
        // Inferring params
        let param_types: Vec<_> = params
            .iter()
            .map(|p| self.infer_type_hint(&p.hint))
            .collect();

        // Entering function scope
        self.resolver.enter_scope();

        // Defining params
        params.iter().zip(param_types.clone()).for_each(|(p, t)| {
            self.resolver.declare_local_def(&p.name, t)
        });

        // Checking body
        let ret = self.infer_expr(body);

        // Exiting function scope
        self.resolver.exit_scope();

        // Done!
        // question - can closure have effects?
        Typ::FunRef(
            Box::new(ret),
            param_types,
            EffectRow {
                known: Vec::new(),
                tail: None,
            },
        )
    }

    /// Infers todo or panic expression
    fn infer_todo_or_panic(
        &mut self,
        span: Span,
        text: &Option<Box<Expr>>,
    ) -> Typ {
        if let Some(text) = text {
            let text = self.infer_expr(text);
            self.eq(&span, text, Typ::Str);
        }

        Typ::Var(self.fresh())
    }

    /// Checks variant pattern
    fn check_variant_pat(
        &mut self,
        span: &Span,
        id: Id<Enum>,
        variant: &Expr,
    ) {
        // Inferring variant
        let variant = self.infer_expr(variant);

        // Checking types equality
        match variant {
            // If variant is an enum variant
            Typ::Meta(Meta::Variant(vid, _)) => {
                // Checking id mismatch
                if vid != id {
                    emit!(
                        self,
                        TypeckError::InvalidPatVariant {
                            src: span.0.clone(),
                            span: span.1.clone().into(),
                            en: self.tx.get_enum(id).name.clone()
                        }
                    )
                }
            }
            // If not
            other => {
                emit!(
                    self,
                    TypeckError::InvalidPat {
                        src: span.0.clone(),
                        span: span.1.clone().into(),
                        t: self.pretty_type(&other)
                    }
                )
            }
        }
    }

    /// Checks unpak pattern
    fn check_unpak_pat(
        &mut self,
        span: &Span,
        id: Id<Enum>,
        args: &[Typ],
        variant: &Expr,
        params: &[Pat],
    ) {
        // Inferring variant
        let variant = self.infer_expr(variant);

        // Checking types equality
        match variant {
            // If variant is an enum variant
            Typ::Meta(Meta::Variant(vid, idx)) => {
                // Getting enum
                let en = self.tx.get_enum(id);

                // Checking ids equality
                if vid == id {
                    // Getting variant
                    let variant = en.variants[idx].clone();

                    // Checking len equality
                    if variant.fields.len() == params.len() {
                        // Checking inner patterns
                        for (typ, pat) in
                            variant.clone().fields.into_iter().zip(params)
                        {
                            self.check_pat(self.subst(typ, args), pat);
                        }
                    } else {
                        emit!(
                            self,
                            TypeckError::ArityMissmatch {
                                src: span.0.clone(),
                                span: span.1.clone().into(),
                                expected: variant.fields.len(),
                                got: params.len()
                            }
                        )
                    }
                } else {
                    emit!(
                        self,
                        TypeckError::InvalidPatVariant {
                            src: span.0.clone(),
                            span: span.1.clone().into(),
                            en: en.name.clone()
                        }
                    )
                }
            }
            // If not
            other => {
                emit!(
                    self,
                    TypeckError::InvalidPat {
                        src: span.0.clone(),
                        span: span.1.clone().into(),
                        t: self.pretty_type(&other)
                    }
                )
            }
        }
    }

    /// Checks match expression pattern
    fn check_pat(&mut self, what: Typ, pat: &Pat) {
        // Matching patterns
        match (what, &pat.kind) {
            // Skipping literals
            (Typ::Bool, PatKind::Lit(Lit::Bool(_)))
            | (Typ::Int(_), PatKind::Lit(Lit::Int(_)))
            | (Typ::Float, PatKind::Lit(Lit::Float(_)))
            | (Typ::Str, PatKind::Lit(Lit::String(_))) => {}
            // Enum patterns
            (Typ::Enum(id, _), PatKind::Variant(variant)) => {
                self.check_variant_pat(&pat.span, id, variant)
            }
            (Typ::Enum(id, args), PatKind::Unpack(variant, params)) => {
                self.check_unpak_pat(
                    &pat.span, id, &args, variant, params,
                );
            }
            // Binding pattern
            (typ, PatKind::BindTo(var)) => {
                self.resolver.declare_local_def(var, typ);
            }
            // Or pattern
            (typ, PatKind::Or(vec)) => {
                for pat in vec {
                    self.check_pat(typ.clone(), pat);
                }
            }
            // Wildcard pattern
            (_, PatKind::Wildcard) => {}
            // Otherwise, raising error
            (typ, _) => emit!(
                self,
                TypeckError::InvalidPat {
                    src: pat.span.0.clone(),
                    span: pat.span.1.clone().into(),
                    t: self.pretty_type(&typ)
                }
            ),
        }
    }

    /// Infers match expression case
    fn infer_case(&mut self, subjects: &[Typ], case: &Case) -> Typ {
        // Checking patterns
        for (what, pat) in subjects.iter().zip(&case.pats) {
            self.check_pat(what.clone(), pat);
        }

        // Inferring body
        self.infer_expr(&case.body)
    }

    /// Infers match expression
    fn infer_match(&mut self, subjects: &[Expr], cases: &[Case]) -> Typ {
        // Inferring matchable subjects
        let values = subjects
            .iter()
            .map(|v| self.infer_expr(v))
            .collect::<Vec<_>>();

        // Performing exhaustiveness check
        // ...

        // Checking case types equality
        let fresh = Typ::Var(self.fresh());
        for case in cases {
            let typ = self.infer_case(&values, case);
            self.eq(&case.span, fresh.clone(), typ);
        }
        fresh
    }

    /// Infers expression
    pub fn infer_expr(&mut self, expr: &Expr) -> Typ {
        let span = expr.span.clone();
        let typ = match &expr.kind {
            ExprKind::Lit(lit) => self.infer_lit(lit),
            ExprKind::Unary(expr, un_op) => {
                self.infer_unary(span, *un_op, expr)
            }
            ExprKind::Bin(lhs, rhs, bin_op) => {
                self.infer_binary(span, *bin_op, lhs, rhs)
            }
            ExprKind::Assign(what, to) => {
                self.infer_assign(span, what, to)
            }
            ExprKind::If(cond, then, else_) => {
                self.infer_if(cond, then, else_)
            }
            ExprKind::Var(name) => self.infer_var(span, name),
            ExprKind::Field(container, name) => {
                self.infer_field(span, container, name)
            }
            ExprKind::Call(callee, args) => {
                self.infer_call(span, callee, args)
            }
            ExprKind::Function(params, ret) => self.infer_fun(params, ret),
            ExprKind::Match(subjects, cases) => {
                self.infer_match(subjects, cases)
            }
            ExprKind::Paren(expr) => self.infer_expr(expr),
            ExprKind::Block(stmts) => self.infer_block(stmts),
            ExprKind::Todo(expr) => self.infer_todo_or_panic(span, expr),
            ExprKind::Panic(expr) => self.infer_todo_or_panic(span, expr),
        };
        self.apply(typ)
    }
}
