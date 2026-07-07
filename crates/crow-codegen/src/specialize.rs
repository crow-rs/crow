use crow_mir::{BasicBlock, Constant, LocalDecl, MirBody, Operand, Rvalue, Statement, Substs, Terminator, subst_ty};

pub struct Specializer;

impl Specializer {
    pub fn specialize_body(body: &MirBody, substs: &Substs) -> MirBody {
        if substs.is_empty() { return body.clone(); }

        let locals = body.locals.iter()
            .map(|decl| LocalDecl {
                ty: subst_ty(&decl.ty, substs),
                name: decl.name.clone(),
                mutability: decl.mutability,
            })
            .collect();

        let blocks = body.blocks.iter()
            .map(|bb| BasicBlock {
                stmts: bb.stmts.iter().map(|s| Self::stmt(s, substs)).collect(),
                term: Self::term(&bb.term, substs),
            })
            .collect();

        MirBody {
            name: body.name.clone(),
            arg_count: body.arg_count,
            type_params: vec![],
            locals,
            blocks,
        }
    }

    fn stmt(stmt: &Statement, substs: &Substs) -> Statement {
        match stmt {
            Statement::Assign(place, rvalue) => {
                Statement::Assign(place.clone(), Self::rvalue(rvalue, substs))
            }
            other => other.clone(),
        }
    }

    fn rvalue(rv: &Rvalue, substs: &Substs) -> Rvalue {
        match rv {
            Rvalue::Cast(kind, op, ty) => {
                Rvalue::Cast(*kind, op.clone(), subst_ty(ty, substs))
            }
            other => other.clone(),
        }
    }

    fn term(term: &Terminator, substs: &Substs) -> Terminator {
        match term {
            Terminator::Call { func, args, dest, target } => {
                let func = match func {
                    Operand::Const(Constant::Fn(id, fn_substs)) => {
                        let resolved: Substs = fn_substs.iter()
                            .map(|t| subst_ty(t, substs))
                            .collect();
                        Operand::Const(Constant::Fn(*id, resolved))
                    }
                    other => other.clone(),
                };
                Terminator::Call {
                    func,
                    args: args.clone(),
                    dest: dest.clone(),
                    target: *target,
                }
            }
            other => other.clone(),
        }
    }
}