use std::collections::HashSet;

use crow_mir::{Block, Local, MirBody, MirTyCtxt, Operand, Place, RETURN_PLACE, Rvalue, Statement, Terminator};

pub trait MirPass {
    fn name(&self) -> &str;

    fn run(&self, tcx: &MirTyCtxt, body: &mut MirBody);
}

pub fn run_passes(tcx: &MirTyCtxt, body: &mut MirBody, passes: &[Box<dyn MirPass>]) {
    for pass in passes {
        pass.run(tcx, body);
    }
}

pub fn mir_optimize(tcx: &MirTyCtxt, body: &mut MirBody) {
    let passes: Vec<Box<dyn MirPass>> = vec![
        Box::new(SimplifyCfg),
        Box::new(DeadCodeElim),
        Box::new(RemoveDeadBlocks),
        Box::new(SimplifyCfg),
    ];
    run_passes(tcx, body, &passes);
}

pub struct SimplifyCfg;

impl MirPass for SimplifyCfg {
    fn name(&self) -> &str { "simplify_cfg" }

    fn run(&self, _tcx: &MirTyCtxt, body: &mut MirBody) {
        loop {
            let mut changed = false;
            let preds = compute_predecessors(body);

            for i in 0..body.blocks.len() {
                if let Terminator::Goto(target) = body.blocks[i].term {
                    let j = target.index();
                    if i != j && preds[j].len() == 1 {
                        let stmts = std::mem::take(&mut body.blocks[j].stmts);
                        let term = std::mem::replace(
                            &mut body.blocks[j].term,
                            Terminator::Unreachable,
                        );
                        body.blocks[i].stmts.extend(stmts);
                        body.blocks[i].term = term;
                        changed = true;
                    }
                }
            }
            if !changed { break; }
        }
    }
}

pub struct DeadCodeElim;

impl MirPass for DeadCodeElim {
    fn name(&self) -> &str { "dce" }

    fn run(&self, _tcx: &MirTyCtxt, body: &mut MirBody) {
        let used = collect_used_locals(body);

        for bb in &mut body.blocks {
            bb.stmts.retain(|stmt| match stmt {
                Statement::Assign(place, _) => {
                    place.local == RETURN_PLACE || used.contains(&place.local)
                }
                _ => true,
            });
        }

        let mut assigned = HashSet::new();
        for bb in &body.blocks {
            for stmt in &bb.stmts {
                if let Statement::Assign(place, _) = stmt {
                    assigned.insert(place.local);
                }
            }
            if let Terminator::Call { dest, .. } = &bb.term {
                assigned.insert(dest.local);
            }
        }

        let dead_locals: HashSet<Local> = (0..body.locals.len())
            .map(Local::from)
            .filter(|l| *l != RETURN_PLACE)
            .filter(|l| !assigned.contains(l))
            .collect();

        for bb in &mut body.blocks {
            bb.stmts.retain(|stmt| match stmt {
                Statement::StorageLive(l) | Statement::StorageDead(l) => {
                    !dead_locals.contains(l)
                }
                _ => true,
            });
        }
    }
}

pub struct RemoveDeadBlocks;

impl MirPass for RemoveDeadBlocks {
    fn name(&self) -> &str { "remove_dead_blocks" }

    fn run(&self, _tcx: &MirTyCtxt, body: &mut MirBody) {
        let mut reachable = vec![false; body.blocks.len()];
        let mut worklist = vec![Block(0)];
        while let Some(bb) = worklist.pop() {
            if reachable[bb.index()] { continue; }
            reachable[bb.index()] = true;
            for succ in body.blocks[bb.index()].term.successors() {
                worklist.push(succ);
            }
        }
        for (i, alive) in reachable.iter().enumerate() {
            if !*alive {
                body.blocks[i].stmts.clear();
                body.blocks[i].term = Terminator::Unreachable;
            }
        }
    }
}

pub fn compute_predecessors(body: &MirBody) -> Vec<Vec<Block>> {
    let mut preds = vec![Vec::new(); body.blocks.len()];

    for (i, bb) in body.blocks.iter().enumerate() {
        let src = Block::from(i);
        for target in bb.term.successors() {
            preds[target.index()].push(src);
        }
    }

    preds
}

pub fn collect_used_locals(body: &MirBody) -> HashSet<Local> {
    let mut used = HashSet::new();

    for bb in &body.blocks {
        for stmt in &bb.stmts {
            if let Statement::Assign(place, rvalue) = stmt {
                collect_place_reads(place, &mut used);
                collect_rvalue_reads(rvalue, &mut used);
            }
        }
        collect_term_reads(&bb.term, &mut used);
    }

    used
}

fn collect_operand_reads(op: &Operand, used: &mut HashSet<Local>) {
    if let Operand::Copy(place) = op {
        used.insert(place.local);
    }
}

fn collect_place_reads(place: &Place, used: &mut HashSet<Local>) {
    if !place.proj.is_empty() {
        used.insert(place.local);
    }
}

fn collect_rvalue_reads(rvalue: &Rvalue, used: &mut HashSet<Local>) {
    match rvalue {
        Rvalue::Use(op) => collect_operand_reads(op, used),
        Rvalue::BinaryOp(_, lhs, rhs) => {
            collect_operand_reads(lhs, used);
            collect_operand_reads(rhs, used);
        }
        Rvalue::UnaryOp(_, op) => collect_operand_reads(op, used),
        Rvalue::Aggregate(_, ops) => {
            for op in ops { collect_operand_reads(op, used); }
        }
        Rvalue::Cast(_, op, _) => collect_operand_reads(op, used),
        Rvalue::Discriminant(place) => { used.insert(place.local); }
    }
}

fn collect_term_reads(term: &Terminator, used: &mut HashSet<Local>) {
    match term {
        Terminator::Call { func, args, .. } => {
            collect_operand_reads(func, used);
            for arg in args { collect_operand_reads(arg, used); }
        }
        Terminator::SwitchInt { discr, .. } => {
            collect_operand_reads(discr, used);
        }
        Terminator::Assert { cond, .. } => {
            collect_operand_reads(cond, used);
        }
        Terminator::Goto(_) | Terminator::Return | Terminator::Unreachable => {}
    }
}