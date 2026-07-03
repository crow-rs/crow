use std::collections::HashMap;
mod mir_fmt;

use crow_ast::atom::Mutability;
use crow_resolving::table::DefId;
use crow_tycheck::ty::{FloatTy, IntTy, Ty};

macro_rules! idx {
    ($($name:ident),*) => {$(
        #[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub struct $name(pub u32);
        impl $name {
            #[inline] pub fn index(self) -> usize { self.0 as usize }
        }
        impl From<usize> for $name {
            fn from(i: usize) -> Self { Self(i as u32) }
        }
    )*};
}

idx!(Local, Block, FnId, NativeId, ConstId);

pub type Substs = Vec<Ty>;

/// _0 — return place.
pub const RETURN_PLACE: Local = Local(0);

pub struct MirTyCtxt {
    pub adts: HashMap<DefId, AdtDef>,
}

pub struct AdtDef {
    pub name: String,
    pub variants: Vec<VariantDef>,
}

pub struct VariantDef {
    pub name: String,
    pub fields: Vec<Ty>,
}

impl MirTyCtxt {
    pub fn adt(&self, id: DefId) -> &AdtDef {
        self.adts.get(&id).expect("unknown ADT DefId")
    }

    pub fn is_enum(&self, id: DefId) -> bool {
        self.adt(id).variants.len() > 1
    }
}

pub struct MirModule {
    pub tcx: MirTyCtxt,
    pub functions: Vec<MirBody>,     
    pub natives: Vec<MirNative>,     
    pub constants: Vec<MirConstDef>, 
}

pub struct MirNative {
    pub name: String,
    pub params: Vec<Ty>,
    pub ret: Ty,
    pub symbol: String,
}

pub struct MirConstDef {
    pub name: String,
    pub ty: Ty,
    pub value: Constant,
}

pub struct MirBody {
    pub name: String,
    pub arg_count: usize,
    pub locals: Vec<LocalDecl>,
    pub blocks: Vec<BasicBlock>,
}

pub struct LocalDecl {
    pub ty: Ty,
    pub name: Option<String>,
    pub mutability: Mutability,
}

impl MirBody {
    pub fn local_ty(&self, l: Local) -> &Ty { &self.locals[l.index()].ty }
    pub fn ret_ty(&self) -> &Ty { self.local_ty(RETURN_PLACE) }
}

pub struct BasicBlock {
    pub stmts: Vec<Statement>,
    pub term: Terminator,
}

pub enum Statement {
    Assign(Place, Rvalue),
    StorageLive(Local),
    StorageDead(Local),
    Nop,
}

#[derive(Clone, PartialEq, Eq)]
pub struct Place {
    pub local: Local,
    pub proj: Vec<Projection>,
}

