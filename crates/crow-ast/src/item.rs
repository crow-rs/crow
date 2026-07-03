/// Imports
use crate::{
    atom::{Effects, Param, Publicity, TypeHint},
    expr::Expr,
};
use crow_common::span::Span;
use miette::NamedSource;
use std::sync::Arc;

/// Use path (e.g `this/is/some/module`)
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct UsePath {
    pub span: Span,
    pub module: String,
}

/// Defines use kind
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum UseKind {
    /// Represents import of module as given name
    As(String),
    /// Represents import of module contents separated by comma
    For(Vec<String>),
    /// Just import of module
    Just,
}

/// Represents using
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Use {
    pub span: Span,
    pub path: UsePath,
    pub kind: UseKind,
}

/// Represents struct field
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Field {
    pub span: Span,
    pub name: String,
    pub hint: TypeHint,
}

/// Represents struct item
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Struct {
    pub name: String,
    pub fields: Vec<Field>,
}

/// Represents enum varisnt
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Variant {
    pub span: Span,
    pub name: String,
    pub fields: Vec<TypeHint>,
}

/// Represents enum item
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Enum {
    pub name: String,
    pub variants: Vec<Variant>,
}

/// Represents function item
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Fun {
    pub span: Span,
    pub name: String,
    pub generics: Vec<String>,
    pub params: Vec<Param>,
    pub effects: Effects,
    pub ret: TypeHint,
    pub block: Expr,
}

/// Native function item
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NativeFun {
    pub span: Span,
    pub name: String,
    pub params: Vec<Param>,
    pub ret: TypeHint,
    pub body: String,
}

/// Constant item
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Const {
    pub span: Span,
    pub name: String,
    pub value: Expr,
    pub hint: TypeHint,
}

/// Defines item kind
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ItemKind {
    Struct(Struct),
    Enum(Enum),
    Fun(Fun),
    Native(NativeFun),
    Const(Const),
}

/// Defines item
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Item {
    pub publicity: Publicity,
    pub span: Span,
    pub kind: ItemKind,
}

/// Module item, the root of a file
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Module {
    /// Source of the module
    pub source: Arc<NamedSource<String>>,

    /// Module usings
    pub uses: Vec<Use>,

    /// Module items
    pub items: Vec<Item>,
}
