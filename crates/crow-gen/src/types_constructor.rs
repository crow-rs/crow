use crow_ir::{MirIntBitness, MirType};
use inkwell::{AddressSpace, context::Context, types::BasicTypeEnum};

fn construct_int_type<'llvm>(ctx: &'llvm Context, bitness: &MirIntBitness) -> BasicTypeEnum<'llvm> {
    match bitness {
        MirIntBitness::Bit8 => ctx.i8_type().into(),
        MirIntBitness::Bit16 => ctx.i16_type().into(),
        MirIntBitness::Bit32 => ctx.i32_type().into(),
        MirIntBitness::Bit64 => ctx.i64_type().into()
    }
}

pub fn construct_type<'llvm>(ctx: &'llvm Context, ty: &MirType) -> BasicTypeEnum<'llvm> {
    match ty {
        MirType::Bool => ctx.i8_type().into(),
        MirType::Float => ctx.f64_type().into(), //todo float bitness
        MirType::Int(bitness) => construct_int_type(ctx, bitness),
        MirType::Unit => ctx.i8_type().into(),
        MirType::Enum(_)
        | MirType::FunPtr { .. }
        | MirType::Array(_)
        | MirType::Closure { .. } => ctx.ptr_type(AddressSpace::default()).into(),
        _ => panic!("Unsupported type for construct: {:?}", ty)
    }
}