use std::sync::Arc;

use miette::{Diagnostic, NamedSource, SourceSpan};
use thiserror::Error;

#[derive(Error, Diagnostic, Debug)]
pub enum SpecialItemError {
    #[error("function `{name}` has no body")]
    #[diagnostic(
        code(spec_checker::fn_no_body),
        help("add a body or mark as @intrinsic")
    )]
    FunctionWithoutBody {
        name: String,
        #[source_code]
        src: Arc<NamedSource<String>>,
        #[label("no body")]
        span: SourceSpan,
    },

    #[error("intrinsic `{name}` should not have a body")]
    #[diagnostic(code(spec_checker::intrin_with_body))]
    IntrinsicWithBody {
        name: String,
        #[source_code]
        src: Arc<NamedSource<String>>,
        #[label("remove body")]
        span: SourceSpan,
    },

    #[error("@intrinsic on `{name}` missing argument")]
    #[diagnostic(
        code(spec_checker::intrin_miss_arg),
        help("use @intrinsic(\"name\")")
    )]
    IntrinsicMissingArg {
        name: String,
        #[source_code]
        src: Arc<NamedSource<String>>,
        #[label("expected @intrinsic(\"name\")")]
        span: SourceSpan,
    },

    #[error("@lang_def on `{name}` missing argument")]
    #[diagnostic(
        code(spec_checker::lang_def_miss_arg),
        help("use @lang_def(\"name\")")
    )]
    LangDefMissingArg {
        name: String,
        #[source_code]
        src: Arc<NamedSource<String>>,
        #[label("expected @lang_def(\"name\")")]
        span: SourceSpan,
    },

    #[error("unknown lang item `{item}` on `{name}`")]
    #[diagnostic(
        code(spec_checker::unknown_lang_item),
        help("known items: mem_alloc, mem_dealloc, rc_alloc, rc_retain, rc_release, panic")
    )]
    UnknownLangItem {
        name: String,
        item: String,
        #[source_code]
        src: Arc<NamedSource<String>>,
        #[label("unknown lang item")]
        span: SourceSpan,
    },

    #[error("duplicate lang item `{name}`")]
    #[diagnostic(code(spec_checker::duplicate_lang_item))]
    DuplicateLangItem {
        name: String,
        #[source_code]
        src: Arc<NamedSource<String>>,
        #[label("already defined")]
        span: SourceSpan,
    },

    #[error("duplicate intrinsic `{name}`")]
    #[diagnostic(code(spec_checker::duplicate_intrin))]
    DuplicateIntrinsic {
        name: String,
        #[source_code]
        src: Arc<NamedSource<String>>,
        #[label("already defined")]
        span: SourceSpan,
    },

    #[error("missing required @lang_def(\"{name}\")")]
    #[diagnostic(
        code(spec_checker::missing_required),
        help("{hint}")
    )]
    MissingRequired {
        name: String,
        hint: String,
    },
}