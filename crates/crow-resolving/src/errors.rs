use std::sync::Arc;

use miette::{Diagnostic, NamedSource, SourceSpan};
use thiserror::Error;

/// Parser error
#[derive(Error, Diagnostic, Debug)]
pub enum ResolverErrors {
    #[error("undefied name `{undef_name:?}`")]
    #[diagnostic(code(resolve::undefined_name))]
    UndefinedName {
        undef_name: String,
        #[source_code]
        src: Arc<NamedSource<String>>,
        #[label("got undefined name here...")]
        span: SourceSpan,
    },

    #[error("name `{name:?} not bound in all alternatives`")]
    #[diagnostic(code(resolve::undefined_name))]
    NotBound {
        name: String,
        #[source_code]
        src: Arc<NamedSource<String>>,
        #[label("got unbounded name here...")]
        span: SourceSpan,
    },

}