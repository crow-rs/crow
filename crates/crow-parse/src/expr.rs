/// Imports
use crate::{Parser, errors::ParseError};
use crow_ast::{
    atom::{BinOp, Lit, UnOp},
    expr::{Case, Expr, ExprKind},
};
use crow_lex::token::TokenKind;
use crow_macros::bail;

/// Exprs parsing implementation
impl<'s> Parser<'s> {
    /// Group `( expr )` expression parsing
    fn group(&mut self) -> Expr {
        let start_span = self.peek().span.clone();
        self.expect(TokenKind::Lparen);
        let expr = self.expr();
        self.expect(TokenKind::Rparen);
        let end_span = self.prev().span.clone();

        Expr {
            span: start_span + end_span,
            kind: ExprKind::Paren(Box::new(expr)),
        }
    }

    /// Variable parsing
    fn variable_expr(&mut self) -> Expr {
        // Parsing base identifier
        let start_span = self.peek().span.clone();
        let id = self.expect(TokenKind::Id).lexeme;

        // Result node
        let mut result = Expr {
            span: start_span.clone(),
            kind: ExprKind::Var(id),
        };

        // Checking for dots and parens
        loop {
            // Checking for chain `a.b.c.d`
            if self.check(TokenKind::Dot) {
                self.bump();

                let id = self.expect(TokenKind::Id).lexeme;
                let end_span = self.prev().span.clone();

                result = Expr {
                    span: start_span.clone() + end_span,
                    kind: ExprKind::Field(Box::new(result), id),
                };
                continue;
            }

            // Checking for start of call with `(`
            if self.check(TokenKind::Lparen) {
                let args = self.sep_by(
                    TokenKind::Lparen,
                    TokenKind::Rparen,
                    TokenKind::Comma,
                    |p| p.expr(),
                );
                let end_span = self.prev().span.clone();

                result = Expr {
                    span: start_span.clone() + end_span,
                    kind: ExprKind::Call(Box::new(result), args),
                };
                continue;
            }

            // Breaking loop
            break;
        }
        result
    }

    /// If expression parsing
    fn if_expr(&mut self) -> Expr {
        // Bumping `if`
        let start_span = self.peek().span.clone();
        self.bump();

        // Parsing if block
        let expr = self.expr();
        let block = self.block();

        // Parsing else branch, if exists
        if self.check(TokenKind::Else) {
            self.bump();

            let branch = if self.check(TokenKind::If) {
                self.if_expr()
            } else {
                self.block()
            };

            let end_span = self.prev().span.clone();
            Expr {
                span: start_span + end_span,
                kind: ExprKind::If(Box::new(expr), Box::new(block), Some(Box::new(branch))),
            }
        } else {
            let end_span = self.prev().span.clone();
            Expr {
                span: start_span + end_span,
                kind: ExprKind::If(Box::new(expr), Box::new(block), None),
            }
        }
    }

    /// Function expression parsing
    fn fun_expr(&mut self) -> Expr {
        // Bumping `fun`
        let start_span = self.peek().span.clone();
        self.bump();

        // Collecting params
        let params = self.params();

        // Parsing block or expr body
        let body = if self.check(TokenKind::Lbrace) {
            self.block()
        } else {
            self.expr()
        };
        let end_span = self.prev().span.clone();

        Expr {
            span: start_span + end_span,
            kind: ExprKind::Function(params, Box::new(body)),
        }
    }

    /// Case parsing
    fn case(&mut self) -> Case {
        // Patterns of the case
        let start_span = self.peek().span.clone();
        let pats = self.sep_by_2(TokenKind::Comma, |p| p.pat());

        // -> { body, ... }
        self.expect(TokenKind::Colon);
        let body = if self.check(TokenKind::Lbrace) {
            self.block()
        } else {
            self.expr()
        };
        let end_span = self.prev().span.clone();

        Case {
            span: start_span + end_span,
            pats,
            body,
        }
    }

    /// Match expression parsing
    fn match_expr(&mut self) -> Expr {
        // Bumping `match`
        let start_span = self.peek().span.clone();
        self.bump();
        let value = self.expr();

        // Parsing cases
        let cases = self.sep_by(
            TokenKind::Lbrace,
            TokenKind::Rbrace,
            TokenKind::Comma,
            |p| p.case(),
        );
        let end_span = self.prev().span.clone();

        Expr {
            span: start_span + end_span,
            kind: ExprKind::Match(Box::new(value), cases),
        }
    }

