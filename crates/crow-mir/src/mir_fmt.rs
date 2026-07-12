use std::fmt;
use crow_ast::atom::Mutability;

use crate::{
    AggregateKind, AssertMsg, BinOp, Block, CastKind, ConstId, Constant, FnId, Local, MirBody, MirConstDef, MirModule, MirNative, NativeId, Operand, Place, Projection, Rvalue, Statement, Terminator, UnOp
};

impl fmt::Display for BinOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            BinOp::Add => "Add", BinOp::Sub => "Sub",
            BinOp::Mul => "Mul", BinOp::Div => "Div", BinOp::Rem => "Rem",
            BinOp::BitAnd => "BitAnd", BinOp::BitOr => "BitOr",
            BinOp::BitXor => "BitXor", BinOp::Shl => "Shl", BinOp::Shr => "Shr",
            BinOp::Eq => "Eq", BinOp::Ne => "Ne",
            BinOp::Lt => "Lt", BinOp::Le => "Le",
            BinOp::Gt => "Gt", BinOp::Ge => "Ge",
        })
    }
}

impl fmt::Display for UnOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            UnOp::Not => "Not",
            UnOp::Neg => "Neg",
        })
    }
}

impl fmt::Display for CastKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            CastKind::IntToInt => "IntToInt",
            CastKind::IntToFloat => "IntToFloat",
            CastKind::FloatToInt => "FloatToInt",
            CastKind::FloatToFloat => "FloatToFloat",
        })
    }
}

impl fmt::Display for Place {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "_{}", self.local.0)?;
        for proj in &self.proj {
            match proj {
                Projection::Field(i) => write!(f, ".{i}")?,
                Projection::Downcast(v) => write!(f, " as variant#{v}")?,
            }
        }
        Ok(())
    }
}

impl fmt::Display for Constant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Constant::Unit => write!(f, "const ()"),
            Constant::Bool(v) => write!(f, "const {v}"),
            Constant::Int(v, ty) => write!(f, "const {v}_{ty}"),
            Constant::Float(v, ty) => write!(f, "const {v}_{ty}"),
            Constant::Str(s) => write!(f, "const {:?}", s),
            Constant::Fn(id, substs) => {
                write!(f, "fn{}", id.0)?;
                if !substs.is_empty() {
                    write!(f, "<")?;
                    for (i, s) in substs.iter().enumerate() {
                        if i > 0 { write!(f, ", ")?; }
                        write!(f, "{s}")?;
                    }
                    write!(f, ">")?;
                }
                Ok(())
            }
            Constant::Native(id) => write!(f, "native{}", id.0),
            Constant::Global(id) => write!(f, "const#{}", id.0),
        }
    }
}

impl fmt::Display for Operand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Operand::Copy(place) => write!(f, "copy {place}"),
            Operand::Const(c) => write!(f, "{c}"),
        }
    }
}

impl fmt::Display for AggregateKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AggregateKind::Adt { def_id, variant } => {
                write!(f, "adt#{}::variant#{}", def_id.0, variant)
            }
            AggregateKind::Closure { fn_id, env_def } => {
                write!(f, "closure(fn{}, env=adt#{})", fn_id.0, env_def.0)
            }
        }
    }
}

impl fmt::Display for Rvalue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Rvalue::Use(op) => write!(f, "{op}"),
            Rvalue::BinaryOp(op, lhs, rhs) => write!(f, "{op}({lhs}, {rhs})"),
            Rvalue::UnaryOp(op, operand) => write!(f, "{op}({operand})"),
            Rvalue::Aggregate(kind, ops) => {
                write!(f, "{kind}(")?;
                for (i, op) in ops.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{op}")?;
                }
                write!(f, ")")
            }
            Rvalue::Discriminant(place) => write!(f, "discriminant({place})"),
            Rvalue::Cast(kind, op, ty) => write!(f, "{op} as {ty} ({kind})"),
        }
    }
}

impl fmt::Display for Statement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Statement::Assign(place, rvalue) => write!(f, "{place} = {rvalue}"),
            Statement::StorageLive(l) => write!(f, "StorageLive(_{l})", l = l.0),
            Statement::StorageDead(l) => write!(f, "StorageDead(_{l})", l = l.0),
            Statement::Release(l) => write!(f, "__release__({l})", l = l.0),
            Statement::Retain(l) => write!(f, "__retain__({l})", l = l.0),
            Statement::Nop => write!(f, "nop"),
        }
    }
}

impl fmt::Display for AssertMsg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AssertMsg::DivisionByZero => write!(f, "division by zero"),
            AssertMsg::RemainderByZero => write!(f, "remainder by zero"),
            AssertMsg::Overflow(op) => write!(f, "overflow on {op}"),
            AssertMsg::MatchFailed => write!(f, "non-exhaustive match"),
        }
    }
}

