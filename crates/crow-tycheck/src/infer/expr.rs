/// Imports
use crate::{ctxt::check::CheckCtxt, errors::TypeckError, typ::Typ};
use crow_ast::{
    atom::{BinOp, Lit, UnOp},
    expr::{Expr, ExprKind},
};
use crow_lex::token::Span;
use crow_macros::emit;

/// Implementation of expressions inference
impl<'tx> CheckCtxt<'tx> {
    /// Infers literal
    pub fn infer_lit(&mut self, lit: Lit) -> Typ {
        match lit {
            Lit::Int(_) => Typ::Int,
            Lit::Float(_) => Typ::Float,
            Lit::String(_) => Typ::Str,
            Lit::Bool(_) => Typ::Bool,
            Lit::None => Typ::Unit,
        }
    }

    /// Infers unary expression
    pub fn infer_unary(&mut self, span: Span, un_op: UnOp, expr: Expr) -> Typ {
        let typ = self.infer_expr(expr);
        match (un_op, typ) {
            // Number neg operator
            (UnOp::Neg, Typ::Int) => Typ::Int,
            (UnOp::Neg, Typ::Float) => Typ::Float,
            // Bool bang operator
            (UnOp::Bang, Typ::Bool) => Typ::Bool,
            (op, t) => {
                emit!(
                    self,
                    TypeckError::InvalidUnOp {
                        src: span.0,
                        span: span.1.into(),
                        t: self.pretty(&t),
                        op
                    }
                );
                Typ::Error
            }
        }
    }

    /// Infers binary expression
    pub fn infer_binary(&mut self, span: Span, bin_op: BinOp, lhs: Expr, rhs: Expr) -> Typ {
        let lhs = self.infer_expr(lhs);
        let rhs = self.infer_expr(rhs);

        match (bin_op, lhs, rhs) {
            // Int operators
            (BinOp::Add, Typ::Int, Typ::Int)
            | (BinOp::Sub, Typ::Int, Typ::Int)
            | (BinOp::Mul, Typ::Int, Typ::Int)
            | (BinOp::Div, Typ::Int, Typ::Int)
            | (BinOp::Rem, Typ::Int, Typ::Int)
            | (BinOp::Xor, Typ::Int, Typ::Int) => Typ::Int,
            // Float operators
            (BinOp::Add, Typ::Float, Typ::Float)
            | (BinOp::Sub, Typ::Float, Typ::Float)
            | (BinOp::Mul, Typ::Float, Typ::Float)
            | (BinOp::Div, Typ::Float, Typ::Float)
            | (BinOp::Rem, Typ::Float, Typ::Float)
            | (BinOp::Xor, Typ::Float, Typ::Float) => Typ::Int,
            // Logical operators
            (BinOp::And, Typ::Bool, Typ::Bool)
            | (BinOp::Or, Typ::Bool, Typ::Bool)
            | (BinOp::BitAnd, Typ::Bool, Typ::Bool)
            | (BinOp::BitOr, Typ::Bool, Typ::Bool)
            | (BinOp::Xor, Typ::Bool, Typ::Bool) => Typ::Bool,
            // Comparison operators
            (BinOp::Gt, Typ::Int, Typ::Int)
            | (BinOp::Gt, Typ::Int, Typ::Float)
            | (BinOp::Ge, Typ::Int, Typ::Int)
            | (BinOp::Ge, Typ::Int, Typ::Float)
            | (BinOp::Lt, Typ::Int, Typ::Int)
            | (BinOp::Lt, Typ::Int, Typ::Float)
            | (BinOp::Le, Typ::Int, Typ::Int)
            | (BinOp::Le, Typ::Int, Typ::Float) => Typ::Bool,
            // Concat operator
            (BinOp::Concat, Typ::Str, Typ::Str) => Typ::Str,
            // Equality operators
            (BinOp::Eq, a, b) | (BinOp::Ne, a, b) if a == b => Typ::Bool,
            // Other
            (op, a, b) => {
                emit!(
                    self,
                    TypeckError::InvalidBinOp {
                        src: span.0,
                        span: span.1.into(),
                        a: self.pretty(&a),
                        b: self.pretty(&b),
                        op
                    }
                );
                Typ::Error
            }
        }
    }

    /// Infers expression
    pub fn infer_expr(&mut self, expr: Expr) -> Typ {
        let span = expr.span;
        let typ = match expr.kind {
            ExprKind::Lit(lit) => self.infer_lit(lit),
            ExprKind::Unary(expr, un_op) => self.infer_unary(span, un_op, *expr),
            ExprKind::Bin(lhs, rhs, bin_op) => self.infer_binary(span, bin_op, *lhs, *rhs),
            ExprKind::Assign(expr, expr1) => todo!(),
            ExprKind::If(expr, expr1, expr2) => todo!(),
            ExprKind::Var(_) => todo!(),
            ExprKind::Field(expr, _) => todo!(),
            ExprKind::Call(expr, exprs) => todo!(),
            ExprKind::Function(params, expr) => todo!(),
            ExprKind::Match(expr, cases) => todo!(),
            ExprKind::Paren(expr) => todo!(),
            ExprKind::Block(stmts) => todo!(),
            ExprKind::Todo(expr) => todo!(),
            ExprKind::Panic(expr) => todo!(),
        };
        self.apply(typ)
    }
}
