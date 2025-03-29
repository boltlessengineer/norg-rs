use janetrs::{Janet, client::JanetClient, env::DefOptions};

fn main() {
    let text = std::fs::read("test.norg").unwrap();
    let ast = norg_rs::parser::parse(&text);

    dbg!(&ast);

    println!("original document:");
    println!("{}", String::from_utf8_lossy(&text));

    let client = {
        let mut client = JanetClient::init_with_default_env().unwrap();
        client.add_def(DefOptions::new(
            "blocks",
            Janet::tuple(ast.into_iter().collect()),
        ));
        client.add_def(DefOptions::new("lang", Janet::keyword("html".into())));
        client.run(r#"(import ./stdlib :prefix "")"#).unwrap();
        client
    };

    let res = client.run(
        r#"
        (print "exported to:" lang)
        (print (string/join (map (fn [block]
                (def res (neorg/export/block block lang nil))
                # (pp res)
                res)
            blocks)))
        "#,
    );
    dbg!(res);
}
