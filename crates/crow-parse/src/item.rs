/// Imports
use crate::{Parser, errors::ParseError};
use crow_ast::{
    atom::{Publicity, TypeHint}, item::{
        AdtAlt, AdtRec, AltField, Attribute, Enum, Fun, Item, ItemKind, NativeFun, RecField, Use, UseKind, UsePath, Variant,
    },
};
use crow_common::bail;
use crow_lex::token::TokenKind;

/// Item parsing implementation
impl<'s> Parser<'s> {
    // Parses rec field
    fn rec_field(&mut self) -> RecField {
        let start_span = self.peek().span.clone();
        let name = self.expect(TokenKind::Id).lexeme;
        self.expect(TokenKind::Colon);
        let hint = self.type_hint();
        let end_span = self.prev().span.clone();

        RecField {
            span: start_span + end_span,
            name,
            hint
        }
    }

    // Parses alt field
    fn alt_field(&mut self) -> AltField {
        let start_span = self.peek().span.clone();
        let name = self.expect(TokenKind::Id).lexeme;
        let end_span = self.prev().span.clone();

        AltField {
            span: start_span + end_span,
            name,
        }
    }

    // Parses rec item kind
    fn rec_item_kind(&mut self) -> ItemKind {
        let start_span = self.peek().span.clone();
        // Bumping `rec`
        self.bump();

        // Parsing signature
        let name = self.expect(TokenKind::Id).lexeme;

        // Parsing fields
        let fields = self.sep_by(
            TokenKind::Lbrace,
            TokenKind::Rbrace,
            TokenKind::Comma,
            |p| p.rec_field(),
        );
        let end_span = self.prev().span.clone();

        ItemKind::Rec(AdtRec {
            span: start_span + end_span,
            name,
            fields,
        })
    }

    // Parses alt item kind
    // alt Name = V | A | ...
    fn alt_item_kind(&mut self) -> ItemKind {
        // Bumping `alt`
        self.bump();

        // Parsing signature
        let name = self.expect(TokenKind::Id).lexeme;

        // Parsing fields
        let fields = self.sep_by(
            TokenKind::Eq,
            TokenKind::Dot,
            TokenKind::Bar,
            |p| p.alt_field(),
        );

        ItemKind::Alt(AdtAlt {
            name,
            fields,
        })
    }

    // Parses enum variant
    fn enum_variant(&mut self) -> Variant {
        // Parsing enum variant
        let start_span = self.peek().span.clone();
        let name = self.expect(TokenKind::Id).lexeme;
        let params = if self.check(TokenKind::Lparen) {
            self.sep_by(
                TokenKind::Lparen,
                TokenKind::Rparen,
                TokenKind::Comma,
                |p| p.type_hint(),
            )
        } else {
            Vec::new()
        };
        let end_span = self.prev().span.clone();

        Variant {
            span: start_span + end_span,
            name,
            fields: params,
        }
    }

    // Parses enum item kind
    fn enum_item_kind(&mut self) -> ItemKind {
        // Bumping `enum`
        self.bump();

        // Parsing signature
        let name = self.expect(TokenKind::Id).lexeme;

        // Parsing variants
        let variants = self.sep_by(
            TokenKind::Lbrace,
            TokenKind::Rbrace,
            TokenKind::Comma,
            |p| p.enum_variant(),
        );

        ItemKind::Enum(Enum {
            name,
            variants,
        })
    }

    // Parses function item kind
    fn fun_item_kind(&mut self, is_native: bool) -> ItemKind {
        // Bumping `pure` if specified
        let start_span = self.peek().span.clone();

        //if fn is native - bump 'native'
        if is_native {
            self.bump();
        }

        // Bumping `fun`
        self.expect(TokenKind::Fun);

        // Parsing signature
        let name = self.expect(TokenKind::Id);

        let generics = self.generic_params();
        let params = self.params();
        let effects = self.effects();
        let ret = if self.check(TokenKind::Arrow) {
            self.bump();
            self.type_hint()
        } else {
            if is_native {
                TypeHint::Unit(start_span.clone())
            } else {
                TypeHint::Infer
            }
        };

        if !is_native {
            let block = if self.check(TokenKind::Lbrace) {
                Some(self.block())
            } else {
                None
            };
            let end_span = self.prev().span.clone();
            return ItemKind::Fun(Fun {
                span: start_span + end_span,
                name: name.lexeme,
                generics,
                params,
                ret,
                block,
                effects,
            })
        }

        if !generics.is_empty() {
            bail!(ParseError::GenericNative { name: name.lexeme, src: start_span.0.clone(), span: name.span.1.clone().into() })
        }

        let link_name = if self.check(TokenKind::Eq) {
            self.bump();
            self.expect(TokenKind::String).lexeme
        } else {
            name.lexeme.clone()
        };

        let end_span = self.prev().span.clone();

        ItemKind::Native(NativeFun {
            name: name.lexeme,
            span: start_span + end_span,
            params,
            ret,
            body: link_name
        })
    }

    /// Using path parsing
    fn use_path(&mut self) -> UsePath {
        // Module name string
        let start_span = self.peek().span.clone();
        let module = self
            .sep_by_2(TokenKind::Slash, |p| p.expect(TokenKind::Id).lexeme)
            .join("/");
        let end_span = self.prev().span.clone();

        UsePath {
            span: start_span + end_span,
            module,
        }
    }

    // Using parsing
    pub(crate) fn use_(&mut self) -> Use {
        // Bumping `use`
        let start_span = self.peek().span.clone();
        self.bump();

        // Use path
        let path = self.use_path();

        // Suffix
        let kind = if self.check(TokenKind::As) {
            self.bump();
            let name = self.expect(TokenKind::Id).lexeme;

            UseKind::As(name)
        } else if self.check(TokenKind::For) {
            self.bump();
            let names = self.sep_by_2(TokenKind::Comma, |p| {
                p.expect(TokenKind::Id).lexeme
            });

            UseKind::For(names)
        } else {
            UseKind::Just
        };
        let end_span = self.prev().span.clone();

        Use {
            span: start_span + end_span,
            path,
            kind,
        }
    }

    fn attributes(&mut self) -> Attribute {
        self.bump(); //skip @
        let attr_name = self.expect(TokenKind::Id);

        let args = self.sep_by(TokenKind::Lparen, TokenKind::Rparen, TokenKind::Comma, |p| {
            p.expect(TokenKind::String).lexeme
        });

        Attribute { 
            span: attr_name.span.clone(), 
            name: attr_name.lexeme, 
            args 
        }
    }

   pub(crate) fn item(&mut self, publicity: Publicity) -> Item {
        let start_span = self.peek().span.clone();

        let mut attributes = Vec::new();
        while self.check(TokenKind::AtSign) {
            attributes.push(self.attributes());
        }

        let tk = self.peek().clone();
        let kind = match &tk.kind {
            TokenKind::Rec => self.rec_item_kind(),
            TokenKind::Alt => self.alt_item_kind(),
            TokenKind::Enum => self.enum_item_kind(),
            TokenKind::Fun | TokenKind::Native => self.fun_item_kind(matches!(tk.kind, TokenKind::Native)),
            _ => bail!(ParseError::UnexpectedItemToken {
                got: tk.kind,
                src: self.source.clone(),
                span: tk.span.1.into(),
            }),
        };

        let end_span = self.prev().span.clone();
        Item {
            span: start_span + end_span,
            publicity,
            kind,
            attributes,
        }
    }
}
