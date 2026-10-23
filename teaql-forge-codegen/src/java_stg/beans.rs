//! Stable Java 25 Class metadata and ObjectModelAdaptor wrapper properties.
use super::*;
use std::sync::OnceLock;

fn metadata() -> &'static Value {
    static DATA: OnceLock<Value> = OnceLock::new();
    DATA.get_or_init(|| {
        let mut data:Value=serde_json::from_str(include_str!("../data/java-bean-classes-25.json")).unwrap();
        let docs:Value=serde_json::from_str(include_str!("../data/doc-bean-classes-25.json")).unwrap();
        for (category,values) in docs.as_object().unwrap() {
            for (name,value) in values.as_object().unwrap() {
                assert!(data[category].as_object_mut().unwrap().insert(name.clone(),value.clone()).is_none());
            }
        }
        data
    })
}

pub(super) fn class(name: &str) -> Value {
    let data = &metadata()["classes"][name];
    if data.is_null() {
        return Value::Null;
    }
    let prefix = if data["primitive"] == true {
        ""
    } else if data["interface"] == true {
        "interface "
    } else {
        "class "
    };
    serde_json::json!({"__stg_java_type":name,"__string":format!("{prefix}{name}")})
}

pub(super) fn byte(value: i8) -> Value {
    serde_json::json!({"__stg_java_box":"java.lang.Byte","__string":value.to_string()})
}

/// String.getBytes() uses the JVM's default charset, which may be changed by
/// file.encoding. Keep the replacement byte used by Java's String encoder.
fn default_charset_bytes(text: &Text) -> Vec<u8> {
    let name = crate::process_properties::property("file.encoding")
        .unwrap_or_else(|| "UTF-8".into());
    let string = text.utf8();
    let upper = name.to_ascii_uppercase();
    match upper.as_str() {
        // ISO-2022-JP is a WHATWG label, but the pinned JDK does not expose
        // it as a default Charset and falls back to UTF-8.
        "UTF-8" | "UTF8" | "COMPAT" | "ISO-2022-JP" => return string.into_bytes(),
        "CESU-8" => {
            let mut bytes = Vec::with_capacity(text.0.len().saturating_mul(3));
            let mut units = text.0.iter().copied().peekable();
            while let Some(unit) = units.next() {
                let paired_high = (0xd800..=0xdbff).contains(&unit)
                    && units.peek().is_some_and(|next| (0xdc00..=0xdfff).contains(next));
                if (0xd800..=0xdfff).contains(&unit) && !paired_high {
                    bytes.push(b'?');
                    continue;
                }
                let encode = |unit: u16, bytes: &mut Vec<u8>| {
                    if unit < 0x80 {
                        bytes.push(unit as u8);
                    } else if unit < 0x800 {
                        bytes.extend_from_slice(&[
                            0xc0 | (unit >> 6) as u8,
                            0x80 | (unit & 0x3f) as u8,
                        ]);
                    } else {
                        bytes.extend_from_slice(&[
                            0xe0 | (unit >> 12) as u8,
                            0x80 | ((unit >> 6) & 0x3f) as u8,
                            0x80 | (unit & 0x3f) as u8,
                        ]);
                    }
                };
                encode(unit, &mut bytes);
                if paired_high { encode(units.next().unwrap(), &mut bytes); }
            }
            return bytes;
        }
        "US-ASCII" | "ASCII" | "ANSI_X3.4-1968" => {
            return string.chars().map(|ch| if ch.is_ascii() { ch as u8 } else { b'?' }).collect();
        }
        "ISO-8859-1" | "ISO8859_1" | "ISO8859-1" | "LATIN1" | "L1" | "IBM819" | "CP819" => {
            return string.chars().map(|ch| u8::try_from(ch as u32).unwrap_or(b'?')).collect();
        }
        "UTF-16" | "UTF-16LE" | "UTF-16BE" => {
            let little = upper == "UTF-16LE";
            let mut bytes = Vec::with_capacity(string.len() * 2 + 2);
            if upper == "UTF-16" { bytes.extend_from_slice(&[0xfe, 0xff]); }
            for unit in string.encode_utf16() {
                let encoded = if little { unit.to_le_bytes() } else { unit.to_be_bytes() };
                bytes.extend_from_slice(&encoded);
            }
            return bytes;
        }
        _ => {}
    }
    let Some(encoding) = encoding_rs::Encoding::for_label(name.as_bytes()) else {
        // The pinned JVM falls back to UTF-8 for an unknown file.encoding.
        return string.into_bytes();
    };
    let mut encoder = encoding.new_encoder();
    let mut bytes = Vec::with_capacity(string.len().saturating_mul(4).saturating_add(32));
    let mut remaining = string.as_str();
    loop {
        let (result, read) = encoder.encode_from_utf8_to_vec_without_replacement(remaining, &mut bytes, true);
        remaining = &remaining[read..];
        match result {
            encoding_rs::EncoderResult::InputEmpty => break,
            encoding_rs::EncoderResult::OutputFull => bytes.reserve(bytes.capacity().max(32)),
            encoding_rs::EncoderResult::Unmappable(_) => bytes.push(b'?'),
        }
    }
    bytes
}

