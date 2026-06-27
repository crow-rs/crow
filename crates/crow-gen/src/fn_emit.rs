use crow_ir::{MirBasicBlock, MirBinOp, MirBody, MirConstant, MirOperand, MirPlace, MirProjection, MirRvalue, MirStatement, MirTerminator, MirTyCtx, MirType, MirUnOp};
use inkwell::{types::{BasicType, BasicTypeEnum}, values::{BasicValueEnum, PointerValue}};

use crate::{Codegen, primitive_builder::build_llvm_binop};

pub (crate) struct FnEmitCtx<'a, 'ctx> {
    pub cg: &'a mut Codegen<'ctx>,
    pub locals: &'a [PointerValue<'ctx>],
    pub llvm_blocks: &'a [inkwell::basic_block::BasicBlock<'ctx>],
    pub body: &'a MirBody,
    pub tcx: MirTyCtx<'a>,
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

                self.emit_binop(*op, l, r, lhs)
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
        operand: &MirOperand,
    ) -> BasicValueEnum<'ctx> {
        let ty = operand.ty(&self.body.locals, &self.tcx);
        build_llvm_binop(&self.cg.builder, lhs, rhs, &op, &ty)
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

    fn resolve_callee(
        &mut self,
        func: &MirOperand,
        arg_vals: &[BasicValueEnum<'ctx>],
    ) -> (PointerValue<'ctx>, inkwell::types::FunctionType<'ctx>) {
        match func {
            MirOperand::Constant(MirConstant::FunRef(id)) => {
                let func_val = self.cg.functions[id];
                let fn_ty = func_val.get_type();
                let ptr = func_val.as_global_value().as_pointer_value();
                (ptr, fn_ty)
            }

            MirOperand::Copy(place) => {
                let mir_ty = place.ty(&self.body.locals, &self.tcx);
                let ptr = self.emit_place_ptr(place);

                let fn_ty = match &mir_ty {
                    MirType::FunPtr { params, ret } => {
                        let param_tys: Vec<inkwell::types::BasicMetadataTypeEnum> = params
                            .iter()
                            .map(|t| self.cg.llvm_type(t).into())
                            .collect();
                        let ret_ty = self.cg.llvm_type(ret);
                        ret_ty.fn_type(&param_tys, false)
                    }
                    _ => panic!("Call on non-function type: {:?}", mir_ty),
                };

                let loaded = self.cg.builder
                    .build_load(self.cg.ctx.ptr_type(Default::default()), ptr, "fptr")
                    .unwrap()
                    .into_pointer_value();

                (loaded, fn_ty)
            }

            _ => panic!("Call with non-callable operand"),
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
                let arg_vals: Vec<_> = args
                    .iter()
                    .map(|a| self.emit_operand(a).into())
                    .collect();

                let (callee_ptr, fn_ty) = self.resolve_callee(func, &arg_vals);

                let arg_vals: Vec<inkwell::values::BasicMetadataValueEnum> = args
                    .iter()
                    .map(|a| self.emit_operand(a).into())
                    .collect();

                let ret = self.cg.builder
                    .build_indirect_call(fn_ty, callee_ptr, &arg_vals, "call")
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