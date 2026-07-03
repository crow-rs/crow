/// Imports
use crow_common::span::Span;

/// Represents item publicity
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Publicity {
    Pub,
    Priv,
}

/// Represents item mutability
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Mutability {
    Mut,
    Immut,
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
    Concat, // `<>`
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
    Neg,  // -
    Bang, // !
}

/// Represents literal
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Lit {
    Int(String),
    Float(String),
    String(String),
    Bool(String),
    None,
}

/// Represents an effect hint (effect annotation)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EffectHint {
    pub span: Span,
    pub name: String,
}

/// Represents effects of function
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Effects {
    // Known effects list
    pub known: Vec<EffectHint>,

    // Effects tail
    pub tail: Option<char>,
}

/// Represents a type hint (type annotation)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TypeHint {
    /// let a: float = 3.14
    ///        ^^^^^
    ///         this
    Local {
        span: Span,
        name: String,
        args: Vec<TypeHint>,
    },

    /// let a: a.B = a.B()
    ///        ^^^
    ///        this
    Mod {
        span: Span,
        module: String,
        name: String,
        args: Vec<TypeHint>,
    },

    /// let a: fn(int, int) -> int = ...
    ///        ^^^^^^^^^^^^^^^^^^^
    ///               this
    Fun {
        span: Span,
        params: Vec<TypeHint>,
        ret: Box<TypeHint>,
        effects: Effects,
    },

    /// Unit type `()`
    Unit(Span),

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