pub(super) fn array(name: &str, values: Vec<Value>) -> Value {
    serde_json::json!({"__stg_java_array":values,"__stg_java_array_type":name})
}

pub(super) fn collection(name: &str, values: Vec<Value>) -> Value {
    serde_json::json!({"__stg_java_sequence":values,"__stg_java_sequence_type":name})
}

fn getter_suffix(name: &str) -> String {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return name.into();
    };
    let (_, upper, _) = crate::preview_domain::character_case(first);
    format!("{upper}{}", chars.as_str())
}

fn getter_name<'a>(properties: impl Iterator<Item = &'a str>, name: &str) -> Option<&'a str> {
    let suffix = getter_suffix(name);
    properties
        .into_iter()
        .find(|candidate| getter_suffix(candidate) == suffix)
}

/// Resolve accessible wrapper members and retained model getter values.
/// None permits the preview graph's attribute/enhancer fallback.
pub(super) fn property(object: &Value, name: &str) -> Option<Value> {
    if let Some(kind) = object.get("__stg_java_bean_class").and_then(Value::as_str) {
        if getter_suffix(name) == "Class" {
            return Some(class(kind));
        }
        return Some(
            metadata()["bean_getters"][kind]
                .get(getter_suffix(name))
                .and_then(Value::as_str)
                .and_then(|key| object.get(key))
                .cloned()
                .unwrap_or(Value::Null),
        );
    }
    if let Some(kind) = object.get("__stg_java_type").and_then(Value::as_str) {
        let properties = metadata()["classes"][kind].as_object()?;
        return Some(
            getter_name(properties.keys().map(String::as_str), name)
                .and_then(|name| properties.get(name))
                .cloned()
                .unwrap_or(Value::Null),
        );
    }
    let kind = if let Some(kind) = object.get("__stg_java_box").and_then(Value::as_str) {
        kind
    } else if let Some(kind) = object.get("__stg_java_enum").and_then(Value::as_str) {
        if getter_suffix(name) == "DeclaringClass" {
            return Some(class(kind));
        }
        kind
    } else if object.is_boolean() {
        "java.lang.Boolean"
    } else if object.is_number() {
        "java.lang.Integer"
    } else if let Some(kind) = object.get("__stg_java_array_type").and_then(Value::as_str) {
        kind
    } else if let Some(kind) = object
        .get("__stg_java_sequence_type")
        .and_then(Value::as_str)
    {
        kind
    } else if object.is_array() {
        "java.util.ArrayList"
    } else if Text::from_value(object).is_some() {
        "java.lang.String"
    } else {
        let kind = object.get("__stg_java_class")?.as_str()?;
        if getter_suffix(name) == "Class" {
            return Some(class(kind));
        }
        let property = metadata()["bean_getters"][kind]
            .get(getter_suffix(name))?
            .as_str()?;
        return Some(
            object["__stg_java_getters"]
                .get(property)
                .cloned()
                .or_else(|| object.get(property).cloned())
                .or_else(|| object.get(getter_suffix(name)).cloned())
                .unwrap_or(Value::Null),
        );
    };
    let getter = getter_name(["class", "empty", "blank", "bytes"].into_iter(), name);
    if getter == Some("class") {
        return Some(class(kind));
    }
    if let Some(text) = Text::from_value(object) {
        return Some(match getter {
            Some("empty") => Value::Bool(text.is_empty()),
            Some("blank") => Value::Bool(text.0.iter().all(|unit| {
                matches!(*unit,
                9..=13 | 28..=32 | 0x1680 | 0x2000..=0x2006 | 0x2008..=0x200a
                | 0x2028..=0x2029 | 0x205f | 0x3000)
            })),
            Some("bytes") => array("[B", default_charset_bytes(&text).into_iter().map(|b| byte(b as i8)).collect()),
            _ => Value::Null,
        });
    }
    if !kind.starts_with('[') && getter == Some("empty") {
        if let Some(items) = expression::sequence_items(object) {
            return Some(Value::Bool(items.is_empty()));
        }
    }
    Some(
        metadata()["fields"][kind]
            .get(name)
            .cloned()
            .unwrap_or(Value::Null),
    )
}