    /// Panic expression parsing
    fn panic_expr(&mut self) -> Expr {
        // Bumping `panic`
        let start_span = self.peek().span.clone();
        self.bump();

        // If `as` presented, parsing panic text
        let text = if self.check(TokenKind::As) {
            self.bump();
            let text = Box::new(self.expr());
            Some(text)
        } else {
            None
        };
        let end_span = self.prev().span.clone();

        Expr {
            span: start_span + end_span,
            kind: ExprKind::Panic(text),
        }
    }

    /// Todo expression parsing
    fn todo_expr(&mut self) -> Expr {
        // Bumping `todo`
        let start_span = self.peek().span.clone();
        self.bump();

        // If `as` presented, parsing todo text
        let text = if self.check(TokenKind::As) {
            self.bump();
            let text = Box::new(self.expr());
            Some(text)
        } else {
            None
        };
        let end_span = self.prev().span.clone();

        Expr {
            span: start_span + end_span,
            kind: ExprKind::Todo(text),
        }
    }

    /// Atom expression parsing
    fn atom_expr(&mut self) -> Expr {
        let tk = self.peek().clone();
        match tk.kind {
            // Literals parsing
            TokenKind::Lparen => self.group(),
            TokenKind::Number => {
                self.bump();
                Expr {
                    span: tk.span,
                    kind: if tk.lexeme.contains(".") {
                        ExprKind::Lit(Lit::Float(tk.lexeme))
                    } else {
                        ExprKind::Lit(Lit::Int(tk.lexeme))
                    },
                }
            }
            TokenKind::String => {
                self.bump();
                Expr {
                    span: tk.span,
                    kind: ExprKind::Lit(Lit::String(tk.lexeme)),
                }
            }
            TokenKind::Bool => {
                self.bump();
                Expr {
                    span: tk.span,
                    kind: ExprKind::Lit(Lit::Bool(tk.lexeme)),
                }
            }
            TokenKind::None => {
                self.bump();
                Expr {
                    span: tk.span,
                    kind: ExprKind::Lit(Lit::None),
                }
            }
            // Variable parsing
            TokenKind::Id => self.variable_expr(),
            // Branch expressions parsing
            TokenKind::If => self.if_expr(),
            TokenKind::Match => self.match_expr(),
            // Function parsing
            TokenKind::Fun => self.fun_expr(),
            // Todo and panic parsing
            TokenKind::Todo => self.todo_expr(),
            TokenKind::Panic => self.panic_expr(),
            // Otherwise, raising error
            _ => bail!(ParseError::UnexpectedExprToken {
                got: tk.kind,
                src: self.source.clone(),
                span: tk.span.1.into(),
            }),
        }
    }

    /// Unary expression parsing
    fn unary_expr(&mut self) -> Expr {
        if self.check(TokenKind::Minus) || self.check(TokenKind::Bang) {
            let start_span = self.peek().span.clone();

            let op = match self.bump().kind {
                TokenKind::Minus => UnOp::Neg,
                TokenKind::Bang => UnOp::Bang,
                _ => unreachable!(),
            };

            let value = self.unary_expr();
            let end_span = self.prev().span.clone();

            return Expr {
                span: start_span + end_span,
                kind: ExprKind::Unary(Box::new(value), op),
            };
        }

        self.atom_expr()
    }

    /// Factor expression parsing
    fn factor_expr(&mut self) -> Expr {
        let start_span = self.peek().span.clone();
        let mut left = self.unary_expr();

        while self.check(TokenKind::Star)
            || self.check(TokenKind::Slash)
            || self.check(TokenKind::Percent)
        {
            let op = match self.bump().kind {
                TokenKind::Star => BinOp::Mul,
                TokenKind::Slash => BinOp::Div,
                TokenKind::Percent => BinOp::Mod,
                _ => unreachable!(),
            };

            let right = self.unary_expr();
            let end_span = self.prev().span.clone();

            left = Expr {
                span: start_span.clone() + end_span,
                kind: ExprKind::Bin(Box::new(left), Box::new(right), op),
            };
        }

        left
    }

    /// Term expression parsing
    fn term_expr(&mut self) -> Expr {
        let start_span = self.peek().span.clone();
        let mut left = self.factor_expr();

        while self.check(TokenKind::Plus) || self.check(TokenKind::Minus) {
            let op = match self.bump().kind {
                TokenKind::Plus => BinOp::Add,
                TokenKind::Minus => BinOp::Sub,
                _ => unreachable!(),
            };

            let right = self.factor_expr();
            let end_span = self.prev().span.clone();

            left = Expr {
                span: start_span.clone() + end_span,
                kind: ExprKind::Bin(Box::new(left), Box::new(right), op),
            };
        }

        left
    }

