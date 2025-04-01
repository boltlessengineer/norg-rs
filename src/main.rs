fn main() {
    let text = std::fs::read("test.norg").unwrap();
    let ast = norg_rs::parser::parse(&text);

    println!("original document:");
    println!("{}", String::from_utf8_lossy(&text));

    println!("exported to:");
    let res = norg_rs::export(norg_rs::ExportTarget::Html, ast);
    println!("{}", res.unwrap());
}
