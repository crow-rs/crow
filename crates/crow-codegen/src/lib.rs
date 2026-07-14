mod type_builder;
mod ops_builder;

pub mod comp_ops;
mod link;
mod fn_gen;
mod specialize;
mod mangle;
mod error;
mod sys_linker;
mod intrinsic_codegen;

use std::{collections::HashMap, path::Path};

use crow_mir_monomorph::{Instance, MonoItem};
use crow_types::Ty;
use inkwell::{
    OptimizationLevel, builder::Builder, context::Context, module::Module, targets::{FileType, InitializationConfig, Target, TargetTriple}, types::{BasicMetadataTypeEnum, BasicType, FunctionType}, values::FunctionValue,
};

use crow_mir::{
    FnId, LangDefs, MirBody, MirModule, MirNative, MirTyCtxt
};


use crate::{comp_ops::TargetConfig, fn_gen::FnCodegen, link::{LinkConfig, LinkInput, link}, mangle::mangle_name, specialize::Specializer, type_builder::TypeCache};

pub struct Codegen<'llvm, 'mir> {
    pub context: &'llvm Context,
    pub module: Module<'llvm>,
    pub builder: Builder<'llvm>,
    pub types: TypeCache<'llvm>,
    pub tcx: &'mir MirTyCtxt,
    pub lang_defs: &'mir LangDefs,
    native_fns: Vec<FunctionValue<'llvm>>,
    instance_fns: HashMap<Instance, FunctionValue<'llvm>>,
}

impl<'llvm, 'mir> Codegen<'llvm, 'mir> {
    pub fn new(context: &'llvm Context, tcx: &'mir MirTyCtxt, lang_defs: &'mir LangDefs, module_name: &str) -> Self {
        Self {
            module: context.create_module(module_name),
            builder: context.create_builder(),
            types: TypeCache::new(context),
            context,
            tcx,
            lang_defs, 
            native_fns: Vec::new(),
            instance_fns: HashMap::new(),
        }
    }

    pub fn codegen_module(&mut self, mir: &MirModule, items: &[MonoItem]) {
        for native in &mir.natives {
            let fv = self.declare_native(native);
            self.native_fns.push(fv);
        }

        for item in items {
            if let MonoItem::Fn(inst) = item {
                if self.lang_defs.intrinsics.contains_key(&inst.fn_id) {
                    continue;
                }
                let body = &mir.functions[inst.fn_id.index()];
                let specialized = Specializer::specialize_body(body, &inst.substs);
                let name = mangle_name(&body.name, &inst.substs);
                let fv = self.declare_function_with_name(&specialized, &name);
                self.instance_fns.insert(inst.clone(), fv);
            }
        }

        for item in items {
            if let MonoItem::Fn(inst) = item {
                if self.lang_defs.intrinsics.contains_key(&inst.fn_id) {
                    continue;
                }
                let body = &mir.functions[inst.fn_id.index()];
                let specialized = Specializer::specialize_body(body, &inst.substs);
                let fv = self.instance_fns[inst];
                self.codegen_function(&specialized, fv);
            }
        }
    }

    fn declare_native(&mut self, native: &MirNative) -> FunctionValue<'llvm> {
        let fn_type = self.build_fn_type(&native.params, &native.ret);
        self.module.add_function(&native.symbol, fn_type, None)
    }

    fn declare_function_with_name(&mut self, body: &MirBody, name: &str) -> FunctionValue<'llvm> {
        let param_tys: Vec<Ty> = body.locals[1..=body.arg_count]
            .iter()
            .map(|l| l.ty.clone())
            .collect();
        let ret_ty = body.ret_ty().clone();
        let fn_type = self.build_fn_type(&param_tys, &ret_ty);
        self.module.add_function(name, fn_type, None)
    }

    fn build_fn_type(&mut self, params: &[Ty], ret: &Ty) -> FunctionType<'llvm> {
        let param_types: Vec<BasicMetadataTypeEnum> = params.iter()
            .map(|t| self.types.construct_type(self.tcx, t).into())
            .collect();
        if *ret == Ty::Unit || *ret == Ty::Never {
            self.context.void_type().fn_type(&param_types, false)
        } else {
            let ret_type = self.types.construct_type(self.tcx, ret);
            ret_type.fn_type(&param_types, false)
        }
    }

    fn codegen_function(&mut self, body: &MirBody, fv: FunctionValue<'llvm>) {
        let mut fcg = FnCodegen {
            cg: self,
            body,
            locals: Vec::new(),
            blocks: Vec::new(),
        };
        fcg.codegen(fv);
    }
}

pub fn codegen_module<'llvm>(mir: &MirModule, items: &[MonoItem], module_name: &str, build_cfg: &TargetConfig) {
    Target::initialize_all(&InitializationConfig::default());
    
    let triple = TargetTriple::create(build_cfg.triple.as_str());
    
    let target = Target::from_triple(&triple).unwrap();
    let tm = target
        .create_target_machine(
            &triple,
            &build_cfg.cpu,
            &build_cfg.features,
            OptimizationLevel::Aggressive,
            inkwell::targets::RelocMode::PIC,
            inkwell::targets::CodeModel::Default,
        )
        .unwrap();
    
    let context = Context::create();

    let mut cg = Codegen::new(&context, &mir.tcx, &mir.lang, module_name);
    cg.codegen_module(mir, items);
    
    cg.module.set_triple(&triple);
    cg.module.set_data_layout(&tm.get_target_data().get_data_layout());
    
    cg.module.verify().unwrap();

    cg.module
            .run_passes("default<O3>", &tm, inkwell::passes::PassBuilderOptions::create())
            .unwrap();
    
    let path = Path::new("/home/f0rits/Documents/crow/test/output.o");
    
    cg.module.print_to_stderr();

    tm.write_to_file(&cg.module, FileType::Object, path).unwrap();

    let link_cfg = LinkConfig::from_target(&build_cfg);

    let link_cfg = LinkConfig { 
        inputs: vec![LinkInput::Object("/home/f0rits/Documents/crow/test/output.o".into())],
        output: "/home/f0rits/Documents/crow/test/output".into(),
        output_kind: link::OutputKind::Executable,
        ..link_cfg
    };

    link(&link_cfg).unwrap();
}