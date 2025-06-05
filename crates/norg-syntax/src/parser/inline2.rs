use unscanny::{Pattern, Scanner};

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

trait ScannerExt {
    fn eat_until_unescaped<T>(&mut self, pat: impl Pattern<T>);
}

impl ScannerExt for Scanner<'_> {
    fn eat_until_unescaped<T>(&mut self, mut pat: impl Pattern<T>) {
        while !self.done() && pat.matches(self.after()).is_none() {
            if self.at('\\') {
                self.eat();
            }
            self.eat();
        }
    }
}

pub struct InlineParser2<'s> {
    lexer: Lexer2<'s>,
    inner_stack: Vec<Frame>,
    nodes: Vec<SyntaxNode>,
    last_kind: SyntaxKind,
}

impl<'s> InlineParser2<'s> {
    pub fn new(text: &'s str) -> Self {
        let lexer = Lexer2::new(text);
        Self {
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
                let peek = self.peek_at(1);
                dbg!(&peek);
                !peek.is_whitespace() && peek.kind != SyntaxKind::Special(c)
            }
    }
    fn can_close(&self, c: char) -> bool {
        let Some(kind) = AttachedMod::from_char(c) else {
            return false;
        };
        self.in_frame(kind.to_syntax())
            && (self.last_kind != SyntaxKind::Whitespace
                && self.last_kind != SyntaxKind::SoftBreak
                && self.last_kind != SyntaxKind::End)
            && self.last_kind != SyntaxKind::Special(c)
            && {
                let peek = self.peek_at(1);
                !peek.is_word() && peek.kind != SyntaxKind::Special(c)
            }
    }
    #[doc(alias = "lex")]
    fn peek(&self) -> Token<SyntaxKind> {
        // TODO: switch lexer based on frame context:
        // has_frame(link) -> lex with link lexer, check for link_close
        // in_frame(bold) -> check for bold_close
        self.lexer.peek_()
    }
    fn at(&self, kind: SyntaxKind) -> bool {
        self.peek().kind == kind
    }
    /// `pos` is count of token to skip
    fn peek_at(&self, pos: usize) -> Token<SyntaxKind> {
        let mut lexer = self.lexer.clone();
        for _ in 0..pos {
            lexer.lex_();
        }
        lexer.peek_()
    }
    fn skip(&mut self, token: &Token<SyntaxKind>) {
        self.last_kind = token.kind;
        self.lexer.jump(token.range.end);
    }
    // FIXME: this should not automatically jump the lexer
    // because lexer might already skipped trivia bytes by itself
    fn consume(&mut self, token: Token<SyntaxKind>) {
        self.skip(&token);
        self.push(SyntaxNode::leaf(token.kind, token.range));
    }
    fn consume_as(&mut self, mut token: Token<SyntaxKind>, kind: SyntaxKind) {
        token.kind = kind;
        self.consume(token);
    }
    fn expect_as(&mut self, kind: SyntaxKind, kind_as: SyntaxKind) {
        let t = self.lexer.lex_();
        if t.kind == kind {
            self.consume_as(t, kind_as);
        } else {
            self.push(SyntaxNode::error(t.range, &format!("missing token. got {:?}", t.kind)));
        }
    }
    /// peek markup token (pre-process token to bold_open or bold_close here)
    fn peek_markup(&self) -> Token<SyntaxKind> {
        use SyntaxKind::*;
        let current = self.peek();
        match current.kind {
            Special('\\') => {
                let next = self.peek_at(1);
                match next.kind {
                    Special(ch) => Token { kind: Escaped(ch), range: Range::new(current.range.start, next.range.end) },
                    Word => Token { kind: InlineTagPrefix, range: current.range },
                    // TODO: add hard break
                    _ => todo!(),
                }
            }
            // TODO: preprocess bold_open / bold_close tokens
            _ => current,
        }
    }
    /// consume multiple markup nodes and return last, not consumed token
    fn markups(&mut self, stop_set: &[SyntaxKind]) -> Token<SyntaxKind> {
        loop {
            let current = self.peek_markup();
            // check if markup token is stop_set. (e.g. End) -> return it to show stop reason
            if stop_set.contains(&current.kind) {
                return current
            }
            self.parse_markup_token(current);
        }
    }
    fn parse_markup_token(&mut self, token: Token<SyntaxKind>) {
        use SyntaxKind::*;
        match token.kind {
            End | Whitespace | SoftBreak | Word | Escaped(_) | Special(_) => self.consume(token),
            InlineTagPrefix => todo!("implement inline tag"),
            DestinationOpen => {
                self.with_frame(Destination, |p| {
                    p.consume(token);
                    let peeked = p.peek().kind;
                    match peeked {
                        Special(':') => {
                            // applink
                        }
                        Special('*' | '?') => {
                            // local scoped link
                        }
                        _ => {
                            // raw link
                        }
                    }
                    todo!("expect close token");
                });
            }
            BoldOpen => {
                self.with_frame(Bold, |p| {
                    p.consume(token);
                    let last = p.markups(&[BoldClose, End]);
                    todo!()
                });
            }
            _ => unreachable!(),
        }
    }
    pub fn next(&mut self) {
        use SyntaxKind::*;
        let current = self.peek();
        match current.kind {
            End | Whitespace | Word => self.consume(current),
            SoftBreak => self.consume_as(current, SyntaxKind::SoftBreak),
            Special('\\') if self.peek_at(1).is_punctuation() => {
                let escaped = self.peek_at(1);
                let ch = match escaped.kind {
                    Special(ch) => ch,
                    _ => unreachable!(),
                };
                let range = Range::new(current.range.start, escaped.range.end);
                self.consume(Token {
                    kind: SyntaxKind::Escaped(ch),
                    range,
                });
            }
            Special('{') => {
                self.with_frame(SyntaxKind::Destination, |p| {
                    p.consume_as(current, SyntaxKind::DestinationOpen);
                    if p.at(Special(':')) {
                        p.with_frame(SyntaxKind::DestApplink, |p| {
                            p.lexer.mode = lexer::LexMode::AppLink(true);
                            p.parse_dest_applink();
                            p.lexer.mode = lexer::LexMode::Markup;
                        });
                    } else if p.at(Special('*')) || p.at(Special('?')) {
                        todo!("local link");
                    } else {
                        p.with_frame(SyntaxKind::DestRawlink, |p| loop {
                            let t = p.lexer.lex_();
                            match t.kind {
                                End | Special('}') => break,
                                _ => p.consume(t),
                            }
                        });
                    }
                    // skip whitespaces (this won't be needed if we don't jump the lexer position
                    // from p.consume()
                    loop {
                        let t = p.peek();
                        if t.kind != SyntaxKind::Whitespace && t.kind != SyntaxKind::SoftBreak {
                            break;
                        }
                        p.lexer.jump(t.range.end);
                    }
                    p.expect_as(SyntaxKind::Special('}'), SyntaxKind::DestinationClose);
                });
            }
            // instead of checking from here, preprocess peeked token before the match statement
            // same goes to can_open and can_close
            Special(':') => {
                let start = current.range.start;
                self.skip(&current);
                let t = self.peek();
                match t.kind {
                    Special(ch) if self.can_open(ch) => {
                        let kind = AttachedMod::from_char(ch).unwrap();
                        self.start_frame(kind.to_syntax());
                        self.consume(Token {
                            kind: kind.to_syntax_open(),
                            range: Range::new(start, t.range.end),
                        });
                    }
                    _ => self.consume(current),
                }
            }
            Special(ch) if self.can_open(ch) => {
                let kind = AttachedMod::from_char(ch).unwrap();
                self.start_frame(kind.to_syntax());
                self.consume_as(current, kind.to_syntax_open());
            }
            Special(ch) if self.can_close(ch) => {
                self.skip(&current);
                let mut range = current.range;
                if self.peek().kind == SyntaxKind::Special(':') {
                    range.end = self.lexer.lex_().range.end;
                }
                let kind = AttachedMod::from_char(ch).unwrap();
                self.consume(Token {
                    kind: kind.to_syntax_close(),
                    range,
                });
                self.close_frame();
                self.lexer.jump(range.end);
            }
            Special(_) => self.consume(current),
            _ => unreachable!(),
        }
    }
    fn parse_dest_applink(&mut self) -> Token<SyntaxKind> {
        use SyntaxKind::*;
        debug_assert_eq!(self.lexer.mode, lexer::LexMode::AppLink(true));
        loop {
            let t = self.peek();
            match t.kind {
                DestinationClose | End => return t,
                DestScopeDelimiter => {
                    self.consume(t);
                    self.lexer.mode = lexer::LexMode::AppLink(true);
                },
                DestScopeHeadingPrefix => {
                    self.lexer.mode = lexer::LexMode::AppLink(false);
                    self.with_frame(DestScopeHeading, |p| {
                        p.consume_as(t, DestScopeHeadingPrefix);
                        p.with_frame(DestScopeText, |p| loop {
                            let t = p.peek();
                            match t.kind {
                                End | DestinationClose | DestScopeDelimiter => break,
                                _ => p.consume(t),
                            }
                        });
                    });
                }
                DestScopeWikiHeadingPrefix => {
                    self.lexer.mode = lexer::LexMode::AppLink(false);
                    self.with_frame(DestScopeWikiHeading, |p| {
                        p.consume_as(t, DestScopeWikiHeadingPrefix);
                        p.with_frame(DestScopeText, |p| loop {
                            let t = p.peek();
                            match t.kind {
                                End | DestinationClose | DestScopeDelimiter => break,
                                _ => p.consume(t),
                            }
                        });
                    });
                }
                _ => {
                    self.lexer.mode = lexer::LexMode::AppLink(false);
                    self.with_frame(DestApplinkPath, |p| loop {
                        let t = p.peek();
                        dbg!(t.kind);
                        match t.kind {
                            End | DestinationClose | DestScopeDelimiter => break,
                            _ => p.consume(t),
                        }
                    });
                }
            }
        }
    }
}
