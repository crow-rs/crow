/// Imports
use crate::Parser;
use crow_ast::{
    atom::TypeHint,
    expr::{Expr, ExprKind},
    stmt::{Stmt, StmtKind},
};
use crow_lex::token::TokenKind;

/// Implementation
impl<'s> Parser<'s> {
    /// Let statement parsing
    fn variable_stmt(&mut self, is_mutable: bool) -> Stmt {
        // Bumping `let`
        let start_span = self.peek().span.clone();
        self.bump();
        let name = self.expect(TokenKind::Id).lexeme;

        // Parsing hint
        let hint = if self.check(TokenKind::Colon) {
            self.bump();
            self.type_hint()
        } else {
            TypeHint::Infer
        };

        // Parsing rhs
        self.expect(TokenKind::Eq);
        let expr = self.expr();
        let end_span = self.prev().span.clone();

        Stmt {
            span: start_span + end_span,
            kind: StmtKind::Variable(name, hint, expr, is_mutable),
        }
    }

    /// Expression statement parsing
    fn expr_stmt(&mut self) -> Stmt {
        let expr = self.expr();

        Stmt {
            span: expr.span.clone(),
            kind: StmtKind::Expr(expr),
        }
    }

    fn wildcard_assign(&mut self) -> Stmt {
        // Bumping `_`
        let start_span = self.peek().span.clone();
        self.bump();

        // Parsing rhs
        self.expect(TokenKind::Eq);
        let expr = self.expr();
        let end_span = self.prev().span.clone();

        Stmt {
            span: start_span.clone() + end_span,
            kind: StmtKind::WildcardAssign(TypeHint::Unit(start_span), expr),
        }
    }

    /// Statement parsing
    fn stmt(&mut self) -> Stmt {
        match self.peek().kind {
            TokenKind::Wildcard => self.wildcard_assign(),
            TokenKind::Val => self.variable_stmt(false),
            TokenKind::Var => self.variable_stmt(true),
            _ => self.expr_stmt(),
        }
    }

    /// Block parsing
    pub fn block(&mut self) -> Expr {
        // Parsing statements vector
        let start_span = self.peek().span.clone();
        let mut stmts = Vec::new();
        self.expect(TokenKind::Lbrace);
        while !self.check(TokenKind::Rbrace) {
            stmts.push(self.stmt());
        }
        self.expect(TokenKind::Rbrace);
        let end_span = self.prev().span.clone();

        Expr {
            span: start_span + end_span,
            kind: ExprKind::Block(stmts),
        }
    }
}