impl Place {
    pub fn local(l: Local) -> Self { Place { local: l, proj: vec![] } }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Projection {
    Field(u32),
    
    Downcast(u32),
}

#[derive(Clone)]
pub struct PlaceTy {
    pub ty: Ty,
    pub variant: Option<u32>,
}

impl Place {
    pub fn ty(&self, body: &MirBody, tcx: &MirTyCtxt) -> PlaceTy {
        let mut pt = PlaceTy { ty: body.local_ty(self.local).clone(), variant: None };
        for p in &self.proj {
            pt = match (*p, &pt.ty) {
                (Projection::Field(i), Ty::Adt(def_id, _)) => {
                    let v = pt.variant.unwrap_or(0) as usize;
                    PlaceTy {
                        ty: tcx.adt(*def_id).variants[v].fields[i as usize].clone(),
                        variant: None,
                    }
                }
                (Projection::Downcast(v), Ty::Adt(..)) => {
                    PlaceTy { ty: pt.ty, variant: Some(v) }
                }
                _ => panic!("ill-typed projection"),
            };
        }
        pt
    }
}

#[derive(Clone)]
pub enum Operand {
    Copy(Place),
    Const(Constant),
}

impl Operand {
    pub fn ty(&self, body: &MirBody, tcx: &MirTyCtxt) -> Ty {
        match self {
            Operand::Copy(p) => p.ty(body, tcx).ty,
            Operand::Const(c) => c.ty(),
        }
    }
}

pub enum Rvalue {
    Use(Operand),
    BinaryOp(BinOp, Operand, Operand),
    UnaryOp(UnOp, Operand),
    Aggregate(AggregateKind, Vec<Operand>),
    Discriminant(Place),
    Cast(CastKind, Operand, Ty),
}

pub enum AggregateKind {
    Adt { def_id: DefId, variant: u32 },
    Closure { fn_id: FnId, env_def: DefId },
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum CastKind {
    IntToInt,
    IntToFloat,
    FloatToInt,
    FloatToFloat,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Add, Sub, Mul, Div, Rem,
    BitAnd, BitOr, BitXor, Shl, Shr,
    Eq, Ne, Lt, Le, Gt, Ge,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum UnOp { Not, Neg }

#[derive(Clone, PartialEq)]
pub enum Constant {
    Unit,
    Bool(bool),
    Int(i128, IntTy),
    Float(f64, FloatTy),
    Str(String),
    Fn(FnId, Substs),
    Native(NativeId),
    Global(ConstId),
}

impl Constant {
    pub fn ty(&self) -> Ty {
        match self {
            Constant::Unit => Ty::Unit,
            Constant::Bool(_) => Ty::Bool,
            Constant::Int(_, t) => Ty::Int(*t),
            Constant::Float(_, t) => Ty::Float(*t),
            Constant::Str(_) => Ty::String,
            _ => unimplemented!("needs module context"),
        }
    }

    pub fn ty_in(&self, module: &MirModule) -> Ty {
        match self {
            Constant::Fn(fn_id, substs) => {
                let body = &module.functions[fn_id.index()];
                let params: Vec<Ty> = body.locals[1..=body.arg_count]
                    .iter()
                    .map(|l| subst_ty(&l.ty, substs))
                    .collect();
                let ret = subst_ty(body.ret_ty(), substs);
                Ty::Fn(params, Box::new(ret))
            }
            Constant::Native(nid) => {
                let n = &module.natives[nid.index()];
                Ty::Fn(n.params.clone(), Box::new(n.ret.clone()))
            }
            Constant::Global(cid) => {
                module.constants[cid.index()].ty.clone()
            }
            other => other.ty(),
        }
    }
}

pub fn subst_ty(ty: &Ty, substs: &Substs) -> Ty {
    if substs.is_empty() { return ty.clone(); }
    match ty {
        Ty::Param(_, idx) => substs
            .get(*idx as usize)
            .cloned()
            .unwrap_or(ty.clone()),
        Ty::Adt(def, args) => Ty::Adt(
            *def,
            args.iter().map(|t| subst_ty(t, substs)).collect(),
        ),
        Ty::Fn(params, ret) => Ty::Fn(
            params.iter().map(|t| subst_ty(t, substs)).collect(),
            Box::new(subst_ty(ret, substs)),
        ),
        other => other.clone(),
    }
}

pub enum Terminator {
    Goto(Block),

    SwitchInt {
        discr: Operand,
        targets: Vec<(u128, Block)>,
        otherwise: Block,
    },

    Call {
        func: Operand,
        args: Vec<Operand>,
        dest: Place,
        target: Block,
    },

    Assert {
        cond: Operand,
        expected: bool,
        msg: AssertMsg,
        target: Block,
    },

    Return,
    Unreachable,
}

impl Terminator {
    pub fn successors(&self) -> Vec<Block> {
        match self {
            Terminator::Goto(b) => vec![*b],
            Terminator::SwitchInt { targets, otherwise, .. } => {
                let mut s: Vec<Block> = targets.iter().map(|(_, b)| *b).collect();
                s.push(*otherwise);
                s
            }
            Terminator::Call { target, .. } => vec![*target],
            Terminator::Assert { target, .. } => vec![*target],
            Terminator::Return | Terminator::Unreachable => vec![],
        }
    }
}

pub enum AssertMsg {
    DivisionByZero,
    RemainderByZero,
    Overflow(BinOp),
    MatchFailed,
}

pub fn verify(module: &MirModule) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();

    for (_, body) in module.functions.iter().enumerate() {
        for local in &body.locals {
            match &local.ty {
                Ty::Infer(_) => errors.push(format!(
                    "fn `{}`: unresolved Ty::Infer in local", body.name
                )),
                Ty::IntVar(_) => errors.push(format!(
                    "fn `{}`: unresolved Ty::IntVar in local", body.name
                )),
                Ty::FloatVar(_) => errors.push(format!(
                    "fn `{}`: unresolved Ty::FloatVar in local", body.name
                )),
                Ty::Error => errors.push(format!(
                    "fn `{}`: Ty::Error leaked into MIR", body.name
                )),
                _ => {}
            }
        }

        for (bi, bb) in body.blocks.iter().enumerate() {
            let check_block = |b: Block| {
                if b.index() >= body.blocks.len() {
                    Some(format!(
                        "fn `{}` bb{}: jump to invalid {}",
                        body.name, bi, b
                    ))
                } else { None }
            };

            match &bb.term {
                Terminator::Goto(b) => {
                    if let Some(e) = check_block(*b) { errors.push(e); }
                }
                Terminator::SwitchInt { targets, otherwise, .. } => {
                    for (_, b) in targets {
                        if let Some(e) = check_block(*b) { errors.push(e); }
                    }
                    if let Some(e) = check_block(*otherwise) { errors.push(e); }
                }
                Terminator::Call { target, .. } => {
                    if let Some(e) = check_block(*target) { errors.push(e); }
                }
                Terminator::Assert { target, .. } => {
                    if let Some(e) = check_block(*target) { errors.push(e); }
                }
                Terminator::Return | Terminator::Unreachable => {}
            }
        }
    }

    if errors.is_empty() { Ok(()) } else { Err(errors) }
}