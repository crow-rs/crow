use std::collections::HashMap;

use crow_mir::{AggregateKind, ConstId, Constant, FnId, MirModule, Operand, Rvalue, Statement, Substs, Terminator, subst_ty};

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Instance {
    pub fn_id: FnId,
    pub substs: Substs,
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub enum MonoItem {
    Fn(Instance),
    Const(ConstId, Substs),
}

pub struct Monomorph {
    items: Vec<MonoItem>,
    seen: HashMap<MonoItem, usize>,
}

impl Monomorph {
    pub fn new() -> Self {
        Self { items: Vec::new(), seen: HashMap::new() }
    }

    pub fn collect(&mut self, module: &MirModule, entry: FnId) {
        let root = Instance { fn_id: entry, substs: vec![] };
        self.visit(module, root);
    }

    fn visit(&mut self, module: &MirModule, inst: Instance) {
        if self.seen.contains_key(&MonoItem::Fn(inst.clone())) {
            return;
        }
        let idx = self.items.len();
        self.seen.insert(MonoItem::Fn(inst.clone()), idx);
        self.items.push(MonoItem::Fn(inst.clone()));

        let body = &module.functions[inst.fn_id.index()];
        for bb in &body.blocks {
            for stmt in &bb.stmts {
                if let Statement::Assign(_, rvalue) = stmt {
                    self.collect_rvalue(module, rvalue, &inst.substs);
                }
            }
            self.collect_terminator(module, &bb.term, &inst.substs);
        }
    }

    fn collect_terminator(
        &mut self,
        module: &MirModule,
        term: &Terminator,
        caller_substs: &Substs,
    ) {
        match term {
            Terminator::Call { func, args, .. } => {
                self.collect_operand(module, func, caller_substs);
                for arg in args {
                    self.collect_operand(module, arg, caller_substs);
                }
            }
            Terminator::SwitchInt { discr, .. } => {
                self.collect_operand(module, discr, caller_substs);
            }
            Terminator::Assert { cond, .. } => {
                self.collect_operand(module, cond, caller_substs);
            }
            Terminator::Goto(_) | Terminator::Return | Terminator::Unreachable => {}
        }
    }

    fn collect_operand(
        &mut self,
        module: &MirModule,
        op: &Operand,
        caller_substs: &Substs,
    ) {
        if let Operand::Const(Constant::Fn(fn_id, fn_substs)) = op {
            let resolved: Substs = fn_substs.iter()
                .map(|t| subst_ty(t, caller_substs))
                .collect();
            self.visit(module, Instance {
                fn_id: *fn_id,
                substs: resolved,
            });
        }
    }

    fn collect_rvalue(
        &mut self,
        module: &MirModule,
        rvalue: &Rvalue,
        caller_substs: &Substs,
    ) {
        match rvalue {
            Rvalue::Use(op) => {
                self.collect_operand(module, op, caller_substs);
            }
            Rvalue::BinaryOp(_, lhs, rhs) => {
                self.collect_operand(module, lhs, caller_substs);
                self.collect_operand(module, rhs, caller_substs);
            }
            Rvalue::UnaryOp(_, op) => {
                self.collect_operand(module, op, caller_substs);
            }
            Rvalue::Aggregate(kind, ops) => {
                if let AggregateKind::Closure { fn_id, .. } = kind {
                    let inst = Instance {
                        fn_id: *fn_id,
                        substs: caller_substs.clone(),
                    };
                    self.visit(module, inst);
                }
                for op in ops {
                    self.collect_operand(module, op, caller_substs);
                }
            }
            Rvalue::Cast(_, op, _) => {
                self.collect_operand(module, op, caller_substs);
            }
            Rvalue::Discriminant(_) => {}
        }
    }

    pub fn into_items(self) -> Vec<MonoItem> {
        self.items
    }
}
