macro_rules! define_id {
    ($($name:ident),*) => {
        $(
            #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
            pub struct $name(pub u32);

            impl $name {
                pub fn as_index(self) -> usize {
                    self.0 as usize
                }
            }
        )*
    };
}
define_id!(ExprId, StmtId, PatId, ItemId, BodyId, HirLocalId);
