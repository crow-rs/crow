use std::sync::Arc;

use miette::{Diagnostic, NamedSource, SourceSpan};
use thiserror::Error;

/// Parser error
#[derive(Error, Diagnostic, Debug)]
pub enum TyCheckErrors {
    #[error("type missmatch. expected: `{expected:?} but got: `{got:?}`")]
    #[diagnostic(code(ty_check::type_missmatch))]
    TypeMissmatch {
        expected: String,
        got: String,
       // #[source_code]
       // src: Arc<NamedSource<String>>,
        #[label("type missmatch here...")]
        span: SourceSpan,
    },

    #[error("occurs checking. Vid `{vid:?}` for type `{ty:?}`")]
    #[diagnostic(code(ty_check::occurs_check))]
    OccursCheck {
        vid: String,
        ty: String,
       // #[source_code]
       // src: Arc<NamedSource<String>>,
        #[label("occurs check here...")]
        span: SourceSpan,
    },

    #[error("Arity missmatched while checking. Expected `{expected:?}` but found `{found:?}`")]
    #[diagnostic(code(ty_check::arity_missmatch))]
    ArityMissmatch {
        expected: usize,
        found: usize,
       //#[source_code]
        //src: Arc<NamedSource<String>>,
        #[label("Arity missmatched here...")]
        span: SourceSpan,
    },

    #[error("No such field for type `{ty:?}` field `{field:?}`")]
    #[diagnostic(code(ty_check::no_such_field))]
    NoSuchField {
        ty: String,
        field: String,
        //#[source_code]
        //src: Arc<NamedSource<String>>,
        #[label("no such field here...")]
        span: SourceSpan,
    },

    #[error("Reference to a type that does not represent a structured object `{ty:?}`")]
    #[diagnostic(code(ty_check::not_a_struct))]
    NotAStruct {
        ty: String,
        //#[source_code]
        //src: Arc<NamedSource<String>>,
        #[label("illegal accsess here...")]
        span: SourceSpan,
    },

    #[error("Invalid type conversion")]
    #[diagnostic(code(ty_check::not_a_struct))]
    InvalidTypeConversion,

    #[error("Literal `{lit:?}` out of resolved range: `{range:?}`")]
    #[diagnostic(code(ty_check::literal_out_or_range))]
    LiteralOutOfRange {
        lit: String,
        range: String,
        //#[source_code]
        //src: Arc<NamedSource<String>>,
        #[label("illegal lit range here...")]
        span: SourceSpan,
    },
}