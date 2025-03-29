#![allow(unused_variables)]

use janetrs::{Janet, JanetKeyword, JanetString, JanetStruct, TaggedJanet};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum NorgInline {
    Text(String),
    Special(String),
    Escape(char),
    Whitespace,
    SoftBreak,
    Bold(Vec<Self>),
    Italic(Vec<Self>),
    Underline(Vec<Self>),
    Strikethrough(Vec<Self>),
    Verbatim(Vec<Self>),
    // TODO: rename this to "tag"
    Macro {
        name: String,
        // markup: Option<Vec<NorgInline>>,
        attrs: Option<Vec<String>>,
        // TODO: add attributes and markup parameter
    },
    Link {
        target: String,
        markup: Option<Vec<Self>>,
    },
    Anchor {
        target: Option<String>,
        markup: Vec<Self>,
    },
    // TODO: embed
}

impl TryFrom<Janet> for NorgInline {
    // TODO: use actual error instead
    type Error = ();

    fn try_from(value: Janet) -> Result<Self, Self::Error> {
        let TaggedJanet::Struct(value) = value.unwrap() else {
            panic!("no struct");
        };
        let kind = value
            .get(JanetKeyword::new(b"kind"))
            .and_then(|&kind| match kind.unwrap() {
                TaggedJanet::Keyword(kind) => Some(kind),
                _ => None,
            })
            .ok_or(())?;
        match kind.as_bytes() {
            b"whitespace" => Ok(NorgInline::Whitespace),
            b"softbreak" => Ok(NorgInline::SoftBreak),
            b"text" => Ok(NorgInline::Text(
                value
                    .get(JanetKeyword::new(b"text"))
                    .and_then(|content| match content.unwrap() {
                        TaggedJanet::String(content) => Some(content.to_string()),
                        _ => None,
                    })
                    .ok_or(())?,
            )),
            b"special" => Ok(NorgInline::Special(
                value
                    .get(JanetKeyword::new(b"special"))
                    .and_then(|content| match content.unwrap() {
                        TaggedJanet::String(content) => Some(content.to_string()),
                        _ => None,
                    })
                    .ok_or(())?,
            )),
            b"escape" => Ok(NorgInline::Escape(
                value
                    .get(JanetKeyword::new(b"escape"))
                    .and_then(|content| match content.unwrap() {
                        TaggedJanet::Number(number) => Some(char::from_u32(number as u32).unwrap()),
                        _ => None,
                    })
                    .ok_or(())?,
            )),
            b"bold" | b"italic" | b"underline" | b"strikethrough" | b"verbatim" => {
                let markup = value
                    .get(JanetKeyword::new(b"markup"))
                    .and_then(|inlines| match inlines.unwrap() {
                        TaggedJanet::Array(inlines) => Some(
                            inlines
                                .iter()
                                .map(|&inline| inline.try_into().unwrap())
                                .collect(),
                        ),
                        _ => None,
                    })
                    .ok_or(())?;
                match kind.as_bytes() {
                    b"bold" => Ok(NorgInline::Bold(markup)),
                    b"italic" => Ok(NorgInline::Italic(markup)),
                    b"underline" => Ok(NorgInline::Underline(markup)),
                    b"strikethrough" => Ok(NorgInline::Strikethrough(markup)),
                    b"verbatim" => Ok(NorgInline::Verbatim(markup)),
                    _ => unreachable!(),
                }
            }
            b"macro" => Ok(NorgInline::Macro {
                name: value
                    .get(JanetKeyword::new(b"name"))
                    .and_then(|name| match name.unwrap() {
                        TaggedJanet::String(name) => Some(name.to_string()),
                        _ => None,
                    })
                    .ok_or(())?,
                attrs: value
                    .get(JanetKeyword::new(b"attrs"))
                    .and_then(|attrs| match attrs.unwrap() {
                        TaggedJanet::Tuple(attrs) => Some(Some(attrs)),
                        TaggedJanet::Nil => Some(None),
                        _ => None,
                    })
                    .ok_or(())?
                    .map(|attrs| {
                        attrs
                            .iter()
                            .map(|&attr| match attr.unwrap() {
                                TaggedJanet::String(attr) => attr.to_string(),
                                _ => todo!("error here"),
                            })
                            .collect()
                    }),
            }),
            b"link" | b"anchor" => {
                let target =
                    value.get(JanetKeyword::new(b"target")).and_then(|target| {
                        match target.unwrap() {
                            TaggedJanet::String(target) => Some(target.to_string()),
                            _ => None,
                        }
                    });
                let markup = value.get(JanetKeyword::new(b"markup")).and_then(|inlines| {
                    match inlines.unwrap() {
                        TaggedJanet::Array(inlines) => Some(
                            inlines
                                .iter()
                                .map(|&inline| inline.try_into().unwrap())
                                .collect(),
                        ),
                        _ => None,
                    }
                });
                match kind.as_bytes() {
                    b"link" => Ok(NorgInline::Link {
                        target: target.ok_or(())?,
                        markup,
                    }),
                    b"anchor" => Ok(NorgInline::Anchor {
                        target,
                        markup: markup.ok_or(())?,
                    }),
                    _ => unreachable!(),
                }
            }
            _ => Err(()),
        }
    }
}

