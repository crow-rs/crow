use crow_ast::atom::{BinOp, UnOp};
use crow_ir::*;
use crow_tycheck::{
    hir::*, typ::{IntBitness, Typ},
};
use std::collections::HashMap;

pub struct LowerCtxt {
    structs: Vec<MirStructDef>,
    enums: Vec<MirEnumDef>,
    functions: Vec<MirFunction>,
    entry: Option<MirFunctionId>,

    fn_ids: HashMap<String, MirFunctionId>,
}

impl LowerCtxt {
    pub fn new() -> Self {
        Self {
            structs: Vec::new(),
            enums: Vec::new(),
            functions: Vec::new(),
            entry: None,
            fn_ids: HashMap::new(),
        }
    }

    pub fn lower(mut self, hir: &HirModule) -> MirModule {
        for (i, f) in hir.functions.iter().enumerate() {
            let id = i as MirFunctionId;
            self.fn_ids.insert(f.name.clone(), id);
            if f.name == "main" {
                self.entry = Some(id);
            }
        }
        for (i, n) in hir.natives.iter().enumerate() {
            let id = (hir.functions.len() + i) as MirFunctionId;
            self.fn_ids.insert(n.name.clone(), id);
        }

        for s in &hir.structs {
            self.structs.push(MirStructDef {
                name: s.name.clone(),
                fields: s
                    .fields
                    .iter()
                    .map(|(name, ty)| MirFieldDef {
                        name: name.clone(),
                        ty: self.lower_typ(ty),
                    })
                    .collect(),
            });
        }

        for e in &hir.enums {
            self.enums.push(MirEnumDef {
                name: e.name.clone(),
                variants: e
                    .variants
                    .iter()
                    .map(|v| MirVariantDef {
                        name: v.name.clone(),
                        fields: v
                            .fields
                            .iter()
                            .map(|t| self.lower_typ(t))
                            .collect(),
                    })
                    .collect(),
            });
        }

        for f in &hir.functions {
            let mir_fn = self.lower_function(f);
            self.functions.push(mir_fn);
        }

        for n in &hir.natives {
            self.functions.push(MirFunction {
                name: n.name.clone(),
                params: n.params.iter().map(|(_, ty)| self.lower_typ(ty)).collect(),
                ret: self.lower_typ(&n.ret),
                body: None,
            });
        }

        MirModule {
            structs: self.structs,
            enums: self.enums,
            functions: self.functions,
            entry: self.entry.unwrap_or(0),
        }
    }

    fn lower_function(&self, f: &HirFunction) -> MirFunction {
        let ret_ty = self.lower_typ(&f.ret);
        let param_tys: Vec<MirType> =
            f.params.iter().map(|(_, ty)| self.lower_typ(ty)).collect();

        let mut fb = FnBuilder::new(&ret_ty, &param_tys, &f.params, self);
        let result = fb.lower_expr(&f.body);

        // Store result in _0
        match &ret_ty {
            MirType::Unit | MirType::Never => {}
            _ => {
                fb.emit_assign(
                    MirPlace::local(0),
                    MirRvalue::Use(MirOperand::Copy(MirPlace::local(result))),
                );
            }
        }

        fb.terminate(MirTerminator::Return);

        MirFunction {
            name: f.name.clone(),
            params: param_tys,
            ret: ret_ty,
            body: Some(fb.build(f.name.clone())),
        }
    }

    fn translate_int_type(&self, ty_bitn: &IntBitness) -> MirIntBitness {
        match ty_bitn {
            IntBitness::I8 => MirIntBitness::Bit8,
            IntBitness::I16 => MirIntBitness::Bit16,
            IntBitness::I32 => MirIntBitness::Bit32,
            IntBitness::I64 => MirIntBitness::Bit64
        }
    }

    //DO NOT IGNORE MACHINE BITNESS
    fn lower_typ(&self, ty: &Typ) -> MirType {
        match ty {
            Typ::Int(bitness) => MirType::Int(self.translate_int_type(bitness)),
            Typ::Float => MirType::Float,
            Typ::Bool => MirType::Bool,
            Typ::Str => MirType::Str,
            Typ::Unit => MirType::Unit,
            Typ::Error => MirType::Never,
            Typ::Struct(_, _) => MirType::Struct(0), // TODO: id mapping
            Typ::Enum(_, _) => MirType::Enum(0),     // TODO: id mapping
            /*Typ::Fun(def, _, _)  => {
                // Function values are pointers
                MirType::FunPtr {
                    params: def
                    ret: Box::new(MirType::Int),
                }
            }*/
            //| Typ::FunRef(_, _, _)
            _ => panic!("Invalid machine type provided!")
        }
    }

