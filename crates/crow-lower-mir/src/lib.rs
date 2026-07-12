use std::collections::{HashMap, HashSet};

use crow_ast::atom::{
    BinOp as AstBinOp, Lit, Mutability, UnOp as AstUnOp,
};
use crow_collect_spec_item::SpecialItems;
use crow_common::{DefId, LocalId};
use crow_hir::{
    Hir,
    body::HirBody,
    expr::{DivergeKind, HirArm, HirExprKind, HirParam},
    id::*,
    item::{HirEnumDef, HirGenericParam, HirItemKind, HirStructDef},
    pat::HirPatKind,
    stmt::HirStmtKind,
    ty::{HirTy, HirTyKind},
};
use crow_mir::{
    AdtDef, AggregateKind, BasicBlock, BinOp, Block, CastKind, ConstId, Constant, FnId, GcDefs, LangDefs, Local, LocalDecl, MirBody, MirConstDef, MirModule, MirNative, MirTyCtxt, NativeId, Operand, Place, Projection, RETURN_PLACE, Rvalue, Statement, Terminator, UnOp,
};
use crow_resolving::table::Res;
use crow_tycheck::typeck::TypeckOutput;
use crow_tycheck::{
    typeck::TypeckBody,
};
use crow_types::{FloatTy, IntTy, Ty};

#[derive(Clone, Copy, PartialEq, Eq)]
enum DefKind {
    Fn,
    Native,
    Const,
    Struct,
    Variant,
}

struct VariantInfo {
    enum_def_id: DefId,
    index: u32,
}

pub struct IntrinsicInfo {
    pub name: String,
    pub ret_ty: Ty,
    pub arg_count: usize,
}

pub struct LoweringCtxt<'hir> {
    hir: &'hir Hir,
    typeck: &'hir TypeckOutput,

    lang_items: &'hir SpecialItems,

    fn_map: HashMap<DefId, FnId>,
    native_map: HashMap<DefId, NativeId>,
    const_map: HashMap<DefId, ConstId>,

    def_kinds: HashMap<DefId, DefKind>,
    variant_info: HashMap<DefId, VariantInfo>,
    field_indices: HashMap<(DefId, String), u32>,

    mir_tcx: MirTyCtxt,
    mir_functions: Vec<MirBody>,
    mir_natives: Vec<MirNative>,
    mir_constants: Vec<MirConstDef>,

    mir_intrin_fns: HashMap<FnId, IntrinsicInfo>,
    drop_glue: HashMap<DefId, FnId>,

    lang_rc_alloc: Option<FnId>,
    lang_rc_retain: Option<FnId>,
    lang_rc_release: Option<FnId>,
    lang_panic: Option<FnId>,
}

impl<'hir> LoweringCtxt<'hir> {
    fn new(hir: &'hir Hir, typeck: &'hir TypeckOutput, spec_items: &'hir SpecialItems) -> Self {
        let mut lcx = LoweringCtxt {
            hir,
            typeck,
            lang_items: spec_items,
            fn_map: HashMap::new(),
            native_map: HashMap::new(),
            const_map: HashMap::new(),
            def_kinds: HashMap::new(),
            variant_info: HashMap::new(),
            field_indices: HashMap::new(),
            mir_tcx: MirTyCtxt {
                adts: HashMap::new(),
            },
            mir_functions: Vec::new(),
            mir_natives: Vec::new(),
            mir_constants: Vec::new(),
            mir_intrin_fns: HashMap::new(),
            lang_rc_alloc: None,
            lang_rc_retain: None,
            lang_rc_release: None,
            lang_panic: None,
            drop_glue: HashMap::new()
        };
        lcx.scan_items();
        lcx
    }

    fn hir_ty_to_ty(&self, hty: &HirTy) -> Ty {
        match &hty.kind {
            HirTyKind::Unit => Ty::Unit,
            HirTyKind::Infer => Ty::Unit,
            HirTyKind::Res {
                res: Res::Def(_, did),
                args,
            } => {
                if let Some(prim) = self.typeck.prim_tys.get(did) {
                    return prim.clone();
                }
                let type_args: Vec<Ty> =
                    args.iter().map(|a| self.hir_ty_to_ty(a)).collect();
                Ty::Adt(*did, type_args)
            }
            HirTyKind::Res { .. } => Ty::Unit,
            HirTyKind::Fn { params, ret, .. } => {
                let p: Vec<Ty> =
                    params.iter().map(|t| self.hir_ty_to_ty(t)).collect();
                Ty::Fn(p, Box::new(self.hir_ty_to_ty(ret)))
            }
        }
    }

    fn find_native(&self, name: &str) -> Option<NativeId> {
        self.mir_natives.iter()
            .enumerate()
            .find(|(_, n)| n.symbol == name)
            .map(|(i, _)| NativeId::from(i))
    }

    fn find_intrinsic_by_ret(&self, intrinsic_name: &str, ret_ty: &Ty) -> Option<FnId> {
        self.mir_intrin_fns.iter()
            .find(|(_, info)| info.name == intrinsic_name && info.ret_ty == *ret_ty)
            .map(|(id, _)| *id)
    }

    fn find_intrinsic_void(&self, intrinsic_name: &str) -> Option<FnId> {
        self.mir_intrin_fns.iter()
            .find(|(_, info)| info.name == intrinsic_name && info.ret_ty == Ty::Unit)
            .map(|(id, _)| *id)
    }

    //todo - proper recalc ptr size from target config
    fn type_size(&self, ty: &Ty) -> u32 {
        match ty {
            Ty::Bool => 1,
            Ty::Int(IntTy::I8 | IntTy::U8) => 1,
            Ty::Int(IntTy::I16 | IntTy::U16) => 2,
            Ty::Int(IntTy::I32 | IntTy::U32) => 4,
            Ty::Int(IntTy::I64 | IntTy::U64) => 8,
            Ty::Float(FloatTy::F32) => 4,
            Ty::Float(FloatTy::F64) => 8,
            Ty::String => 16,
            Ty::RawPtr => 8,
            Ty::Adt(_, _) => 8, // pointer на куче
            _ => 8,
        }
    }

