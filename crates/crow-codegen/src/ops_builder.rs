use crow_common::bug;
use crow_mir::BinOp;
use crow_types::Ty;
use inkwell::{FloatPredicate, IntPredicate, builder::Builder, values::{BasicValue, BasicValueEnum}};

fn unwrap_or_bug<'llvm, T, E: std::fmt::Debug>(result: Result<T, E>, op: &str) -> T {
    result.unwrap_or_else(|e| bug!("failed to build {} instruction: {:?}", op, e))
}

fn build_sum<'llvm>(builder: &Builder<'llvm>, _type: &Ty, lhs: BasicValueEnum<'llvm>, rhs: BasicValueEnum<'llvm>) -> BasicValueEnum<'llvm> {
    match _type {
        Ty::Int(_) =>
            unwrap_or_bug(builder.build_int_add(lhs.into_int_value(), rhs.into_int_value(), "int_add"), "int add")
            .as_basic_value_enum(),
        Ty::Float(_) =>
            unwrap_or_bug(builder.build_float_add(lhs.into_float_value(), rhs.into_float_value(), "float_add"), "float add")
            .as_basic_value_enum(),
        Ty::String =>
            bug!("string concat not folded by ConstProp"),
        _ => bug!("unsupported type {:?} for sum operation", _type)
    }
}

fn build_sub<'llvm>(builder: &Builder<'llvm>, _type: &Ty, lhs: BasicValueEnum<'llvm>, rhs: BasicValueEnum<'llvm>) -> BasicValueEnum<'llvm> {
    match _type {
        Ty::Int(_) =>
            unwrap_or_bug(builder.build_int_sub(lhs.into_int_value(), rhs.into_int_value(), "int_sub"), "int sub")
            .as_basic_value_enum(),
        Ty::Float(_) =>
            unwrap_or_bug(builder.build_float_sub(lhs.into_float_value(), rhs.into_float_value(), "float_sub"), "float sub")
            .as_basic_value_enum(),
        _ => bug!("unsupported type {:?} for sub operation", _type)
    }
}

fn build_div<'llvm>(builder: &Builder<'llvm>, _type: &Ty, lhs: BasicValueEnum<'llvm>, rhs: BasicValueEnum<'llvm>) -> BasicValueEnum<'llvm> {
    match _type {
        Ty::Int(_) =>
            unwrap_or_bug(builder.build_int_signed_div(lhs.into_int_value(), rhs.into_int_value(), "int_div"), "int div")
            .as_basic_value_enum(),
        Ty::Float(_) =>
            unwrap_or_bug(builder.build_float_div(lhs.into_float_value(), rhs.into_float_value(), "float_div"), "float div")
            .as_basic_value_enum(),
        _ => bug!("unsupported type {:?} for div operation", _type)
    }
}

fn build_mul<'llvm>(builder: &Builder<'llvm>, _type: &Ty, lhs: BasicValueEnum<'llvm>, rhs: BasicValueEnum<'llvm>) -> BasicValueEnum<'llvm> {
    match _type {
        Ty::Int(_) =>
            unwrap_or_bug(builder.build_int_mul(lhs.into_int_value(), rhs.into_int_value(), "int_mul"), "int mul")
            .as_basic_value_enum(),
        Ty::Float(_) =>
            unwrap_or_bug(builder.build_float_mul(lhs.into_float_value(), rhs.into_float_value(), "float_mul"), "float mul")
            .as_basic_value_enum(),
        _ => bug!("unsupported type {:?} for mul operation", _type)
    }
}

fn build_gt_compare<'llvm>(builder: &Builder<'llvm>, _type: &Ty, lhs: BasicValueEnum<'llvm>, rhs: BasicValueEnum<'llvm>) -> BasicValueEnum<'llvm> {
    match _type {
        Ty::Int(_) => unwrap_or_bug(builder.build_int_compare(IntPredicate::SGT, lhs.into_int_value(), rhs.into_int_value(), "cmpres"), "int gt").as_basic_value_enum(),
        Ty::Float(_) => unwrap_or_bug(builder.build_float_compare(FloatPredicate::OGT, lhs.into_float_value(), rhs.into_float_value(), "cmpres"), "float gt").as_basic_value_enum(),
        _ => bug!("unsupported type {:?} for gt operation", _type)
    }
}

