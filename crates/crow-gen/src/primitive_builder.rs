use crow_ir::{MirBinOp, MirType};
use inkwell::{FloatPredicate, IntPredicate, builder::Builder, values::{BasicValue, BasicValueEnum}};


fn build_sum<'llvm>(builder: &Builder<'llvm>, _type: &MirType, lhs: BasicValueEnum<'llvm>, rhs: BasicValueEnum<'llvm>) -> BasicValueEnum<'llvm> {
    match _type {
        MirType::Int(_) => 
            builder.build_int_add(lhs.into_int_value(), rhs.into_int_value(), "int_add")
            .unwrap().as_basic_value_enum(),
        MirType::Float => 
            builder.build_float_add(lhs.into_float_value(), rhs.into_float_value(), "float_add")
            .unwrap().as_basic_value_enum(),
        _ => panic!("{:?}", _type)
    }
}


fn build_sub<'llvm>(builder: &Builder<'llvm>, _type: &MirType, lhs: BasicValueEnum<'llvm>, rhs: BasicValueEnum<'llvm>) -> BasicValueEnum<'llvm> {
    match _type {
        MirType::Int(_) => 
            builder.build_int_sub(lhs.into_int_value(), rhs.into_int_value(), "int_sub")
            .unwrap().as_basic_value_enum(),
        MirType::Float => 
            builder.build_float_sub(lhs.into_float_value(), rhs.into_float_value(), "float_sub")
            .unwrap().as_basic_value_enum(),
        _ => panic!("{:?}", _type)
    }
}

fn build_div<'llvm>(builder: &Builder<'llvm>, _type: &MirType, lhs: BasicValueEnum<'llvm>, rhs: BasicValueEnum<'llvm>) -> BasicValueEnum<'llvm> {
    match _type {
        MirType::Int(_) => 
            builder.build_int_signed_div(lhs.into_int_value(), rhs.into_int_value(), "int_div")
            .unwrap().as_basic_value_enum(),
        MirType::Float => 
            builder.build_float_div(lhs.into_float_value(), rhs.into_float_value(), "float_div")
            .unwrap().as_basic_value_enum(),
        _ => panic!("")
    }
}

fn build_mul<'llvm>(builder: &Builder<'llvm>, _type: &MirType, lhs: BasicValueEnum<'llvm>, rhs: BasicValueEnum<'llvm>) -> BasicValueEnum<'llvm> {
    match _type {
        MirType::Int(_) => 
            builder.build_int_mul(lhs.into_int_value(), rhs.into_int_value(), "int_mul")
            .unwrap().as_basic_value_enum(),
        MirType::Float => 
            builder.build_float_mul(lhs.into_float_value(), rhs.into_float_value(), "float_mul")
            .unwrap().as_basic_value_enum(),
        _ => panic!("{:?}", _type)
    }
}

fn build_gt_compare<'llvm>(builder: &Builder<'llvm>, _type: &MirType, lhs: BasicValueEnum<'llvm>, rhs: BasicValueEnum<'llvm>) -> BasicValueEnum<'llvm> {
    match _type {
        MirType::Int(_) => builder.build_int_compare(IntPredicate::SGT, lhs.into_int_value(), rhs.into_int_value(), "cmpres").unwrap().as_basic_value_enum(),
        MirType::Float => builder.build_float_compare(FloatPredicate::OGT, lhs.into_float_value(), rhs.into_float_value(), "cmpres").unwrap().as_basic_value_enum(),
        _ => panic!("{:?}", _type)
    }
}

fn build_lt_compare<'llvm>(builder: &Builder<'llvm>, _type: &MirType, lhs: BasicValueEnum<'llvm>, rhs: BasicValueEnum<'llvm>) -> BasicValueEnum<'llvm> {
    match _type {
        MirType::Int(_) => builder.build_int_compare(IntPredicate::SLT, lhs.into_int_value(), rhs.into_int_value(), "cmpres").unwrap().as_basic_value_enum(),
        MirType::Float => builder.build_float_compare(FloatPredicate::OLT, lhs.into_float_value(), rhs.into_float_value(), "cmpres").unwrap().as_basic_value_enum(),
        _ => panic!("{:?}", _type)
    }
}

fn build_eq_compare<'llvm>(builder: &Builder<'llvm>, _type: &MirType, lhs: BasicValueEnum<'llvm>, rhs: BasicValueEnum<'llvm>) -> BasicValueEnum<'llvm> {
    match _type {
        MirType::Int(_) | MirType::Bool => builder.build_int_compare(IntPredicate::EQ, lhs.into_int_value(), rhs.into_int_value(), "cmpres").unwrap().as_basic_value_enum(),
        MirType::Float => builder.build_float_compare(FloatPredicate::UEQ, lhs.into_float_value(), rhs.into_float_value(), "cmpres").unwrap().as_basic_value_enum(),
        _ => panic!("{:?}", _type)
    }
}

fn build_and<'llvm>(builder: &Builder<'llvm>, _type: &MirType, lhs: BasicValueEnum<'llvm>, rhs: BasicValueEnum<'llvm>) -> BasicValueEnum<'llvm> {
    match _type {
        MirType::Bool => builder.build_and(lhs.into_int_value(), rhs.into_int_value(), "ssl_and").unwrap().as_basic_value_enum(),
        _ => panic!("Invalid type to process [and] operation.")
    }
}

fn build_or<'llvm>(builder: &Builder<'llvm>, _type: &MirType, lhs: BasicValueEnum<'llvm>, rhs: BasicValueEnum<'llvm>) -> BasicValueEnum<'llvm> {
    match _type {
        MirType::Bool => builder.build_or(lhs.into_int_value(), rhs.into_int_value(), "ssl_or").unwrap().as_basic_value_enum(),
        _ => panic!("Invalid type to process [or] operation.")
    }
}

pub(crate) fn build_llvm_binop<'llvm>(
    builder: &Builder<'llvm>,
    lhs: BasicValueEnum<'llvm>,
    rhs: BasicValueEnum<'llvm>,
    op: &MirBinOp,
    binary_ops_type: &MirType
) -> BasicValueEnum<'llvm> {
    match op {
        MirBinOp::Add => build_sum(builder, binary_ops_type, lhs, rhs),
        MirBinOp::Sub => build_sub(builder, binary_ops_type, lhs, rhs),
        MirBinOp::Div => build_div(builder, binary_ops_type, lhs, rhs),
        MirBinOp::Mul => build_mul(builder, binary_ops_type, lhs, rhs),
        MirBinOp::And => build_and(builder, binary_ops_type, lhs, rhs),
        MirBinOp::Or => build_or(builder, binary_ops_type, lhs, rhs),
        MirBinOp::Gt => build_gt_compare(builder, binary_ops_type, lhs, rhs),
        MirBinOp::Lt => build_lt_compare(builder, binary_ops_type, lhs, rhs),
        MirBinOp::Eq => build_eq_compare(builder, binary_ops_type, lhs, rhs),
        _ => panic!("Assignments are not supported")
    }
}