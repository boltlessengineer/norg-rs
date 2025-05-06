use norg_rs::export::{ExportTarget, Exporter};

fn main() {
    let text = std::fs::read("test2.norg").unwrap();
    let ast = norg_rs::parser::parse(&text);
    let mut exporter = Exporter::new();
    let (res, _meta) = exporter.export(ExportTarget::Html, ast, None).unwrap();
    println!("{res}");
}
