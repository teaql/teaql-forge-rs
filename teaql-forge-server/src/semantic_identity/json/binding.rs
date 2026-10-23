//! Bind members in source order; a later duplicate cannot undo a binding error.
use super::string::JavaString;
use super::{Binding, Document, Result, BAD};
use serde::{
    de::{MapAccess, Visitor},
    Deserialize, Deserializer,
};
use serde_json::value::RawValue;
use std::fmt;

struct Object(Vec<(JavaString, Box<RawValue>)>);
impl<'de> Deserialize<'de> for Object {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        struct ObjectVisitor;
        impl<'de> Visitor<'de> for ObjectVisitor {
            type Value = Object;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("an object")
            }
            fn visit_map<A: MapAccess<'de>>(
                self,
                mut map: A,
            ) -> std::result::Result<Object, A::Error> {
                let mut fields = Vec::new();
                while let Some(pair) = map.next_entry()? {
                    fields.push(pair);
                }
                Ok(Object(fields))
            }
        }
        deserializer.deserialize_map(ObjectVisitor)
    }
}

pub(super) struct Request {
    pub document: Option<Document>,
    pub expected: i64,
    pub id: Option<JavaString>,
    pub path: Option<JavaString>,
}
pub(super) fn request(bytes: &[u8]) -> Result<Request> {
    let bytes = bytes.strip_prefix(b"\xef\xbb\xbf").unwrap_or(bytes);
    // Boot/Jackson reads one DTO without FAIL_ON_TRAILING_TOKENS. Keep trailing
    // content unread, including a second root value or malformed suffix.
    let Object(fields) =
        Object::deserialize(&mut serde_json::Deserializer::from_slice(bytes)).map_err(|_| BAD)?;
    let mut request = Request {
        document: None,
        expected: 0,
        id: None,
        path: None,
    };
    for (key, value) in fields {
        if key.is("document") {
            request.document = if value.get() == "null" {
                None
            } else {
                Some(document(&value)?)
            };
        } else if key.is("expectedRevision") {
            request.expected = number(&value, false)?;
        } else if key.is("id") {
            request.id = string(&value)?;
        } else if key.is("path") {
            request.path = string(&value)?;
        }
    }
    Ok(request)
}

fn document(raw: &RawValue) -> Result<Document> {
    let Object(fields) = serde_json::from_str(raw.get()).map_err(|_| BAD)?;
    let mut doc = Document {
        schema_version: 1,
        revision: 0,
        model_id: None,
        bindings: Vec::new(),
    };
    for (key, value) in fields {
        if key.is("schemaVersion") {
            doc.schema_version = number(&value, true)? as i32;
        } else if key.is("revision") {
            doc.revision = number(&value, false)?;
        } else if key.is("modelId") {
            doc.model_id = string(&value)?;
        } else if key.is("bindings") {
            doc.bindings = array(&value)?
                .iter()
                .map(|item| {
                    if item.get() == "null" {
                        Ok(None)
                    } else {
                        binding(item).map(Some)
                    }
                })
                .collect::<Result<_>>()?;
        }
    }
    Ok(doc)
}
fn binding(raw: &RawValue) -> Result<Binding> {
    let Object(fields) = serde_json::from_str(raw.get()).map_err(|_| BAD)?;
    let mut item = Binding {
        id: None,
        kind: None,
        parent_id: None,
        path: None,
        aliases: Vec::new(),
        status: Some("active".into()),
        derived_from: Vec::new(),
    };
    for (key, value) in fields {
        if key.is("id") {
            item.id = string(&value)?;
        } else if key.is("kind") {
            item.kind = string(&value)?;
        } else if key.is("parentId") {
            item.parent_id = string(&value)?;
        } else if key.is("path") {
            item.path = string(&value)?;
        } else if key.is("status") {
            item.status = string(&value)?;
        } else if key.is("aliases") {
            item.aliases = array(&value)?
                .iter()
                .map(|item| string(item))
                .collect::<Result<_>>()?;
        } else if key.is("derivedFrom") {
            item.derived_from = array(&value)?
                .iter()
                .map(|item| string(item))
                .collect::<Result<_>>()?;
        }
    }
    Ok(item)
}
fn array(raw: &RawValue) -> Result<Vec<Box<RawValue>>> {
    if raw.get() == "null" {
        Ok(Vec::new())
    } else {
        serde_json::from_str(raw.get()).map_err(|_| BAD)
    }
}
fn string(raw: &RawValue) -> Result<Option<JavaString>> {
    let value = raw.get();
    match value.as_bytes().first() {
        Some(b'"') => serde_json::from_str(value).map(Some).map_err(|_| BAD),
        Some(b'n') => Ok(None),
        Some(b't' | b'f' | b'-' | b'0'..=b'9') => Ok(Some(value.into())),
        _ => Err(BAD),
    }
}
fn number(raw: &RawValue, narrow: bool) -> Result<i64> {
    let value = raw.get();
    let result = if value == "null" {
        0
    } else if value.starts_with('"') {
        let text = serde_json::from_str::<JavaString>(value)
            .map_err(|_| BAD)?
            .text()
            .map_err(|_| BAD)?;
        let text = text.trim_matches(|ch| ch <= '\u{20}');
        if text.is_empty() {
            0
        } else {
            // Jackson ultimately uses Java's signed integer parser, whose
            // digits include these BMP decimal blocks. Supplementary digits
            // remain invalid because that parser reads UTF-16 chars.
            const DIGITS: &[u32] = &[
                0x30, 0x660, 0x6f0, 0x7c0, 0x966, 0x9e6, 0xa66, 0xae6, 0xb66, 0xbe6, 0xc66, 0xce6,
                0xd66, 0xde6, 0xe50, 0xed0, 0xf20, 0x1040, 0x1090, 0x17e0, 0x1810, 0x1946, 0x19d0,
                0x1a80, 0x1a90, 0x1b50, 0x1bb0, 0x1c40, 0x1c50, 0xa620, 0xa8d0, 0xa900, 0xa9d0,
                0xa9f0, 0xaa50, 0xabf0, 0xff10,
            ];
            let ascii = text
                .chars()
                .map(|ch| {
                    let code = ch as u32;
                    DIGITS
                        .iter()
                        .find(|start| code >= **start && code < **start + 10)
                        .and_then(|start| char::from_u32(u32::from(b'0') + code - start))
                        .unwrap_or(ch)
                })
                .collect::<String>();
            ascii.parse::<i64>().map_err(|_| BAD)?
        }
    } else if value.contains(['.', 'e', 'E']) {
        let value = value.parse::<f64>().map_err(|_| BAD)?;
        let (min, max) = if narrow {
            (i32::MIN as f64, i32::MAX as f64)
        } else {
            (i64::MIN as f64, i64::MAX as f64)
        };
        if !value.is_finite() || value < min || value > max {
            return Err(BAD);
        }
        value as i64
    } else {
        value.parse::<i64>().map_err(|_| BAD)?
    };
    if narrow && i32::try_from(result).is_err() {
        Err(BAD)
    } else {
        Ok(result)
    }
}
