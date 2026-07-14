/// Imports
use crate::errors::TyCheckError;
use crate::infer::InferCtxt;
use crow_ast::atom::{BinOp, Lit, UnOp};
use crow_common::{DefId, LocalId};
use crow_common::span::Span;
use crow_hir::Hir;
use crow_hir::body::HirBody;
use crow_hir::expr::HirExprKind;
use crow_hir::id::{ExprId, PatId, StmtId};
use crow_hir::item::{HirConstDef, HirFnDef, HirItemKind};
use crow_hir::pat::HirPatKind;
use crow_hir::stmt::HirStmtKind;
use crow_hir::ty::{HirTy, HirTyKind};
use crow_mod_codec_ty::{ExportedTypeKind, ModuleTop};
use crow_resolving::table::{DefKind, Res};
use crow_types::{IntTy, Ty};
use std::collections::{HashMap, HashSet};

/// Defines typeck result for single body
pub struct TypeckBody {
    pub expr_tys: HashMap<ExprId, Ty>,
    pub local_tys: HashMap<LocalId, Ty>,
    pub pat_tys: HashMap<PatId, Ty>,
    pub expr_substs: HashMap<ExprId, Vec<Ty>>,
}

/// Typeck results
pub struct TypeckOutput {
    pub bodies: Vec<TypeckBody>,
    pub prim_tys: HashMap<DefId, Ty>,
    pub fn_sigs: HashMap<DefId, FnSig>,
    pub constants: HashMap<DefId, Ty>,
    pub field_tys: HashMap<DefId, Vec<(String, Ty)>>,
}

/// Function signature
#[derive(Clone)]
pub struct FnSig {
    pub params: Vec<Ty>,
    pub type_params: Vec<(DefId, u32)>,
    pub ret: Ty,
}

/// Defines type checker
pub struct TypeChecker<'hir> {
    /// Hir refere ce
    hir: &'hir Hir,

    /// Inference context
    icx: InferCtxt,

    // Function signatures
    fn_sigs: HashMap<DefId, FnSig>,

    //rec field types
    field_tys: HashMap<DefId, Vec<(String, Ty)>>,

    // Constant types
    const_tys: HashMap<DefId, Ty>,

    // Local types
    locals: HashMap<LocalId, Ty>,

    // Expression types
    expr_tys: HashMap<ExprId, Ty>,

    // Pattern types
    pat_tys: HashMap<PatId, Ty>,

    // Type check errors
    errors: Vec<TyCheckError>,

    primitive_tys: HashMap<DefId, Ty>,

    expr_substs: HashMap<ExprId, Vec<Ty>>,
}

/// Implementation of the type checker
impl<'hir> TypeChecker<'hir> {
    /// Creates new type checker
    pub fn new(hir: &'hir Hir) -> Self {
        Self {
            hir,
            icx: InferCtxt::new(),
            fn_sigs: HashMap::new(),
            const_tys: HashMap::new(),
            locals: HashMap::new(),
            expr_tys: HashMap::new(),
            pat_tys: HashMap::new(),
            errors: Vec::new(),
            primitive_tys: HashMap::new(),
            expr_substs: HashMap::new(),
            field_tys: HashMap::new()
        }
    }

    /// Performs equality coercion
    fn eq(&mut self, expected: Ty, found: Ty, span: Span) {
        if let Err(kind) = self.icx.unify(expected, found, span) {
            self.errors.push(kind);
        }
    }

    /// Defines local types
    fn define_local_types(&mut self) {
        for item_kind in &self.hir.resolve.def_kinds {
            if matches!(item_kind.1, DefKind::BuiltinType) {
                let item_name =
                    self.hir.resolve.def_names.get(item_kind.0).unwrap();
                let ty = Ty::from_name(item_name);
                self.primitive_tys.insert(item_kind.0.clone(), ty);
            }
        }
    }

