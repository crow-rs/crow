use inkwell::targets::TargetMachine;


#[derive(Clone, Copy)]
pub enum Arch {
    X86_64,
    Aarch64,
    Riscv64,
}

#[derive(Clone, Copy)]
pub enum Os {
    Linux,
    MacOs,
    Windows,
    None,  // bare metal
}

#[derive(Clone, Copy)]
pub enum RelocModel {
    Static,
    Pic,
}

#[derive(Clone, Copy)]
pub enum EmitKind {
    LlvmIr,     // .ll
    Object,      // .o
    Assembly,    // .s
    Executable,  // link
}

pub struct TargetConfig {
    pub triple: String,
    pub arch: Arch,
    pub os: Os,
    pub linker: String,
    pub pointer_width: u32,
    pub cpu: String,
    pub features: String,
    pub reloc_model: RelocModel,
    pub output: String,
    pub emit: EmitKind,
}

impl TargetConfig {
    pub fn host() -> Self {
        let triple_raw = TargetMachine::get_default_triple();
        let triple = triple_raw.as_str().to_str().unwrap().to_string();
        let arch = detect_arch(&triple);
        let os = detect_os(&triple);
        Self {
            triple,
            arch,
            os,
            linker: default_linker(os),
            pointer_width: pointer_width(arch),
            cpu: "generic".to_string(),
            features: String::new(),
            reloc_model: RelocModel::Pic,
            output: "a.out".to_string(),
            emit: EmitKind::Executable,
        }
    }

    pub fn from_triple(triple: &str) -> Self {
        let os = detect_os(triple);
        let arch = detect_arch(&triple);
        Self {
            triple: triple.to_string(),
            arch,
            os,
            linker: default_linker(os),
            pointer_width: pointer_width(arch),
            cpu: "generic".to_string(),
            features: String::new(),
            reloc_model: RelocModel::Pic,
            output: "a.out".to_string(),
            emit: EmitKind::Executable,
        }
    }
}

fn default_linker(os: Os) -> String {
    match os {
        Os::Linux => "cc".to_string(),
        Os::MacOs => "cc".to_string(),
        Os::Windows => "link.exe".to_string(),
        Os::None => "ld.lld".to_string(),
    }
}

fn detect_arch(triple: &str) -> Arch {
    let arch_part = triple.split('-').next().unwrap_or("");
    match arch_part {
        "x86_64" | "amd64" => Arch::X86_64,
        "aarch64" | "arm64" => Arch::Aarch64,
        "riscv64" => Arch::Riscv64,
        other => panic!("unsupported host arch: {other}"),
    }
}

fn detect_os(triple: &str) -> Os {
    let parts: Vec<&str> = triple.split('-').collect();
    let os_candidates = &parts[1..];
    for part in os_candidates {
        match *part {
            "linux" => return Os::Linux,
            "darwin" | "macos" => return Os::MacOs,
            "windows" | "win32" => return Os::Windows,
            "none" | "unknown" if parts.last() == Some(&"elf") => return Os::None,
            _ => continue,
        }
    }
    Os::Linux
}


fn pointer_width(arch: Arch) -> u32 {
    match arch {
        Arch::X86_64 | Arch::Aarch64 | Arch::Riscv64 => 64,
    }
}