/// Imports
use crate::Parser;
use crow_ast::atom::{EffectHint, Effects, Param, TypeHint};
use crow_lex::token::TokenKind;

/// Atoms parsing implementation
impl<'s> Parser<'s> {
    /// Parses generic args
    fn generic_args(&mut self) -> Vec<TypeHint> {
        if self.check(TokenKind::Lbracket) {
            self.sep_by(
                TokenKind::Lbracket,
                TokenKind::Rbracket,
                TokenKind::Comma,
                |p| p.type_hint(),
            )
        } else {
            Vec::new()
        }
    }

    /// Parses generic params
    pub(crate) fn generic_params(&mut self) -> Vec<String> {
        if self.check(TokenKind::Lbracket) {
            self.sep_by(
                TokenKind::Lbracket,
                TokenKind::Rbracket,
                TokenKind::Comma,
                |p| p.expect(TokenKind::Id).lexeme,
            )
        } else {
            Vec::new()
        }
    }

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
            let args = self.generic_args();
            let end_span = self.prev().span.clone();

            TypeHint::Mod {
                span: start_span + end_span,
                module: id,
                name,
                args,
            }
        }
        // If not
        else {
            let args = self.generic_args();
            let end_span = self.prev().span.clone();
            TypeHint::Local {
                span: start_span + end_span,
                name: id,
                args,
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

        // Parsing function effects
        let effects = self.effects();

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
            effects,
        }
    }

    /// Parses type hint
    pub(crate) fn type_hint(&mut self) -> TypeHint {
        if self.check(TokenKind::Fun) {
            self.fun_type_hint()
        } else {
            self.id_type_hint()
        }
    }

    /// Parses effect hint
    /// todo: supports generic effect
    pub(crate) fn effect_hint(&mut self) -> EffectHint {
        let name = self.expect(TokenKind::Id);
        EffectHint {
            span: name.span,
            name: name.lexeme,
        }
    }

    /// Parses function effects
    pub(crate) fn effects(&mut self) -> Effects {
        // Preparing effects
        let mut effects = Effects {
            known: Vec::new(),
            tail: None,
        };

        // If effects presented
        if self.check(TokenKind::Colon) {
            self.bump();
            effects.known =
                self.sep_by_2(TokenKind::Comma, |p| p.effect_hint());
        }

        effects
    }
}