    pub fn inject_sigs(&mut self, modules: &[ModuleTop]) {
        let name_to_def: HashMap<&str, DefId> = self.hir.resolve.def_names.iter()
            .map(|(did, name)| (name.as_str(), *did))
            .collect();

        for module in modules {
            for func in &module.functions {
                if let Some(&def_id) = name_to_def.get(func.name.as_str()) {
                    let params: Vec<Ty> = func.params.iter()
                        .map(|(_, ty_str)| Ty::from_name(ty_str))
                        .collect();
                    let ret = Ty::from_name(&func.ret);
                    self.fn_sigs.insert(def_id, FnSig {
                        type_params: vec![],
                        params,
                        ret,
                    });
                }
            }

            for cons in &module.constants {
                if let Some(&def_id) = name_to_def.get(cons.name.as_str()) {
                    let ty = Ty::from_name(&cons.ty);
                    self.const_tys.insert(def_id, ty);
                }
            }

            for ty in &module.types {
                if let Some(&def_id) = name_to_def.get(ty.name.as_str()) {
                    //to vyacheslav - do not refactor, it is mind point for future, when i will add uni type and enum
                    if let ExportedTypeKind::Struct { fields } = &ty.kind {
                        let field_tys: Vec<(String, Ty)> = fields.iter()
                            .map(|(name, ty_str)| (name.clone(), Ty::from_name(ty_str)))
                            .collect();
                        self.field_tys.insert(def_id, field_tys);
                    }
                }
            }
        }
    }

    /// Checks module
    pub fn check_module(&mut self) -> TypeckOutput {
        //transform primitive types to types for ty_sys
        self.define_local_types();

        // Collecting signatures
        self.early_pass();

        // Checking bodies
        let bodies = self.late_pass();

        TypeckOutput {
            bodies,
            prim_tys: self.primitive_tys.clone(), //adt_field_tys: ()
            fn_sigs: self.fn_sigs.clone(),
            field_tys: self.field_tys.clone(),
            constants: self.const_tys.clone()
        }
    }

    /// Substitutes type
    fn subst_ty(&self, ty: &Ty, substs: &[Ty]) -> Ty {
        match ty {
            Ty::Param(_, idx) => substs[*idx as usize].clone(),
            Ty::Adt(did, args) => Ty::Adt(
                *did,
                args.iter().map(|t| self.subst_ty(t, substs)).collect(),
            ),
            Ty::Fn(params, ret) => Ty::Fn(
                params.iter().map(|t| self.subst_ty(t, substs)).collect(),
                Box::new(self.subst_ty(ret, substs)),
            ),
            other => other.clone(),
        }
    }

    /// Instantaiates signature
    fn instantiate_sig(&mut self, sig: &FnSig) -> (Vec<Ty>, Ty, Vec<Ty>) {
        let substs: Vec<Ty> = sig
            .type_params
            .iter()
            .map(|_| self.icx.fresh_var())
            .collect();
        let params = sig
            .params
            .iter()
            .map(|ty| self.subst_ty(ty, &substs))
            .collect();
        let ret = self.subst_ty(&sig.ret, &substs);
        (params, ret, substs)
    }

    /// Early pass: collects all the signatures
    fn early_pass(&mut self) {
        for item in self.hir.items.vec() {
            match &item.kind {
                HirItemKind::Fun(f) => {
                    let type_params: Vec<(DefId, u32)> = f
                        .type_params
                        .iter()
                        .map(|tp| (tp.def_id, tp.idx))
                        .collect();
                    let params: Vec<Ty> = f
                        .params
                        .iter()
                        .map(|p| self.lower_hir_ty(&p.ty))
                        .collect();
                    let mut ret = self.lower_hir_ty(&f.ret);

                    if matches!(ret, Ty::Infer(_)) {
                        ret = Ty::Unit;
                    }

                    self.fn_sigs.insert(
                        item.def_id,
                        FnSig {
                            type_params,
                            params,
                            ret,
                        },
                    );
                }
                HirItemKind::Native(n) => {
                    let params: Vec<Ty> = n
                        .params
                        .iter()
                        .map(|p| self.lower_hir_ty(&p.ty))
                        .collect();
                    let ret = self.lower_hir_ty(&n.ret);
                    self.fn_sigs.insert(
                        item.def_id,
                        FnSig {
                            type_params: vec![],
                            params,
                            ret,
                        },
                    );
                }
                HirItemKind::Const(c) => {
                    let ty = self.lower_hir_ty(&c.ty);
                    self.const_tys.insert(item.def_id, ty);
                }
                HirItemKind::Struct(s) => {
                    let fields: Vec<(String, Ty)> = s.fields.iter()
                        .map(|f| (f.name.clone(), self.lower_hir_ty(&f.ty)))
                        .collect();
                    self.field_tys.insert(item.def_id, fields);
                }
                _ => {}
            }
        }
    }

