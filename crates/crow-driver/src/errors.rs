/// IMports
use miette::Diagnostic;
use thiserror::Error;

/// Driver error
#[derive(Debug, Error, Diagnostic)]
pub enum DriverError {
    #[error("found an imports cycle: `{a}` <-> `{b}`.")]
    #[diagnostic(code(driver::found_imports_cycle))]
    FoundImportsCycle { a: String, b: String },
}
