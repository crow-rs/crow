/// Imports
use crate::{
    errors::ResolverErrors,
    table::{
        DefId, DefKind, FieldDef, LocalId, Res, ResolveTable, VariantDef,
    },
};
use crow_ast::{
    atom::{Mutability, TypeHint},
    expr::{Case, Expr, ExprKind, Pat, PatKind},
    item::{
        Const, Enum, Field, Fun, ItemKind, Module, NativeFun, Struct,
        Variant,
    },
    stmt::{Stmt, StmtKind},
};
use crow_common::{bug, span::Span};
use crow_fresh::Freshen;
use std::{
    collections::{HashMap, HashSet},
};

/// Defines single rib
type Rib = HashMap<String, Res>;

/// Defines ribs stack
struct RibsStack {
    ribs: Vec<Rib>,
}

/// Ribs stack implementation
impl RibsStack {
    /// Creates new ribs stack
    fn new() -> Self {
        Self {
            ribs: vec![HashMap::new()],
        }
    }

    /// Pushes new rib onto the stack
    fn push(&mut self) {
        self.ribs.push(HashMap::new());
    }

    /// Pops one rib from the stack
    fn pop(&mut self) {
        self.ribs.pop();
    }

    /// Inserts variable into last rib
    fn insert(&mut self, name: String, res: Res) {
        self.ribs
            .last_mut()
            .unwrap_or_else(|| bug!("insert with empty ribs stack"))
            .insert(name, res);
    }

    /// Lookups name in the ribs stack
    fn lookup(&self, name: &str) -> Option<&Res> {
        for rib in self.ribs.iter().rev() {
            if let Some(res) = rib.get(name) {
                return Some(res);
            }
        }
        None
    }

    /// Returns current bindings hash set
    fn current_bindings(&self) -> HashSet<String> {
        self.ribs
            .last()
            .unwrap_or_else(|| {
                bug!("request for bindings with empty ribs stack")
            })
            .keys()
            .cloned()
            .collect()
    }
}

/// All the builtin-types
const BUILTIN_TYPES: [&'static str; 13] = [
    "i8", "i16", "i32", "i64", "u8", "u16", "u32", "u64", "f32", "f64",
    "bool", "string", "unit",
];

/// Defines a resolver used to resolve
/// all the top-level items, locals and type hints
pub struct Resolver {
    /// Resolutions table
    table: ResolveTable,

    /// Defs freshen
    freshen_defs: Freshen<u32>,

    /// Locals freshen
    freshen_locals: Freshen<u32>,

    /// Ribs stack
    ribs: RibsStack,

    /// Top-level resolutions
    top_level: HashMap<String, Res>,

    /// Resolver errors
    errors: Vec<ResolverErrors>,
}

/// Resolver implementation
impl Resolver {
    /// Creates new resolver with registered builtins
    pub fn new() -> Self {
        let mut resolver = Self {
            table: ResolveTable::default(),
            freshen_defs: Freshen::new(),
            freshen_locals: Freshen::new(),
            ribs: RibsStack::new(),
            top_level: HashMap::new(),
            errors: Vec::new(),
        };
        resolver.register_builtins();
        resolver
    }

    /// Registers builtin types
    fn register_builtin_types(&mut self) {
        for name in BUILTIN_TYPES {
            let def_id = DefId(self.freshen_defs.fresh());
            self.table.def_kinds.insert(def_id, DefKind::BuiltinType);
            self.table.def_names.insert(def_id, name.to_string());
            self.ribs.insert(
                name.to_string(),
                Res::Def(DefKind::BuiltinType, def_id),
            );
        }
    }

    /// Registers builtins
    fn register_builtins(&mut self) {
        // Registering builtin types
        self.register_builtin_types();
    }

    /// Defines local in the ribs stack and resolve table
    fn define_local(
        &mut self,
        name: &str,
        mutability: Mutability,
        span: Span,
    ) -> LocalId {
        let lid = LocalId(self.freshen_locals.fresh());
        self.table.resolutions.insert(span.clone(), Res::Local(lid));
        self.table.local_spans.insert(lid, span);
        self.table.local_names.insert(lid, name.to_string());
        self.table.local_mutabilities.insert(lid, mutability);
        self.ribs.insert(name.to_string(), Res::Local(lid));
        lid
    }

    /// Resolves local name
    fn resolve_local(&mut self, name: &str, span: Span) {
        // Resolving name in ribs stack
        let res = self.ribs.lookup(name).cloned().unwrap_or_else(|| {
            self.errors.push(ResolverErrors::UndefinedName {
                name: name.to_string(),
                src: span.0.clone(),
                span: span.1.clone().into(),
            });
            Res::Err
        });

        // Binding in table
        self.table.resolutions.insert(span, res);
    }

