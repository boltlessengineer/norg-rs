use unscanny::Scanner;

use crate::{node::{SyntaxKind, SyntaxNode}, Range};

use super::lexer2::*;

#[derive(Clone)]
enum Peeked {
    Token(SToken),
    ParagraphBreak(SToken),
}

#[derive(Clone)]
pub struct InlineParser<'s> {
    s: Scanner<'s>,
    // TODO: `leftover` is missleading. rename this to `leftover`
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
    ) -> Result<SToken, ParagraphBreak> {
        // TODO: maybe just move paragraph break lexer to here
        match self.peeked.take() {
            Some(Peeked::Token(peeked)) => Ok(peeked),
            // TODO: if peeked value was paragraph break, save it back to peeked and return None
            // so parsers can continue while lex_inline return Some() value
            // and optionally break on Some(DestinationClose)
            // because this left-over DestinationClose can be stored in `peeked`, upper parser can
            // retrieve it without re-parsing
            Some(Peeked::ParagraphBreak(peeked)) => Err(ParagraphBreak(peeked)),
            None => lexer(&mut self.s),
        }
    }
    pub fn parse_paragraph(&mut self) -> (Vec<SyntaxNode>, Option<ParagraphBreak>) {
        let mut sink = InlineNodeSink::new();
        let pb = self.parse_inline(MarkupKind::Base, &mut sink).err();
        (sink.nodes, pb)
    }
    fn parse_inline(&mut self, markup_kind: MarkupKind, sink: &mut InlineNodeSink) -> Result<(), ParagraphBreak> {
        use SyntaxKind::*;
        loop {
            let t = self.lex_inline(|s| lex_markup(s, markup_kind, sink.last_kind))?;
            match t.kind {
                // base tokens
                End => break,
                SoftBreak | HardBreak => sink.eat(t),
                Whitespace | Word | Escaped(_) | Special(_) => sink.eat(t),

                // close tokens
                BoldClose if markup_kind == MarkupKind::Bold => {
                    sink.eat(t);
                    // TODO: peek next to see if it's attribute opener
                    // and then parse it as attributes
                    break;
                }
                ItalicClose if markup_kind == MarkupKind::Italic => {
                    sink.eat(t);
                    break;
                }

                BoldOpen => {
                    sink.wrap(Bold, |sink| {
                        sink.eat(t);
                        self.parse_inline(MarkupKind::Bold, sink)
                    })?;
                }
                ItalicOpen => {
                    sink.wrap(Italic, |sink| {
                        sink.eat(t);
                        self.parse_inline(MarkupKind::Italic, sink)
                    })?;
                }
                DestinationOpen => {
                    sink.wrap(Destination, |sink| {
                        sink.eat(t);
                        let mut tmp_s = self.s.clone();
                        // TODO: skip whitespaces while lookahead for paragraph break
                        let peek = lex_smart_destination(&mut tmp_s, true)?;
                        let res = match peek.kind {
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
                        // FIXME: I hate how I made this.
                        // seems like Result is not a good way to pass paragraph break state
                        // I should rather save peeked paragraph break in parser stated like how
                        // I'm saving the peeked token.
                        // ```
                        // enum Peeked {
                        //     Token(SToken),
                        //     ParagraphBreak(SToken),
                        // }
                        // struct Parser {
                        //     peeked: Option<Peeked>,
                        //     ...
                        // }
                        // fn lex(&mut self, lexer) -> Option<SToken> {
                        //     // return non-paragraph-break token
                        // }
                        //
                        // self.expect(sink, DestinationClose);
                        // ```
                        self.expect(sink, DestinationClose);
                        res
                    })?;
                }
                k => unreachable!("unexpected token kind: {k:?}")
            }
        }
        Ok(())
    }
    fn expect(&mut self, sink: &mut InlineNodeSink, kind: SyntaxKind) {
        let peeked = self.peeked.take();
        match peeked {
            Some(Peeked::Token(peeked)) if peeked.kind == kind => sink.eat(peeked),
            _ => {
                self.peeked = peeked;
                sink.nodes.push(SyntaxNode::error(
                    Range::point(0),
                    &format!("missing expected token {kind:?}"),
                ));
            }
        }
    }
    fn parse_dest_raw(&mut self, sink: &mut InlineNodeSink) -> Result<(), ParagraphBreak> {
        use SyntaxKind::*;
        loop {
            let t = lex_destination(&mut self.s)?;
            match t.kind {
                End => break,
                DestinationClose => {
                    self.peeked = Some(Peeked::Token(t));
                    break
                }
                _ => sink.eat(t),
            }
        }
        Ok(())
    }
    fn parse_scope_text(&mut self, sink: &mut InlineNodeSink) -> Result<(), ParagraphBreak> {
        use SyntaxKind::*;
        // eat until End | DestinationClose | ScopeDelimiter
        loop {
            let t = lex_smart_destination(&mut self.s, false)?;
            match t.kind {
                End => break,
                DestinationClose | DestScopeDelimiter => {
                    self.peeked = Some(Peeked::Token(t));
                    break
                },
                _ => sink.eat(t),
            }
        }
        Ok(())
    }
    fn parse_dest_scope(&mut self, sink: &mut InlineNodeSink) -> Result<(), ParagraphBreak> {
        use SyntaxKind::*;
        loop {
            let t = self.lex_inline(|s| lex_smart_destination(s, true))?;
            // let t = self.peeked.take().map(Ok).unwrap_or_else(|| lex_smart_destination(&mut self.s, true))?;
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
                        self.parse_scope_text(sink)
                    })?;
                }
                DestScopeWikiHeadingPrefix => {
                    sink.wrap(DestScopeWikiHeading, |sink| {
                        sink.eat(t);
                        self.parse_scope_text(sink)
                    })?;
                }
                _ => {
                    sink.wrap(DestApplinkPath, |sink| {
                        sink.eat(t);
                        self.parse_scope_text(sink)
                    })?;
                }
            }
        }
        Ok(())
    }
}

// TODO: test all these three cases
//
// word *word
// * heading
//
// word {:word
// * heading
//
// word *word*(
// * heading

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
