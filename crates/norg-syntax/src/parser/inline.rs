use unicode_categories::UnicodeCategories;
use unscanny::Scanner;

use crate::{node::{SyntaxKind, SyntaxNode}, Range};

pub struct InlineParser<'s> {
    i_lexer: InlineLexer<'s>,
    markup_stack: Vec<Frame>,
    nodes: Vec<SyntaxNode>,
}

struct Frame {
    wrap_kind: InlineMarkup,
    children: Vec<SyntaxNode>,
}

impl Frame {
    fn to_syntax(self) -> SyntaxNode {
        SyntaxNode::inner(self.wrap_kind.to_syntax(), self.children)
    }
}

fn open_marker(c: char) -> Option<InlineMarkup> {
    match c {
        '*' => Some(InlineMarkup::Bold),
        '/' => Some(InlineMarkup::Italic),
        '_' => Some(InlineMarkup::Underline),
        '~' => Some(InlineMarkup::Strikethrough),
        '[' => Some(InlineMarkup::Markup),
        _ => None,
    }
}

fn close_marker(c: char) -> Option<InlineMarkup> {
    match c {
        '*' => Some(InlineMarkup::Bold),
        '/' => Some(InlineMarkup::Italic),
        '_' => Some(InlineMarkup::Underline),
        '~' => Some(InlineMarkup::Strikethrough),
        ']' => Some(InlineMarkup::Markup),
        _ => None,
    }
}

impl<'s> InlineParser<'s> {
    pub fn new(text: &'s str) -> Self {
        Self {
            i_lexer: InlineLexer::new(text),
            markup_stack: Vec::new(),
            nodes: Vec::new(),
        }
    }

    pub fn parse(mut self) -> Vec<SyntaxNode> {
        while self.parse_next() {
        }
        while let Some(frame) = self.markup_stack.pop() {
            self.push(frame.to_syntax());
        }
        self.nodes
    }
    fn push(&mut self, node: SyntaxNode) {
        if let Some(frame) = self.markup_stack.last_mut() {
            frame.children.push(node);
        } else {
            self.nodes.push(node);
        }
    }
    fn parse_next(&mut self) -> bool {
        let t = self.i_lexer.next();
        let start = t.range.start;
        match t.kind {
            InlineTokenKind::Word => self.push(SyntaxNode::leaf(SyntaxKind::Word, t.range)),
            InlineTokenKind::Whitespace => self.push(SyntaxNode::leaf(SyntaxKind::Whitespace, t.range)),
            InlineTokenKind::Newline => self.push(SyntaxNode::leaf(SyntaxKind::SoftBreak, t.range)),
            InlineTokenKind::Eof => { return false },
            InlineTokenKind::Punct(c) => {
                if let Some(markup) = close_marker(c) {
                    // found '*' which is previously opened attached_modifier
                    println!("might be closing modifier {c}");
                    if self.can_close(&markup) {
                        println!("find closing modifier");
                        // markup (`[this]`) can skip all tests below
                        if markup != InlineMarkup::Markup {
                            let current = self.i_lexer.current();
                            // ...**
                            //    ^
                            if current.kind == t.kind {
                                self.push(SyntaxNode::leaf(SyntaxKind::Punctuation, Range::new(start, self.i_lexer.cursor())));
                                return true
                            }
                            // ...*word
                            if current.kind == InlineTokenKind::Word {
                                self.push(SyntaxNode::leaf(SyntaxKind::Punctuation, Range::new(start, self.i_lexer.cursor())));
                                return true
                            }
                            // ...*:...
                            if current.kind == InlineTokenKind::Punct(':') {
                                self.i_lexer.next();
                            }
                        }
                        let close = SyntaxNode::leaf(markup.to_syntax_close(), Range::new(start, self.i_lexer.cursor()));
                        let mut last = self.markup_stack.pop().unwrap();
                        last.children.push(close);
                        self.push(last.to_syntax());
                        return true
                    }
                }
                if let Some(markup) = open_marker(c).or_else(|| {
                    // consume ':' and check again for open marker
                    if c == ':' {
                        match self.i_lexer.current().kind {
                            InlineTokenKind::Punct(c) => {
                                let marker = open_marker(c)?;
                                // ignore pattern ":["
                                if marker == InlineMarkup::Markup {
                                    return None;
                                }
                                self.i_lexer.next();
                                return Some(marker);
                            }
                            _ => {}
                        }
                    }
                    None
                }) {
                    println!("might be opening modifier {c}");
                    // found '*' which is previously opened attached_modifier
                    if self.can_open(&markup) {
                        println!("find opening modifier {c}");
                        if markup != InlineMarkup::Markup {
                            let current = self.i_lexer.current();
                            // ...**
                            //    ^
                            if current.kind == t.kind {
                                self.push(SyntaxNode::leaf(SyntaxKind::Punctuation, Range::new(start, self.i_lexer.cursor())));
                                return true;
                            }
                            // ...*( )
                            if current.kind.is_whitespace() {
                                self.push(SyntaxNode::leaf(SyntaxKind::Punctuation, Range::new(start, self.i_lexer.cursor())));
                                return true
                            }
                        }
                        println!("open");
                        let open = SyntaxNode::leaf(markup.to_syntax_open(), Range::new(start, self.i_lexer.cursor()));
                        self.markup_stack.push(Frame { wrap_kind: markup, children: vec![open] });
                        return true;
                    }
                }
                self.push(SyntaxNode::leaf(SyntaxKind::Punctuation, Range::new(start, self.i_lexer.cursor())));
            },
        }
        return true;
    }
    fn can_close(&self, markup: &InlineMarkup) -> bool {
        // check for previous token kind
        self.markup_stack.last().is_some_and(|f| f.wrap_kind == *markup)
            && (*markup == InlineMarkup::Markup
                || (!self.i_lexer.prev_kind.is_whitespace()
                    && self.i_lexer.prev_kind != markup.to_token_kind()))
    }
    fn can_open(&self, markup: &InlineMarkup) -> bool {
        // check for previous token kind
        *markup == InlineMarkup::Markup
            || (!self.i_lexer.prev_kind.is_word()
                && self.i_lexer.prev_kind != markup.to_token_kind())
    }
}

