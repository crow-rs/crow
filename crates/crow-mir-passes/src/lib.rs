use std::collections::{HashMap, HashSet};

use crow_mir::{BinOp, Block, Constant, FnId, Local, MirBody, MirModule, MirTyCtxt, Operand, Place, Projection, RETURN_PLACE, Rvalue, Statement, Terminator, UnOp};
use crow_types::Ty;

pub trait MirPass {
    fn name(&self) -> &str;

    fn run(&self, tcx: &MirTyCtxt, body: &mut MirBody);
}

pub fn run_passes(tcx: &MirTyCtxt, body: &mut MirBody, passes: &[Box<dyn MirPass>]) {
    for pass in passes {
        pass.run(tcx, body);
    }
}

pub fn mir_optimize(mir: &mut MirModule) {
    let rc = InsertRefCounting::new(mir);

    let mut functions = std::mem::take(&mut mir.functions);

    for body in &mut functions {
        if body.blocks.is_empty() { continue; }

        let passes: Vec<Box<dyn MirPass>> = vec![
            Box::new(ConstProp),
            Box::new(SimplifyCfg),
            Box::new(DeadCodeElim),
            Box::new(rc.clone()),
            Box::new(RemoveDeadBlocks),
            Box::new(SimplifyCfg),
        ];
        run_passes(&mir.tcx, body, &passes);
    }

    mir.functions = functions;
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
        let mut count = 0usize;
        while let Some(bb) = worklist.pop() {
            if reachable[bb.index()] { continue; }
            reachable[bb.index()] = true;
            count += 1;
            for succ in body.blocks[bb.index()].term.successors() {
                worklist.push(succ);
            }
        }

        if count == body.blocks.len() { return; }

        let mut new_index = vec![0u32; body.blocks.len()];
        let mut next = 0u32;
        for (i, &alive) in reachable.iter().enumerate() {
            if alive {
                new_index[i] = next;
                next += 1;
            }
        }

        let mut i = 0;
        body.blocks.retain(|_| {
            let keep = reachable[i];
            i += 1;
            keep
        });

        for bb in &mut body.blocks {
            remap_terminator(&mut bb.term, &new_index);
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

fn remap_terminator(term: &mut Terminator, map: &[u32]) {
    match term {
        Terminator::Goto(b) => {
            *b = Block(map[b.index()]);
        }
        Terminator::SwitchInt { targets, otherwise, .. } => {
            for (_, b) in targets.iter_mut() {
                *b = Block(map[b.index()]);
            }
            *otherwise = Block(map[otherwise.index()]);
        }
        Terminator::Call { target, .. } => {
            *target = Block(map[target.index()]);
        }
        Terminator::Assert { target, .. } => {
            *target = Block(map[target.index()]);
        }
        Terminator::Return | Terminator::Unreachable => {}
    }
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

#[derive(Clone)]
pub struct InsertRefCounting {
    retain_fn: FnId,
    release_fn: FnId,
}

impl InsertRefCounting {
    pub fn new(mir: &MirModule) -> Self {
        Self {
            retain_fn: mir.lang.gc.as_ref().unwrap().release.clone(),
            release_fn: mir.lang.gc.as_ref().unwrap().retain.clone(),
        }
    }
}

fn place_ty_full(tcx: &MirTyCtxt, body: &MirBody, place: &Place) -> Ty {
    let mut ty = body.locals[place.local.index()].ty.clone();
    let mut active_variant: u32 = 0;
    for proj in &place.proj {
        match proj {
            Projection::Downcast(v) => { active_variant = *v; }
            Projection::Field(idx) => {
                ty = match &ty {
                    Ty::Adt(def_id, _) => {
                        tcx.adt(*def_id).variants[active_variant as usize]
                            .fields[*idx as usize].clone()
                    }
                    _ => ty.clone(),
                };
                active_variant = 0;
            }
        }
    }
    ty
}

impl MirPass for InsertRefCounting {
    fn name(&self) -> &str { "rc_insert" }
    
    fn run(&self, tcx: &MirTyCtxt, body: &mut MirBody) {
        if body.locals.first().map_or(false, |ret| ret.ty == Ty::Never) {
            return;
        }

        let rc_locals: Vec<Local> = body.locals.iter()
            .enumerate()
            .filter(|(_, decl)| is_rc_type(&decl.ty))
            .map(|(i, _)| Local::from(i))
            .collect();

        if rc_locals.is_empty() { return; }

        let mut initialized: HashSet<Local> = HashSet::new();

        for bb_idx in 0..body.blocks.len() {
            let mut new_stmts = Vec::new();

            for stmt in &body.blocks[bb_idx].stmts {
                match stmt {
                    Statement::Assign(dest, Rvalue::Aggregate(_, ops)) => {
                        for op in ops {
                            if let Operand::Copy(src) = op {
                                let src_ty = place_ty_full(tcx, body, src);
                                if is_rc_type(&src_ty) {
                                    // Нужно retain src — но src может быть с proj
                                    // Retain(src.local) если proj пустой
                                    if src.proj.is_empty() {
                                        new_stmts.push(Statement::Retain(src.local));
                                    }
                                }
                            }
                        }
                        if is_rc_local(dest.local, &rc_locals)
                            && dest.proj.is_empty()
                            && initialized.contains(&dest.local)
                        {
                            new_stmts.push(Statement::Release(dest.local));
                        }
                        new_stmts.push(stmt.clone());
                        if is_rc_local(dest.local, &rc_locals) && dest.proj.is_empty() {
                            initialized.insert(dest.local);
                        }
                    }

                    Statement::Assign(dest, Rvalue::Use(Operand::Copy(src))) => {
                        let src_ty = place_ty_full(tcx, body, src);

                        // Release старого dest
                        if is_rc_type(&src_ty)
                            && dest.proj.is_empty()
                            && initialized.contains(&dest.local)
                        {
                            new_stmts.push(Statement::Release(dest.local));
                        }

                        // Присваивание
                        new_stmts.push(stmt.clone());

                        // Retain dest ПОСЛЕ присваивания
                        if is_rc_type(&src_ty) && dest.proj.is_empty() {
                            new_stmts.push(Statement::Retain(dest.local));
                            initialized.insert(dest.local);
                        }
                    }

                    Statement::Assign(dest, _) => {
                        if is_rc_local(dest.local, &rc_locals)
                            && dest.proj.is_empty()
                            && initialized.contains(&dest.local)
                        {
                            new_stmts.push(Statement::Release(dest.local));
                        }
                        new_stmts.push(stmt.clone());
                        if is_rc_local(dest.local, &rc_locals) && dest.proj.is_empty() {
                            initialized.insert(dest.local);
                        }
                    }

                    Statement::StorageDead(local) => {
                        if is_rc_local(*local, &rc_locals) && initialized.contains(local) {
                            new_stmts.push(Statement::Release(*local));
                            initialized.remove(local);
                        }
                        new_stmts.push(stmt.clone());
                    }

                    other => new_stmts.push(other.clone()),
                }
            }

            match &body.blocks[bb_idx].term {
                Terminator::Call { args, dest, .. } => {
                    for arg in args {
                        if let Operand::Copy(src) = arg {
                            let src_ty = place_ty_full(tcx, body, src);
                            if is_rc_type(&src_ty) && src.proj.is_empty() {
                                new_stmts.push(Statement::Retain(src.local));
                            }
                        }
                    }
                    if is_rc_type(&body.locals[dest.local.index()].ty)
                        && dest.proj.is_empty()
                    {
                        initialized.insert(dest.local);
                    }
                }
                _ => {}
            }

            if matches!(body.blocks[bb_idx].term, Terminator::Return) {
                for i in 0..body.locals.len() {
                    let local = Local::from(i);
                    if local == RETURN_PLACE { continue; }
                    if !is_rc_type(&body.locals[i].ty) { continue; }
                    let is_arg = i >= 1 && i <= body.arg_count;
                    if is_arg || initialized.contains(&local) {
                        new_stmts.push(Statement::Release(local));
                    }
                }
            }

            body.blocks[bb_idx].stmts = new_stmts;
        }
    }
}

fn is_rc_type(ty: &Ty) -> bool {
    matches!(ty, Ty::Adt(_, _))
}

fn is_rc_local(local: Local, rc_locals: &[Local]) -> bool {
    rc_locals.contains(&local)
}

pub struct ConstProp;

impl MirPass for ConstProp {
    fn name(&self) -> &str { "const_prop" }

    fn run(&self, _tcx: &MirTyCtxt, body: &mut MirBody) {
        let mut consts: HashMap<Local, Constant> = HashMap::new();

        for bb_idx in 0..body.blocks.len() {
            let stmts = std::mem::take(&mut body.blocks[bb_idx].stmts);
            let mut new_stmts = Vec::new();

            for mut stmt in stmts {
                if let Statement::Assign(ref place, ref mut rvalue) = stmt {
                    subst_rvalue(rvalue, &consts);

                    if place.proj.is_empty() {
                        if let Some(c) = try_fold(rvalue) {
                            consts.insert(place.local, c.clone());
                            *rvalue = Rvalue::Use(Operand::Const(c));
                        } else if let Rvalue::Use(Operand::Const(c)) = rvalue {
                            consts.insert(place.local, c.clone());
                        } else {
                            consts.remove(&place.local);
                        }
                    }
                }
                new_stmts.push(stmt);
            }

            body.blocks[bb_idx].stmts = new_stmts;

            subst_terminator(&mut body.blocks[bb_idx].term, &consts);

            match &body.blocks[bb_idx].term {
                Terminator::SwitchInt { .. } | Terminator::Return => {
                    consts.clear();
                }
                _ => {}
            }
        }
    }
}

fn subst_rvalue(rvalue: &mut Rvalue, consts: &HashMap<Local, Constant>) {
    match rvalue {
        Rvalue::Use(op) => subst_operand(op, consts),
        Rvalue::BinaryOp(_, lhs, rhs) => {
            subst_operand(lhs, consts);
            subst_operand(rhs, consts);
        }
        Rvalue::UnaryOp(_, op) => subst_operand(op, consts),
        Rvalue::Cast(_, op, _) => subst_operand(op, consts),
        _ => {}
    }
}

fn subst_operand(op: &mut Operand, consts: &HashMap<Local, Constant>) {
    if let Operand::Copy(place) = op {
        if place.proj.is_empty() {
            if let Some(c) = consts.get(&place.local) {
                *op = Operand::Const(c.clone());
            }
        }
    }
}

fn subst_terminator(term: &mut Terminator, consts: &HashMap<Local, Constant>) {
    match term {
        Terminator::SwitchInt { discr, .. } => {
            subst_operand(discr, consts);
        }
        Terminator::Call { args, .. } => {
            for arg in args {
                subst_operand(arg, consts);
            }
        }
        Terminator::Assert { cond, .. } => {
            subst_operand(cond, consts);
        }
        _ => {}
    }
}

fn try_fold(rvalue: &Rvalue) -> Option<Constant> {
    match rvalue {
        Rvalue::BinaryOp(op, Operand::Const(lhs), Operand::Const(rhs)) => {
            fold_binop(op, lhs, rhs)
        }
        Rvalue::UnaryOp(op, Operand::Const(c)) => {
            fold_unop(op, c)
        }
        _ => None,
    }
}

fn fold_binop(op: &BinOp, lhs: &Constant, rhs: &Constant) -> Option<Constant> {
    match (op, lhs, rhs) {
        (BinOp::Add, Constant::Int(a, ty), Constant::Int(b, _)) => {
            Some(Constant::Int(a + b, *ty))
        }
        (BinOp::Sub, Constant::Int(a, ty), Constant::Int(b, _)) => {
            Some(Constant::Int(a - b, *ty))
        }
        (BinOp::Mul, Constant::Int(a, ty), Constant::Int(b, _)) => {
            Some(Constant::Int(a * b, *ty))
        }
        (BinOp::Div, Constant::Int(a, ty), Constant::Int(b, _)) if *b != 0 => {
            Some(Constant::Int(a / b, *ty))
        }
        (BinOp::Rem, Constant::Int(a, ty), Constant::Int(b, _)) if *b != 0 => {
            Some(Constant::Int(a % b, *ty))
        }

        (BinOp::Eq, Constant::Int(a, _), Constant::Int(b, _)) => {
            Some(Constant::Bool(a == b))
        }
        (BinOp::Ne, Constant::Int(a, _), Constant::Int(b, _)) => {
            Some(Constant::Bool(a != b))
        }
        (BinOp::Lt, Constant::Int(a, _), Constant::Int(b, _)) => {
            Some(Constant::Bool(a < b))
        }
        (BinOp::Le, Constant::Int(a, _), Constant::Int(b, _)) => {
            Some(Constant::Bool(a <= b))
        }
        (BinOp::Gt, Constant::Int(a, _), Constant::Int(b, _)) => {
            Some(Constant::Bool(a > b))
        }
        (BinOp::Ge, Constant::Int(a, _), Constant::Int(b, _)) => {
            Some(Constant::Bool(a >= b))
        }

        (BinOp::Add, Constant::Str(a), Constant::Str(b)) => {
            Some(Constant::Str(format!("{}{}", a, b)))
        }

        _ => None,
    }
}

fn fold_unop(op: &UnOp, c: &Constant) -> Option<Constant> {
    match (op, c) {
        (UnOp::Neg, Constant::Int(v, ty)) => Some(Constant::Int(-v, *ty)),
        (UnOp::Not, Constant::Bool(v)) => Some(Constant::Bool(!v)),
        _ => None,
    }
}