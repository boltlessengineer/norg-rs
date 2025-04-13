#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unreachable_code)]

use std::{iter::Peekable, ops::Range};

use crate::block::{ListItem, NorgBlock};
use unicode_categories::UnicodeCategories as _;

fn flatatom_to_block(text: &str, block: Mark<AtomBlock>) -> NorgBlock {
    match block.kind {
        AtomBlock::BlankLine => {
            unreachable!("indented line can't have blank line as content")
        }
        AtomBlock::Heading { level, inline } => {
            todo!("heading in list item is not implemented yet")
        }
        AtomBlock::Paragraph => NorgBlock::Paragraph {
            attrs: vec![],
            inlines: vec![],
        },
        AtomBlock::InfirmTag { ident } => NorgBlock::InfirmTag {
            name: text[ident.clone()].to_string(),
            params: None,
        },
        AtomBlock::RangedTag { ident, content } => NorgBlock::RangedTag {
            name: text[ident.clone()].to_string(),
            params: None,
            content: content
                .iter()
                .map(|line| text[line.clone()].to_string())
                .collect(),
        },
        AtomBlock::CarryoverTag { ident } => NorgBlock::CarryoverTag {
            name: text[ident.clone()].to_string(),
            params: None,
            target: todo!("should I parse carryover tag here?"),
        },
    }
}

pub fn flatnodes_to_ao<I>(iter: &mut Peekable<I>, text: &str, lv: usize) -> Option<NorgBlock>
where
    I: Iterator<Item = FlatNode>,
{
    let flat = iter.peek()?;
    match flat {
        FlatNode::Block(block) => {
            let block = block.clone();
            match &block.kind {
                AtomBlock::Heading { level, inline } => {
                    let level = level.clone();
                    if level <= lv {
                        None
                    } else {
                        let mut contents = vec![];
                        iter.next();
                        while let Some(block) = flatnodes_to_ao(iter, text, level) {
                            contents.push(block);
                        }
                        Some(NorgBlock::Section {
                            attrs: vec![],
                            level: level as u16,
                            // TODO: convert inline to Option<Vec<NorgInline>>
                            heading: None,
                            contents,
                        })
                    }
                }
                AtomBlock::BlankLine => {
                    iter.next();
                    flatnodes_to_ao(iter, text, lv)
                }
                _ => {
                    iter.next();
                    Some(flatatom_to_block(text, block))
                }
            }
        }
        FlatNode::Indented(indent_mark, block) => {
            let indent_kind = indent_mark.kind.clone();
            let level = indent_mark.len();
            let mut list_items = vec![ListItem {
                attrs: vec![],
                contents: block
                    .clone()
                    .map_or(vec![], |block| vec![flatatom_to_block(text, block)]),
            }];
            iter.next();
            while let Some(next @ FlatNode::Indented(next_indent_mark, next_block)) = iter.peek() {
                let next_indent_kind = next_indent_mark.kind.clone();
                let next_level = next_indent_mark.len();
                if next_level > level {
                    // push sublist to last list item content
                    let block = flatnodes_to_ao(iter, text, lv).unwrap();
                    list_items.last_mut().unwrap().contents.push(block);
                    continue;
                } else if next_level == level && next_indent_kind == Indent::Null {
                    // push block to last list item's content
                    if let Some(block) = next_block {
                        let block = flatatom_to_block(text, block.clone());
                        list_items.last_mut().unwrap().contents.push(block);
                    }
                } else if next_level == level && indent_kind == next_indent_kind {
                    // push sibling list item to current list
                    list_items.push(ListItem {
                        attrs: vec![],
                        contents: next_block
                            .clone()
                            .map_or(vec![], |block| vec![flatatom_to_block(text, block)]),
                    });
                } else {
                    // end current list
                    break;
                }
                iter.next();
            }
            // rename for convenience
            let attrs = vec![];
            let items = list_items;
            let level = level as u16;
            Some(match indent_kind {
                Indent::Unordered => NorgBlock::UnorderedList {
                    attrs,
                    level,
                    items,
                },
                Indent::Ordered => NorgBlock::OrderedList {
                    attrs,
                    level,
                    items,
                },
                Indent::Quote => NorgBlock::Quote {
                    attrs,
                    level,
                    items,
                },
                Indent::Null => todo!(),
            })
        }
    }
}

pub struct Parser<I: Iterator<Item = FlatNode>> {
    iter: I,
    heading_level: usize,
    nest_level: usize,
}

pub struct Scanner<'src> {
    source: &'src str,
    pos: usize,
}

#[derive(Clone, Debug)]
pub enum FlatNode {
    Indented(Mark<Indent>, Option<Mark<AtomBlock>>),
    Block(Mark<AtomBlock>),
}

impl<'src> Scanner<'src> {
    pub fn new(source: &'src str) -> Self {
        Self { source, pos: 0 }
    }