    fn generate_drop_glue(&mut self, def_id: DefId, adt: &AdtDef) -> FnId {
        let fn_id = FnId::from(self.mir_functions.len());

        let release_fn = self.lang_rc_release
            .expect("drop glue needs @lang_def(\"rc_release\")");

        let fields = &adt.variants[0].fields;

        let mut rc_fields: Vec<(u32, DefId)> = Vec::new();
        let mut offset: u32 = 0;
        for field_ty in fields {
            if let Ty::Adt(field_def_id, _) = field_ty {
                rc_fields.push((offset, *field_def_id));
            }
            offset += self.type_size(field_ty);
        }

        let mut locals = Vec::new();
        locals.push(LocalDecl { ty: Ty::Unit, name: None, mutability: Mutability::Immut });       
        locals.push(LocalDecl { ty: Ty::RawPtr, name: Some("ptr".into()), mutability: Mutability::Immut }); 
        locals.push(LocalDecl { ty: Ty::RawPtr, name: Some("header".into()), mutability: Mutability::Immut }); 
        locals.push(LocalDecl { ty: Ty::Int(IntTy::I32), name: Some("count".into()), mutability: Mutability::Immut }); 
        locals.push(LocalDecl { ty: Ty::Bool, name: None, mutability: Mutability::Immut });
        locals.push(LocalDecl { ty: Ty::Unit, name: None, mutability: Mutability::Immut });

        let arg_ptr = Local(1);
        let header = Local(2);
        let count = Local(3);
        let cmp = Local(4);
        let void_tmp = Local(5);

        let mut field_locals: Vec<Local> = Vec::new();
        for _ in &rc_fields {
            let local = Local::from(locals.len());
            locals.push(LocalDecl { ty: Ty::RawPtr, name: None, mutability: Mutability::Immut });
            field_locals.push(local);
        }

        let ptr_offset_fn = self.find_intrinsic_by_ret("ptr_offset", &Ty::RawPtr).unwrap();
        let ptr_read_i32_fn = self.find_intrinsic_by_ret("ptr_read", &Ty::Int(IntTy::I32)).unwrap();
        let ptr_read_ptr_fn = self.find_intrinsic_by_ret("ptr_read", &Ty::RawPtr).unwrap();

        let mut blocks: Vec<BasicBlock> = Vec::new();

        let bb_read_count = Block(1);
        let bb_switch = Block(2);
        let bb_release_self = if rc_fields.is_empty() {
            Block(3)
        } else {
            Block(3 + rc_fields.len() as u32 * 2)
        };
        let bb_ret = Block(bb_release_self.0 + 1);

        blocks.push(BasicBlock {
            stmts: vec![],
            term: Terminator::Call {
                func: Operand::Const(Constant::Fn(ptr_offset_fn, vec![])),
                args: vec![
                    Operand::Copy(Place::local(arg_ptr)),
                    Operand::Const(Constant::Int(-4, IntTy::I64)),
                ],
                dest: Place::local(header),
                target: bb_read_count,
            },
        });

        blocks.push(BasicBlock {
            stmts: vec![],
            term: Terminator::Call {
                func: Operand::Const(Constant::Fn(ptr_read_i32_fn, vec![])),
                args: vec![
                    Operand::Copy(Place::local(header)),
                    Operand::Const(Constant::Int(0, IntTy::I64)),
                ],
                dest: Place::local(count),
                target: bb_switch,
            },
        });

        let first_child = if rc_fields.is_empty() { bb_release_self } else { Block(3) };
        blocks.push(BasicBlock {
            stmts: vec![
                Statement::Assign(
                    Place::local(cmp),
                    Rvalue::BinaryOp(
                        BinOp::Eq,
                        Operand::Copy(Place::local(count)),
                        Operand::Const(Constant::Int(1, IntTy::I32)),
                    ),
                ),
            ],
            term: Terminator::SwitchInt {
                discr: Operand::Copy(Place::local(cmp)),
                targets: vec![(1, first_child)],
                otherwise: bb_release_self,
            },
        });

        for (i, (field_offset, _field_def_id)) in rc_fields.iter().enumerate() {
            let field_local = field_locals[i];
            let next = if i + 1 < rc_fields.len() {
                Block(3 + (i as u32 + 1) * 2)
            } else {
                bb_release_self
            };

            blocks.push(BasicBlock {
                stmts: vec![],
                term: Terminator::Call {
                    func: Operand::Const(Constant::Fn(ptr_read_ptr_fn, vec![])),
                    args: vec![
                        Operand::Copy(Place::local(arg_ptr)),
                        Operand::Const(Constant::Int(*field_offset as i128, IntTy::I64)),
                    ],
                    dest: Place::local(field_local),
                    target: Block(3 + i as u32 * 2 + 1),
                },
            });

            blocks.push(BasicBlock {
                stmts: vec![],
                term: Terminator::Call {
                    func: Operand::Const(Constant::Fn(release_fn, vec![])),
                    args: vec![Operand::Copy(Place::local(field_local))],
                    dest: Place::local(void_tmp),
                    target: next,
                },
            });
        }

        blocks.push(BasicBlock {
            stmts: vec![],
            term: Terminator::Call {
                func: Operand::Const(Constant::Fn(release_fn, vec![])),
                args: vec![Operand::Copy(Place::local(arg_ptr))],
                dest: Place::local(void_tmp),
                target: bb_ret,
            },
        });

        blocks.push(BasicBlock {
            stmts: vec![],
            term: Terminator::Return,
        });

        self.mir_functions.push(MirBody {
            name: format!("__drop_{}", adt.name),
            arg_count: 1,
            type_params: vec![],
            locals,
            blocks,
        });

        self.drop_glue.insert(def_id, fn_id);
        fn_id
    }

    fn handle_specialized_items(&mut self, def_id: DefId, fn_id: FnId) {
        let lang = &self.lang_items.lang;

        // GC
        if Some(def_id) == lang.rc_alloc { self.lang_rc_alloc = Some(fn_id); }
        if Some(def_id) == lang.rc_retain { self.lang_rc_retain = Some(fn_id); }
        if Some(def_id) == lang.rc_release { self.lang_rc_release = Some(fn_id); }

        // Panic
        if Some(def_id) == lang.panic { self.lang_panic = Some(fn_id); }
    }