    /// Late pass: check all the bodies
    fn late_pass(&mut self) -> Vec<TypeckBody> {
        // Preparing results
        let mut results = Vec::new();

        // Checking bodies
        for item in self.hir.items.vec() {
            match &item.kind {
                HirItemKind::Fun(f) => {
                    if let Some(_) = f.body {
                        let result = self.check_fn_body(item.def_id, f);
                        results.push(result);
                    }
                }
                HirItemKind::Const(c) => {
                    let result = self.check_const_body(item.def_id, c);
                    results.push(result);
                }
                _ => {}
            }
        }
        results
    }

    /// Lowers hir type
    fn lower_hir_ty(&mut self, hir_ty: &HirTy) -> Ty {
        match &hir_ty.kind {
            HirTyKind::Res {
                res: Res::Def(DefKind::TypeParam, did),
                ..
            } => {
                let tp = self
                    .hir
                    .resolve
                    .type_params
                    .get(did)
                    .expect("unknown type param");
                Ty::Param(*did, tp.index)
            }
            HirTyKind::Res { res, args } => self.resolve_type(res, args),
            HirTyKind::Fn { params, ret, .. } => {
                let param_tys: Vec<Ty> =
                    params.iter().map(|p| self.lower_hir_ty(p)).collect();
                let ret_ty = self.lower_hir_ty(ret);
                Ty::Fn(param_tys, Box::new(ret_ty))
            }
            HirTyKind::Unit => Ty::Unit,
            HirTyKind::Infer => self.icx.fresh_var(),
        }
    }

    /// Resolves resolution type
    fn resolve_type(&mut self, res: &Res, args: &[HirTy]) -> Ty {
        match res {
            Res::Def(DefKind::BuiltinType, def_id) => {
                self.primitive_tys.get(def_id).unwrap().clone()
            }
            Res::Def(DefKind::Struct | DefKind::Enum, def_id) => {
                let ty_args: Vec<Ty> =
                    args.iter().map(|a| self.lower_hir_ty(a)).collect();
                Ty::Adt(*def_id, ty_args)
            }
            _ => Ty::Error,
        }
    }

    fn check_cast(&mut self, from: &Ty, to: &Ty, span: Span) -> Ty {
        let valid = matches!((from, to),
            (Ty::Int(_), Ty::Int(_)) |
            (Ty::Int(_), Ty::Float(_)) |
            (Ty::Float(_), Ty::Int(_)) |
            (Ty::Float(_), Ty::Float(_)) |
            (Ty::Bool, Ty::Int(_)) |
            (Ty::Int(_), Ty::Bool)
        );

        if !valid {
            self.errors.push(TyCheckError::InvalidCast {
                from: format!("{from}"),
                to: format!("{to}"),
                src: span.0.clone(),
                span: span.1.clone().into(),
            });
            return Ty::Error;
        }

        to.clone()
    }

