use norg_rs::{block::NorgBlock, parser2::{Scanner, flatnodes_to_ao}};

fn main() {
    let text = std::fs::read("test2.norg").unwrap();
    let text = &String::from_utf8_lossy(&text);
    let flat_blocks = Scanner::new(text).parse_flat();
    dbg!(&flat_blocks);
    let mut ast: Vec<NorgBlock> = vec![];
    let mut iter = flat_blocks.into_iter().peekable();
    while let Some(ao) = flatnodes_to_ao(&mut iter, text, 0) {
        ast.push(ao);
    }
    dbg!(ast);
}
