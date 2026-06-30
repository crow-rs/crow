// hir/lower.rs — AST → HIR

use crow_ast::{
    atom::{AssignOp, BinOp, Lit, TypeHint, Effects, EffectHint},
    expr::{Expr, ExprKind, Case, Pat, PatKind},
    item::{Item, ItemKind, Module},
    stmt::{Stmt, StmtKind},
};
use crow_hir::{Hir, body::HirBody, expr::{DivergeKind, HirArm, HirExpr, HirExprKind, HirParam}, id::{BodyId, ExprId, ItemId, PatId, StmtId}, item::{HirConstDef, HirEnumDef, HirFieldDef, HirFnDef, HirItem, HirItemKind, HirNativeFnDef, HirStructDef, HirVariantDef}, pat::{HirPat, HirPatKind}, stmt::{HirStmt, HirStmtKind}, ty::{HirEffectRef, HirEffects, HirTy, HirTyKind}};
use crow_lex::token::Span;
use crow_resolving::resolve_ctx::{DefId, LocalId, Res, ResolveCtxt};

pub struct LoweringCtxt {
    resolve: ResolveCtxt,

    next_item_id: u32,
    next_body_id: u32,

    current_exprs: Vec<HirExpr>,
    current_stmts: Vec<HirStmt>,
    current_pats: Vec<HirPat>,

    items: Vec<HirItem>,
    bodies: Vec<HirBody>,
}

impl LoweringCtxt {
    pub fn new(resolve: ResolveCtxt) -> Self {
        Self {
            resolve,
            next_item_id: 0,
            next_body_id: 0,
            current_exprs: Vec::new(),
            current_stmts: Vec::new(),
            current_pats: Vec::new(),
            items: Vec::new(),
            bodies: Vec::new(),
        }
    }


    fn alloc_item_id(&mut self) -> ItemId {
        let id = ItemId(self.next_item_id);
        self.next_item_id += 1;
        id
    }

    fn alloc_body_id(&mut self) -> BodyId {
        let id = BodyId(self.next_body_id);
        self.next_body_id += 1;
        id
    }

    fn alloc_expr(&mut self, span: Span, kind: HirExprKind) -> ExprId {
        let id = ExprId(self.current_exprs.len() as u32);
        self.current_exprs.push(HirExpr { id, span, kind });
        id
    }

    fn alloc_stmt(&mut self, span: Span, kind: HirStmtKind) -> StmtId {
        let id = StmtId(self.current_stmts.len() as u32);
        self.current_stmts.push(HirStmt { id, span, kind });
        id
    }

    fn alloc_pat(&mut self, span: Span, kind: HirPatKind) -> PatId {
        let id = PatId(self.current_pats.len() as u32);
        self.current_pats.push(HirPat { id, span, kind });
        id
    }

    fn lower_body(&mut self, expr: &Expr) -> BodyId {
        let body_id = self.alloc_body_id();

        let prev_exprs = std::mem::take(&mut self.current_exprs);
        let prev_stmts = std::mem::take(&mut self.current_stmts);
        let prev_pats = std::mem::take(&mut self.current_pats);

        let root_expr = self.lower_expr(expr);

        let body = HirBody {
            id: body_id,
            root_expr,
            exprs: std::mem::replace(&mut self.current_exprs, prev_exprs),
            stmts: std::mem::replace(&mut self.current_stmts, prev_stmts),
            pats: std::mem::replace(&mut self.current_pats, prev_pats),
        };
        self.bodies.push(body);
        body_id
    }

