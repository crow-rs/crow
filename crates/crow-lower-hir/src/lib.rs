/// Imports
use crow_ast::{
    atom::{Effects, Publicity, TypeHint},
    expr::{Case, Expr, ExprKind, Pat, PatKind},
    item::{Item, ItemKind, Module},
    stmt::{Stmt, StmtKind},
};
use crow_common::{bug, span::Span};
use crow_fresh::FreshenVec;
use crow_hir::{
    Hir, body::HirBody, expr::{DivergeKind, HirArm, HirExpr, HirExprKind, HirParam}, id::{BodyId, ExprId, ItemId, PatId, StmtId}, item::{
        HirConstDef, HirEnumDef, HirFieldDef, HirFnDef, HirGenericParam, HirItem, HirItemKind, HirNativeFnDef, HirStructDef, HirVariantDef,
    }, pat::{HirPat, HirPatKind}, stmt::{HirStmt, HirStmtKind}, ty::{HirEffectRow, HirEffects, HirTy, HirTyKind},
};
use crow_resolving::table::{DefId, LocalId, Res, ResolveTable};

/// Defines lowering context,
/// AST → HIR
pub struct LoweringCtxt {
    /// Resolve context
    resolve: ResolveTable,

    /// Current expr, stmts, pats, arms, items, bodies
    exprs: FreshenVec<u32, HirExpr>,
    stmts: FreshenVec<u32, HirStmt>,
    pats: FreshenVec<u32, HirPat>,
    items: FreshenVec<u32, HirItem>,
    bodies: FreshenVec<u32, HirBody>,
}

/// Implementation of lowering
impl LoweringCtxt {
    /// Creates new lowering context
    pub fn new(resolve: ResolveTable) -> Self {
        Self {
            resolve,
            exprs: FreshenVec::new(),
            stmts: FreshenVec::new(),
            pats: FreshenVec::new(),
            items: FreshenVec::new(),
            bodies: FreshenVec::new(),
        }
    }

    /// Lowers body
    fn lower_body(&mut self, expr: &Expr) -> BodyId {
        // Taking expressions, statements and patterns
        let prev_exprs = self.exprs.take();
        let prev_stmts = self.stmts.take();
        let prev_pats = self.pats.take();

        // Lowering root expr
        let root_expr = self.lower_expr(expr);

        // Preparing body
        let body = HirBody {
            id: BodyId(self.bodies.next_id()),
            root_expr,
            exprs: self.exprs.replace(prev_exprs),
            stmts: self.stmts.replace(prev_stmts),
            pats: self.pats.replace(prev_pats),
        };

        // Allocating body
        BodyId(self.bodies.alloc(body))
    }

    /// Allocates expression
    fn alloc_expr(&mut self, span: Span, kind: HirExprKind) -> ExprId {
        ExprId(self.exprs.alloc(HirExpr {
            id: ExprId(self.exprs.next_id()),
            span,
            kind,
        }))
    }