    fn scan_items(&mut self) {
        for item in self.hir.items.vec() {
            let did = item.def_id;
            match &item.kind {
                HirItemKind::Fun(f) => {
                    let fn_id = FnId::from(self.mir_functions.len());
                    self.fn_map.insert(did, fn_id);
                    self.def_kinds.insert(did, DefKind::Fn);

                    if let Some(name) = self.lang_items.intrinsics.get(did) {
                        let ret = self.hir_ty_to_ty(&f.ret);
                        self.mir_intrin_fns.insert(fn_id, IntrinsicInfo {
                            name: name.to_string(),
                            ret_ty: ret,
                            arg_count: f.params.len(),
                        });
                    }

                    self.handle_specialized_items(did, fn_id);

                    self.mir_functions.push(MirBody {
                        name: f.name.clone(),
                        arg_count: f.params.len(),
                        type_params: f.type_params.iter().map(|tp| tp.def_id).collect(),
                        locals: Vec::new(),
                        blocks: Vec::new(),
                    });
                }
                HirItemKind::Native(n) => {
                    let nid = NativeId::from(self.mir_natives.len());
                    self.native_map.insert(did, nid);
                    self.def_kinds.insert(did, DefKind::Native);
                    let params: Vec<Ty> = n
                        .params
                        .iter()
                        .map(|p| self.hir_ty_to_ty(&p.ty))
                        .collect();
                    let ret = self.hir_ty_to_ty(&n.ret);
                    self.mir_natives.push(MirNative {
                        name: n.name.clone(),
                        params,
                        ret,
                        symbol: n.native_body.clone(),
                    });
                }
                HirItemKind::Const(c) => {
                    let cid = ConstId::from(self.mir_constants.len());
                    self.const_map.insert(did, cid);
                    self.def_kinds.insert(did, DefKind::Const);
                    self.mir_constants.push(MirConstDef {
                        name: c.name.clone(),
                        ty: Ty::Unit,
                        value: Constant::Unit,
                    });
                }
                HirItemKind::Struct(s) => {
                    self.def_kinds.insert(did, DefKind::Struct);
                    self.scan_struct(did, s);
                }
                HirItemKind::Enum(e) => {
                    self.scan_enum(did, e);
                }
            }
        }

        let adts: Vec<(DefId, AdtDef)> = self.mir_tcx.adts.iter()
            .map(|(did, adt)| (*did, adt.clone()))
            .collect();
        for (def_id, adt) in adts {
            self.generate_drop_glue(def_id, &adt);
        }
    }

    fn scan_struct(&mut self, did: DefId, s: &HirStructDef) {
        for f in &s.fields {
            self.field_indices.insert((did, f.name.clone()), f.index);
        }
        let fields: Vec<Ty> =
            s.fields.iter().map(|f| self.hir_ty_to_ty(&f.ty)).collect();
        self.mir_tcx.adts.insert(
            did,
            AdtDef {
                name: s.name.clone(),
                variants: vec![crow_mir::VariantDef {
                    name: s.name.clone(),
                    fields,
                }],
            },
        );
    }

    fn scan_enum(&mut self, did: DefId, e: &HirEnumDef) {
        let mut variants = Vec::new();
        for v in &e.variants {
            self.def_kinds.insert(v.def_id, DefKind::Variant);
            self.variant_info.insert(
                v.def_id,
                VariantInfo {
                    enum_def_id: did,
                    index: v.index,
                },
            );
            let fields: Vec<Ty> =
                v.fields.iter().map(|t| self.hir_ty_to_ty(t)).collect();
            variants.push(crow_mir::VariantDef {
                name: v.name.clone(),
                fields,
            });
        }
        self.mir_tcx.adts.insert(
            did,
            AdtDef {
                name: e.name.clone(),
                variants,
            },
        );
    }

    fn lower_all_bodies(&mut self) {
        let fn_items: Vec<(
            usize,
            BodyId,
            Vec<HirParam>,
            Vec<HirGenericParam>,
        )> = self
            .hir
            .items
            .vec()
            .iter()
            .filter_map(|item| {
                if let HirItemKind::Fun(f) = &item.kind {
                    let body_id = f.body?;
                    let fn_id = self.fn_map[&item.def_id];
                    Some((
                        fn_id.index(),
                        body_id,
                        f.params.clone(),
                        f.type_params.clone(),
                    ))
                } else {
                    None
                }
            })
            .collect();

        for (idx, body_id, params, type_params) in fn_items {
            let body = self.hir.body(body_id);
            let tc = &self.typeck.bodies[body_id.0 as usize];
            let mir_body =
                self.lower_fn_body(body, tc, &params, &type_params);
            self.mir_functions[idx] = mir_body;
        }

        let const_items: Vec<(usize, BodyId, String)> = self
            .hir
            .items
            .vec()
            .iter()
            .filter_map(|item| {
                if let HirItemKind::Const(c) = &item.kind {
                    let cid = self.const_map[&item.def_id];
                    Some((cid.index(), c.body, c.name.clone()))
                } else {
                    None
                }
            })
            .collect();

        for (idx, body_id, name) in const_items {
            let body = self.hir.body(body_id);
            let tc = &self.typeck.bodies[body_id.0 as usize];
            let mir_body = self.lower_const_body(body, tc, &name);
            self.mir_constants[idx].ty = mir_body.ret_ty().clone();
            self.mir_functions.push(mir_body);
        }
    }

    fn lower_fn_body(
        &self,
        hir_body: &HirBody,
        tx: &TypeckBody,
        params: &[HirParam],
        type_params: &[HirGenericParam],
    ) -> MirBody {
        let mut bb = BodyBuilder::new(self, hir_body, tx);
        let ret_ty = bb.expr_ty(hir_body.root_expr);
        bb.new_local(ret_ty, None, Mutability::Immut);
        for p in params {
            let ty = bb.local_ty(p.local_id);
            let local =
                bb.new_local(ty, Some(p.name.clone()), Mutability::Immut);
            bb.local_map.insert(p.local_id, local);
        }
        let arg_count = params.len();
        bb.start_block();
        bb.lower_expr(hir_body.root_expr, Place::local(RETURN_PLACE));
        bb.terminate(Terminator::Return);
        MirBody {
            name: self.fn_name_for_body(hir_body),
            arg_count,
            type_params: type_params.iter().map(|tp| tp.def_id).collect(),
            locals: bb.locals,
            blocks: bb.blocks,
        }
    }

    fn lower_const_body(
        &self,
        hir_body: &HirBody,
        tx: &TypeckBody,
        name: &str,
    ) -> MirBody {
        let mut bb = BodyBuilder::new(self, hir_body, tx);
        let ret_ty = bb.expr_ty(hir_body.root_expr);
        bb.new_local(ret_ty, None, Mutability::Immut);
        bb.start_block();
        bb.lower_expr(hir_body.root_expr, Place::local(RETURN_PLACE));
        bb.terminate(Terminator::Return);
        MirBody {
            name: name.to_string(),
            arg_count: 0,
            type_params: vec![],
            locals: bb.locals,
            blocks: bb.blocks,
        }
    }

    fn fn_name_for_body(&self, body: &HirBody) -> String {
        for item in self.hir.items.vec() {
            if let HirItemKind::Fun(f) = &item.kind {
                if f.body == Some(body.id) {
                    return f.name.clone();
                }
            }
        }
        format!("anon_{}", body.id.0)
    }
}

struct BodyBuilder<'a, 'hir> {
    lcx: &'a LoweringCtxt<'hir>,
    hir_body: &'hir HirBody,
    tx: &'a TypeckBody,

    locals: Vec<LocalDecl>,
    blocks: Vec<BasicBlock>,
    current: Block,
    local_map: HashMap<LocalId, Local>,
}

impl<'a, 'hir> BodyBuilder<'a, 'hir> {
    fn new(
        lcx: &'a LoweringCtxt<'hir>,
        hir_body: &'hir HirBody,
        tx: &'a TypeckBody,
    ) -> Self {
        Self {
            lcx,
            hir_body,
            tx,
            locals: Vec::new(),
            blocks: Vec::new(),
            current: Block(0),
            local_map: HashMap::new(),
        }
    }