    fn resolve_fn(&self, name: &str) -> Option<MirFunctionId> {
        self.fn_ids.get(name).copied()
    }
}

struct FnBuilder<'a> {
    lcx: &'a LowerCtxt,
    locals: Vec<MirLocal>,
    blocks: Vec<MirBasicBlock>,
    current_stmts: Vec<MirStatement>,
    arg_count: usize,
    named_locals: HashMap<String, MirLocalId>,
}

impl<'a> FnBuilder<'a> {
    fn new(
        ret_ty: &MirType,
        param_tys: &[MirType],
        params: &[(String, Typ)],
        lcx: &'a LowerCtxt,
    ) -> Self {
        let mut locals = Vec::new();
        let mut named_locals = HashMap::new();

        locals.push(MirLocal {
            ty: ret_ty.clone(),
            kind: MirLocalKind::Return,
            name: Some("_0".to_string()),
        });

        for (i, (ty, (name, _))) in
            param_tys.iter().zip(params.iter()).enumerate()
        {
            let id = (i + 1) as MirLocalId;
            locals.push(MirLocal {
                ty: ty.clone(),
                kind: MirLocalKind::Arg,
                name: Some(name.clone()),
            });
            named_locals.insert(name.clone(), id);
        }

        Self {
            lcx,
            locals,
            blocks: Vec::new(),
            current_stmts: Vec::new(),
            arg_count: param_tys.len(),
            named_locals,
        }
    }

    fn temp(&mut self, ty: MirType) -> MirLocalId {
        let id = self.locals.len() as MirLocalId;
        self.locals.push(MirLocal {
            ty,
            kind: MirLocalKind::Temp,
            name: None,
        });
        id
    }

    fn user_local(&mut self, name: &str, ty: MirType) -> MirLocalId {
        let id = self.locals.len() as MirLocalId;
        self.locals.push(MirLocal {
            ty,
            kind: MirLocalKind::User,
            name: Some(name.to_string()),
        });
        self.named_locals.insert(name.to_string(), id);
        id
    }

    fn emit(&mut self, stmt: MirStatement) {
        self.current_stmts.push(stmt);
    }

    fn emit_assign(&mut self, place: MirPlace, rvalue: MirRvalue) {
        self.emit(MirStatement::Assign(place, rvalue));
    }

    fn terminate(&mut self, term: MirTerminator) -> MirBlockId {
        let id = self.blocks.len() as MirBlockId;
        let stmts = std::mem::take(&mut self.current_stmts);
        self.blocks.push(MirBasicBlock {
            stmts,
            terminator: term,
        });
        id
    }

    fn next_block_id(&self) -> MirBlockId {
        self.blocks.len() as MirBlockId
    }

    fn build(mut self, name: String) -> MirBody {
        if !self.current_stmts.is_empty() {
            self.terminate(MirTerminator::Return);
        }

        MirBody {
            name,
            locals: self.locals,
            arg_count: self.arg_count,
            blocks: self.blocks,
        }
    }

    fn lower_expr(&mut self, expr: &HirExpr) -> MirLocalId {
        match &expr.kind {
            HirExprKind::Lit(lit) => self.lower_lit(lit),
            HirExprKind::Var(name) => self.lower_var(name, &expr.ty),
            HirExprKind::BinOp(op, lhs, rhs) => self.lower_binop(*op, lhs, rhs, &expr.ty),
            HirExprKind::UnOp(op, operand) => self.lower_unop(*op, operand, &expr.ty),
            HirExprKind::Assign(lhs, rhs) => self.lower_assign(lhs, rhs),
            HirExprKind::If(cond, then_br, else_br) => {
                self.lower_if(cond, then_br, else_br.as_deref(), &expr.ty)
            }
            HirExprKind::Field(obj, _name, idx) => self.lower_field(obj, *idx, &expr.ty),
            HirExprKind::Call(callee, args) => self.lower_call(callee, args, &expr.ty),
            HirExprKind::Block(stmts) => self.lower_block(stmts),
            HirExprKind::Match(scrutinees, cases) => {
                self.lower_match(scrutinees, cases, &expr.ty)
            }
            HirExprKind::Lambda(_, _) => {
                // TODO: closure conversion
                self.temp(MirType::Unit)
            }
            HirExprKind::Todo(_) | HirExprKind::Panic(_) => {
                self.terminate(MirTerminator::Abort);
                self.temp(MirType::Never)
            }
        }
    }

