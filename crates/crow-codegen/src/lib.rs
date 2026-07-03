mod type_builder;
mod ops_builder;

use std::collections::HashMap;

use crow_mir_monomorph::{Instance, MonoItem};
use inkwell::{
    IntPredicate,
    basic_block::BasicBlock as LlvmBlock,
    builder::Builder,
    context::Context,
    module::Module,
    types::{BasicMetadataTypeEnum, BasicType, BasicTypeEnum, FunctionType},
    values::{
        BasicMetadataValueEnum, BasicValue, BasicValueEnum, FunctionValue,
        IntValue, PointerValue,
    },
};

use crow_mir::{
    AggregateKind, BasicBlock, Constant, LocalDecl, MirBody, MirModule, MirNative, Operand, Place, Projection, Rvalue, Statement, Substs, Terminator, UnOp, subst_ty,
};
use crow_tycheck::ty::{FloatTy, IntTy, Ty};

use crate::{ops_builder::build_llvm_binop, type_builder::construct_type};

pub struct Codegen<'llvm> {
    pub context: &'llvm Context,
    pub module: Module<'llvm>,
    pub builder: Builder<'llvm>,

    fn_values: Vec<FunctionValue<'llvm>>,
    native_fns: Vec<FunctionValue<'llvm>>,
    instance_fns: HashMap<Instance, FunctionValue<'llvm>>,
}

struct FnCodegen<'a, 'llvm> {
    cg: &'a Codegen<'llvm>,
    body: &'a MirBody,

    locals: Vec<PointerValue<'llvm>>,
    blocks: Vec<LlvmBlock<'llvm>>,
}

impl<'llvm> Codegen<'llvm> {
    pub fn new(context: &'llvm Context, module_name: &str) -> Self {
        Self {
            module: context.create_module(module_name),
            builder: context.create_builder(),
            context,
            fn_values: Vec::new(),
            native_fns: Vec::new(),
            instance_fns: HashMap::new()
        }
    }

    fn declare_function_with_name(&self, body: &MirBody, name: &str) -> FunctionValue<'llvm> {
        let param_tys: Vec<&Ty> = body.locals[1..=body.arg_count]
            .iter()
            .map(|l| &l.ty)
            .collect();
        let ret_ty = body.ret_ty();
        let fn_type = self.build_fn_type_from_refs(&param_tys, ret_ty);
        self.module.add_function(name, fn_type, None)
    }

    pub fn codegen_module(&mut self, mir: &MirModule, items: &[MonoItem]) {
        for native in &mir.natives {
            let fv = self.declare_native(native);
            self.native_fns.push(fv);
        }

        // Первый проход: объявляем сигнатуры
        for item in items {
            if let MonoItem::Fn(inst) = item {
                let body = &mir.functions[inst.fn_id.index()];
                let specialized = specialize_body(body, &inst.substs);
                let name = mangle_name(&body.name, &inst.substs);
                let fv = self.declare_function_with_name(&specialized, &name);
                self.instance_fns.insert(inst.clone(), fv);
            }
        }

        // Второй проход: генерим тела
        for item in items {
            if let MonoItem::Fn(inst) = item {
                let body = &mir.functions[inst.fn_id.index()];
                let specialized = specialize_body(body, &inst.substs);
                let fv = self.instance_fns[inst];
                self.codegen_function(&specialized, fv);
            }
        }
    }

    fn declare_native(&self, native: &MirNative) -> FunctionValue<'llvm> {
        let fn_type = self.build_fn_type(&native.params, &native.ret);
        self.module.add_function(&native.symbol, fn_type, None)
    }

    fn declare_function(&self, body: &MirBody) -> FunctionValue<'llvm> {
        let param_tys: Vec<&Ty> = body.locals[1..=body.arg_count]
            .iter()
            .map(|l| &l.ty)
            .collect();
        let ret_ty = body.ret_ty();
        let fn_type = self.build_fn_type_from_refs(&param_tys, ret_ty);
        self.module.add_function(&body.name, fn_type, None)
    }

    fn build_fn_type(&self, params: &[Ty], ret: &Ty) -> FunctionType<'llvm> {
        let param_types: Vec<BasicMetadataTypeEnum> = params.iter()
            .map(|t| construct_type(self.context, t).into())
            .collect();

        if *ret == Ty::Unit || *ret == Ty::Never {
            self.context.void_type().fn_type(&param_types, false)
        } else {
            let ret_type = construct_type(self.context, ret);
            ret_type.fn_type(&param_types, false)
        }
    }

    fn build_fn_type_from_refs(&self, params: &[&Ty], ret: &Ty) -> FunctionType<'llvm> {
        let param_types: Vec<BasicMetadataTypeEnum> = params.iter()
            .map(|t| construct_type(self.context, t).into())
            .collect();

        if *ret == Ty::Unit || *ret == Ty::Never {
            self.context.void_type().fn_type(&param_types, false)
        } else {
            let ret_type = construct_type(self.context, ret);
            ret_type.fn_type(&param_types, false)
        }
    }

    fn codegen_function(&self, body: &MirBody, fv: FunctionValue<'llvm>) {
        let mut fcg = FnCodegen {
            cg: self,
            body,
            locals: Vec::new(),
            blocks: Vec::new(),
        };
        fcg.codegen(fv);
    }
}

