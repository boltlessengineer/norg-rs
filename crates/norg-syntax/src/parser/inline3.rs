use crate::{node::{SyntaxKind, SyntaxNode}, parser::lexer, Range};

use super::lexer::{Lexer2, Token};

#[derive(Debug)]
struct Frame {
    wrap_kind: SyntaxKind,
    children: Vec<SyntaxNode>,
}

impl Frame {
    fn to_syntax(self) -> SyntaxNode {
        SyntaxNode::inner(self.wrap_kind, self.children)
    }
    fn to_unclosed(self) -> SyntaxNode {
        // insert 'missing close modifier' error node to children and close as SyntaxNode
        SyntaxNode::inner(self.wrap_kind, self.children)
    }
}

#[derive(Debug, PartialEq)]
enum AttachedMod {
    /// *text*
    Bold,
    /// /text/
    Italic,
    /// _text_
    Underline,
    /// ~text~
    Strikethrough,
}

impl AttachedMod {
    fn from_char(c: char) -> Option<Self> {
        match c {
            '*' => Some(Self::Bold),
            '/' => Some(Self::Italic),
            '_' => Some(Self::Underline),
            '~' => Some(Self::Strikethrough),
            _ => None,
        }
    }
    fn to_syntax(&self) -> SyntaxKind {
        match self {
            Self::Bold => SyntaxKind::Bold,
            Self::Italic => SyntaxKind::Italic,
            Self::Underline => SyntaxKind::Underline,
            Self::Strikethrough => SyntaxKind::Strikethrough,
        }
    }
    fn to_syntax_open(&self) -> SyntaxKind {
        match self {
            Self::Bold => SyntaxKind::BoldOpen,
            Self::Italic => SyntaxKind::ItalicOpen,
            Self::Underline => SyntaxKind::UnderlineOpen,
            Self::Strikethrough => SyntaxKind::StrikethroughOpen,
        }
    }
    fn to_syntax_close(&self) -> SyntaxKind {
        match self {
            Self::Bold => SyntaxKind::BoldClose,
            Self::Italic => SyntaxKind::ItalicClose,
            Self::Underline => SyntaxKind::UnderlineClose,
            Self::Strikethrough => SyntaxKind::StrikethroughClose,
        }
    }
}

pub struct InlineParser3<'s> {
    current: Token<SyntaxKind>,
    lexer: Lexer2<'s>,
    inner_stack: Vec<Frame>,
    nodes: Vec<SyntaxNode>,
    last_kind: SyntaxKind,
}

