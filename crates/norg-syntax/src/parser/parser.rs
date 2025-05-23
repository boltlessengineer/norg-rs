use super::flat_parser::{BaseBlockKind, FlatBlockData, IndentKind};

use crate::node::{SyntaxKind, SyntaxNode};

pub struct Parser<I>
where
    I: Iterator<Item = FlatBlockData>,
{
    current: Option<FlatBlockData>,
    flat_iter: I,
}

impl<I: Iterator<Item = FlatBlockData>> Parser<I> {
    pub fn new(mut flat_iter: I) -> Self {
        let current = flat_iter.next();
        Self { flat_iter, current }
    }
    pub fn parse(&mut self) -> SyntaxNode {
        let mut doc_content = vec![];
        while let Some(node) = self.parse_block() {
            doc_content.push(node);
        }
        SyntaxNode::inner(SyntaxKind::Document, doc_content)
    }
    fn parse_block(&mut self) -> Option<SyntaxNode> {
        let current = self.current.as_ref()?;
        Some(match (current.indent.level, current.block_kind.clone()) {
            // section
            (0, BaseBlockKind::Heading(level)) => {
                let current = self.flat_next().unwrap();
                let mut children = vec![current.node];
                while let Some(block) = self.parse_section_content(level) {
                    children.push(block);
                }
                SyntaxNode::inner(SyntaxKind::Section, children)
            }
            // indent_group
            (1.., _) => {
                let current = self.flat_next().unwrap();
                let base_level = current.indent.level;
                let base_kind = current.indent.kind;
                let mut items = vec![current.nodes()];
                while let Some((indent_kind, flat)) = self.flat_next_indented(base_level, base_kind)
                {
                    match indent_kind {
                        IndentedNextKind::Item => {
                            // push to group items
                            items.push(flat.nodes());
                        }
                        IndentedNextKind::ItemContent => {
                            // finalize to SyntaxNode & push it to last Item
                            let last = items.last_mut().unwrap();
                            last.push(flat.to_syntax());
                        }
                    }
                }
                // collect items to single SyntaxNode
                SyntaxNode::inner(
                    base_kind.to_syntax_group(),
                    items.into_iter().map(|nodes| {
                        SyntaxNode::inner(base_kind.to_syntax_item(), nodes)
                    }).collect(),
                )
            }
            // unindented base blocks
            (0, _) => {
                let current = self.flat_next().unwrap();
                current.node
            }
        })
    }
    fn parse_section_content(&mut self, level: usize) -> Option<SyntaxNode> {
        if self.current.as_ref()?.section_level() <= level {
            return None;
        }
        self.parse_block()
    }

    fn flat_next(&mut self) -> Option<FlatBlockData> {
        let prev = self.current.take();
        self.current = self.flat_iter.next();
        prev
    }

    fn flat_next_indented(
        &mut self,
        base_level: usize,
        base_kind: IndentKind,
    ) -> Option<(IndentedNextKind, FlatBlockData)> {
        let flat = self.current.as_ref()?;
        if flat.indent.level > base_level {
            let flat = self.flat_next()?;
            return Some((IndentedNextKind::ItemContent, flat));
        }
        if flat.indent.level == base_level {
            let flat = self.flat_next()?;
            if flat.indent.kind == IndentKind::Null {
                return Some((IndentedNextKind::ItemContent, flat));
            }
            if flat.indent.kind == base_kind {
                return Some((IndentedNextKind::Item, flat));
            }
        }
        None
    }
}

enum IndentedNextKind {
    Item,
    ItemContent,
}
