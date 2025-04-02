#![allow(unused_variables)]

use janetrs::{Janet, JanetKeyword, JanetString, JanetStruct, JanetTuple, TaggedJanet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Attribute {
    Key(String),
    KeyValue(String, String),
}

impl Into<JanetTuple<'_>> for Attribute {
    fn into(self) -> JanetTuple<'static> {
        match self {
            Self::Key(key) => JanetTuple::builder(2)
                .put(JanetString::from(key))
                .finalize(),
            Self::KeyValue(key, val) => JanetTuple::builder(2)
                .put(JanetString::from(key))
                .put(JanetString::from(val))
                .finalize(),
        }
    }
}

impl Into<Janet> for Attribute {
    fn into(self) -> Janet {
        Janet::tuple(self.into())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NorgInline {
    Text(String),
    Special(String),
    Escape(char),
    Whitespace,
    SoftBreak,
    Bold {
        markup: Vec<Self>,
        attrs: Vec<Attribute>,
    },
    Italic {
        markup: Vec<Self>,
        attrs: Vec<Attribute>,
    },
    Underline {
        markup: Vec<Self>,
        attrs: Vec<Attribute>,
    },
    Strikethrough {
        markup: Vec<Self>,
        attrs: Vec<Attribute>,
    },
    Verbatim {
        markup: Vec<Self>,
        attrs: Vec<Attribute>,
    },
    // TODO: rename this to "InlineTag"
    Macro {
        name: String,
        // markup: Option<Vec<NorgInline>>,
        attrs: Option<Vec<String>>,
        // TODO: add attributes and markup parameter
    },
    Link {
        target: String,
        markup: Option<Vec<Self>>,
        attrs: Vec<Attribute>,
    },
    Anchor {
        target: Option<String>,
        markup: Vec<Self>,
        attrs: Vec<Attribute>,
    },
    // TODO: embed
}

// IF abstract objects are janet abstact type
// - no need to serialize
// - have to implement method to get all properties
//
// IF abstract objects can be represented in janet struct type
// - need to implement serializing logic for EVERY objects

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
                let attrs = todo!();
                match kind.as_bytes() {
                    b"bold" => Ok(NorgInline::Bold { markup, attrs }),
                    b"italic" => Ok(NorgInline::Italic { markup, attrs }),
                    b"underline" => Ok(NorgInline::Underline { markup, attrs }),
                    b"strikethrough" => Ok(NorgInline::Strikethrough { markup, attrs }),
                    b"verbatim" => Ok(NorgInline::Verbatim { markup, attrs }),
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
                    .and_then(|attr| match attr.unwrap() {
                        TaggedJanet::Tuple(attr) => Some(
                            attr.iter()
                                .map(|&attr| {
                                    let attr =
                                        attr.try_unwrap().map(|attr: JanetString| attr.to_string());
                                    attr
                                })
                                .collect(),
                        ),
                        _ => None,
                    })
                    .transpose()
                    .or(Err(()))?,
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
                let attrs = todo!();
                match kind.as_bytes() {
                    b"link" => Ok(NorgInline::Link {
                        target: target.ok_or(())?,
                        markup,
                        attrs,
                    }),
                    b"anchor" => Ok(NorgInline::Anchor {
                        target,
                        markup: markup.ok_or(())?,
                        attrs,
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
            Bold { markup, attrs } => JanetStruct::builder(3)
                .put(JanetKeyword::new(b"kind"), JanetKeyword::new(b"bold"))
                .put(
                    JanetKeyword::new(b"markup"),
                    Janet::tuple(markup.into_iter().collect()),
                )
                .put(
                    JanetKeyword::new(b"attrs"),
                    Janet::tuple(attrs.into_iter().collect()),
                )
                .finalize(),
            Italic { markup, attrs } => JanetStruct::builder(3)
                .put(JanetKeyword::new(b"kind"), JanetKeyword::new(b"italic"))
                .put(
                    JanetKeyword::new(b"markup"),
                    Janet::tuple(markup.into_iter().collect()),
                )
                .put(
                    JanetKeyword::new(b"attrs"),
                    Janet::tuple(attrs.into_iter().collect()),
                )
                .finalize(),
            Underline { markup, attrs } => JanetStruct::builder(3)
                .put(JanetKeyword::new(b"kind"), JanetKeyword::new(b"underline"))
                .put(
                    JanetKeyword::new(b"markup"),
                    Janet::tuple(markup.into_iter().collect()),
                )
                .put(
                    JanetKeyword::new(b"attrs"),
                    Janet::tuple(attrs.into_iter().collect()),
                )
                .finalize(),
            Strikethrough { markup, attrs } => JanetStruct::builder(3)
                .put(
                    JanetKeyword::new(b"kind"),
                    JanetKeyword::new(b"strikethrough"),
                )
                .put(
                    JanetKeyword::new(b"markup"),
                    Janet::tuple(markup.into_iter().collect()),
                )
                .put(
                    JanetKeyword::new(b"attrs"),
                    Janet::tuple(attrs.into_iter().collect()),
                )
                .finalize(),
            Verbatim { markup, attrs } => JanetStruct::builder(3)
                .put(JanetKeyword::new(b"kind"), JanetKeyword::new(b"verbatim"))
                .put(
                    JanetKeyword::new(b"markup"),
                    Janet::tuple(markup.into_iter().collect()),
                )
                .put(
                    JanetKeyword::new(b"attrs"),
                    Janet::tuple(attrs.into_iter().collect()),
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
            Anchor {
                target,
                markup,
                attrs,
            } => JanetStruct::builder(4)
                .put(JanetKeyword::new(b"kind"), JanetKeyword::new(b"anchor"))
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
                .put(
                    JanetKeyword::new(b"attrs"),
                    Janet::tuple(attrs.into_iter().collect()),
                )
                .finalize(),
            Link {
                target,
                markup,
                attrs,
            } => JanetStruct::builder(4)
                .put(JanetKeyword::new(b"kind"), JanetKeyword::new(b"link"))
                .put(JanetKeyword::new(b"target"), Janet::string(target.into()))
                .put(
                    JanetKeyword::new(b"markup"),
                    match markup {
                        Some(markup) => Janet::tuple(markup.into_iter().collect()),
                        None => Janet::nil(),
                    },
                )
                .put(
                    JanetKeyword::new(b"attrs"),
                    Janet::tuple(attrs.into_iter().collect()),
                )
                .finalize(),
        };
        st.into()
    }
}
