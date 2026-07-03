use crow_ast::atom::Mutability;
use crow_common::span::Span;
use crow_hir::{
    Hir,
    body::HirBody,
    expr::{HirArm, HirExpr, HirExprKind},
    id::*,
    item::{HirItem, HirItemKind},
    pat::{HirPat, HirPatKind},
    stmt::{HirStmt, HirStmtKind},
};
use crow_tycheck::typeck::TypeckResults;

pub mod warnings;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LintId {
    UnusedResult,
    UnusedVariable,
    UnusedMut,
    UnreachableCode,
}

pub struct LintCtxt<'hir> {
    pub hir_types: &'hir TypeckResults,
    pub diags: Vec<LinterWarnings>,
    pub body: &'hir HirBody,
}

impl<'hir> LintCtxt<'hir> {
    pub fn warn(&mut self, warn: LinterWarnings) {
        self.diags.push(warn);
    }

    pub fn expr(&self, id: ExprId) -> &'hir HirExpr {
        self.body.expr(id)
    }
    pub fn stmt(&self, id: StmtId) -> &'hir HirStmt {
        self.body.stmt(id)
    }
    pub fn pat(&self, id: PatId) -> &'hir HirPat {
        self.body.pat(id)
    }
}

#[allow(unused_variables)]
pub trait LintPass {
    fn id(&self) -> LintId;

    fn enter_item(&mut self, cx: &mut LintCtxt, item: &HirItem) {}
    fn exit_item(&mut self, cx: &mut LintCtxt, item: &HirItem) {}

    fn enter_body(&mut self, cx: &mut LintCtxt, body: &HirBody) {}
    fn exit_body(&mut self, cx: &mut LintCtxt, body: &HirBody) {}

    fn enter_expr(&mut self, cx: &mut LintCtxt, expr: &HirExpr) {}
    fn exit_expr(&mut self, cx: &mut LintCtxt, expr: &HirExpr) {}

    fn enter_stmt(&mut self, cx: &mut LintCtxt, stmt: &HirStmt) {}
    fn enter_pat(&mut self, cx: &mut LintCtxt, pat: &HirPat) {}
    fn enter_arm(&mut self, cx: &mut LintCtxt, arm: &HirArm) {}
}

pub struct LintDriver {
    passes: Vec<Box<dyn LintPass>>,
    diags: Vec<LinterWarnings>,
}

impl LintDriver {
    pub fn new(passes: Vec<Box<dyn LintPass>>) -> Self {
        Self {
            passes,
            diags: Vec::new(),
        }
    }

    pub fn run(
        mut self,
        hir: &Hir,
        tycx: &TypeckResults,
    ) -> Vec<LinterWarnings> {
        for item in hir.items.vec() {
            self.visit_item(hir, item, tycx);
        }
        self.diags
    }

    fn visit_item(
        &mut self,
        hir: &Hir,
        item: &HirItem,
        tycx: &TypeckResults,
    ) {
        let body_id = match &item.kind {
            HirItemKind::Fun(f) => Some(f.body),
            HirItemKind::Const(c) => Some(c.body),
            HirItemKind::Struct(_)
            | HirItemKind::Enum(_)
            | HirItemKind::Native(_) => None,
        };

        if let Some(bid) = body_id {
            let body = hir.body(bid);
            let mut cx = LintCtxt {
                diags: Vec::new(),
                body,
                hir_types: tycx,
            };

            for p in &mut self.passes {
                p.enter_item(&mut cx, item);
            }
            for p in &mut self.passes {
                p.enter_body(&mut cx, body);
            }

            self.visit_expr(&mut cx, body.root_expr);

            for p in &mut self.passes {
                p.exit_body(&mut cx, body);
            }
            for p in &mut self.passes {
                p.exit_item(&mut cx, item);
            }

            self.diags.extend(cx.diags);
        }
    }