    fn expr_ty(&self, id: ExprId) -> Ty {
        self.tx
            .expr_tys
            .get(&id)
            .cloned()
            .expect("no type for expr")
    }

    fn local_ty(&self, id: LocalId) -> Ty {
        self.tx
            .local_tys
            .get(&id)
            .cloned()
            .expect("no type for local")
    }

    fn pat_ty(&self, id: PatId) -> Ty {
        self.tx.pat_tys.get(&id).cloned().expect("no type for pat")
    }

    fn new_local(
        &mut self,
        ty: Ty,
        name: Option<String>,
        mutability: Mutability,
    ) -> Local {
        let id = Local::from(self.locals.len());
        self.locals.push(LocalDecl {
            ty,
            name,
            mutability,
        });
        id
    }

    fn new_temp(&mut self, ty: Ty) -> Local {
        self.new_local(ty, None, Mutability::Immut)
    }

    fn new_block(&mut self) -> Block {
        let id = Block::from(self.blocks.len());
        self.blocks.push(BasicBlock {
            stmts: Vec::new(),
            term: Terminator::Unreachable,
        });
        id
    }

    fn start_block(&mut self) -> Block {
        let b = self.new_block();
        self.current = b;
        b
    }

    fn push_stmt(&mut self, stmt: Statement) {
        self.blocks[self.current.index()].stmts.push(stmt);
    }

    fn push_assign(&mut self, place: Place, rvalue: Rvalue) {
        self.push_stmt(Statement::Assign(place, rvalue));
    }

    fn terminate(&mut self, term: Terminator) {
        self.blocks[self.current.index()].term = term;
    }

    fn as_operand(&mut self, expr_id: ExprId) -> Operand {
        let kind = self.hir_body.expr(expr_id).kind.clone();
        match kind {
            HirExprKind::Lit(ref lit) => {
                let ty = self.expr_ty(expr_id);
                lower_lit(lit, &ty)
            }
            HirExprKind::Var(ref res) => {
                self.lower_var_operand(res, expr_id)
            }
            _ => {
                let ty = self.expr_ty(expr_id);
                let tmp = self.new_temp(ty);
                self.lower_expr(expr_id, Place::local(tmp));
                Operand::Copy(Place::local(tmp))
            }
        }
    }

    fn as_place(&mut self, expr_id: ExprId) -> Place {
        let kind = self.hir_body.expr(expr_id).kind.clone();
        match kind {
            HirExprKind::Var(Res::Local(lid)) => {
                Place::local(self.local_map[&lid])
            }
            HirExprKind::Field(base_id, ref name) => {
                let mut place = self.as_place(base_id);
                let base_ty = self.expr_ty(base_id);
                let fidx = self.field_index(&base_ty, name);
                place.proj.push(Projection::Field(fidx));
                place
            }
            _ => {
                let ty = self.expr_ty(expr_id);
                let tmp = self.new_temp(ty);
                self.lower_expr(expr_id, Place::local(tmp));
                Place::local(tmp)
            }
        }
    }

    fn field_index(&self, ty: &Ty, name: &str) -> u32 {
        match ty {
            Ty::Adt(did, _) => *self
                .lcx
                .field_indices
                .get(&(*did, name.to_string()))
                .unwrap_or_else(|| panic!("no field `{name}` on {did:?}")),
            _ => panic!("field access on non-ADT type {ty:?}"),
        }
    }

    fn lower_var_operand(
        &mut self,
        res: &Res,
        expr_id: ExprId,
    ) -> Operand {
        match res {
            Res::Local(lid) => {
                Operand::Copy(Place::local(self.local_map[lid]))
            }
            Res::Def(_, did) => match self.lcx.def_kinds.get(did) {
                Some(DefKind::Fn) => {
                    let fn_id = self.lcx.fn_map[did];
                    let substs = self
                        .tx
                        .expr_substs
                        .get(&expr_id)
                        .cloned()
                        .unwrap_or_default();
                    Operand::Const(Constant::Fn(fn_id, substs))
                }
                Some(DefKind::Native) => {
                    let nid = self.lcx.native_map[did];
                    Operand::Const(Constant::Native(nid))
                }
                Some(DefKind::Const) => {
                    let cid = self.lcx.const_map[did];
                    Operand::Const(Constant::Global(cid))
                }
                Some(DefKind::Struct) => {
                    let ty = self.expr_ty(expr_id);
                    let tmp = self.new_temp(ty);
                    self.push_assign(
                        Place::local(tmp),
                        Rvalue::Aggregate(
                            AggregateKind::Adt {
                                def_id: *did,
                                variant: 0,
                            },
                            vec![],
                        ),
                    );
                    Operand::Copy(Place::local(tmp))
                }
                Some(DefKind::Variant) => {
                    let vi = &self.lcx.variant_info[did];
                    let ty = self.expr_ty(expr_id);
                    let tmp = self.new_temp(ty);
                    self.push_assign(
                        Place::local(tmp),
                        Rvalue::Aggregate(
                            AggregateKind::Adt {
                                def_id: vi.enum_def_id,
                                variant: vi.index,
                            },
                            vec![],
                        ),
                    );
                    Operand::Copy(Place::local(tmp))
                }
                None => panic!("unknown DefId {did:?}"),
            },
            Res::Err => panic!("Res::Err in lowering"),
        }
    }

