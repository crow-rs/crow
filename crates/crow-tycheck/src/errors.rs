/// Imports
use crow_ast::atom::{BinOp, UnOp};
use miette::{Diagnostic, NamedSource, SourceSpan};
use std::sync::Arc;
use thiserror::Error;

/// Typechecking error
#[derive(Debug, Error, Diagnostic)]
pub(crate) enum TypeckError {
    #[error("types missmatch. expected `{expected}`, got `{got}`.")]
    #[diagnostic(code(typeck::types_missmatch))]
    TypesMissmatch {
        #[source_code]
        src: Arc<NamedSource<String>>,
        #[label("here...")]
        span: SourceSpan,
        expected: String,
        got: String,
    },
    #[error("found recursive type `{t}`.")]
    #[diagnostic(
        code(typeck::types_recursion),
        help("types recursion is not supported.")
    )]
    RecursiveType {
        #[source_code]
        src: Arc<NamedSource<String>>,
        #[label("found during occurs check here...")]
        span: SourceSpan,
        t: String,
    },
    #[error("arity missmatch. expected {expected}, got {got}")]
    #[diagnostic(code(typeck::arity_missmatch))]
    ArityMissmatch {
        #[source_code]
        src: Arc<NamedSource<String>>,
        #[label("here...")]
        span: SourceSpan,
        expected: usize,
        got: usize,
    },
    #[error("name `{name}` is defined multiple times")]
    #[diagnostic(code(typeck::mod_def_redefinition))]
    ModDefRedefinition {
        #[source_code]
        src: Arc<NamedSource<String>>,
        #[label("redeclaration here...")]
        span: SourceSpan,
        name: String,
    },
    #[error("type `{name}` is not defined")]
    #[diagnostic(code(typeck::undefined_type))]
    UndefinedType {
        #[source_code]
        src: Arc<NamedSource<String>>,
        #[label("access here...")]
        span: SourceSpan,
        name: String,
    },
    #[error("name `{name}` is not defined")]
    #[diagnostic(code(typeck::undefined_name))]
    UndefinedName {
        #[source_code]
        src: Arc<NamedSource<String>>,
        #[label("access here...")]
        span: SourceSpan,
        name: String,
    },
    #[error("field `{name}` is not defined")]
    #[diagnostic(code(typeck::undefined_field))]
    UndefinedField {
        #[source_code]
        src: Arc<NamedSource<String>>,
        #[label("access here...")]
        span: SourceSpan,
        name: String,
    },
    #[error("`{typ}` is not callable")]
    #[diagnostic(code(typeck::non_callable))]
    NonCallable {
        #[source_code]
        src: Arc<NamedSource<String>>,
        #[label("access here...")]
        span: SourceSpan,
        typ: String,
    },
    #[error("field `{name}` in module `{module}` is private")]
    #[diagnostic(code(typeck::private_mod_field))]
    PrivateModField {
        #[source_code]
        src: Arc<NamedSource<String>>,
        #[label("access here...")]
        span: SourceSpan,
        name: String,
        module: String,
    },
    #[error("type `{name}` is private")]
    #[diagnostic(code(typeck::private_type))]
    PrivateType {
        #[source_code]
        src: Arc<NamedSource<String>>,
        #[label("access here...")]
        span: SourceSpan,
        name: String,
    },
    #[error("module `{name}` is not defined")]
    #[diagnostic(code(typeck::undefined_mod))]
    UndefinedMod {
        #[source_code]
        src: Arc<NamedSource<String>>,
        #[label("access here...")]
        span: SourceSpan,
        name: String,
    },
    #[error("invalid binary operation `{op:?}` on types `{a}` & `{b}`.")]
    #[diagnostic(code(typeck::invalid_bin_op))]
    InvalidBinOp {
        #[source_code]
        src: Arc<NamedSource<String>>,
        #[label("this binary operation is incorrect.")]
        span: SourceSpan,
        a: String,
        b: String,
        op: BinOp,
    },
    #[error("invalid unary operation `{op:?}` on type `{t}`.")]
    #[diagnostic(code(typeck::invalid_un_op))]
    InvalidUnOp {
        #[source_code]
        src: Arc<NamedSource<String>>,
        #[label("this unary operation is incorrect.")]
        span: SourceSpan,
        t: String,
        op: UnOp,
    },
    #[error("invalid pattern for type `{t}`")]
    #[diagnostic(code(typeck::invalid_pat))]
    InvalidPat {
        #[source_code]
        src: Arc<NamedSource<String>>,
        #[label("this pattern in invalid.")]
        span: SourceSpan,
        t: String,
    },
    #[error("invalid pattern for enum `{en}`")]
    #[diagnostic(code(typeck::invalid_pat_variant))]
    InvalidPatVariant {
        #[source_code]
        src: Arc<NamedSource<String>>,
        #[label("this pattern in invalid.")]
        span: SourceSpan,
        en: String,
    },

    #[error("undefined effect `{name}`")]
    #[diagnostic(code(typeck::undefined_effect))]
    UndefinedEffect {
        #[source_code]
        src: Arc<NamedSource<String>>,
        #[label("access here...")]
        span: SourceSpan,
        name: String,
    },

    #[error("Effect missmatch. expected: {expected} but got: {got}")]
    #[diagnostic(code(typeck::effect_missmatch))]
    EffectsMismatch {
        #[source_code]
        src: Arc<NamedSource<String>>,
        #[label("access here...")]
        span: SourceSpan,
        expected: String,
        got: String,
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
