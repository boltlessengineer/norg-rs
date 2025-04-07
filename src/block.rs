use janetrs::{
    Janet, JanetConversionError, JanetFunction, JanetKeyword, JanetString, JanetStruct, JanetType,
    TaggedJanet,
};
use serde::{Deserialize, Serialize};

use crate::inline::{Attribute, NorgInline};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NorgBlock {
    Section {
        attrs: Vec<Attribute>,
        level: u16,
        heading: Option<Vec<NorgInline>>,
        contents: Vec<Self>,
    },
    Paragraph {
        attrs: Vec<Attribute>,
        inlines: Vec<NorgInline>,
    },
    UnorderedList {
        attrs: Vec<Attribute>,
        level: u16,
        items: Vec<ListItem>,
    },
    OrderedList {
        attrs: Vec<Attribute>,
        level: u16,
        items: Vec<ListItem>,
    },
    Quote {
        attrs: Vec<Attribute>,
        level: u16,
        items: Vec<ListItem>,
    },
    InfirmTag {
        // TODO: consider rename to attrs
        params: Option<String>,
        name: String,
    },
    // TODO: how to parse this...
    CarryoverTag {
        params: Option<String>,
        name: String,
        target: Box<Self>,
    },
    RangedTag {
        params: Option<String>,
        name: String,
        content: Vec<String>,
    },
    // TODO: do I really need this type in rust?
    #[serde(skip)]
    Embed {
        attrs: Vec<Attribute>,
        // TODO: switch to HashMap<JanetKeyword, JanetFunction> instead
        // to check if "embed" support specific target language
        export: JanetFunction<'static>,
    },
    HorizontalLine {
        attrs: Vec<Attribute>,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListItem {
    pub attrs: Vec<Attribute>,
    pub contents: Vec<NorgBlock>,
}

impl TryFrom<Janet> for NorgBlock {
    // TODO: change JanetConversionError::Other to more verbose one
    type Error = JanetConversionError;

    fn try_from(value: Janet) -> Result<Self, Self::Error> {
        let value = JanetStruct::try_from(value)?;
        let kind: JanetKeyword = value
            .get_owned(JanetKeyword::new(b"kind"))
            .ok_or(JanetConversionError::Other)?
            .try_into()?;
        let node = match kind.as_bytes() {
            b"embed" => NorgBlock::Embed {
                attrs: vec![],
                export: value
                    .get_owned(JanetKeyword::new(b"export"))
                    .ok_or(JanetConversionError::Other)?
                    .try_into()?,
            },
            b"section" => {
                let level = value
                    .get_owned(JanetKeyword::new(b"level"))
                    .ok_or(JanetConversionError::Other)?
                    .try_unwrap::<u32>()? as u16;
                let heading = value
                    .get(JanetKeyword::new(b"heading"))
                    .map(|inlines| match inlines.unwrap() {
                        TaggedJanet::Tuple(tuple) => {
                            tuple.iter().map(|&inline| inline.try_into()).collect()
                        }
                        TaggedJanet::Array(array) => {
                            array.iter().map(|&inline| inline.try_into()).collect()
                        }
                        got => Err(JanetConversionError::multi_wrong_kind(
                            vec![JanetType::Array, JanetType::Tuple],
                            got.kind(),
                        )),
                    })
                    .transpose()?;
                let contents: Vec<NorgBlock> = match value
                    .get(JanetKeyword::new(b"contents"))
                    .ok_or(JanetConversionError::Other)?
                    .unwrap()
                {
                    TaggedJanet::Tuple(blocks) => blocks
                        .iter()
                        .map(|&block| block.try_into().unwrap())
                        .collect(),
                    _ => vec![],
                };
                NorgBlock::Section {
                    attrs: vec![],
                    level,
                    heading,
                    contents,
                }
            }
            b"paragraph" => NorgBlock::Paragraph {
                attrs: vec![],
                inlines: {
                    let inlines = value
                        .get_owned(JanetKeyword::new(b"inlines"))
                        .ok_or(JanetConversionError::Other)?;
                    match inlines.unwrap() {
                        TaggedJanet::Tuple(tuple) => tuple
                            .into_iter()
                            .map(|inline: Janet| inline.try_into().unwrap())
                            .collect(),
                        TaggedJanet::Array(array) => array
                            .into_iter()
                            .map(|inline: Janet| inline.try_into().unwrap())
                            .collect(),
                        got => {
                            return Err(JanetConversionError::multi_wrong_kind(
                                vec![JanetType::Array, JanetType::Tuple],
                                got.kind(),
                            ));
                        }
                    }
                },
            },
            b"infirm-tag" => {
                let name = value
                    .get_owned(JanetKeyword::new(b"name"))
                    .ok_or(JanetConversionError::Other)?;
                let name = JanetString::try_from(name)?.to_string();
                NorgBlock::InfirmTag { params: None, name }
            }
            b"unordered-list" | b"ordered-list" | b"quote" => {
                let attrs = vec![];
                let level = value
                    .get_owned(JanetKeyword::new(b"level"))
                    .ok_or(JanetConversionError::Other)?
                    .try_unwrap::<u32>()? as u16;
                let items = value
                    .get(JanetKeyword::new(b"items"))
                    .ok_or(JanetConversionError::Other)?;
                let items: Vec<ListItem> = match items.unwrap() {
                    TaggedJanet::Tuple(items) => {
                        items.iter().map(|&item| item.try_into().unwrap()).collect()
                    }
                    TaggedJanet::Array(items) => {
                        items.iter().map(|&item| item.try_into().unwrap()).collect()
                    }
                    got => {
                        return Err(JanetConversionError::multi_wrong_kind(
                            vec![JanetType::Array, JanetType::Tuple],
                            got.kind(),
                        ));
                    }
                };
                match kind.as_bytes() {
                    b"unorderd-list" => Self::UnorderedList {
                        attrs,
                        level,
                        items,
                    },
                    b"orderd-list" => Self::OrderedList {
                        attrs,
                        level,
                        items,
                    },
                    b"quote-list" => Self::Quote {
                        attrs,
                        level,
                        items,
                    },
                    _ => unreachable!(),
                }
            }
            _ => unimplemented!("implement for kind: {kind}"),
        };
        Ok(node)
    }
}

impl Into<Janet> for ListItem {
    fn into(self) -> Janet {
        JanetStruct::builder(3)
            .put(JanetKeyword::new(b"kind"), JanetKeyword::new(b"list-item"))
            .put(
                JanetKeyword::new(b"attrs"),
                Janet::tuple(self.attrs.into_iter().collect()),
            )
            .put(
                JanetKeyword::new(b"contents"),
                Janet::tuple(self.contents.into_iter().collect()),
            )
            .finalize()
            .into()
    }
}

impl TryFrom<Janet> for ListItem {
    type Error = JanetConversionError;

    fn try_from(value: Janet) -> Result<Self, Self::Error> {
        let value: JanetStruct = value.try_into()?;
        let kind: JanetKeyword = value
            .get_owned(JanetKeyword::new(b"kind"))
            .ok_or(JanetConversionError::Other)?
            .try_into()?;
        if kind != JanetKeyword::new(b"list-item") {
            return Err(JanetConversionError::Other);
        }
        let contents: Vec<NorgBlock> = match value
            .get(JanetKeyword::new(b"contents"))
            .ok_or(JanetConversionError::Other)?
            .unwrap()
        {
            TaggedJanet::Tuple(blocks) => blocks
                .iter()
                .map(|&block| block.try_into().unwrap())
                .collect(),
            TaggedJanet::Array(blocks) => blocks
                .iter()
                .map(|&block| block.try_into().unwrap())
                .collect(),
            got => {
                return Err(JanetConversionError::multi_wrong_kind(
                    vec![JanetType::Array, JanetType::Tuple],
                    got.kind(),
                ));
            }
        };
        Ok(Self {
            // TODO: parse attrs
            attrs: vec![],
            contents,
        })
    }
}

impl Into<Janet> for NorgBlock {
    fn into(self) -> Janet {
        use NorgBlock::*;
        match self {
            Section {
                attrs,
                level,
                heading,
                contents,
            } => JanetStruct::builder(5)
                .put(JanetKeyword::new(b"kind"), JanetKeyword::new(b"section"))
                .put(
                    JanetKeyword::new(b"attrs"),
                    Janet::tuple(attrs.into_iter().collect()),
                )
                .put(JanetKeyword::new(b"level"), level as usize)
                .put(
                    JanetKeyword::new(b"heading"),
                    match heading {
                        Some(heading) => Janet::tuple(heading.into_iter().collect()),
                        None => Janet::nil(),
                    },
                )
                .put(
                    JanetKeyword::new(b"contents"),
                    Janet::tuple(contents.into_iter().collect()),
                )
                .finalize()
                .into(),
            Paragraph { attrs, inlines } => JanetStruct::builder(3)
                .put(JanetKeyword::new(b"kind"), JanetKeyword::new(b"paragraph"))
                .put(
                    JanetKeyword::new(b"attrs"),
                    Janet::tuple(attrs.into_iter().collect()),
                )
                .put(
                    JanetKeyword::new(b"inlines"),
                    Janet::tuple(inlines.into_iter().collect()),
                )
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
            Embed { attrs, export } => JanetStruct::builder(3)
                .put(JanetKeyword::new(b"kind"), JanetKeyword::new(b"embed"))
                .put(
                    JanetKeyword::new(b"attrs"),
                    Janet::tuple(attrs.into_iter().collect()),
                )
                .put(JanetKeyword::new(b"export"), export)
                .finalize()
                .into(),
            UnorderedList {
                attrs,
                level,
                items,
            } => JanetStruct::builder(4)
                .put(
                    JanetKeyword::new(b"kind"),
                    JanetKeyword::new(b"unordered-list"),
                )
                .put(
                    JanetKeyword::new(b"attrs"),
                    Janet::tuple(attrs.into_iter().collect()),
                )
                .put(JanetKeyword::new(b"level"), level as usize)
                .put(
                    JanetKeyword::new(b"items"),
                    Janet::tuple(items.into_iter().collect()),
                )
                .finalize()
                .into(),
            OrderedList {
                attrs,
                level,
                items,
            } => JanetStruct::builder(4)
                .put(
                    JanetKeyword::new(b"kind"),
                    JanetKeyword::new(b"ordered-list"),
                )
                .put(
                    JanetKeyword::new(b"attrs"),
                    Janet::tuple(attrs.into_iter().collect()),
                )
                .put(JanetKeyword::new(b"level"), level as usize)
                .put(
                    JanetKeyword::new(b"items"),
                    Janet::tuple(items.into_iter().collect()),
                )
                .finalize()
                .into(),
            Quote {
                attrs,
                level,
                items,
            } => JanetStruct::builder(4)
                .put(JanetKeyword::new(b"kind"), JanetKeyword::new(b"quote"))
                .put(
                    JanetKeyword::new(b"attrs"),
                    Janet::tuple(attrs.into_iter().collect()),
                )
                .put(JanetKeyword::new(b"level"), level as usize)
                .put(
                    JanetKeyword::new(b"items"),
                    Janet::tuple(items.into_iter().collect()),
                )
                .finalize()
                .into(),
            CarryoverTag {
                params,
                name,
                target,
            } => JanetStruct::builder(4)
                .put(
                    JanetKeyword::new(b"kind"),
                    JanetKeyword::new(b"carryover-tag"),
                )
                .put(
                    JanetKeyword::new(b"params"),
                    match params {
                        Some(params) => Janet::string(params.as_bytes().into()),
                        None => Janet::nil(),
                    },
                )
                .put(JanetKeyword::new(b"name"), name.as_str())
                .put(JanetKeyword::new(b"block"), *target)
                .finalize()
                .into(),
            HorizontalLine { attrs } => JanetStruct::builder(2)
                .put(JanetKeyword::new(b"kind"), JanetKeyword::new(b"horizontal-line"))
                .put(
                    JanetKeyword::new(b"attrs"),
                    Janet::tuple(attrs.into_iter().collect()),
                )
                .finalize()
                .into(),
        }
    }
}
