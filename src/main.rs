fn main() {
    // let text = std::fs::read_to_string("test2.norg").unwrap();
    // let ast = norg_syntax::parse(&text);
    // dbg!(ast);
    let text = "asdf:[asdf]";
    let inline_ast = norg_syntax::parse_inline(text);
    println!("from: {text}");
    dbg!(inline_ast);
}
