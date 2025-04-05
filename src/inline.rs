#![allow(unused_variables)]

use janetrs::{
    Janet, JanetArray, JanetConversionError, JanetKeyword, JanetString, JanetStruct, JanetTuple,
    JanetType, TaggedJanet,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Attribute {
    Key(String),
    KeyValue(String, String),
}

impl TryFrom<JanetTuple<'_>> for Attribute {
    type Error = JanetConversionError;
    fn try_from(value: JanetTuple) -> Result<Self, Self::Error> {
        todo!()
    }
}
impl TryFrom<JanetArray<'_>> for Attribute {
    type Error = JanetConversionError;
    fn try_from(value: JanetArray) -> Result<Self, Self::Error> {
        todo!()
    }
}
impl TryFrom<Janet> for Attribute {
    type Error = JanetConversionError;

    fn try_from(value: Janet) -> Result<Self, Self::Error> {
        match value.unwrap() {
            TaggedJanet::Array(array) => array.try_into(),
            TaggedJanet::Tuple(array) => array.try_into(),
            got => {
                return Err(JanetConversionError::multi_wrong_kind(
                    vec![JanetType::Array, JanetType::Tuple],
                    got.kind(),
                ));
            }
        }
    }
}

impl Into<JanetTuple<'_>> for Attribute {
    fn into(self) -> JanetTuple<'static> {
        match self {
            Self::Key(key) => janetrs::tuple![JanetString::from(key)],
            Self::KeyValue(key, val) => {
                janetrs::tuple![JanetString::from(key), JanetString::from(val)]
            }
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
    type Error = JanetConversionError;

    fn try_from(value: Janet) -> Result<Self, Self::Error> {
        let TaggedJanet::Struct(value) = value.unwrap() else {
            panic!("no struct");
        };
        let kind: JanetKeyword = value
            .get_owned(JanetKeyword::new(b"kind"))
            .ok_or(JanetConversionError::Other)?
            .try_into()?;
        match kind.as_bytes() {
            b"whitespace" => Ok(NorgInline::Whitespace),
            b"softbreak" => Ok(NorgInline::SoftBreak),
            b"text" => Ok(NorgInline::Text(
                value
                    .get_owned(JanetKeyword::new(b"text"))
                    .ok_or(JanetConversionError::Other)?
                    .try_unwrap::<JanetString>()?
                    .to_string(),
            )),
            b"special" => Ok(NorgInline::Special(
                value
                    .get_owned(JanetKeyword::new(b"text"))
                    .ok_or(JanetConversionError::Other)?
                    .try_unwrap::<JanetString>()?
                    .to_string(),
            )),
            b"escape" => Ok(NorgInline::Escape(
                value
                    .get_owned(JanetKeyword::new(b"escape"))
                    .ok_or(JanetConversionError::Other)?
                    .try_unwrap::<u32>()?
                    .try_into()
                    .unwrap(),
            )),
            b"bold" | b"italic" | b"underline" | b"strikethrough" | b"verbatim" => {
                let markup = value
                    .get(JanetKeyword::new(b"markup"))
                    .ok_or(JanetConversionError::Other)?;
                let markup = match markup.unwrap() {
                    TaggedJanet::Tuple(inlines) => inlines
                        .iter()
                        .map(|&inline| inline.try_into().unwrap())
                        .collect(),
                    TaggedJanet::Array(inlines) => inlines
                        .iter()
                        .map(|&inline| inline.try_into().unwrap())
                        .collect(),
                    got => {
                        return Err(JanetConversionError::multi_wrong_kind(
                            vec![JanetType::Array, JanetType::Tuple],
                            got.kind(),
                        ));
                    }
                };
                let attrs = value
                    .get(JanetKeyword::new(b"attrs"))
                    .ok_or(JanetConversionError::Other)?;
                let attrs = vec![];
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
                    .get_owned(JanetKeyword::new(b"name"))
                    .ok_or(JanetConversionError::Other)?
                    .try_unwrap::<JanetString>()?
                    .to_string(),
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
                    .transpose()?,
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
                        TaggedJanet::Tuple(inlines) => Some(
                            inlines
                                .iter()
                                .map(|&inline| inline.try_into().unwrap())
                                .collect(),
                        ),
                        _ => None,
                    }
                });
                let attrs = vec![];
                match kind.as_bytes() {
                    b"link" => Ok(NorgInline::Link {
                        target: target.ok_or(JanetConversionError::Other)?,
                        markup,
                        attrs,
                    }),
                    b"anchor" => Ok(NorgInline::Anchor {
                        target,
                        markup: markup.ok_or(JanetConversionError::Other)?,
                        attrs,
                    }),
                    _ => unreachable!(),
                }
            }
            _ => todo!("error here"),
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
