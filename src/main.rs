fn main() {
    let text = std::fs::read("test.norg").unwrap();
    let ast = norg_rs::parser::parse(&text);

    println!("original document:");
    println!("{}", String::from_utf8_lossy(&text));

    println!("exported to:");
    let mut exporter = norg_rs::export::Exporter::new();
    let (res, ctx) = exporter
        .export(norg_rs::export::ExportTarget::Html, ast)
        .unwrap();
    println!("{}", res);
    println!("{:#?}", ctx);
}
