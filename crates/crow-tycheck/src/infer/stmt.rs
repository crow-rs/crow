/// Imports
use crate::{ctxt::check::SolveCtxt, typ::Typ};
use crow_ast::{
    atom::TypeHint,
    expr::Expr,
    stmt::{Stmt, StmtKind},
};
use crow_lex::token::Span;

/// Implementation of statements inference
impl<'tx> SolveCtxt<'tx> {
    /// Checks let statement
    pub fn check_let_stmt(&mut self, span: Span, name: String, hint: TypeHint, expr: Expr) {
        // Inferring types
        let expr_typ = self.infer_expr(expr);
        let typ = self.infer_type_hint(hint);

        // Checking types equality
        self.eq(&span, expr_typ, typ.clone());

        // Declaring local variable
        self.resolver.declare_local_def(&name, typ);
    }

    /// Infers statement
    pub fn infer_stmt(&mut self, stmt: Stmt) -> Typ {
        match stmt.kind {
            StmtKind::Let(name, hint, expr) => {
                self.check_let_stmt(stmt.span, name, hint, expr);
                Typ::Unit
            }
            StmtKind::Expr(expr) => self.infer_expr(expr),
        }
    }

    /// Infers block
    pub fn infer_block(&mut self, mut block: Vec<Stmt>) -> Typ {
        // Getting last statement
        let last = block.pop();

        // Iterating over statements
        for stmt in block {
            self.infer_stmt(stmt);
        }

        // Matching last statement
        match last {
            Some(stmt) => self.infer_stmt(stmt),
            None => Typ::Unit,
        }
    }
}
