/// Modules
mod atom;
mod errors;
mod expr;
mod item;
mod pat;
mod stmt;

/// Imports
use crate::errors::ParseError;
use crow_ast::{atom::Publicity, item::Module};
use crow_common::bail;
use crow_lex::{
    Lexer,
    token::{Token, TokenKind},
};
use miette::NamedSource;
use std::sync::Arc;

/// Parser is struct that converts a stream of tokens
/// produced by the lexer into an abstract syntax tree (AST).
pub struct Parser<'s> {
    /// Named source of the file
    pub(crate) source: Arc<NamedSource<String>>,

    /// Lexer used to iterate over tokens
    lexer: Lexer<'s>,

    /// Previously consumed token
    /// (useful for spans and error reporting)
    previous: Option<Token>,

    /// Current token under inspection
    pub(crate) current: Option<Token>,

    /// Lookahead token
    /// (used for predictive parsing)
    next: Option<Token>,

    lookahead: Option<Token>,
}

/// Implementation
impl<'s> Parser<'s> {
    /// Creates new parser
    pub fn new(
        source: Arc<NamedSource<String>>,
        mut lexer: Lexer<'s>,
    ) -> Self {
        let current = lexer.next();
        let next = lexer.next();
        Self {
            source,
            lexer,
            previous: None,
            current,
            next,
            lookahead: None,
        }
    }

    /// Parses module
    pub fn parse(&mut self) -> Module {
        let mut uses = Vec::new();
        let mut items = Vec::new();

        // Parsing all the items
        while self.current.is_some() {
            match self.peek().kind {
                // Parsing using
                TokenKind::Use => uses.push(self.use_()),
                // Parsing public or private item
                TokenKind::Pub => {
                    self.expect(TokenKind::Pub);
                    items.push(self.item(Publicity::Pub))
                }
                _ => items.push(self.item(Publicity::Priv)),
            }
        }

        Module {
            source: self.source.clone(),
            uses,
            items,
        }
    }

    /// Parses separated items with open and close tokens
    pub(crate) fn sep_by<T>(
        &mut self,
        open: TokenKind,
        close: TokenKind,
        sep: TokenKind,
        mut parse_item: impl FnMut(&mut Self) -> T,
    ) -> Vec<T> {
        let mut items = Vec::new();
        self.expect(open);

        if !self.check(close) {
            loop {
                items.push(parse_item(self));
                if self.check(sep) {
                    self.expect(sep);
                    if self.check(close) {
                        break;
                    }
                } else {
                    break;
                }
            }
        }

        self.expect(close);
        items
    }

    /// Parses separated items without open and close tokens
    pub(crate) fn sep_by_2<T>(
        &mut self,
        sep: TokenKind,
        mut parse_item: impl FnMut(&mut Self) -> T,
    ) -> Vec<T> {
        let mut items = Vec::new();

        loop {
            items.push(parse_item(self));
            if self.check(sep) {
                self.expect(sep);
            } else {
                break;
            }
        }

        items
    }

    /// Checks, does token match or not
    pub(crate) fn check(&self, tk: TokenKind) -> bool {
        self.current
            .as_ref()
            .map(|x| x.kind == tk)
            .unwrap_or_default()
    }

    /// Retrieves current token
    pub(crate) fn peek(&self) -> &Token {
        match &self.current {
            Some(tk) => tk,
            // Note: previous token is guaranteed `Some`
            None => bail!(ParseError::UnexpectedEof {
                src: self.source.clone(),
                span: self.previous.clone().unwrap().span.1.into(),
            }),
        }
    }
    
    pub(crate) fn peek_next(&self) -> Option<&Token> {
        self.next.as_ref()
    }

    pub(crate) fn peek_nth(&mut self, n: usize) -> Option<&Token> {
        match n {
            0 => self.current.as_ref(),
            1 => self.next.as_ref(),
            2 => {
                if self.lookahead.is_none() {
                    self.lookahead = self.lexer.next();
                }
                self.lookahead.as_ref()
            }
            _ => None,
        }
    }

    /// Retrieves previous token
    pub(crate) fn prev(&self) -> &Token {
        match &self.previous {
            Some(tk) => tk,
            // Note: previous token is guaranteed `Some`
            None => bail!(ParseError::UnexpectedEof {
                src: self.source.clone(),
                span: self.previous.clone().unwrap().span.1.into(),
            }),
        }
    }

    /// Expects token with kind
    pub(crate) fn expect(&mut self, tk: TokenKind) -> Token {
        match &self.current {
            Some(it) => {
                if it.kind == tk {
                    self.bump()
                } else {
                    bail!(ParseError::UnexpectedToken {
                        got: it.kind,
                        expected: tk,
                        src: self.source.clone(),
                        span: it.span.1.clone().into(),
                        prev: self.prev().span.1.clone().into(),
                    })
                }
            }
            // Note: previous token is guaranteed `Some`
            None => bail!(ParseError::UnexpectedEof {
                src: self.source.clone(),
                span: self.previous.clone().unwrap().span.1.into(),
            }),
        }
    }

    /// Advances current token
    pub(crate) fn bump(&mut self) -> Token {
        let prev = self.current.take();
        self.previous = prev.clone();
        self.current = self.next.take();
        self.next = self.lookahead.take().or_else(|| self.lexer.next());
        prev.unwrap()
    }
}
