use crow_mir::{AggregateKind, Constant, MirBody, Operand, Place, Projection, Rvalue, Statement, Terminator, UnOp};
use crow_mir_monomorph::Instance;
use crow_tycheck::ty::{FloatTy, IntTy, Ty};
use inkwell::{IntPredicate, basic_block::BasicBlock as LlvmBlock, builder::Builder, context::Context, types::BasicTypeEnum, values::{BasicMetadataValueEnum, BasicValue, BasicValueEnum, FunctionValue, IntValue, PointerValue}};

use crate::{Codegen, ops_builder::build_llvm_binop};

pub struct FnCodegen<'a, 'llvm, 'mir> {
    pub cg: &'a mut Codegen<'llvm, 'mir>,
    pub body: &'a MirBody,
    pub locals: Vec<PointerValue<'llvm>>,
    pub blocks: Vec<LlvmBlock<'llvm>>,
}

impl<'a, 'llvm, 'mir> FnCodegen<'a, 'llvm, 'mir> {
    fn ctx(&self) -> &'llvm Context { self.cg.context }
    fn builder(&self) -> &Builder<'llvm> { &self.cg.builder }

    pub fn codegen(&mut self, fv: FunctionValue<'llvm>) {
        let entry = self.ctx().append_basic_block(fv, "entry");
        self.builder().position_at_end(entry);

        for (i, decl) in self.body.locals.iter().enumerate() {
            let ty = self.cg.types.construct_type(self.cg.tcx, &decl.ty);
            let default_name = format!("_{i}");
            let name = decl.name.as_deref().unwrap_or(&default_name);
            let alloca = self.builder().build_alloca(ty, name).unwrap();
            self.locals.push(alloca);
        }

        for i in 0..self.body.arg_count {
            let arg = fv.get_nth_param(i as u32).unwrap();
            let alloca = self.locals[i + 1]; // _1 .. _N
            self.builder().build_store(alloca, arg).unwrap();
        }

        for (i, _) in self.body.blocks.iter().enumerate() {
            let bb = self.ctx().append_basic_block(fv, &format!("bb{i}"));
            self.blocks.push(bb);
        }

        self.builder().build_unconditional_branch(self.blocks[0]).unwrap();

        for (i, mir_bb) in self.body.blocks.iter().enumerate() {
            self.builder().position_at_end(self.blocks[i]);
            self.codegen_block(mir_bb);
        }
    }

    fn codegen_block(&mut self, bb: &crow_mir::BasicBlock) {
        for stmt in &bb.stmts {
            self.codegen_stmt(stmt);
        }
        self.codegen_terminator(&bb.term);
    }

    fn codegen_stmt(&mut self, stmt: &Statement) {
        match stmt {
            Statement::Assign(place, rvalue) => {
                let dest_ty = self.place_ty(place);
                let val = self.codegen_rvalue(rvalue, &dest_ty);
                let ptr = self.codegen_place_ptr(place);
                self.builder().build_store(ptr, val).unwrap();
            }
            Statement::StorageLive(_) | Statement::StorageDead(_) | Statement::Nop => {
                // Хинты для оптимизатора, LLVM справляется через mem2reg
            }
        }
    }