    /// Checks function body
    fn check_fn_body(
        &mut self,
        def_id: DefId,
        f: &HirFnDef,
    ) -> TypeckBody {
        // Clearing last check data
        self.locals.clear();
        self.expr_tys.clear();
        self.pat_tys.clear();
        self.expr_substs.clear();

        // Getting signature
        let sig = match self.fn_sigs.get(&def_id) {
            Some(s) => s.clone(),
            None => return self.empty_results(),
        };

        // Type-checking params
        for (param, param_ty) in f.params.iter().zip(sig.params.iter()) {
            self.locals.insert(param.local_id, param_ty.clone());
        }

        if f.body.is_some() {
            // Checking body
            let body = self.hir.body(f.body.unwrap());
            let body_ty = self.check_expr(body, body.root_expr);

            // Equality coercion
            self.eq(
                sig.ret.clone(),
                body_ty,
                body.expr(body.root_expr).span.clone(),
            );
        }

        // Finalizing results
        self.finalize_results()
    }

    /// Checks constant body
    fn check_const_body(
        &mut self,
        def_id: DefId,
        c: &HirConstDef,
    ) -> TypeckBody {
        // Clearing last check data
        self.locals.clear();
        self.expr_tys.clear();
        self.pat_tys.clear();

        // Getting const type
        let expected = match self.const_tys.get(&def_id) {
            Some(t) => t.clone(),
            None => return self.empty_results(),
        };

        // Type-checking body
        let body = self.hir.body(c.body);
        let body_ty = self.check_expr(body, body.root_expr);

        // Equality coercion
        self.eq(expected, body_ty, body.expr(body.root_expr).span.clone());

        // Finalizing results
        self.finalize_results()
    }

    /// Finalizes the results by types fallback
    fn finalize_results(&mut self) -> TypeckBody {
        for ty in self.expr_tys.values_mut() {
            *ty = self.icx.fallback(ty.clone());
        }
        for ty in self.locals.values_mut() {
            *ty = self.icx.fallback(ty.clone());
        }
        for ty in self.pat_tys.values_mut() {
            *ty = self.icx.fallback(ty.clone());
        }
        for substs in self.expr_substs.values_mut() {
            for ty in substs.iter_mut() {
                *ty = self.icx.fallback(ty.clone());
            }
        }

        TypeckBody {
            expr_tys: std::mem::take(&mut self.expr_tys),
            local_tys: std::mem::take(&mut self.locals),
            pat_tys: std::mem::take(&mut self.pat_tys),
            expr_substs: std::mem::take(&mut self.expr_substs),
        }
    }

    /// Returns empty results for body
    fn empty_results(&self) -> TypeckBody {
        TypeckBody {
            expr_tys: HashMap::new(),
            local_tys: HashMap::new(),
            pat_tys: HashMap::new(),
            expr_substs: HashMap::new(),
        }
    }

    /// Checks statement
    fn check_stmt(&mut self, body: &HirBody, stmt_id: StmtId) -> Ty {
        let stmt = body.stmt(stmt_id);
        match &stmt.kind {
            HirStmtKind::Binding {
                local_id, ty, init, ..
            } => {
                let hint_ty = self.lower_hir_ty(ty);
                let init_ty = self.check_expr(body, *init);
                self.eq(hint_ty.clone(), init_ty, stmt.span.clone());
                self.locals.insert(*local_id, hint_ty);
                Ty::Unit
            }
            HirStmtKind::Expr(expr_id) => {
                let ty = self.check_expr(body, *expr_id);
                ty
            }
            HirStmtKind::Wildcard { init, .. } => {
                self.check_expr(body, *init);
                // achive - Thank you for actually checking return values! You're a good coder!
                Ty::Unit
            }
        }
    }

