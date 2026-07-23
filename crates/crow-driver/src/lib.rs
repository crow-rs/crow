/// Modules
mod errors;
mod io;

use camino::{Utf8Path, Utf8PathBuf};
use crow_ast::item::{Item, Module, Use};
use crow_codegen::{codegen_module, comp_ops::TargetConfig, linker};
use crow_collect_spec_item::collect_special_items;
use crow_common::{bail, bug, emit};
use crow_lex::{Lexer, token::TokenKind};
use crow_lint::run_lints;
use crow_lower_hir::lower_module;
use crow_lower_mir::lower_hir_to_mir;
use crow_mir::{FnId, MirModule, verify};
use crow_mir_monomorph::{Monomorph};
use crow_mir_passes::mir_optimize;
use crow_mod_codec_ty::{ModuleTop, load_crowi, save_crowi};
use crow_module_codec::{serialize_module_info};
use crow_parse::Parser;
use crow_resolving::resolver::Resolver;
use crow_tycheck::typeck::typeck_module;
use miette::NamedSource;
use std::{
    collections::{HashMap, HashSet, VecDeque}, format, path::Path, println, sync::Arc,
};
use tracing::info;

#[derive(Clone, Debug)]
pub struct ModuleInfo {
    pub name: String,
    pub files: Vec<Utf8PathBuf>,
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
            ModuleKind::Package => {
                self.files.iter()
                    .find(|f| f.file_name() == Some("mod.cw"))
                    .unwrap()
            }
        }
    }
}

/// Driver configuration
pub struct DriverConfig {
    /// Compilation income path
    income: Utf8PathBuf,

    /// Compilation outcome path
    outcome: Utf8PathBuf,
}

/// Orchestrator config implementation
impl DriverConfig {
    /// Creates new orchestrator config
    pub fn new(income: Utf8PathBuf, outcome: Utf8PathBuf) -> Self {
        Self { income, outcome }
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

    fn discover_modules(&self, src_dir: &Utf8Path) -> Vec<ModuleInfo> {
        let mut modules = Vec::new();
        let mut package_dirs: HashSet<Utf8PathBuf> = HashSet::new();

        // First pass: find package modules (directories with mod.cw)
        for entry in self.walkdir(src_dir) {
            if entry.file_name() == Some("mod.cw") {
                let dir = entry.parent().unwrap().to_path_buf();
                let name = dir.file_name().unwrap().to_string();

                let files: Vec<Utf8PathBuf> = self.read_dir(&dir)
                    .into_iter()
                    .filter(|f| f.extension() == Some("cw"))
                    .collect();

                package_dirs.insert(dir);

                modules.push(ModuleInfo {
                    name,
                    files,
                    kind: ModuleKind::Package,
                });
            }
        }

        // Second pass: find single-file modules
        for entry in self.read_dir(src_dir) {
            if entry.extension() != Some("cw") { continue; }
            if entry.file_name() == Some("mod.cw") { continue; }

            let stem = entry.file_stem().unwrap().to_string();

            let potential_dir = src_dir.join(&stem);
            if package_dirs.contains(&potential_dir) {
                panic!(
                    "module conflict: both {}.cw and {}/mod.cw exist",
                    stem, stem
                );
            }

            modules.push(ModuleInfo {
                name: stem,
                files: vec![entry],
                kind: ModuleKind::SingleFile,
            });
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
                _ => { i += 1; }
            }
        }

        (module_name, deps)
    }

    fn build_dep_graph(
        &self,
        modules: &[(ModuleInfo, Vec<String>)],
    ) -> HashMap<String, Vec<String>> {
        let known: HashSet<&str> = modules.iter()
            .map(|(m, _)| m.name.as_str())
            .collect();

        let mut graph = HashMap::new();

        for (module, deps) in modules {
            let valid_deps: Vec<String> = deps.iter()
                .filter(|d| {
                    if !known.contains(d.as_str()) {
                        eprintln!(
                            "warning: module `{}` imports unknown module `{}`",
                            module.name, d
                        );
                        false
                    } else {
                        true
                    }
                })
                .cloned()
                .collect();

            graph.insert(module.name.clone(), valid_deps);
        }

        graph
    }