    //todo - literal must have type allready
    fn lower_lit(&mut self, lit: &HirLit) -> MirLocalId {
        let (ty, constant) = match lit {
            HirLit::Int(v) => (MirType::Int(MirIntBitness::Bit64), MirConstant::Int(*v, MirIntBitness::Bit64)), // todo - HIR MUST HAVE ALLREADY TYPES
            HirLit::Float(v) => (MirType::Float, MirConstant::Float(*v)),
            HirLit::Bool(v) => (MirType::Bool, MirConstant::Bool(*v)),
            HirLit::Str(s) => (MirType::Str, MirConstant::Str(s.clone())),
            HirLit::Unit => (MirType::Unit, MirConstant::Unit),
        };
        let tmp = self.temp(ty);
        self.emit_assign(
            MirPlace::local(tmp),
            MirRvalue::Use(MirOperand::Constant(constant)),
        );
        tmp
    }


    fn lower_var(&mut self, name: &str, ty: &Typ) -> MirLocalId {
        if let Some(&id) = self.named_locals.get(name) {
            return id;
        }

        if let Some(fn_id) = self.lcx.resolve_fn(name) {
            let mir_ty = self.lcx.lower_typ(ty);
            let tmp = self.temp(mir_ty);
            self.emit_assign(
                MirPlace::local(tmp),
                MirRvalue::Use(MirOperand::Constant(MirConstant::FunRef(fn_id))),
            );
            return tmp;
        }

        self.temp(self.lcx.lower_typ(ty))
    }


    fn lower_binop(
        &mut self,
        op: BinOp,
        lhs: &HirExpr,
        rhs: &HirExpr,
        result_ty: &Typ,
    ) -> MirLocalId {
        // Short-circuit: && and || lower to CFG
        if op == BinOp::And {
            return self.lower_short_circuit(lhs, rhs, true, result_ty);
        }
        if op == BinOp::Or {
            return self.lower_short_circuit(lhs, rhs, false, result_ty);
        }

        let l = self.lower_expr(lhs);
        let r = self.lower_expr(rhs);
        let mir_op = lower_binop(op);
        let tmp = self.temp(self.lcx.lower_typ(result_ty));
        self.emit_assign(
            MirPlace::local(tmp),
            MirRvalue::BinOp(
                mir_op,
                MirOperand::Copy(MirPlace::local(l)),
                MirOperand::Copy(MirPlace::local(r)),
            ),
        );
        tmp
    }

    /// Lower && / || to CFG for short-circuit evaluation
    ///
    /// `a && b`:
    ///   bb_entry: eval a → switch(a) [1: bb_rhs, otherwise: bb_join(false)]
    ///   bb_rhs:   eval b → result = b → goto bb_join
    ///   bb_join:  ...
    ///
    /// `a || b`:
    ///   bb_entry: eval a → switch(a) [1: bb_join(true), otherwise: bb_rhs]
    ///   bb_rhs:   eval b → result = b → goto bb_join
    ///   bb_join:  ...
    fn lower_short_circuit(
        &mut self,
        lhs: &HirExpr,
        rhs: &HirExpr,
        is_and: bool,
        result_ty: &Typ,
    ) -> MirLocalId {
        let result = self.temp(MirType::Bool);
        let lhs_val = self.lower_expr(lhs);

        let bb_rhs = self.next_block_id() + 1;
        let bb_join = bb_rhs + 1; // will adjust after rhs lowering

        if is_and {
            // false short-circuits to false
            self.emit_assign(
                MirPlace::local(result),
                MirRvalue::Use(MirOperand::Constant(MirConstant::Bool(false))),
            );
            let bb_join_placeholder = bb_rhs + 1;
            self.terminate(MirTerminator::SwitchInt {
                discr: MirOperand::Copy(MirPlace::local(lhs_val)),
                targets: vec![(1, bb_rhs)],
                otherwise: bb_join_placeholder,
            });
        } else {
            // true short-circuits to true
            self.emit_assign(
                MirPlace::local(result),
                MirRvalue::Use(MirOperand::Constant(MirConstant::Bool(true))),
            );
            let bb_join_placeholder = bb_rhs + 1;
            self.terminate(MirTerminator::SwitchInt {
                discr: MirOperand::Copy(MirPlace::local(lhs_val)),
                targets: vec![(1, bb_join_placeholder)],
                otherwise: bb_rhs,
            });
        }

        // bb_rhs: evaluate rhs
        let rhs_val = self.lower_expr(rhs);
        self.emit_assign(
            MirPlace::local(result),
            MirRvalue::Use(MirOperand::Copy(MirPlace::local(rhs_val))),
        );
        let actual_join = self.next_block_id() + 1;
        self.terminate(MirTerminator::Goto(actual_join));

        // bb_join: continues from here
        result
    }

