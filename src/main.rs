use janetrs::{Janet, client::JanetClient, env::DefOptions};

enum ExportTarget {
    Html,
    // Pandoc,
    // CommonMark,
    // Gfm,
}

impl Into<janetrs::JanetKeyword<'_>> for ExportTarget {
    fn into(self) -> janetrs::JanetKeyword<'static> {
        match self {
            Self::Html => janetrs::JanetKeyword::new(b"html"),
        }
    }
}

fn export(target: ExportTarget, ast: Vec<norg_rs::block::NorgBlock>) -> Result<String, ()> {
    let client = {
        let mut client = JanetClient::init_with_default_env().unwrap();
        client.add_def(DefOptions::new(
            "blocks",
            Janet::tuple(ast.into_iter().collect()),
        ));
        client.add_def(DefOptions::new("lang", Janet::keyword(target.into())));
        client.run(r#"(use ./janet-src/stdlib)"#).unwrap();
        // TODO: laod more libraries to override the default logics
        // e.g. `neorg/export/logic` should probably be provided.
        client
    };
    let res = client.run(r#"
        # custom inline tag
        (defn norg/inline-tag/gh
          [[src] &]
          [{:kind :link
            :target (string "https://github.com/" src)
            :markup [{:kind :text
                      :text src}]}])
        (put norg/inline-tag "gh" norg/inline-tag/gh)

        (neorg/export blocks lang)
        "#).or(Err(()))?;
    match res.unwrap() {
        janetrs::TaggedJanet::String(janet_string) => Ok(janet_string.to_string()),
        _ => Err(()),
    }
}

fn main() {
    let text = std::fs::read("test.norg").unwrap();
    let ast = norg_rs::parser::parse(&text);

    dbg!(&ast);

    println!("original document:");
    println!("{}", String::from_utf8_lossy(&text));

    println!("exported to:");
    let res = export(ExportTarget::Html, ast);
    println!("{}", res.unwrap());
}
