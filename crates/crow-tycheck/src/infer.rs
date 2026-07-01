/// Imports
use crate::{errors::TyCheckError, ty::*};
use crow_lex::token::Span;
use ena::unify::{InPlace, InPlaceUnificationTable};
use std::collections::HashMap;

/// Inference context that used across
/// the type-checking process
pub struct InferCtxt {
    /// Inference variables table
    table: InPlaceUnificationTable<TyVid>,

    /// Int inference variables table
    int_table: InPlaceUnificationTable<IntVid>,

    /// Float inference variables table
    float_table: InPlaceUnificationTable<FloatVid>,

    /// Int literal values table
    int_lit_values: HashMap<IntVid, i128>,
}

/// Inference context implementation
impl InferCtxt {
    /// Creates new inference context
    pub fn new() -> Self {
        Self {
            table: InPlaceUnificationTable::new(),
            int_table: InPlaceUnificationTable::new(),
            float_table: InPlaceUnificationTable::new(),
            int_lit_values: HashMap::new(),
        }
    }

    /// Returns fresh inference variable type
    pub fn fresh_var(&mut self) -> Ty {
        let vid = self.table.new_key(TyValue(None));
        Ty::Infer(vid)
    }

    /// Returns fresh int inference variable type
    /// and registers literal value
    pub fn fresh_int_var_with_value(&mut self, value: i128) -> Ty {
        let vid = self.int_table.new_key(None);
        self.int_lit_values.insert(vid, value);
        Ty::IntVar(vid)
    }

    /// Returns fresh int inference variable type
    pub fn fresh_int_var(&mut self) -> Ty {
        let vid = self.int_table.new_key(None);
        Ty::IntVar(vid)
    }

    /// Returns fresh float inference variable type
    pub fn fresh_float_var(&mut self) -> Ty {
        let vid = self.float_table.new_key(None);
        Ty::FloatVar(vid)
    }

    /// Shallowly-applies type
    pub fn shallow_apply(&mut self, ty: Ty) -> Ty {
        match ty {
            Ty::Infer(vid) => {
                let root = self.table.find(vid);
                match self.table.probe_value(root).0 {
                    Some(resolved) => self.shallow_apply(resolved),
                    None => Ty::Infer(root),
                }
            }
            Ty::IntVar(vid) => {
                let root = self.int_table.find(vid);
                match self.int_table.probe_value(root) {
                    Some(int_ty) => Ty::Int(int_ty),
                    None => Ty::IntVar(root),
                }
            }
            Ty::FloatVar(vid) => {
                let root = self.float_table.find(vid);
                match self.float_table.probe_value(root) {
                    Some(float_ty) => Ty::Float(float_ty),
                    None => Ty::FloatVar(root),
                }
            }
            other => other,
        }
    }

    /// Deeply-applies type
    pub fn deep_apply(&mut self, ty: Ty) -> Ty {
        let ty = self.shallow_apply(ty);
        match ty {
            Ty::Fn(params, ret) => {
                let params = params
                    .into_iter()
                    .map(|t| self.deep_apply(t))
                    .collect();
                let ret = self.deep_apply(*ret);
                Ty::Fn(params, Box::new(ret))
            }
            Ty::Adt(def_id, args) => {
                let args =
                    args.into_iter().map(|t| self.deep_apply(t)).collect();
                Ty::Adt(def_id, args)
            }
            other => other,
        }
    }

    /// Fallbacks type to default one
    pub fn fallback(&mut self, ty: Ty) -> Ty {
        let ty = self.shallow_apply(ty);
        match ty {
            Ty::IntVar(_) => Ty::Int(IntTy::I32),
            Ty::FloatVar(_) => Ty::Float(FloatTy::F64),
            Ty::Infer(_) => Ty::Error,
            Ty::Fn(params, ret) => {
                let params =
                    params.into_iter().map(|t| self.fallback(t)).collect();
                let ret = self.fallback(*ret);
                Ty::Fn(params, Box::new(ret))
            }
            Ty::Adt(def_id, args) => {
                let args =
                    args.into_iter().map(|t| self.fallback(t)).collect();
                Ty::Adt(def_id, args)
            }
            other => other,
        }
    }