    /// Loweres expression
    fn lower_expr(&mut self, expr: &Expr) -> ExprId {
        // Getting expression span
        let span = expr.span.clone();

        // Lowering expresion
        match &expr.kind {
            ExprKind::Lit(lit) => {
                self.alloc_expr(span, HirExprKind::Lit(lit.clone()))
            }
            ExprKind::Var(_) => {
                let res = self
                    .resolve
                    .resolutions
                    .get(&expr.span)
                    .cloned()
                    .unwrap_or(Res::Err);
                self.alloc_expr(span, HirExprKind::Var(res))
            }
            ExprKind::Unary(inner, op) => {
                let inner_id = self.lower_expr(inner);
                self.alloc_expr(span, HirExprKind::Unary(inner_id, *op))
            }
            ExprKind::Bin(lhs, rhs, op) => {
                let lhs_id = self.lower_expr(lhs);
                let rhs_id = self.lower_expr(rhs);
                self.alloc_expr(
                    span,
                    HirExprKind::Bin(lhs_id, rhs_id, *op),
                )
            }
            // Desugar: `a += b` → `a = a + b`
            ExprKind::Assign(target, value) => {
                let target_id = self.lower_expr(target);
                let value_id = self.lower_expr(value);
                self.alloc_expr(
                    span,
                    HirExprKind::Assign(target_id, value_id),
                )
            }
            ExprKind::If(cond, then_, else_) => {
                let cond_id = self.lower_expr(cond);
                let then_id = self.lower_expr(then_);
                let else_id = else_.as_ref().map(|e| self.lower_expr(e));
                self.alloc_expr(
                    span,
                    HirExprKind::If(cond_id, then_id, else_id),
                )
            }
            ExprKind::Field(base, name) => {
                let base_id = self.lower_expr(base);
                self.alloc_expr(
                    span,
                    HirExprKind::Field(base_id, name.clone()),
                )
            }
            ExprKind::Call(func, args) => {
                let func_id = self.lower_expr(func);
                let arg_ids: Vec<ExprId> =
                    args.iter().map(|a| self.lower_expr(a)).collect();
                self.alloc_expr(span, HirExprKind::Call(func_id, arg_ids))
            }
            ExprKind::Function(params, body) => {
                let hir_params: Vec<HirParam> =
                    params.iter().map(|p| self.lower_param(p)).collect();
                let body_id = self.lower_expr(body);
                self.alloc_expr(
                    span,
                    HirExprKind::Lambda {
                        params: hir_params,
                        body: body_id,
                    },
                )
            }
            ExprKind::Match(scrutinees, cases) => {
                let subject_ids: Vec<_> = scrutinees
                    .iter()
                    .map(|s| self.lower_expr(s))
                    .collect();
                let arms: Vec<_> =
                    cases.iter().map(|c| self.lower_arm(c)).collect();
                self.alloc_expr(
                    span,
                    HirExprKind::Match {
                        subjects: subject_ids,
                        arms,
                    },
                )
            }
            // Desugar `(a + b)` → `a + b`
            ExprKind::Paren(inner) => self.lower_expr(inner),
            ExprKind::Block(stmts) => {
                let stmt_ids: Vec<StmtId> =
                    stmts.iter().map(|s| self.lower_stmt(s)).collect();
                self.alloc_expr(span, HirExprKind::Block(stmt_ids))
            }
            // Lower `ExprKind::Todo` → `ExprKind::Diverge`
            ExprKind::Todo(msg) => {
                let msg_id = msg.as_ref().map(|e| self.lower_expr(e));
                self.alloc_expr(
                    span,
                    HirExprKind::Diverge(DivergeKind::Todo, msg_id),
                )
            }
            // Lower `ExprKind::Panic` → `ExprKind::Diverge`
            ExprKind::Panic(msg) => {
                let msg_id = msg.as_ref().map(|e| self.lower_expr(e));
                self.alloc_expr(
                    span,
                    HirExprKind::Diverge(DivergeKind::Panic, msg_id),
                )
            }
        }
    }

    /// Allocates statement
    fn alloc_stmt(&mut self, span: Span, kind: HirStmtKind) -> StmtId {
        StmtId(self.stmts.alloc(HirStmt {
            id: StmtId(self.stmts.next_id()),
            span,
            kind,
        }))
    }

    /// Lowers statement
    fn lower_stmt(&mut self, stmt: &Stmt) -> StmtId {
        // Getting statement span
        let span = stmt.span.clone();

        // Lowering statement
        match &stmt.kind {
            // Linking resolution with statement for binding
            StmtKind::Binding(name, hint, mutability, value) => {
                let init_id = self.lower_expr(value);
                let ty = self.lower_type_hint(hint);

                let local_id =
                    match self.resolve.resolutions.get(&stmt.span) {
                        Some(Res::Local(lid)) => *lid,
                        _ => {
                            bug!(format!(
                                "no local found for span `{:?}`",
                                stmt.span
                            ))
                        }
                    };

                self.alloc_stmt(
                    span,
                    HirStmtKind::Binding {
                        local_id,
                        name: name.clone(),
                        ty,
                        init: init_id,
                        mutability: *mutability,
                    },
                )
            }
            // Lowering expression for expr-stmt
            StmtKind::Expr(expr) => {
                let expr_id = self.lower_expr(expr);
                self.alloc_stmt(span, HirStmtKind::Expr(expr_id))
            }

            StmtKind::Wildcard(hint, rhs) => {
                let ty = self.lower_type_hint(hint);
                let init_id = self.lower_expr(rhs);
                self.alloc_stmt(
                    span,
                    HirStmtKind::Wildcard { ty, init: init_id },
                )
            }
        }
    }

    /// Allocates pattern
    fn alloc_pat(&mut self, span: Span, kind: HirPatKind) -> PatId {
        PatId(self.pats.alloc(HirPat {
            id: PatId(self.pats.next_id()),
            span,
            kind,
        }))
    }