    /// Checks record constructor
    fn check_record_ctor(
        &mut self,
        body: &HirBody,
        span: Span,
        res: &Res,
        fields: &Vec<(String, ExprId)>,
    ) -> Ty {
        match res {
            Res::Def(DefKind::Struct, did) => {
                let expected_fields = match self.field_tys.get(did) {
                    Some(f) => f.clone(),
                    None => return Ty::Error,
                };

                for (name, init_id) in fields {
                    let init_ty = self.check_expr(body, *init_id);
                    match expected_fields.iter().find(|(n, _)| n == name) {
                        Some((_, field_ty)) => {
                            self.eq(field_ty.clone(), init_ty, span.clone());
                        }
                        None => {
                            self.errors.push(TyCheckError::NoSuchField {
                                ty: format!("adt#{}", did.0),
                                field: name.clone(),
                                src: span.0.clone(),
                                span: span.1.clone().into(),
                            });
                        }
                    }
                }

                // Missing fields
                let provided: HashSet<&str> = fields.iter()
                    .map(|(name, _)| name.as_str())
                    .collect();
                for (name, _) in &expected_fields {
                    if !provided.contains(name.as_str()) {
                        self.errors.push(TyCheckError::MissingField {
                            ty: format!("adt#{}", did.0),
                            field: name.clone(),
                            src: span.0.clone(),
                            span: span.1.clone().into(),
                        });
                    }
                }

                Ty::Adt(*did, vec![])
            }
            _ => Ty::Error,
        }
    }

    /// Checks expression
    fn check_expr(&mut self, body: &HirBody, expr_id: ExprId) -> Ty {
        // Getting expression info
        let expr = body.expr(expr_id);
        let span = expr.span.clone();

        // Inferring expression type
        let ty = match &expr.kind {
            HirExprKind::Cast { expr, ty } => {
                let expr_ty = self.check_expr(body, *expr);
                let resolverd_ty = self.lower_hir_ty(ty);

                self.check_cast(&expr_ty, &resolverd_ty, span)
            }
            HirExprKind::Rec { res, fields } => {
                self.check_record_ctor(body, span, res, fields)
            }
            HirExprKind::Lit(lit) => self.infer_lit(lit),
            HirExprKind::Var(res) => self.infer_var(res, expr_id),
            HirExprKind::Unary(inner_id, op) => {
                let inner_ty = self.check_expr(body, *inner_id);
                self.check_unary_op(*op, inner_ty, span.clone())
            }
            HirExprKind::Bin(lhs_id, rhs_id, op) => {
                let lhs_ty = self.check_expr(body, *lhs_id);
                let rhs_ty = self.check_expr(body, *rhs_id);
                self.check_bin_op(*op, lhs_ty, rhs_ty, span.clone())
            }
            HirExprKind::Assign(target_id, value_id) => {
                // todo: check mutability
                let target_ty = self.check_expr(body, *target_id);
                let value_ty = self.check_expr(body, *value_id);
                self.eq(target_ty, value_ty, span.clone());
                Ty::Unit
            }
            HirExprKind::If(cond_id, then_id, else_id) => {
                let cond_ty = self.check_expr(body, *cond_id);
                self.eq(Ty::Bool, cond_ty, span.clone());

                let then_ty = self.check_expr(body, *then_id);

                match else_id {
                    Some(else_id) => {
                        let else_ty = self.check_expr(body, *else_id);
                        self.eq(then_ty.clone(), else_ty, span.clone());
                        then_ty
                    }
                    None => {
                        self.eq(Ty::Unit, then_ty, span.clone());
                        Ty::Unit
                    }
                }
            }
            HirExprKind::Field(base_id, field_name) => {
                let base_ty = self.check_expr(body, *base_id);
                self.check_field(base_ty, field_name, span.clone())
            }
            HirExprKind::Call(func_id, arg_ids) => {
                let func_ty = self.check_expr(body, *func_id);
                let arg_tys: Vec<Ty> = arg_ids
                    .iter()
                    .map(|id| self.check_expr(body, *id))
                    .collect();
                self.check_call(func_ty, arg_tys, span.clone())
            }
            HirExprKind::Lambda {
                params,
                body: lambda_body_id,
            } => {
                let param_tys: Vec<Ty> = params
                    .iter()
                    .map(|p| {
                        let ty = self.lower_hir_ty(&p.ty);
                        self.locals.insert(p.local_id, ty.clone());
                        ty
                    })
                    .collect();
                let body_ty = self.check_expr(body, *lambda_body_id);
                Ty::Fn(param_tys, Box::new(body_ty))
            }
            HirExprKind::Match { subjects, arms } => {
                let subject_tys: Vec<Ty> = subjects
                    .iter()
                    .map(|id| self.check_expr(body, *id))
                    .collect();
                let result_ty = self.icx.fresh_var();

                for arm in arms {
                    for (pat_id, scr_ty) in
                        arm.pats.iter().zip(subject_tys.iter())
                    {
                        let pat_ty = self.check_pat(body, *pat_id);
                        self.eq(scr_ty.clone(), pat_ty, span.clone());
                    }

                    let arm_ty = self.check_expr(body, arm.body);
                    self.eq(result_ty.clone(), arm_ty, span.clone());
                }

                result_ty
            }
            HirExprKind::Block(stmt_ids) => {
                let mut last_ty = Ty::Unit;
                for stmt_id in stmt_ids {
                    last_ty = self.check_stmt(body, *stmt_id);
                }
                last_ty
            }
            HirExprKind::Diverge(_, msg_id) => {
                if let Some(msg_id) = msg_id {
                    let msg_ty = self.check_expr(body, *msg_id);
                    self.eq(Ty::String, msg_ty, span.clone());
                }
                Ty::Never
            }
        };

        self.expr_tys.insert(expr_id, ty.clone());
        ty
    }

