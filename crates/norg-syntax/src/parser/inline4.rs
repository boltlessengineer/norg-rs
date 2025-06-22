use unscanny::Scanner;

use crate::{node::{SyntaxKind, SyntaxNode}, Range};

use super::lexer2::*;

#[derive(Clone, Debug)]
enum Peeked {
    Token(SToken),
    ParagraphBreak(SToken),
}
impl Peeked {
    fn token(&self) -> &SToken {
        match self {
            Self::Token(t) |
            Self::ParagraphBreak(t) => t
        }
    }
}

#[derive(Clone)]
pub struct InlineParser<'s> {
    s: Scanner<'s>,
    // TODO: `peeked` is missleading. rename this to `leftover`
    // TODO: No, this is basically a `current` token. refactor it to be a current token
    peeked: Option<Peeked>,
}
impl<'s> InlineParser<'s> {
    pub fn new(text: &'s str) -> Self {
        Self {
            s: Scanner::new(text),
            peeked: None,
        }
    }
    /// lex with given inline lexer
    /// will return peeked token if there are any
    fn lex_inline(
        &mut self,
        lexer: impl FnOnce(&mut Scanner) -> Result<SToken, ParagraphBreak>,
    ) -> Option<SToken> {
        // TODO: maybe just move paragraph break lexer to here
        let peeked = self.peeked.take();
        match peeked {
            Some(Peeked::Token(t)) => Some(t),
            Some(Peeked::ParagraphBreak(_)) => {
                self.peeked = peeked;
                None
            },
            None => match lexer(&mut self.s) {
                Ok(t) => Some(t),
                Err(pb) => {
                    self.peeked = Some(Peeked::ParagraphBreak(pb.0));
                    None
                }
            }
        }
    }
    fn peek_inline(
        &mut self,
        lexer: impl FnOnce(&mut Scanner) -> Result<SToken, ParagraphBreak>,
    ) -> Option<&SToken> {
        let t = self.lex_inline(lexer);
        match t {
            Some(t) => {
                self.peeked = Some(Peeked::Token(t));
                self.peeked.as_ref().map(|p| p.token())
            }
            None => None
        }
    }
    pub fn parse_paragraph(&mut self) -> (Vec<SyntaxNode>, Option<ParagraphBreak>) {
        let mut sink = InlineNodeSink::new();
        self.parse_inline(MarkupKind::Base, &mut sink);
        let pb = match &self.peeked {
            Some(Peeked::ParagraphBreak(pb)) => Some(ParagraphBreak(pb.clone())),
            _ => None,
        };
        (sink.nodes, pb)
    }
    fn parse_inline(&mut self, markup_kind: MarkupKind, sink: &mut InlineNodeSink) {
        use SyntaxKind::*;
        while let Some(t) = self.lex_inline(|s| lex_markup(s, markup_kind, sink.last_kind)) {
            match t.kind {
                // base tokens
                End => break,
                SoftBreak | HardBreak => sink.eat(t),
                Whitespace | Word | Escaped(_) | Special(_) => sink.eat(t),

                // close tokens
                // cache lexed token and break the loop
                BoldClose if markup_kind == MarkupKind::Bold => {
                    self.peeked = Some(Peeked::Token(t));
                    break;
                }
                ItalicClose if markup_kind == MarkupKind::Italic => {
                    self.peeked = Some(Peeked::Token(t));
                    break;
                }
                MarkupClose if markup_kind == MarkupKind::Markup => {
                    self.peeked = Some(Peeked::Token(t));
                    break;
                }

                BoldOpen => {
                    sink.wrap(Bold, |sink| {
                        sink.eat(t);
                        self.parse_inline(MarkupKind::Bold, sink);
                        if !self.expect(sink, BoldClose) {
                            return
                        }
                        if let Some(next) = self.peek_inline(|s| lex_markup(s, markup_kind, sink.last_kind)) {
                            if next.kind == AttributesOpen {
                                self.parse_attributes(sink);
                            }
                        }
                    });
                }
                ItalicOpen => {
                    sink.wrap(Italic, |sink| {
                        sink.eat(t);
                        self.parse_inline(MarkupKind::Italic, sink);
                        if !self.expect(sink, ItalicClose) {
                            return
                        }
                        if let Some(next) = self.peek_inline(|s| lex_markup(s, markup_kind, sink.last_kind)) {
                            if next.kind == AttributesOpen {
                                self.parse_attributes(sink);
                            }
                        }
                    });
                }
                MarkupOpen => {
                    self.peeked = Some(Peeked::Token(t));
                    sink.wrap(Anchor, |sink| {
                        self.parse_markup(sink);
                        if let Some(next) = self.peek_inline(|s| lex_markup(s, markup_kind, sink.last_kind)) {
                            if next.kind == DestinationOpen {
                                self.parse_destination(sink);
                            }
                        }
                        if let Some(next) = self.peek_inline(|s| lex_markup(s, markup_kind, sink.last_kind)) {
                            if next.kind == AttributesOpen {
                                self.parse_attributes(sink);
                            }
                        }
                    })
                }
                DestinationOpen => {
                    // HACK: seems like I should just peek every new node instead
                    self.peeked = Some(Peeked::Token(t));
                    sink.wrap(Link, |sink| {
                        self.parse_destination(sink);
                        if let Some(next) = self.peek_inline(|s| lex_markup(s, markup_kind, sink.last_kind)) {
                            if next.kind == MarkupOpen {
                                self.parse_markup(sink);
                            }
                        }
                        if let Some(next) = self.peek_inline(|s| lex_markup(s, markup_kind, sink.last_kind)) {
                            if next.kind == AttributesOpen {
                                self.parse_attributes(sink);
                            }
                        }
                    });
                }
                k => unreachable!("unexpected token kind: {k:?}")
            }
        }
    }
    fn assert(&mut self, sink: &mut InlineNodeSink, kind: SyntaxKind) {
        let peeked = self.peeked.take();
        match peeked {
            Some(Peeked::Token(peeked)) if peeked.kind == kind => {
                sink.eat(peeked);
            }
            _ => panic!("expected {kind:?}, got {:?}", peeked),
        }
    }
    /// expects given kind of node from cached token
    /// returns true if cached token matches expected kind
    fn expect(&mut self, sink: &mut InlineNodeSink, kind: SyntaxKind) -> bool {
        let peeked = self.peeked.take();
        match peeked {
            Some(Peeked::Token(peeked)) if peeked.kind == kind => {
                sink.eat(peeked);
                true
            },
            _ => {
                self.peeked = peeked;
                // FIXME: this isn't an ideal solution to get error node position
                // should have two scanner state (one for current, one for peeked)
                // and switch them when peeked token is consumed
                // or just remove range property from error node (detached range?)
                let point = self.peeked.as_ref().map(|p| p.token().range.start).unwrap_or(self.s.cursor());
                sink.nodes.push(SyntaxNode::error(
                    Range::point(point),
                    &format!("missing expected token {kind:?}"),
                ));
                false
            }
        }
    }
    // TODO: don't pass opener. just have `current` token in the parser
    fn parse_attributes(&mut self, sink: &mut InlineNodeSink) {
        use SyntaxKind::*;
        sink.wrap(Attributes, |sink| {
            self.assert(sink, AttributesOpen);
            self.parse_attrs(sink, true);
            self.expect(sink, AttributesClose);
        });
    }
    /// parse nodes in context of attributes
    fn parse_attrs(&mut self, sink: &mut InlineNodeSink, single_line: bool) {
        use SyntaxKind::*;
        while let Some(t) = self.lex_inline(|s| lex_attributes(s, true)) {
            match t.kind {
                End => break,

                // close token
                AttributesClose => {
                    self.peeked = Some(Peeked::Token(t));
                    break
                }

                // attributes tokens
                AttributeDelimiter => sink.eat(t),
                Identifier => {
                    sink.wrap(Attribute, |sink| {
                        sink.eat(t);
                        self.eat_attrs_text(sink);
                    })
                }

                _ => todo!()
            }
        }
    }
    fn parse_markup(&mut self, sink: &mut InlineNodeSink) {
        use SyntaxKind::*;
        sink.wrap(Markup, |sink| {
            self.assert(sink, MarkupOpen);
            self.parse_inline(MarkupKind::Markup, sink);
            self.expect(sink, MarkupClose);
        });
    }
    fn parse_destination(&mut self, sink: &mut InlineNodeSink) {
        use SyntaxKind::*;
        sink.wrap(Destination, |sink| {
            self.assert(sink, DestinationOpen);
            let mut tmp_s = self.s.clone();
            // TODO: skip whitespaces while lookahead for paragraph break
            if let Ok(peek) = lex_smart_destination(&mut tmp_s, true) {
                match peek.kind {
                    DestScopeDelimiter => {
                        self.s = tmp_s;
                        sink.eat_as(peek, DestApplinkPrefix);
                        self.parse_dest_scope(sink)
                    }
                    DestScopeHeadingPrefix | DestScopeWikiHeadingPrefix => {
                        self.parse_dest_scope(sink)
                    }
                    _ => {
                        sink.wrap(DestRawlink, |sink| self.parse_dest_raw(sink))
                    }
                };
            }
            self.expect(sink, DestinationClose);
        });
    }
    fn parse_dest_raw(&mut self, sink: &mut InlineNodeSink) {
        use SyntaxKind::*;
        while let Some(t) = self.lex_inline(|s| lex_destination(s)) {
            match t.kind {
                End => break,
                DestinationClose => {
                    self.peeked = Some(Peeked::Token(t));
                    break
                }
                _ => sink.eat(t),
            }
        }
    }
    // TODO: generalize as eat_until([...])
    fn eat_scope_texts(&mut self, sink: &mut InlineNodeSink) {
        use SyntaxKind::*;
        // eat until End | DestinationClose | ScopeDelimiter
        while let Some(t) = self.lex_inline(|s| lex_smart_destination(s, false)) {
            match t.kind {
                End => break,
                DestinationClose | DestScopeDelimiter => {
                    self.peeked = Some(Peeked::Token(t));
                    break
                },
                _ => sink.eat(t),
            }
        }
    }
    fn eat_attrs_text(&mut self, sink: &mut InlineNodeSink) {
        use SyntaxKind::*;
        while let Some(t) = self.lex_inline(|s| lex_attributes(s, false)) {
            match t.kind {
                End => break,
                AttributesClose | AttributeDelimiter => {
                    self.peeked = Some(Peeked::Token(t));
                    break
                },
                _ => sink.eat(t),
            }
        }
    }
    fn parse_dest_scope(&mut self, sink: &mut InlineNodeSink) {
        use SyntaxKind::*;
        while let Some(t) = self.lex_inline(|s| lex_smart_destination(s, true)) {
            match t.kind {
                // base tokens
                End => break,

                // close token
                DestinationClose => {
                    // pass closing modifier to upper parser and exit the loop
                    self.peeked = Some(Peeked::Token(t));
                    break
                },

                // destination scope tokens
                DestScopeDelimiter => sink.eat(t),
                DestScopeHeadingPrefix => {
                    sink.wrap(DestScopeHeading, |sink| {
                        sink.eat(t);
                        self.eat_scope_texts(sink)
                    });
                }
                DestScopeWikiHeadingPrefix => {
                    sink.wrap(DestScopeWikiHeading, |sink| {
                        sink.eat(t);
                        self.eat_scope_texts(sink)
                    });
                }
                _ => {
                    sink.wrap(DestApplinkPath, |sink| {
                        sink.eat(t);
                        self.eat_scope_texts(sink)
                    });
                }
            }
        }
    }
}

