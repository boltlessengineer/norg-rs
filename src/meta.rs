use std::collections::BTreeMap;

use janetrs::{Janet, JanetConversionError, JanetString, TaggedJanet};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub enum NorgMeta {
    Nil,
    Bool(bool),
    Str(String),
    Num(f64),
    Array(Vec<NorgMeta>),
    Object(BTreeMap<String, NorgMeta>),
}

pub(crate) fn janetkv_to_metaobj(
    kv: impl IntoIterator<Item = (Janet, Janet)>,
) -> Result<BTreeMap<String, NorgMeta>, JanetConversionError> {
    let mut obj = BTreeMap::new();
    for (key, value) in kv.into_iter() {
        let key = match key.unwrap() {
            TaggedJanet::String(str) => str.to_string(),
            TaggedJanet::Keyword(keyword) => keyword.to_string(),
            _ => todo!("type error"),
        };
        let value: NorgMeta = value.try_into()?;
        obj.insert(key, value);
    }
    Ok(obj)
}

impl TryFrom<Janet> for NorgMeta {
    type Error = JanetConversionError;

    fn try_from(value: Janet) -> Result<Self, Self::Error> {
        match value.unwrap() {
            TaggedJanet::Nil => Ok(Self::Nil),
            TaggedJanet::Keyword(keyword) if keyword.as_bytes() == b"nil" => Ok(Self::Nil),
            TaggedJanet::Boolean(boolean) => Ok(Self::Bool(boolean)),
            TaggedJanet::Number(num) => Ok(Self::Num(num)),
            TaggedJanet::Buffer(buffer) => Ok(Self::Str(buffer.to_string())),
            TaggedJanet::String(string) => Ok(Self::Str(string.to_string())),
            TaggedJanet::Struct(janet_struct) => {
                let mut obj = BTreeMap::new();
                for (key, &value) in janet_struct.iter() {
                    let key = key.try_unwrap::<JanetString>()?.to_string();
                    let value: NorgMeta = value.try_into()?;
                    obj.insert(key, value);
                }
                Ok(Self::Object(obj))
            }
            TaggedJanet::Table(janet_table) => {
                let obj = janetkv_to_metaobj(janet_table)?;
                Ok(Self::Object(obj))
            }
            TaggedJanet::Tuple(janet_tuple) => {
                let mut arr = vec![];
                for &value in janet_tuple.iter() {
                    arr.push(value.try_into()?);
                }
                Ok(Self::Array(arr))
            }
            TaggedJanet::Array(janet_array) => {
                let mut arr = vec![];
                for &value in janet_array.iter() {
                    arr.push(value.try_into()?);
                }
                Ok(Self::Array(arr))
            }
            _ => todo!("error here"),
        }
    }
}