    fn toposort(&self, graph: &HashMap<String, Vec<String>>) -> Vec<String> {
        let mut dependents: HashMap<&str, Vec<&str>> = HashMap::new();
        let mut in_degree: HashMap<&str, usize> = HashMap::new();

        for name in graph.keys() {
            in_degree.entry(name.as_str()).or_insert(0);
            dependents.entry(name.as_str()).or_default();
        }

        for (name, deps) in graph {
            *in_degree.entry(name.as_str()).or_insert(0) += deps.len();
            for dep in deps {
                dependents.entry(dep.as_str()).or_default().push(name.as_str());
            }
        }

        let mut queue: VecDeque<&str> = in_degree.iter()
            .filter(|(_, deg)| **deg == 0)
            .map(|(name, _)| *name)
            .collect();

        let mut result = Vec::new();

        while let Some(node) = queue.pop_front() {
            result.push(node.to_string());

            if let Some(users) = dependents.get(node) {
                for user in users {
                    let deg = in_degree.get_mut(user).unwrap();
                    *deg -= 1;
                    if *deg == 0 {
                        queue.push_back(user);
                    }
                }
            }
        }

        if result.len() != graph.len() {
            let in_cycle: Vec<&str> = in_degree.iter()
                .filter(|(_, deg)| **deg > 0)
                .map(|(name, _)| *name)
                .collect();
            panic!("import cycle detected: {}", in_cycle.join(" → "));
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

    pub fn compile(&self) {
        println!("⌛ Compiling...");

        // 1. Discover modules
        info!("discovering modules in `{}`", self.config.income);
        let modules = self.discover_modules(&self.config.income);

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
        
        let mut build_cfg = TargetConfig::host();
        build_cfg.output = "/home/f0rits/Documents/crow/test/".to_string();

        // 3. Compile each module in order
        let mut had_errors = false;
        let mut objects = Vec::new();
        for name in &order {
            let (info, deps) = dep_info.iter()
                .find(|(m, _)| &m.name == name)
                .unwrap();

            let deps_meta: Vec<ModuleTop> = deps.iter().map(|dep| {
                load_crowi(Path::new(&format!("/home/f0rits/Documents/crow/test/{}.crowi", dep)))
            }).collect();

            println!("{:?}", deps);

            match self.compile_module(info, &deps_meta, &build_cfg) {
                Err(()) => {
                    had_errors = true;
                    break;
                }
                Ok(path) => objects.push(path),
            }
        }

        if had_errors { return; }

        linker(&build_cfg, &objects, "/home/f0rits/Documents/crow/test/test_exec".into());

        println!("✨ Done!");
    }

    fn compile_module(&self, info: &ModuleInfo, deps: &[ModuleTop], build_cfg: &TargetConfig) -> Result<Utf8PathBuf, ()> {
        let name = &info.name;
        info!("compiling module `{name}`");

        // Parse
        let module = self.parse_module(info);

        // Resolve
        let mut resolver = Resolver::new();
        resolver.inject_imports(deps);
        let resolve = match resolver.resolve_ast(&module) {
            Ok(r) => r,
            Err(errors) => {
                for err in errors { emit!(err); }
                return Err(());
            }
        };

        // HIR
        let hir = lower_module(&module, resolve);

        // Typeck
        let (typeck_output, typeck_errors) = typeck_module(&hir, deps);
        if !typeck_errors.is_empty() {
            for err in typeck_errors { emit!(err); }
            return Err(());
        }

        let exports = serialize_module_info(&info.name, &hir, &typeck_output);
        let crowi_path = format!("/home/f0rits/Documents/crow/test/{}.crowi", info.name);
        save_crowi(std::path::Path::new(&crowi_path), &exports);

        // Lint
        let warnings = run_lints(&hir, &typeck_output);
        for wrn in warnings { emit!(wrn); }

        // Collect lang defs
        let mut items = collect_special_items(&hir);
        let errors = std::mem::take(&mut items.errors);
        if !errors.is_empty() {
            for error in errors { emit!(error); }
            return Err(());
        }

        // MIR
        let mut mir = lower_hir_to_mir(&hir, &typeck_output, &items);

        // MIR optimize
        mir_optimize(&mut mir);
        
        // Verify
        if let Err(errs) = verify(&mir) {
            for err in errs { println!("{err}"); }
            return Err(());
        }

        let mut mono = Monomorph::new();

        if info.name == "main" {
            let entry_id = mir.functions.iter()
                .enumerate()
                .find(|(_, body)| body.name == "main")
                .map(|(i, _)| FnId::from(i))
                .expect("no `main` function found");
            mono.collect(&mir, entry_id);
        }  else {
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
        let obj_path: String = format!("/home/f0rits/Documents/crow/test/{}.o", info.name);
        codegen_module(&mir, &items, info.name.as_str(), &build_cfg, obj_path.clone());

        info!("module `{name}` compiled successfully");

        Ok(Utf8PathBuf::from(obj_path))
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

                let source = Arc::new(NamedSource::new(
                    info.name.clone(),
                    full_code.clone(),
                ));
                let lexer = Lexer::new(source.clone(), &full_code);
                let mut parser = Parser::new(source, lexer);
                parser.parse()
            }
        }
    }
}