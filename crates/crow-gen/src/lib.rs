use crow_ir::{
    MirEnumDef,
    MirFunction, MirModule, 
    MirStructDef, MirType,
};
use inkwell::{
    builder::Builder,
    context::Context,
    module::Module,
    targets::{InitializationConfig, Target, TargetMachine},
    types::{BasicMetadataTypeEnum, BasicType, BasicTypeEnum},
    values::{FunctionValue, PointerValue},
    AddressSpace, OptimizationLevel,
};
use std::collections::HashMap;

use crate::fn_emit::FnEmitCtx;

mod fn_emit;

pub struct Codegen<'ctx> {
    ctx: &'ctx Context,
    llvm_module: Module<'ctx>,
    builder: Builder<'ctx>,

    functions: HashMap<u32, FunctionValue<'ctx>>,
    struct_types: HashMap<u32, inkwell::types::StructType<'ctx>>,
}

impl<'ctx> Codegen<'ctx> {
    pub fn new(ctx: &'ctx Context, name: &str) -> Self {
        Self {
            ctx,
            llvm_module: ctx.create_module(name),
            builder: ctx.create_builder(),
            functions: HashMap::new(),
            struct_types: HashMap::new(),
        }
    }

    fn llvm_type(&self, ty: &MirType) -> BasicTypeEnum<'ctx> {
        match ty {
            MirType::Int => self.ctx.i64_type().into(),
            MirType::Float => self.ctx.f64_type().into(),
            MirType::Bool => self.ctx.bool_type().into(),
            MirType::Str => self.ctx.ptr_type(AddressSpace::default()).into(),
            MirType::Unit => self.ctx.i8_type().into(),
            MirType::Struct(id) => self.struct_types[id].into(),
            MirType::Enum(_) => {
                self.ctx.ptr_type(AddressSpace::default()).into()
            }
            MirType::Array(_) => self.ctx.ptr_type(AddressSpace::default()).into(),
            MirType::FunPtr { .. } => self.ctx.ptr_type(AddressSpace::default()).into(),
            MirType::Closure { .. } => self.ctx.ptr_type(AddressSpace::default()).into(),
            MirType::Never => self.ctx.i8_type().into(),
        }
    }

    fn llvm_fn_type(
        &self,
        params: &[MirType],
        ret: &MirType,
    ) -> inkwell::types::FunctionType<'ctx> {
        let param_types: Vec<BasicMetadataTypeEnum<'ctx>> = params
            .iter()
            .map(|p| self.llvm_type(p).into())
            .collect();

        match ret {
            MirType::Unit | MirType::Never => {
                self.ctx.void_type().fn_type(&param_types, false)
            }
            _ => {
                self.llvm_type(ret).fn_type(&param_types, false)
            }
        }
    }


    fn declare_functions(&mut self, module: &MirModule) {
        for (idx, f) in module.functions.iter().enumerate() {
            let fn_type = self.llvm_fn_type(&f.params, &f.ret);
            let fn_val = self.llvm_module.add_function(&f.name, fn_type, None);
            self.functions.insert(idx as u32, fn_val);
        }
    }

    fn emit_function(&mut self, id: u32, f: &MirFunction) {
        let body = match &f.body {
            Some(b) => b,
            None => return, 
        };

        let fn_val = self.functions[&id];

        let llvm_blocks: Vec<_> = (0..body.blocks.len())
            .map(|i| self.ctx.append_basic_block(fn_val, &format!("bb{}", i)))
            .collect();

        self.builder.position_at_end(llvm_blocks[0]);

        let locals: Vec<PointerValue<'ctx>> = body
            .locals
            .iter()
            .enumerate()
            .map(|(i, local)| {
                let ty = self.llvm_type(&local.ty);
                let default_name = format!("_{}", i);
                let name = local.name.as_deref().unwrap_or(&default_name);
                self.builder.build_alloca(ty, name).unwrap()
            })
            .collect();

        for i in 0..body.arg_count {
            let arg = fn_val.get_nth_param(i as u32).unwrap();
            self.builder.build_store(locals[i + 1], arg).unwrap();
        }

        let mut ctx = FnEmitCtx {
            cg: self,
            fn_val,
            locals: &locals,
            llvm_blocks: &llvm_blocks,
            body,
        };

        for (i, bb) in body.blocks.iter().enumerate() {
            ctx.emit_block(i, bb);
        }
    }

    pub fn run(&mut self, module: &MirModule) {
        for (idx, s) in module.structs.iter().enumerate() {
            self.emit_struct(idx as u32, s);
        }

        self.declare_functions(module);

        for (idx, f) in module.functions.iter().enumerate() {
            self.emit_function(idx as u32, f);
        }
    }

    fn emit_struct(&mut self, id: u32, s: &MirStructDef) {
        let field_types: Vec<BasicTypeEnum<'ctx>> = s
            .fields
            .iter()
            .map(|f| self.llvm_type(&f.ty))
            .collect();

        let struct_type = self.ctx.struct_type(&field_types, false);
        self.struct_types.insert(id, struct_type);
    }

    fn emit_enum(&mut self, _e: &MirEnumDef) {
        // TODO: tagged union layout
    }
}


pub fn emit(module: &MirModule) {
    Target::initialize_all(&InitializationConfig::default());

    let triple = TargetMachine::get_default_triple();
    let cpu_features = TargetMachine::get_host_cpu_features();
    let cpu_name = TargetMachine::get_host_cpu_name();

    let target = Target::from_triple(&triple).unwrap();
    let tm = target
        .create_target_machine(
            &triple,
            cpu_name.to_str().unwrap(),
            cpu_features.to_str().unwrap(),
            OptimizationLevel::Aggressive,
            inkwell::targets::RelocMode::PIC,
            inkwell::targets::CodeModel::Default,
        )
        .unwrap();

    let ctx = Context::create();
    let mut codegen = Codegen::new(&ctx, "main");

    codegen.llvm_module.set_triple(&triple);
    codegen
        .llvm_module
        .set_data_layout(&tm.get_target_data().get_data_layout());

    codegen.run(module);

    codegen.llvm_module.verify().unwrap();

    codegen
        .llvm_module
        .run_passes("default<O3>", &tm, inkwell::passes::PassBuilderOptions::create())
        .unwrap();

    codegen.llvm_module.print_to_stderr();
}