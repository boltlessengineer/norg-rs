use janetrs::{client::JanetClient, env::{DefOptions, JanetEnvironment}, Janet};

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

// TODO: implement compile_janet!("path/to/janet-file.janet");
// which will marshal the janet code and expand as bytes
// see https://github.com/ianthehenry/toodle.studio/blob/da7a9a31e2f770140c2b8df824047c0eb2435bb0/src/driver.cpp#L130
static NEORG_IMAGE_EMBED: &[u8] = include_bytes!("../janet-src/stdlib.jimage");

fn export(target: ExportTarget, ast: Vec<norg_rs::block::NorgBlock>) -> Result<String, ()> {
    let client = JanetClient::init().unwrap();
    let norg_env: janetrs::JanetTable = client.unmarshal(NEORG_IMAGE_EMBED).try_into().unwrap();
    let mut client = client.load_env(JanetEnvironment::new(norg_env));
    client.add_def(DefOptions::new(
        "ast",
        Janet::tuple(ast.into_iter().collect()),
    ));
    client.add_def(DefOptions::new("lang", Janet::keyword(target.into())));
    let res = client.run(r#"
        # (put norg/ast/tag
        #      "\\gh"
        #      (fn [[src] markup]
        #          [{:kind :link
        #            :target (string "https://github.com/" src)
        #            :markup [{:kind :text
        #                      :text src}]}]))
        (norg/export/doc lang ast)
    "#).or(Err(()))?;
    match res.unwrap() {
        janetrs::TaggedJanet::String(janet_string) => Ok(janet_string.to_string()),
        _ => Err(()),
    }
}

// HACK: extracted version of JanetClient::run(self, code) to run with different environment
fn client_run(
    env: *mut janetrs::lowlevel::JanetTable,
    code: impl AsRef<str>,
) -> Result<Janet, janetrs::client::Error> {
    let code = code.as_ref().as_bytes();
    unsafe {
        use janetrs::lowlevel::*;
        let mut out = janet_wrap_nil();
        let res = janet_dobytes(
            env,
            code.as_ptr(),
            code.len() as i32,
            c"main".as_ptr(),
            &mut out,
        );
        match res {
            0x01 => Err(janetrs::client::Error::RunError),
            0x02 => Err(janetrs::client::Error::CompileError),
            0x04 => Err(janetrs::client::Error::ParseError),
            _ => Ok(out),
        }
    }
    .map(Janet::from)
}

// HACK: implement as JanetClient::janet_unmarshal(self, image: impl AsRef<[u8]>) -> Janet
fn client_unmarshal(image: impl AsRef<[u8]>) -> Janet {
    let image = image.as_ref();
    let marsh_out = unsafe {
        use janetrs::lowlevel::*;
        let env = janet_core_env(std::ptr::null_mut());
        let lookup = janet_env_lookup(env);
        janet_unmarshal(
            image.as_ptr(),
            image.len(),
            0,
            lookup,
            std::ptr::null_mut(),
        )
    };
    Janet::from(marsh_out)
}

fn test_unmarshal_env() -> Result<String, ()> {
    // TODO: patch janetrs to implement JanetClient::with_env(env);
    let _client = JanetClient::init().unwrap();
    let mut norg_env: janetrs::JanetTable = client_unmarshal(NEORG_IMAGE_EMBED).try_into().unwrap();
    let res = client_run(norg_env.as_mut_raw(), r#"
        (string message "asdf")
    "#).or(Err(()))?;
    match res.unwrap() {
        janetrs::TaggedJanet::String(janet_string) => Ok(janet_string.to_string()),
        _ => Err(()),
    }
}

fn main() {
    let text = std::fs::read("test.norg").unwrap();
    let ast = norg_rs::parser::parse(&text);

    // dbg!(&ast);

    println!("original document:");
    println!("{}", String::from_utf8_lossy(&text));

    println!("exported to:");
    let res = export(ExportTarget::Html, ast);
    // let res = test_unmarshal_env();
    println!("{}", res.unwrap());
}