    /// Resolves struct fields
    fn resolve_struct_fields(&mut self, root_id: DefId, fields: &[Field]) {
        let fields: Vec<FieldDef> = fields
            .iter()
            .enumerate()
            .map(|(i, f)| FieldDef {
                name: f.name.clone(),
                index: i as u32,
                parent: root_id,
            })
            .collect();
        self.table.struct_fields.insert(root_id, fields);
    }

    /// Resolves enum variant
    fn resolve_enum_variant(
        &mut self,
        enum_id: DefId,
        idx: usize,
        variant: &Variant,
    ) -> VariantDef {
        // Getting fresh def id for variant
        let variant_id = DefId(self.freshen_defs.fresh());

        // Preparing def kind
        let def_kind = DefKind::Variant {
            enum_def: enum_id,
            index: idx as u32,
        };

        // Updating definitions in table
        self.table.def_kinds.insert(variant_id, def_kind);
        self.table
            .def_names
            .insert(variant_id, variant.name.clone());
        self.table
            .def_spans
            .insert(variant_id, variant.span.clone());

        // Updating top-levels
        self.top_level.insert(
            variant.name.clone(),
            Res::Def(
                DefKind::Variant {
                    enum_def: enum_id,
                    index: idx as u32,
                },
                variant_id,
            ),
        );

        // Updating variant-by-name in table
        self.table
            .variant_by_name
            .insert((enum_id, variant.name.clone()), variant_id);

        // Preparing variant def
        VariantDef {
            name: variant.name.clone(),
            index: idx as u32,
            parent: enum_id,
            arity: variant.fields.len(),
        }
    }

    /// Resolves enum variant
    fn resolve_enum_variants(
        &mut self,
        enum_id: DefId,
        variants: &[Variant],
    ) {
        // Resolving variants
        let mut variant_defs = Vec::new();
        for (idx, variant) in variants.iter().enumerate() {
            variant_defs
                .push(self.resolve_enum_variant(enum_id, idx, variant));
        }

        // Updating table
        self.table.enum_variants.insert(enum_id, variant_defs);
    }

    /// Resolves struct
    fn resolve_struct(&mut self, span: &Span, s: &Struct) {
        // Getting fresh def id
        let def_id = DefId(self.freshen_defs.fresh());

        // Updating table
        self.table.def_kinds.insert(def_id, DefKind::Struct);
        self.table.def_spans.insert(def_id, span.clone());
        self.table.def_names.insert(def_id, s.name.clone());

        // Updating top-level
        self.top_level
            .insert(s.name.clone(), Res::Def(DefKind::Struct, def_id));

        // Resolving struct fields
        self.resolve_struct_fields(def_id, &s.fields);
    }

    /// Resolves enum
    fn resolve_enum(&mut self, span: &Span, e: &Enum) {
        // Getting fresh def id
        let def_id = DefId(self.freshen_defs.fresh());

        // Updating table
        self.table.def_kinds.insert(def_id, DefKind::Enum);
        self.table.def_spans.insert(def_id, span.clone());
        self.table.def_names.insert(def_id, e.name.clone());

        // Updating top-level
        self.top_level
            .insert(e.name.clone(), Res::Def(DefKind::Enum, def_id));

        // Resolving enum variants
        self.resolve_enum_variants(def_id, &e.variants);
    }

    /// Resolves function
    fn resolve_function(&mut self, span: &Span, f: &Fun) {
        // Getting fresh def id
        let def_id = DefId(self.freshen_defs.fresh());

        // Updating table
        self.table.def_kinds.insert(def_id, DefKind::Fun);
        self.table.def_spans.insert(def_id, span.clone());
        self.table.def_names.insert(def_id, f.name.clone());

        // Updating top-level
        self.top_level
            .insert(f.name.clone(), Res::Def(DefKind::Fun, def_id));
    }

    /// Resolves native function
    fn resolve_native_function(&mut self, span: &Span, f: &NativeFun) {
        // Getting fresh def id
        let def_id = DefId(self.freshen_defs.fresh());

        // Updating table
        self.table.def_kinds.insert(def_id, DefKind::NativeFun);
        self.table.def_spans.insert(def_id, span.clone());
        self.table.def_names.insert(def_id, f.name.clone());

        // Updating top-level
        self.top_level
            .insert(f.name.clone(), Res::Def(DefKind::NativeFun, def_id));
    }

