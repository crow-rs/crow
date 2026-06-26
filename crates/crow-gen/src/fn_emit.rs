use crow_ir::{MirBasicBlock, MirBinOp, MirBody, MirConstant, MirOperand, MirPlace, MirProjection, MirRvalue, MirStatement, MirTerminator, MirType, MirUnOp};
use inkwell::{FloatPredicate, IntPredicate, types::BasicTypeEnum, values::{BasicValueEnum, FunctionValue, PointerValue}};

use crate::Codegen;

pub (crate) struct FnEmitCtx<'a, 'ctx> {
    pub cg: &'a mut Codegen<'ctx>,
    pub fn_val: FunctionValue<'ctx>,
    pub locals: &'a [PointerValue<'ctx>],
    pub llvm_blocks: &'a [inkwell::basic_block::BasicBlock<'ctx>],
    pub body: &'a MirBody,
}

impl<'a, 'ctx> FnEmitCtx<'a, 'ctx> {
    pub fn emit_block(&mut self, idx: usize, bb: &MirBasicBlock) {
        self.cg.builder.position_at_end(self.llvm_blocks[idx]);

        for stmt in &bb.stmts {
            self.emit_statement(stmt);
        }

        self.emit_terminator(&bb.terminator);
    }

    fn emit_statement(&mut self, stmt: &MirStatement) {
        match stmt {
            MirStatement::Assign(place, rvalue) => {
                let val = self.emit_rvalue(rvalue);
                let ptr = self.emit_place_ptr(place);
                self.cg.builder.build_store(ptr, val).unwrap();
            }
            MirStatement::Nop => {}
        }
    }

