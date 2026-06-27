use crate::*;
use std::fmt::Write;

pub fn dump_mir_module(module: &MirModule) {
    let mut out = String::new();

    for (i, s) in module.structs.iter().enumerate() {
        writeln!(out, "struct {} {{ // id={}", s.name, i).unwrap();
        for f in &s.fields {
            writeln!(out, "    {}: {},", f.name, fmt_type(&f.ty)).unwrap();
        }
        writeln!(out, "}}\n").unwrap();
    }

    for (i, e) in module.enums.iter().enumerate() {
        writeln!(out, "enum {} {{ // id={}", e.name, i).unwrap();
        for (vi, v) in e.variants.iter().enumerate() {
            if v.fields.is_empty() {
                writeln!(out, "    {}, // variant={}", v.name, vi).unwrap();
            } else {
                let fields: Vec<_> = v.fields.iter().map(|f| fmt_type(f)).collect();
                writeln!(out, "    {}({}), // variant={}", v.name, fields.join(", "), vi).unwrap();
            }
        }
        writeln!(out, "}}\n").unwrap();
    }

    for (i, f) in module.functions.iter().enumerate() {
        let entry_marker = if module.entry == i as u32 { " // [entry]" } else { "" };
        let params: Vec<_> = f.params.iter().enumerate()
            .map(|(j, ty)| format!("_{}: {}", j + 1, fmt_type(ty)))
            .collect();
        writeln!(out, "fn {}({}) -> {} {{{}", f.name, params.join(", "), fmt_type(&f.ret), entry_marker).unwrap();

        match &f.body {
            Some(body) => dump_body(&mut out, body),
            None => { writeln!(out, "    // <native>").unwrap(); }
        }

        writeln!(out, "}}\n").unwrap();
    }

    print!("{}", out);
}

fn dump_body(out: &mut String, body: &MirBody) {
    // Locals
    for (i, local) in body.locals.iter().enumerate() {
        let kind = match local.kind {
            MirLocalKind::Return => "return",
            MirLocalKind::Arg => "arg",
            MirLocalKind::Temp => "temp",
            MirLocalKind::User => "user",
        };
        let name = local.name.as_deref().unwrap_or("");
        if !name.is_empty() && kind == "user" {
            writeln!(out, "    let _{}: {};  // {}", i, fmt_type(&local.ty), name).unwrap();
        } else if kind == "return" {
            writeln!(out, "    let _0: {};  // return", fmt_type(&local.ty)).unwrap();
        } else if kind == "arg" {
            writeln!(out, "    let _{}: {};  // arg \"{}\"", i, fmt_type(&local.ty), name).unwrap();
        } else {
            writeln!(out, "    let _{}: {};", i, fmt_type(&local.ty)).unwrap();
        }
    }

    if !body.locals.is_empty() {
        writeln!(out).unwrap();
    }

    // Blocks
    for (i, bb) in body.blocks.iter().enumerate() {
        writeln!(out, "    bb{}: {{", i).unwrap();

        for stmt in &bb.stmts {
            write!(out, "        ").unwrap();
            dump_stmt(out, stmt);
            writeln!(out, ";").unwrap();
        }

        write!(out, "        ").unwrap();
        dump_terminator(out, &bb.terminator);
        writeln!(out, ";").unwrap();

        writeln!(out, "    }}").unwrap();
        if i + 1 < body.blocks.len() {
            writeln!(out).unwrap();
        }
    }
}

fn dump_stmt(out: &mut String, stmt: &MirStatement) {
    match stmt {
        MirStatement::Assign(place, rvalue) => {
            write!(out, "{} = {}", fmt_place(place), fmt_rvalue(rvalue)).unwrap();
        }
        MirStatement::Nop => {
            write!(out, "nop").unwrap();
        }
    }
}

fn dump_terminator(out: &mut String, term: &MirTerminator) {
    match term {
        MirTerminator::Goto(bb) => {
            write!(out, "goto -> bb{}", bb).unwrap();
        }
        MirTerminator::SwitchInt { discr, targets, otherwise } => {
            let cases: Vec<_> = targets.iter()
                .map(|(val, bb)| format!("{}: bb{}", val, bb))
                .collect();
            write!(
                out, "switchInt({}) -> [{}otherwise: bb{}]",
                fmt_operand(discr),
                if cases.is_empty() { String::new() } else { format!("{}, ", cases.join(", ")) },
                otherwise
            ).unwrap();
        }
        MirTerminator::Call { dest, func, args, target } => {
            let args_str: Vec<_> = args.iter().map(|a| fmt_operand(a)).collect();
            write!(
                out, "{} = call {}({}) -> bb{}",
                fmt_place(dest),
                fmt_operand(func),
                args_str.join(", "),
                target
            ).unwrap();
        }
        MirTerminator::Return => {
            write!(out, "return").unwrap();
        }
        MirTerminator::Abort => {
            write!(out, "abort").unwrap();
        }
        MirTerminator::Unreachable => {
            write!(out, "unreachable").unwrap();
        }
    }
}