    fn lower_expr(&mut self, expr_id: ExprId, dest: Place) {
        let kind = self.hir_body.expr(expr_id).kind.clone();
        match kind {
            HirExprKind::Cast { expr, .. } => {
                let inner_op = self.as_operand(expr);
                let from_ty = self.expr_ty(expr);
                let to_ty = self.expr_ty(expr_id);
                let kind = match (&from_ty, &to_ty) {
                    (Ty::Int(_), Ty::Int(_))     => CastKind::IntToInt,
                    (Ty::Int(_), Ty::Float(_))   => CastKind::IntToFloat,
                    (Ty::Float(_), Ty::Int(_))   => CastKind::FloatToInt,
                    (Ty::Float(_), Ty::Float(_)) => CastKind::FloatToFloat,
                    (Ty::Bool, Ty::Int(_))       => CastKind::IntToInt,
                    (Ty::Int(_), Ty::Bool)       => CastKind::IntToInt,
                    _ => panic!("invalid cast should be caught by typeck"),
                };
                self.push_assign(dest, Rvalue::Cast(kind, inner_op, to_ty));
            }
            HirExprKind::Rec {
                ref res,
                ref fields,
            } => {
                if let Res::Def(_, did) = res {
                    let mut sorted: Vec<(u32, ExprId)> = fields
                        .iter()
                        .map(|(name, eid)| {
                            let idx = *self
                                .lcx
                                .field_indices
                                .get(&(*did, name.clone()))
                                .unwrap_or_else(|| {
                                    panic!("no field `{name}` on {did:?}")
                                });
                            (idx, *eid)
                        })
                        .collect();
                    sorted.sort_by_key(|(idx, _)| *idx);

                    let ops: Vec<Operand> = sorted
                        .iter()
                        .map(|(_, eid)| self.as_operand(*eid))
                        .collect();

                    self.push_assign(
                        dest,
                        Rvalue::Aggregate(
                            AggregateKind::Adt {
                                def_id: *did,
                                variant: 0,
                            },
                            ops,
                        ),
                    );
                }
            }
            HirExprKind::Lit(ref lit) => {
                let ty = self.expr_ty(expr_id);
                self.push_assign(dest, Rvalue::Use(lower_lit(lit, &ty)));
            }

            HirExprKind::Var(ref res) => {
                let op = self.lower_var_operand(res, expr_id);
                self.push_assign(dest, Rvalue::Use(op));
            }

            HirExprKind::Unary(inner, ref op) => {
                let operand = self.as_operand(inner);
                self.push_assign(
                    dest,
                    Rvalue::UnaryOp(lower_unop(op), operand),
                );
            }

            HirExprKind::Bin(lhs, rhs, ref op) => {
                self.lower_binop(lhs, rhs, op, dest);
            }

            HirExprKind::Assign(lhs, rhs) => {
                let rhs_op = self.as_operand(rhs);
                let lhs_place = self.as_place(lhs);
                self.push_assign(lhs_place, Rvalue::Use(rhs_op));
                self.push_assign(
                    dest,
                    Rvalue::Use(Operand::Const(Constant::Unit)),
                );
            }

            HirExprKind::If(cond, then_e, else_e) => {
                self.lower_if(cond, then_e, else_e, dest);
            }

            HirExprKind::Field(base, ref name) => {
                let mut src = self.as_place(base);
                let base_ty = self.expr_ty(base);
                src.proj.push(Projection::Field(
                    self.field_index(&base_ty, name),
                ));
                self.push_assign(dest, Rvalue::Use(Operand::Copy(src)));
            }

            HirExprKind::Call(callee_id, ref args) => {
                self.lower_call(callee_id, args, dest);
            }

            HirExprKind::Lambda { ref params, body } => {
                self.lower_lambda(expr_id, params, body, dest);
            }

            HirExprKind::Match {
                ref subjects,
                ref arms,
            } => {
                self.lower_match(subjects, arms, dest);
            }

            HirExprKind::Block(ref stmt_ids) => {
                self.lower_block(stmt_ids, dest);
            }

            HirExprKind::Diverge(kind, arg) => {
                self.lower_diverge(kind, arg);
            }
        }
    }

    fn lower_if(
        &mut self,
        cond: ExprId,
        then_e: ExprId,
        else_e: Option<ExprId>,
        dest: Place,
    ) {
        let cond_op = self.as_operand(cond);
        let bb_then = self.new_block();
        let bb_else = self.new_block();
        let bb_join = self.new_block();

        self.terminate(Terminator::SwitchInt {
            discr: cond_op,
            targets: vec![(0, bb_else)],
            otherwise: bb_then,
        });

        self.current = bb_then;
        self.lower_expr(then_e, dest.clone());
        self.terminate(Terminator::Goto(bb_join));

        self.current = bb_else;
        match else_e {
            Some(e) => self.lower_expr(e, dest),
            None => self.push_assign(
                dest,
                Rvalue::Use(Operand::Const(Constant::Unit)),
            ),
        }
        self.terminate(Terminator::Goto(bb_join));

        self.current = bb_join;
    }

    fn lower_concat(&mut self, lhs: ExprId, rhs: ExprId, dest: Place) {
        let lhs_ty = self.expr_ty(lhs);
        let rhs_ty = self.expr_ty(rhs);

        if lhs_ty != Ty::String || rhs_ty != Ty::String {
            panic!(
                "runtime string concatenation not yet supported (requires Concat protocol). Got: {} ++ {}",
                lhs_ty, rhs_ty
            );
        }

        let lhs_op = self.as_operand(lhs);
        let rhs_op = self.as_operand(rhs);
        self.push_assign(dest, Rvalue::BinaryOp(BinOp::Add, lhs_op, rhs_op));
    }

    fn lower_binop(
        &mut self,
        lhs: ExprId,
        rhs: ExprId,
        op: &AstBinOp,
        dest: Place,
    ) {
        if is_logical_and(op) {
            return self.lower_short_circuit(lhs, rhs, true, dest);
        }
        if is_logical_or(op) {
            return self.lower_short_circuit(lhs, rhs, false, dest);
        }

        if matches!(op, AstBinOp::Concat) {
            return self.lower_concat(lhs, rhs, dest);
        }

        let mir_op = lower_arith_binop(op);
        let lhs_op = self.as_operand(lhs);
        let rhs_op = self.as_operand(rhs);

        if mir_op == BinOp::Div || mir_op == BinOp::Rem {
            let zero = zero_constant(&self.expr_ty(rhs));
            let is_zero = self.new_temp(Ty::Bool);
            self.push_assign(
                Place::local(is_zero),
                Rvalue::BinaryOp(
                    BinOp::Eq,
                    rhs_op.clone(),
                    Operand::Const(zero),
                ),
            );
            let bb_ok = self.new_block();
            let msg = if mir_op == BinOp::Div {
                crow_mir::AssertMsg::DivisionByZero
            } else {
                crow_mir::AssertMsg::RemainderByZero
            };
            self.terminate(Terminator::Assert {
                cond: Operand::Copy(Place::local(is_zero)),
                expected: false,
                msg,
                target: bb_ok,
            });
            self.current = bb_ok;
        }

        self.push_assign(dest, Rvalue::BinaryOp(mir_op, lhs_op, rhs_op));
    }

    fn lower_short_circuit(
        &mut self,
        lhs: ExprId,
        rhs: ExprId,
        is_and: bool,
        dest: Place,
    ) {
        let lhs_op = self.as_operand(lhs);
        let bb_rhs = self.new_block();
        let bb_short = self.new_block();
        let bb_join = self.new_block();

        if is_and {
            self.terminate(Terminator::SwitchInt {
                discr: lhs_op,
                targets: vec![(0, bb_short)],
                otherwise: bb_rhs,
            });
        } else {
            self.terminate(Terminator::SwitchInt {
                discr: lhs_op,
                targets: vec![(0, bb_rhs)],
                otherwise: bb_short,
            });
        }

        self.current = bb_short;
        self.push_assign(
            dest.clone(),
            Rvalue::Use(Operand::Const(Constant::Bool(!is_and))),
        );
        self.terminate(Terminator::Goto(bb_join));

        self.current = bb_rhs;
        self.lower_expr(rhs, dest);
        self.terminate(Terminator::Goto(bb_join));

        self.current = bb_join;
    }