    /// Resolves constant
    fn resolve_const(&mut self, span: &Span, c: &Const) {
        // Getting fresh def id
        let def_id = DefId(self.freshen_defs.fresh());

        // Updating table
        self.table.def_kinds.insert(def_id, DefKind::Const);
        self.table.def_spans.insert(def_id, span.clone());
        self.table.def_names.insert(def_id, c.name.clone());

        // Updating top-level
        self.top_level
            .insert(c.name.clone(), Res::Def(DefKind::Const, def_id));
    }

    /// Resolves top-level
    fn resolve_top_level(&mut self, module: &Module) {
        // Resolving items
        for item in &module.items {
            match &item.kind {
                ItemKind::Struct(s) => self.resolve_struct(&item.span, s),
                ItemKind::Enum(e) => self.resolve_enum(&item.span, e),
                ItemKind::Fun(f) => self.resolve_function(&item.span, f),
                ItemKind::Native(n) => {
                    self.resolve_native_function(&item.span, n)
                }
                ItemKind::Const(c) => self.resolve_const(&item.span, c),
            }
        }

        // Updating ribs stack
        for (name, res) in &self.top_level {
            self.ribs.insert(name.clone(), res.clone());
        }
    }

    /// Ensures local is mutable
    fn ensure_mutable(&mut self, span: &Span) {
        match self.table.resolutions.get(&span) {
            Some(res) => {
                match res {
                    Res::Local(lid) => {
                        if self
                            .table
                            .local_mutabilities
                            .get(lid)
                            .unwrap_or_else(|| {
                                bug!(
                                    "no mutability found for lid `{lid:?}`"
                                )
                            })
                            != &Mutability::Mut
                        {
                            self.errors.push(ResolverErrors::ImmutAssign {
                            name: self.table.local_names.get(lid).unwrap_or_else(|| {
                                bug!("no name found for lid `{lid:?}`")
                            }).clone(),
                            src: span.0.clone(),
                            span: span.1.clone().into(),
                        });
                        }
                    }
                    _ => {}
                }
            }
            None => bug!("no resolution for span {span:?}"),
        }
    }

    /// Resolves expression
    fn resolve_expr(&mut self, expr: &Expr) {
        match &expr.kind {
            ExprKind::Lit(_) => {}
            ExprKind::Var(name) => {
                self.resolve_local(name, expr.span.clone());
            }
            ExprKind::Unary(inner, _op) => {
                self.resolve_expr(inner);
            }
            ExprKind::Bin(lhs, rhs, _op) => {
                self.resolve_expr(lhs);
                self.resolve_expr(rhs);
            }
            ExprKind::Assign(target, value) => {
                self.resolve_expr(target);
                self.ensure_mutable(&target.span);
                self.resolve_expr(value);
            }
            ExprKind::If(cond, then_, else_) => {
                self.resolve_expr(cond);
                self.resolve_expr(then_);
                if let Some(else_branch) = else_ {
                    self.resolve_expr(else_branch);
                }
            }
            ExprKind::Field(base, _name) => {
                self.resolve_expr(base);
            }
            ExprKind::Call(func, args) => {
                self.resolve_expr(func);
                for arg in args {
                    self.resolve_expr(arg);
                }
            }
            ExprKind::Function(params, body) => {
                self.ribs.push();
                for param in params {
                    self.define_local(
                        &param.name,
                        Mutability::Immut, // todo: support mutable params
                        param.span.clone(),
                    );
                    self.resolve_type_hint(&param.hint);
                }
                self.resolve_expr(body);
                self.ribs.pop();
            }
            ExprKind::Match(scrutinees, cases) => {
                for scrutinee in scrutinees {
                    self.resolve_expr(scrutinee);
                }
                for case in cases {
                    self.resolve_case(case);
                }
            }
            ExprKind::Paren(inner) => {
                self.resolve_expr(inner);
            }

            ExprKind::Block(stmts) => {
                self.ribs.push();
                for stmt in stmts {
                    self.resolve_stmt(stmt);
                }
                self.ribs.pop();
            }
            ExprKind::Todo(msg) => {
                if let Some(e) = msg {
                    self.resolve_expr(e);
                }
            }
            ExprKind::Panic(msg) => {
                if let Some(e) = msg {
                    self.resolve_expr(e);
                }
            }
        }
    }

    /// Resolves statement
    fn resolve_stmt(&mut self, stmt: &Stmt) {
        match &stmt.kind {
            StmtKind::Binding(name, hint, mutability, value) => {
                self.resolve_expr(value);
                self.resolve_type_hint(hint);
                self.define_local(name, *mutability, stmt.span.clone());
            }
            StmtKind::Expr(expr) => {
                self.resolve_expr(expr);
            }
            StmtKind::Wildcard(hint, value) => {
                self.resolve_type_hint(hint);
                self.resolve_expr(value);
            }
        }
    }

