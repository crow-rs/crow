use crow_types::Ty;
use inkwell::values::{BasicValue, BasicValueEnum};

use crate::fn_gen::FnCodegen;

impl<'a, 'llvm, 'mir> FnCodegen<'a, 'llvm, 'mir> {
    pub fn codegen_intrinsic(
        &mut self,
        name: &str,
        args: &[BasicValueEnum<'llvm>],
        dest_ty: &Ty,
    ) -> Option<BasicValueEnum<'llvm>> {
        match name {
            "ptr_read" => {
                let ptr = args[0].into_pointer_value();
                let offset = args[1].into_int_value();
                let addr = unsafe {
                    self.cg.builder.build_in_bounds_gep(
                        self.ctx().i8_type(), ptr, &[offset], "addr"
                    ).unwrap()
                };
                let llvm_ty = self.cg.types.construct_type(self.cg.tcx, dest_ty);
                Some(self.cg.builder.build_load(llvm_ty, addr, "ptr_read").unwrap())
            }
            "ptr_write" => {
                let ptr = args[0].into_pointer_value();
                let offset = args[1].into_int_value();
                let val = args[2];
                let addr = unsafe {
                    self.cg.builder.build_in_bounds_gep(
                        self.ctx().i8_type(), ptr, &[offset], "addr"
                    ).unwrap()
                };
                self.cg.builder.build_store(addr, val).unwrap();
                None
            }
            "ptr_offset" => {
                let ptr = args[0].into_pointer_value();
                let offset = args[1].into_int_value();
                let result = unsafe {
                    self.cg.builder.build_in_bounds_gep(
                        self.ctx().i8_type(), ptr, &[offset], "offset"
                    ).unwrap()
                };
                Some(result.as_basic_value_enum())
            }
            "trap" => {
                self.build_trap();
                None
            }
            "size_of" => {
                let size = self.cg.types.type_size(self.cg.tcx, dest_ty);
                Some(self.ctx().i64_type().const_int(size as u64, false).as_basic_value_enum())
            }
            "str_len" => {
                let str_val = args[0].into_struct_value();
                Some(self.cg.builder.build_extract_value(str_val, 1, "len").unwrap())
            }
            "str_ptr" => {
                let str_val = args[0].into_struct_value();
                Some(self.cg.builder.build_extract_value(str_val, 0, "ptr").unwrap())
            }
            other => panic!("unknown intrinsic: {other}"),
        }
    }
}