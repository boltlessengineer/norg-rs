use unicode_categories::UnicodeCategories;
use unscanny::Scanner;

use crate::{node::{SyntaxKind, SyntaxNode}, Range};

#[derive(Clone, Debug, PartialEq)]
pub struct SToken {
    pub kind: SyntaxKind,
    pub range: Range,
}

impl SToken {
    pub fn to_node(self) -> SyntaxNode {
        SyntaxNode::leaf(self.kind, self.range)
    }
    pub fn as_kind(self, kind: SyntaxKind) -> Self {
        Self { kind, range: self.range }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MarkupKind {
    Bold,
    Italic,
    Markup,
    /// base kind where inline markup isn't nested
    Base,
}

impl MarkupKind {
    fn to_open(&self) -> SyntaxKind {
        match self {
            Self::Bold => SyntaxKind::BoldOpen,
            Self::Italic => SyntaxKind::ItalicOpen,
            Self::Markup => SyntaxKind::MarkupOpen,
            Self::Base => panic!(),
        }
    }
    fn to_close(&self) -> SyntaxKind {
        match self {
            Self::Bold => SyntaxKind::BoldClose,
            Self::Italic => SyntaxKind::ItalicClose,
            Self::Markup => SyntaxKind::MarkupClose,
            Self::Base => panic!(),
        }
    }
}

/// lex
/// - base tokens (end, word, whitespace, escaped, etc)
fn lex_base(s: &mut Scanner) -> SToken {
    let start = s.cursor();
    let Some(ch) = s.eat() else {
        return SToken { kind: SyntaxKind::End, range: Range::point(start) };
    };
    let kind = if is_newline(ch) {
        SyntaxKind::SoftBreak
    } else if is_whitespace(ch) {
        s.eat_while(|ch| !is_newline(ch) && is_whitespace(ch));
        SyntaxKind::Whitespace
    } else if ch == '\\' && s.eat_if(is_punctuation) {
        SyntaxKind::Escaped(s.scout(-1).unwrap())
    } else if is_punctuation(ch) {
        SyntaxKind::Special(ch)
    } else {
        s.eat_while(is_word);
        SyntaxKind::Word
    };
    let range = Range::new(start, s.cursor());
    SToken { kind, range }
}

fn attached_mod_from_char(ch: char) -> Option<MarkupKind> {
    match ch {
        '*' => Some(MarkupKind::Bold),
        '/' => Some(MarkupKind::Italic),
        _ => None,
    }
}

/// - base tokens (end, word, whitespace, escaped, etc)
/// - (not current)_open (this can include markup_open)
/// - (current)_close
/// - verbatim_open
/// - destination_open
/// - markup_open
pub fn lex_markup(s: &mut Scanner, current: MarkupKind, last_kind: SyntaxKind) -> Result<SToken, ParagraphBreak> {
    use SyntaxKind::*;
    if s.scout(-1).is_none_or(is_newline) {
        if let Some(pb) = lex_non_para_prefix(s) {
            return Err(ParagraphBreak(pb));
        }
    }
    let base = lex_base(s);
    let can_open = |s: &mut Scanner, ch: char, last_kind: SyntaxKind| {
        let Some(kind) = attached_mod_from_char(ch) else {
            return false;
        };
        current != kind
            && last_kind != Word
            && last_kind != Special(ch)
            && {
                let peek = s.peek();
                peek.is_some_and(|c| !is_whitespace(c) && c != ch)
            }
    };
    let can_close = |s: &mut Scanner, ch: char, last_kind: SyntaxKind| {
        let Some(kind) = attached_mod_from_char(ch) else {
            return false;
        };
        current == kind
            && (last_kind != End
                && last_kind != Whitespace
                && last_kind != SoftBreak
                && last_kind != HardBreak)
            && last_kind != Special(ch)
            && {
                let peek = s.peek();
                peek.is_none_or(|c| !is_word(c) && c != ch)
            }
    };
    Ok(match base.kind {
        Special('{') => base.as_kind(DestinationOpen),
        Special('[') => base.as_kind(MarkupOpen),
        Special(']') if current == MarkupKind::Markup => base.as_kind(MarkupClose),
        // TODO(boltless): do I need to parse this from lexer?
        // it's faster to peek from parser side
        Special('(') if last_kind == BoldClose => base.as_kind(AttributeOpen),
        Special(ch) if can_open(s, ch, last_kind) => {
            let kind = attached_mod_from_char(ch).unwrap();
            base.as_kind(kind.to_open())
        }
        Special(ch) if can_close(s, ch, last_kind) => {
            let kind = attached_mod_from_char(ch).unwrap();
            let mut tmp_s = s.clone();
            let next = lex_base(&mut tmp_s);
            match next.kind {
                Special(':') => {
                    *s = tmp_s;
                    SToken {
                        kind: kind.to_close(),
                        range: Range::new(base.range.start, next.range.end),
                    }
                }
                _ => base.as_kind(kind.to_close()),
            }
        }
        Special(':') => {
            let mut tmp_s = s.clone();
            let next = lex_base(&mut tmp_s);
            match next.kind {
                Special(ch) if can_open(&mut tmp_s, ch, base.kind) => {
                    let kind = attached_mod_from_char(ch).unwrap();
                    *s = tmp_s;
                    SToken {
                        kind: kind.to_open(),
                        range: Range::new(base.range.start, next.range.end),
                    }
                }
                _ => base
            }
        }
        _ => base,
    })
}

/// - base tokens (end, word, whitespace, escaped, etc)
/// - verbatim_close
pub fn lex_verbatim(s: &mut Scanner) -> SToken {
    use SyntaxKind::*;
    let base = lex_base(s);
    match base.kind {
        Special('`') => base.as_kind(VerbatimClose),
        _ => base
    }
}

/// pretty similar to lex_verbatim but for destination
/// - base tokens (end, word, whitespace, escaped, etc)
/// - destination_close
pub fn lex_destination(s: &mut Scanner) -> Result<SToken, ParagraphBreak> {
    use SyntaxKind::*;
    if s.scout(-1).is_none_or(is_newline) {
        if let Some(pb) = lex_non_para_prefix(s) {
            return Err(ParagraphBreak(pb));
        }
    }
    let base = lex_base(s);
    Ok(match base.kind {
        Special('}') => base.as_kind(DestinationClose),
        _ => base
    })
}

/// parse applink and local link. parse prefix tokens when `start` is true
/// - base tokens (end, word, whitespace, escaped, etc)
/// - scope_delimiter
/// - heading_prefix
/// - destination_close
pub fn lex_smart_destination(s: &mut Scanner, start: bool) -> Result<SToken, ParagraphBreak> {
    use SyntaxKind::*;
    if s.scout(-1).is_none_or(is_newline) {
        if let Some(pb) = lex_non_para_prefix(s) {
            return Err(ParagraphBreak(pb));
        }
    }
    if start {
        s.eat_while(is_whitespace);
    }
    let base = lex_base(s);
    Ok(match base.kind {
        Special('}') => base.as_kind(DestinationClose),
        Special(':') => {
            // skip all trailing whitespaces
            s.eat_while(is_whitespace);
            base.as_kind(DestScopeDelimiter)
        }
        Special('*') if start => {
            // eat all repeated characters
            s.eat_while('*');
            let range = Range::new(base.range.start, s.cursor());
            s.eat_while(is_whitespace);
            SToken { kind: DestScopeHeadingPrefix, range }
        }
        Special('?') if start => {
            s.eat_while(is_whitespace);
            base.as_kind(DestScopeWikiHeadingPrefix)
        }
        Special('$') if start => {
            base.as_kind(DestApplinkWorkspacePrefix)
        }
        // NOTE: this won't work well because it will break lookahead_paragraph_break rule
        // Whitespace | SoftBreak => {
        //     let mut tmp_s = s.clone();
        //     tmp_s.eat_while(is_whitespace);
        //     let start = tmp_s.cursor();
        //     if tmp_s.eat_if(':') {
        //         let range = Range::new(start, tmp_s.cursor());
        //         *s = tmp_s;
        //         SToken { kind: DestScopeDelimiter, range }
        //     } else {
        //         base
        //     }
        // }
        _ => base
    })
}

/// - identifier
/// - base tokens (end, word, whitespace, escaped, etc)
/// - attribute_delimiter
/// - attribute_end
pub fn lex_attributes(s: &mut Scanner, start: bool) -> Result<SToken, ParagraphBreak> {
    todo!()
}

/// - Indent
/// - HeadingPrefix
/// - RangedTagPrefix
/// - BlankLine
/// - HorizontalRule
pub fn lex_non_para_prefix(s: &mut Scanner) -> Option<SToken> {
    use SyntaxKind::*;
    debug_assert!(s.scout(-1).is_none_or(is_newline), "should be called at line start");
    // skip preceding whitespaces
    s.eat_while(|ch| !is_newline(ch) && is_whitespace(ch));
    let mut tmp_s = s.clone();
    let start = tmp_s.cursor();
    let ch = tmp_s.eat()?;
    match ch {
        '-' | '~' | '>' | '/' => {
            tmp_s.eat_while('*');
            if tmp_s.peek().is_none_or(is_whitespace) {
                *s = tmp_s;
                Some(SToken {
                    kind: match ch {
                        '-' => UnorderedPrefix,
                        '~' => OrderedPrefix,
                        '>' => QuotePrefix,
                        '/' => NullPrefix,
                        _ => unreachable!(),
                    },
                    range: Range::new(start, s.cursor()),
                })
            } else {
                None
            }
        }
        '*' => {
            tmp_s.eat_while('*');
            if tmp_s.peek().is_none_or(is_whitespace) {
                *s = tmp_s;
                Some(SToken {
                    kind: HeadingPrefix,
                    range: Range::new(start, s.cursor()),
                })
            } else {
                None
            }
        }
        '_' => {
            tmp_s.eat_while('_');
            let range = Range::new(start, s.cursor());
            if range.len() < 2 {
                return None
            }
            tmp_s.eat_while(is_inline_whitespace);
            if tmp_s.done() || tmp_s.eat_if(is_newline) {
                *s = tmp_s;
                Some(SToken {
                    kind: HorizontalLine,
                    range,
                })
            } else {
                None
            }
        }
        '.' if tmp_s.peek().is_some_and(is_word) => {
            *s = tmp_s;
            Some(SToken {
                kind: InfirmTagPrefix,
                range: Range::new(start, tmp_s.cursor()),
            })
        }
        '#' if tmp_s.peek().is_some_and(|ch| ch == '(' || is_word(ch)) => {
            *s = tmp_s;
            Some(SToken {
                kind: CarryoverPrefix,
                range: Range::new(start, tmp_s.cursor()),
            })
        }
        '@' => {
            tmp_s.eat_while('@');
            if tmp_s.peek().is_some_and(is_word) {
                let range = Range::new(start, tmp_s.cursor());
                *s = tmp_s;
                Some(SToken {
                    kind: RangedTagPrefix,
                    range,
                })
            } else {
                None
            }
        }
        _ if is_newline(ch) => {
            *s = tmp_s;
            Some(SToken {
                kind: BlankLine,
                range: Range::new(start, tmp_s.cursor()),
            })
        }
        _ => None,
    }
}

fn is_whitespace(ch: char) -> bool {
    ch.is_ascii_whitespace()
}

fn is_newline(ch: char) -> bool {
    ch == '\n' || ch == '\r'
}

fn is_inline_whitespace(ch: char) -> bool {
    !is_newline(ch) && is_whitespace(ch)
}

fn is_punctuation(ch: char) -> bool {
    ch.is_ascii_punctuation()
}

fn is_word(ch: char) -> bool {
    !is_whitespace(ch) && !is_punctuation(ch)
}

#[derive(Clone, Debug, PartialEq)]
pub struct ParagraphBreak(pub(crate) SToken);

// #[derive(Clone, Debug, PartialEq)]
// pub enum ParagraphBreak {
//     // End,
//     // UnorderedIndent,
//     // OrderedIndent,
//     // QuoteIndent,
//     // NullIndent,
//     // HeadingPrefix,
//     BlankLine,
//     // TODO: add base_blocks like tag prefix and horizontal rule
// }
