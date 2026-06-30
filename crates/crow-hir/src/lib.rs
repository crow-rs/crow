use crow_resolving::resolve_ctx::ResolveCtxt;

use crate::id::BodyId;

pub mod id;
pub mod ty;
pub mod expr;
pub mod stmt;
pub mod pat;
pub mod item;
pub mod body;


#[derive(Debug)]
pub struct Hir {
    pub items: Vec<item::HirItem>,

    pub bodies: Vec<body::HirBody>,

    pub resolve: ResolveCtxt,
}

impl Hir {
    pub fn body(&self, id: BodyId) -> &body::HirBody {
        &self.bodies[id.as_index()]
    }
}