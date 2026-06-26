use std::collections::HashMap;

use crow_ast::{
    atom::{BinOp, Lit, Publicity},
    expr::{Case, Expr, ExprKind, Pat, PatKind},
    item::{Const, Enum, Fun, ItemKind, Module, NativeFun, Struct},
    stmt::{Stmt, StmtKind},
};
use crow_tycheck::{
    ctxt::typ::TypesCtxt,
    def::{Def, EffectRow},
    hir::{
        HirCase, HirConst, HirEnum, HirExpr, HirExprKind, HirFunction,
        HirLit, HirModule, HirNativeFun, HirPat, HirStmt, HirStruct,
        HirVariant,
    },
    typ::{Typ, Var},
};

pub struct HirBuilder<'tx> {
    tx: &'tx TypesCtxt,
    defs: HashMap<String, Def>,
    scopes: Vec<HashMap<String, Typ>>,
}

impl<'tx> HirBuilder<'tx> {
    pub fn new(tx: &'tx TypesCtxt, defs: HashMap<String, Def>) -> Self {
        Self {
            tx,
            defs,
            scopes: Vec::new(),
        }
    }

    fn enter_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn exit_scope(&mut self) {
        self.scopes.pop();
    }

    fn declare_local(&mut self, name: &str, ty: Typ) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name.to_string(), ty);
        }
    }

    fn resolve_local(&self, name: &str) -> Option<Typ> {
        for scope in self.scopes.iter().rev() {
            if let Some(ty) = scope.get(name) {
                return Some(ty.clone());
            }
        }
        None
    }

    fn resolve_def(&self, name: &str) -> Option<&Def> {
        self.defs.get(name)
    }


    pub fn build(mut self, module: &Module) -> HirModule {
        let mut functions = Vec::new();
        let mut structs = Vec::new();
        let mut enums = Vec::new();
        let mut constants = Vec::new();
        let mut natives = Vec::new();

        for item in &module.items {
            match &item.kind {
                ItemKind::Struct(s) => structs.push(self.lower_struct(s)),
                ItemKind::Enum(e) => enums.push(self.lower_enum(e)),
                ItemKind::Fun(f) => functions.push(self.lower_fun(f)),
                ItemKind::Native(n) => natives.push(self.lower_native(n)),
                ItemKind::Const(c) => {
                    constants.push(self.lower_const(item.publicity, c))
                }
            }
        }

        HirModule {
            functions,
            structs,
            enums,
            constants,
            natives,
        }
    }

    fn lower_struct(&self, s: &Struct) -> HirStruct {
        let def_id = match self.resolve_def(&s.name) {
            Some(Def::Struct(id)) => *id,
            _ => unreachable!("struct should exist after typeck"),
        };
        let def = self.tx.get_struct(def_id);

        HirStruct {
            name: s.name.clone(),
            fields: def
                .fields
                .iter()
                .map(|f| (f.name.clone(), self.apply(f.typ.clone())))
                .collect(),
        }
    }

    fn lower_enum(&self, e: &Enum) -> HirEnum {
        let def_id = match self.resolve_def(&e.name) {
            Some(Def::Enum(id)) => *id,
            _ => unreachable!("enum should exist after typeck"),
        };
        let def = self.tx.get_enum(def_id);

        HirEnum {
            name: e.name.clone(),
            variants: def
                .variants
                .iter()
                .map(|v| HirVariant {
                    name: v.name.clone(),
                    fields: v
                        .fields
                        .iter()
                        .map(|f| self.apply(f.clone()))
                        .collect(),
                })
                .collect(),
        }
    }

    fn lower_fun(&mut self, f: &Fun) -> HirFunction {
        let def_id = match self.resolve_def(&f.name) {
            Some(Def::Function(id)) => *id,
            _ => unreachable!("function should exist after typeck"),
        };

        let fun_def = self.tx.get_function(def_id);
        let ret = self.apply(fun_def.ret.clone());
        let params: Vec<(String, Typ)> = f
            .params
            .iter()
            .zip(fun_def.params.iter())
            .map(|(p, ty)| (p.name.clone(), self.apply(ty.clone())))
            .collect();
        let effects = fun_def.effects.clone();

        // Enter scope, declare params
        self.enter_scope();
        for (name, ty) in &params {
            self.declare_local(name, ty.clone());
        }

        let body = self.lower_expr(&f.block);

        self.exit_scope();

        HirFunction {
            name: f.name.clone(),
            params,
            ret,
            body,
            effects,
        }
    }

    fn lower_native(&self, n: &NativeFun) -> HirNativeFun {
        let def_id = match self.resolve_def(&n.name) {
            Some(Def::Function(id)) => *id,
            _ => unreachable!("native should exist after typeck"),
        };

        let fun_def = self.tx.get_function(def_id);

        HirNativeFun {
            name: n.name.clone(),
            params: n
                .params
                .iter()
                .zip(fun_def.params.iter())
                .map(|(p, ty)| (p.name.clone(), self.apply(ty.clone())))
                .collect(),
            ret: self.apply(fun_def.ret.clone()),
            body: n.body.clone(),
            effects: fun_def.effects.clone(),
        }
    }

    fn lower_const(&mut self, _pub: Publicity, c: &Const) -> HirConst {
        let ty = match self.resolve_def(&c.name) {
            Some(Def::Const(ty)) => self.apply(ty.clone()),
            _ => unreachable!("const should exist after typeck"),
        };

        let value = self.lower_expr(&c.value);

        HirConst {
            name: c.name.clone(),
            ty,
            value,
        }
    }

    fn lower_expr(&mut self, expr: &Expr) -> HirExpr {
        match &expr.kind {
            ExprKind::Lit(lit) => self.lower_lit(lit),
            ExprKind::Var(name) => self.lower_var(name),

            ExprKind::Bin(lhs, rhs, op) => {
                let lhs = self.lower_expr(lhs);
                let rhs = self.lower_expr(rhs);
                let ty = self.binop_result_type(*op, &lhs.ty);
                HirExpr {
                    ty,
                    kind: HirExprKind::BinOp(*op, Box::new(lhs), Box::new(rhs)),
                }
            }

            ExprKind::Unary(operand, op) => {
                let operand = self.lower_expr(operand);
                let ty = operand.ty.clone();
                HirExpr {
                    ty,
                    kind: HirExprKind::UnOp(*op, Box::new(operand)),
                }
            }

            ExprKind::Assign(lhs, rhs) => {
                let lhs = self.lower_expr(lhs);
                let rhs = self.lower_expr(rhs);
                HirExpr {
                    ty: Typ::Unit,
                    kind: HirExprKind::Assign(Box::new(lhs), Box::new(rhs)),
                }
            }

            ExprKind::If(cond, then_br, else_br) => {
                let cond = self.lower_expr(cond);
                let then_br = self.lower_expr(then_br);
                let ty = then_br.ty.clone();
                let else_br =
                    else_br.as_ref().map(|e| Box::new(self.lower_expr(e)));
                HirExpr {
                    ty,
                    kind: HirExprKind::If(
                        Box::new(cond),
                        Box::new(then_br),
                        else_br,
                    ),
                }
            }

            ExprKind::Field(obj, field_name) => {
                let obj = self.lower_expr(obj);
                let (field_idx, field_ty) =
                    self.resolve_field(&obj.ty, field_name);
                HirExpr {
                    ty: field_ty,
                    kind: HirExprKind::Field(
                        Box::new(obj),
                        field_name.clone(),
                        field_idx,
                    ),
                }
            }

            ExprKind::Call(callee, args) => {
                let callee = self.lower_expr(callee);
                let args: Vec<HirExpr> =
                    args.iter().map(|a| self.lower_expr(a)).collect();
                let ret_ty = self.call_return_type(&callee.ty);
                HirExpr {
                    ty: ret_ty,
                    kind: HirExprKind::Call(Box::new(callee), args),
                }
            }

            ExprKind::Function(params, body) => {
                self.enter_scope();

                let typed_params: Vec<(String, Typ)> = params
                    .iter()
                    .map(|p| {
                        let ty = self.lower_type_hint(&p.hint);
                        self.declare_local(&p.name, ty.clone());
                        (p.name.clone(), ty)
                    })
                    .collect();

                let body = self.lower_expr(body);
                let ret_ty = body.ty.clone();

                self.exit_scope();

                let fn_ty = Typ::FunRef(
                    Box::new(ret_ty),
                    typed_params.iter().map(|(_, t)| t.clone()).collect(),
                    EffectRow {
                        known: vec![],
                        tail: None,
                    },
                );

                HirExpr {
                    ty: fn_ty,
                    kind: HirExprKind::Lambda(typed_params, Box::new(body)),
                }
            }

            ExprKind::Match(scrutinees, cases) => {
                let scrutinees: Vec<HirExpr> =
                    scrutinees.iter().map(|s| self.lower_expr(s)).collect();
                let cases: Vec<HirCase> = cases
                    .iter()
                    .map(|c| self.lower_case(c, &scrutinees))
                    .collect();

                let ty = cases
                    .first()
                    .map(|c| c.body.ty.clone())
                    .unwrap_or(Typ::Unit);

                HirExpr {
                    ty,
                    kind: HirExprKind::Match(scrutinees, cases),
                }
            }

            ExprKind::Paren(inner) => self.lower_expr(inner),

            ExprKind::Block(stmts) => self.lower_block(stmts),

            ExprKind::Todo(msg) => {
                let msg =
                    msg.as_ref().map(|e| Box::new(self.lower_expr(e)));
                HirExpr {
                    ty: Typ::Error,
                    kind: HirExprKind::Todo(msg),
                }
            }

            ExprKind::Panic(msg) => {
                let msg =
                    msg.as_ref().map(|e| Box::new(self.lower_expr(e)));
                HirExpr {
                    ty: Typ::Error,
                    kind: HirExprKind::Panic(msg),
                }
            }
        }
    }

    fn lower_lit(&self, lit: &Lit) -> HirExpr {
        match lit {
            Lit::Int(s) => HirExpr {
                ty: Typ::Int,
                kind: HirExprKind::Lit(HirLit::Int(
                    s.parse::<i64>().unwrap_or(0),
                )),
            },
            Lit::Float(s) => HirExpr {
                ty: Typ::Float,
                kind: HirExprKind::Lit(HirLit::Float(
                    s.parse::<f64>().unwrap_or(0.0),
                )),
            },
            Lit::String(s) => HirExpr {
                ty: Typ::Str,
                kind: HirExprKind::Lit(HirLit::Str(s.clone())),
            },
            Lit::Bool(s) => HirExpr {
                ty: Typ::Bool,
                kind: HirExprKind::Lit(HirLit::Bool(s == "true")),
            },
            Lit::None => HirExpr {
                ty: Typ::Unit,
                kind: HirExprKind::Lit(HirLit::Unit),
            },
        }
    }

    fn lower_var(&self, name: &str) -> HirExpr {
        // Try local first
        if let Some(ty) = self.resolve_local(name) {
            return HirExpr {
                ty: self.apply(ty),
                kind: HirExprKind::Var(name.to_string()),
            };
        }

        // Try top-level def
        match self.resolve_def(name) {
            Some(Def::Function(id)) => {
                let fun = self.tx.get_function(*id);
                let ty = Typ::Fun(*id, vec![], fun.effects.clone());
                HirExpr {
                    ty: self.apply(ty),
                    kind: HirExprKind::Var(name.to_string()),
                }
            }
            Some(Def::Const(ty)) => HirExpr {
                ty: self.apply(ty.clone()),
                kind: HirExprKind::Var(name.to_string()),
            },
            Some(Def::Variant(enum_id, _idx)) => {
                let ty = Typ::Enum(*enum_id, vec![]);
                HirExpr {
                    ty: self.apply(ty),
                    kind: HirExprKind::Var(name.to_string()),
                }
            }
            _ => HirExpr {
                ty: Typ::Error,
                kind: HirExprKind::Var(name.to_string()),
            },
        }
    }

    fn lower_block(&mut self, stmts: &[Stmt]) -> HirExpr {
        self.enter_scope();

        let mut hir_stmts: Vec<HirStmt> = Vec::new();
        let mut last_ty = Typ::Unit;

        for stmt in stmts {
            match &stmt.kind {
                StmtKind::Let(name, _hint, init) => {
                    let init_expr = self.lower_expr(init);
                    let ty = init_expr.ty.clone();
                    self.declare_local(name, ty.clone());
                    hir_stmts.push(HirStmt::Let(
                        name.clone(),
                        ty,
                        init_expr,
                    ));
                    last_ty = Typ::Unit;
                }
                StmtKind::Expr(expr) => {
                    let hir_expr = self.lower_expr(expr);
                    last_ty = hir_expr.ty.clone();
                    hir_stmts.push(HirStmt::Expr(hir_expr));
                }
            }
        }

        self.exit_scope();

        HirExpr {
            ty: last_ty,
            kind: HirExprKind::Block(hir_stmts),
        }
    }

    fn lower_case(
        &mut self,
        case: &Case,
        scrutinees: &[HirExpr],
    ) -> HirCase {
        self.enter_scope();

        let scrut_ty = scrutinees
            .first()
            .map(|s| &s.ty)
            .unwrap_or(&Typ::Error);

        let pats: Vec<HirPat> = case
            .pats
            .iter()
            .map(|p| self.lower_pat(p, scrut_ty))
            .collect();

        let body = self.lower_expr(&case.body);

        self.exit_scope();

        HirCase { pats, body }
    }

    fn lower_pat(&mut self, pat: &Pat, scrut_ty: &Typ) -> HirPat {
        match &pat.kind {
            PatKind::Lit(lit) => HirPat::Lit(self.lower_lit_value(lit)),

            PatKind::Variant(expr) => {
                if let ExprKind::Var(name) = &expr.kind {
                    if let Some(Def::Variant(enum_id, idx)) =
                        self.resolve_def(name)
                    {
                        return HirPat::Variant(*enum_id, *idx);
                    }
                }
                HirPat::Wildcard
            }

            PatKind::Unpack(expr, sub_pats) => {
                if let ExprKind::Var(name) = &expr.kind {
                    if let Some(&Def::Variant(enum_id, idx)) =
                        self.resolve_def(name)
                    {
                        let variant_fields = self
                            .tx
                            .get_enum(enum_id)
                            .variants[idx]
                            .fields
                            .clone();

                        let sub: Vec<HirPat> = sub_pats
                            .iter()
                            .enumerate()
                            .map(|(i, p)| {
                                let field_ty = variant_fields
                                    .get(i)
                                    .cloned()
                                    .unwrap_or(Typ::Error);
                                self.lower_pat(p, &field_ty)
                            })
                            .collect();
                        return HirPat::Unpack(enum_id, idx, sub);
                    }
                }
                HirPat::Wildcard
            }

            PatKind::BindTo(name) => {
                let ty = scrut_ty.clone();
                self.declare_local(name, ty.clone());
                HirPat::Bind(name.clone(), ty)
            }

            PatKind::Wildcard => HirPat::Wildcard,

            PatKind::Or(pats) => {
                let sub: Vec<HirPat> = pats
                    .iter()
                    .map(|p| self.lower_pat(p, scrut_ty))
                    .collect();
                HirPat::Or(sub)
            }
        }
    }

    fn lower_lit_value(&self, lit: &Lit) -> HirLit {
        match lit {
            Lit::Int(s) => HirLit::Int(s.parse().unwrap_or(0)),
            Lit::Float(s) => HirLit::Float(s.parse().unwrap_or(0.0)),
            Lit::String(s) => HirLit::Str(s.clone()),
            Lit::Bool(s) => HirLit::Bool(s == "true"),
            Lit::None => HirLit::Unit,
        }
    }
    
    fn apply(&self, ty: Typ) -> Typ {
        match ty {
            Typ::Var(id) => match self.tx.get_var(id) {
                Var::Unbound => Typ::Var(id),
                Var::Bound(bound) => self.apply(bound.clone()),
            },
            Typ::Struct(id, args) => Typ::Struct(
                id,
                args.into_iter().map(|a| self.apply(a)).collect(),
            ),
            Typ::Enum(id, args) => Typ::Enum(
                id,
                args.into_iter().map(|a| self.apply(a)).collect(),
            ),
            Typ::Fun(id, args, eff) => Typ::Fun(
                id,
                args.into_iter().map(|a| self.apply(a)).collect(),
                eff,
            ),
            Typ::FunRef(ret, params, eff) => Typ::FunRef(
                Box::new(self.apply(*ret)),
                params.into_iter().map(|p| self.apply(p)).collect(),
                eff,
            ),
            other => other,
        }
    }

    fn resolve_field(&self, ty: &Typ, name: &str) -> (usize, Typ) {
        match ty {
            Typ::Struct(id, _args) => {
                let s = self.tx.get_struct(*id);
                for (i, field) in s.fields.iter().enumerate() {
                    if field.name == name {
                        let ty = self.apply(field.typ.clone());
                        return (i, ty);
                    }
                }
                (0, Typ::Error)
            }
            _ => (0, Typ::Error),
        }
    }

    fn call_return_type(&self, callee_ty: &Typ) -> Typ {
        match callee_ty {
            Typ::Fun(id, _args, _) => {
                let fun = self.tx.get_function(*id);
                self.apply(fun.ret.clone())
            }
            Typ::FunRef(ret, _, _) => self.apply(*ret.clone()),
            _ => Typ::Error,
        }
    }

    fn binop_result_type(&self, op: BinOp, operand_ty: &Typ) -> Typ {
        match op {
            BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Le
            | BinOp::Gt | BinOp::Ge | BinOp::And | BinOp::Or => Typ::Bool,
            BinOp::Concat => Typ::Str,
            _ => operand_ty.clone(),
        }
    }

    fn lower_type_hint(
        &self,
        hint: &crow_ast::atom::TypeHint,
    ) -> Typ {
        use crow_ast::atom::TypeHint;
        match hint {
            TypeHint::Local { name, .. } => match name.as_str() {
                "int" => Typ::Int,
                "float" => Typ::Float,
                "bool" => Typ::Bool,
                "str" => Typ::Str,
                _ => {
                    if let Some(Def::Struct(id)) = self.resolve_def(name) {
                        Typ::Struct(*id, vec![])
                    } else if let Some(Def::Enum(id)) =
                        self.resolve_def(name)
                    {
                        Typ::Enum(*id, vec![])
                    } else {
                        Typ::Error
                    }
                }
            },
            TypeHint::Unit(_) => Typ::Unit,
            TypeHint::Fun { params, ret, .. } => {
                let params: Vec<Typ> =
                    params.iter().map(|p| self.lower_type_hint(p)).collect();
                let ret = self.lower_type_hint(ret);
                Typ::FunRef(
                    Box::new(ret),
                    params,
                    EffectRow {
                        known: vec![],
                        tail: None,
                    },
                )
            }
            TypeHint::Infer => Typ::Error,
            TypeHint::Mod { .. } => Typ::Error,
        }
    }
}

pub fn build_hir(
    tx: &TypesCtxt,
    defs: HashMap<String, Def>,
    module: &Module,
) -> HirModule {
    let builder = HirBuilder::new(tx, defs);
    builder.build(module)
}