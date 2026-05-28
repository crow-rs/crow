/// Imports
use crate::Parser;
use crow_ast::atom::{Param, TypeHint};
use crow_lex::token::TokenKind;

/// Atoms parsing implementation
impl<'s> Parser<'s> {
    /// Parses parameters
    pub(crate) fn params(&mut self) -> Vec<Param> {
        self.sep_by(
            TokenKind::Lparen,
            TokenKind::Rparen,
            TokenKind::Comma,
            |p| {
                let start_span = p.peek().span.clone();
                let name = p.expect(TokenKind::Id).lexeme;
                p.expect(TokenKind::Colon);
                let hint = p.type_hint();
                let end_span = p.prev().span.clone();

                Param {
                    span: start_span + end_span,
                    name,
                    hint,
                }
            },
        )
    }

    /// Parses id type hint
    fn id_type_hint(&mut self) -> TypeHint {
        // Bumping id
        let start_span = self.peek().span.clone();
        let id = self.bump().lexeme;

        // If dot presented
        if self.check(TokenKind::Dot) {
            self.bump();

            let name = self.expect(TokenKind::Id).lexeme;
            let end_span = self.prev().span.clone();

            TypeHint::Mod {
                span: start_span + end_span,
                module: id,
                name,
            }
        }
        // If not
        else {
            let end_span = self.prev().span.clone();

            TypeHint::Local {
                span: start_span + end_span,
                name: id,
            }
        }
    }

    /// Parses function type hint
    fn fun_type_hint(&mut self) -> TypeHint {
        // Bumping `fun`
        let start_span = self.peek().span.clone();
        self.bump();

        // Parsing params
        let params = self.sep_by(
            TokenKind::Lparen,
            TokenKind::Rparen,
            TokenKind::Comma,
            |p| p.type_hint(),
        );

        // Parsing return type
        let ret = if self.check(TokenKind::Arrow) {
            self.bump();
            Box::new(self.type_hint())
        } else {
            Box::new(TypeHint::Infer)
        };
        let end_span = self.prev().span.clone();

        TypeHint::Fun {
            span: start_span + end_span,
            params,
            ret,
        }
    }

    /// Parses union type hint
    fn union_type_hint(&mut self) -> TypeHint {
        // Parsing types
        let start_span = self.peek().span.clone();
        let types = self.sep_by(
            TokenKind::Lparen,
            TokenKind::Rparen,
            TokenKind::Colon,
            |p| p.type_hint(),
        );
        let end_span = self.prev().span.clone();

        TypeHint::Union {
            span: start_span + end_span,
            types,
        }
    }

    /// Reference type hint
    fn ref_type_hint(&mut self) -> TypeHint {
        // Bumping `&`
        let start_span = self.peek().span.clone();
        self.bump();
        let end_span = self.prev().span.clone();

        TypeHint::Ref {
            span: start_span + end_span,
            hint: Box::new(self.type_hint()),
        }
    }

    /// Parses type hint
    pub(crate) fn type_hint(&mut self) -> TypeHint {
        if self.check(TokenKind::Fun) {
            self.fun_type_hint()
        } else if self.check(TokenKind::Lparen) {
            self.union_type_hint()
        } else if self.check(TokenKind::Ampersand) {
            self.ref_type_hint()
        } else {
            self.id_type_hint()
        }
    }
}
