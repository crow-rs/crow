/// Imports
use crate::{
    ctxt::check::InferCtxt,
    def::{Def, Enum, Function, Module, Struct, Variant},
    errors::TypeckError,
    typ::{Meta, Typ},
};
use crow_ast::{
    atom::{BinOp, Lit, Publicity, UnOp},
    expr::{Expr, ExprKind},
};
use crow_lex::token::Span;
use crow_macros::emit;
use id_arena::Id;

/// Implementation of expressions inference
impl<'tx> InferCtxt<'tx> {
    /// Infers literal
    fn infer_lit(&mut self, lit: Lit) -> Typ {
        match lit {
            Lit::Int(_) => Typ::Int,
            Lit::Float(_) => Typ::Float,
            Lit::String(_) => Typ::Str,
            Lit::Bool(_) => Typ::Bool,
            Lit::None => Typ::Unit,
        }
    }

    /// Infers unary expression
    fn infer_unary(&mut self, span: Span, un_op: UnOp, expr: Expr) -> Typ {
        let typ = self.infer_expr(expr);
        match (un_op, typ) {
            // Number neg operator
            (UnOp::Neg, Typ::Int) => Typ::Int,
            (UnOp::Neg, Typ::Float) => Typ::Float,
            // Bool bang operator
            (UnOp::Bang, Typ::Bool) => Typ::Bool,
            (op, t) => {
                emit!(
                    self,
                    TypeckError::InvalidUnOp {
                        src: span.0,
                        span: span.1.into(),
                        t: self.pretty(&t),
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
        lhs: Expr,
        rhs: Expr,
    ) -> Typ {
        let lhs = self.infer_expr(lhs);
        let rhs = self.infer_expr(rhs);

        match (bin_op, lhs, rhs) {
            // Int operators
            (BinOp::Add, Typ::Int, Typ::Int)
            | (BinOp::Sub, Typ::Int, Typ::Int)
            | (BinOp::Mul, Typ::Int, Typ::Int)
            | (BinOp::Div, Typ::Int, Typ::Int)
            | (BinOp::Rem, Typ::Int, Typ::Int)
            | (BinOp::Xor, Typ::Int, Typ::Int) => Typ::Int,
            // Float operators
            (BinOp::Add, Typ::Float, Typ::Float)
            | (BinOp::Sub, Typ::Float, Typ::Float)
            | (BinOp::Mul, Typ::Float, Typ::Float)
            | (BinOp::Div, Typ::Float, Typ::Float)
            | (BinOp::Rem, Typ::Float, Typ::Float)
            | (BinOp::Xor, Typ::Float, Typ::Float) => Typ::Int,
            // Logical operators
            (BinOp::And, Typ::Bool, Typ::Bool)
            | (BinOp::Or, Typ::Bool, Typ::Bool)
            | (BinOp::BitAnd, Typ::Bool, Typ::Bool)
            | (BinOp::BitOr, Typ::Bool, Typ::Bool)
            | (BinOp::Xor, Typ::Bool, Typ::Bool) => Typ::Bool,
            // Comparison operators
            (BinOp::Gt, Typ::Int, Typ::Int)
            | (BinOp::Gt, Typ::Int, Typ::Float)
            | (BinOp::Ge, Typ::Int, Typ::Int)
            | (BinOp::Ge, Typ::Int, Typ::Float)
            | (BinOp::Lt, Typ::Int, Typ::Int)
            | (BinOp::Lt, Typ::Int, Typ::Float)
            | (BinOp::Le, Typ::Int, Typ::Int)
            | (BinOp::Le, Typ::Int, Typ::Float) => Typ::Bool,
            // Concat operator
            (BinOp::Concat, Typ::Str, Typ::Str) => Typ::Str,
            // Equality operators
            (BinOp::Eq, a, b) | (BinOp::Ne, a, b) => {
                self.eq(&span, a, b);
                Typ::Bool
            }
            // Other
            (op, a, b) => {
                emit!(
                    self,
                    TypeckError::InvalidBinOp {
                        src: span.0,
                        span: span.1.into(),
                        a: self.pretty(&a),
                        b: self.pretty(&b),
                        op
                    }
                );
                Typ::Error
            }
        }
    }

    /// Infers assign expression
    fn infer_assign(&mut self, span: Span, what: Expr, to: Expr) -> Typ {
        let what = self.infer_expr(what);
        let to = self.infer_expr(to);

        self.eq(&span, what, to.clone());
        to
    }

    /// Infers if expression
    fn infer_if(
        &mut self,
        cond: Expr,
        then: Expr,
        else_: Option<Box<Expr>>,
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
                (else_.span.clone(), self.infer_expr(*else_));
            self.eq(&else_span, then_typ.clone(), else_typ.clone());
        }

        then_typ
    }

    /// Infers variable expression
    fn infer_var(&mut self, span: Span, name: String) -> Typ {
        // Local variable
        match self.resolver.resolve_local_def(&name) {
            Some(typ) => typ,
            // Module definition
            None => match self.resolver.resolve_mod_def(&name) {
                Some(Def::Const(t)) => t,
                Some(Def::Enum(e)) => Typ::Meta(Meta::Enum(e)),
                Some(Def::Struct(s)) => Typ::Meta(Meta::Struct(s)),
                Some(Def::Function(id)) => {
                    let fun = self.tx.get_function(id);
                    Typ::Fun(
                        id,
                        self.fresh_generic_args(fun.generics.len()),
                    )
                }
                Some(Def::Variant(id, idx)) => {
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
                // Module definition
                None => match self.resolver.resolve_mod(&name) {
                    Some(m) => Typ::Meta(Meta::Module(m)),
                    None => {
                        emit!(
                            self,
                            TypeckError::UndefinedName {
                                src: span.0,
                                span: span.1.into(),
                                name
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
        name: String,
    ) -> Typ {
        match self
            .tx
            .get_struct(id)
            .fields
            .iter()
            .find(|f| f.name == name)
            .clone()
        {
            Some(field) => self.subst(field.typ.clone(), &args),
            None => {
                emit!(
                    self,
                    TypeckError::UndefinedField {
                        src: span.0,
                        span: span.1.into(),
                        name
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
        name: String,
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
                        name
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
        name: String,
    ) -> Typ {
        match self.tx.get_mod(id).defs.get(&name).cloned() {
            Some((p, def)) => {
                let typ = match def {
                    Def::Struct(id) => Typ::Meta(Meta::Struct(id)),
                    Def::Enum(id) => Typ::Meta(Meta::Enum(id)),
                    Def::Function(id) => {
                        let fun = self.tx.get_function(id);
                        Typ::Fun(
                            id,
                            self.fresh_generic_args(fun.generics.len()),
                        )
                    }
                    Def::Const(typ) => typ.clone(),
                    Def::Variant(id, idx) => {
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
                                name,
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
                        name
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
        container: Expr,
        name: String,
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
                        name
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
    ) -> Typ {
        // Getting function
        let fun = self.tx.get_function(id);

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
                    src: span.0,
                    span: span.1.into(),
                    expected: params.len(),
                    got: args.len()
                }
            );
        }

        ret
    }

    /// Infers function reference call
    fn infer_fun_ref_call(
        &mut self,
        span: Span,
        ret: Typ,
        params: Vec<Typ>,
        args: Vec<Typ>,
    ) -> Typ {
        // Checking arity
        if params.len() == args.len() {
            // Checking params and args types equality
            params
                .iter()
                .zip(args)
                .for_each(|(p, a)| self.eq(&span, p.clone(), a));
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
        callee: Expr,
        args: Vec<Expr>,
    ) -> Typ {
        // Inferring callee and args
        let callee = self.infer_expr(callee);
        let args = args
            .into_iter()
            .map(|a| self.infer_expr(a))
            .collect::<Vec<Typ>>();

        // Matching callee
        match callee {
            // Function call
            Typ::Fun(id, generics) => {
                self.infer_fun_call(span, id, generics, args)
            }
            // Function reference call
            Typ::FunRef(ret, params) => {
                self.infer_fun_ref_call(span, *ret, params, args)
            }
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
                        typ: self.pretty(&other)
                    }
                );
                Typ::Error
            }
        }
    }

    /// Infers expression
    pub fn infer_expr(&mut self, expr: Expr) -> Typ {
        let span = expr.span;
        let typ = match expr.kind {
            ExprKind::Lit(lit) => self.infer_lit(lit),
            ExprKind::Unary(expr, un_op) => {
                self.infer_unary(span, un_op, *expr)
            }
            ExprKind::Bin(lhs, rhs, bin_op) => {
                self.infer_binary(span, bin_op, *lhs, *rhs)
            }
            ExprKind::Assign(what, to) => {
                self.infer_assign(span, *what, *to)
            }
            ExprKind::If(cond, then, else_) => {
                self.infer_if(*cond, *then, else_)
            }
            ExprKind::Var(name) => self.infer_var(span, name),
            ExprKind::Field(container, name) => {
                self.infer_field(span, *container, name)
            }
            ExprKind::Call(callee, args) => {
                self.infer_call(span, *callee, args)
            }
            ExprKind::Function(params, expr) => todo!(),
            ExprKind::Match(expr, cases) => todo!(),
            ExprKind::Paren(expr) => todo!(),
            ExprKind::Block(stmts) => self.infer_block(stmts),
            ExprKind::Todo(expr) => todo!(),
            ExprKind::Panic(expr) => todo!(),
        };
        self.apply(typ)
    }
}
