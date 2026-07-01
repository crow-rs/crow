/// Imports
use miette::{Diagnostic, NamedSource, SourceSpan};
use std::sync::Arc;
use thiserror::Error;

/// Parser error
#[derive(Error, Diagnostic, Debug)]
pub enum ResolverErrors {
    #[error("undefied name `{name:?}`")]
    #[diagnostic(code(resolve::undefined_name))]
    UndefinedName {
        name: String,
        #[source_code]
        src: Arc<NamedSource<String>>,
        #[label("got undefined name here...")]
        span: SourceSpan,
    },
    #[error("name `{name:?}` not bound in all alternatives`")]
    #[diagnostic(code(resolve::undefined_name))]
    NotBound {
        name: String,
        #[source_code]
        src: Arc<NamedSource<String>>,
        #[label("got unbounded name here...")]
        span: SourceSpan,
    },
}