    /// Infers variant pattern
    fn infer_variant_pat(
        &mut self,
        res: &Res,
        sub_tys: &[Ty],
        span: Span,
    ) -> Ty {
        match res {
            Res::Def(DefKind::Variant { enum_def, index }, _) => {
                let variants =
                    match self.hir.resolve.enum_variants.get(enum_def) {
                        Some(v) => v,
                        None => return Ty::Error,
                    };
                let variant = match variants.get(*index as usize) {
                    Some(v) => v,
                    None => return Ty::Error,
                };

                if sub_tys.len() != variant.arity {
                    self.errors.push(TyCheckError::ArityMissmatch {
                        expected: variant.arity,
                        found: sub_tys.len(),
                        src: span.0.clone(),
                        span: span.1.clone().into(),
                    });
                }

                Ty::Adt(*enum_def, vec![])
            }
            _ => Ty::Error,
        }
    }

    /// Checks pattern
    fn check_pat(&mut self, body: &HirBody, pat_id: PatId) -> Ty {
        let pat = body.pat(pat_id);

        let ty = match &pat.kind {
            HirPatKind::Lit(lit) => self.infer_lit(lit),
            HirPatKind::Wildcard => self.icx.fresh_var(),
            HirPatKind::Bind(local_id, _name) => {
                let ty = self.icx.fresh_var();
                self.locals.insert(*local_id, ty.clone());
                ty
            }
            HirPatKind::Variant(res) => {
                self.infer_variant_pat(res, &[], pat.span.clone())
            }
            HirPatKind::Unpack(res, sub_pats) => {
                let sub_tys: Vec<Ty> = sub_pats
                    .iter()
                    .map(|id| self.check_pat(body, *id))
                    .collect();
                self.infer_variant_pat(res, &sub_tys, pat.span.clone())
            }
            HirPatKind::Or(alternatives) => {
                let ty = self.icx.fresh_var();
                for alt_id in alternatives {
                    let alt_ty = self.check_pat(body, *alt_id);
                    self.eq(ty.clone(), alt_ty, pat.span.clone());
                }
                ty
            }
        };

        self.pat_tys.insert(pat_id, ty.clone());
        ty
    }

    /// Infers literal
    fn infer_lit(&mut self, lit: &Lit) -> Ty {
        match lit {
            Lit::Int(value_str) => {
                let value: i128 = value_str.parse().unwrap_or(0);
                let vid = self.icx.fresh_int_var_with_value(value);
                vid
            }
            Lit::Float(_) => self.icx.fresh_float_var(),
            Lit::String(_) => Ty::String,
            Lit::Bool(_) => Ty::Bool,
            Lit::None => Ty::Unit,
        }
    }

