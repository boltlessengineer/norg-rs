#![allow(dead_code)]
// TODO: implement JanetIterable, JanetKVIterable, JanetStringKind to make
// dealing with tuple/array, struct/table, string/buffer easier

use janetrs::{Janet, JanetArray, JanetConversionError, JanetTuple, JanetType, TaggedJanet};

pub(crate) enum JanetIterable<'a> {
    Array(JanetArray<'a>),
    Tuple(JanetTuple<'a>),
}

impl TryFrom<Janet> for JanetIterable<'_> {
    type Error = JanetConversionError;

    fn try_from(value: Janet) -> Result<Self, Self::Error> {
        match value.unwrap() {
            TaggedJanet::Array(array) => Ok(Self::Array(array)),
            TaggedJanet::Tuple(tuple) => Ok(Self::Tuple(tuple)),
            got => Err(JanetConversionError::multi_wrong_kind(
                vec![JanetType::Array, JanetType::Tuple],
                got.kind(),
            )),
        }
    }
}

// impl IntoIterator for JanetIterable<'_> {
//     type Item = Janet;
//     type IntoIter = JanetIterator;
//
//     fn into_iter(self) -> Self::IntoIter {
//         todo!()
//     }
// }
