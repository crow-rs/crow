use crate::io;
use camino::Utf8Path;
use crow_common::ModuleId;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};
use tracing::info;

const REGISTRY_VERSION: u32 = 1;

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct DepStamp {
    pub id: u32,
    pub iface_hash: u64,
}

/// Result of the last successful compilation of one module.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ModuleRecord {
    /// Hash of all source files of the module.
    pub src_hash: u64,

    /// Hash of the emitted `.crowi` — the module's public interface.
    pub iface_hash: u64,

    /// Interfaces this module was compiled against.
    pub deps: Vec<DepStamp>,
}

/// Persistent map `module identity -> ModuleId` plus incremental fingerprints.
#[derive(Serialize, Deserialize)]
pub struct BuildRegistry {
    version: u32,

    /// Next free id. Monotone: ids are never recycled.
    next_id: u32,

    /// `module identity path -> ModuleId`, the part that must stay stable.
    ids: BTreeMap<String, u32>,

    /// Incremental fingerprints, keyed the same way.
    records: BTreeMap<String, ModuleRecord>,
}

/// Default implementation for build registry
impl Default for BuildRegistry {
    fn default() -> Self {
        Self {
            version: REGISTRY_VERSION,
            next_id: ModuleId::FIRST_USER.0,
            ids: BTreeMap::new(),
            records: BTreeMap::new(),
        }
    }
}

/// Build registry implementation
impl BuildRegistry {
    /// Loads registry from disk, falling back to an empty one.
    pub fn load(path: &Utf8Path) -> Self {
        let Ok(text) = std::fs::read_to_string(path) else {
            info!("no build registry at `{path}`, starting fresh");
            return Self::default();
        };

        match serde_json::from_str::<Self>(&text) {
            Ok(registry) if registry.version == REGISTRY_VERSION => registry,
            Ok(_) => {
                info!("build registry version mismatch, rebuilding");
                Self::default()
            }
            Err(e) => {
                info!("corrupted build registry (`{e}`), rebuilding");
                Self::default()
            }
        }
    }

    /// Writes registry back to disk.
    pub fn save(&self, path: &Utf8Path) {
        let json = serde_json::to_string_pretty(self)
            .unwrap_or_else(|e| panic!("cannot serialize build registry: {e}"));
        io::write(path, &json);
    }

    /// Returns the id of a module, allocating a fresh one on first sight.
    pub fn intern(&mut self, key: &str) -> ModuleId {
        if let Some(id) = self.ids.get(key) {
            return ModuleId(*id);
        }

        let id = self.next_id;
        self.next_id += 1;
        self.ids.insert(key.to_string(), id);

        info!("assigned {:?} to module `{key}`", ModuleId(id));
        ModuleId(id)
    }

    /// Returns the fingerprints of the last successful build of a module.
    pub fn record(&self, key: &str) -> Option<&ModuleRecord> {
        self.records.get(key)
    }

    /// Stores fingerprints of a freshly compiled module.
    pub fn update(
        &mut self,
        key: &str,
        src_hash: u64,
        iface_hash: u64,
        deps: Vec<DepStamp>,
    ) {
        self.records.insert(
            key.to_string(),
            ModuleRecord {
                src_hash,
                iface_hash,
                deps,
            },
        );
    }

    /// Forgets modules that no longer exist on disk.
    pub fn prune(&mut self, alive: &HashSet<String>) {
        self.ids.retain(|key, _| alive.contains(key));
        self.records.retain(|key, _| alive.contains(key));
    }
}

/// FNV-1a 64.
pub fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in bytes {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}
