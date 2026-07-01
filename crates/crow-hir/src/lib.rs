/// Modules
pub mod body;
pub mod expr;
pub mod id;
pub mod item;
pub mod pat;
pub mod stmt;
pub mod ty;

/// Imports
use crate::id::{BodyId, ItemId};
use crow_fresh::FreshenVec;
use crow_resolving::table::ResolveTable;

/// HIR root
#[derive(Debug)]
pub struct Hir {
    /// Items mapping: ItemId -> HirItem
    pub items: FreshenVec<u32, item::HirItem>,

    /// Bodies mapping: BodyId -> HirBody
    pub bodies: FreshenVec<u32, body::HirBody>,

    /// Resolve context
    pub resolve: ResolveTable,
}

/// Implementation of a hir
impl Hir {
    /// Returns body by id
    pub fn body(&self, id: BodyId) -> &body::HirBody {
        &self.bodies.item_at(id.0)
    }

    /// Returns item by id
    pub fn item(&self, id: ItemId) -> &item::HirItem {
        &self.items.item_at(id.0)
    }
}
