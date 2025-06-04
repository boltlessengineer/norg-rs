use unscanny::{Pattern, Scanner};

use crate::{node::{SyntaxKind, SyntaxNode}, Range};

use super::lexer::{Lexer2, NormalTokenKind, Token};

macro_rules! leaf {
    ($kind:ident, $range:expr) => {
        SyntaxNode::leaf(SyntaxKind::$kind, $range)
    };
}

#[derive(Debug)]
struct Frame {
    // TODO: use this
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
    current: Token<SyntaxKind>,
    inner_stack: Vec<Frame>,
    nodes: Vec<SyntaxNode>,
    dest_mode: bool,
    dest_start: bool,
    last_kind: SyntaxKind,
}

impl<'s> InlineParser2<'s> {
    pub fn new(text: &'s str) -> Self {
        let mut lexer = Lexer2::new(text);
        let current = lexer.eat();
        let current = Token {
            kind: current.kind.to_syntax_kind(),
            range: current.range,
        };
        Self {
            lexer,
            current,
            inner_stack: Vec::new(),
            nodes: Vec::new(),
            dest_mode: false,
            dest_start: true,
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
        if node.is_leaf() {
            self.last_kind = node.kind();
        }
        if let Some(frame) = self.inner_stack.last_mut() {
            frame.children.push(node);
        } else {
            self.nodes.push(node);
        }
    }
    fn current(&self) -> SyntaxKind {
        self.current.kind
    }
    fn at(&self, kind: SyntaxKind) -> bool {
        self.current() == kind
    }
    fn eat(&mut self) {
        self.push(SyntaxNode::leaf(self.current.kind, self.current.range));
        self.current = self.lex();
    }
    fn skip(&mut self) {
        self.current = self.lex();
    }
    fn eat_as(&mut self, kind: SyntaxKind) {
        self.push(SyntaxNode::leaf(kind, self.current.range));
        self.current = self.lex();
    }
    fn expect(&mut self, kind: SyntaxKind) {
        let at = self.at(kind);
        if at {
            self.eat();
        } else {
            self.push(SyntaxNode::error(self.current.range, "missing token"));
            // todo!("push error token")
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
    fn with_frame(&mut self, kind: SyntaxKind, func: impl FnOnce(&mut Self)) {
        self.start_frame(kind);
        func(self);
        self.close_frame();
    }
    pub fn next(&mut self) {
        use SyntaxKind::*;
        match self.current() {
            End | Whitespace | Word => self.eat(),
            SoftBreak => self.eat_as(SyntaxKind::SoftBreak),
            Special('\\') if self.lexer.peek().is_punctuation() => {
                let start = self.current.range.start;
                self.skip();
                self.skip();
                let range = Range::new(start, self.lexer.cursor());
                self.push(SyntaxNode::leaf(SyntaxKind::Escaped('c'), range));
            }
            Special('{') => {
                self.with_frame(SyntaxKind::Destination, |p| {
                    p.eat_as(SyntaxKind::DestinationOpen);
                    if p.at(Special(':')) {
                        p.current.kind = DestApplinkPrefix;
                        p.with_frame(SyntaxKind::DestApplink, |p| {
                            p.dest_mode = true;
                            p.parse_dest_applink();
                            p.dest_mode = false;
                        });
                    } else if p.at(Special('*')) || p.at(Special('?')) {
                        todo!("local link");
                    } else {
                        p.with_frame(SyntaxKind::DestRawlink, |p| loop {
                            match p.current() {
                                Special('}') => {
                                    p.current.kind = DestinationClose;
                                    break;
                                }
                                _ => p.eat(),
                            }
                        });
                    }
                    p.expect(SyntaxKind::DestinationClose);
                });
            }
            Special(':') => {
                let start = self.current.range.start;
                let origin = self.lexer.cursor();
                let t = self.lexer.eat();
                match t.kind {
                    NormalTokenKind::Special(ch) if self.can_open(ch) => {
                        let kind = AttachedMod::from_char(ch).unwrap();
                        self.start_frame(kind.to_syntax());
                        self.push(SyntaxNode::leaf(kind.to_syntax_open(), Range::new(start, self.lexer.cursor())));
                        self.skip();
                    }
                    _ => {
                        self.lexer.jump(origin);
                        self.eat();
                    }
                }
            }
            Special(ch) if self.can_open(ch) => {
                let kind = AttachedMod::from_char(ch).unwrap();
                self.start_frame(kind.to_syntax());
                self.eat_as(kind.to_syntax_open());
            }
            Special(ch) if self.can_close(ch) => {
                let mut range = self.current.range;
                if self.lexer.peek().kind == NormalTokenKind::Special(':') {
                    range.end = self.lexer.eat().range.end;
                }
                let kind = AttachedMod::from_char(ch).unwrap();
                self.push(SyntaxNode::leaf(kind.to_syntax_close(), range));
                self.skip();
                self.close_frame();
            }
            Special(_) => self.eat(),
            _ => unreachable!(),
        }
    }
    fn lex(&mut self) -> Token<SyntaxKind> {
        if self.dest_mode {
            self.lex_dest_applink(self.dest_start)
        } else {
            self.lexer.eat().to_syntax_token()
        }
    }
    fn lex_dest_applink(&mut self, prefix: bool) -> Token<SyntaxKind> {
        use NormalTokenKind::*;
        let token = self.lexer.eat();
        let start = token.range.start;
        let (kind, range) = match token.kind {
            End => (SyntaxKind::End, token.range),
            Special('\\') => match self.lexer.peek().kind {
                Special(ch) => {
                    self.lexer.eat();
                    (
                        SyntaxKind::Escaped(ch),
                        Range::new(start, self.lexer.cursor()),
                    )
                }
                _ => (SyntaxKind::DestScopeText, token.range),
            },
            Special('}') => (SyntaxKind::DestinationClose, token.range),
            Special(':') => (SyntaxKind::DestScopeDelimiter, token.range),
            Whitespace | Newline if prefix => {
                return self.lex_dest_applink(prefix);
            }
            Whitespace | Newline => {
                if matches!(self.lexer.peek().kind, Special(':' | '}') | End) {
                    // skip scope's trailing whitespace
                    return self.lex_dest_applink(prefix);
                } else {
                    (SyntaxKind::Whitespace, token.range)
                }
            }
            Special('*') if prefix => {
                while self.lexer.peek().kind == token.kind {
                    self.lexer.eat();
                }
                let range = Range::new(start, self.lexer.cursor());
                if self.lexer.peek().is_whitespace() {
                    self.lexer.eat();
                    (SyntaxKind::DestScopeHeadingPrefix, range)
                } else {
                    (SyntaxKind::DestScopeText, range)
                }
            }
            Special('?') if prefix => {
                if self.lexer.peek().is_whitespace() {
                    self.lexer.eat();
                    (SyntaxKind::DestScopeWikiHeadingPrefix, token.range)
                } else {
                    (SyntaxKind::DestScopeText, token.range)
                }
            }
            Special(_) | Word => (SyntaxKind::DestScopeText, token.range),
        };
        Token { kind, range }
    }
    fn parse_dest_applink(&mut self) {
        use SyntaxKind::*;
        debug_assert!(self.dest_mode);
        self.expect(DestApplinkPrefix);
        while !matches!(self.current(), DestinationClose | End) {
            // self.current = self.lex_dest_applink(true);
            match self.current() {
                End | DestinationClose => return,
                DestScopeDelimiter => {
                    self.dest_start = true;
                    self.eat();
                },
                DestScopeHeadingPrefix => {
                    self.dest_start = false;
                    self.with_frame(DestScopeHeading, |p| {
                        p.eat_as(DestScopeHeadingPrefix);
                        p.with_frame(DestScopeText, |p| loop {
                            match p.current() {
                                End | DestinationClose | DestScopeDelimiter => break,
                                _ => p.eat(),
                            }
                        });
                    });
                }
                DestScopeWikiHeadingPrefix => {
                    self.dest_start = false;
                    self.with_frame(DestScopeWikiHeading, |p| {
                        p.eat_as(DestScopeWikiHeadingPrefix);
                        p.with_frame(DestScopeText, |p| loop {
                            match p.current() {
                                End | DestinationClose | DestScopeDelimiter => break,
                                _ => p.eat(),
                            }
                        });
                    });
                }
                _ => {
                    self.dest_start = false;
                    self.with_frame(DestApplinkPath, |p| loop {
                        match p.current() {
                            End | DestinationClose | DestScopeDelimiter => break,
                            _ => p.eat(),
                        }
                    });
                }
            }
        }
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
                let peek = self.lexer.peek();
                !peek.is_whitespace() && peek.kind != NormalTokenKind::Special(c)
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
                let peek = self.lexer.peek();
                !peek.is_word() && peek.kind != NormalTokenKind::Special(c)
            }
    }
}