    fn lower_call(
        &mut self,
        callee_id: ExprId,
        args: &[ExprId],
        dest: Place,
    ) {
        let callee_kind = self.hir_body.expr(callee_id).kind.clone();
        if let HirExprKind::Var(Res::Def(_, did)) = &callee_kind {
            match self.lcx.def_kinds.get(did) {
                Some(DefKind::Struct) => {
                    let ops: Vec<Operand> =
                        args.iter().map(|&a| self.as_operand(a)).collect();
                    self.push_assign(
                        dest,
                        Rvalue::Aggregate(
                            AggregateKind::Adt {
                                def_id: *did,
                                variant: 0,
                            },
                            ops,
                        ),
                    );
                    return;
                }
                Some(DefKind::Variant) => {
                    let vi = &self.lcx.variant_info[did];
                    let ops: Vec<Operand> =
                        args.iter().map(|&a| self.as_operand(a)).collect();
                    self.push_assign(
                        dest,
                        Rvalue::Aggregate(
                            AggregateKind::Adt {
                                def_id: vi.enum_def_id,
                                variant: vi.index,
                            },
                            ops,
                        ),
                    );
                    return;
                }
                _ => {}
            }
        }

        let func = self.as_operand(callee_id);
        let arg_ops: Vec<Operand> =
            args.iter().map(|&a| self.as_operand(a)).collect();

        let ret_ty = self.expr_ty(callee_id);
        let is_never = if let Ty::Fn(_, ret) = &ret_ty {
            **ret == Ty::Never
        } else {
            false
        };

        let bb_ret = self.new_block();
        self.terminate(Terminator::Call {
            func,
            args: arg_ops,
            dest,
            target: bb_ret,
        });
        self.current = bb_ret;

        if is_never {
            self.terminate(Terminator::Unreachable);
            let dead = self.new_block();
            self.current = dead;
        }
    }

    fn lower_block(&mut self, stmt_ids: &[StmtId], dest: Place) {
        if stmt_ids.is_empty() {
            self.push_assign(
                dest,
                Rvalue::Use(Operand::Const(Constant::Unit)),
            );
            return;
        }

        let (last_id, init_ids) = stmt_ids.split_last().unwrap();

        for &sid in init_ids {
            self.lower_stmt(sid);
        }

        let last_kind = self.hir_body.stmt(*last_id).kind.clone();
        match last_kind {
            HirStmtKind::Expr(eid) => self.lower_expr(eid, dest),
            _ => {
                self.lower_stmt(*last_id);
                self.push_assign(
                    dest,
                    Rvalue::Use(Operand::Const(Constant::Unit)),
                );
            }
        }
    }

    fn lower_stmt(&mut self, sid: StmtId) {
        let kind = self.hir_body.stmt(sid).kind.clone();
        match kind {
            HirStmtKind::Binding {
                local_id,
                ref name,
                init,
                mutability,
                ..
            } => {
                let ty = self.local_ty(local_id);
                let local =
                    self.new_local(ty, Some(name.clone()), mutability);
                self.local_map.insert(local_id, local);
                self.push_stmt(Statement::StorageLive(local));
                self.lower_expr(init, Place::local(local));
            }
            HirStmtKind::Wildcard { init, .. } => {
                let ty = self.expr_ty(init);
                let tmp = self.new_temp(ty);
                self.lower_expr(init, Place::local(tmp));
                self.push_stmt(Statement::StorageDead(tmp));
            }
            HirStmtKind::Expr(eid) => {
                let ty = self.expr_ty(eid);
                let tmp = self.new_temp(ty);
                self.lower_expr(eid, Place::local(tmp));
            }
        }
    }

    fn lower_diverge(&mut self, _kind: DivergeKind, arg: Option<ExprId>) {
        if let Some(a) = arg {
            let ty = self.expr_ty(a);
            let tmp = self.new_temp(ty);
            self.lower_expr(a, Place::local(tmp));
        }

        if let Some(panic_def) = self.lcx.lang_items.lang.panic {
            let fn_id = self.lcx.fn_map[&panic_def];
            let bb_unreach = self.new_block();
            let never_tmp = self.new_temp(Ty::Never);
            self.terminate(Terminator::Call {
                func: Operand::Const(Constant::Fn(fn_id, vec![])),
                args: vec![],
                dest: Place::local(never_tmp),
                target: bb_unreach,
            });
            self.current = bb_unreach;
        }

        self.terminate(Terminator::Unreachable);
        let dead = self.new_block();
        self.current = dead;
    }

    fn lower_match(
        &mut self,
        subjects: &[ExprId],
        arms: &[HirArm],
        dest: Place,
    ) {
        let subject_places: Vec<Place> = subjects
            .iter()
            .map(|&s| {
                let ty = self.expr_ty(s);
                let tmp = self.new_temp(ty);
                self.lower_expr(s, Place::local(tmp));
                Place::local(tmp)
            })
            .collect();

        let bb_join = self.new_block();

        for (i, arm) in arms.iter().enumerate() {
            let bb_body = self.new_block();
            let bb_next = self.new_block();

            self.compile_arm_patterns(
                &arm.pats,
                &subject_places,
                0,
                bb_body,
                bb_next,
            );

            self.current = bb_body;
            self.lower_expr(arm.body, dest.clone());
            self.terminate(Terminator::Goto(bb_join));

            self.current = bb_next;
            if i + 1 == arms.len() {
                let f = self.new_temp(Ty::Bool);
                self.push_assign(
                    Place::local(f),
                    Rvalue::Use(Operand::Const(Constant::Bool(false))),
                );
                self.terminate(Terminator::Assert {
                    cond: Operand::Copy(Place::local(f)),
                    expected: true,
                    msg: crow_mir::AssertMsg::MatchFailed,
                    target: bb_join,
                });
            }
        }

        self.current = bb_join;
    }

    fn compile_arm_patterns(
        &mut self,
        pats: &[PatId],
        scrutinees: &[Place],
        idx: usize,
        success: Block,
        failure: Block,
    ) {
        if idx >= pats.len() {
            self.terminate(Terminator::Goto(success));
            return;
        }
        let next = if idx + 1 < pats.len() {
            self.new_block()
        } else {
            success
        };
        self.compile_pattern(
            pats[idx],
            scrutinees[idx].clone(),
            next,
            failure,
        );
        if idx + 1 < pats.len() {
            self.current = next;
            self.compile_arm_patterns(
                pats,
                scrutinees,
                idx + 1,
                success,
                failure,
            );
        }
    }