    fn visit_expr(&mut self, cx: &mut LintCtxt, id: ExprId) {
        let expr = cx.body.expr(id);
        for p in &mut self.passes {
            p.enter_expr(cx, expr);
        }

        match &expr.kind {
            HirExprKind::Block(stmt_ids) => {
                for &sid in stmt_ids {
                    self.visit_stmt(cx, sid);
                }
            }
            HirExprKind::If(cond, then, els) => {
                self.visit_expr(cx, *cond);
                self.visit_expr(cx, *then);
                if let Some(e) = els {
                    self.visit_expr(cx, *e);
                }
            }
            HirExprKind::Call(callee, args) => {
                self.visit_expr(cx, *callee);
                for &a in args {
                    self.visit_expr(cx, a);
                }
            }
            HirExprKind::Match { subjects, arms } => {
                for &s in subjects {
                    self.visit_expr(cx, s);
                }
                for arm in arms {
                    for p in &mut self.passes {
                        p.enter_arm(cx, arm);
                    }
                    for &pid in &arm.pats {
                        self.visit_pat(cx, pid);
                    }
                    self.visit_expr(cx, arm.body);
                }
            }
            HirExprKind::Bin(lhs, rhs, _) => {
                self.visit_expr(cx, *lhs);
                self.visit_expr(cx, *rhs);
            }
            HirExprKind::Unary(inner, _) => self.visit_expr(cx, *inner),
            HirExprKind::Assign(lhs, rhs) => {
                self.visit_expr(cx, *lhs);
                self.visit_expr(cx, *rhs);
            }
            HirExprKind::Field(base, _) => self.visit_expr(cx, *base),
            HirExprKind::Lambda { body, .. } => self.visit_expr(cx, *body),
            HirExprKind::Diverge(_, arg) => {
                if let Some(a) = arg {
                    self.visit_expr(cx, *a);
                }
            }
            HirExprKind::Lit(_) | HirExprKind::Var(_) => {}
        }

        let expr = cx.body.expr(id);
        for p in &mut self.passes {
            p.exit_expr(cx, expr);
        }
    }

    fn visit_stmt(&mut self, cx: &mut LintCtxt, id: StmtId) {
        let stmt = cx.body.stmt(id);
        for p in &mut self.passes {
            p.enter_stmt(cx, stmt);
        }

        match &stmt.kind {
            HirStmtKind::Binding { init, .. } => {
                self.visit_expr(cx, *init)
            }
            HirStmtKind::Wildcard { init, .. } => {
                self.visit_expr(cx, *init)
            }
            HirStmtKind::Expr(eid) => self.visit_expr(cx, *eid),
        }
    }

    fn visit_pat(&mut self, cx: &mut LintCtxt, id: PatId) {
        let pat = cx.body.pat(id);
        for p in &mut self.passes {
            p.enter_pat(cx, pat);
        }

        match &pat.kind {
            HirPatKind::Unpack(_, children) => {
                for &c in children {
                    self.visit_pat(cx, c);
                }
            }
            HirPatKind::Or(alts) => {
                for &a in alts {
                    self.visit_pat(cx, a);
                }
            }
            _ => {}
        }
    }
}

pub struct UnusedResultLint;

impl LintPass for UnusedResultLint {
    fn id(&self) -> LintId {
        LintId::UnusedResult
    }

    fn enter_stmt(&mut self, cx: &mut LintCtxt, stmt: &HirStmt) {
        if let HirStmtKind::Expr(eid) = &stmt.kind {
            let expr = cx.expr(*eid);
            if matches!(expr.kind, HirExprKind::Call(id, ..)) {
                //let name = //todo - normal name
                cx.warn(LinterWarnings::UnusedResult {
                    stmt: format!("{:?}", expr),
                    src: stmt.span.0.clone().into(),
                    span: stmt.span.1.clone().into(),
                });
            }
        }
    }
}

use crow_resolving::table::LocalId;
use std::collections::{HashMap, HashSet};

use crate::warnings::LinterWarnings;

pub struct UnusedVariableLint {
    declared: HashMap<LocalId, (String, Span)>,
    used: HashSet<LocalId>,
}

impl UnusedVariableLint {
    pub fn new() -> Self {
        Self {
            declared: HashMap::new(),
            used: HashSet::new(),
        }
    }
}

impl LintPass for UnusedVariableLint {
    fn id(&self) -> LintId {
        LintId::UnusedVariable
    }

    fn enter_body(&mut self, _cx: &mut LintCtxt, _body: &HirBody) {
        self.declared.clear();
        self.used.clear();
    }

