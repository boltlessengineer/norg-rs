use std::collections::BTreeMap;

use janetrs::{Janet, JanetConversionError, JanetString, TaggedJanet};
use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(untagged)]
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

#[cfg(test)]
mod tests {
    use super::NorgMeta;
    use serde_json;
    use std::collections::BTreeMap;

    #[test]
    fn test_serde_norgmeta_serialization() {
        // Nil → null
        assert_eq!(serde_json::to_string(&NorgMeta::Nil).unwrap(), "null");

        // Bool → true/false
        assert_eq!(serde_json::to_string(&NorgMeta::Bool(true)).unwrap(), "true");
        assert_eq!(serde_json::to_string(&NorgMeta::Bool(false)).unwrap(), "false");

        // Str → "string"
        assert_eq!(
            serde_json::to_string(&NorgMeta::Str("hello".into())).unwrap(),
            "\"hello\""
        );

        // Num → number
        assert_eq!(serde_json::to_string(&NorgMeta::Num(3.14)).unwrap(), "3.14");

        // Array → [ … ]
        let arr = NorgMeta::Array(vec![
            NorgMeta::Num(1.0),
            NorgMeta::Bool(false),
            NorgMeta::Str("x".into()),
        ]);
        assert_eq!(
            serde_json::to_string(&arr).unwrap(),
            "[1.0,false,\"x\"]"
        );

        // Object → { … }
        let mut map = BTreeMap::new();
        map.insert("a".to_string(), NorgMeta::Nil);
        map.insert("b".to_string(), NorgMeta::Num(2.0));
        let obj = NorgMeta::Object(map);
        // BTreeMap serializes keys in sorted order
        assert_eq!(
            serde_json::to_string(&obj).unwrap(),
            "{\"a\":null,\"b\":2.0}"
        );
    }
}
