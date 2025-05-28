use unicode_categories::UnicodeCategories as _;
use unscanny::{Pattern, Scanner};

use crate::{node::{SyntaxKind, SyntaxNode}, Range};

use super::link::{self, LinkTokenKind};

macro_rules! leaf {
    ($kind:ident, $range:expr) => {
        SyntaxNode::leaf(SyntaxKind::$kind, $range)
    };
}

pub struct InlineParser<'s> {
    s: Scanner<'s>,
    inner_stack: Vec<Frame>,
    nodes: Vec<SyntaxNode>,
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

impl<'s> InlineParser<'s> {
    pub fn new(text: &'s str) -> Self {
        Self {
            s: Scanner::new(text),
            inner_stack: Vec::new(),
            nodes: Vec::new(),
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
    pub fn next(&mut self) {
        let start = self.s.cursor();
        if self.in_frame(SyntaxKind::Verbatim) {
            // TODO: parse in verbatim mode
            return
        }
        let Some(c) = self.s.eat() else {
            let range = Range::new(start, self.s.cursor());
            self.push(leaf!(Eof, range));
            return
        };
        match c {
            '{' => {
                self.with_frame(SyntaxKind::Destination, |p| {
                    p.push(leaf!(DestinationOpen, Range::new(start, p.s.cursor())));
                    if p.s.at(':') {
                        p.parse_destapplink();
                    } else {
                        let start = p.s.cursor();
                        p.s.eat_until_unescaped(['}']);
                        p.push(leaf!(DestRawlink, Range::new(start, p.s.cursor())));
                    }
                    if p.s.at('}') {
                        p.s.eat();
                        p.push(leaf!(DestinationClose, Range::new(start, p.s.cursor())));
                    }
                });
            }
            ':' => 'left_link_mod: {
                let origin = self.s.cursor();
                if let Some(c) = self.s.eat() {
                    if self.can_open(c) {
                        let kind = AttachedMod::from_char(c).unwrap();
                        self.start_frame(kind.to_syntax());
                        self.push(SyntaxNode::leaf(kind.to_syntax_open(), Range::new(start, self.s.cursor())));
                        break 'left_link_mod;
                    }
                }
                self.s.jump(origin);
                self.push(leaf!(Punctuation, Range::new(start, self.s.cursor())));
            }
            _ if self.can_open(c) => {
                let kind = AttachedMod::from_char(c).unwrap();
                self.start_frame(kind.to_syntax());
                self.push(SyntaxNode::leaf(kind.to_syntax_open(), Range::new(start, self.s.cursor())));
            }
            _ if self.can_close(c) => {
                if self.s.peek().is_some_and(|ch| ch == ':') {
                    self.s.eat();
                }
                let kind = AttachedMod::from_char(c).unwrap();
                self.push(SyntaxNode::leaf(kind.to_syntax_close(), Range::new(start, self.s.cursor())));
                self.close_frame();
            }
            _ if char_is_space(c) => {
                self.s.eat_while(char_is_space);
                if self.s.at(char_is_eol) {
                    self.s.eat();
                    let range = Range::new(start, self.s.cursor());
                    self.push(leaf!(SoftBreak, range));
                } else if self.s.done() {
                    let range = Range::new(start, self.s.cursor());
                    self.push(leaf!(Eof, range));
                } else {
                    let range = Range::new(start, self.s.cursor());
                    self.push(leaf!(Whitespace, range));
                }
            }
            _ if char_is_eol(c) => {
                let range = Range::new(start, self.s.cursor());
                self.push(leaf!(SoftBreak, range));
            }
            // extra punctuation
            _ if char_is_punct(c) => {
                self.push(leaf!(Punctuation, Range::new(start, self.s.cursor())));
            }
            // word
            _ => {
                self.s.eat_while(char_is_word);
                let range = Range::new(start, self.s.cursor());
                self.push(leaf!(Word, range));
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
            && self.s.scout(-2).is_none_or(|ch| !char_is_word(ch) && ch != c)
            && self.s.peek().is_some_and(|ch| !ch.is_whitespace() && ch != c)
    }
    fn can_close(&self, c: char) -> bool {
        let Some(kind) = AttachedMod::from_char(c) else {
            return false;
        };
        self.in_frame(kind.to_syntax())
            && self.s.scout(-2).is_some_and(|ch| !ch.is_whitespace() && ch != c)
            && self.s.peek().is_none_or(|ch| !char_is_word(ch) && ch != c)
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

// list of /inner/ nodes:
//
// Destination => DestApplink | DestScopedLink | DestRawlink
// DestApplink => DestApplinkPrefix (+ DestApplinkWorkspace) (+ DestApplinkPath) (+ DestScopeHeading | DestScopeWikiHeading)
// DestApplinkWorkspace

impl InlineParser<'_> {
    // TODO: replace with `with_frame(&mut self, kind: SyntaxKind)`
    fn start_frame(&mut self, kind: SyntaxKind) {
        self.inner_stack.push(Frame { wrap_kind: kind, children: Vec::new() });
    }
    fn close_frame(&mut self) {
        let Some(frame) = self.inner_stack.pop() else {
            return;
        };
        self.push(frame.to_syntax());
    }
    fn with_frame(&mut self, kind: SyntaxKind, func: impl FnOnce(&mut Self)) {
        self.start_frame(kind);
        func(self);
        self.close_frame();
    }
    /// :$workspace/path/to/file:* heading:? heading
    fn parse_destapplink(&mut self) {
        const SCOPE_DELIMITER: [char; 2] = [':', '}'];
        let start = self.s.cursor();
        let ch = self.s.eat().unwrap();
        debug_assert_eq!(ch, ':');
        self.with_frame(SyntaxKind::DestApplink, |p| {
            p.push(leaf!(DestApplinkPrefix, Range::new(start, p.s.cursor())));
            p.s.eat_whitespace();
            if p.s.peek().is_some_and(|ch| ch == '$') {
                let start = p.s.cursor();
                p.s.eat();
                p.with_frame(SyntaxKind::DestApplinkWorkspace, |p| {
                    p.push(leaf!(
                        DestApplinkWorkspacePrefix,
                        Range::new(start, p.s.cursor())
                    ));
                    let _start = p.s.cursor();
                    p.with_frame(SyntaxKind::DestApplinkWorkspaceName, |p| {
                        while let Some((kind, range)) = link::lex_next(&mut p.s) {
                            match kind {
                                LinkTokenKind::Delimiter | LinkTokenKind::PathSep => {
                                    p.s.jump(range.start);
                                    break;
                                }
                                LinkTokenKind::Text => {
                                    p.push(leaf!(Word, range));
                                }
                                LinkTokenKind::WorkspacePrefix
                                | LinkTokenKind::HeadingPrefix
                                | LinkTokenKind::WikiHeadingPrefix
                                | LinkTokenKind::BackSlash
                                | LinkTokenKind::Escaped
                                | LinkTokenKind::Whitespace => {
                                    p.push(SyntaxNode::error(range, "unexpected token"));
                                }
                            }
                        }
                    });
                });
            }
            p.s.eat_whitespace();
            let start = p.s.cursor();
            if p.s
                .peek()
                .is_some_and(|ch| ch != ':' && ch != '*' && ch != '?')
            {
                p.s.eat_until_unescaped([':', '}']);
                p.push(leaf!(DestApplinkPath, Range::new(start, p.s.cursor())));
            }
            let start = p.s.cursor();
            // parse scopes
            while let Some((kind, range)) = link::lex_next(&mut p.s) {
                match kind {
                    LinkTokenKind::Whitespace => continue,
                    LinkTokenKind::Delimiter => {
                        p.push(leaf!(DestScopeDelimiter, range));
                    }
                    LinkTokenKind::HeadingPrefix => {
                        p.with_frame(SyntaxKind::DestScopeHeading, |p| {
                            p.push(leaf!(DestScopeHeadingPrefix, range));
                            p.s.eat_whitespace();
                            let start = p.s.cursor();
                            p.s.eat_until_unescaped(SCOPE_DELIMITER);
                            p.push(leaf!(DestScopeText, Range::new(start, p.s.cursor())));
                        });
                    }
                    LinkTokenKind::WikiHeadingPrefix => {
                        p.with_frame(SyntaxKind::DestScopeWikiHeading, |p| {
                            p.push(leaf!(DestScopeWikiHeadingPrefix, range));
                            p.s.eat_whitespace();
                            let start = p.s.cursor();
                            p.s.eat_until_unescaped(SCOPE_DELIMITER);
                            p.push(leaf!(DestScopeText, Range::new(start, p.s.cursor())));
                        });
                    },
                    LinkTokenKind::WorkspacePrefix
                    | LinkTokenKind::PathSep
                    | LinkTokenKind::Escaped
                    | LinkTokenKind::BackSlash
                    | LinkTokenKind::Text => {
                        p.s.eat_until_unescaped(SCOPE_DELIMITER);
                        let range = Range::new(start, p.s.cursor());
                        p.push(SyntaxNode::error(range, "unexpected tokens"));
                    },
                }
            }
        });
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

fn char_is_space(c: char) -> bool {
    c == ' '
}

fn char_is_eol(c: char) -> bool {
    c == '\n'
}

fn char_is_punct(c: char) -> bool {
    c.is_punctuation()
}

fn char_is_word(c: char) -> bool {
    !char_is_space(c) && !char_is_eol(c) && !char_is_punct(c)
}