fn build_ge_compare<'llvm>(builder: &Builder<'llvm>, _type: &Ty, lhs: BasicValueEnum<'llvm>, rhs: BasicValueEnum<'llvm>) -> BasicValueEnum<'llvm> {
    match _type {
        Ty::Int(_) => unwrap_or_bug(builder.build_int_compare(IntPredicate::SGE, lhs.into_int_value(), rhs.into_int_value(), "cmpres"), "int ge").as_basic_value_enum(),
        Ty::Float(_) => unwrap_or_bug(builder.build_float_compare(FloatPredicate::OGE, lhs.into_float_value(), rhs.into_float_value(), "cmpres"), "float ge").as_basic_value_enum(),
        _ => bug!("unsupported type {:?} for ge operation", _type)
    }
}

fn build_lt_compare<'llvm>(builder: &Builder<'llvm>, _type: &Ty, lhs: BasicValueEnum<'llvm>, rhs: BasicValueEnum<'llvm>) -> BasicValueEnum<'llvm> {
    match _type {
        Ty::Int(_) => unwrap_or_bug(builder.build_int_compare(IntPredicate::SLT, lhs.into_int_value(), rhs.into_int_value(), "cmpres"), "int lt").as_basic_value_enum(),
        Ty::Float(_) => unwrap_or_bug(builder.build_float_compare(FloatPredicate::OLT, lhs.into_float_value(), rhs.into_float_value(), "cmpres"), "float lt").as_basic_value_enum(),
        _ => bug!("unsupported type {:?} for lt operation", _type)
    }
}

fn build_eq_compare<'llvm>(builder: &Builder<'llvm>, _type: &Ty, lhs: BasicValueEnum<'llvm>, rhs: BasicValueEnum<'llvm>) -> BasicValueEnum<'llvm> {
    match _type {
        Ty::Int(_) | Ty::Bool => unwrap_or_bug(builder.build_int_compare(IntPredicate::EQ, lhs.into_int_value(), rhs.into_int_value(), "cmpres"), "int eq").as_basic_value_enum(),
        Ty::Float(_) => unwrap_or_bug(builder.build_float_compare(FloatPredicate::OEQ, lhs.into_float_value(), rhs.into_float_value(), "cmpres"), "float eq").as_basic_value_enum(),
        _ => bug!("unsupported type {:?} for eq operation", _type)
    }
}

fn build_and<'llvm>(builder: &Builder<'llvm>, _type: &Ty, lhs: BasicValueEnum<'llvm>, rhs: BasicValueEnum<'llvm>) -> BasicValueEnum<'llvm> {
    match _type {
        Ty::Bool => unwrap_or_bug(builder.build_and(lhs.into_int_value(), rhs.into_int_value(), "ssl_and"), "bool and").as_basic_value_enum(),
        _ => bug!("invalid type {:?} to process and operation", _type)
    }
}

fn build_or<'llvm>(builder: &Builder<'llvm>, _type: &Ty, lhs: BasicValueEnum<'llvm>, rhs: BasicValueEnum<'llvm>) -> BasicValueEnum<'llvm> {
    match _type {
        Ty::Bool => unwrap_or_bug(builder.build_or(lhs.into_int_value(), rhs.into_int_value(), "ssl_or"), "bool or").as_basic_value_enum(),
        _ => bug!("invalid type {:?} to process or operation", _type)
    }
}

fn build_bit_xor<'llvm>(builder: &Builder<'llvm>, _type: &Ty, lhs: BasicValueEnum<'llvm>, rhs: BasicValueEnum<'llvm>) -> BasicValueEnum<'llvm> {
    match _type {
        Ty::Int(_) => unwrap_or_bug(builder.build_xor(lhs.into_int_value(), rhs.into_int_value(), "xor"), "int xor").as_basic_value_enum(),
        _ => bug!("invalid type {:?} to process xor operation", _type)
    }
}

fn build_remainder<'llvm>(builder: &Builder<'llvm>, _type: &Ty, lhs: BasicValueEnum<'llvm>, rhs: BasicValueEnum<'llvm>) -> BasicValueEnum<'llvm> {
    match _type {
        Ty::Int(int_ty) => {
            if int_ty.is_signed() {
                unwrap_or_bug(builder.build_int_signed_rem(lhs.into_int_value(), rhs.into_int_value(), "srem"), "signed rem").as_basic_value_enum()
            } else {
                unwrap_or_bug(builder.build_int_unsigned_rem(lhs.into_int_value(), rhs.into_int_value(), "urem"), "unsigned rem").as_basic_value_enum()
            }
        },
        Ty::Float(_) => unwrap_or_bug(builder.build_float_rem(lhs.into_float_value(), rhs.into_float_value(), "frem"), "float rem").as_basic_value_enum(),
        _ => bug!("invalid type {:?} to process rem operation", _type)
    }
}