    pub fn parse_flat(&mut self) -> Vec<FlatNode> {
        let mut blocks: Vec<FlatNode> = vec![];
        loop {
            let flat_node = self.parse_node();
            if let FlatNode::Block(block) = &flat_node {
                if block.span.is_empty() {
                    break;
                }
            }
            if let (
                Some(FlatNode::Indented(_, Some(prev_block)) | FlatNode::Block(prev_block)),
                FlatNode::Block(block),
            ) = (blocks.last_mut(), &flat_node)
            {
                if prev_block.kind == AtomBlock::Paragraph
                    && block.kind == AtomBlock::Paragraph
                {
                    println!("append {:?}", block.span);
                    prev_block.span = prev_block.span.start..block.span.end;
                    self.pos = block.span.end;
                    continue;
                }
            }
            self.pos = match &flat_node {
                FlatNode::Indented(_, Some(block)) => block.span.end,
                FlatNode::Indented(indent, None) => self.pos, // don't update
                FlatNode::Block(block) => block.span.end,
            };
            blocks.push(flat_node);
        }
        blocks
    }

    fn parse_node(&mut self) -> FlatNode {
        self.skip_whitespace();
        let indent = self.parse_indent();
        if let Some(indent) = indent {
            let whitespace = self.lex_common_at(indent.span.end);
            self.pos = whitespace.span.end;
            match whitespace.kind {
                CommonToken::Space => {
                    // NOTE: parse potential attributes (`(...`)
                    // TODO: update the `indent` to `(indent, attrs)`
                    let block = self.parse_pb_block().unwrap_or(Mark::new(AtomBlock::Paragraph, self.lex_line(self.pos)));
                    return FlatNode::Indented(indent, Some(block));
                }
                CommonToken::Newline | CommonToken::Eof => {
                    // TODO: I'm not sure if I should allow newline between indent prefix and the
                    // content or not. It makes sence for null indents, but doesn't make sense for
                    // normal list items:
                    // ```
                    // - <|cursor|>
                    // paragraph. oops I'm belong to the list now
                    // ```
                    // actually wait, isn't it just what multi-line paragraphs do?
                    // like... if I start typing something there, the paragraph below will be part
                    // of list item anyways.
                    return FlatNode::Indented(indent, None);
                }
                _ => unreachable!("parse_indent success only when next token is whitespace"),
            }
        } else {
            let block = self.parse_pb_block().unwrap_or(Mark::new(AtomBlock::Paragraph, self.lex_line(self.pos)));
            FlatNode::Block(block)
        }
    }

    fn skip_whitespace(&mut self) {
        loop {
            let next = self.char_at(self.pos);
            if next != ' ' {
                break;
            }
            self.pos += next.len_utf8();
        }
    }

    fn parse_indent(&self) -> Option<Mark<Indent>> {
        let first = self.current_char();
        let Ok(indent_kind) = Indent::try_from(first) else {
            return None;
        };
        let mut pos = self.pos;
        while self.char_at(pos) == first {
            pos += 1;
        }
        let _count = pos - self.pos;
        if !self.char_at(pos).is_whitespace() {
            return None;
        }
        Some(Mark::new(
            indent_kind,
            self.pos..pos,
        ))
    }

    fn parse_pb_block(&self) -> Option<Mark<AtomBlock>> {
        let first = self.current_char();
        match first {
            '*' => {
                let mut pos = self.pos;
                while self.char_at(pos) == first {
                    pos += 1;
                }
                let count = pos - self.pos;
                let next = self.lex_common_at(pos);
                pos += next.len();
                match next.kind {
                    CommonToken::Space => {
                        if self.char_at(pos) == '(' {
                            todo!("parse attributes");
                        }
                        let title_span = self.lex_line(pos);
                        let span = self.pos..title_span.end;
                        Some(Mark::new(
                            AtomBlock::Heading {
                                level: count,
                                inline: Some(title_span),
                            },
                            span,
                        ))
                    }
                    CommonToken::Newline | CommonToken::Eof => {
                        let span = self.pos..next.span.end;
                        Some(Mark::new(
                            AtomBlock::Heading {
                                level: count,
                                inline: None,
                            },
                            span,
                        ))
                    }
                    _ => None
                }
            }
            '.' | '#' => {
                let pos = self.pos + 1;
                let next = self.char_at(pos);
                if next.is_whitespace() {
                    return None;
                }
                if (first, next) == ('#', '(') {
                    // block attribute
                    todo!("parse attributes")
                } else {
                    // infirm/carryover tags
                    let line_span = self.lex_line(pos);
                    let span = self.pos..line_span.end;
                    let (ident, param) = {
                        let mut res = self.parse_tag_line(line_span).into_iter();
                        (res.next().unwrap(), res.next())
                    };
                    Some(Mark::new(
                        match first {
                            '#' => AtomBlock::CarryoverTag { ident },
                            '.' => AtomBlock::InfirmTag { ident },
                            _ => unreachable!(),
                        },
                        span,
                    ))
                }
            }
            '@' => {
                let mut pos = self.pos + 1;
                let next = self.char_at(pos);
                if next.is_whitespace() {
                    return None;
                }
                let line_span = self.lex_line(pos);
                pos = line_span.end;
                let (ident, param) = {
                    let mut res = self.parse_tag_line(line_span).into_iter();
                    (res.next().unwrap(), res.next())
                };
                let mut content = vec![];
                loop {
                    let line = self.lex_line(pos);
                    if line.is_empty() {
                        todo!("handle ranged tag with missing end modifier");
                    }
                    pos = line.end;
                    let line_str = &self.source[line.clone()];
                    if line_str.trim() == "@end" {
                        break;
                    }
                    content.push(line);
                }
                Some(Mark::new(
                    AtomBlock::RangedTag {
                        ident,
                        content
                    },
                    (self.pos)..pos,
                ))
            }
            '\r' | '\n' => {
                let mark = self.lex_common_at(self.pos);
                debug_assert_eq!(mark.kind, CommonToken::Newline);
                Some(Mark::new(
                    AtomBlock::BlankLine,
                    mark.span,
                ))
            }
            _ => None,
        }
    }

