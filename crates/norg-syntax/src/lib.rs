use node::SyntaxNode;

pub mod node;
pub mod parser;

pub fn parse(text: &str) -> SyntaxNode {
    let mut fp = parser::flat_parser::FlatParser::new(text);
    let flat_blocks = fp.parse();
    let mut p = parser::parser::Parser::new(flat_blocks.into_iter());
    p.parse()
}

pub fn parse_inline(text: &str) -> Vec<SyntaxNode> {
    let p = parser::inline::InlineParser::new(text);
    p.parse()
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Default)]
pub struct Range {
    pub start: usize,
    pub end: usize,
}

impl Range {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }
    pub fn len(&self) -> usize {
        self.end - self.start
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}