    fn lower_expr(&mut self, expr: &Expr) -> ExprId {
        let span = expr.span.clone();

        match &expr.kind {
            ExprKind::Lit(lit) => {
                self.alloc_expr(span, HirExprKind::Lit(lit.clone()))
            }

            ExprKind::Var(name) => {
                let res = self.resolve.resolutions
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
                self.alloc_expr(span, HirExprKind::Bin(lhs_id, rhs_id, *op))
            }

            // Desugar: `a += b` → `a = a + b`
            ExprKind::Assign(target, value) => {
                let target_id = self.lower_expr(target);
                let value_id = self.lower_expr(value);
                self.alloc_expr(span, HirExprKind::Assign(target_id, value_id))
            }

            ExprKind::If(cond, then_, else_) => {
                let cond_id = self.lower_expr(cond);
                let then_id = self.lower_expr(then_);
                let else_id = else_.as_ref().map(|e| self.lower_expr(e));
                self.alloc_expr(span, HirExprKind::If(cond_id, then_id, else_id))
            }

            ExprKind::Field(base, name) => {
                let base_id = self.lower_expr(base);
                self.alloc_expr(span, HirExprKind::Field(base_id, name.clone()))
            }

            ExprKind::Call(func, args) => {
                let func_id = self.lower_expr(func);
                let arg_ids: Vec<ExprId> = args.iter()
                    .map(|a| self.lower_expr(a))
                    .collect();
                self.alloc_expr(span, HirExprKind::Call(func_id, arg_ids))
            }

            ExprKind::Function(params, body) => {
                let hir_params: Vec<HirParam> = params.iter()
                    .map(|p| self.lower_param(p))
                    .collect();
                let body_id = self.lower_expr(body);
                self.alloc_expr(span, HirExprKind::Lambda {
                    params: hir_params,
                    body: body_id,
                })
            }

            ExprKind::Match(scrutinees, cases) => {
                let scrutinee_ids: Vec<ExprId> = scrutinees.iter()
                    .map(|s| self.lower_expr(s))
                    .collect();
                let arms: Vec<HirArm> = cases.iter()
                    .map(|c| self.lower_arm(c))
                    .collect();
                self.alloc_expr(span, HirExprKind::Match {
                    scrutinees: scrutinee_ids,
                    arms,
                })
            }

            // Paren — десугарится, просто пробрасываем inner
            ExprKind::Paren(inner) => {
                self.lower_expr(inner)
            }

            ExprKind::Block(stmts) => {
                let stmt_ids: Vec<StmtId> = stmts.iter()
                    .map(|s| self.lower_stmt(s))
                    .collect();
                self.alloc_expr(span, HirExprKind::Block(stmt_ids))
            }

            ExprKind::Todo(msg) => {
                let msg_id = msg.as_ref().map(|e| self.lower_expr(e));
                self.alloc_expr(span, HirExprKind::Diverge(DivergeKind::Todo, msg_id))
            }

            ExprKind::Panic(msg) => {
                let msg_id = msg.as_ref().map(|e| self.lower_expr(e));
                self.alloc_expr(span, HirExprKind::Diverge(DivergeKind::Panic, msg_id))
            }
        }
    }

    fn lower_stmt(&mut self, stmt: &Stmt) -> StmtId {
        let span = stmt.span.clone();

        match &stmt.kind {
            StmtKind::Let(name, hint, value) => {
                let init_id = self.lower_expr(value);
                let ty = self.lower_type_hint(hint);

                let local_id = match self.resolve.resolutions.get(&stmt.span) {
                    Some(Res::Local(lid)) => *lid,
                    _ => LocalId(u32::MAX), // fallback, не должно случиться
                };

                self.alloc_stmt(span, HirStmtKind::Let {
                    local_id,
                    name: name.clone(),
                    ty,
                    init: init_id,
                })
            }

            StmtKind::Expr(expr) => {
                let expr_id = self.lower_expr(expr);
                self.alloc_stmt(span, HirStmtKind::Expr(expr_id))
            }
        }
    }

    fn lower_pat(&mut self, pat: &Pat) -> PatId {
        let span = pat.span.clone();

        match &pat.kind {
            PatKind::Lit(lit) => {
                self.alloc_pat(span, HirPatKind::Lit(lit.clone()))
            }

            PatKind::Wildcard => {
                self.alloc_pat(span, HirPatKind::Wildcard)
            }

            PatKind::BindTo(name) => {
                let local_id = match self.resolve.resolutions.get(&pat.span) {
                    Some(Res::Local(lid)) => *lid,
                    _ => LocalId(u32::MAX),
                };
                self.alloc_pat(span, HirPatKind::Bind(local_id, name.clone()))
            }

            PatKind::Variant(expr) => {
                let res = self.resolve.resolutions
                    .get(&expr.span)
                    .cloned()
                    .unwrap_or(Res::Err);
                self.alloc_pat(span, HirPatKind::Variant(res))
            }

            PatKind::Unpack(constructor, sub_pats) => {
                let res = self.resolve.resolutions
                    .get(&constructor.span)
                    .cloned()
                    .unwrap_or(Res::Err);
                let sub_pat_ids: Vec<PatId> = sub_pats.iter()
                    .map(|p| self.lower_pat(p))
                    .collect();
                self.alloc_pat(span, HirPatKind::Unpack(res, sub_pat_ids))
            }

            PatKind::Or(alternatives) => {
                let alt_ids: Vec<PatId> = alternatives.iter()
                    .map(|p| self.lower_pat(p))
                    .collect();
                self.alloc_pat(span, HirPatKind::Or(alt_ids))
            }
        }
    }

    fn lower_arm(&mut self, case: &Case) -> HirArm {
        let pat_ids: Vec<PatId> = case.pats.iter()
            .map(|p| self.lower_pat(p))
            .collect();
        let body_id = self.lower_expr(&case.body);
        HirArm {
            span: case.span.clone(),
            pats: pat_ids,
            body: body_id,
        }
    }