impl<'s> InlineParser3<'s> {
    pub fn new(text: &'s str) -> Self {
        let mut lexer = Lexer2::new(text);
        let current = lexer.lex_();
        Self {
            current,
            lexer,
            inner_stack: Vec::new(),
            nodes: Vec::new(),
            last_kind: SyntaxKind::End,
        }
    }
    pub fn finish(mut self) -> Vec<SyntaxNode> {
        while let Some(frame) = self.inner_stack.pop() {
            self.push(frame.to_unclosed());
        }
        self.nodes
    }
    fn push(&mut self, node: SyntaxNode) {
        if let Some(frame) = self.inner_stack.last_mut() {
            frame.children.push(node);
        } else {
            self.nodes.push(node);
        }
    }
    fn start_frame(&mut self, kind: SyntaxKind) {
        self.inner_stack.push(Frame {
            wrap_kind: kind,
            children: Vec::new(),
        });
    }
    fn close_frame(&mut self) {
        if let Some(frame) = self.inner_stack.pop() {
            if !frame.children.is_empty() {
                self.push(frame.to_syntax());
            }
        };
    }
    fn with_frame<R>(&mut self, kind: SyntaxKind, func: impl FnOnce(&mut Self) -> R) -> R {
        self.start_frame(kind);
        let r = func(self);
        self.close_frame();
        // lex current token again because context has been changed by closing last frame
        // e.g. BoldClose comming right after the ItalicClose
        self.re_lex();
        r
    }
    fn has_frame(&self, kind: SyntaxKind) -> bool {
        self.inner_stack.iter().find(|f| f.wrap_kind == kind).is_some()
    }
    fn in_frame(&self, kind: SyntaxKind) -> bool {
        self.inner_stack.last().is_some_and(|f| f.wrap_kind == kind)
    }
    fn can_open(&self, c: char) -> bool {
        let Some(kind) = AttachedMod::from_char(c) else {
            return false;
        };
        !self.has_frame(kind.to_syntax())
            && self.last_kind != SyntaxKind::Word
            && self.last_kind != SyntaxKind::Special(c)
            && {
                let peek = self.lexer.peek_();
                dbg!(&peek);
                !peek.is_whitespace() && peek.kind != SyntaxKind::Special(c)
            }
    }
    fn can_close(&self, c: char) -> bool {
        let Some(kind) = AttachedMod::from_char(c) else {
            return false;
        };
        // dbg!(c);
        // dbg!(&self.inner_stack);
        // dbg!(self.in_frame(kind.to_syntax()));
        // dbg!(self.last_kind);
        self.in_frame(kind.to_syntax())
            && (self.last_kind != SyntaxKind::Whitespace
                && self.last_kind != SyntaxKind::SoftBreak
                && self.last_kind != SyntaxKind::End)
            && self.last_kind != SyntaxKind::Special(c)
            && {
                let peek = self.lexer.peek_();
                dbg!(peek.kind);
                !peek.is_word() && peek.kind != SyntaxKind::Special(c)
            }
    }
    fn at(&self, kind: SyntaxKind) -> bool {
        self.current.kind == kind
    }
    fn eat(&mut self) {
        self.push(SyntaxNode::leaf(self.current.kind, self.current.range));
        self.last_kind = self.current.kind;
        self.current = self.lex();
    }
    fn eat_as(&mut self, kind: SyntaxKind) {
        self.current.kind = kind;
        self.eat();
    }
    fn assert(&mut self, kind: SyntaxKind) {
        assert_eq!(self.current.kind, kind);
        self.eat();
    }
    fn expect(&mut self, kind: SyntaxKind) {
        if self.at(kind) {
            self.eat();
        } else {
            self.push(SyntaxNode::error(
                self.current.range,
                &format!("missing token. expected {kind:?}, got {:?}", self.current.kind),
            ));
        }
    }
    fn re_lex(&mut self) {
        self.lexer.jump(self.current.range.start);
        self.current = self.lex();
    }
    // lexer context that depends on parser state:
    // - in bold/italic -> parse markup, excluding open, including close
    // - in verbatim -> parse verbatim | VerbatimClose
    // - in destination -> parse destination tokens | destinationClose
    // - in attributes -> parse identifier, text, escaped,
    fn lex(&mut self) -> Token<SyntaxKind> {
        use SyntaxKind::*;
        let t = self.lexer.lex_();
        if self.lexer.mode == lexer::LexMode::Markup {
            // FIXME: add verbatim mode to use on raw link or actual verbatim
            match t.kind {
                Special('\\') => {
                    let next = self.lexer.peek_();
                    match next.kind {
                        Special(ch) => Token {
                            kind: Escaped(ch),
                            range: Range::new(t.range.start, next.range.end),
                        },
                        Word => Token { kind: InlineTagPrefix, range: t.range },
                        // TODO: add hard break
                        _ => t,
                    }
                }
                Special('{') => Token { kind: DestinationOpen, range: t.range },
                Special('}') if self.has_frame(Destination) => Token { kind: DestinationClose, range: t.range },
                Special(':') => {
                    let origin = self.lexer.cursor();
                    let next = self.lexer.lex_();
                    match next.kind {
                        Special(ch) if self.can_open(ch) => {
                            let kind = AttachedMod::from_char(ch).unwrap();
                            Token {
                                kind: kind.to_syntax_open(),
                                range: Range::new(t.range.start, next.range.end),
                            }
                        }
                        _ => {
                            self.lexer.jump(origin);
                            t
                        }
                    }
                }
                Special(ch) if self.can_open(ch) => {
                    let kind = AttachedMod::from_char(ch).unwrap();
                    Token {
                        kind: kind.to_syntax_open(),
                        range: t.range,
                    }
                }
                Special(ch) if self.can_close(ch) => {
                    let kind = AttachedMod::from_char(ch).unwrap();
                    Token {
                        kind: kind.to_syntax_close(),
                        range: t.range,
                    }
                }
                _ => t
            }
        } else {
            t
        }
    }
    pub fn parse(&mut self) {
        self.markups(&[SyntaxKind::End]);
    }
    /// parse multiple markup nodes until stop_set
    fn markups(&mut self, stop_set: &[SyntaxKind]) {
        while !stop_set.contains(&self.current.kind) {
            self.parse_markup_token();
        }
    }
    fn parse_markup_token(&mut self) {
        use SyntaxKind::*;
        match self.current.kind {
            End | Whitespace | SoftBreak | Word | Escaped(_) | Special(_) => self.eat(),
            InlineTagPrefix => todo!("implement inline tag"),
            DestinationOpen => {
                self.with_frame(Destination, |p| {
                    p.assert(DestinationOpen);
                    let peeked = p.current.kind;
                    match peeked {
                        Special(':') => {
                            p.with_frame(DestApplink, |p| {
                                p.lexer.mode = lexer::LexMode::AppLink(true);
                                p.parse_dest_applink();
                                p.lexer.mode = lexer::LexMode::Markup;
                            });
                        }
                        Special('*' | '?') => {
                            // local scoped link
                        }
                        _ => {
                            p.with_frame(DestRawlink, |p| loop {
                                p.lexer.mode = lexer::LexMode::Markup;
                                match p.current.kind {
                                    End | DestinationClose => break,
                                    _ => p.eat(),
                                }
                                p.lexer.mode = lexer::LexMode::Markup;
                            });
                        }
                    }
                    p.expect(DestinationClose);
                });
            }
            BoldOpen => {
                self.with_frame(Bold, |p| {
                    p.assert(BoldOpen);
                    p.markups(&[BoldClose, End]);
                    p.expect(BoldClose);
                });
            }
            ItalicOpen => {
                self.with_frame(Italic, |p| {
                    p.assert(ItalicOpen);
                    p.markups(&[ItalicClose, End]);
                    p.expect(ItalicClose);
                });
                println!("italic closed");
            }
            k => unreachable!("unexepected: {k:?}"),
        }
    }
    fn parse_dest_applink(&mut self) {
        use SyntaxKind::*;
        debug_assert_eq!(self.lexer.mode, lexer::LexMode::AppLink(true));
        loop {
            match self.current.kind {
                DestinationClose | End => return,
                DestScopeDelimiter => {
                    self.eat();
                    self.lexer.mode = lexer::LexMode::AppLink(true);
                },
                DestScopeHeadingPrefix => {
                    self.lexer.mode = lexer::LexMode::AppLink(false);
                    self.with_frame(DestScopeHeading, |p| {
                        p.assert(DestScopeHeadingPrefix);
                        p.with_frame(DestScopeText, |p| loop {
                            match p.current.kind {
                                End | DestinationClose | DestScopeDelimiter => break,
                                _ => p.eat(),
                            }
                        });
                    });
                }
                DestScopeWikiHeadingPrefix => {
                    self.lexer.mode = lexer::LexMode::AppLink(false);
                    self.with_frame(DestScopeWikiHeading, |p| {
                        p.assert(DestScopeWikiHeadingPrefix);
                        p.with_frame(DestScopeText, |p| loop {
                            match p.current.kind {
                                End | DestinationClose | DestScopeDelimiter => break,
                                _ => p.eat(),
                            }
                        });
                    });
                }
                _ => {
                    self.lexer.mode = lexer::LexMode::AppLink(false);
                    self.with_frame(DestApplinkPath, |p| loop {
                        match p.current.kind {
                            End | DestinationClose | DestScopeDelimiter => break,
                            _ => p.eat(),
                        }
                    });
                }
            }
        }
    }
}