impl fmt::Display for Terminator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Terminator::Goto(bb) => write!(f, "goto -> {bb}"),
            Terminator::SwitchInt { discr, targets, otherwise } => {
                write!(f, "switchInt({discr}) -> [")?;
                for (i, (val, bb)) in targets.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{val}: bb{}", bb.0)?;
                }
                write!(f, ", otherwise: bb{}]", otherwise.0)
            }
            Terminator::Call { func, args, dest, target } => {
                write!(f, "{dest} = call {func}(")?;
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{arg}")?;
                }
                write!(f, ") -> bb{}", target.0)
            }
            Terminator::Assert { cond, expected, msg, target } => {
                write!(f, "assert({cond}, {expected}, \"{msg}\") -> bb{}", target.0)
            }
            Terminator::Return => write!(f, "return"),
            Terminator::Unreachable => write!(f, "unreachable"),
        }
    }
}

impl fmt::Display for MirBody {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.locals.is_empty() {
            return writeln!(f, "fn {}() = <intrinsic>", self.name);
        }
        write!(f, "fn {}(", self.name)?;

        for i in 1..=self.arg_count {
            if i > 1 { write!(f, ", ")?; }
            let decl = &self.locals[i];
            write!(f, "_{i}: {}", decl.ty)?;
        }
        writeln!(f, ") -> {} {{", self.locals[0].ty)?;

        for (i, decl) in self.locals.iter().enumerate().skip(1 + self.arg_count) {
            let mutstr = match decl.mutability {
                Mutability::Mut => "mut ",
                Mutability::Immut => "",
            };
            write!(f, "    let {mutstr}_{i}: {}", decl.ty)?;
            if let Some(name) = &decl.name {
                write!(f, ";{:>pad$}// \"{name}\"", "", pad = 20usize.saturating_sub(
                    format!("_{i}: {}", decl.ty).len()
                ))?;
            } else {
                write!(f, ";")?;
            }
            writeln!(f)?;
        }

        if self.locals.len() > 1 + self.arg_count {
            writeln!(f)?;
        }

        for (i, bb) in self.blocks.iter().enumerate() {
            writeln!(f, "    bb{i}: {{")?;
            for stmt in &bb.stmts {
                writeln!(f, "        {stmt};")?;
            }
            writeln!(f, "        {};", bb.term)?;
            writeln!(f, "    }}")?;
        }

        write!(f, "}}")
    }
}

impl fmt::Display for MirNative {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "native fn {}(", self.name)?;
        for (i, ty) in self.params.iter().enumerate() {
            if i > 0 { write!(f, ", ")?; }
            write!(f, "{ty}")?;
        }
        write!(f, ") -> {} = \"{}\"", self.ret, self.symbol)
    }
}


impl fmt::Display for MirConstDef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "const {}: {} = {}", self.name, self.ty, self.value)
    }
}

impl fmt::Display for Block {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "bb{}", self.0)
    }
}

impl fmt::Display for Local {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "_{}", self.0)
    }
}

impl fmt::Display for FnId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "fn{}", self.0)
    }
}

impl fmt::Display for NativeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "native{}", self.0)
    }
}

impl fmt::Display for ConstId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "const#{}", self.0)
    }
}

impl fmt::Display for MirModule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // ADT definitions
        for (def_id, adt) in &self.tcx.adts {
            if adt.variants.len() == 1 {
                // rec (struct)
                writeln!(f, "rec {} {{  // adt#{}", adt.name, def_id.0)?;
                for (i, field) in adt.variants[0].fields.iter().enumerate() {
                    writeln!(f, "    {i}: {field}")?;
                }
                writeln!(f, "}}\n")?;
            } else {
                // enum
                writeln!(f, "enum {} {{  // adt#{}", adt.name, def_id.0)?;
                for variant in &adt.variants {
                    if variant.fields.is_empty() {
                        writeln!(f, "    {}", variant.name)?;
                    } else {
                        let fields: Vec<String> = variant.fields.iter()
                            .map(|t| format!("{t}"))
                            .collect();
                        writeln!(f, "    {}({})", variant.name, fields.join(", "))?;
                    }
                }
                writeln!(f, "}}\n")?;
            }
        }

        for native in &self.natives {
            writeln!(f, "{native}")?;
        }
        if !self.natives.is_empty() { writeln!(f)?; }

        for c in &self.constants {
            writeln!(f, "{c}")?;
        }
        if !self.constants.is_empty() { writeln!(f)?; }

        for (i, body) in self.functions.iter().enumerate() {
            if i > 0 { writeln!(f)?; }
            writeln!(f, "{body}")?;
        }

        Ok(())
    }
}