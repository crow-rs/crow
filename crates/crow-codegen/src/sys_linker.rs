use std::process::Command;
use crate::{error::LinkerError, link::{LinkConfig, LinkInput, Linker, OutputKind}};

pub struct SystemLinker;

impl Linker for SystemLinker {
    fn link(&self, config: &LinkConfig) -> Result<(), LinkerError> {
        let mut cmd = Command::new(&config.linker);

        // Output
        cmd.arg("-o").arg(&config.output);

        // Output kind
        match config.output_kind {
            OutputKind::Executable => {}
            OutputKind::StaticLib => {
                return self.archive_static(config);
            }
            OutputKind::DynLib => {
                cmd.arg("-shared");
            }
        }

        // PIE
        if config.pie {
            cmd.arg("-pie");
        } else {
            cmd.arg("-no-pie");
        }

        // Strip
        if config.strip {
            cmd.arg("-s");
        }

        // Inputs
        for input in &config.inputs {
            match input {
                LinkInput::Object(path) => { cmd.arg(path); }
                LinkInput::Archive(path) => { cmd.arg(path); }
                LinkInput::SystemLib(name) => { cmd.arg(format!("-l{name}")); }
                LinkInput::SearchPath(path) => { cmd.arg("-L").arg(path); }
            }
        }

        // Extra args
        for arg in &config.extra_args {
            cmd.arg(arg);
        }

        // Invoke
        let output = cmd.output().map_err(|e| {
            LinkerError::ExternalInstrumentError { err: e.to_string() }
        })?;

        if !output.status.success() {
            return Err(LinkerError::ExternalInstrumentError {
                err: String::from_utf8_lossy(&output.stderr).to_string(),
            });
        }

        Ok(())
    }
}

impl SystemLinker {
    fn archive_static(&self, config: &LinkConfig) -> Result<(), LinkerError> {
        let mut cmd = Command::new("ar");
        cmd.arg("rcs").arg(&config.output);

        for input in &config.inputs {
            if let LinkInput::Object(path) = input {
                cmd.arg(path);
            }
        }

        let output = cmd.output().map_err(|e| {
            LinkerError::ExternalInstrumentError { err: e.to_string() }
        })?;

        if !output.status.success() {
            return Err(LinkerError::ExternalInstrumentError {
                err: String::from_utf8_lossy(&output.stderr).to_string(),
            });
        }

        Ok(())
    }
}