    fn lower_type_hint(&self, hint: &TypeHint) -> HirTy {
        match hint {
            TypeHint::Local { span, name, args } => {
                let res = self.resolve.resolutions
                    .get(span)
                    .cloned()
                    .unwrap_or(Res::Err);
                let hir_args: Vec<HirTy> = args.iter()
                    .map(|a| self.lower_type_hint(a))
                    .collect();
                HirTy {
                    span: span.clone(),
                    kind: HirTyKind::Resolved { res, args: hir_args },
                }
            }

            TypeHint::Mod { span, module, name, args } => {
                let hir_args: Vec<HirTy> = args.iter()
                    .map(|a| self.lower_type_hint(a))
                    .collect();
                HirTy {
                    span: span.clone(),
                    kind: HirTyKind::ModPath {
                        module_res: Res::Err, // TODO: resolve module
                        name: name.clone(),
                        args: hir_args,
                    },
                }
            }

            TypeHint::Fun { span, params, ret, effects } => {
                let hir_params: Vec<HirTy> = params.iter()
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

            TypeHint::Unit(span) => {
                HirTy {
                    span: span.clone(),
                    kind: HirTyKind::Unit,
                }
            }

            TypeHint::Infer => {
                HirTy {
                    span: Span::zeroed(), 
                    kind: HirTyKind::Infer,
                }
            }
        }
    }

    fn lower_effects(&self, effects: &Effects) -> HirEffects {
        HirEffects {
            known: effects.known.iter()
                .map(|e| HirEffectRef {
                    span: Span::zeroed(), // TODO: span от EffectHint
                    name: e.name.clone(),
                })
                .collect(),
            tail: effects.tail,
        }
    }

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

    fn lower_item(&mut self, item: &Item) -> HirItem {
        let item_id = self.alloc_item_id();
        let span = item.span.clone();

        let (def_id, kind) = match &item.kind {
            ItemKind::Struct(s) => {
                let def_id = self.find_toplevel_def(&s.name);
                let fields: Vec<HirFieldDef> = s.fields.iter()
                    .enumerate()
                    .map(|(i, f)| HirFieldDef {
                        span: f.span.clone(),
                        name: f.name.clone(),
                        index: i as u32,
                        ty: self.lower_type_hint(&f.hint),
                    })
                    .collect();
                (def_id, HirItemKind::Struct(HirStructDef {
                    name: s.name.clone(),
                    fields,
                }))
            }

            ItemKind::Enum(e) => {
                let def_id = self.find_toplevel_def(&e.name);
                let variants: Vec<HirVariantDef> = e.variants.iter()
                    .enumerate()
                    .map(|(i, v)| {
                        let variant_def_id = self.resolve.variant_by_name
                            .get(&(def_id, v.name.clone()))
                            .copied()
                            .unwrap_or(DefId(u32::MAX));
                        HirVariantDef {
                            span: v.span.clone(),
                            def_id: variant_def_id,
                            name: v.name.clone(),
                            index: i as u32,
                            fields: v.fields.iter()
                                .map(|f| self.lower_type_hint(f))
                                .collect(),
                        }
                    })
                    .collect();
                (def_id, HirItemKind::Enum(HirEnumDef {
                    name: e.name.clone(),
                    variants,
                }))
            }

            ItemKind::Fun(f) => {
                let def_id = self.find_toplevel_def(&f.name);
                let params: Vec<HirParam> = f.params.iter()
                    .map(|p| self.lower_param(p))
                    .collect();
                let ret = self.lower_type_hint(&f.ret);
                let effects = self.lower_effects(&f.effects);
                let body_id = self.lower_body(&f.block);

                (def_id, HirItemKind::Fun(HirFnDef {
                    name: f.name.clone(),
                    params,
                    effects,
                    ret,
                    body: body_id,
                }))
            }

            ItemKind::Native(n) => {
                let def_id = self.find_toplevel_def(&n.name);
                let params: Vec<HirParam> = n.params.iter()
                    .map(|p| self.lower_param(p))
                    .collect();
                let ret = self.lower_type_hint(&n.ret);
                (def_id, HirItemKind::Native(HirNativeFnDef {
                    name: n.name.clone(),
                    params,
                    ret,
                    native_body: n.body.clone(),
                }))
            }

            ItemKind::Const(c) => {
                let def_id = self.find_toplevel_def(&c.name);
                let ty = self.lower_type_hint(&c.hint);
                let body_id = self.lower_body(&c.value);
                (def_id, HirItemKind::Const(HirConstDef {
                    name: c.name.clone(),
                    ty,
                    body: body_id,
                }))
            }
        };

        HirItem {
            id: item_id,
            def_id,
            publicity: item.publicity,
            span,
            kind,
        }
    }

    fn find_toplevel_def(&self, name: &str) -> DefId {
        self.resolve.def_names.iter()
            .find(|(_, n)| n.as_str() == name)
            .map(|(id, _)| *id)
            .unwrap_or(DefId(u32::MAX))
    }

    pub fn lower(mut self, module: &Module) -> Hir {
        let items: Vec<HirItem> = module.items.iter()
            .map(|item| self.lower_item(item))
            .collect();

        Hir {
            items,
            bodies: self.bodies,
            resolve: self.resolve,
        }
    }
}

pub fn lower_module(module: &Module, resolve: ResolveCtxt) -> Hir {
    let lcx = LoweringCtxt::new(resolve);
    lcx.lower(module)
}