    fn enter_item(&mut self, _cx: &mut LintCtxt, item: &HirItem) {
        if let HirItemKind::Fun(f) = &item.kind {
            for param in &f.params {
                self.declared.insert(
                    param.local_id,
                    (param.name.clone(), param.span.clone()),
                );
            }
        }
    }

    fn enter_stmt(&mut self, _cx: &mut LintCtxt, stmt: &HirStmt) {
        if let HirStmtKind::Binding { local_id, name, .. } = &stmt.kind {
            self.declared
                .insert(*local_id, (name.clone(), stmt.span.clone()));
        }
    }

    fn enter_expr(&mut self, _cx: &mut LintCtxt, expr: &HirExpr) {
        if let HirExprKind::Var(crow_resolving::table::Res::Local(lid)) =
            &expr.kind
        {
            self.used.insert(*lid);
        }
    }

    fn exit_body(&mut self, cx: &mut LintCtxt, _body: &HirBody) {
        for (lid, (name, span)) in &self.declared {
            if !self.used.contains(lid) && !name.starts_with('_') {
                cx.warn(LinterWarnings::UnusedVariable {
                    val_name: name.clone(),
                    src: span.0.clone().into(),
                    span: span.1.clone().into(),
                });
            }
        }
    }
}

pub struct UnreachableCodeLint;

impl LintPass for UnreachableCodeLint {
    fn id(&self) -> LintId {
        LintId::UnreachableCode
    }

    fn enter_expr(&mut self, cx: &mut LintCtxt, expr: &HirExpr) {
        if let HirExprKind::Block(stmt_ids) = &expr.kind {
            let mut diverged = false;
            for &sid in stmt_ids {
                let stmt = cx.stmt(sid);
                if diverged {
                    cx.warn(LinterWarnings::UnreachableCode {
                        src: stmt.span.0.clone().into(),
                        span: stmt.span.1.clone().into(),
                    });
                    break;
                }
                if let HirStmtKind::Expr(eid) = &stmt.kind {
                    let e = cx.expr(*eid);
                    if matches!(e.kind, HirExprKind::Diverge(..)) {
                        diverged = true;
                    }
                }
            }
        }
    }
}

pub struct UnusedMutLint {
    mut_locals: HashMap<LocalId, (String, Span)>,
    assigned: HashSet<LocalId>,
}

impl UnusedMutLint {
    pub fn new() -> Self {
        Self {
            mut_locals: HashMap::new(),
            assigned: HashSet::new(),
        }
    }
}

impl LintPass for UnusedMutLint {
    fn id(&self) -> LintId {
        LintId::UnusedMut
    }

    fn enter_body(&mut self, _cx: &mut LintCtxt, _body: &HirBody) {
        self.mut_locals.clear();
        self.assigned.clear();
    }

    fn enter_stmt(&mut self, _cx: &mut LintCtxt, stmt: &HirStmt) {
        if let HirStmtKind::Binding {
            local_id,
            name,
            mutability: Mutability::Mut,
            ..
        } = &stmt.kind
        {
            self.mut_locals
                .insert(*local_id, (name.clone(), stmt.span.clone()));
        }
    }

    fn enter_expr(&mut self, cx: &mut LintCtxt, expr: &HirExpr) {
        if let HirExprKind::Assign(lhs_id, _) = &expr.kind {
            let lhs = cx.expr(*lhs_id);
            if let HirExprKind::Var(crow_resolving::table::Res::Local(
                lid,
            )) = &lhs.kind
            {
                self.assigned.insert(*lid);
            }
        }
    }

    fn exit_body(&mut self, cx: &mut LintCtxt, _body: &HirBody) {
        for (lid, (name, span)) in &self.mut_locals {
            if !self.assigned.contains(lid) {
                cx.warn(LinterWarnings::UnusedMut {
                    var_name: name.clone(),
                    src: span.0.clone().into(),
                    span: span.1.clone().into(),
                });
            }
        }
    }
}

pub fn run_lints(hir: &Hir, tycx: &TypeckResults) -> Vec<LinterWarnings> {
    let passes: Vec<Box<dyn LintPass>> = vec![
        Box::new(UnusedResultLint),
        Box::new(UnusedVariableLint::new()),
        Box::new(UnreachableCodeLint),
        Box::new(UnusedMutLint::new()),
    ];
    LintDriver::new(passes).run(hir, tycx)
}
