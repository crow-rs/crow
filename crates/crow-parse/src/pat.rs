/// Imports
use crate::{Parser, errors::ParseError};
use crow_ast::{
    atom::{Lit, Mutability},
    expr::{Expr, ExprKind, Pat, PatKind},
};
use crow_common::bail;
use crow_lex::token::TokenKind;

/// Patterns parsing implementation
impl<'s> Parser<'s> {
    /// Name parsing for enum pattern
    fn enum_pat_name(&mut self) -> Expr {
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

            // Breaking loop
            break;
        }
        result
    }

    /// Enum pattern parsing
    fn enum_pat(&mut self) -> Pat {
        // Parsing enum pattern name
        let start_span = self.prev().span.clone();
        let id = self.enum_pat_name();

        // Checking for unpack postfix
        let kind = if self.check(TokenKind::Lparen) {
            let params = self.sep_by(
                TokenKind::Lparen,
                TokenKind::Rparen,
                TokenKind::Comma,
                |p| p.pat(),
            );
            PatKind::Unpack(id, params)
        } else {
            PatKind::Variant(id)
        };
        let end_span = self.prev().span.clone();

        Pat {
            span: start_span + end_span,
            kind,
        }
    }

    /// Signle pattern parsing
    fn single_pat(&mut self) -> Pat {
        let tk = self.bump();
        match tk.kind {
            // Literal patterns
            TokenKind::String => Pat {
                span: tk.span,
                kind: PatKind::Lit(Lit::String(tk.lexeme)),
            },
            TokenKind::Bool => Pat {
                span: tk.span,
                kind: PatKind::Lit(Lit::Bool(tk.lexeme)),
            },
            TokenKind::Number => Pat {
                span: tk.span,
                kind: PatKind::Lit(if tk.lexeme.contains(".") {
                    Lit::Float(tk.lexeme)
                } else {
                    Lit::Int(tk.lexeme)
                }),
            },
            // Wilcard pattern
            TokenKind::Wildcard => Pat {
                span: tk.span,
                kind: PatKind::Wildcard,
            },
            // Immutable binding pattern
            TokenKind::Val => {
                let binding = self.expect(TokenKind::Id);
                Pat {
                    span: tk.span + binding.span,
                    kind: PatKind::BindTo(Mutability::Immut, tk.lexeme),
                }
            }
            // Mutable binding pattern
            TokenKind::Var => {
                let binding = self.expect(TokenKind::Id);
                Pat {
                    span: tk.span + binding.span,
                    kind: PatKind::BindTo(Mutability::Mut, tk.lexeme),
                }
            }
            // Variant or unpack pattern
            TokenKind::Dot => self.enum_pat(),
            // Otherwise, bailing error
            got => bail!(ParseError::UnexpectedPatToken {
                got,
                src: tk.span.0,
                span: tk.span.1.into()
            }),
        }
    }

    /// Pattern parsing
    pub(crate) fn pat(&mut self) -> Pat {
        // Parsing first pattern
        let start_span = self.peek().span.clone();
        let pat = self.single_pat();

        // Cecking if more patterns presented
        if self.check(TokenKind::Bar) {
            // Parsing patterns
            let mut pats = vec![pat];
            while self.check(TokenKind::Bar) {
                self.bump();
                pats.push(self.single_pat())
            }

            // Done!
            let end_span = self.prev().span.clone();
            Pat {
                span: start_span + end_span,
                kind: PatKind::Or(pats),
            }
        } else {
            pat
        }
    }
}
