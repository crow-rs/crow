use std::collections::HashMap;

use crow_mir::{MirTyCtxt};
use crow_resolving::table::DefId;
use crow_tycheck::ty::{FloatTy, IntTy, Ty};
use inkwell::{context::Context, types::{BasicTypeEnum, StructType}};

pub struct TypeCache<'llvm> {
    ctx: &'llvm Context,
    adts: HashMap<DefId, StructType<'llvm>>,
}

impl<'llvm> TypeCache<'llvm> {
    pub fn new(ctx: &'llvm Context) -> Self {
        Self { ctx, adts: HashMap::new() }
    }

    pub fn construct_type(&mut self, tcx: &MirTyCtxt, ty: &Ty) -> BasicTypeEnum<'llvm> {
        match ty {
            Ty::Bool => self.ctx.bool_type().into(),
            Ty::Int(ity) => self.int_type(ity).into(),
            Ty::Float(fty) => self.float_type(fty).into(),
            Ty::String => self.ctx.ptr_type(inkwell::AddressSpace::default()).into(),
            Ty::Unit => self.ctx.i8_type().into(),
            Ty::Never => self.ctx.i8_type().into(),
            Ty::Adt(def_id, _) => self.adt_type(tcx, *def_id).into(),
            _ => panic!("unsupported type for codegen: {ty}"),
        }
    }

    fn int_type(&self, ity: &IntTy) -> inkwell::types::IntType<'llvm> {
        match ity {
            IntTy::I8  | IntTy::U8  => self.ctx.i8_type(),
            IntTy::I16 | IntTy::U16 => self.ctx.i16_type(),
            IntTy::I32 | IntTy::U32 => self.ctx.i32_type(),
            IntTy::I64 | IntTy::U64 => self.ctx.i64_type(),
        }
    }

    fn float_type(&self, fty: &FloatTy) -> inkwell::types::FloatType<'llvm> {
        match fty {
            FloatTy::F32 => self.ctx.f32_type(),
            FloatTy::F64 => self.ctx.f64_type(),
        }
    }

    pub fn adt_type(&mut self, tcx: &MirTyCtxt, def_id: DefId) -> StructType<'llvm> {
        if let Some(&cached) = self.adts.get(&def_id) {
            return cached;
        }

        let adt = tcx.adt(def_id);
        let is_sum_ty = tcx.is_sum_ty(def_id);

        let st = if is_sum_ty {
            let fields: Vec<BasicTypeEnum<'llvm>> = adt.variants[0]
                .fields
                .iter()
                .map(|f| self.construct_type(tcx, f))
                .collect();
            self.ctx.struct_type(&fields, false)
        } else {
            let tag: BasicTypeEnum = self.ctx.i32_type().into();
            let max_size = adt.variants.iter()
                .map(|v| v.fields.iter().map(|f| self.type_size(tcx, f)).sum::<u32>())
                .max()
                .unwrap_or(0);
            let payload: BasicTypeEnum = self.ctx.i8_type().array_type(max_size).into();
            self.ctx.struct_type(&[tag, payload], false)
        };

        self.adts.insert(def_id, st);
        st
    }

    fn type_size(&mut self, tcx: &MirTyCtxt, ty: &Ty) -> u32 {
        match ty {
            Ty::Bool => 1,
            Ty::Int(IntTy::I8  | IntTy::U8)  => 1,
            Ty::Int(IntTy::I16 | IntTy::U16) => 2,
            Ty::Int(IntTy::I32 | IntTy::U32) => 4,
            Ty::Int(IntTy::I64 | IntTy::U64) => 8,
            Ty::Float(FloatTy::F32) => 4,
            Ty::Float(FloatTy::F64) => 8,
            Ty::String => 8,
            Ty::Unit | Ty::Never => 1,
            Ty::Adt(def_id, _) => {
                let adt = tcx.adt(*def_id);
                let is_sum_ty = tcx.is_sum_ty(*def_id);
                if is_sum_ty {
                    adt.variants[0].fields.iter()
                        .map(|f| self.type_size(tcx, f))
                        .sum()
                } else {
                    4 + adt.variants.iter()
                        .map(|v| v.fields.iter().map(|f| self.type_size(tcx, f)).sum::<u32>())
                        .max()
                        .unwrap_or(0)
                }
            }
            _ => 8,
        }
    }
}