fn build_bit_shift<'llvm>(builder: &Builder<'llvm>, _type: &Ty, lhs: BasicValueEnum<'llvm>, rhs: BasicValueEnum<'llvm>, left_shift: bool) -> BasicValueEnum<'llvm> {
    match _type {
        Ty::Int(ty) => {
            if left_shift {
                unwrap_or_bug(builder.build_left_shift(lhs.into_int_value(), rhs.into_int_value(), "shl"), "left shift").as_basic_value_enum()
            } else {
                unwrap_or_bug(builder.build_right_shift(lhs.into_int_value(), rhs.into_int_value(), ty.is_signed(), "shr"), "right shift").as_basic_value_enum()
            }
        },
        _ => bug!("invalid type {:?} to process bit shift operation", _type)
    }
}

fn build_not_eq<'llvm>(builder: &Builder<'llvm>, _type: &Ty, lhs: BasicValueEnum<'llvm>, rhs: BasicValueEnum<'llvm>) -> BasicValueEnum<'llvm> {
    match _type {
        Ty::Int(_) | Ty::Bool => {
            unwrap_or_bug(builder.build_int_compare(IntPredicate::NE, lhs.into_int_value(), rhs.into_int_value(), "ne"), "int ne")
                .as_basic_value_enum()
        }
        Ty::Float(_) => {
            unwrap_or_bug(builder.build_float_compare(FloatPredicate::ONE, lhs.into_float_value(), rhs.into_float_value(), "fne"), "float ne")
                .as_basic_value_enum()
        }
        _ => bug!("invalid type {:?} to process not equal operation", _type),
    }
}

fn build_le<'llvm>(builder: &Builder<'llvm>, _type: &Ty, lhs: BasicValueEnum<'llvm>, rhs: BasicValueEnum<'llvm>) -> BasicValueEnum<'llvm> {
    match _type {
        Ty::Int(_) | Ty::Bool => {
            unwrap_or_bug(builder.build_int_compare(IntPredicate::SLE, lhs.into_int_value(), rhs.into_int_value(), "sle"), "int le")
                .as_basic_value_enum()
        }
        Ty::Float(_) => {
            unwrap_or_bug(builder.build_float_compare(FloatPredicate::OLE, lhs.into_float_value(), rhs.into_float_value(), "ole"), "float le")
                .as_basic_value_enum()
        }
        _ => bug!("invalid type {:?} to process less equal operation", _type),
    }
}

pub(crate) fn build_llvm_binop<'llvm>(
    builder: &Builder<'llvm>,
    lhs: BasicValueEnum<'llvm>,
    rhs: BasicValueEnum<'llvm>,
    op: &BinOp,
    binary_ops_type: &Ty
) -> BasicValueEnum<'llvm> {
    match op {
        BinOp::Add => build_sum(builder, binary_ops_type, lhs, rhs),
        BinOp::Sub => build_sub(builder, binary_ops_type, lhs, rhs),
        BinOp::Div => build_div(builder, binary_ops_type, lhs, rhs),
        BinOp::Mul => build_mul(builder, binary_ops_type, lhs, rhs),
        BinOp::BitAnd => build_and(builder, binary_ops_type, lhs, rhs),
        BinOp::BitOr => build_or(builder, binary_ops_type, lhs, rhs),
        BinOp::Gt => build_gt_compare(builder, binary_ops_type, lhs, rhs),
        BinOp::Lt => build_lt_compare(builder, binary_ops_type, lhs, rhs),
        BinOp::Eq => build_eq_compare(builder, binary_ops_type, lhs, rhs),
        BinOp::Ge => build_ge_compare(builder, binary_ops_type, lhs, rhs),
        BinOp::BitXor => build_bit_xor(builder, binary_ops_type, lhs, rhs),
        BinOp::Rem => build_remainder(builder, binary_ops_type, lhs, rhs),
        BinOp::Shl => build_bit_shift(builder, binary_ops_type, lhs, rhs, true),
        BinOp::Shr => build_bit_shift(builder, binary_ops_type, lhs, rhs, false),
        BinOp::Ne => build_not_eq(builder, binary_ops_type, lhs, rhs),
        BinOp::Le => build_le(builder, binary_ops_type, lhs, rhs)
    }
}
