/// Imports
use miette::{Diagnostic, NamedSource, SourceSpan};
use std::sync::Arc;
use thiserror::Error;

/// Parser error
#[derive(Error, Diagnostic, Debug)]
pub enum TyCheckError {
    #[error("type missmatch. expected: `{expected:?} but got: `{got:?}`")]
    #[diagnostic(code(ty_check::type_missmatch))]
    TypeMissmatch {
        expected: String,
        got: String,
        #[source_code]
        src: Arc<NamedSource<String>>,
        #[label("type missmatch here...")]
        span: SourceSpan,
    },
    #[error("occurs check failure. vid `{vid:?}` for type `{ty:?}`")]
    #[diagnostic(code(ty_check::occurs_check))]
    OccursCheck {
        vid: String,
        ty: String,
        #[source_code]
        src: Arc<NamedSource<String>>,
        #[label("occurs check here...")]
        span: SourceSpan,
    },
    #[error(
        "arity missmatch. expected: `{expected:?}` but found `{found:?}`"
    )]
    #[diagnostic(code(ty_check::arity_missmatch))]
    ArityMissmatch {
        expected: usize,
        found: usize,
        #[source_code]
        src: Arc<NamedSource<String>>,
        #[label("Arity missmatched here...")]
        span: SourceSpan,
    },
    #[error("no field `{field:?}` for type `{ty:?}``")]
    #[diagnostic(code(ty_check::no_such_field))]
    NoSuchField {
        ty: String,
        field: String,
        #[source_code]
        src: Arc<NamedSource<String>>,
        #[label("no such field here...")]
        span: SourceSpan,
    },
    #[error("missing field `{field:?}` for type `{ty:?}``")]
    #[diagnostic(code(ty_check::missing_filed))]
    MissingField {
        ty: String,
        field: String,
        #[source_code]
        src: Arc<NamedSource<String>>,
        #[label("no such field here...")]
        span: SourceSpan,
    },
    #[error("duplicating field for type `{ty:?}``")]
    #[diagnostic(code(ty_check::duplicated_field))]
    DuplicateField {
        ty: String,
        #[source_code]
        src: Arc<NamedSource<String>>,
        #[label("no such field here...")]
        span: SourceSpan,
    },
    #[error("type `{ty:?}` is not a struct")]
    #[diagnostic(code(ty_check::not_a_struct))]
    NotAStruct {
        ty: String,
        #[source_code]
        src: Arc<NamedSource<String>>,
        #[label("illegal accsess here...")]
        span: SourceSpan,
    },
    #[error("invalid type conversion")]
    #[diagnostic(code(ty_check::invalid_type_conversion))]
    InvalidTypeConversion,
    #[error("literal `{lit:?}` out of resolved range: `{range:?}`")]
    #[diagnostic(code(ty_check::literal_out_or_range))]
    LiteralOutOfRange {
        lit: String,
        range: String,
        #[source_code]
        src: Arc<NamedSource<String>>,
        #[label("illegal lit range here...")]
        span: SourceSpan,
    },
}