    fn codegen_terminator(&mut self, term: &Terminator) {
        match term {
            Terminator::Goto(target) => {
                self.builder()
                    .build_unconditional_branch(self.blocks[target.index()])
                    .unwrap();
            }

            Terminator::SwitchInt { discr, targets, otherwise } => {
                let val = self.codegen_operand(discr);
                let int_val = val.into_int_value();
                let else_bb = self.blocks[otherwise.index()];

                let cases: Vec<(IntValue<'llvm>, LlvmBlock<'llvm>)> = targets
                    .iter()
                    .map(|(v, bb)| {
                        let const_val = int_val
                            .get_type()
                            .const_int(*v as u64, false);
                        (const_val, self.blocks[bb.index()])
                    })
                    .collect();

                self.builder()
                    .build_switch(int_val, else_bb, &cases)
                    .unwrap();
            }

            Terminator::Call { func, args, dest, target } => {
                let arg_vals: Vec<BasicMetadataValueEnum<'llvm>> = args
                    .iter()
                    .map(|a| self.codegen_operand(a).into())
                    .collect();

                let callee = self.resolve_callee(func);
                let call_val = self.builder()
                    .build_call(callee, &arg_vals, "call")
                    .unwrap();

                if let Some(ret_val) = call_val.try_as_basic_value().left() {
                    let dest_ptr = self.codegen_place_ptr(dest);
                    self.builder().build_store(dest_ptr, ret_val).unwrap();
                }

                self.builder()
                    .build_unconditional_branch(self.blocks[target.index()])
                    .unwrap();
            }

            Terminator::Assert { cond, expected, target, .. } => {
                let cond_val = self.codegen_operand(cond).into_int_value();
                let expected_val = self.ctx()
                    .bool_type()
                    .const_int(*expected as u64, false);

                let ok = self.builder()
                    .build_int_compare(IntPredicate::EQ, cond_val, expected_val, "assert_ok")
                    .unwrap();

                let fn_val = self.builder()
                    .get_insert_block()
                    .unwrap()
                    .get_parent()
                    .unwrap();
                let panic_bb = self.ctx().append_basic_block(fn_val, "assert_fail");
                let cont_bb = self.blocks[target.index()];

                self.builder().build_conditional_branch(ok, cont_bb, panic_bb).unwrap();

                // Panic block: trap + unreachable
                self.builder().position_at_end(panic_bb);
                self.build_trap();
                self.builder().build_unreachable().unwrap();
            }

            Terminator::Return => {
                let ret_ty = self.body.locals[0].ty.clone();
                if ret_ty == Ty::Unit || ret_ty == Ty::Never {
                    self.builder().build_return(None).unwrap();
                } else {
                    let llvm_ty = self.cg.types.construct_type(self.cg.tcx, &ret_ty);
                    let local_ptr = self.locals[0];
                    let ret_val = self.cg.builder.build_load(llvm_ty, local_ptr, "ret").unwrap();
                    self.cg.builder.build_return(Some(&ret_val)).unwrap();
                }
            }

            Terminator::Unreachable => {
                self.builder().build_unreachable().unwrap();
            }
        }
    }

