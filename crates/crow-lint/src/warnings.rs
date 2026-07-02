use std::sync::Arc;

use miette::{Diagnostic, NamedSource, SourceSpan};
use thiserror::Error;

//todo - warnings

#[derive(Error, Diagnostic, Debug)]
pub enum LinterWarnings {
    #[error("unused result of: `{stmt:?}`")]
    #[diagnostic(code(linter::unused_result))]
    UnusedResult {
        stmt: String,
        #[source_code]
        src: Arc<NamedSource<String>>,
        #[label("unused result")]
        span: SourceSpan,
    },

    #[error("unused value: `{val_name:?}`")]
    #[diagnostic(code(linter::unused_value))]
    UnusedVariable {
        val_name: String,
        #[source_code]
        src: Arc<NamedSource<String>>,
        #[label("unused value")]
        span: SourceSpan,
    },

    #[error("variable no need to be mutable: `{var_name:?}`")]
    #[diagnostic(code(linter::unused_mut))]
    UnusedMut {
        var_name: String,
        #[source_code]
        src: Arc<NamedSource<String>>,
        #[label("unused mutability")]
        span: SourceSpan,
    },

    #[error("unreachable code detected")]
    #[diagnostic(code(linter::unreachable_code))]
    UnreachableCode {
        #[source_code]
        src: Arc<NamedSource<String>>,
        #[label("unreachable code")]
        span: SourceSpan,
    },
}