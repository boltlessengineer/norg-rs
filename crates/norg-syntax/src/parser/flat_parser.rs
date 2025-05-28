use super::lexer::{ArgumentTokenKind, Lexer, NormalTokenKind, RangedTagTokenKind};

use crate::{
    node::{SyntaxKind, SyntaxNode},
    Range,
};

pub struct FlatParser<'s> {
    lexer: Lexer<'s>,
}

impl<'s> FlatParser<'s> {
    pub fn new(text: &'s str) -> Self {
        Self {
            lexer: Lexer::new(text),
        }
    }
    pub fn parse(&mut self) -> Vec<FlatBlockData> {
        let mut blocks = vec![];
        loop {
            match self.parse_next() {
                IncompleteNext::FlatBlock(data) => blocks.push(data),
                IncompleteNext::ParagraphSegment(range) => match blocks.last_mut() {
                    Some(prev) if prev.block_kind == BaseBlockKind::Paragraph => {
                        let base_block_node = &mut prev.node;
                        *base_block_node = SyntaxNode::leaf(
                            SyntaxKind::Paragraph,
                            Range::new(base_block_node.range().start, range.end),
                        );
                    }
                    _ => {
                        blocks.push(FlatBlockData {
                            indent: Indent::base(),
                            block_kind: BaseBlockKind::Paragraph,
                            node: SyntaxNode::leaf(SyntaxKind::Paragraph, range),
                        });
                    }
                },
                IncompleteNext::EOF => break,
            }
        }
        blocks
    }
    fn parse_next(&mut self) -> IncompleteNext {
        let t = self.lexer.peek();
        let start = t.range.start;
        match t.kind {
            NormalTokenKind::EOF => IncompleteNext::EOF,
            NormalTokenKind::Whitespace => {
                // skip preceding whitespaces and continue
                self.lexer.next();
                self.parse_next()
            }
            NormalTokenKind::Special(c @ ('-' | '~' | '>' | '/')) => {
                self.lexer.next();
                let wt = self.lexer.next();
                if !wt.is_whitespace() {
                    let range = self.lexer.eat_line();
                    return IncompleteNext::ParagraphSegment(Range::new(start, range.end));
                }
                let level = t.len();
                let kind = match c {
                    '-' => IndentKind::Unordered,
                    '~' => IndentKind::Ordered,
                    '>' => IndentKind::Quote,
                    '/' => IndentKind::Null,
                    _ => unreachable!(),
                };
                let mut indent_nodes = vec![];
                let prefix = SyntaxNode::leaf(kind.to_syntax_prefix(), t.range);
                indent_nodes.push(prefix);
                if wt.kind == NormalTokenKind::Whitespace && self.lexer.peek().is_char('(') {
                    todo!("parse attributes");
                }
                let block = self.parse_base_block();
                let block_kind = BaseBlockKind::from(block.kind());
                // let indented_flat_block = SyntaxNode::inner(kind.to_syntax(), nodes);
                IncompleteNext::FlatBlock(FlatBlockData {
                    indent: Indent::new(kind, level, indent_nodes),
                    block_kind,
                    node: block,
                })
            }
            _ => {
                let block = self.parse_base_block();
                let block_kind = BaseBlockKind::from(block.kind());
                if block_kind == BaseBlockKind::Paragraph {
                    return IncompleteNext::ParagraphSegment(block.range());
                }
                IncompleteNext::FlatBlock(FlatBlockData {
                    indent: Indent::base(),
                    block_kind,
                    node: block,
                })
            }
        }
    }
    fn parse_base_block(&mut self) -> SyntaxNode {
        let t = self.lexer.peek();
        let start = t.range.start;
        match t.kind {
            // remove preceding whitespaces e.g.
            // - ( )   asdf
            //      ^^^
            NormalTokenKind::Whitespace => {
                self.lexer.next();
                return self.parse_base_block();
            }
            NormalTokenKind::Newline | NormalTokenKind::EOF => {
                self.lexer.next();
                return SyntaxNode::leaf(SyntaxKind::BlankLine, t.range);
            }
            NormalTokenKind::Special('_') if t.len() > 1 => {
                self.lexer.next();
                let next = self.lexer.peek();
                if next.is_eol() {
                    self.lexer.next();
                    return SyntaxNode::leaf(SyntaxKind::HorizontalLine, t.range);
                }
            }
            NormalTokenKind::Special('*') => 'heading: {
                let mut level = 0;
                while self.lexer.peek().kind == t.kind {
                    level += 1;
                    self.lexer.next();
                }
                let wt = self.lexer.peek();
                if !wt.is_whitespace() {
                    break 'heading;
                }
                self.lexer.next();
                let prefix_node = SyntaxNode::leaf(SyntaxKind::HeadingPrefix, t.range);
                let mut nodes = vec![prefix_node];
                if !wt.is_eol() {
                    let title_range = self.lexer.eat_line();
                    nodes.push(SyntaxNode::leaf(SyntaxKind::Paragraph, title_range));
                    return SyntaxNode::inner(SyntaxKind::Heading(level), nodes);
                }
                return SyntaxNode::inner(SyntaxKind::BlankHeading(level), nodes);
            }
            NormalTokenKind::Special('.') if t.len() == 1 => 'infirm_tag: {
                let Some(leafs) = self.parse_tag_line(SyntaxKind::InfirmTagPrefix) else {
                    break 'infirm_tag;
                };
                // advance possible newline token
                self.lexer.next();
                return SyntaxNode::inner(SyntaxKind::InfirmTag, leafs);
            }
            NormalTokenKind::Special('#') if t.len() == 1 => 'carryover_tag: {
                let Some(leafs) = self.parse_tag_line(SyntaxKind::InfirmTagPrefix) else {
                    break 'carryover_tag;
                };
                // advance possible newline token
                self.lexer.next();
                return SyntaxNode::inner(SyntaxKind::InfirmTag, leafs);
            }
            NormalTokenKind::Special('@') => 'ranged_tag: {
                let mut level = 0;
                while self.lexer.peek().kind == t.kind {
                    level += 1;
                    self.lexer.next();
                }
                let Some(mut leafs) = self.parse_tag_line(SyntaxKind::InfirmTagPrefix) else {
                    break 'ranged_tag;
                };
                let p = self.lexer.peek();
                match p.kind {
                    NormalTokenKind::Newline => {
                        self.lexer.next();
                    }
                    NormalTokenKind::EOF => {
                        todo!("generate error token");
                    }
                    _ => unreachable!("last non-argument character should be eol token. get: {p:?}"),
                }
                let mut lines = vec![];
                loop {
                    let t = self.lexer.next_ranged_tag(level);
                    match t.kind {
                        RangedTagTokenKind::VerbatimLine => {
                            // push to lines
                            lines.push(SyntaxNode::leaf(SyntaxKind::RangedTagLine, t.range));
                        }
                        RangedTagTokenKind::EndModifier => break,
                        RangedTagTokenKind::EOF => todo!("generate error token"),
                    }
                }
                if !lines.is_empty() {
                    leafs.push(SyntaxNode::inner(SyntaxKind::RangedTagLines, lines));
                }
                return SyntaxNode::inner(SyntaxKind::RangedTag, leafs);
            }
            NormalTokenKind::Special(_) => {
                todo!("parse tags")
            }
            _ => {}
        }
        let range = self.lexer.eat_line();
        return SyntaxNode::leaf(SyntaxKind::Paragraph, Range::new(start, range.end));
    }
    // TODO: replace `prefix_kind: SyntaxKind` to `kind: TagKind`
    /// parse first line of block-tag call
    /// NOTE: this does not include the trailing EOL/EOF token
    fn parse_tag_line(&mut self, prefix_kind: SyntaxKind) -> Option<Vec<SyntaxNode>> {
        let prefix = self.lexer.next();
        if !self.lexer.peek().is_identifier() {
            return None;
        };
        let prefix = SyntaxNode::leaf(prefix_kind, prefix.range);
        let mut ident_tokens = vec![];
        // get identifier
        // TODO: generate errors if there are characters not matching [a-zA-Z\-]
        loop {
            if self.lexer.peek().is_whitespace() {
                break;
            }
            let t = self.lexer.next();
            ident_tokens.push(t)
        }
        let ident = SyntaxNode::leaf(
            SyntaxKind::Identifier,
            Range::new(
                ident_tokens.first().unwrap().range.start,
                ident_tokens.last().unwrap().range.end,
            ),
        );
        let mut leafs = vec![prefix, ident];
        let ws = self.lexer.peek();
        if ws.kind == NormalTokenKind::Whitespace {
            self.lexer.next();
            let mut args = vec![];
            loop {
                let token = self.lexer.next_arg();
                let kind = match token.kind {
                    ArgumentTokenKind::Argument => SyntaxKind::Argument,
                    ArgumentTokenKind::Delimiter => SyntaxKind::ArgDelimiter,
                    ArgumentTokenKind::EOF => break,
                };
                args.push(SyntaxNode::leaf(kind, token.range))
            }
            leafs.push(SyntaxNode::inner(SyntaxKind::Arguments, args));
        }
        return Some(leafs);
    }
}

