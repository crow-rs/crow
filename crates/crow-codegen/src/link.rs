use std::path::PathBuf;

use crate::{comp_ops::TargetConfig, error::LinkerError, sys_linker::SystemLinker};

#[derive(Clone, Copy, Debug)]
pub enum OutputKind {
    Executable,
    StaticLib,
    DynLib,
}

#[derive(Clone, Debug)]
pub enum LinkInput {
    Object(PathBuf),
    Archive(PathBuf),
    SystemLib(String),
    SearchPath(PathBuf),
}

#[derive(Clone, Debug)]
pub struct LinkConfig {
    pub output: PathBuf,
    pub output_kind: OutputKind,
    pub inputs: Vec<LinkInput>,
    pub linker: String,
    pub extra_args: Vec<String>,
    pub strip: bool,
    pub pie: bool,
}

impl LinkConfig {
    pub fn builder() -> LinkConfigBuilder {
        LinkConfigBuilder::new()
    }

    pub fn from_target(target: &TargetConfig) -> Self {
        Self {
            output: PathBuf::from(&target.output),
            output_kind: OutputKind::Executable,
            inputs: Vec::new(),
            linker: target.linker.clone(),
            extra_args: Vec::new(),
            strip: false,
            pie: true,
        }
    }
}

pub struct LinkConfigBuilder {
    config: LinkConfig,
}

impl LinkConfigBuilder {
    fn new() -> Self {
        Self {
            config: LinkConfig {
                output: PathBuf::from("a.out"),
                output_kind: OutputKind::Executable,
                inputs: Vec::new(),
                linker: "cc".to_string(),
                extra_args: Vec::new(),
                strip: false,
                pie: true,
            },
        }
    }

    pub fn output(mut self, path: impl Into<PathBuf>) -> Self {
        self.config.output = path.into();
        self
    }

    pub fn output_kind(mut self, kind: OutputKind) -> Self {
        self.config.output_kind = kind;
        self
    }

    pub fn object(mut self, path: impl Into<PathBuf>) -> Self {
        self.config.inputs.push(LinkInput::Object(path.into()));
        self
    }

    pub fn archive(mut self, path: impl Into<PathBuf>) -> Self {
        self.config.inputs.push(LinkInput::Archive(path.into()));
        self
    }

    pub fn system_lib(mut self, name: impl Into<String>) -> Self {
        self.config.inputs.push(LinkInput::SystemLib(name.into()));
        self
    }

    pub fn search_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.config.inputs.push(LinkInput::SearchPath(path.into()));
        self
    }

    pub fn linker(mut self, linker: impl Into<String>) -> Self {
        self.config.linker = linker.into();
        self
    }

    pub fn extra_arg(mut self, arg: impl Into<String>) -> Self {
        self.config.extra_args.push(arg.into());
        self
    }

    pub fn strip(mut self, strip: bool) -> Self {
        self.config.strip = strip;
        self
    }

    pub fn pie(mut self, pie: bool) -> Self {
        self.config.pie = pie;
        self
    }

    pub fn build(self) -> LinkConfig {
        self.config
    }
}

pub trait Linker {
    fn link(&self, config: &LinkConfig) -> Result<(), LinkerError>;
}

pub fn link(config: &LinkConfig) -> Result<(), LinkerError> {
    let linker = SystemLinker;
    linker.link(config)
}