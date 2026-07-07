use miette::Diagnostic;
use thiserror::Error;

#[derive(Debug, Error, Diagnostic)]
pub enum CodegenError {
    #[error("found an imports cycle: `{a}` <-> `{b}`.")]
    #[diagnostic(code(driver::found_imports_cycle))]
    FoundImportsCycle { a: String, b: String },
}

#[derive(Debug, Error, Diagnostic)]
pub enum LinkerError {
    #[error("external linker error: `{err}`.")]
    #[diagnostic(code(driver::external_link_error))]
    ExternalInstrumentError { err: String },
}