    /// Infers variable
    fn infer_var(&mut self, res: &Res, expr_id: ExprId) -> Ty {
        match res {
            Res::Local(lid) => {
                self.locals.get(lid).cloned().unwrap_or(Ty::Error)
            }
            Res::Def(DefKind::Fun | DefKind::NativeFun, def_id) => {
                match self.fn_sigs.get(def_id).cloned() {
                    Some(sig) => {
                        if sig.type_params.is_empty() {
                            Ty::Fn(sig.params, Box::new(sig.ret))
                        } else {
                            let (params, ret, substs) =
                                self.instantiate_sig(&sig);
                            self.expr_substs.insert(expr_id, substs);
                            Ty::Fn(params, Box::new(ret))
                        }
                    }
                    None => Ty::Error,
                }
            }
            Res::Def(DefKind::Const, def_id) => {
                self.const_tys.get(def_id).cloned().unwrap_or(Ty::Error)
            }
            Res::Def(DefKind::Variant { enum_def, index }, _) => {
                self.infer_variant_constructor(*enum_def, *index)
            }
            _ => Ty::Error,
        }
    }

    /// Infers variant constructor
    fn infer_variant_constructor(
        &mut self,
        enum_def: DefId,
        index: u32,
    ) -> Ty {
        let variants = match self.hir.resolve.enum_variants.get(&enum_def)
        {
            Some(v) => v,
            None => return Ty::Error,
        };
        let variant = match variants.get(index as usize) {
            Some(v) => v,
            None => return Ty::Error,
        };

        if variant.arity == 0 {
            Ty::Adt(enum_def, vec![])
        } else {
            let enum_item = self
                .hir
                .items
                .vec()
                .iter()
                .find(|item| item.def_id == enum_def)
                .and_then(|item| match &item.kind {
                    HirItemKind::Enum(e) => Some(e),
                    _ => None,
                });

            match enum_item {
                Some(enum_def_hir) => {
                    match enum_def_hir.variants.get(index as usize) {
                        Some(hir_variant) => {
                            let field_tys: Vec<Ty> = hir_variant
                                .fields
                                .iter()
                                .map(|f| self.lower_hir_ty(f))
                                .collect();
                            Ty::Fn(
                                field_tys,
                                Box::new(Ty::Adt(enum_def, vec![])),
                            )
                        }
                        None => Ty::Error,
                    }
                }
                None => Ty::Error,
            }
        }
    }

    /// Checks call
    fn check_call(
        &mut self,
        func_ty: Ty,
        arg_tys: Vec<Ty>,
        span: Span,
    ) -> Ty {
        // Applying substitutions
        let func_ty = self.icx.shallow_apply(func_ty);

        // Matching function type
        match func_ty {
            Ty::Fn(param_tys, ret_ty) => {
                if param_tys.len() != arg_tys.len() {
                    self.errors.push(TyCheckError::ArityMissmatch {
                        expected: param_tys.len(),
                        found: arg_tys.len(),
                        src: span.0.clone(),
                        span: span.1.clone().into(),
                    });
                    return Ty::Error;
                }
                for (param_ty, arg_ty) in
                    param_tys.into_iter().zip(arg_tys)
                {
                    self.eq(param_ty, arg_ty, span.clone());
                }
                *ret_ty
            }
            Ty::Infer(_) => {
                let ret_ty = self.icx.fresh_var();
                let expected = Ty::Fn(arg_tys, Box::new(ret_ty.clone()));
                self.eq(expected, func_ty, span.clone());
                ret_ty
            }
            Ty::Error => Ty::Error,
            other => {
                self.errors.push(TyCheckError::TypeMissmatch {
                    expected: format!(
                        "{}",
                        Ty::Fn(vec![], Box::new(Ty::Error))
                    ),
                    got: format!("{}", other),
                    src: span.0.clone(),
                    span: span.1.clone().into(),
                });
                Ty::Error
            }
        }
    }

