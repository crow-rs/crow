use std::{collections::{HashMap, HashSet}, sync::Arc};
use crow_ast::{
    atom::TypeHint,
    expr::{Case, Expr, ExprKind, Pat, PatKind},
    item::{Field, Item, ItemKind, Module, Variant},
    stmt::{Stmt, StmtKind},
};
use crow_lex::token::Span;
use miette::NamedSource;
use crate::{errors::ResolverErrors, resolve_ctx::{DefId, DefKind, FieldDef, LocalId, Res, ResolveCtxt, VariantDef}};

struct Scope {
    frames: Vec<HashMap<String, Res>>,
}

impl Scope {
    fn new() -> Self {
        Self { frames: vec![HashMap::new()] }
    }

    fn push(&mut self) {
        self.frames.push(HashMap::new());
    }

    fn pop(&mut self) {
        self.frames.pop();
    }

    fn insert(&mut self, name: String, res: Res) {
        self.frames.last_mut().unwrap().insert(name, res);
    }

    fn lookup(&self, name: &str) -> Option<&Res> {
        for frame in self.frames.iter().rev() {
            if let Some(res) = frame.get(name) {
                return Some(res);
            }
        }
        None
    }

    fn current_bindings(&self) -> HashSet<String> {
        self.frames.last().unwrap().keys().cloned().collect()
    }
}

pub struct Resolver {
    building_ctx: ResolveCtxt,
    next_def_id: u32,
    next_local_id: u32,
    scope: Scope,
    toplevel: HashMap<String, Res>,
    errors: Vec<ResolverErrors>
}

impl Resolver {
    pub fn new() -> Self {
        let mut resolver = Self {
            building_ctx: ResolveCtxt::default(),
            next_def_id: 0,
            next_local_id: 0,
            scope: Scope::new(),
            toplevel: HashMap::new(),
            errors: Vec::new(),
        };
        resolver.register_builtins();
        resolver
    }

    fn register_builtins(&mut self) {
        let builtins = [
            "i8", "i16", "i32", "i64",
            "u8", "u16", "u32", "u64",
            "f32", "f64",
            "bool", "string", "unit",
        ];

        for name in builtins {
            let def_id = self.alloc_def_id();
            self.building_ctx.def_kinds.insert(def_id, DefKind::BuiltinType);
            self.building_ctx.def_names.insert(def_id, name.to_string());
            self.scope.insert(
                name.to_string(),
                Res::Def(DefKind::BuiltinType, def_id),
            );
        }
    }

    fn alloc_def_id(&mut self) -> DefId {
        let id = DefId(self.next_def_id);
        self.next_def_id += 1;
        id
    }

    fn alloc_local_id(&mut self) -> LocalId {
        let id = LocalId(self.next_local_id);
        self.next_local_id += 1;
        id
    }

    fn define_local(&mut self, name: &str, span: Span) -> LocalId {
        let lid = self.alloc_local_id();
        self.building_ctx.local_spans.insert(lid, span);
        self.building_ctx.local_names.insert(lid, name.to_string());
        self.scope.insert(name.to_string(), Res::Local(lid));
        lid
    }

    fn resolve_name(&mut self, name: &str, span: Span, source: Arc<NamedSource<String>>) {
        
        let res = self.scope.lookup(name)
            .cloned()
            .unwrap_or_else(|| {
                self.errors.push(ResolverErrors::UndefinedName { 
                    undef_name: name.to_string(), 
                    src: source.clone(),
                    span: span.1.clone().into()
                });
                Res::Err
            });
        self.building_ctx.resolutions.insert(span, res);
    }


    fn resolve_struct_fields(&mut self, root_id: DefId, fields: &[Field]) {
        let fields: Vec<FieldDef> = fields.iter()
            .enumerate()
            .map(|(i, f)| FieldDef {
                name: f.name.clone(),
                index: i as u32,
                parent: root_id,
            })
            .collect();
        self.building_ctx.struct_fields.insert(root_id, fields);
    }

    fn resolve_enum_variants(&mut self, root_id: DefId, variants: &[Variant]) {
        let mut variant_defs = Vec::new();
        for (i, v) in variants.iter().enumerate() {
            let variant_id = self.alloc_def_id();
            let def_kind = DefKind::Variant {
                enum_def: root_id,
                index: i as u32,
            };
            self.building_ctx.def_kinds.insert(variant_id, def_kind);
            self.building_ctx.def_names.insert(variant_id, v.name.clone());
            self.building_ctx.def_spans.insert(variant_id, v.span.clone());

            self.toplevel.insert(
                v.name.clone(),
                Res::Def(DefKind::Variant { enum_def: root_id, index: i as u32 }, variant_id),
            );
            self.building_ctx.variant_by_name.insert(
                (root_id, v.name.clone()),
                variant_id,
            );

            variant_defs.push(VariantDef {
                name: v.name.clone(),
                index: i as u32,
                parent: root_id,
                arity: v.fields.len(),
            });
        }
        self.building_ctx.enum_variants.insert(root_id, variant_defs);
    }

