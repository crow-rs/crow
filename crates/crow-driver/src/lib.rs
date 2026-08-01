/// Modules
mod errors;
mod io;
mod registry;

use camino::{Utf8Path, Utf8PathBuf};
use crow_ast::item::Module;
use crow_codegen::{codegen_module, comp_ops::TargetConfig, linker};
use crow_collect_spec_item::collect_special_items;
use crow_common::{ModuleId, emit};
use crow_lex::{Lexer, token::TokenKind};
use crow_lint::run_lints;
use crow_lower_hir::lower_module;
use crow_lower_mir::lower_hir_to_mir;
use crow_mir::{FnId, verify};
use crow_mir_monomorph::Monomorph;
use crow_mir_passes::mir_optimize;
use crow_mod_codec_ty::{ModuleTop, load_crowi, save_crowi};
use crow_module_codec::serialize_module_info;
use crow_parse::Parser;
use crow_resolving::resolver::Resolver;
use crow_tycheck::typeck::typeck_module;
use miette::NamedSource;
use registry::{BuildRegistry, DepStamp, fnv1a};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::Arc,
};
use tracing::info;

/// Single compilation unit.
///
/// `id` is the module's identity for the whole compiler: `DefId` is
/// `(ModuleId, LocalDefId)`, so every definition produced while compiling this
/// module is tagged with this id and stays distinguishable from a definition
/// of another module that happens to have the same name.
#[derive(Clone, Debug)]
pub struct ModuleInfo {
    /// Stable id, allocated by the build registry
    pub id: ModuleId,

    /// Import-facing name — what `use <name>` refers to
    pub name: String,

    /// Identity key: path relative to the source root.
    ///
    /// Unlike `name` it is unique by construction, so the registry keys
    /// module ids on it and nested packages never collide.
    pub key: String,

    /// Source files belonging to the module
    pub files: Vec<Utf8PathBuf>,

    /// Module kind
    pub kind: ModuleKind,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ModuleKind {
    SingleFile,
    Package,
}

impl ModuleInfo {
    pub fn root_file(&self) -> &Utf8PathBuf {
        match self.kind {
            ModuleKind::SingleFile => &self.files[0],
            ModuleKind::Package => self
                .files
                .iter()
                .find(|f| f.file_name() == Some("mod.cw"))
                .unwrap(),
        }
    }
}

/// Result of looking at a module before compiling it.
enum Freshness {
    /// Sources and all dependency interfaces are unchanged — artifacts reused.
    UpToDate { iface_hash: u64 },

    /// Something moved: the module has to go through the whole pipeline.
    Stale { reason: String },
}

/// Driver configuration
pub struct DriverConfig {
    /// Compilation income path
    income: Utf8PathBuf,

    /// Compilation outcome path
    outcome: Utf8PathBuf,

    /// Name of the linked executable, relative to `outcome`
    exec_name: String,
}

/// Orchestrator config implementation
impl DriverConfig {
    /// Creates new orchestrator config
    pub fn new(income: Utf8PathBuf, outcome: Utf8PathBuf) -> Self {
        Self {
            income,
            outcome,
            exec_name: "a.out".to_string(),
        }
    }

    /// Overrides the name of the produced executable
    pub fn with_exec_name(mut self, name: impl Into<String>) -> Self {
        self.exec_name = name.into();
        self
    }
}

/// Represents orchestrator of the compilation or analysis
pub struct Driver {
    config: DriverConfig,
}

/// Driver implementation
impl Driver {
    /// Creates new orchestrator
    pub fn new(config: DriverConfig) -> Self {
        Self { config }
    }

    /// Directory holding incremental build state
    fn build_dir(&self) -> Utf8PathBuf {
        self.config.outcome.join(".crow-build")
    }

    /// Path of the registry file
    fn registry_path(&self) -> Utf8PathBuf {
        self.build_dir().join("registry.json")
    }

    /// Path of the interface artifact of a module
    fn crowi_path(&self, info: &ModuleInfo) -> Utf8PathBuf {
        self.config.outcome.join(format!("{}.crowi", info.name))
    }