impl Into<Janet> for NorgInline {
    fn into(self) -> Janet {
        use crate::inline::NorgInline::*;
        let st = match self {
            Whitespace => JanetStruct::builder(1)
                .put(JanetKeyword::new(b"kind"), JanetKeyword::new(b"whitespace"))
                .finalize(),
            SoftBreak => JanetStruct::builder(1)
                .put(JanetKeyword::new(b"kind"), JanetKeyword::new(b"softbreak"))
                .finalize(),
            Text(text) => JanetStruct::builder(2)
                .put(JanetKeyword::new(b"kind"), JanetKeyword::new(b"text"))
                .put(JanetKeyword::new(b"text"), JanetString::new(&text))
                .finalize(),
            Special(text) => JanetStruct::builder(2)
                .put(JanetKeyword::new(b"kind"), JanetKeyword::new(b"special"))
                .put(JanetKeyword::new(b"special"), JanetString::new(&text))
                .finalize(),
            Escape(c) => JanetStruct::builder(2)
                .put(JanetKeyword::new(b"kind"), JanetKeyword::new(b"escape"))
                .put(JanetKeyword::new(b"escape"), c)
                .finalize(),
            Bold(markup) => JanetStruct::builder(2)
                .put(JanetKeyword::new(b"kind"), JanetKeyword::new(b"bold"))
                .put(
                    JanetKeyword::new(b"markup"),
                    Janet::tuple(markup.into_iter().collect()),
                )
                .finalize(),
            Italic(markup) => JanetStruct::builder(2)
                .put(JanetKeyword::new(b"kind"), JanetKeyword::new(b"italic"))
                .put(
                    JanetKeyword::new(b"markup"),
                    Janet::tuple(markup.into_iter().collect()),
                )
                .finalize(),
            Underline(markup) => JanetStruct::builder(2)
                .put(JanetKeyword::new(b"kind"), JanetKeyword::new(b"underline"))
                .put(
                    JanetKeyword::new(b"markup"),
                    Janet::tuple(markup.into_iter().collect()),
                )
                .finalize(),
            Strikethrough(markup) => JanetStruct::builder(2)
                .put(
                    JanetKeyword::new(b"kind"),
                    JanetKeyword::new(b"strikethrough"),
                )
                .put(
                    JanetKeyword::new(b"markup"),
                    Janet::tuple(markup.into_iter().collect()),
                )
                .finalize(),
            Verbatim(markup) => JanetStruct::builder(2)
                .put(JanetKeyword::new(b"kind"), JanetKeyword::new(b"verbatim"))
                .put(
                    JanetKeyword::new(b"markup"),
                    Janet::tuple(markup.into_iter().collect()),
                )
                .finalize(),
            Macro { name, attrs } => JanetStruct::builder(3)
                .put(JanetKeyword::new(b"kind"), JanetKeyword::new(b"macro"))
                .put(JanetKeyword::new(b"name"), JanetString::new(&name))
                .put(
                    JanetKeyword::new(b"attrs"),
                    match attrs {
                        Some(attrs) => Janet::tuple(attrs.iter().map(|x| x.as_str()).collect()),
                        None => Janet::nil(),
                    },
                )
                .finalize(),
            Anchor { target, markup } => JanetStruct::builder(2)
                .put(JanetKeyword::new(b"kind"), JanetKeyword::new(b"macro"))
                .put(
                    JanetKeyword::new(b"markup"),
                    Janet::tuple(markup.into_iter().collect()),
                )
                .put(
                    JanetKeyword::new(b"target"),
                    match target {
                        Some(target) => Janet::string(target.into()),
                        None => Janet::nil(),
                    },
                )
                .finalize(),
            Link { target, markup } => JanetStruct::builder(2)
                .put(JanetKeyword::new(b"kind"), JanetKeyword::new(b"macro"))
                .put(JanetKeyword::new(b"target"), Janet::string(target.into()))
                .put(
                    JanetKeyword::new(b"markup"),
                    match markup {
                        Some(markup) => Janet::tuple(markup.into_iter().collect()),
                        None => Janet::nil(),
                    },
                )
                .finalize(),
        };
        st.into()
    }
}
