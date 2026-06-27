use crow_ir::{MirEnumDef, MirEnumId};
use inkwell::values::{IntValue, PointerValue};

use crate::Codegen;

pub (crate) struct EnumRepresentation<'a> {
    pub tag: IntValue<'a>,
    pub payload: PointerValue<'a>
}

pub (crate) struct EnumEmitCtx<'a, 'ctx> {
    pub cg: &'a mut Codegen<'ctx>,
    pub enums: Vec<EnumRepresentation<'a>>
}

impl<'a, 'ctx> EnumEmitCtx<'a, 'ctx> {

    pub fn emit_enum(&mut self, enum_id: MirEnumId, def: &'a MirEnumDef) {

    }
}