struct InlineNodeSink {
    nodes: Vec<SyntaxNode>,
    last_kind: SyntaxKind,
}
impl InlineNodeSink {
    fn new() -> Self {
        Self {
            nodes: Vec::new(),
            last_kind: SyntaxKind::End,
        }
    }
    fn eat(&mut self, t: SToken) {
        self.last_kind = t.kind;
        self.nodes.push(SyntaxNode::leaf(t.kind, t.range));
    }
    fn eat_as(&mut self, t: SToken, kind: SyntaxKind) {
        self.last_kind = kind;
        self.nodes.push(SyntaxNode::leaf(kind, t.range));
    }
    fn wrap<R>(&mut self, kind: SyntaxKind, func: impl FnOnce(&mut Self) -> R) -> R {
        let prev = std::mem::take(&mut self.nodes);
        let res = func(self);
        let children = std::mem::take(&mut self.nodes);
        self.nodes = prev;
        if !children.is_empty() {
            self.nodes.push(SyntaxNode::inner(kind, children));
        }
        res
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use SyntaxKind::*;

    macro_rules! token {
        ($kind:ident, $start:literal..$end:literal) => {
            SToken {
                kind: SyntaxKind::$kind,
                range: Range::new($start, $end),
            }
        };
    }

    macro_rules! leaf {
        ($kind:expr, $start:literal..$end:literal) => {
            SyntaxNode::leaf($kind, Range::new($start, $end))
        };
    }

    macro_rules! inner {
        ($kind:ident, [$( $child:expr ),* $(,)?]) => {
            SyntaxNode::inner(SyntaxKind::$kind, vec![$($child),*])
        };
    }

    macro_rules! error {
        ($start:literal..$end:literal, $msg:expr) => {
            SyntaxNode::error(Range::new($start, $end), $msg)
        };
    }

    fn parse_paragraph(text: &str) -> (Vec<SyntaxNode>, Option<ParagraphBreak>) {
        let mut p = InlineParser::new(text);
        p.parse_paragraph()
    }

    #[test]
    fn test_paragraph_break() {
        let text = concat!(
            "word\n",
            "* heading\n",
        );
        let (ast, pb) = parse_paragraph(text);
        assert_eq!(
            ast,
            vec![
                leaf!(Word, 0..4),
                leaf!(SoftBreak, 4..5),
            ],
        );
        assert_eq!(pb, Some(ParagraphBreak(token!(HeadingPrefix, 5..6))));

        let text = concat!(
            "word\n",
            "\n",
            "word\n",
        );
        let (ast, pb) = parse_paragraph(text);
        assert_eq!(
            ast,
            vec![
                leaf!(Word, 0..4),
                leaf!(SoftBreak, 4..5),
            ],
        );
        assert_eq!(pb, Some(ParagraphBreak(token!(BlankLine, 5..6))));

        let text = concat!(
            "*/word\n",
            "* heading\n",
        );
        let (ast, pb) = parse_paragraph(text);
        assert_eq!(
            ast,
            vec![
                inner!(Bold, [
                    leaf!(BoldOpen, 0..1),
                    inner!(Italic, [
                        leaf!(ItalicOpen, 1..2),
                        leaf!(Word, 2..6),
                        leaf!(SoftBreak, 6..7),
                        error!(7..7, "missing expected token ItalicClose"),
                    ]),
                    error!(7..7, "missing expected token BoldClose"),
                ]),
            ],
        );
        assert_eq!(pb, Some(ParagraphBreak(token!(HeadingPrefix, 7..8))));

        let text = concat!(
            "[word\n",
            "* heading\n",
        );
        let (ast, pb) = parse_paragraph(text);
        assert_eq!(
            ast,
            vec![
                inner!(Anchor, [
                    inner!(Markup, [
                        leaf!(MarkupOpen, 0..1),
                        leaf!(Word, 1..5),
                        leaf!(SoftBreak, 5..6),
                        error!(6..6, "missing expected token MarkupClose"),
                    ]),
                ]),
            ],
        );
        assert_eq!(pb, Some(ParagraphBreak(token!(HeadingPrefix, 6..7))));

        let text = concat!(
            "{https\n",
            "* heading\n",
        );
        let (ast, pb) = parse_paragraph(text);
        assert_eq!(
            ast,
            vec![
                inner!(Link, [
                    inner!(Destination, [
                        leaf!(DestinationOpen, 0..1),
                        inner!(DestRawlink, [
                            leaf!(Word, 1..6),
                            leaf!(SoftBreak, 6..7),
                        ]),
                        error!(7..7, "missing expected token DestinationClose"),
                    ]),
                ]),
            ],
        );
        assert_eq!(pb, Some(ParagraphBreak(token!(HeadingPrefix, 7..8))));
    }
}