    fn compile_pattern(
        &mut self,
        pat_id: PatId,
        scrutinee: Place,
        success: Block,
        failure: Block,
    ) {
        let kind = self.hir_body.pat(pat_id).kind.clone();
        match kind {
            HirPatKind::Wildcard => {
                self.terminate(Terminator::Goto(success));
            }

            HirPatKind::Bind(local_id, ref name) => {
                let ty = self.pat_ty(pat_id);
                let local = self.new_local(
                    ty,
                    Some(name.clone()),
                    Mutability::Immut,
                );
                self.local_map.insert(local_id, local);
                self.push_assign(
                    Place::local(local),
                    Rvalue::Use(Operand::Copy(scrutinee)),
                );
                self.terminate(Terminator::Goto(success));
            }

            HirPatKind::Lit(ref lit) => {
                let ty = self.pat_ty(pat_id);
                let lit_op = lower_lit(lit, &ty);
                let eq = self.new_temp(Ty::Bool);
                self.push_assign(
                    Place::local(eq),
                    Rvalue::BinaryOp(
                        BinOp::Eq,
                        Operand::Copy(scrutinee),
                        lit_op,
                    ),
                );
                self.terminate(Terminator::SwitchInt {
                    discr: Operand::Copy(Place::local(eq)),
                    targets: vec![(0, failure)],
                    otherwise: success,
                });
            }

            HirPatKind::Variant(ref res) => {
                let vidx = self.variant_index(res);
                let discr = self.new_temp(Ty::Int(IntTy::I64));
                self.push_assign(
                    Place::local(discr),
                    Rvalue::Discriminant(scrutinee),
                );
                self.terminate(Terminator::SwitchInt {
                    discr: Operand::Copy(Place::local(discr)),
                    targets: vec![(vidx as u128, success)],
                    otherwise: failure,
                });
            }

            HirPatKind::Unpack(ref res, ref sub_pats) => {
                let vidx = self.variant_index(res);
                let discr = self.new_temp(Ty::Int(IntTy::I64));
                self.push_assign(
                    Place::local(discr),
                    Rvalue::Discriminant(scrutinee.clone()),
                );

                if sub_pats.is_empty() {
                    self.terminate(Terminator::SwitchInt {
                        discr: Operand::Copy(Place::local(discr)),
                        targets: vec![(vidx as u128, success)],
                        otherwise: failure,
                    });
                } else {
                    let bb_fields = self.new_block();
                    self.terminate(Terminator::SwitchInt {
                        discr: Operand::Copy(Place::local(discr)),
                        targets: vec![(vidx as u128, bb_fields)],
                        otherwise: failure,
                    });
                    self.current = bb_fields;
                    let mut base = scrutinee;
                    base.proj.push(Projection::Downcast(vidx));
                    self.compile_sub_patterns(
                        sub_pats, &base, 0, success, failure,
                    );
                }
            }

            HirPatKind::Or(ref alts) => {
                for (i, &alt_id) in alts.iter().enumerate() {
                    let next_alt = if i + 1 < alts.len() {
                        self.new_block()
                    } else {
                        failure
                    };
                    self.compile_pattern(
                        alt_id,
                        scrutinee.clone(),
                        success,
                        next_alt,
                    );
                    if i + 1 < alts.len() {
                        self.current = next_alt;
                    }
                }
            }
        }
    }

    fn compile_sub_patterns(
        &mut self,
        pats: &[PatId],
        base: &Place,
        field_idx: u32,
        success: Block,
        failure: Block,
    ) {
        if field_idx as usize >= pats.len() {
            self.terminate(Terminator::Goto(success));
            return;
        }
        let mut field_place = base.clone();
        field_place.proj.push(Projection::Field(field_idx));
        let next = if ((field_idx + 1) as usize) < pats.len() {
            self.new_block()
        } else {
            success
        };
        self.compile_pattern(
            pats[field_idx as usize],
            field_place,
            next,
            failure,
        );
        if ((field_idx + 1) as usize) < pats.len() {
            self.current = next;
            self.compile_sub_patterns(
                pats,
                base,
                field_idx + 1,
                success,
                failure,
            );
        }
    }

    fn variant_index(&self, res: &Res) -> u32 {
        match res {
            Res::Def(_, did) => self
                .lcx
                .variant_info
                .get(did)
                .map(|vi| vi.index)
                .unwrap_or(0),
            _ => panic!("expected variant Res::Def"),
        }
    }

    fn lower_lambda(
        &mut self,
        expr_id: ExprId,
        params: &[HirParam],
        body_expr: ExprId,
        dest: Place,
    ) {
        let bound: HashSet<LocalId> =
            params.iter().map(|p| p.local_id).collect();
        let free_vars =
            collect_free_vars(self.hir_body, body_expr, &bound);

        let env_def_id =
            DefId(u32::MAX - self.lcx.mir_tcx.adts.len() as u32);
        let env_fields: Vec<Ty> = free_vars
            .iter()
            .map(|(lid, _)| self.local_ty(*lid))
            .collect();
        let env_adt = AdtDef {
            name: format!("closure_env_{}", expr_id.0),
            variants: vec![crow_mir::VariantDef {
                name: "env".to_string(),
                fields: env_fields,
            }],
        };

        let closure_fn_id = FnId::from(self.lcx.mir_functions.len() + 1);

        let capture_ops: Vec<Operand> = free_vars
            .iter()
            .map(|(lid, _)| {
                Operand::Copy(Place::local(self.local_map[lid]))
            })
            .collect();

        self.push_assign(
            dest,
            Rvalue::Aggregate(
                AggregateKind::Closure {
                    fn_id: closure_fn_id,
                    env_def: env_def_id,
                },
                capture_ops,
            ),
        );

        // env_adt и closure body создаются при сборке MirModule.
        // В полной реализации: LoweringCtxt хранит Vec<PendingClosure>
        // и post-процессит их после lower_all_bodies, создавая
        // MirBody с env-параметром + маппинг captures → env.field[i].
        let _ = (env_adt, closure_fn_id, params, body_expr);
    }
}

fn collect_free_vars(
    body: &HirBody,
    root: ExprId,
    bound: &HashSet<LocalId>,
) -> Vec<(LocalId, String)> {
    let mut free = Vec::new();
    let mut seen = HashSet::new();
    let mut bound = bound.clone();
    walk_free(body, root, &mut bound, &mut free, &mut seen);
    free
}

