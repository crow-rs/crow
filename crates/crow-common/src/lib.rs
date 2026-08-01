pub mod macros;
pub mod span;

use serde::{Deserialize, Serialize};

/// Identity of a compilation unit (module).
///
/// Ids are handed out by the driver's persistent build registry and are stable
/// across builds, because a `DefId` is `(ModuleId, LocalDefId)` and ends up
/// serialized into `.crowi` interface artifacts.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ModuleId(pub u32);

/// Module id implementation
impl ModuleId {
    /// Reserved for builtin types and other compiler-synthesized defs
    /// that belong to no user module.
    pub const BUILTIN: ModuleId = ModuleId(0);

    /// First id available to user modules.
    pub const FIRST_USER: ModuleId = ModuleId(1);

    /// Returns `true` if the module is the reserved builtin one
    pub fn is_builtin(&self) -> bool {
        *self == ModuleId::BUILTIN
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LocalDefId(pub u32);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DefId {
    pub module: ModuleId,
    pub local: LocalDefId,
}

impl DefId {
    pub fn is_local(&self, current: ModuleId) -> bool {
        self.module == current
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LocalId(pub u32);