    fn resolve_toplevel(&mut self, module: &Module) {
        for item in &module.items {
            match &item.kind {
                ItemKind::Struct(s) => {
                    let def_id = self.alloc_def_id();
                    self.building_ctx.def_kinds.insert(def_id, DefKind::Struct);
                    self.building_ctx.def_spans.insert(def_id, item.span.clone());
                    self.building_ctx.def_names.insert(def_id, s.name.clone());
                    self.toplevel.insert(s.name.clone(), Res::Def(DefKind::Struct, def_id));
                    self.resolve_struct_fields(def_id, &s.fields);
                }
                ItemKind::Enum(e) => {
                    let def_id = self.alloc_def_id();
                    self.building_ctx.def_kinds.insert(def_id, DefKind::Enum);
                    self.building_ctx.def_spans.insert(def_id, item.span.clone());
                    self.building_ctx.def_names.insert(def_id, e.name.clone());
                    self.toplevel.insert(e.name.clone(), Res::Def(DefKind::Enum, def_id));
                    self.resolve_enum_variants(def_id, &e.variants);
                }
                ItemKind::Fun(f) => {
                    let def_id = self.alloc_def_id();
                    self.building_ctx.def_kinds.insert(def_id, DefKind::Fun);
                    self.building_ctx.def_spans.insert(def_id, item.span.clone());
                    self.building_ctx.def_names.insert(def_id, f.name.clone());
                    self.toplevel.insert(f.name.clone(), Res::Def(DefKind::Fun, def_id));
                }
                ItemKind::Native(n) => {
                    let def_id = self.alloc_def_id();
                    self.building_ctx.def_kinds.insert(def_id, DefKind::NativeFun);
                    self.building_ctx.def_spans.insert(def_id, item.span.clone());
                    self.building_ctx.def_names.insert(def_id, n.name.clone());
                    self.toplevel.insert(n.name.clone(), Res::Def(DefKind::NativeFun, def_id));
                }
                ItemKind::Const(c) => {
                    let def_id = self.alloc_def_id();
                    self.building_ctx.def_kinds.insert(def_id, DefKind::Const);
                    self.building_ctx.def_spans.insert(def_id, item.span.clone());
                    self.building_ctx.def_names.insert(def_id, c.name.clone());
                    self.toplevel.insert(c.name.clone(), Res::Def(DefKind::Const, def_id));
                }
            }
        }

        for (name, res) in &self.toplevel {
            self.scope.insert(name.clone(), res.clone());
        }
    }


    fn resolve_expr(&mut self, source: Arc<NamedSource<String>>, expr: &Expr) {
        match &expr.kind {
            ExprKind::Lit(_) => {}

            ExprKind::Var(name) => {
                self.resolve_name(name, expr.span.clone(), source.clone());
            }

            ExprKind::Unary(inner, _op) => {
                self.resolve_expr(source.clone(), inner);
            }

            ExprKind::Bin(lhs, rhs, _op) => {
                self.resolve_expr(source.clone(), lhs);
                self.resolve_expr(source.clone(), rhs);
            }

            ExprKind::Assign(target, value) => {
                self.resolve_expr(source.clone(), target);
                self.resolve_expr(source.clone(), value);
            }

            ExprKind::If(cond, then_, else_) => {
                self.resolve_expr(source.clone(), cond);
                self.resolve_expr(source.clone(), then_);
                if let Some(else_branch) = else_ {
                    self.resolve_expr(source.clone(), else_branch);
                }
            }

            ExprKind::Field(base, _name) => {
                self.resolve_expr(source.clone(), base);
            }

            ExprKind::Call(func, args) => {
                self.resolve_expr(source.clone(), func);
                for arg in args {
                    self.resolve_expr(source.clone(), arg);
                }
            }

            ExprKind::Function(params, body) => {
                self.scope.push();
                for param in params {
                    self.define_local(&param.name, param.span.clone());
                    self.resolve_type_hint(&param.hint, source.clone());
                }
                self.resolve_expr(source.clone(), body);
                self.scope.pop();
            }

            ExprKind::Match(scrutinees, cases) => {
                for scrutinee in scrutinees {
                    self.resolve_expr(source.clone(), scrutinee);
                }
                for case in cases {
                    self.resolve_case(source.clone(), case);
                }
            }

            ExprKind::Paren(inner) => {
                self.resolve_expr(source.clone(), inner);
            }

            ExprKind::Block(stmts) => {
                self.scope.push();
                for stmt in stmts {
                    self.resolve_stmt(source.clone(), stmt);
                }
                self.scope.pop();
            }

            ExprKind::Todo(msg) => {
                if let Some(e) = msg {
                    self.resolve_expr(source.clone(), e);
                }
            }

            ExprKind::Panic(msg) => {
                if let Some(e) = msg {
                    self.resolve_expr(source.clone(), e);
                }
            }
        }
    }