    fn codegen_rvalue(&mut self, rvalue: &Rvalue, dest_ty: &Ty) -> BasicValueEnum<'llvm> {
        match rvalue {
            Rvalue::Use(op) => self.codegen_operand(op),

            Rvalue::BinaryOp(op, lhs, rhs) => {
                let lhs_val = self.codegen_operand(lhs);
                let rhs_val = self.codegen_operand(rhs);
                let op_ty = self.operand_ty(lhs);
                build_llvm_binop(&self.cg.builder, lhs_val, rhs_val, op, &op_ty)
            }

            Rvalue::UnaryOp(op, operand) => {
                let val = self.codegen_operand(operand);
                match op {
                    UnOp::Neg => {
                        let op_ty = self.operand_ty(operand);
                        match op_ty {
                            Ty::Int(_) => self.builder()
                                .build_int_neg(val.into_int_value(), "neg")
                                .unwrap()
                                .as_basic_value_enum(),
                            Ty::Float(_) => self.builder()
                                .build_float_neg(val.into_float_value(), "neg")
                                .unwrap()
                                .as_basic_value_enum(),
                            _ => panic!("neg on non-numeric type"),
                        }
                    }
                    UnOp::Not => {
                        self.builder()
                            .build_not(val.into_int_value(), "not")
                            .unwrap()
                            .as_basic_value_enum()
                    }
                }
            }

            Rvalue::Aggregate(kind, ops) => {
                let vals: Vec<BasicValueEnum<'llvm>> = ops.iter()
                    .map(|o| self.codegen_operand(o))
                    .collect();
                self.codegen_aggregate(kind, &vals, dest_ty)
            }

            Rvalue::Discriminant(place) => {
                // Для enum: загружаем первое поле (tag)
                let place_ptr = self.codegen_place_ptr(place);
                let place_ty = self.place_ty(place);
                let llvm_ty = self.cg.types.construct_type(self.cg.tcx, &place_ty);
                let tag_ptr = self.builder()
                    .build_struct_gep(
                        llvm_ty,
                        place_ptr,
                        0,
                        "discr_ptr",
                    )
                    .unwrap();
                self.builder()
                    .build_load(self.ctx().i64_type(), tag_ptr, "discr")
                    .unwrap()
            }

            Rvalue::Cast(kind, op, target_ty) => {
                let val = self.codegen_operand(op);
                let target = self.cg.types.construct_type(self.cg.tcx, target_ty);
                let op_ty = self.operand_ty(op);
                self.codegen_cast(kind, val, target, &op_ty, target_ty)
            }
        }
    }

    fn codegen_aggregate(
        &mut self,
        kind: &AggregateKind,
        vals: &[BasicValueEnum<'llvm>],
        dest_ty: &Ty,
    ) -> BasicValueEnum<'llvm> {
        match kind {
            AggregateKind::Adt { .. } => {
                let struct_ty = self.cg.types.construct_type(self.cg.tcx, dest_ty);
                let mut agg = struct_ty.into_struct_type()
                    .get_undef();
                for (i, val) in vals.iter().enumerate() {
                    agg = self.builder()
                        .build_insert_value(agg, *val, i as u32, "field")
                        .unwrap()
                        .into_struct_value();
                }
                agg.as_basic_value_enum()
            }
            AggregateKind::Closure { .. } => {
                let struct_ty = self.cg.types.construct_type(self.cg.tcx, dest_ty);
                let mut agg = struct_ty.into_struct_type().get_undef();
                for (i, val) in vals.iter().enumerate() {
                    agg = self.builder()
                        .build_insert_value(agg, *val, i as u32, "capture")
                        .unwrap()
                        .into_struct_value();
                }
                agg.as_basic_value_enum()
            }
        }
    }

    fn codegen_cast(
        &mut self,
        kind: &crow_mir::CastKind,
        val: BasicValueEnum<'llvm>,
        target: BasicTypeEnum<'llvm>,
        from_ty: &Ty,
        to_ty: &Ty,
    ) -> BasicValueEnum<'llvm> {
        match kind {
            crow_mir::CastKind::IntToInt => {
                let from_bits = val.into_int_value().get_type().get_bit_width();
                let to_bits = target.into_int_type().get_bit_width();
                if from_bits < to_bits {
                    if is_signed_int(from_ty) {
                        self.builder()
                            .build_int_s_extend(val.into_int_value(), target.into_int_type(), "sext")
                            .unwrap()
                            .as_basic_value_enum()
                    } else {
                        self.builder()
                            .build_int_z_extend(val.into_int_value(), target.into_int_type(), "zext")
                            .unwrap()
                            .as_basic_value_enum()
                    }
                } else if from_bits > to_bits {
                    self.builder()
                        .build_int_truncate(val.into_int_value(), target.into_int_type(), "trunc")
                        .unwrap()
                        .as_basic_value_enum()
                } else {
                    val
                }
            }
            crow_mir::CastKind::IntToFloat => {
                if is_signed_int(from_ty) {
                    self.builder()
                        .build_signed_int_to_float(val.into_int_value(), target.into_float_type(), "sitofp")
                        .unwrap()
                        .as_basic_value_enum()
                } else {
                    self.builder()
                        .build_unsigned_int_to_float(val.into_int_value(), target.into_float_type(), "uitofp")
                        .unwrap()
                        .as_basic_value_enum()
                }
            }
            crow_mir::CastKind::FloatToInt => {
                if is_signed_int(to_ty) {
                    self.builder()
                        .build_float_to_signed_int(val.into_float_value(), target.into_int_type(), "fptosi")
                        .unwrap()
                        .as_basic_value_enum()
                } else {
                    self.builder()
                        .build_float_to_unsigned_int(val.into_float_value(), target.into_int_type(), "fptoui")
                        .unwrap()
                        .as_basic_value_enum()
                }
            }
            crow_mir::CastKind::FloatToFloat => {
                self.builder()
                    .build_float_cast(val.into_float_value(), target.into_float_type(), "fpcast")
                    .unwrap()
                    .as_basic_value_enum()
            }
        }
    }

    fn codegen_operand(&mut self, op: &Operand) -> BasicValueEnum<'llvm> {
        match op {
            Operand::Copy(place) => {
                let ptr = self.codegen_place_ptr(place);
                let p_ty = self.place_ty(place);
                let ty = self.cg.types.construct_type(self.cg.tcx, &p_ty);
                self.builder().build_load(ty, ptr, "load").unwrap()
            }
            Operand::Const(c) => self.codegen_constant(c),
        }
    }

    fn codegen_place_ptr(&mut self, place: &Place) -> PointerValue<'llvm> {
        let mut ptr = self.locals[place.local.index()];
        let mut current_ty = self.body.locals[place.local.index()].ty.clone();
        let mut active_variant: u32 = 0;

        for proj in &place.proj {
            match proj {
                Projection::Downcast(v) => {
                    active_variant = *v;
                    if let Ty::Adt(_, _) = &current_ty {
                        let adt_ty = self.cg.types.construct_type(self.cg.tcx, &current_ty);
                        ptr = self.cg.builder
                            .build_struct_gep(adt_ty, ptr, 1, "payload_ptr")
                            .unwrap();
                    }
                }
                Projection::Field(idx) => {
                    let field_tys: Vec<BasicTypeEnum<'llvm>> = match &current_ty {
                        Ty::Adt(def_id, _) => self.cg.tcx.adt(*def_id)
                            .variants[active_variant as usize]
                            .fields
                            .iter()
                            .map(|f| self.cg.types.construct_type(self.cg.tcx, f))
                            .collect(),
                        _ => panic!("field on non-ADT"),
                    };
                    let variant_ty = self.ctx().struct_type(&field_tys, false);
                    ptr = self.cg.builder
                        .build_struct_gep(variant_ty, ptr, *idx, "field_ptr")
                        .unwrap();
                    current_ty = match &current_ty {
                        Ty::Adt(def_id, _) => {
                            self.cg.tcx.adt(*def_id).variants[active_variant as usize]
                                .fields[*idx as usize].clone()
                        }
                        _ => panic!("field on non-ADT"),
                    };
                    active_variant = 0;
                }
            }
        }

        ptr
    }

    fn place_ty(&mut self, place: &Place) -> Ty {
        self.body.locals[place.local.index()].ty.clone()
    }

    fn operand_ty(&mut self, op: &Operand) -> Ty {
        match op {
            Operand::Copy(place) => self.place_ty(place),
            Operand::Const(c) => c.ty(),
        }
    }

    fn codegen_constant(&mut self, c: &Constant) -> BasicValueEnum<'llvm> {
        match c {
            Constant::Unit => {
                self.ctx().i8_type().const_zero().as_basic_value_enum()
            }
            Constant::Bool(v) => {
                self.ctx()
                    .bool_type()
                    .const_int(*v as u64, false)
                    .as_basic_value_enum()
            }
            Constant::Int(v, ity) => {
                let llvm_ty = match ity {
                    IntTy::I8 | IntTy::U8 => self.ctx().i8_type(),
                    IntTy::I16 | IntTy::U16 => self.ctx().i16_type(),
                    IntTy::I32 | IntTy::U32 => self.ctx().i32_type(),
                    IntTy::I64 | IntTy::U64 => self.ctx().i64_type(),
                };
                llvm_ty.const_int(*v as u64, is_signed(ity)).as_basic_value_enum()
            }
            Constant::Float(v, fty) => {
                let llvm_ty = match fty {
                    FloatTy::F32 => self.ctx().f32_type(),
                    FloatTy::F64 => self.ctx().f64_type(),
                };
                llvm_ty.const_float(*v).as_basic_value_enum()
            }
            Constant::Str(s) => {
                let global = self.cg.builder
                    .build_global_string_ptr(s, "str")
                    .unwrap();
                let ptr_val = global.as_pointer_value();
                let len_val = self.ctx().i64_type().const_int(s.len() as u64, false);

                let str_ty = self.ctx().struct_type(
                    &[
                        self.ctx().ptr_type(inkwell::AddressSpace::default()).into(),
                        self.ctx().i64_type().into(),
                    ],
                    false,
                );
                let mut str_val = str_ty.get_undef();
                str_val = self.cg.builder
                    .build_insert_value(str_val, ptr_val, 0, "str_ptr")
                    .unwrap()
                    .into_struct_value();
                str_val = self.cg.builder
                    .build_insert_value(str_val, len_val, 1, "str_len")
                    .unwrap()
                    .into_struct_value();
                str_val.as_basic_value_enum()
            }
            Constant::Fn(_, _) | Constant::Native(_) | Constant::Global(_) => {
                // Fn/Native используются только как callee в Call terminator,
                // не как значения. Если дойдёт сюда — function pointer.
                self.ctx().i8_type().const_zero().as_basic_value_enum()
            }
        }
    }

    fn resolve_callee(&mut self, func: &Operand) -> FunctionValue<'llvm> {
        match func {
            Operand::Const(Constant::Fn(fn_id, substs)) => {
                let inst = Instance { fn_id: *fn_id, substs: substs.clone() };
                *self.cg.instance_fns.get(&inst)
                    .unwrap_or_else(|| panic!("unresolved instance: {fn_id} {substs:?}"))
            }
            Operand::Const(Constant::Native(nid)) => {
                self.cg.native_fns[nid.index()]
            }
            _ => panic!("indirect calls not yet supported"),
        }
    }

    fn build_trap(&mut self) {
        let trap = inkwell::intrinsics::Intrinsic::find("llvm.trap").unwrap();
        let trap_fn = trap
            .get_declaration(&self.cg.module, &[])
            .unwrap();
        self.builder().build_call(trap_fn, &[], "trap").unwrap();
    }
}

fn is_signed(ity: &IntTy) -> bool {
    matches!(ity, IntTy::I8 | IntTy::I16 | IntTy::I32 | IntTy::I64)
}

fn is_signed_int(ty: &Ty) -> bool {
    match ty {
        Ty::Int(ity) => is_signed(ity),
        _ => false,
    }
}