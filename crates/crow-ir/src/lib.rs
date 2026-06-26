pub type MirLocalId = u32;

pub type MirBlockId = u32;

#[derive(Clone, Debug, PartialEq)]
pub enum MirType {
    Int,
    Float,
    Bool,
    Str,
    Unit,
    /// GC-managed pointer to a struct
    Struct(MirStructId),
    /// GC-managed pointer to an enum
    Enum(MirEnumId),
    /// Array of homogeneous elements
    Array(Box<MirType>),
    /// Function pointer: params -> ret
    FunPtr {
        params: Vec<MirType>,
        ret: Box<MirType>,
    },
    /// Closure (function + captured env)
    Closure {
        params: Vec<MirType>,
        ret: Box<MirType>,
    },
    /// Never type (diverges)
    Never,
}

pub type MirStructId = u32;
pub type MirEnumId = u32;
pub type MirFunctionId = u32;

pub struct MirBody {
    pub name: String,
    pub locals: Vec<MirLocal>,
    pub arg_count: usize,
    pub blocks: Vec<MirBasicBlock>,
}

pub struct MirLocal {
    pub ty: MirType,
    pub kind: MirLocalKind,
    pub name: Option<String>,
}

#[derive(Clone, Copy, PartialEq)]
pub enum MirLocalKind {
    Return,
    Arg,
    Temp,
    User,
}

pub struct MirBasicBlock {
    pub stmts: Vec<MirStatement>,
    pub terminator: MirTerminator,
}

pub enum MirStatement {
    /// place = rvalue
    Assign(MirPlace, MirRvalue),
    /// No-op (can be used as placeholder)
    Nop,
}

#[derive(Clone, Debug)]
pub struct MirPlace {
    pub local: MirLocalId,
    pub projection: Vec<MirProjection>,
}

impl MirPlace {
    /// Simple local with no projection: _N
    pub fn local(id: MirLocalId) -> Self {
        Self {
            local: id,
            projection: vec![],
        }
    }
}

#[derive(Clone, Debug)]
pub enum MirProjection {
    /// Access struct field by index
    Field(usize),
    /// Array/slice index (runtime value in a local)
    Index(MirLocalId),
    /// Downcast enum to variant before accessing fields
    Downcast(usize),
}

pub enum MirRvalue {
    /// Simply copy a value
    Use(MirOperand),

    BinOp(MirBinOp, MirOperand, MirOperand),

    UnOp(MirUnOp, MirOperand),
    /// Construct aggregate: struct, enum variant, array, closure
    Aggregate(MirAggregateKind, Vec<MirOperand>),
    /// Take a GC reference to a place
    Ref(MirPlace),
    /// Type cast
    Cast(MirCastKind, MirOperand, MirType),
    /// Get discriminant of an enum (for switch)
    Discriminant(MirPlace),
    /// String/array length
    Len(MirPlace),
}

#[derive(Clone, Debug)]
pub enum MirOperand {
    /// Read from a place
    Copy(MirPlace),
    /// Compile-time constant
    Constant(MirConstant),
}

#[derive(Clone, Debug)]
pub enum MirConstant {
    Int(i64),
    Float(f64),
    Bool(bool),
    Str(String),
    Unit,
    /// Reference to a function (for calls / function pointers)
    FunRef(MirFunctionId),
}

pub enum MirAggregateKind {
    /// Construct a struct instance
    Struct(MirStructId),
    /// Construct an enum variant
    Variant(MirEnumId, usize),
    /// Construct an array literal
    Array,
    /// Construct a closure (function id + captured values)
    Closure(MirFunctionId),
}

pub enum MirTerminator {
    Goto(MirBlockId),

    SwitchInt {
        discr: MirOperand,
        targets: Vec<(u128, MirBlockId)>,
        otherwise: MirBlockId,
    },

    Call {
        dest: MirPlace,
        func: MirOperand,
        args: Vec<MirOperand>,
        target: MirBlockId,
    },

    Return,

    Abort,

    Unreachable,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MirBinOp {
    /// Arithmetic
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    /// Bitwise
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
    /// Comparison
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    /// Logical (short-circuit already lowered to CFG)
    And,
    Or,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MirUnOp {
    Neg,
    Not,
}

#[derive(Clone, Debug)]
pub enum MirCastKind {
    Primitive,
    Upcast,
}

pub struct MirStructDef {
    pub name: String,
    pub fields: Vec<MirFieldDef>,
}

pub struct MirFieldDef {
    pub name: String,
    pub ty: MirType,
}

pub struct MirEnumDef {
    pub name: String,
    pub variants: Vec<MirVariantDef>,
}

pub struct MirVariantDef {
    pub name: String,
    pub fields: Vec<MirType>,
}

pub struct MirFunction {
    pub name: String,
    pub params: Vec<MirType>,
    pub ret: MirType,
    pub body: Option<MirBody>,
}

pub struct MirModule {
    pub structs: Vec<MirStructDef>,
    pub enums: Vec<MirEnumDef>,
    pub functions: Vec<MirFunction>,

    pub entry: MirFunctionId,
}