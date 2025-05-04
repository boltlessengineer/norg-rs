use std::collections::BTreeMap;

use janetrs::{
    Janet, JanetConversionError,
    client::JanetClient,
    env::{DefOptions, JanetEnvironment},
};
use serde::Serialize;

use crate::meta::NorgMeta;

// TODO: implement compile_janet!("path/to/janet-file.janet");
// which will marshal the janet code and expand as bytes
// see https://github.com/ianthehenry/toodle.studio/blob/da7a9a31e2f770140c2b8df824047c0eb2435bb0/src/driver.cpp#L130
static NEORG_IMAGE_EMBED: &[u8] = include_bytes!("../janet-src/stdlib.jimage");

#[derive(Debug)]
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

#[derive(Debug, Serialize)]
pub struct ExportCtx {
    pub meta: BTreeMap<String, NorgMeta>,
}

impl TryFrom<janetrs::JanetTable<'_>> for ExportCtx {
    type Error = JanetConversionError;

    fn try_from(value: janetrs::JanetTable) -> Result<Self, Self::Error> {
        let meta_value = value.get_owned(janetrs::JanetKeyword::new(b"meta"));
        let Some(meta_value) = meta_value else {
            return Ok(Self {
                meta: Default::default(),
            });
        };
        let meta = match meta_value.unwrap() {
            janetrs::TaggedJanet::Table(meta_table) => crate::meta::janetkv_to_metaobj(meta_table)?,
            janetrs::TaggedJanet::Struct(meta_struct) => {
                crate::meta::janetkv_to_metaobj(meta_struct)?
            }
            got => {
                return Err(JanetConversionError::multi_wrong_kind(
                    vec![janetrs::JanetType::Table, janetrs::JanetType::Struct],
                    got.kind(),
                ));
            }
        };
        Ok(Self { meta })
    }
}

#[derive(Debug)]
pub enum ExportError {
    ClientRunError(janetrs::client::Error),
    ResultConversionError(JanetConversionError),
}

impl From<janetrs::client::Error> for ExportError {
    fn from(value: janetrs::client::Error) -> Self {
        Self::ClientRunError(value)
    }
}

impl From<JanetConversionError> for ExportError {
    fn from(value: JanetConversionError) -> Self {
        Self::ResultConversionError(value)
    }
}

#[derive(Debug)]
pub struct Exporter {
    janet_client: JanetClient,
}

impl Exporter {
    pub fn new() -> Self {
        let client = JanetClient::init().unwrap();
        let norg_env: janetrs::JanetTable = client.unmarshal(NEORG_IMAGE_EMBED).try_into().unwrap();
        let janet_client = client.load_env(JanetEnvironment::new(norg_env));
        Self { janet_client }
    }

    pub fn with_janet<F, T>(&mut self, f: F) -> T
    where
        F: FnOnce(&mut JanetClient) -> T,
    {
        f(&mut self.janet_client)
    }

    pub fn export(
        &mut self,
        target: ExportTarget,
        ast: Vec<crate::block::NorgBlock>,
    ) -> Result<(String, ExportCtx), ExportError> {
        self.janet_client.add_def(DefOptions::new(
            "ast",
            Janet::tuple(ast.into_iter().collect()),
        ));
        self.janet_client
            .add_def(DefOptions::new("lang", Janet::keyword(target.into())));
        let res = self.janet_client.run(
            r#"
            (norg/export/doc lang ast)
        "#,
        )?;
        let janetrs::TaggedJanet::Tuple(tuple) = res.unwrap() else {
            todo!("no tuple error");
        };
        let [res, ctx] = tuple.as_ref() else {
            todo!("tuple dismatch error");
        };
        let res = res.try_unwrap::<janetrs::JanetString>()?.to_string();
        let ctx = ctx.try_unwrap::<janetrs::JanetTable>()?.try_into()?;
        Ok((res, ctx))
    }
}

impl Default for Exporter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_exporter_run_janet() {
        let mut exporter = Exporter::new();
        exporter
            .with_janet(|janet| janet.run(r#" (def test-message "hello world") "#))
            .unwrap();
        let test_message = exporter
            .with_janet(|janet| janet.run(r#" test-message "#))
            .unwrap();
        let test_message = test_message
            .try_unwrap::<janetrs::JanetString>()
            .unwrap()
            .to_string();
        assert_eq!(test_message, String::from("hello world"));
    }

    #[test]
    fn test_add_c_fn() {
        use janetrs::Janet;

        #[janetrs::janet_fn(arity(fix(1)))]
        fn chars(args: &mut [Janet]) -> Janet {
            use janetrs::JanetArgs as _;
            use janetrs::JanetType::*;
            use janetrs::{JanetTuple, TaggedJanet};

            match args.get_tagged_matches(0, &[Buffer, String]) {
                TaggedJanet::Buffer(b) => b.chars().collect::<JanetTuple>().into(),
                TaggedJanet::String(s) => s.chars().collect::<JanetTuple>().into(),
                _ => unreachable!("Already checked to be a buffer|string"),
            }
        }

        let mut exporter = Exporter::new();
        exporter.with_janet(|client| {
            client.add_c_fn(janetrs::env::CFunOptions::new(c"chars", chars_c));
        });
        let res = exporter
            .with_janet(|janet| janet.run(r#" (chars "helo") "#))
            .unwrap();
        assert_eq!(
            res,
            Janet::from(janetrs::tuple!["h", "e", "l", "o"])
        );
    }
}