fn fmt_place(place: &MirPlace) -> String {
    let mut s = format!("_{}", place.local);
    for proj in &place.projection {
        match proj {
            MirProjection::Field(idx) => write!(s, ".{}", idx).unwrap(),
            MirProjection::Index(local) => write!(s, "[_{}]", local).unwrap(),
            MirProjection::Downcast(idx) => write!(s, " as variant#{}", idx).unwrap(),
        }
    }
    s
}

fn fmt_rvalue(rv: &MirRvalue) -> String {
    match rv {
        MirRvalue::Use(op) => fmt_operand(op),
        MirRvalue::BinOp(op, l, r) => {
            format!("{}({}, {})", fmt_binop(op), fmt_operand(l), fmt_operand(r))
        }
        MirRvalue::UnOp(op, v) => {
            format!("{}({})", fmt_unop(op), fmt_operand(v))
        }
        MirRvalue::Aggregate(kind, fields) => {
            let fields_str: Vec<_> = fields.iter().map(|f| fmt_operand(f)).collect();
            match kind {
                MirAggregateKind::Struct(id) => {
                    format!("Struct#{}({})", id, fields_str.join(", "))
                }
                MirAggregateKind::Variant(id, idx) => {
                    format!("Variant#{}::{}({})", id, idx, fields_str.join(", "))
                }
                MirAggregateKind::Array => {
                    format!("[{}]", fields_str.join(", "))
                }
                MirAggregateKind::Closure(id) => {
                    format!("Closure#{}({})", id, fields_str.join(", "))
                }
            }
        }
        MirRvalue::Ref(place) => format!("&{}", fmt_place(place)),
        MirRvalue::Cast(kind, op, ty) => {
            let kind_str = match kind {
                MirCastKind::Primitive => "cast",
                MirCastKind::Upcast => "upcast",
            };
            format!("{} {} as {}", kind_str, fmt_operand(op), fmt_type(ty))
        }
        MirRvalue::Discriminant(place) => {
            format!("discriminant({})", fmt_place(place))
        }
        MirRvalue::Len(place) => {
            format!("len({})", fmt_place(place))
        }
    }
}

fn fmt_operand(op: &MirOperand) -> String {
    match op {
        MirOperand::Copy(place) => fmt_place(place),
        MirOperand::Constant(c) => fmt_constant(c),
    }
}

fn fmt_constant(c: &MirConstant) -> String {
    match c {
        MirConstant::Int(v) => format!("const {}_i64", v),
        MirConstant::Float(v) => format!("const {}_f64", v),
        MirConstant::Bool(v) => format!("const {}", v),
        MirConstant::Str(s) => format!("const {:?}", s),
        MirConstant::Unit => "const ()".to_string(),
        MirConstant::FunRef(id) => format!("fn#{}", id),
    }
}

fn fmt_type(ty: &MirType) -> String {
    match ty {
        MirType::Int => "int".to_string(),
        MirType::Float => "float".to_string(),
        MirType::Bool => "bool".to_string(),
        MirType::Str => "str".to_string(),
        MirType::Unit => "()".to_string(),
        MirType::Struct(id) => format!("Struct#{}", id),
        MirType::Enum(id) => format!("Enum#{}", id),
        MirType::Array(inner) => format!("[{}]", fmt_type(inner)),
        MirType::FunPtr { params, ret } => {
            let params: Vec<_> = params.iter().map(|p| fmt_type(p)).collect();
            format!("fn({}) -> {}", params.join(", "), fmt_type(ret))
        }
        MirType::Closure { params, ret } => {
            let params: Vec<_> = params.iter().map(|p| fmt_type(p)).collect();
            format!("closure({}) -> {}", params.join(", "), fmt_type(ret))
        }
        MirType::Never => "!".to_string(),
    }
}

fn fmt_binop(op: &MirBinOp) -> &'static str {
    match op {
        MirBinOp::Add => "Add",
        MirBinOp::Sub => "Sub",
        MirBinOp::Mul => "Mul",
        MirBinOp::Div => "Div",
        MirBinOp::Rem => "Rem",
        MirBinOp::BitAnd => "BitAnd",
        MirBinOp::BitOr => "BitOr",
        MirBinOp::BitXor => "BitXor",
        MirBinOp::Shl => "Shl",
        MirBinOp::Shr => "Shr",
        MirBinOp::Eq => "Eq",
        MirBinOp::Ne => "Ne",
        MirBinOp::Lt => "Lt",
        MirBinOp::Le => "Le",
        MirBinOp::Gt => "Gt",
        MirBinOp::Ge => "Ge",
        MirBinOp::And => "And",
        MirBinOp::Or => "Or",
    }
}

fn fmt_unop(op: &MirUnOp) -> &'static str {
    match op {
        MirUnOp::Neg => "Neg",
        MirUnOp::Not => "Not",
    }
}