    fn lower_unop(
        &mut self,
        op: UnOp,
        operand: &HirExpr,
        result_ty: &Typ,
    ) -> MirLocalId {
        let val = self.lower_expr(operand);
        let mir_op = lower_unop(op);
        let tmp = self.temp(self.lcx.lower_typ(result_ty));
        self.emit_assign(
            MirPlace::local(tmp),
            MirRvalue::UnOp(mir_op, MirOperand::Copy(MirPlace::local(val))),
        );
        tmp
    }

    fn lower_assign(&mut self, lhs: &HirExpr, rhs: &HirExpr) -> MirLocalId {
        let rhs_val = self.lower_expr(rhs);
        let place = self.expr_to_place(lhs);
        self.emit_assign(
            place,
            MirRvalue::Use(MirOperand::Copy(MirPlace::local(rhs_val))),
        );
        let unit = self.temp(MirType::Unit);
        self.emit_assign(
            MirPlace::local(unit),
            MirRvalue::Use(MirOperand::Constant(MirConstant::Unit)),
        );
        unit
    }


    fn lower_field(
        &mut self,
        obj: &HirExpr,
        idx: usize,
        result_ty: &Typ,
    ) -> MirLocalId {
        let obj_val = self.lower_expr(obj);
        let tmp = self.temp(self.lcx.lower_typ(result_ty));
        self.emit_assign(
            MirPlace::local(tmp),
            MirRvalue::Use(MirOperand::Copy(MirPlace {
                local: obj_val,
                projection: vec![MirProjection::Field(idx)],
            })),
        );
        tmp
    }

    fn lower_call(
        &mut self,
        callee: &HirExpr,
        args: &[HirExpr],
        result_ty: &Typ,
    ) -> MirLocalId {
        // Lower args first
        let arg_ops: Vec<MirOperand> = args
            .iter()
            .map(|a| {
                let id = self.lower_expr(a);
                MirOperand::Copy(MirPlace::local(id))
            })
            .collect();

        // Resolve callee
        let func_op = match &callee.kind {
            HirExprKind::Var(name) => {
                if let Some(fn_id) = self.lcx.resolve_fn(name) {
                    MirOperand::Constant(MirConstant::FunRef(fn_id))
                } else {
                    let id = self.lower_expr(callee);
                    MirOperand::Copy(MirPlace::local(id))
                }
            }
            _ => {
                let id = self.lower_expr(callee);
                MirOperand::Copy(MirPlace::local(id))
            }
        };

        let ret_mir = self.lcx.lower_typ(result_ty);
        let result = self.temp(ret_mir);

        // Call is a terminator
        let cont = self.next_block_id() + 1;
        self.terminate(MirTerminator::Call {
            dest: MirPlace::local(result),
            func: func_op,
            args: arg_ops,
            target: cont,
        });

        result
    }