    fn parse_tag_line(&self, line_span: Range<usize>) -> Vec<Range<usize>> {
        let mut pos = line_span.start;
        loop {
            let ch = self.char_at(pos);
            if ch.is_whitespace() {
                break;
            }
            pos += ch.len_utf8();
        }
        let mut res = vec![];
        let ident = line_span.start..pos;
        res.push(ident);
        if !matches!(self.char_at(pos), '\n' | '\0') {
            loop {
                let ch = self.char_at(pos);
                if ch != ' ' {
                    break;
                }
                pos += ch.len_utf8();
            }
            let param = self.lex_line(pos);
            res.push(param);
        }
        res
    }

    fn char_at(&self, pos: usize) -> char {
        if pos >= self.source.len() {
            return '\0';
        }
        self.source[pos..].chars().next().unwrap_or('\0')
    }

    // NOTE: as Parser needs to backtrace a lot, it would be better remove
    // `self.pos` state entirely and pass it to common methods like:
    // - `lex_common(pos: usize)`
    // - `parse_block(pos: usize)`
    fn current_char(&self) -> char {
        self.char_at(self.pos)
    }

    fn lex_common_at(&self, start: usize) -> Mark<CommonToken> {
        let first = self.char_at(start);
        match first {
            ' ' => {
                let mut pos = start;
                loop {
                    let next = self.char_at(pos);
                    if next != ' ' {
                        break;
                    }
                    pos += next.len_utf8();
                }
                if self.char_at(pos) == '\n' {
                    pos += '\n'.len_utf8();
                    Mark::new(CommonToken::Newline, start..pos)
                } else {
                    Mark::new(CommonToken::Space, start..pos)
                }
            }
            '\r' => {
                let mut pos = start + '\r'.len_utf8();
                if self.char_at(pos) == '\n' {
                    pos += '\n'.len_utf8();
                }
                Mark::new(CommonToken::Newline, start..pos)
            },
            '\n' => Mark::new(CommonToken::Newline, start..(start + '\n'.len_utf8())),
            '\0' => Mark::new(CommonToken::Eof, start..start),
            ch if ch.is_punctuation() => Mark::new(
                CommonToken::Special(ch),
                start..(start + ch.len_utf8()),
            ),
            ch => unimplemented!("implement for {ch}")
        }
    }
    fn lex_line(&self, from: usize) -> Range<usize> {
        let mut to = from;
        loop {
            let next = self.char_at(to);
            if next == '\0' {
                break;
            }
            to += next.len_utf8();
            if next == '\n' || next == '\0' {
                break;
            }
        }
        from..to
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum AtomBlock {
    BlankLine,
    Paragraph,
    Heading {
        level: usize,
        inline: Option<Range<usize>>,
    },
    InfirmTag {
        ident: Range<usize>,
    },
    CarryoverTag {
        ident: Range<usize>,
    },
    RangedTag {
        ident: Range<usize>,
        content: Vec<Range<usize>>,
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Indent {
    Unordered,
    Ordered,
    Quote,
    Null,
}

impl TryFrom<char> for Indent {
    type Error = ();
    fn try_from(ch: char) -> Result<Self, Self::Error> {
        match ch {
            '-' => Ok(Self::Unordered),
            '~' => Ok(Self::Ordered),
            '>' => Ok(Self::Quote),
            '/' => Ok(Self::Null),
            _ => Err(()),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Mark<K> {
    kind: K,
    span: Range<usize>,
}

impl<K> Mark<K> {
    fn new(kind: K, span: Range<usize>) -> Self {
        Self { kind, span }
    }
    /// len_utf8
    fn len(&self) -> usize {
        self.span.end - self.span.start
    }

    fn text<'src>(&self, source: &'src str) -> &'src str {
        &source[self.span.clone()]
    }
}

#[derive(Debug, PartialEq)]
enum CommonToken {
    Space,
    Newline,
    Eof,
    Special(char),
    Text,
}