    /// Path of the object file of a module
    fn object_path(&self, info: &ModuleInfo) -> Utf8PathBuf {
        self.config.outcome.join(format!("{}.o", info.name))
    }

    /// Path of the linked executable
    fn exec_path(&self) -> Utf8PathBuf {
        self.config.outcome.join(&self.config.exec_name)
    }

    fn walkdir(&self, dir: &Utf8Path) -> Vec<Utf8PathBuf> {
        let mut result = Vec::new();
        self.walkdir_inner(dir, &mut result);
        result
    }

    fn read_dir(&self, dir: &Utf8Path) -> Vec<Utf8PathBuf> {
        std::fs::read_dir(dir.as_std_path())
            .unwrap_or_else(|e| panic!("cannot read dir {}: {}", dir, e))
            .filter_map(|entry| {
                let entry = entry.ok()?;
                Utf8PathBuf::from_path_buf(entry.path()).ok()
            })
            .collect()
    }

    fn walkdir_inner(&self, dir: &Utf8Path, result: &mut Vec<Utf8PathBuf>) {
        for entry in self.read_dir(dir) {
            if entry.is_dir() {
                self.walkdir_inner(&entry, result);
            } else {
                result.push(entry);
            }
        }
    }

    /// Identity key of a module: its path relative to the source root
    fn module_key(&self, src_dir: &Utf8Path, path: &Utf8Path) -> String {
        path.strip_prefix(src_dir)
            .unwrap_or(path)
            .to_string()
            .replace('\\', "/")
    }

    /// Discovers modules and assigns each of them a stable id.
    ///
    /// Ids come from the persistent registry, so a module keeps the same id
    /// across builds and cached `.crowi` files stay meaningful.
    fn discover_modules(
        &self,
        src_dir: &Utf8Path,
        registry: &mut BuildRegistry,
    ) -> Vec<ModuleInfo> {
        let mut modules = Vec::new();
        let mut package_dirs: HashSet<Utf8PathBuf> = HashSet::new();

        // First pass: find package modules (directories with mod.cw)
        for entry in self.walkdir(src_dir) {
            if entry.file_name() == Some("mod.cw") {
                let dir = entry.parent().unwrap().to_path_buf();
                let name = dir.file_name().unwrap().to_string();
                let key = self.module_key(src_dir, &dir);

                let mut files: Vec<Utf8PathBuf> = self
                    .read_dir(&dir)
                    .into_iter()
                    .filter(|f| f.extension() == Some("cw"))
                    .collect();

                // Fingerprints must not depend on readdir order
                files.sort();

                package_dirs.insert(dir);

                let id = registry.intern(&key);
                modules.push(ModuleInfo {
                    id,
                    name,
                    key,
                    files,
                    kind: ModuleKind::Package,
                });
            }
        }

        // Second pass: find single-file modules
        for entry in self.read_dir(src_dir) {
            if entry.extension() != Some("cw") {
                continue;
            }
            if entry.file_name() == Some("mod.cw") {
                continue;
            }

            let stem = entry.file_stem().unwrap().to_string();

            let potential_dir = src_dir.join(&stem);
            if package_dirs.contains(&potential_dir) {
                panic!(
                    "module conflict: both {}.cw and {}/mod.cw exist",
                    stem, stem
                );
            }

            let key = self.module_key(src_dir, &entry);
            let id = registry.intern(&key);
            modules.push(ModuleInfo {
                id,
                name: stem,
                key,
                files: vec![entry],
                kind: ModuleKind::SingleFile,
            });
        }

        // `use` resolves by name, so two modules sharing one is ambiguous
        // even though their ids differ
        let mut seen: HashMap<&str, &str> = HashMap::new();
        for module in &modules {
            if let Some(other) = seen.insert(&module.name, &module.key) {
                eprintln!(
                    "warning: modules `{}` and `{}` share the name `{}`; \
                     imports of it are ambiguous",
                    other, module.key, module.name
                );
            }
        }

        modules
    }

