/// Imports
use miette::{Diagnostic, NamedSource, SourceSpan};
use std::sync::Arc;
use thiserror::Error;

/// Typechecking related
#[derive(Debug, Error, Diagnostic)]
pub(crate) enum TypeckRelated {
    #[error("here...")]
    #[diagnostic(severity(hint))]
    Here {
        #[source_code]
        src: Arc<NamedSource<String>>,
        #[label()]
        span: SourceSpan,
    },
    #[error("this type is {t:?}")]
    #[diagnostic(severity(hint))]
    ThisType {
        #[source_code]
        src: Arc<NamedSource<String>>,
        #[label()]
        span: SourceSpan,
        t: String,
    },
}

/// Typechecking error
#[derive(Debug, Error, Diagnostic)]
pub(crate) enum TypeckError {
    #[error("types missmatch. expected `{expected}`, got `{got}`.")]
    #[diagnostic(code(typeck::types_missmatch))]
    TypesMissmatch {
        #[related]
        related: Vec<TypeckRelated>,
        expected: String,
        got: String,
    },
    #[error("found recursive type `{t}`.")]
    #[diagnostic(
        code(typeck::types_recursion),
        help("types recursion is not supported.")
    )]
    RecursiveType {
        #[related]
        related: Vec<TypeckRelated>,
        t: String,
    },
}

/// Exhaustiveness error
#[derive(Debug, Error, Diagnostic)]
pub enum ExError {
    #[error("enum patterns missmatch.")]
    #[diagnostic(code(ex::enum_patterns_missmatch))]
    EnumPatternsMissmatch {
        #[source_code]
        src: Arc<NamedSource<String>>,
        #[label("patterns are missmatched.")]
        span: SourceSpan,
    },
    #[error("enum fields missmatch.")]
    #[diagnostic(code(ex::enum_unwrap_fields_missmatch))]
    EnumUnwrapFieldsMissmatch {
        #[source_code]
        src: Arc<NamedSource<String>>,
        #[label("fields of patterns are missmatched.")]
        span: SourceSpan,
    },
}