    /// Unifies two types
    pub fn unify(
        &mut self,
        a: Ty,
        b: Ty,
        span: Span,
    ) -> Result<(), TyCheckError> {
        // Applying substitutions
        let a = self.shallow_apply(a);
        let b = self.shallow_apply(b);

        // Unifying types
        match (a, b) {
            (Ty::Error, _) | (_, Ty::Error) => Ok(()),
            (Ty::Never, _) | (_, Ty::Never) => Ok(()),
            (Ty::Infer(a), Ty::Infer(b)) if a == b => Ok(()),
            (Ty::Infer(a), Ty::Infer(b)) => self
                .table
                .unify_var_var(a, b)
                .map_err(|_| TyCheckError::InvalidTypeConversion),
            (Ty::Infer(vid), ty) | (ty, Ty::Infer(vid)) => {
                self.occurs_check(vid, &ty, span.clone())?;
                self.table
                    .unify_var_value(vid, TyValue(Some(ty)))
                    .map_err(|_| TyCheckError::InvalidTypeConversion)
            }
            (Ty::IntVar(a), Ty::IntVar(b)) => self
                .int_table
                .unify_var_var(a, b)
                .map_err(|_| TyCheckError::InvalidTypeConversion),
            (Ty::IntVar(vid), Ty::Int(int_ty))
            | (Ty::Int(int_ty), Ty::IntVar(vid)) => {
                let root = self.int_table.find(vid);

                let violations: Vec<i128> = self
                    .int_lit_values
                    .iter()
                    .map(|(&v, &val)| (v, val))
                    .collect::<Vec<_>>()
                    .into_iter()
                    .filter(|(lit_vid, value)| {
                        let lit_root = self.int_table.find(*lit_vid);
                        lit_root == root && !int_ty.fits(*value)
                    })
                    .map(|(_, value)| value)
                    .collect();

                if let Some(&value) = violations.first() {
                    if !int_ty.fits(value) {
                        return Err(TyCheckError::LiteralOutOfRange {
                            lit: value.to_string(),
                            range: format!("{:?}", int_ty),
                            src: span.0.clone(),
                            span: span.1.clone().into(),
                        });
                    }
                }

                self.int_table.unify_var_value(vid, Some(int_ty)).map_err(
                    |_| TyCheckError::TypeMissmatch {
                        expected: format!("{}", Ty::Int(int_ty)),
                        got: format!("{}", Ty::IntVar(vid)),
                        src: span.0.clone(),
                        span: span.1.clone().into(),
                    },
                )
            }
            (Ty::FloatVar(a), Ty::FloatVar(b)) => self
                .float_table
                .unify_var_var(a, b)
                .map_err(|_| TyCheckError::InvalidTypeConversion),

            (Ty::FloatVar(vid), Ty::Float(float_ty))
            | (Ty::Float(float_ty), Ty::FloatVar(vid)) => self
                .float_table
                .unify_var_value(vid, Some(float_ty))
                .map_err(|_| TyCheckError::TypeMissmatch {
                    expected: format!("{}", Ty::Float(float_ty)),
                    got: format!("{}", Ty::FloatVar(vid)),
                    src: span.0.clone(),
                    span: span.1.clone().into(),
                }),
            (Ty::Int(a), Ty::Int(b)) if a == b => Ok(()),
            (Ty::Float(a), Ty::Float(b)) if a == b => Ok(()),
            (Ty::Bool, Ty::Bool) => Ok(()),
            (Ty::String, Ty::String) => Ok(()),
            (Ty::Unit, Ty::Unit) => Ok(()),
            (Ty::Fn(params_a, ret_a), Ty::Fn(params_b, ret_b)) => {
                if params_a.len() != params_b.len() {
                    return Err(TyCheckError::ArityMissmatch {
                        expected: params_a.len(),
                        found: params_b.len(),
                        src: span.0.clone(),
                        span: span.1.clone().into(),
                    });
                }
                for (a, b) in params_a.into_iter().zip(params_b) {
                    self.unify(a, b, span.clone())?;
                }
                self.unify(*ret_a, *ret_b, span.clone())
            }
            (Ty::Adt(def_a, args_a), Ty::Adt(def_b, args_b))
                if def_a == def_b =>
            {
                if args_a.len() != args_b.len() {
                    return Err(TyCheckError::ArityMissmatch {
                        expected: args_a.len(),
                        found: args_b.len(),
                        src: span.0.clone(),
                        span: span.1.clone().into(),
                    });
                }
                for (a, b) in args_a.into_iter().zip(args_b) {
                    self.unify(a, b, span.clone())?;
                }
                Ok(())
            }

            (Ty::Param(d1, i1), Ty::Param(d2, i2))
                if d1 == d2 && i1 == i2 =>
            {
                Ok(())
            }

            (a, b) => Err(TyCheckError::TypeMissmatch {
                expected: format!("{}", a),
                got: format!("{}", b),
                src: span.0.clone(),
                span: span.1.clone().into(),
            }),
        }
    }

    /// Performs occurs check
    fn occurs_check(
        &mut self,
        vid: TyVid,
        ty: &Ty,
        span: Span,
    ) -> Result<(), TyCheckError> {
        match ty {
            Ty::Infer(other_vid) => {
                let root = self.table.find(*other_vid);
                if root == vid {
                    return Err(TyCheckError::OccursCheck {
                        vid: format!("{:?}", vid),
                        ty: format!("{}", ty),
                        src: span.0.clone(),
                        span: span.1.clone().into(),
                    });
                }
                match self.table.probe_value(root).0 {
                    Some(inner) => {
                        self.occurs_check(vid, &inner, span.clone())
                    }
                    None => Ok(()),
                }
            }
            Ty::Fn(params, ret) => {
                for p in params {
                    self.occurs_check(vid, p, span.clone())?;
                }
                self.occurs_check(vid, ret, span.clone())
            }
            Ty::Adt(_, args) => {
                for arg in args {
                    self.occurs_check(vid, arg, span.clone())?;
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }

    /// Snapshot inference data
    pub fn snapshot(&mut self) -> InferSnapshot {
        InferSnapshot {
            ty_snap: self.table.snapshot(),
            int_snap: self.int_table.snapshot(),
            float_snap: self.float_table.snapshot(),
        }
    }

    /// Rollbacks to inference snapshot
    pub fn rollback_to(&mut self, snap: InferSnapshot) {
        self.table.rollback_to(snap.ty_snap);
        self.int_table.rollback_to(snap.int_snap);
        self.float_table.rollback_to(snap.float_snap);
    }

    /// Commits snapsoht
    pub fn commit(&mut self, snap: InferSnapshot) {
        self.table.commit(snap.ty_snap);
        self.int_table.commit(snap.int_snap);
        self.float_table.commit(snap.float_snap);
    }
}

/// Inference snapshot
pub struct InferSnapshot {
    ty_snap: ena::unify::Snapshot<InPlace<TyVid>>,
    int_snap: ena::unify::Snapshot<InPlace<IntVid>>,
    float_snap: ena::unify::Snapshot<InPlace<FloatVid>>,
}