    /// Compare expression parsing
    fn compare_expr(&mut self) -> Expr {
        let start_span = self.peek().span.clone();
        let mut left = self.term_expr();

        while self.check(TokenKind::Ge)
            || self.check(TokenKind::Gt)
            || self.check(TokenKind::Le)
            || self.check(TokenKind::Lt)
        {
            let op = match self.bump().kind {
                TokenKind::Ge => BinOp::Ge,
                TokenKind::Gt => BinOp::Gt,
                TokenKind::Le => BinOp::Le,
                TokenKind::Lt => BinOp::Lt,
                _ => unreachable!(),
            };

            let right = self.factor_expr();
            let end_span = self.prev().span.clone();

            left = Expr {
                span: start_span.clone() + end_span,
                kind: ExprKind::Bin(Box::new(left), Box::new(right), op),
            };
        }

        left
    }

    /// Equality expression parsing
    fn equality_expr(&mut self) -> Expr {
        let start_span = self.peek().span.clone();
        let mut left = self.compare_expr();

        while self.check(TokenKind::DoubleEq) || self.check(TokenKind::BangEq) {
            let op = match self.bump().kind {
                TokenKind::DoubleEq => BinOp::Eq,
                TokenKind::BangEq => BinOp::Ne,
                _ => unreachable!(),
            };

            let right = self.compare_expr();
            let end_span = self.prev().span.clone();

            left = Expr {
                span: start_span.clone() + end_span,
                kind: ExprKind::Bin(Box::new(left), Box::new(right), op),
            };
        }

        left
    }

    /// `bitwise and` expression parsing
    fn bit_and_expr(&mut self) -> Expr {
        let start_span = self.peek().span.clone();
        let mut left = self.equality_expr();

        while self.check(TokenKind::Ampersand) {
            self.bump();

            let right = self.equality_expr();
            let end_span = self.prev().span.clone();

            left = Expr {
                span: start_span.clone() + end_span,
                kind: ExprKind::Bin(Box::new(left), Box::new(right), BinOp::BitAnd),
            };
        }

        left
    }

    /// `bitwise xor` expression parsing
    fn bit_xor_expr(&mut self) -> Expr {
        let start_span = self.peek().span.clone();
        let mut left = self.bit_and_expr();

        while self.check(TokenKind::Caret) {
            self.bump();

            let right = self.bit_and_expr();
            let end_span = self.prev().span.clone();

            left = Expr {
                span: start_span.clone() + end_span,
                kind: ExprKind::Bin(Box::new(left), Box::new(right), BinOp::Xor),
            };
        }

        left
    }

    /// `bitwise or` expression parsing
    fn bit_or_expr(&mut self) -> Expr {
        let start_span = self.peek().span.clone();
        let mut left = self.bit_xor_expr();

        while self.check(TokenKind::Bar) {
            self.bump();

            let right = self.bit_xor_expr();
            let end_span = self.prev().span.clone();

            left = Expr {
                span: start_span.clone() + end_span,
                kind: ExprKind::Bin(Box::new(left), Box::new(right), BinOp::BitOr),
            };
        }

        left
    }

    /// `Logical and` expression parsing
    fn logical_and_expr(&mut self) -> Expr {
        let start_span = self.peek().span.clone();
        let mut left = self.bit_or_expr();

        while self.check(TokenKind::DoubleAmp) {
            self.bump();

            let right = self.bit_or_expr();
            let end_span = self.prev().span.clone();

            left = Expr {
                span: start_span.clone() + end_span,
                kind: ExprKind::Bin(Box::new(left), Box::new(right), BinOp::And),
            };
        }

        left
    }

    /// `Logical or` expression parsing
    fn logical_or_expr(&mut self) -> Expr {
        let start_span = self.peek().span.clone();
        let mut left = self.logical_and_expr();

        while self.check(TokenKind::DoubleBar) {
            self.bump();

            let right = self.logical_and_expr();
            let end_span = self.prev().span.clone();

            left = Expr {
                span: start_span.clone() + end_span,
                kind: ExprKind::Bin(Box::new(left), Box::new(right), BinOp::Or),
            };
        }

        left
    }

    /// `Assign` expression parsing
    fn assign_expr(&mut self) -> Expr {
        let start_span = self.peek().span.clone();
        let mut left = self.logical_or_expr();

        while self.check(TokenKind::Eq) {
            self.bump();

            let right = self.logical_or_expr();
            let end_span = self.prev().span.clone();
            left = Expr {
                span: start_span.clone() + end_span,
                kind: ExprKind::Assign(Box::new(left), Box::new(right)),
            };
        }

        left
    }

    /// Parses expression
    pub fn expr(&mut self) -> Expr {
        self.assign_expr()
    }
}