    fn emit_rvalue(&mut self, rv: &MirRvalue) -> BasicValueEnum<'ctx> {
        match rv {
            MirRvalue::Use(op) => self.emit_operand(op),

            MirRvalue::BinOp(op, lhs, rhs) => {
                let l = self.emit_operand(lhs);
                let r = self.emit_operand(rhs);
                self.emit_binop(*op, l, r)
            }

            MirRvalue::UnOp(op, val) => {
                let v = self.emit_operand(val);
                self.emit_unop(*op, v)
            }

            MirRvalue::Aggregate(kind, fields) => {
                let vals: Vec<_> = fields.iter().map(|f| self.emit_operand(f)).collect();
                self.emit_aggregate(kind, &vals)
            }

            MirRvalue::Discriminant(place) => {
                // Read tag field from enum
                let ptr = self.emit_place_ptr(place);
                self.cg.builder.build_load(self.cg.ctx.i32_type(), ptr, "discr").unwrap()
            }

            MirRvalue::Len(place) => {
                // TODO: array/string length
                let _ptr = self.emit_place_ptr(place);
                self.cg.ctx.i64_type().const_int(0, false).into()
            }

            MirRvalue::Ref(place) => {
                let ptr = self.emit_place_ptr(place);
                ptr.into()
            }

            MirRvalue::Cast(_kind, op, target_ty) => {
                let val = self.emit_operand(op);
                self.emit_cast(val, target_ty)
            }
        }
    }

    fn emit_operand(&mut self, op: &MirOperand) -> BasicValueEnum<'ctx> {
        match op {
            MirOperand::Copy(place) => {
                let ptr = self.emit_place_ptr(place);
                let ty = self.local_llvm_type(place.local);
                self.cg.builder.build_load(ty, ptr, "load").unwrap()
            }
            MirOperand::Constant(c) => self.emit_constant(c),
        }
    }

    fn emit_constant(&self, c: &MirConstant) -> BasicValueEnum<'ctx> {
        match c {
            MirConstant::Int(v) => self.cg.ctx.i64_type().const_int(*v as u64, true).into(),
            MirConstant::Float(v) => self.cg.ctx.f64_type().const_float(*v).into(),
            MirConstant::Bool(v) => self.cg.ctx.bool_type().const_int(*v as u64, false).into(),
            MirConstant::Str(s) => {
                let global = self.cg.builder.build_global_string_ptr(s, "str").unwrap();
                global.as_pointer_value().into()
            }
            MirConstant::Unit => self.cg.ctx.i8_type().const_zero().into(),
            MirConstant::FunRef(id) => {
                let f = self.cg.functions[id];
                f.as_global_value().as_pointer_value().into()
            }
        }
    }

    fn emit_place_ptr(&mut self, place: &MirPlace) -> PointerValue<'ctx> {
        let mut ptr = self.locals[place.local as usize];

        for proj in &place.projection {
            match proj {
                MirProjection::Field(idx) => {
                    ptr = self.cg.builder
                        .build_struct_gep(
                            self.local_llvm_type(place.local),
                            ptr,
                            *idx as u32,
                            "field",
                        )
                        .unwrap();
                }
                MirProjection::Index(local_id) => {
                    let index = self.cg.builder
                        .build_load(
                            self.cg.ctx.i64_type(),
                            self.locals[*local_id as usize],
                            "idx",
                        )
                        .unwrap()
                        .into_int_value();

                    unsafe {
                        ptr = self.cg.builder
                            .build_gep(
                                self.cg.ctx.i8_type(), // element type — simplified
                                ptr,
                                &[index],
                                "elem",
                            )
                            .unwrap();
                    }
                }
                MirProjection::Downcast(_variant_idx) => {
                    // For now, enum payload sits right after tag
                    // TODO: proper tagged union GEP
                }
            }
        }

        ptr
    }

    fn local_llvm_type(&self, local_id: u32) -> BasicTypeEnum<'ctx> {
        self.cg.llvm_type(&self.body.locals[local_id as usize].ty)
    }

    fn emit_binop(
        &self,
        op: MirBinOp,
        lhs: BasicValueEnum<'ctx>,
        rhs: BasicValueEnum<'ctx>,
    ) -> BasicValueEnum<'ctx> {
        if lhs.is_int_value() {
            let l = lhs.into_int_value();
            let r = rhs.into_int_value();
            let result = match op {
                MirBinOp::Add => self.cg.builder.build_int_add(l, r, "add"),
                MirBinOp::Sub => self.cg.builder.build_int_sub(l, r, "sub"),
                MirBinOp::Mul => self.cg.builder.build_int_mul(l, r, "mul"),
                MirBinOp::Div => self.cg.builder.build_int_signed_div(l, r, "div"),
                MirBinOp::Rem => self.cg.builder.build_int_signed_rem(l, r, "rem"),
                MirBinOp::BitAnd | MirBinOp::And => self.cg.builder.build_and(l, r, "and"),
                MirBinOp::BitOr | MirBinOp::Or => self.cg.builder.build_or(l, r, "or"),
                MirBinOp::BitXor => self.cg.builder.build_xor(l, r, "xor"),
                MirBinOp::Shl => self.cg.builder.build_left_shift(l, r, "shl"),
                MirBinOp::Shr => self.cg.builder.build_right_shift(l, r, true, "shr"),
                MirBinOp::Eq => self.cg.builder.build_int_compare(IntPredicate::EQ, l, r, "eq"),
                MirBinOp::Ne => self.cg.builder.build_int_compare(IntPredicate::NE, l, r, "ne"),
                MirBinOp::Lt => self.cg.builder.build_int_compare(IntPredicate::SLT, l, r, "lt"),
                MirBinOp::Le => self.cg.builder.build_int_compare(IntPredicate::SLE, l, r, "le"),
                MirBinOp::Gt => self.cg.builder.build_int_compare(IntPredicate::SGT, l, r, "gt"),
                MirBinOp::Ge => self.cg.builder.build_int_compare(IntPredicate::SGE, l, r, "ge"),
            };
            return result.unwrap().into();
        }

        let l = lhs.into_float_value();
        let r = rhs.into_float_value();
        let result = match op {
            MirBinOp::Add => self.cg.builder.build_float_add(l, r, "fadd").unwrap().into(),
            MirBinOp::Sub => self.cg.builder.build_float_sub(l, r, "fsub").unwrap().into(),
            MirBinOp::Mul => self.cg.builder.build_float_mul(l, r, "fmul").unwrap().into(),
            MirBinOp::Div => self.cg.builder.build_float_div(l, r, "fdiv").unwrap().into(),
            MirBinOp::Rem => self.cg.builder.build_float_rem(l, r, "frem").unwrap().into(),
            MirBinOp::Eq => self.cg.builder.build_float_compare(FloatPredicate::OEQ, l, r, "feq").unwrap().into(),
            MirBinOp::Ne => self.cg.builder.build_float_compare(FloatPredicate::ONE, l, r, "fne").unwrap().into(),
            MirBinOp::Lt => self.cg.builder.build_float_compare(FloatPredicate::OLT, l, r, "flt").unwrap().into(),
            MirBinOp::Le => self.cg.builder.build_float_compare(FloatPredicate::OLE, l, r, "fle").unwrap().into(),
            MirBinOp::Gt => self.cg.builder.build_float_compare(FloatPredicate::OGT, l, r, "fgt").unwrap().into(),
            MirBinOp::Ge => self.cg.builder.build_float_compare(FloatPredicate::OGE, l, r, "fge").unwrap().into(),
            _ => unreachable!("bitwise ops on floats"),
        };
        result
    }

    fn emit_unop(
        &self,
        op: MirUnOp,
        val: BasicValueEnum<'ctx>,
    ) -> BasicValueEnum<'ctx> {
        match op {
            MirUnOp::Neg => {
                if val.is_int_value() {
                    self.cg.builder.build_int_neg(val.into_int_value(), "neg").unwrap().into()
                } else {
                    self.cg.builder.build_float_neg(val.into_float_value(), "fneg").unwrap().into()
                }
            }
            MirUnOp::Not => {
                self.cg.builder.build_not(val.into_int_value(), "not").unwrap().into()
            }
        }
    }

    fn emit_aggregate(
        &mut self,
        kind: &crow_ir::MirAggregateKind,
        vals: &[BasicValueEnum<'ctx>],
    ) -> BasicValueEnum<'ctx> {
        match kind {
            crow_ir::MirAggregateKind::Struct(id) => {
                let struct_ty = self.cg.struct_types[id];
                let mut agg = struct_ty.get_undef();
                for (i, val) in vals.iter().enumerate() {
                    agg = self.cg.builder
                        .build_insert_value(agg, *val, i as u32, "field")
                        .unwrap()
                        .into_struct_value();
                }
                agg.into()
            }
            _ => {
                // TODO: variant, array, closure
                self.cg.ctx.i8_type().const_zero().into()
            }
        }
    }

    fn emit_cast(
        &self,
        val: BasicValueEnum<'ctx>,
        target: &MirType,
    ) -> BasicValueEnum<'ctx> {
        let target_ty = self.cg.llvm_type(target);
        match target {
            MirType::Float if val.is_int_value() => {
                self.cg.builder
                    .build_signed_int_to_float(val.into_int_value(), target_ty.into_float_type(), "itof")
                    .unwrap()
                    .into()
            }
            MirType::Int if val.is_float_value() => {
                self.cg.builder
                    .build_float_to_signed_int(val.into_float_value(), target_ty.into_int_type(), "ftoi")
                    .unwrap()
                    .into()
            }
            _ => val, // identity / pointer cast
        }
    }

    fn emit_terminator(&mut self, term: &MirTerminator) {
        match term {
            MirTerminator::Goto(target) => {
                self.cg.builder
                    .build_unconditional_branch(self.llvm_blocks[*target as usize])
                    .unwrap();
            }

            MirTerminator::SwitchInt { discr, targets, otherwise } => {
                let val = self.emit_operand(discr).into_int_value();
                let cases: Vec<_> = targets
                    .iter()
                    .map(|(v, bb)| {
                        let const_val = self.cg.ctx.i64_type().const_int(*v as u64, false);
                        (const_val, self.llvm_blocks[*bb as usize])
                    })
                    .collect();
                self.cg.builder
                    .build_switch(val, self.llvm_blocks[*otherwise as usize], &cases)
                    .unwrap();
            }

            MirTerminator::Call { dest, func, args, target } => {
                let callee = self.emit_operand(func).into_pointer_value();
                let arg_vals: Vec<_> = args
                    .iter()
                    .map(|a| self.emit_operand(a).into())
                    .collect();

                // Recover function type for indirect call
                let fn_ty = self.cg.ctx.i64_type().fn_type(
                    &arg_vals.iter().map(|_| self.cg.ctx.i64_type().into()).collect::<Vec<_>>(),
                    false,
                );

                let ret = self.cg.builder
                    .build_indirect_call(fn_ty, callee, &arg_vals, "call")
                    .unwrap()
                    .try_as_basic_value()
                    .left();

                if let Some(ret_val) = ret {
                    let ptr = self.emit_place_ptr(dest);
                    self.cg.builder.build_store(ptr, ret_val).unwrap();
                }

                self.cg.builder
                    .build_unconditional_branch(self.llvm_blocks[*target as usize])
                    .unwrap();
            }

            MirTerminator::Return => {
                let ret_ty = &self.body.locals[0].ty;
                match ret_ty {
                    MirType::Unit | MirType::Never => {
                        self.cg.builder.build_return(None).unwrap();
                    }
                    _ => {
                        let ty = self.local_llvm_type(0);
                        let ret_val = self.cg.builder
                            .build_load(ty, self.locals[0], "ret")
                            .unwrap();
                        self.cg.builder.build_return(Some(&ret_val)).unwrap();
                    }
                }
            }

            MirTerminator::Abort => {
                // Call llvm.trap or abort
                self.cg.builder.build_unreachable().unwrap();
            }

            MirTerminator::Unreachable => {
                self.cg.builder.build_unreachable().unwrap();
            }
        }
    }
}