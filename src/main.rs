fn main() {
    // // let text = std::fs::read_to_string("test2.norg").unwrap();
    // // let ast = norg_syntax::parse(&text);
    // // dbg!(ast);
    // // let text = r#"asdf  adsf:*/{:$asdf*/asdf: * heading: ** heading}* *word/*"#;
    // // let text = r#"{:* * heading:* heading}"#;
    // // let text = "{word";
    // // let text = r#"{: $adf/* adf : * heading asdf: ** asdf }"#;
    // // let text = r#"word:*/bold/*:word"#;
    // let text = r#"word{}"#;
    // // let text = r#"\**bold*:word"#;
    // // let mut p = norg_syntax::parser::inline2::InlinePraser::new(text);
    // // p.next();
    // // let inline_ast = norg_syntax::parse_inline(text);
    // let inline_ast = {
    //     let mut p = norg_syntax::parser::inline3::InlineParser3::new(text);
    //     p.parse();
    //     p.finish()
    // };
    // println!("input:\n{text}");
    // dbg!(inline_ast);
    let text = r#"*word word:

* heading"#;
    // let text = r#"{:word"#;
    let ast = {
        let mut p = norg_syntax::parser::inline4::InlineParser::new(text);
        p.parse_paragraph()
    };
    println!("input:\n{text}");
    dbg!(ast);
}