    fn lower_if(
        &mut self,
        cond: &HirExpr,
        then_br: &HirExpr,
        else_br: Option<&HirExpr>,
        result_ty: &Typ,
    ) -> MirLocalId {
        let result = self.temp(self.lcx.lower_typ(result_ty));
        let cond_val = self.lower_expr(cond);

        // bb_switch → then_bb / else_bb → join_bb
        let then_bb = self.next_block_id() + 1;

        // We don't know else_bb yet — it depends on how many blocks
        // `then` generates. Use a two-pass approach:
        // 1. Terminate switch with placeholder
        // 2. Lower then, record where else starts
        // 3. Patch is not needed if we calculate correctly

        // Terminate current block: switch on condition
        // then_bb is always next_block_id + 1 (which is right after this terminate)
        // else_bb we'll figure out after lowering then
        let switch_bb = self.terminate(MirTerminator::Unreachable); // placeholder

        // Lower then
        let then_val = self.lower_expr(then_br);
        self.emit_assign(
            MirPlace::local(result),
            MirRvalue::Use(MirOperand::Copy(MirPlace::local(then_val))),
        );
        // After then, goto join — but we don't know join yet
        let then_exit = self.terminate(MirTerminator::Unreachable); // placeholder

        // else_bb is right here
        let else_bb = self.next_block_id();

        // Lower else
        if let Some(else_expr) = else_br {
            let else_val = self.lower_expr(else_expr);
            self.emit_assign(
                MirPlace::local(result),
                MirRvalue::Use(MirOperand::Copy(MirPlace::local(else_val))),
            );
        } else {
            self.emit_assign(
                MirPlace::local(result),
                MirRvalue::Use(MirOperand::Constant(MirConstant::Unit)),
            );
        }
        let else_exit = self.terminate(MirTerminator::Unreachable); // placeholder

        // join_bb is right here
        let join_bb = self.next_block_id();

        // Patch terminators
        self.blocks[switch_bb as usize].terminator = MirTerminator::SwitchInt {
            discr: MirOperand::Copy(MirPlace::local(cond_val)),
            targets: vec![(1, then_bb)],
            otherwise: else_bb,
        };
        self.blocks[then_exit as usize].terminator = MirTerminator::Goto(join_bb);
        self.blocks[else_exit as usize].terminator = MirTerminator::Goto(join_bb);

        result
    }

    fn lower_block(&mut self, stmts: &[HirStmt]) -> MirLocalId {
        let mut last = self.temp(MirType::Unit);

        for stmt in stmts {
            match stmt {
                HirStmt::Let(name, ty, init) => {
                    let val = self.lower_expr(init);
                    let mir_ty = self.lcx.lower_typ(ty);
                    let local = self.user_local(name, mir_ty);
                    self.emit_assign(
                        MirPlace::local(local),
                        MirRvalue::Use(MirOperand::Copy(MirPlace::local(val))),
                    );
                    last = local;
                }
                HirStmt::Expr(expr) => {
                    last = self.lower_expr(expr);
                }
            }
        }

        last
    }

    fn lower_match(
        &mut self,
        scrutinees: &[HirExpr],
        cases: &[HirCase],
        result_ty: &Typ,
    ) -> MirLocalId {
        let result = self.temp(self.lcx.lower_typ(result_ty));

        // Single scrutinee for now
        let scrut = self.lower_expr(&scrutinees[0]);

        // Get discriminant (for enums); for ints/bools, use value directly
        let discr = match &scrutinees[0].ty {
            Typ::Enum(_, _) => {
                let d = self.temp(MirType::Int(MirIntBitness::Bit64)); //TODO
                self.emit_assign(
                    MirPlace::local(d),
                    MirRvalue::Discriminant(MirPlace::local(scrut)),
                );
                d
            }
            _ => scrut,
        };

        // Terminate with placeholder switch
        let switch_bb = self.terminate(MirTerminator::Unreachable);

        // Lower each case body, collect (pattern_value, block_id)
        let mut targets: Vec<(u128, MirBlockId)> = Vec::new();
        let mut otherwise: Option<MirBlockId> = None;
        let mut exit_bbs: Vec<MirBlockId> = Vec::new();

        for case in cases {
            let case_bb = self.next_block_id();

            // Bind variables from patterns
            for pat in &case.pats {
                self.bind_pattern(pat, scrut);

                match pat {
                    HirPat::Lit(HirLit::Int(v)) => {
                        targets.push((*v as u128, case_bb));
                    }
                    HirPat::Lit(HirLit::Bool(v)) => {
                        targets.push((if *v { 1 } else { 0 }, case_bb));
                    }
                    HirPat::Variant(_, idx) | HirPat::Unpack(_, idx, _) => {
                        targets.push((*idx as u128, case_bb));
                    }
                    HirPat::Wildcard | HirPat::Bind(_, _) => {
                        otherwise = Some(case_bb);
                    }
                    _ => {
                        otherwise = Some(case_bb);
                    }
                }
            }

            // Lower case body
            let body_val = self.lower_expr(&case.body);
            self.emit_assign(
                MirPlace::local(result),
                MirRvalue::Use(MirOperand::Copy(MirPlace::local(body_val))),
            );
            let exit = self.terminate(MirTerminator::Unreachable); // placeholder
            exit_bbs.push(exit);
        }

        // Join block
        let join_bb = self.next_block_id();

        // Patch switch
        let otherwise_bb = otherwise.unwrap_or(join_bb);
        self.blocks[switch_bb as usize].terminator = MirTerminator::SwitchInt {
            discr: MirOperand::Copy(MirPlace::local(discr)),
            targets,
            otherwise: otherwise_bb,
        };

        // Patch all case exits → join
        for exit in exit_bbs {
            self.blocks[exit as usize].terminator = MirTerminator::Goto(join_bb);
        }

        result
    }