#[derive(Debug, PartialEq)]
enum InlineMarkup {
    /// *text*
    Bold,
    /// /text/
    Italic,
    /// _text_
    Underline,
    /// ~text~
    Strikethrough,
    /// [text]
    Markup,
}

impl InlineMarkup {
    fn to_token_kind(&self) -> InlineTokenKind {
        match self {
            Self::Bold => InlineTokenKind::Punct('*'),
            Self::Italic => InlineTokenKind::Punct('/'),
            Self::Underline => InlineTokenKind::Punct('_'),
            Self::Strikethrough => InlineTokenKind::Punct('~'),
            Self::Markup => InlineTokenKind::Punct('['),
        }
    }
    fn to_syntax(&self) -> SyntaxKind {
        match self {
            Self::Bold => SyntaxKind::Bold,
            Self::Italic => SyntaxKind::Italic,
            Self::Underline => SyntaxKind::Underline,
            Self::Strikethrough => SyntaxKind::Strikethrough,
            Self::Markup => SyntaxKind::Markup,
        }
    }
    fn to_syntax_open(&self) -> SyntaxKind {
        match self {
            Self::Bold => SyntaxKind::BoldOpen,
            Self::Italic => SyntaxKind::ItalicOpen,
            Self::Underline => SyntaxKind::UnderlineOpen,
            Self::Strikethrough => SyntaxKind::StrikethroughOpen,
            Self::Markup => SyntaxKind::MarkupOpen,
        }
    }
    fn to_syntax_close(&self) -> SyntaxKind {
        match self {
            Self::Bold => SyntaxKind::BoldClose,
            Self::Italic => SyntaxKind::ItalicClose,
            Self::Underline => SyntaxKind::UnderlineClose,
            Self::Strikethrough => SyntaxKind::StrikethroughClose,
            Self::Markup => SyntaxKind::MarkupClose,
        }
    }
}

const SPACE: char = ' ';
const NEWLINE: char = '\n';

// TODO: do I need an Inline lexer..?
// I think I can just use raw Scanner instead in inline mode.
pub struct InlineLexer<'s> {
    s: Scanner<'s>,
    prev_kind: InlineTokenKind,
    current_kind: InlineTokenKind,
}

impl<'s> InlineLexer<'s> {
    pub fn new(text: &'s str) -> Self {
        Self {
            s: Scanner::new(text),
            prev_kind: InlineTokenKind::Eof,
            current_kind: InlineTokenKind::Eof,
        }
    }
    fn cursor(&self) -> usize {
        self.s.cursor()
    }
    fn current(&mut self) -> InlineToken {
        let origin = self.s.cursor();
        let token = self._next();
        self.s.jump(origin);
        token
    }
    fn next(&mut self) -> InlineToken {
        let t = self._next();
        self.prev_kind = self.current_kind;
        self.current_kind = t.kind;
        t
    }
    fn _next(&mut self) -> InlineToken {
        let start = self.s.cursor();
        let Some(c) = self.s.eat() else {
            let range = Range::new(start, self.s.cursor());
            return InlineToken { kind: InlineTokenKind::Eof, range };
        };
        let kind = if c == SPACE {
            InlineTokenKind::Whitespace
        } else if c == NEWLINE {
            InlineTokenKind::Newline
        } else if c.is_punctuation() {
            InlineTokenKind::Punct(c)
        } else {
            self.s.eat_while(|c: char| !c.is_whitespace() && !c.is_punctuation());
            InlineTokenKind::Word
        };
        let range = Range::new(start, self.s.cursor());
        InlineToken { kind, range }
    }
}

#[derive(Debug, PartialEq)]
struct InlineToken {
    kind: InlineTokenKind,
    range: Range,
}
#[derive(Debug, Copy, Clone, PartialEq)]
enum InlineTokenKind {
    Word,
    Whitespace,
    Punct(char),
    Newline,
    Eof,
}
impl InlineTokenKind {
    fn is_whitespace(&self) -> bool {
        matches!(self, Self::Whitespace | Self::Newline | Self::Eof)
    }
    fn is_word(&self) -> bool {
        *self == Self::Word
    }
}