    /// Lowers pattern
    fn lower_pat(&mut self, pat: &Pat) -> PatId {
        // Getting pattern span
        let span = pat.span.clone();

        // Lowering pattern
        match &pat.kind {
            PatKind::Lit(lit) => {
                self.alloc_pat(span, HirPatKind::Lit(lit.clone()))
            }
            PatKind::Wildcard => {
                self.alloc_pat(span, HirPatKind::Wildcard)
            }
            PatKind::BindTo(_, name) => {
                let local_id =
                    match self.resolve.resolutions.get(&pat.span) {
                        Some(Res::Local(lid)) => *lid,
                        _ => LocalId(u32::MAX),
                    };
                self.alloc_pat(
                    span,
                    HirPatKind::Bind(local_id, name.clone()),
                )
            }
            PatKind::Variant(expr) => {
                let res = self
                    .resolve
                    .resolutions
                    .get(&expr.span)
                    .cloned()
                    .unwrap_or(Res::Err);
                self.alloc_pat(span, HirPatKind::Variant(res))
            }
            PatKind::Unpack(constructor, sub_pats) => {
                let res = self
                    .resolve
                    .resolutions
                    .get(&constructor.span)
                    .cloned()
                    .unwrap_or(Res::Err);
                let sub_pat_ids: Vec<PatId> =
                    sub_pats.iter().map(|p| self.lower_pat(p)).collect();
                self.alloc_pat(span, HirPatKind::Unpack(res, sub_pat_ids))
            }
            PatKind::Or(alternatives) => {
                let alt_ids: Vec<PatId> = alternatives
                    .iter()
                    .map(|p| self.lower_pat(p))
                    .collect();
                self.alloc_pat(span, HirPatKind::Or(alt_ids))
            }
        }
    }

    /// Lowers match arm
    fn lower_arm(&mut self, case: &Case) -> HirArm {
        let pats: Vec<_> =
            case.pats.iter().map(|p| self.lower_pat(p)).collect();
        let body = self.lower_expr(&case.body);
        HirArm {
            span: case.span.clone(),
            pats,
            body,
        }
    }

    /// Lowers type hint
    fn lower_type_hint(&self, hint: &TypeHint) -> HirTy {
        // Lowering hint
        match hint {
            // Linking type with res
            TypeHint::Local { span, args, .. } => {
                let res = self
                    .resolve
                    .resolutions
                    .get(span)
                    .cloned()
                    .unwrap_or(Res::Err);
                let hir_args: Vec<HirTy> =
                    args.iter().map(|a| self.lower_type_hint(a)).collect();
                HirTy {
                    span: span.clone(),
                    kind: HirTyKind::Res {
                        res,
                        args: hir_args,
                    },
                }
            }
            TypeHint::Mod {
                span,
                module,
                name,
                args,
            } => {
                let res = self
                    .resolve
                    .module_by_name
                    .get(module)
                    .and_then(|mod_def_id| {
                        self.resolve.module_exports.get(mod_def_id)
                    })
                    .and_then(|mod_exports| mod_exports.get(name))
                    .map(|def_id| {
                        Res::Def(
                            self.resolve
                                .def_kinds
                                .get(def_id)
                                .unwrap()
                                .clone(),
                            def_id.clone(),
                        )
                    })
                    .unwrap_or(Res::Err);
                let hir_args: Vec<HirTy> =
                    args.iter().map(|a| self.lower_type_hint(a)).collect();
                HirTy {
                    span: span.clone(),
                    kind: HirTyKind::Res {
                        res,
                        args: hir_args,
                    },
                }
            }
            TypeHint::Fun {
                span,
                params,
                ret,
                effects,
            } => {
                let hir_params: Vec<HirTy> = params
                    .iter()
                    .map(|p| self.lower_type_hint(p))
                    .collect();
                let hir_ret = Box::new(self.lower_type_hint(ret));
                let hir_effects = self.lower_effects(effects);
                HirTy {
                    span: span.clone(),
                    kind: HirTyKind::Fn {
                        params: hir_params,
                        ret: hir_ret,
                        effects: hir_effects,
                    },
                }
            }
            TypeHint::Unit(span) => HirTy {
                span: span.clone(),
                kind: HirTyKind::Unit,
            },
            TypeHint::Infer => HirTy {
                span: Span::zeroed(),
                kind: HirTyKind::Infer,
            },
        }
    }

    /// Lowers effects
    fn lower_effects(&self, effects: &Effects) -> HirEffects {
        HirEffects {
            known: effects
                .known
                .iter()
                .map(|e| HirEffectRow {
                    span: e.span.clone(),
                    res: todo!("not implemented :("),
                })
                .collect(),
            tail: effects.tail,
        }
    }

    /// Lowers params
    fn lower_param(&self, param: &crow_ast::atom::Param) -> HirParam {
        let local_id = match self.resolve.resolutions.get(&param.span) {
            Some(Res::Local(lid)) => *lid,
            _ => LocalId(u32::MAX),
        };
        HirParam {
            span: param.span.clone(),
            local_id,
            name: param.name.clone(),
            ty: self.lower_type_hint(&param.hint),
        }
    }

    /// Allocates item
    fn alloc_item(
        &mut self,
        span: Span,
        def_id: DefId,
        publicity: Publicity,
        kind: HirItemKind,
    ) -> ItemId {
        ItemId(self.items.alloc(HirItem {
            id: ItemId(self.items.next_id()),
            span,
            def_id,
            publicity,
            kind,
        }))
    }

