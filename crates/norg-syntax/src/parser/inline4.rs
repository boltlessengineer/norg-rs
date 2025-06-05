use unscanny::Scanner;

use crate::{node::{SyntaxKind, SyntaxNode}, Range};

use super::lexer2::*;

#[derive(Clone)]
pub struct InlineParser<'s> {
    s: Scanner<'s>,
    cached_pb: Option<ParagraphBreak>,
    peeked: Option<SToken>,
}
impl<'s> InlineParser<'s> {
    pub fn new(text: &'s str) -> Self {
        Self {
            s: Scanner::new(text),
            cached_pb: None,
            peeked: None,
        }
    }
    /// lookahead next line for paragraph break
    /// if found any, cache it to self.cached_pb
    fn lookahead_paragraph_break(&mut self) -> bool {
        if self.s.eat_if("---") {
            self.cached_pb = Some(ParagraphBreak::BlankLine);
            true
        } else {
            false
        }
    }
    /// skip whitespaces with the awareness of possible following paragraph break
    fn safe_skip_whitespaces(s: &mut Scanner, cached_pb: &mut Option<ParagraphBreak>) -> bool {
        while s.at(is_whitespace) {
            let is_eol = s.at(is_newline);
            s.eat();
            if is_eol && s.eat_if("---") {
                *cached_pb = Some(ParagraphBreak::BlankLine);
                return true
            }
        }
        return false;
    }
    pub fn parse_paragraph(&mut self) -> Vec<SyntaxNode> {
        let mut sink = InlineNodeSink::new();
        self.parse_inline(MarkupKind::Base, &mut sink);
        sink.nodes
    }
    fn parse_inline(&mut self, markup_kind: MarkupKind, sink: &mut InlineNodeSink) -> Result<(), ParagraphBreak> {
        use SyntaxKind::*;
        while self.cached_pb.is_none() {
            let t = {
                if let Some(t) = self.peeked.take() {
                    t
                } else {
                    lex_markup(&mut self.s, markup_kind, sink.last_kind)?
                }
            };
            match t.kind {
                // base tokens
                End => break,
                SoftBreak | HardBreak => {
                    sink.eat(t);
                    if self.lookahead_paragraph_break() {
                        break
                    }
                }
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
                        // TODO: we just peek here so it's safe to bravely use
                        // lex_smart_destination(&mut tmp_s, true)
                        // we can even use it to eat peeked value and advance instead of backtrace
                        if Self::safe_skip_whitespaces(&mut tmp_s, &mut self.cached_pb) {
                            return
                        }
                        let peek = lex_smart_destination(&mut tmp_s, true);
                        match peek.kind {
                            DestScopeDelimiter => {
                                self.s = tmp_s;
                                sink.eat_as(peek, DestApplinkPrefix);
                                self.parse_dest_scope(sink);
                            }
                            DestScopeHeadingPrefix | DestScopeWikiHeadingPrefix => {
                                self.parse_dest_scope(sink)
                            }
                            _ => {
                                sink.wrap(DestRawlink, |sink| self.parse_dest_raw(sink));
                            }
                        }
                        sink.eat_if(&mut self.peeked, DestinationClose);
                    });
                }
                k => unreachable!("unexpected token kind: {k:?}")
            }
        }
        Ok(())
    }
    fn parse_dest_raw(&mut self, sink: &mut InlineNodeSink) {
        use SyntaxKind::*;
        loop {
            let t = lex_destination(&mut self.s);
            match t.kind {
                DestinationClose => {
                    self.peeked = Some(t);
                    break
                }
                End => break,
                SoftBreak | HardBreak => {
                    sink.eat(t);
                    if self.lookahead_paragraph_break() {
                        break
                    }
                }
                _ => sink.eat(t),
            }
        }
    }
    //
    //
    //
    // TODO: integrate lookahead_paragraph_break into lexer
    // not in lex_XXX() methods, but from where they are called.
    // pass `lookahead_pb() -> B` to lexer
    // (use `s.scout(-1)` to check the line start instead of placing callback on
    // `SoftBreak | HardBreak`)
    // and the lexer methods return `Result<SToken, B>` so that we can use wonderful `?` syntax
    // the parser will return `Result<(), B>`
    //
    // one point I'm worried about this is when lexer is called in `wrap()`.
    // also I still need `peeked` for DestScopeDelimiter and DestinationClose
    //
    //
    //
    //
    fn parse_dest_scope(&mut self, sink: &mut InlineNodeSink) {
        use SyntaxKind::*;
        loop {
            let t = self.peeked.take().unwrap_or_else(|| lex_smart_destination(&mut self.s, true));
            match t.kind {
                // base tokens
                End => break,

                // TODO: do I need these here?
                // will these ever work?
                SoftBreak | HardBreak => {
                    sink.eat(t);
                    if self.lookahead_paragraph_break() {
                        break
                    }
                }
                // skip whitespace
                Whitespace => {}
                Word | Escaped(_) | Special(_) => sink.eat(t),

                // close token
                DestinationClose => {
                    self.peeked = Some(t);
                    break
                },

                // destination scope tokens
                DestScopeDelimiter => sink.eat(t),
                DestScopeHeadingPrefix => {
                    sink.wrap(DestScopeHeading, |sink| {
                        sink.eat(t);
                        // eat until End | DestinationClose | ScopeDelimiter
                        loop {
                            let t = lex_smart_destination(&mut self.s, false);
                            match t.kind {
                                End => break,
                                DestinationClose | DestScopeDelimiter => {
                                    self.peeked = Some(t);
                                    break
                                },
                                // TODO: handle paragraph break
                                _ => sink.eat(t),
                            }
                        }
                    });
                }
                DestScopeWikiHeadingPrefix => {
                    sink.wrap(DestScopeWikiHeading, |sink| {
                        sink.eat(t);
                        // eat until End | DestinationClose | ScopeDelimiter
                        loop {
                            let t = lex_smart_destination(&mut self.s, false);
                            match t.kind {
                                End => break,
                                DestinationClose | DestScopeDelimiter => {
                                    self.peeked = Some(t);
                                    break
                                },
                                // TODO: handle paragraph break
                                _ => sink.eat(t),
                            }
                        }
                    });
                }
                k => unreachable!("unexpected token kind: {k:?}")
            }
        }
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
