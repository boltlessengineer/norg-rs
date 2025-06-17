use unscanny::Scanner;

use crate::{node::{SyntaxKind, SyntaxNode}, Range};

use super::lexer2::*;

#[derive(Clone)]
pub struct InlineParser<'s> {
    s: Scanner<'s>,
    // TODO: `leftover` is missleading. rename this to `leftover`
    peeked: Option<SToken>,
}
impl<'s> InlineParser<'s> {
    pub fn new(text: &'s str) -> Self {
        Self {
            s: Scanner::new(text),
            peeked: None,
        }
    }
    /// lex with given lexer
    /// it will return peeked token if there are any
    fn lex(
        &mut self,
        lexer: impl FnOnce(&mut Scanner) -> Result<SToken, ParagraphBreak>,
    ) -> Result<SToken, ParagraphBreak> {
        // TODO: maybe move paragraph break lexer to here
        self.peeked.take().map(Ok).unwrap_or_else(|| lexer(&mut self.s))
    }
    pub fn parse_paragraph(&mut self) -> Vec<SyntaxNode> {
        let mut sink = InlineNodeSink::new();
        let pb = self.parse_inline(MarkupKind::Base, &mut sink);
        dbg!(pb);
        sink.nodes
    }
    fn parse_inline(&mut self, markup_kind: MarkupKind, sink: &mut InlineNodeSink) -> Result<(), ParagraphBreak> {
        use SyntaxKind::*;
        loop {
            let t = self.lex(|s| lex_markup(s, markup_kind, sink.last_kind))?;
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
                        match peek.kind {
                            DestScopeDelimiter => {
                                self.s = tmp_s;
                                sink.eat_as(peek, DestApplinkPrefix);
                                self.parse_dest_scope(sink)?
                            }
                            DestScopeHeadingPrefix | DestScopeWikiHeadingPrefix => {
                                self.parse_dest_scope(sink)?
                            }
                            _ => {
                                sink.wrap(DestRawlink, |sink| self.parse_dest_raw(sink))?;
                            }
                        }
                        sink.eat_if(&mut self.peeked, DestinationClose);
                        Ok::<(), ParagraphBreak>(())
                    })?;
                }
                k => unreachable!("unexpected token kind: {k:?}")
            }
        }
        Ok(())
    }
    fn parse_dest_raw(&mut self, sink: &mut InlineNodeSink) -> Result<(), ParagraphBreak> {
        use SyntaxKind::*;
        loop {
            let t = lex_destination(&mut self.s)?;
            match t.kind {
                End => break,
                DestinationClose => {
                    self.peeked = Some(t);
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
                    self.peeked = Some(t);
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
            let t = self.lex(|s| lex_smart_destination(s, true))?;
            // let t = self.peeked.take().map(Ok).unwrap_or_else(|| lex_smart_destination(&mut self.s, true))?;
            match t.kind {
                // base tokens
                End => break,

                // close token
                DestinationClose => {
                    // pass closing modifier to upper parser and exit the loop
                    self.peeked = Some(t);
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
    fn eat_if(&mut self, t: &mut Option<SToken>, kind: SyntaxKind) {
        if let Some(t) = t.take() {
            if t.kind == kind {
                self.eat(t);
                return
            }
        }
        self.nodes.push(SyntaxNode::error(
            Range::point(0),
            &format!("missing token. expected {kind:?}"),
        ));
    }
}

fn is_whitespace(ch: char) -> bool {
    ch.is_ascii_whitespace()
}

fn is_newline(ch: char) -> bool {
    ch == '\n' || ch == '\r'
}

fn is_punctuation(ch: char) -> bool {
    ch.is_ascii_punctuation()
}

fn is_word(ch: char) -> bool {
    !is_whitespace(ch) && !is_punctuation(ch)
}