    fn translate_generics(&self, fn_def_id: DefId, params: &[String]) -> Vec<HirGenericParam> {
        params.iter().enumerate().map(|(i, name)| {
            let def_id = self.resolve.type_params.iter()
                .find(|(_, tp)| tp.parent == fn_def_id && tp.name == *name)
                .map(|(did, _)| *did)
                .unwrap_or_else(|| panic!("unresolved type param `{name}`"));

            HirGenericParam {
                def_id,
                name: name.clone(),
                idx: i as u32,
            }
        }).collect()
    }

    /// Lowers item
    fn lower_item(&mut self, item: &Item) -> ItemId {
        // Getting publicity and span
        let publicity = item.publicity;
        let span = item.span.clone();

        // Lowering item
        let (def_id, kind) = match &item.kind {
            ItemKind::Struct(s) => {
                let def_id = self.find_toplevel_def(&s.name);
                let fields: Vec<HirFieldDef> = s
                    .fields
                    .iter()
                    .enumerate()
                    .map(|(i, f)| HirFieldDef {
                        span: f.span.clone(),
                        name: f.name.clone(),
                        index: i as u32,
                        ty: self.lower_type_hint(&f.hint),
                    })
                    .collect();
                (
                    def_id,
                    HirItemKind::Struct(HirStructDef {
                        name: s.name.clone(),
                        fields,
                    }),
                )
            }
            ItemKind::Enum(e) => {
                let def_id = self.find_toplevel_def(&e.name);
                let variants: Vec<HirVariantDef> = e
                    .variants
                    .iter()
                    .enumerate()
                    .map(|(i, v)| {
                        let variant_def_id = self
                            .resolve
                            .variant_by_name
                            .get(&(def_id, v.name.clone()))
                            .copied()
                            .unwrap_or(DefId(u32::MAX));
                        HirVariantDef {
                            span: v.span.clone(),
                            def_id: variant_def_id,
                            name: v.name.clone(),
                            index: i as u32,
                            fields: v
                                .fields
                                .iter()
                                .map(|f| self.lower_type_hint(f))
                                .collect(),
                        }
                    })
                    .collect();
                (
                    def_id,
                    HirItemKind::Enum(HirEnumDef {
                        name: e.name.clone(),
                        variants,
                    }),
                )
            }
            ItemKind::Fun(f) => {
                let def_id = self.find_toplevel_def(&f.name);
                let params: Vec<HirParam> =
                    f.params.iter().map(|p| self.lower_param(p)).collect();
                let ret = self.lower_type_hint(&f.ret);
                let effects = self.lower_effects(&f.effects);
                let body_id = self.lower_body(&f.block);
                let type_params = self.translate_generics(def_id, &f.generics);

                (
                    def_id,
                    HirItemKind::Fun(HirFnDef {
                        name: f.name.clone(),
                        params,
                        effects,
                        type_params,
                        ret,
                        body: body_id,
                    }),
                )
            }
            ItemKind::Native(n) => {
                let def_id = self.find_toplevel_def(&n.name);
                let params: Vec<HirParam> =
                    n.params.iter().map(|p| self.lower_param(p)).collect();
                let ret = self.lower_type_hint(&n.ret);
                (
                    def_id,
                    HirItemKind::Native(HirNativeFnDef {
                        name: n.name.clone(),
                        params,
                        ret,
                        native_body: n.body.clone(),
                    }),
                )
            }
            ItemKind::Const(c) => {
                let def_id = self.find_toplevel_def(&c.name);
                let ty = self.lower_type_hint(&c.hint);
                let body_id = self.lower_body(&c.value);
                (
                    def_id,
                    HirItemKind::Const(HirConstDef {
                        name: c.name.clone(),
                        ty,
                        body: body_id,
                    }),
                )
            }
        };

        self.alloc_item(span, def_id, publicity, kind)
    }

    /// Finds top-level def
    fn find_toplevel_def(&self, name: &str) -> DefId {
        self.resolve
            .def_names
            .iter()
            .find(|(_, n)| n.as_str() == name)
            .map(|(id, _)| *id)
            .unwrap_or(DefId(u32::MAX))
    }

    /// Lowers module
    pub fn lower(mut self, module: &Module) -> Hir {
        // Lowering items
        for item in &module.items {
            let _ = self.lower_item(&item);
        }

        // Done!
        Hir {
            items: self.items,
            bodies: self.bodies,
            resolve: self.resolve,
        }
    }
}

/// Lowers module
pub fn lower_module(module: &Module, resolve: ResolveTable) -> Hir {
    let lcx = LoweringCtxt::new(resolve);
    lcx.lower(module)
}
