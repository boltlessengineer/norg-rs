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
    let mut p = parser::inline2::InlineParser2::new(text);
    p.next();
    p.next();
    p.next();
    p.next();
    p.next();
    p.next();
    p.next();
    p.next();
    p.next();
    p.next();
    p.next();
    p.next();
    p.next();
    p.finish()
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
    pub fn point(point: usize) -> Self {
        Self::new(point, point)
    }
    pub fn len(&self) -> usize {
        self.end - self.start
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}
