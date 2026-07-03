use crow_tycheck::ty::{FloatTy, IntTy, Ty};
use inkwell::{context::Context, types::BasicTypeEnum};

fn construct_basic_int_type<'llvm>(ctx: &'llvm Context, bitness: &IntTy) -> BasicTypeEnum<'llvm> {
    match bitness {
        IntTy::I8 => ctx.i8_type().into(),
        IntTy::I16 => ctx.i16_type().into(),
        IntTy::I32 => ctx.i32_type().into(),
        IntTy::I64 => ctx.i64_type().into(),
        IntTy::U8 => ctx.i64_type().into(),
        IntTy::U16 => ctx.i64_type().into(),
        IntTy::U32 => ctx.i64_type().into(),
        IntTy::U64 => ctx.i64_type().into(),
    }
}

fn construct_basic_float_type<'llvm>(ctx: &'llvm Context, bitness: &FloatTy) -> BasicTypeEnum<'llvm> {
    match bitness {
        FloatTy::F32 => ctx.f32_type().into(),
        FloatTy::F64 => ctx.f64_type().into(),
    }
}

pub fn construct_type<'llvm>(ctx: &'llvm Context, ty: &Ty) -> BasicTypeEnum<'llvm> {
    match ty {
        Ty::Bool => ctx.i8_type().into(),
        Ty::Float(bitness) => construct_basic_float_type(ctx, bitness),
        Ty::Int(bitness) => construct_basic_int_type(ctx, bitness),
        Ty::Unit => ctx.i8_type().into(),
        Ty::Never => ctx.i8_type().into(),
        _ => panic!("Unsupported type for construct: {:?}", ty)
    }
}