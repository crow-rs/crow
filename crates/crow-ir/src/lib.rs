pub type MirLocalId = u32;

pub type MirBlockId = u32;

#[derive(Clone, Debug, PartialEq)]
pub enum MirIntBitness {
    Bit8,
    Bit16,
    Bit32,
    Bit64
}

#[derive(Clone, Debug, PartialEq)]
pub enum MirType {
    Int(MirIntBitness),
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
    Int(i64, MirIntBitness),
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

pub struct MirTyCtx<'a> {
    pub structs: &'a [MirStructDef],
    pub enums: &'a [MirEnumDef],
    pub functions: &'a [MirFunction],
}

impl MirModule {
    pub fn ty_ctx(&self) -> MirTyCtx<'_> {
        MirTyCtx {
            structs: &self.structs,
            enums: &self.enums,
            functions: &self.functions,
        }
    }
}

impl MirConstant {
    pub fn ty(&self, tcx: &MirTyCtx<'_>) -> MirType {
        match self {
            MirConstant::Int(_, bitness) => MirType::Int(bitness.clone()),
            MirConstant::Float(_) => MirType::Float,
            MirConstant::Bool(_) => MirType::Bool,
            MirConstant::Str(_) => MirType::Str,
            MirConstant::Unit => MirType::Unit,
            MirConstant::FunRef(id) => {
                let func = &tcx.functions[*id as usize];
                MirType::FunPtr {
                    params: func.params.clone(),
                    ret: Box::new(func.ret.clone()),
                }
            }
        }
    }
}

impl MirOperand {
    pub fn ty(&self, locals: &[MirLocal], tcx: &MirTyCtx<'_>) -> MirType {
        match self {
            MirOperand::Copy(place) => place.ty(locals, tcx),
            MirOperand::Constant(c) => c.ty(tcx),
        }
    }
}

impl MirPlace {
    pub fn base_ty(&self, locals: &[MirLocal]) -> MirType {
        locals[self.local as usize].ty.clone()
    }

    pub fn ty(&self, locals: &[MirLocal], tcx: &MirTyCtx<'_>) -> MirType {
        let mut ty = self.base_ty(locals);
        let mut active_variant: Option<usize> = None;

        for proj in &self.projection {
            match proj {
                MirProjection::Field(idx) => {
                    ty = match &ty {
                        MirType::Struct(sid) => {
                            let def = &tcx.structs[*sid as usize];
                            def.fields[*idx].ty.clone()
                        }
                        MirType::Enum(eid) => {
                            let vi = active_variant
                                .expect("Field on Enum without preceding Downcast");
                            let def = &tcx.enums[*eid as usize];
                            def.variants[vi].fields[*idx].clone()
                        }
                        _ => panic!("Field projection on {:?}", ty),
                    };
                    active_variant = None; 
                }

                MirProjection::Downcast(variant_idx) => {
                    assert!(
                        matches!(ty, MirType::Enum(_)),
                        "Downcast on non-enum {:?}",
                        ty
                    );
                    active_variant = Some(*variant_idx);
                }

                MirProjection::Index(_) => {
                    ty = match ty {
                        MirType::Array(elem) => *elem,
                        _ => panic!("Index projection on {:?}", ty),
                    };
                }
            }
        }

        ty
    }
}

impl MirProjection {
    pub fn project_ty(&self, base: MirType, tcx: &MirTyCtx<'_>) -> MirType {
        match self {
            MirProjection::Field(idx) => match &base {
                MirType::Struct(sid) => {
                    tcx.structs[*sid as usize].fields[*idx].ty.clone()
                }
                _ => panic!("Field on {:?}", base),
            },
            MirProjection::Index(_) => match base {
                MirType::Array(elem) => *elem,
                _ => panic!("Index on {:?}", base),
            },
            MirProjection::Downcast(_) => base, 
        }
    }
}