fn walk_free(
    body: &HirBody,
    eid: ExprId,
    bound: &mut HashSet<LocalId>,
    free: &mut Vec<(LocalId, String)>,
    seen: &mut HashSet<LocalId>,
) {
    let kind = body.expr(eid).kind.clone();
    match kind {
        HirExprKind::Cast { expr, .. } => {
            walk_free(body, expr, bound, free, seen);
        }
        HirExprKind::Rec { fields, .. } => {
            for (_, eid) in fields {
                walk_free(body, eid, bound, free, seen);
            }
        }
        HirExprKind::Var(Res::Local(lid)) => {
            if !bound.contains(&lid) && seen.insert(lid) {
                let name = find_local_name(body, lid);
                free.push((lid, name));
            }
        }
        HirExprKind::Lit(_) | HirExprKind::Var(_) => {}
        HirExprKind::Unary(inner, _) => {
            walk_free(body, inner, bound, free, seen)
        }
        HirExprKind::Bin(l, r, _) | HirExprKind::Assign(l, r) => {
            walk_free(body, l, bound, free, seen);
            walk_free(body, r, bound, free, seen);
        }
        HirExprKind::If(c, t, e) => {
            walk_free(body, c, bound, free, seen);
            walk_free(body, t, bound, free, seen);
            if let Some(e) = e {
                walk_free(body, e, bound, free, seen);
            }
        }
        HirExprKind::Field(base, _) => {
            walk_free(body, base, bound, free, seen)
        }
        HirExprKind::Call(callee, ref args) => {
            walk_free(body, callee, bound, free, seen);
            for &a in args {
                walk_free(body, a, bound, free, seen);
            }
        }
        HirExprKind::Lambda {
            ref params,
            body: lb,
        } => {
            let mut inner = bound.clone();
            for p in params {
                inner.insert(p.local_id);
            }
            walk_free(body, lb, &mut inner, free, seen);
        }
        HirExprKind::Match {
            ref subjects,
            ref arms,
        } => {
            for &s in subjects {
                walk_free(body, s, bound, free, seen);
            }
            for arm in arms {
                let mut ab = bound.clone();
                for &pid in &arm.pats {
                    collect_pat_binds(body, pid, &mut ab);
                }
                walk_free(body, arm.body, &mut ab, free, seen);
            }
        }
        HirExprKind::Block(ref sids) => {
            for &sid in sids {
                let sk = body.stmt(sid).kind.clone();
                match sk {
                    HirStmtKind::Binding { local_id, init, .. } => {
                        walk_free(body, init, bound, free, seen);
                        bound.insert(local_id);
                    }
                    HirStmtKind::Wildcard { init, .. } => {
                        walk_free(body, init, bound, free, seen);
                    }
                    HirStmtKind::Expr(eid) => {
                        walk_free(body, eid, bound, free, seen)
                    }
                }
            }
        }
        HirExprKind::Diverge(_, arg) => {
            if let Some(a) = arg {
                walk_free(body, a, bound, free, seen);
            }
        }
    }
}

fn collect_pat_binds(
    body: &HirBody,
    pid: PatId,
    bound: &mut HashSet<LocalId>,
) {
    let kind = body.pat(pid).kind.clone();
    match kind {
        HirPatKind::Bind(lid, _) => {
            bound.insert(lid);
        }
        HirPatKind::Unpack(_, ref children) => {
            for &c in children {
                collect_pat_binds(body, c, bound);
            }
        }
        HirPatKind::Or(ref alts) => {
            for &a in alts {
                collect_pat_binds(body, a, bound);
            }
        }
        _ => {}
    }
}

fn find_local_name(body: &HirBody, lid: LocalId) -> String {
    for stmt in body.stmts.vec() {
        if let HirStmtKind::Binding {
            local_id, ref name, ..
        } = stmt.kind
        {
            if local_id == lid {
                return name.clone();
            }
        }
    }
    format!("_capture_{}", lid.0)
}

fn lower_lit(lit: &Lit, ty: &Ty) -> Operand {
    match lit {
        Lit::Int(s) => {
            let v: i128 = s.parse().expect("invalid int literal");
            let ity = match ty {
                Ty::Int(i) => *i,
                _ => IntTy::I64,
            };
            Operand::Const(Constant::Int(v, ity))
        }
        Lit::Float(s) => {
            let v: f64 = s.parse().expect("invalid float literal");
            let fty = match ty {
                Ty::Float(f) => *f,
                _ => FloatTy::F64,
            };
            Operand::Const(Constant::Float(v, fty))
        }
        Lit::Bool(s) => Operand::Const(Constant::Bool(s == "true")),
        Lit::String(s) => Operand::Const(Constant::Str(s.clone())),
        Lit::None => Operand::Const(Constant::Unit),
    }
}

fn lower_unop(op: &AstUnOp) -> UnOp {
    match op {
        AstUnOp::Bang => UnOp::Not,
        AstUnOp::Neg => UnOp::Neg,
    }
}

fn lower_arith_binop(op: &AstBinOp) -> BinOp {
    match op {
        AstBinOp::Add => BinOp::Add,
        AstBinOp::Sub => BinOp::Sub,
        AstBinOp::Mul => BinOp::Mul,
        AstBinOp::Div => BinOp::Div,
        AstBinOp::Rem => BinOp::Rem,
        AstBinOp::Eq => BinOp::Eq,
        AstBinOp::Ne => BinOp::Ne,
        AstBinOp::Lt => BinOp::Lt,
        AstBinOp::Le => BinOp::Le,
        AstBinOp::Gt => BinOp::Gt,
        AstBinOp::Ge => BinOp::Ge,
        AstBinOp::BitAnd => BinOp::BitAnd,
        AstBinOp::BitOr => BinOp::BitOr,
        _ => panic!("unexpected binop in arith lowering"),
    }
}

fn is_logical_and(op: &AstBinOp) -> bool {
    matches!(op, AstBinOp::And)
}
fn is_logical_or(op: &AstBinOp) -> bool {
    matches!(op, AstBinOp::Or)
}

fn zero_constant(ty: &Ty) -> Constant {
    match ty {
        Ty::Int(ity) => Constant::Int(0, *ity),
        Ty::Float(fty) => Constant::Float(0.0, *fty),
        _ => Constant::Int(0, IntTy::I64),
    }
}

pub fn lower_hir_to_mir(hir: &Hir, typeck: &TypeckOutput, spec_items: &SpecialItems) -> MirModule {
    let mut lcx = LoweringCtxt::new(hir, typeck, spec_items);
    lcx.lower_all_bodies();

    MirModule {
        tcx: lcx.mir_tcx,
        functions: lcx.mir_functions,
        natives: lcx.mir_natives,
        constants: lcx.mir_constants,
        lang: LangDefs {
            gc: match (lcx.lang_rc_alloc, lcx.lang_rc_retain, lcx.lang_rc_release) {
                (Some(a), Some(ret), Some(rel)) => Some(GcDefs {
                    rc_alloc: a,
                    retain: ret,
                    release: rel,
                    drop_glue: lcx.drop_glue.clone(),
                }),
                _ => None,
            },
            panic: lcx.lang_panic,
            intrinsics: lcx.mir_intrin_fns.iter()
                .map(|(id, info)| (*id, info.name.clone()))
                .collect(),
        },
    }
}