    /// Resolves field
    fn check_field(&mut self, base_ty: Ty, field_name: &str, span: Span) -> Ty {
        let base_ty = self.icx.shallow_apply(base_ty);

        match &base_ty {
            Ty::Adt(def_id, _) => {
                match self.field_tys.get(def_id) {
                    Some(fields) => {
                        match fields.iter().find(|(n, _)| n == field_name) {
                            Some((_, ty)) => ty.clone(),
                            None => {
                                self.errors.push(TyCheckError::NoSuchField {
                                    ty: format!("{}", base_ty),
                                    field: field_name.to_string(),
                                    src: span.0.clone(),
                                    span: span.1.clone().into(),
                                });
                                Ty::Error
                            }
                        }
                    }
                    None => {
                        self.errors.push(TyCheckError::NotAStruct {
                            ty: format!("{}", base_ty),
                            src: span.0.clone(),
                            span: span.1.clone().into(),
                        });
                        Ty::Error
                    }
                }
            }
            Ty::Error => Ty::Error,
            _ => {
                self.errors.push(TyCheckError::NotAStruct {
                    ty: format!("{}", base_ty),
                    src: span.0.clone(),
                    span: span.1.clone().into(),
                });
                Ty::Error
            }
        }
    }

    /// Checks unary op
    fn check_unary_op(&mut self, op: UnOp, inner: Ty, span: Span) -> Ty {
        match op {
            UnOp::Neg => {
                let resolved = self.icx.shallow_apply(inner.clone());
                match &resolved {
                    Ty::Int(_)
                    | Ty::IntVar(_)
                    | Ty::Float(_)
                    | Ty::FloatVar(_)
                    | Ty::Infer(_) => inner,
                    Ty::Error => Ty::Error,
                    _ => {
                        self.errors.push(TyCheckError::TypeMissmatch {
                            expected: format!("{}", Ty::Int(IntTy::I32)), // "some numeric type"
                            got: format!("{}", resolved),
                            src: span.0.clone(),
                            span: span.1.clone().into(),
                        });
                        Ty::Error
                    }
                }
            }
            UnOp::Bang => {
                self.eq(Ty::Bool, inner, span.clone());
                Ty::Bool
            }
        }
    }

    /// Checks binary op
    fn check_bin_op(
        &mut self,
        op: BinOp,
        lhs: Ty,
        rhs: Ty,
        span: Span,
    ) -> Ty {
        match op {
            BinOp::Add
            | BinOp::Sub
            | BinOp::Mul
            | BinOp::Div
            | BinOp::Rem => {
                self.eq(lhs.clone(), rhs, span.clone());
                lhs
            }

            BinOp::Eq
            | BinOp::Ne
            | BinOp::Gt
            | BinOp::Ge
            | BinOp::Lt
            | BinOp::Le => {
                self.eq(lhs, rhs, span.clone());
                Ty::Bool
            }

            BinOp::And | BinOp::Or => {
                self.eq(Ty::Bool, lhs, span.clone());
                self.eq(Ty::Bool, rhs, span.clone());
                Ty::Bool
            }

            BinOp::Xor | BinOp::BitAnd | BinOp::BitOr => {
                self.eq(lhs.clone(), rhs, span.clone());
                lhs
            }

            BinOp::Concat => {
                self.eq(Ty::String, lhs, span.clone());
                self.eq(Ty::String, rhs, span.clone());
                Ty::String
            }
        }
    }
}

/// Performs module typecheck
pub fn typeck_module(hir: &Hir, deps: &[ModuleTop]) -> (TypeckOutput, Vec<TyCheckError>) {
    let mut checker = TypeChecker::new(hir);
    checker.inject_sigs(deps);
    let result = checker.check_module();
    (result, checker.errors)
}
