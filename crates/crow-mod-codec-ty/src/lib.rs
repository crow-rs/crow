use crow_common::ModuleId;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct ModuleTop {
    /// Id of the module this interface belongs to.
    ///
    /// Makes the artifact self-identifying: a consumer can tag imported
    /// definitions with the id of their real owner instead of inventing
    /// local ones.
    pub module_id: ModuleId,

    pub module_name: String,
    pub functions: Vec<ExportedFn>,
    pub types: Vec<ExportedType>,
    pub constants: Vec<ExportedConst>,
}

#[derive(Serialize, Deserialize)]
pub struct ExportedFn {
    pub name: String,
    pub params: Vec<(String, String)>,
    pub ret: String,
    pub is_native: bool,
}

#[derive(Serialize, Deserialize)]
pub struct ExportedType {
    pub name: String,
    pub kind: ExportedTypeKind,
}

#[derive(Serialize, Deserialize)]
pub enum ExportedTypeKind {
    Struct { fields: Vec<(String, String)> },
}

#[derive(Serialize, Deserialize)]
pub struct ExportedConst {
    pub name: String,
    pub ty: String,
}

pub fn save_crowi(path: &std::path::Path, info: &ModuleTop) {
    let json = serde_json::to_string_pretty(info).unwrap();
    std::fs::write(path, json).unwrap();
}

pub fn load_crowi(path: &std::path::Path) -> ModuleTop {
    let json = std::fs::read_to_string(path).unwrap();
    serde_json::from_str(&json).unwrap()
}