    fn scan_imports(&self, path: &Utf8Path) -> (String, Vec<String>) {
        let code: String = std::fs::read_to_string(path.as_std_path())
            .unwrap_or_else(|e| panic!("cannot read {}: {}", path, e));

        let mut module_name = path.file_stem().unwrap_or("unknown").to_string();
        let mut deps = Vec::new();

        let source = Arc::new(NamedSource::new(path.to_string(), code.clone()));
        let lexer = Lexer::new(source, &code);
        let tokens: Vec<_> = lexer.collect();
        let mut i = 0;

        while i < tokens.len() {
            match tokens[i].kind {
                TokenKind::Module => {
                    i += 1;
                    if i < tokens.len() && tokens[i].kind == TokenKind::Id {
                        module_name = tokens[i].lexeme.clone();
                    }
                    i += 1;
                }
                TokenKind::Use => {
                    i += 1;
                    if i < tokens.len() && tokens[i].kind == TokenKind::Id {
                        let dep = tokens[i].lexeme.clone();
                        if !deps.contains(&dep) {
                            deps.push(dep);
                        }
                    }
                    i += 1;
                }
                TokenKind::Fun
                | TokenKind::Rec
                | TokenKind::Enum
                | TokenKind::Native
                | TokenKind::Pub
                | TokenKind::AtSign => break,
                _ => {
                    i += 1;
                }
            }
        }

        (module_name, deps)
    }

    /// Builds the dependency graph over module ids.
    ///
    /// Names are only an import-time convenience; from here on everything —
    /// ordering, fingerprints, artifact lookup — works on ids.
    fn build_dep_graph(
        &self,
        modules: &[(ModuleInfo, Vec<String>)],
    ) -> HashMap<ModuleId, Vec<ModuleId>> {
        let by_name: HashMap<&str, ModuleId> = modules
            .iter()
            .map(|(m, _)| (m.name.as_str(), m.id))
            .collect();

        let mut graph = HashMap::new();

        for (module, deps) in modules {
            let valid_deps: Vec<ModuleId> = deps
                .iter()
                .filter_map(|d| match by_name.get(d.as_str()) {
                    Some(id) => Some(*id),
                    None => {
                        eprintln!(
                            "warning: module `{}` imports unknown module `{}`",
                            module.name, d
                        );
                        None
                    }
                })
                .collect();

            graph.insert(module.id, valid_deps);
        }

        graph
    }

    fn toposort(&self, graph: &HashMap<ModuleId, Vec<ModuleId>>) -> Vec<ModuleId> {
        let mut dependents: HashMap<ModuleId, Vec<ModuleId>> = HashMap::new();
        let mut in_degree: HashMap<ModuleId, usize> = HashMap::new();

        for id in graph.keys() {
            in_degree.entry(*id).or_insert(0);
            dependents.entry(*id).or_default();
        }

        for (id, deps) in graph {
            *in_degree.entry(*id).or_insert(0) += deps.len();
            for dep in deps {
                dependents.entry(*dep).or_default().push(*id);
            }
        }

        let mut queue: VecDeque<ModuleId> = in_degree
            .iter()
            .filter(|(_, deg)| **deg == 0)
            .map(|(id, _)| *id)
            .collect();

        let mut result = Vec::new();

        while let Some(node) = queue.pop_front() {
            result.push(node);

            if let Some(users) = dependents.get(&node) {
                for user in users.clone() {
                    let deg = in_degree.get_mut(&user).unwrap();
                    *deg -= 1;
                    if *deg == 0 {
                        queue.push_back(user);
                    }
                }
            }
        }

        if result.len() != graph.len() {
            let in_cycle: Vec<String> = in_degree
                .iter()
                .filter(|(_, deg)| **deg > 0)
                .map(|(id, _)| format!("{:?}", id))
                .collect();
            panic!("import cycle detected: {}", in_cycle.join(" -> "));
        }

        result
    }

    fn load_module(&self, path: Utf8PathBuf) -> (String, Module) {
        info!("loading module `{path}`");
        let name = io::module_name(&self.config.income, &path);
        let code = io::read(&path);
        let source = Arc::new(NamedSource::new(name.clone(), code.clone()));

        let lexer = Lexer::new(source.clone(), &code);
        let mut parser = Parser::new(source, lexer);
        let ast = parser.parse();

        info!("loaded module `{path}` with name `{name}`");
        (name, ast)
    }

