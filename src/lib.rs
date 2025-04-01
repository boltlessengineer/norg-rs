use janetrs::{client::JanetClient, env::{DefOptions, JanetEnvironment}, Janet};

pub mod block;
pub mod inline;
pub mod parser;

pub enum ExportTarget {
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

// TODO: implement compile_janet!("path/to/janet-file.janet");
// which will marshal the janet code and expand as bytes
// see https://github.com/ianthehenry/toodle.studio/blob/da7a9a31e2f770140c2b8df824047c0eb2435bb0/src/driver.cpp#L130
static NEORG_IMAGE_EMBED: &[u8] = include_bytes!("../janet-src/stdlib.jimage");

pub fn export(target: ExportTarget, ast: Vec<crate::block::NorgBlock>) -> Result<String, ()> {
    let client = JanetClient::init().unwrap();
    let norg_env: janetrs::JanetTable = client.unmarshal(NEORG_IMAGE_EMBED).try_into().unwrap();
    let mut client = client.load_env(JanetEnvironment::new(norg_env));
    client.add_def(DefOptions::new(
        "ast",
        Janet::tuple(ast.into_iter().collect()),
    ));
    client.add_def(DefOptions::new("lang", Janet::keyword(target.into())));
    let res = client.run(r#"
        (norg/export/doc lang ast)
    "#).or(Err(()))?;
    match res.unwrap() {
        janetrs::TaggedJanet::String(janet_string) => Ok(janet_string.to_string()),
        _ => Err(()),
    }
}
