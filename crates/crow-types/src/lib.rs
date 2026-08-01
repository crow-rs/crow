use core::fmt;
use crow_common::DefId;
use ena::unify::{EqUnifyValue, UnifyKey, UnifyValue};

/// Type variable id
#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub struct TyVid(pub u32);

/// Int type variable id
#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub struct IntVid(pub u32);

/// Float type variable id
#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub struct FloatVid(pub u32);

/// Defines type in the type-system
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Ty {
    Infer(TyVid),
    IntVar(IntVid),
    FloatVar(FloatVid),
    Int(IntTy),
    Float(FloatTy),
    RawPtr, // todo - generic type
    Bool,
    String,
    Unit,
    Never,
    Adt(DefId, Vec<Ty>),
    Fn(Vec<Ty>, Box<Ty>),
    Param(DefId, u32),
    Error,
}

impl Ty {
    pub fn from_name(name: &str) -> Ty {
        match name {
            "i8" => Ty::Int(IntTy::I8),
            "i16" => Ty::Int(IntTy::I16),
            "i32" => Ty::Int(IntTy::I32),
            "i64" => Ty::Int(IntTy::I64),
            "u8" => Ty::Int(IntTy::U8),
            "u16" => Ty::Int(IntTy::U16),
            "u32" => Ty::Int(IntTy::U32),
            "u64" => Ty::Int(IntTy::U64),
            "f32" => Ty::Float(FloatTy::F32),
            "f64" => Ty::Float(FloatTy::F64),
            "raw_ptr" => Ty::RawPtr,
            "bool" => Ty::Bool,
            "str" => Ty::String,
            "unit" => Ty::Unit,
            "!" => Ty::Never,
            _ => Ty::Error,
        }
    }

    pub fn builtin_names() -> &'static [&'static str] {
        &[
            "i8", "i16", "i32", "i64",
            "u8", "u16", "u32", "u64",
            "f32", "f64",
            "bool", "str", "unit", "raw_ptr", "!",
        ]
    }
}

/// Int type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum IntTy {
    I8,
    I16,
    I32,
    I64,
    U8,
    U16,
    U32,
    U64,
}

/// Eq-unify-value implementation for int type
impl EqUnifyValue for IntTy {}

/// Implementation of int type
impl IntTy {
    /// Returns available int type values range
    pub fn range(self) -> (i128, i128) {
        match self {
            IntTy::I8 => (i8::MIN as i128, i8::MAX as i128),
            IntTy::I16 => (i16::MIN as i128, i16::MAX as i128),
            IntTy::I32 => (i32::MIN as i128, i32::MAX as i128),
            IntTy::I64 => (i64::MIN as i128, i64::MAX as i128),
            IntTy::U8 => (0, u8::MAX as i128),
            IntTy::U16 => (0, u16::MAX as i128),
            IntTy::U32 => (0, u32::MAX as i128),
            IntTy::U64 => (0, u64::MAX as i128),
        }
    }

    /// Returns `true` if value fits in possible values range
    pub fn fits(self, value: i128) -> bool {
        let (min, max) = self.range();
        value >= min && value <= max
    }

    pub fn is_signed(&self) -> bool {
        matches!(self, IntTy::I8 | IntTy::I16 | IntTy::I32 | IntTy::I64)
    }
}

/// Float type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FloatTy {
    F32,
    F64,
}

/// Eq-unify-value implementation for float type
impl EqUnifyValue for FloatTy {}

/// Defines type value
#[derive(Clone, Debug, PartialEq)]
pub struct TyValue(pub Option<Ty>);

/// Unify-key implementation for type variable id
impl UnifyKey for TyVid {
    type Value = TyValue;
    fn index(&self) -> u32 {
        self.0
    }
    fn from_index(u: u32) -> Self {
        TyVid(u)
    }
    fn tag() -> &'static str {
        "TyVid"
    }
}

pub enum UnificationError {
    InvalidTypeConversion
}

/// Unify-value implementation for type value
impl UnifyValue for TyValue {
    type Error = UnificationError;

    fn unify_values(a: &Self, b: &Self) -> Result<Self, UnificationError> {
        match (&a.0, &b.0) {
            (None, None) => Ok(TyValue(None)),
            (Some(v), None) | (None, Some(v)) => {
                Ok(TyValue(Some(v.clone())))
            }
            (Some(_), Some(_)) => Err(UnificationError::InvalidTypeConversion),
        }
    }
}

/// Unify-key implementation for int type variable id
impl UnifyKey for IntVid {
    type Value = Option<IntTy>;
    fn index(&self) -> u32 {
        self.0
    }
    fn from_index(u: u32) -> Self {
        IntVid(u)
    }
    fn tag() -> &'static str {
        "IntVid"
    }
}

/// Unify-key implementation for float type variable id
impl UnifyKey for FloatVid {
    type Value = Option<FloatTy>;
    fn index(&self) -> u32 {
        self.0
    }
    fn from_index(u: u32) -> Self {
        FloatVid(u)
    }
    fn tag() -> &'static str {
        "FloatVid"
    }
}

/// Display implementation for int type
impl fmt::Display for IntTy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IntTy::I8 => write!(f, "i8"),
            IntTy::I16 => write!(f, "i16"),
            IntTy::I32 => write!(f, "i32"),
            IntTy::I64 => write!(f, "i64"),
            IntTy::U8 => write!(f, "u8"),
            IntTy::U16 => write!(f, "u16"),
            IntTy::U32 => write!(f, "u32"),
            IntTy::U64 => write!(f, "u64"),
        }
    }
}

/// Display implementation for float type
impl fmt::Display for FloatTy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FloatTy::F32 => write!(f, "f32"),
            FloatTy::F64 => write!(f, "f64"),
        }
    }
}

/// Display implementation for type
impl fmt::Display for Ty {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Ty::Bool => write!(f, "bool"),
            Ty::String => write!(f, "string"),
            Ty::Unit => write!(f, "()"),
            Ty::Never => write!(f, "!"),
            Ty::Error => write!(f, "<error>"),
            Ty::Int(i) => write!(f, "{}", i),
            Ty::Float(fl) => write!(f, "{}", fl),
            Ty::Infer(vid) => write!(f, "?{}", vid.0),
            Ty::IntVar(vid) => write!(f, "?int{}", vid.0),
            Ty::FloatVar(vid) => write!(f, "?float{}", vid.0),
            Ty::Param(_, idx) => write!(f, "T{}", idx),
            Ty::RawPtr => write!(f, "raw"),
            Ty::Adt(def_id, args) => {
                write!(f, "adt#[Module id: {:?} | local_id: {:?}]", def_id.module, def_id.local)?;
                if !args.is_empty() {
                    write!(f, "<")?;
                    for (i, arg) in args.iter().enumerate() {
                        if i > 0 {
                            write!(f, ", ")?;
                        }
                        write!(f, "{}", arg)?;
                    }
                    write!(f, ">")?;
                }
                Ok(())
            }
            Ty::Fn(params, ret) => {
                write!(f, "fn(")?;
                for (i, p) in params.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", p)?;
                }
                write!(f, ") -> {}", ret)
            }
        }
    }
}