enum IncompleteNext {
    FlatBlock(FlatBlockData),
    ParagraphSegment(Range),
    EOF,
}

/// Unpacked version of flat_block node.
/// It is _unpacked_ to quickly see it as higher level object.
#[derive(Debug, PartialEq)]
pub struct FlatBlockData {
    /// field to quickly find indent type
    pub indent: Indent,
    /// field to quickly find base_block type
    pub(crate) block_kind: BaseBlockKind,
    /// all children nodes for this flat block
    pub node: SyntaxNode,
}

impl FlatBlockData {
    pub fn nodes(self) -> Vec<SyntaxNode> {
        let mut nodes = self.indent.nodes;
        nodes.push(self.node);
        nodes
    }
    pub fn to_syntax(self) -> SyntaxNode {
        if self.indent == Indent::base() {
            self.node
        } else {
            let mut nodes = self.indent.nodes;
            nodes.push(self.node);
            SyntaxNode::inner(self.indent.kind.to_syntax_item(), nodes)
        }
    }
    pub fn section_level(&self) -> usize {
        if self.indent.level > 0 {
            return usize::max_value();
        }
        match self.block_kind {
            BaseBlockKind::Heading(level) | BaseBlockKind::BlankHeading(level) => level,
            _ => usize::max_value(),
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct Indent {
    pub kind: IndentKind,
    pub level: usize,
    pub nodes: Vec<SyntaxNode>,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum IndentKind {
    Unordered,
    Ordered,
    Quote,
    Null,
}

impl Indent {
    pub fn new(kind: IndentKind, level: usize, nodes: Vec<SyntaxNode>) -> Self {
        Self { kind, level, nodes }
    }
    pub fn base() -> Self {
        Self::new(IndentKind::Null, 0, vec![])
    }
}

impl IndentKind {
    pub fn to_syntax_prefix(&self) -> SyntaxKind {
        match self {
            Self::Unordered => SyntaxKind::UnorderedPrefix,
            Self::Ordered => SyntaxKind::OrderedPrefix,
            Self::Quote => SyntaxKind::QuotePrefix,
            Self::Null => SyntaxKind::NullPrefix,
        }
    }
    pub fn to_syntax_item(&self) -> SyntaxKind {
        match self {
            Self::Unordered => SyntaxKind::UnorderedListItem,
            Self::Ordered => SyntaxKind::OrderedListItem,
            Self::Quote => SyntaxKind::QuoteItem,
            Self::Null => SyntaxKind::NullItem,
        }
    }
    pub fn to_syntax_group(&self) -> SyntaxKind {
        match self {
            Self::Unordered => SyntaxKind::UnorderedList,
            Self::Ordered => SyntaxKind::OrderedList,
            Self::Quote => SyntaxKind::Quote,
            Self::Null => SyntaxKind::Null,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum BaseBlockKind {
    Heading(usize),
    BlankHeading(usize),
    BlankLine,
    HorizontalLine,
    // TODO: add more tag types
    Tag,
    Paragraph,
}

impl BaseBlockKind {
    fn from(syntax: SyntaxKind) -> Self {
        match syntax {
            SyntaxKind::Heading(level) => Self::Heading(level),
            SyntaxKind::BlankHeading(level) => Self::BlankHeading(level),
            SyntaxKind::BlankLine => Self::BlankLine,
            SyntaxKind::HorizontalLine => Self::HorizontalLine,
            SyntaxKind::CarryoverTag | SyntaxKind::InfirmTag | SyntaxKind::RangedTag => Self::Tag,
            SyntaxKind::Paragraph => Self::Paragraph,
            _ => panic!(),
        }
    }
}