    fn resolve_stmt(&mut self, source: Arc<NamedSource<String>>, stmt: &Stmt) {
        match &stmt.kind {
            StmtKind::Let(name, hint, value) => {
                self.resolve_expr(source.clone(), value);
                self.resolve_type_hint(hint, source.clone());
                self.define_local(name, stmt.span.clone());
            }
            StmtKind::Expr(expr) => {
                self.resolve_expr(source.clone(), expr);
            }
        }
    }

    fn resolve_pat(&mut self, source: Arc<NamedSource<String>>, pat: &Pat) {
        
        match &pat.kind {
            PatKind::Lit(_) => {}

            PatKind::Wildcard => {}

            PatKind::BindTo(name) => {
                self.define_local(name, pat.span.clone());
            }

            PatKind::Variant(expr) => {
                self.resolve_expr(source.clone(), expr);
            }

            PatKind::Unpack(constructor, sub_pats) => {
                self.resolve_expr(source.clone(), constructor);
                for sub_pat in sub_pats {
                    self.resolve_pat(source.clone(), sub_pat);
                }
            }

            PatKind::Or(alternatives) => {
                let mut all_bindings: Vec<HashSet<String>> = Vec::new();

                for alt in alternatives {
                    self.scope.push();
                    self.resolve_pat(source.clone(),alt);
                    let bindings = self.scope.current_bindings();
                    all_bindings.push(bindings);
                    self.scope.pop();
                }

                let first = &all_bindings[0];
                for bindings in all_bindings.iter().skip(1) {
                    for name in first.difference(bindings) {
                        self.errors.push(ResolverErrors::NotBound { 
                            name: name.clone(), 
                            src: source.clone(),
                            span: pat.span.1.clone().into()
                        });
                    }
                    for name in bindings.difference(first) {
                        self.errors.push(ResolverErrors::NotBound { 
                            name: name.clone(), 
                            src: source.clone(),
                            span: pat.span.1.clone().into() 
                        });
                    }
                }

                self.scope.push();
                self.resolve_pat(source.clone(),&alternatives[0]);
            }
        }
    }

    fn resolve_case(&mut self, source: Arc<NamedSource<String>>, case: &Case) {
        self.scope.push();
        for pat in &case.pats {
            self.resolve_pat(source.clone(), pat);
        }
        self.resolve_expr(source.clone(), &case.body);
        self.scope.pop();
    }

    fn resolve_type_hint(&mut self, hint: &TypeHint, source: Arc<NamedSource<String>>) {
        match hint {
            TypeHint::Local { span, name, args } => {
                self.resolve_name(name, span.clone(), source.clone());
                for arg in args {
                    self.resolve_type_hint(arg, source.clone());
                }
            }
            TypeHint::Mod { span, module, name, args } => {
                // module.Name — резолв модуля
                // TODO: resolve module path
                for arg in args {
                    self.resolve_type_hint(arg, source.clone());
                }
            }
            TypeHint::Fun { params, ret, .. } => {
                for param in params {
                    self.resolve_type_hint(param, source.clone());
                }
                self.resolve_type_hint(ret, source.clone());
            }
            TypeHint::Unit(_) => {}
            TypeHint::Infer => {}
        }
    }

    fn resolve_bodies(&mut self, module: &Module) {
        for item in &module.items {
            match &item.kind {
                ItemKind::Fun(f) => {
                    self.scope.push();
                    for param in &f.params {
                        self.define_local(&param.name, param.span.clone());
                        self.resolve_type_hint(&param.hint, module.source.clone());
                    }
                    self.resolve_type_hint(&f.ret, module.source.clone());
                    self.resolve_expr(module.source.clone(), &f.block);
                    self.scope.pop();
                }
                ItemKind::Native(n) => {
                    self.scope.push();
                    for param in &n.params {
                        self.resolve_type_hint(&param.hint, module.source.clone());
                    }
                    self.resolve_type_hint(&n.ret, module.source.clone());
                    self.scope.pop();
                }
                ItemKind::Const(c) => {
                    self.resolve_type_hint(&c.hint, module.source.clone());
                    self.resolve_expr(module.source.clone(), &c.value);
                }
                ItemKind::Struct(s) => {
                    self.scope.push();
                    for field in &s.fields {
                        self.resolve_type_hint(&field.hint, module.source.clone());
                    }
                    self.scope.pop();
                }
                ItemKind::Enum(e) => {
                    self.scope.push();
                    for variant in &e.variants {
                        for field_hint in &variant.fields {
                            self.resolve_type_hint(field_hint, module.source.clone());
                        }
                    }
                    self.scope.pop();
                }
            }
        }
    }

    pub fn resolve_ast(&mut self, module: &Module) -> Result<ResolveCtxt, Vec<ResolverErrors>> {
        self.resolve_toplevel(&module);
        self.resolve_bodies(&module);
        if self.errors.is_empty() {
            Ok(std::mem::take(&mut self.building_ctx))
        } else {
            Err(std::mem::take(&mut self.errors))
        }
    }
}