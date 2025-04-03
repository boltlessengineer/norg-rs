use janetrs::{Janet, JanetFunction, JanetKeyword, JanetStruct, TaggedJanet};

use crate::inline::{Attribute, NorgInline};

#[derive(Debug, Clone, PartialEq, Eq)]
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
    },
    RangedTag {
        params: Option<String>,
        name: String,
        content: Vec<String>,
    },
    Embed {
        attrs: Vec<Attribute>,
        // TODO: switch to HashMap<JanetKeyword, JanetFunction> instead
        // to check if "embed" support specific target language
        export: JanetFunction<'static>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListItem {
    pub attrs: Vec<Attribute>,
    pub contents: Vec<NorgBlock>,
}

impl TryFrom<Janet> for NorgBlock {
    // TODO: use actual error instead
    type Error = ();

    fn try_from(value: Janet) -> Result<Self, Self::Error> {
        let TaggedJanet::Struct(value) = value.unwrap() else {
            return Err(());
        };
        let kind = value
            .get(JanetKeyword::new(b"kind"))
            .and_then(|&kind| match kind.unwrap() {
                TaggedJanet::Keyword(kind) => Some(kind),
                _ => None,
            })
            .ok_or(())?;
        let node = match kind.as_bytes() {
            b"embed" => {
                let export = value.get_owned(Janet::keyword("export".into())).unwrap();
                let TaggedJanet::Function(export) = export.unwrap() else {
                    return Err(());
                };
                NorgBlock::Embed {
                    attrs: vec![],
                    export,
                }
            }
            b"section" => {
                let level = value.get_owned(JanetKeyword::new(b"level")).unwrap();
                let TaggedJanet::Number(level) = level.unwrap() else {
                    return Err(());
                };
                let level = level as u16;
                let heading: Option<Vec<NorgInline>> = value
                    .get(JanetKeyword::new(b"heading"))
                    .and_then(|inlines| match inlines.unwrap() {
                        TaggedJanet::Tuple(inlines) => Some(
                            inlines
                                .iter()
                                .map(|&inline| inline.try_into().unwrap())
                                .collect(),
                        ),
                        _ => None,
                    });
                let contents: Vec<NorgBlock> = match value
                    .get(JanetKeyword::new(b"contents"))
                    .ok_or(())?
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
                attrs: todo!(),
                inlines: value
                    .get(JanetKeyword::new(b"inlines"))
                    .and_then(|inlines| match inlines.unwrap() {
                        TaggedJanet::Tuple(inlines) => Some(
                            inlines
                                .iter()
                                .map(|&inline| inline.try_into().unwrap())
                                .collect(),
                        ),
                        _ => None,
                    })
                    .ok_or(())?,
            },
            b"infirm-tag" => {
                let name = value.get_owned(JanetKeyword::new(b"name")).unwrap();
                let TaggedJanet::String(name) = name.unwrap() else {
                    return Err(());
                };
                let name = name.to_string();
                NorgBlock::InfirmTag { params: None, name }
            }
            b"unordered-list" | b"ordered-list" | b"quote" => {
                let level = value.get_owned(JanetKeyword::new(b"level")).unwrap();
                let TaggedJanet::Number(level) = level.unwrap() else {
                    return Err(());
                };
                let level = level as u16;
                let items: Vec<ListItem> =
                    match value.get(JanetKeyword::new(b"items")).ok_or(())?.unwrap() {
                        TaggedJanet::Tuple(items) => {
                            items.iter().map(|&item| item.try_into().unwrap()).collect()
                        }
                        _ => vec![],
                    };
                let attrs = vec![];
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
    // TODO: use actual error instead
    type Error = ();

    fn try_from(value: Janet) -> Result<Self, Self::Error> {
        let value: JanetStruct = value.try_unwrap().or(Err(()))?;
        let kind = value
            .get(JanetKeyword::new(b"kind"))
            .and_then(|&kind| match kind.unwrap() {
                TaggedJanet::Keyword(kind) => Some(kind),
                _ => None,
            })
            .ok_or(())?;
        if kind != JanetKeyword::new(b"list-item") {
            return Err(());
        }
        let contents: Vec<NorgBlock> = match value
            .get(JanetKeyword::new(b"contents"))
            .ok_or(())?
            .unwrap()
        {
            TaggedJanet::Tuple(blocks) => blocks
                .iter()
                .map(|&block| block.try_into().unwrap())
                .collect(),
            _ => vec![],
        };
        Ok(Self {
            attrs: todo!(),
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
                .put(
                    JanetKeyword::new(b"kind"),
                    JanetKeyword::new(b"quote"),
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
            _ => unimplemented!(),
        }
    }
}
