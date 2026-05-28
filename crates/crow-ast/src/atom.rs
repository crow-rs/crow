/// Imports
use crow_lex::token::Span;

use crate::expr::Expr;

/// Represents item publicity
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Publicity {
    Pub,
    Priv,
}

/// Represents variable mutability
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Mutability {
    Mut,
    Not,
}

/// Binary operator
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BinOp {
    Add,    // `+`
    Sub,    // `-`
    Mul,    // `*`
    Div,    // `/`
    Rem,    // `%`
    Eq,     // `==`
    Ne,     // `!=`
    Gt,     // `>`
    Ge,     // `>=`
    Lt,     // `<`
    Le,     // `<=`
    And,    // `&&`
    Or,     // `||`
    Xor,    // `^`
    BitAnd, // `&`
    BitOr,  // `|`
}

/// Assignment operation used in assignment expressions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AssignOp {
    AddEq, // +=
    SubEq, // -=
    MulEq, // *=
    DivEq, // /=
    ModEq, // %=
    AndEq, // &=
    OrEq,  // |=
    XorEq, // ^=
    Eq,    // =
}

/// Unary operator
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UnOp {
    Neg,   // -
    Bang,  // !
    Ref,   // &
    Deref, // *
}

/// Represents literal
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Lit {
    Int(String),
    Float(String),
    String(String),
    Bool(String),
    Array(Vec<Expr>),
    None,
}

/// Represents a type hint (type annotation)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TypeHint {
    /// let a: float = 3.14
    ///        ^^^^^
    ///         this
    Local { span: Span, name: String },

    /// let a: a.B = a.B()
    ///        ^^^
    ///        this
    Mod {
        span: Span,
        module: String,
        name: String,
    },

    /// let a: fn(int, int) -> int = ...
    ///        ^^^^^^^^^^^^^^^^^^^
    ///               this
    Fun {
        span: Span,
        params: Vec<TypeHint>,
        ret: Box<TypeHint>,
    },

    /// Unit type `()`
    Unit(Span),

    /// Reference type `&hint`
    Ref { span: Span, hint: Box<TypeHint> },

    /// Union type
    Union { span: Span, types: Vec<TypeHint> },

    /// Type is not specified
    /// and should be inferred
    Infer,
}

/// Represents a function parameter
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Param {
    pub span: Span,
    pub name: String,
    pub hint: TypeHint,
}