    /// Hashes every source file of a module.
    ///
    /// Relative names are part of the hash, so adding or renaming a file
    /// inside a package invalidates the module too.
    fn hash_sources(&self, info: &ModuleInfo) -> u64 {
        let mut hash = fnv1a(info.key.as_bytes());
        for file in &info.files {
            let name = self.module_key(&self.config.income, file);
            hash ^= fnv1a(name.as_bytes());
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
            hash ^= fnv1a(io::read(file).as_bytes());
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
        hash
    }

    /// Decides whether a module can be skipped.
    ///
    /// A module is up to date only when its own sources are unchanged AND every
    /// dependency exposes exactly the interface it was compiled against — the
    /// latter is why `iface_hash` is kept separately from `src_hash`: a
    /// dependency may be recompiled without its public surface moving, and then
    /// its dependents do not have to be touched at all.
    fn freshness(
        &self,
        info: &ModuleInfo,
        src_hash: u64,
        dep_stamps: &[DepStamp],
        registry: &BuildRegistry,
    ) -> Freshness {
        let Some(record) = registry.record(&info.key) else {
            return Freshness::Stale {
                reason: "never compiled".to_string(),
            };
        };

        if record.src_hash != src_hash {
            return Freshness::Stale {
                reason: "sources changed".to_string(),
            };
        }

        if record.deps != dep_stamps {
            return Freshness::Stale {
                reason: "dependency interface changed".to_string(),
            };
        }

        let crowi = self.crowi_path(info);
        let object = self.object_path(info);
        if !crowi.exists() || !object.exists() {
            return Freshness::Stale {
                reason: "artifacts missing".to_string(),
            };
        }

        Freshness::UpToDate {
            iface_hash: record.iface_hash,
        }
    }

    pub fn compile(&self) {
        println!("⌛ Compiling...");

        io::mkdir_all(&self.config.outcome);
        io::mkdir_all(&self.build_dir());

        // 0. Load incremental state
        let mut registry = BuildRegistry::load(&self.registry_path());

        // 1. Discover modules and give each of them a stable id
        info!("discovering modules in `{}`", self.config.income);
        let modules = self.discover_modules(&self.config.income, &mut registry);
        registry.prune(&modules.iter().map(|m| m.key.clone()).collect());

        // 2. Scan imports + build dep graph
        info!("scanning imports...");
        let dep_info: Vec<(ModuleInfo, Vec<String>)> = modules
            .into_iter()
            .map(|m| {
                let (_, deps) = self.scan_imports(m.root_file());
                (m, deps)
            })
            .collect();

        let graph = self.build_dep_graph(&dep_info);
        let order = self.toposort(&graph);
        info!("compilation order: {:?}", order);

        let by_id: HashMap<ModuleId, &ModuleInfo> =
            dep_info.iter().map(|(m, _)| (m.id, m)).collect();

        let mut build_cfg = TargetConfig::host();
        build_cfg.output = self.config.outcome.to_string();

        // 3. Compile each module in order, skipping the untouched ones
        let mut had_errors = false;
        let mut relink = !self.exec_path().exists();
        let mut objects = Vec::new();
        let mut iface_hashes: HashMap<ModuleId, u64> = HashMap::new();

        for id in &order {
            let info = by_id[id];
            let deps = &graph[id];

            // Fingerprint of the module and of the interfaces it sees.
            // Topological order guarantees every dependency already has one.
            let src_hash = self.hash_sources(info);
            let dep_stamps: Vec<DepStamp> = deps
                .iter()
                .map(|dep| DepStamp {
                    id: dep.0,
                    iface_hash: iface_hashes[dep],
                })
                .collect();

            match self.freshness(info, src_hash, &dep_stamps, &registry) {
                Freshness::UpToDate { iface_hash } => {
                    println!("  ↺ {} (up to date)", info.name);
                    iface_hashes.insert(info.id, iface_hash);
                    objects.push(self.object_path(info));
                    continue;
                }
                Freshness::Stale { reason } => {
                    println!("  → {} ({reason})", info.name);
                }
            }

            let deps_meta: Vec<ModuleTop> = deps
                .iter()
                .map(|dep| load_crowi(self.crowi_path(by_id[dep]).as_std_path()))
                .collect();

            match self.compile_module(info, &deps_meta, &build_cfg) {
                Err(()) => {
                    had_errors = true;
                    break;
                }
                Ok(object) => {
                    let iface_hash =
                        fnv1a(io::read(&self.crowi_path(info)).as_bytes());
                    iface_hashes.insert(info.id, iface_hash);
                    registry.update(&info.key, src_hash, iface_hash, dep_stamps);
                    objects.push(object);
                    relink = true;
                }
            }
        }

        // Whatever succeeded stays cached even if a later module failed
        registry.save(&self.registry_path());

        if had_errors {
            return;
        }

        if relink {
            linker(&build_cfg, &objects, &self.exec_path());
        }

        println!("✨ Done!");
    }

    fn compile_module(
        &self,
        info: &ModuleInfo,
        deps: &[ModuleTop],
        build_cfg: &TargetConfig,
    ) -> Result<Utf8PathBuf, ()> {
        let name = &info.name;
        info!("compiling module `{name}` ({:?})", info.id);

        // Parse
        let module = self.parse_module(info);

        // Resolve
        //
        // The id is handed to the resolver because that is where `DefId`s are
        // born: every definition of this module is tagged with `info.id`.
        let mut resolver = Resolver::new(info.id);
        resolver.inject_imports(deps);
        let resolve = match resolver.resolve_ast(&module) {
            Ok(r) => r,
            Err(errors) => {
                for err in errors {
                    emit!(err);
                }
                return Err(());
            }
        };

        // HIR
        let hir = lower_module(&module, resolve);

        // Typeck
        let (typeck_output, typeck_errors) = typeck_module(&hir, deps);
        if !typeck_errors.is_empty() {
            for err in typeck_errors {
                emit!(err);
            }
            return Err(());
        }

        let exports =
            serialize_module_info(info.id, &info.name, &hir, &typeck_output);
        save_crowi(self.crowi_path(info).as_std_path(), &exports);

        // Lint
        let warnings = run_lints(&hir, &typeck_output);
        for wrn in warnings {
            emit!(wrn);
        }

        // Collect lang defs
        let mut items = collect_special_items(&hir);
        let errors = std::mem::take(&mut items.errors);
        if !errors.is_empty() {
            for error in errors {
                emit!(error);
            }
            return Err(());
        }

        // MIR
        let mut mir = lower_hir_to_mir(&hir, &typeck_output, &items);

        // MIR optimize
        mir_optimize(&mut mir);

        // Verify
        if let Err(errs) = verify(&mir) {
            for err in errs {
                println!("{err}");
            }
            return Err(());
        }

        let mut mono = Monomorph::new();

        if info.name == "main" {
            let entry_id = mir
                .functions
                .iter()
                .enumerate()
                .find(|(_, body)| body.name == "main")
                .map(|(i, _)| FnId::from(i))
                .expect("no `main` function found");
            mono.collect(&mir, entry_id);
        } else {
            for (i, body) in mir.functions.iter().enumerate() {
                // TODO: проверять publicity
                // Пока — все функции с телом
                if !body.blocks.is_empty() {
                    mono.collect(&mir, FnId::from(i));
                }
            }
        }

        mono.add_lang_items(&mir);
        let items = mono.into_items();

        // 5. Codegen
        let obj_path = self.object_path(info);
        codegen_module(
            &mir,
            &items,
            info.name.as_str(),
            &build_cfg,
            obj_path.to_string(),
        );

        info!("module `{name}` compiled successfully");

        Ok(obj_path)
    }

    fn parse_module(&self, info: &ModuleInfo) -> Module {
        match info.kind {
            ModuleKind::SingleFile => {
                let (_, module) = self.load_module(info.files[0].clone());
                module
            }
            ModuleKind::Package => {
                let mut full_code = String::new();
                for path in &info.files {
                    let code = io::read(path);
                    full_code.push_str(&code);
                    full_code.push('\n');
                }

                let source =
                    Arc::new(NamedSource::new(info.name.clone(), full_code.clone()));
                let lexer = Lexer::new(source.clone(), &full_code);
                let mut parser = Parser::new(source, lexer);
                parser.parse()
            }
        }
    }
}
