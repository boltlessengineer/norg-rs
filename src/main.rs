fn main() {
    let text = std::fs::read_to_string("test2.norg").unwrap();
    let ast = norg_syntax::parse(&text);
    dbg!(ast);
    // let text = r#"asdf  adsf:*/{:$asdf*/asdf: * heading: ** heading}* *word/*"#;
    // // let mut p = norg_syntax::parser::inline2::InlinePraser::new(text);
    // // p.next();
    // let inline_ast = norg_syntax::parse_inline(text);
    // println!("input:\n{text}");
    // dbg!(inline_ast);
}
