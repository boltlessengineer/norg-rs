#![allow(dead_code)]
#![allow(unused_variables)]

use janetrs::{Janet, JanetFunction, JanetKeyword, JanetStruct, TaggedJanet};

use crate::inline::NorgInline;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NorgBlock {
    Section {
        params: Option<String>,
        level: u16,
        heading: Option<Vec<NorgInline>>,
        contents: Vec<NorgBlock>,
    },
    Paragraph {
        params: Option<String>,
        inlines: Vec<NorgInline>,
    },
    // UnorderedList
    InfirmTag {
        // TODO: change this to Vec<String>
        params: Option<String>,
        name: String,
    },
    // TODO: how to parse this...
    CarryoverTag {
        params: Option<String>,
        name: String,
    },
    RangedTag {
        params: Option<String>,
        name: String,
        content: Vec<String>,
    },
    Embed {
        params: Option<String>,
        export: JanetFunction<'static>,
    },
}

impl TryFrom<Janet> for NorgBlock {
    type Error = ();

    fn try_from(value: Janet) -> Result<Self, Self::Error> {
        let TaggedJanet::Struct(value) = value.unwrap() else {
            panic!("no struct");
        };
        let kind = value.get(Janet::keyword("kind".into())).unwrap();
        let TaggedJanet::Keyword(kind) = kind.unwrap() else {
            panic!("no no kind");
        };
        let node = match kind.as_bytes() {
            b"embed" => {
                let export = value.get_owned(Janet::keyword("export".into())).unwrap();
                let TaggedJanet::Function(export) = export.unwrap() else {
                    unimplemented!("not yet");
                };
                NorgBlock::Embed {
                    params: None,
                    export,
                }
            }
            b"paragraph" => NorgBlock::Paragraph {
                params: None,
                inlines: vec![],
            },
            b"infirm-tag" => {
                let name = value.get_owned(JanetKeyword::new(b"name")).unwrap();
                let TaggedJanet::String(name) = name.unwrap() else {
                    unimplemented!();
                };
                let name = name.to_string();
                NorgBlock::InfirmTag { params: None, name }
            }
            _ => unimplemented!("implement for kind: {kind}"),
        };
        Ok(node)
    }
}

impl Into<Janet> for NorgBlock {
    fn into(self) -> Janet {
        use NorgBlock::*;
        match self {
            Section {
                params,
                level,
                heading,
                contents,
            } => JanetStruct::builder(5)
                .put(JanetKeyword::new(b"kind"), JanetKeyword::new(b"section"))
                .put(JanetKeyword::new(b"level"), level as usize)
                .put(
                    JanetKeyword::new(b"heading"),
                    match heading {
                        Some(heading) => Janet::tuple(heading.into_iter().collect()),
                        None => Janet::nil(),
                    },
                )
                .put(
                    JanetKeyword::new(b"content"),
                    Janet::tuple(contents.into_iter().collect()),
                )
                .finalize()
                .into(),
            Paragraph { params, inlines } => JanetStruct::builder(3)
                .put(JanetKeyword::new(b"kind"), JanetKeyword::new(b"paragraph"))
                .put(
                    JanetKeyword::new(b"params"),
                    match params {
                        Some(params) => Janet::string(params.as_bytes().into()),
                        None => Janet::nil(),
                    },
                )
                .put("inlines", Janet::tuple(inlines.into_iter().collect()))
                .finalize()
                .into(),
            InfirmTag { params, name } => JanetStruct::builder(3)
                .put(JanetKeyword::new(b"kind"), JanetKeyword::new(b"infirm-tag"))
                .put(
                    JanetKeyword::new(b"params"),
                    match params {
                        Some(params) => Janet::string(params.as_bytes().into()),
                        None => Janet::nil(),
                    },
                )
                .put(JanetKeyword::new(b"name"), name.as_str())
                .finalize()
                .into(),
            RangedTag {
                params,
                name,
                content,
            } => JanetStruct::builder(4)
                .put(JanetKeyword::new(b"kind"), JanetKeyword::new(b"ranged-tag"))
                .put(
                    JanetKeyword::new(b"params"),
                    match params {
                        Some(params) => Janet::string(params.as_bytes().into()),
                        None => Janet::nil(),
                    },
                )
                .put(JanetKeyword::new(b"name"), name.as_str())
                .put(
                    JanetKeyword::new(b"content"),
                    Janet::tuple(content.iter().map(|x| x.as_str()).collect()),
                )
                .finalize()
                .into(),
            Embed { params, export } => JanetStruct::builder(3)
                .put(JanetKeyword::new(b"kind"), JanetKeyword::new(b"embed"))
                .put(JanetKeyword::new(b"export"), export)
                .finalize()
                .into(),
            _ => unimplemented!(),
        }
    }
}