    /// Bind pattern variables to locals
    fn bind_pattern(&mut self, pat: &HirPat, scrut: MirLocalId) {
        match pat {
            HirPat::Bind(name, ty) => {
                let mir_ty = self.lcx.lower_typ(ty);
                let local = self.user_local(name, mir_ty);
                self.emit_assign(
                    MirPlace::local(local),
                    MirRvalue::Use(MirOperand::Copy(MirPlace::local(scrut))),
                );
            }
            HirPat::Unpack(_enum_id, variant_idx, sub_pats) => {
                for (i, sub) in sub_pats.iter().enumerate() {
                    // Read field i from variant
                    let field_tmp = self.temp(MirType::Int(MirIntBitness::Bit64)); // TODO: actual field type
                    self.emit_assign(
                        MirPlace::local(field_tmp),
                        MirRvalue::Use(MirOperand::Copy(MirPlace {
                            local: scrut,
                            projection: vec![
                                MirProjection::Downcast(*variant_idx),
                                MirProjection::Field(i),
                            ],
                        })),
                    );
                    self.bind_pattern(sub, field_tmp);
                }
            }
            _ => {}
        }
    }

    fn expr_to_place(&self, expr: &HirExpr) -> MirPlace {
        match &expr.kind {
            HirExprKind::Var(name) => {
                if let Some(&id) = self.named_locals.get(name) {
                    MirPlace::local(id)
                } else {
                    MirPlace::local(0)
                }
            }
            HirExprKind::Field(obj, _, idx) => {
                let mut place = self.expr_to_place(obj);
                place.projection.push(MirProjection::Field(*idx));
                place
            }
            _ => MirPlace::local(0),
        }
    }
}

fn lower_binop(op: BinOp) -> MirBinOp {
    match op {
        BinOp::Add => MirBinOp::Add,
        BinOp::Sub => MirBinOp::Sub,
        BinOp::Mul => MirBinOp::Mul,
        BinOp::Div => MirBinOp::Div,
        BinOp::Rem => MirBinOp::Rem,
        BinOp::Eq => MirBinOp::Eq,
        BinOp::Ne => MirBinOp::Ne,
        BinOp::Gt => MirBinOp::Gt,
        BinOp::Ge => MirBinOp::Ge,
        BinOp::Lt => MirBinOp::Lt,
        BinOp::Le => MirBinOp::Le,
        BinOp::And => MirBinOp::And, 
        BinOp::Or => MirBinOp::Or,   
        BinOp::Xor => MirBinOp::BitXor,
        BinOp::BitAnd => MirBinOp::BitAnd,
        BinOp::BitOr => MirBinOp::BitOr,
        BinOp::Concat => MirBinOp::Add, // TODO: string intrinsic
    }
}

fn lower_unop(op: UnOp) -> MirUnOp {
    match op {
        UnOp::Neg => MirUnOp::Neg,
        UnOp::Bang => MirUnOp::Not,
    }
}

pub fn lower_to_mir(hir: &HirModule) -> MirModule {
    LowerCtxt::new().lower(hir)
}