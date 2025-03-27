#![allow(unused_variables)]

use janetrs::Janet;

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
    // TODO: verbatim should have different content type
    Verbatim(Vec<Self>),
    Macro {
        name: String,
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
}

impl Into<Janet> for NorgInline {
    fn into(self) -> Janet {
        use crate::inline::NorgInline::*;
        match self {
            Text(text) => Janet::string(text.into()),
            Special(text) => todo!(),
            Escape(c) => todo!(),
            Whitespace => todo!(),
            SoftBreak => Janet::string("\n".into()),
            Bold(vec) => todo!(),
            Italic(vec) => todo!(),
            Underline(vec) => todo!(),
            Strikethrough(vec) => todo!(),
            Verbatim(vec) => todo!(),
            Macro { name } => todo!(),
            Link { target, markup } => todo!(),
            Anchor { target, markup } => todo!(),
        }
    }
}