    /// Resolves pattern
    fn resolve_pat(&mut self, pat: &Pat) {
        match &pat.kind {
            PatKind::Lit(_) => {}
            PatKind::Wildcard => {}
            PatKind::BindTo(mutability, name) => {
                self.define_local(name, *mutability, pat.span.clone());
            }
            PatKind::Variant(expr) => {
                self.resolve_expr(expr);
            }
            PatKind::Unpack(constructor, sub_pats) => {
                self.resolve_expr(constructor);
                for sub_pat in sub_pats {
                    self.resolve_pat(sub_pat);
                }
            }
            PatKind::Or(alternatives) => {
                // Resolving alternatives
                let mut all_bindings: Vec<HashSet<String>> = Vec::new();
                for alt in alternatives {
                    self.ribs.push();
                    self.resolve_pat(alt);
                    let bindings = self.ribs.current_bindings();
                    all_bindings.push(bindings);
                    self.ribs.pop();
                }

                // Checking all alternatives bounds same bindings
                let first = &all_bindings[0];
                for bindings in all_bindings.iter().skip(1) {
                    for name in first.difference(bindings) {
                        self.errors.push(ResolverErrors::NotBound {
                            name: name.clone(),
                            src: pat.span.0.clone(),
                            span: pat.span.1.clone().into(),
                        });
                    }
                    for name in bindings.difference(first) {
                        self.errors.push(ResolverErrors::NotBound {
                            name: name.clone(),
                            src: pat.span.0.clone(),
                            span: pat.span.1.clone().into(),
                        });
                    }
                }
            }
        }
    }

    /// Resolves case
    fn resolve_case(&mut self, case: &Case) {
        self.ribs.push();
        for pat in &case.pats {
            self.resolve_pat(pat);
        }
        self.resolve_expr(&case.body);
        self.ribs.pop();
    }

    /// Resolves type hint
    fn resolve_type_hint(&mut self, hint: &TypeHint) {
        match hint {
            TypeHint::Local { span, name, args } => {
                self.resolve_local(name, span.clone());
                for arg in args {
                    self.resolve_type_hint(arg);
                }
            }
            TypeHint::Mod { args, .. } => {
                for arg in args {
                    self.resolve_type_hint(arg);
                }
            }
            TypeHint::Fun { params, ret, .. } => {
                for param in params {
                    self.resolve_type_hint(param);
                }
                self.resolve_type_hint(ret);
            }
            TypeHint::Unit(_) => {}
            TypeHint::Infer => {}
        }
    }

    /// Resolves bodies
    fn resolve_bodies(&mut self, module: &Module) {
        for item in &module.items {
            match &item.kind {
                ItemKind::Fun(f) => {
                    self.ribs.push();
                    for param in &f.params {
                        self.define_local(
                            &param.name,
                            Mutability::Immut, // todo: support mutable params
                            param.span.clone(),
                        );
                        self.resolve_type_hint(&param.hint);
                    }
                    self.resolve_type_hint(&f.ret);
                    self.resolve_expr(&f.block);
                    self.ribs.pop();
                }
                ItemKind::Native(n) => {
                    self.ribs.push();
                    for param in &n.params {
                        self.resolve_type_hint(&param.hint);
                    }
                    self.resolve_type_hint(&n.ret);
                    self.ribs.pop();
                }
                ItemKind::Const(c) => {
                    self.resolve_type_hint(&c.hint);
                    self.resolve_expr(&c.value);
                }
                ItemKind::Struct(s) => {
                    self.ribs.push();
                    for field in &s.fields {
                        self.resolve_type_hint(&field.hint);
                    }
                    self.ribs.pop();
                }
                ItemKind::Enum(e) => {
                    self.ribs.push();
                    for variant in &e.variants {
                        for hint in &variant.fields {
                            self.resolve_type_hint(hint);
                        }
                    }
                    self.ribs.pop();
                }
            }
        }
    }

    /// Resolves ast
    pub fn resolve_ast(
        &mut self,
        module: &Module,
    ) -> Result<ResolveTable, Vec<ResolverErrors>> {
        // Resolving top-level
        self.resolve_top_level(&module);

        // Resolving top-level bodies
        self.resolve_bodies(&module);

        // Checking for errors
        if self.errors.is_empty() {
            Ok(std::mem::take(&mut self.table))
        } else {
            Err(std::mem::take(&mut self.errors))
        }
    }
}