impl<'a, 'llvm> FnCodegen<'a, 'llvm> {
    fn ctx(&self) -> &'llvm Context { self.cg.context }
    fn builder(&self) -> &Builder<'llvm> { &self.cg.builder }

    fn codegen(&mut self, fv: FunctionValue<'llvm>) {
        let entry = self.ctx().append_basic_block(fv, "entry");
        self.builder().position_at_end(entry);

        for (i, decl) in self.body.locals.iter().enumerate() {
            let ty = construct_type(self.ctx(), &decl.ty);
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

    fn codegen_block(&self, bb: &crow_mir::BasicBlock) {
        for stmt in &bb.stmts {
            self.codegen_stmt(stmt);
        }
        self.codegen_terminator(&bb.term);
    }

    fn codegen_stmt(&self, stmt: &Statement) {
        match stmt {
            Statement::Assign(place, rvalue) => {
                let val = self.codegen_rvalue(rvalue, &self.place_ty(place));
                let ptr = self.codegen_place_ptr(place);
                self.builder().build_store(ptr, val).unwrap();
            }
            Statement::StorageLive(_) | Statement::StorageDead(_) | Statement::Nop => {
                // Хинты для оптимизатора, LLVM справляется через mem2reg
            }
        }
    }

    fn codegen_terminator(&self, term: &Terminator) {
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

            Terminator::Assert { cond, expected, msg, target } => {
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
                let ret_ty = &self.body.locals[0].ty;
                if *ret_ty == Ty::Unit || *ret_ty == Ty::Never {
                    self.builder().build_return(None).unwrap();
                } else {
                    let ret_val = self.builder()
                        .build_load(
                            construct_type(self.ctx(), ret_ty),
                            self.locals[0],
                            "ret",
                        )
                        .unwrap();
                    self.builder().build_return(Some(&ret_val)).unwrap();
                }
            }

            Terminator::Unreachable => {
                self.builder().build_unreachable().unwrap();
            }
        }
    }

    fn codegen_rvalue(&self, rvalue: &Rvalue, dest_ty: &Ty) -> BasicValueEnum<'llvm> {
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
                let tag_ptr = self.builder()
                    .build_struct_gep(
                        construct_type(self.ctx(), &self.place_ty(place)),
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
                let target = construct_type(self.ctx(), target_ty);
                self.codegen_cast(kind, val, target, &self.operand_ty(op), target_ty)
            }
        }
    }

    fn codegen_aggregate(
        &self,
        kind: &AggregateKind,
        vals: &[BasicValueEnum<'llvm>],
        dest_ty: &Ty,
    ) -> BasicValueEnum<'llvm> {
        match kind {
            AggregateKind::Adt { .. } => {
                let struct_ty = construct_type(self.ctx(), dest_ty);
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
                let struct_ty = construct_type(self.ctx(), dest_ty);
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
        &self,
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

    fn codegen_operand(&self, op: &Operand) -> BasicValueEnum<'llvm> {
        match op {
            Operand::Copy(place) => {
                let ptr = self.codegen_place_ptr(place);
                let ty = construct_type(self.ctx(), &self.place_ty(place));
                self.builder().build_load(ty, ptr, "load").unwrap()
            }
            Operand::Const(c) => self.codegen_constant(c),
        }
    }

    fn codegen_place_ptr(&self, place: &Place) -> PointerValue<'llvm> {
        let mut ptr = self.locals[place.local.index()];
        let mut current_ty = self.body.locals[place.local.index()].ty.clone();

        for proj in &place.proj {
            match proj {
                Projection::Field(idx) => {
                    let llvm_ty = construct_type(self.ctx(), &current_ty);
                    ptr = self.builder()
                        .build_struct_gep(llvm_ty, ptr, *idx, "field_ptr")
                        .unwrap();
                    current_ty = self.field_ty(&current_ty, *idx);
                }
                Projection::Downcast(_) => {
                    // Downcast не генерирует код — просто переключает
                    // "активный вариант" для последующих Field проекций.
                    // Pointer остаётся тем же.
                }
            }
        }

        ptr
    }

    fn place_ty(&self, place: &Place) -> Ty {
        self.body.locals[place.local.index()].ty.clone()
    }

    fn field_ty(&self, ty: &Ty, field_idx: u32) -> Ty {
        match ty {
            Ty::Adt(def_id, _) => {
                // Нужен MirTyCtxt, но у нас его нет в FnCodegen.
                // Fallback: возвращаем i64 как заглушку.
                // TODO: передать MirTyCtxt в FnCodegen.
                Ty::Int(IntTy::I64)
            }
            _ => panic!("field projection on non-ADT type"),
        }
    }

    fn operand_ty(&self, op: &Operand) -> Ty {
        match op {
            Operand::Copy(place) => self.place_ty(place),
            Operand::Const(c) => c.ty(),
        }
    }

    fn codegen_constant(&self, c: &Constant) -> BasicValueEnum<'llvm> {
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
                global.as_pointer_value().as_basic_value_enum()
            }
            Constant::Fn(_, _) | Constant::Native(_) | Constant::Global(_) => {
                // Fn/Native используются только как callee в Call terminator,
                // не как значения. Если дойдёт сюда — function pointer.
                self.ctx().i8_type().const_zero().as_basic_value_enum()
            }
        }
    }

    fn resolve_callee(&self, func: &Operand) -> FunctionValue<'llvm> {
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

    fn build_trap(&self) {
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

fn specialize_body(body: &MirBody, substs: &Substs) -> MirBody {
    if substs.is_empty() { return body.clone(); }

    let locals = body.locals.iter().map(|decl| LocalDecl {
        ty: subst_ty(&decl.ty, substs),
        name: decl.name.clone(),
        mutability: decl.mutability,
    }).collect();

    let blocks = body.blocks.iter().map(|bb| BasicBlock {
        stmts: bb.stmts.iter().map(|s| specialize_stmt(s, substs)).collect(),
        term: specialize_term(&bb.term, substs),
    }).collect();

    MirBody {
        name: body.name.clone(),
        arg_count: body.arg_count,
        type_params: vec![],
        locals,
        blocks,
    }
}

fn specialize_stmt(stmt: &Statement, substs: &Substs) -> Statement {
    match stmt {
        Statement::Assign(place, rvalue) => {
            Statement::Assign(place.clone(), specialize_rvalue(rvalue, substs))
        }
        other => other.clone()
    }
}

fn specialize_rvalue(rv: &Rvalue, substs: &Substs) -> Rvalue {
    match rv {
        Rvalue::Cast(kind, op, ty) => {
            Rvalue::Cast(*kind, op.clone(), subst_ty(ty, substs))
        }
        other => other.clone()
    }
}

fn specialize_term(term: &Terminator, substs: &Substs) -> Terminator {
    match term {
        Terminator::Call { func, args, dest, target } => {
            let func = match func {
                Operand::Const(Constant::Fn(id, fn_substs)) => {
                    let resolved: Substs = fn_substs.iter()
                        .map(|t| subst_ty(t, substs))
                        .collect();
                    Operand::Const(Constant::Fn(*id, resolved))
                }
                other => other.clone()
            };
            Terminator::Call { func, args: args.clone(), dest: dest.clone(), target: *target }
        }
        other => other.clone()
    }
}

fn mangle_name(name: &str, substs: &Substs) -> String {
    if substs.is_empty() { return name.to_string(); }
    let suffix: Vec<String> = substs.iter().map(|t| format!("{t}")).collect();
    format!("{name}__{}", suffix.join("_"))
}

pub fn codegen_mir_to_llvm<'llvm>(
    context: &'llvm Context,
    mir: &MirModule,
    items: &[MonoItem],
    module_name: &str,
) -> Module<'llvm> {
    let mut cg = Codegen::new(context, module_name);
    cg.codegen_module(mir, items);
    cg.module
}