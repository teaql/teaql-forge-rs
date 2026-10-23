//! Jackson-compatible sidecar coercion and source error locations.
use super::SemanticIdentityDocument;
use serde_json::Value;

const DOCUMENT: &str = "com.skynet.codegenerator.semantic.SemanticIdentityDocument";
const BINDING: &str = "com.skynet.codegenerator.semantic.SemanticIdentityBinding";

struct Node {
    value: Value,
    start: usize,
    end: usize,
    fields: Vec<(String, Node)>,
    items: Vec<Node>,
}

fn skip_space(bytes: &[u8], mut offset: usize) -> usize {
    while bytes.get(offset).is_some_and(u8::is_ascii_whitespace) {
        offset += 1;
    }
    offset
}

fn node(bytes: &[u8], offset: usize) -> Result<Node, serde_json::Error> {
    let start = skip_space(bytes, offset);
    let mut stream = serde_json::Deserializer::from_slice(&bytes[start..]).into_iter::<Value>();
    let value = stream
        .next()
        .unwrap_or_else(|| serde_json::from_slice(b""))?;
    let end = start + stream.byte_offset();
    let mut fields = Vec::new();
    let mut items = Vec::new();
    let mut cursor = skip_space(bytes, start + 1);
    if value.is_object() {
        while bytes.get(cursor) != Some(&b'}') {
            let key = node(bytes, cursor)?;
            cursor = skip_space(bytes, key.end) + 1;
            let item = node(bytes, cursor)?;
            cursor = skip_space(bytes, item.end);
            fields.push((key.value.as_str().unwrap_or("").into(), item));
            if bytes.get(cursor) == Some(&b',') {
                cursor = skip_space(bytes, cursor + 1);
            }
        }
    } else if value.is_array() {
        while bytes.get(cursor) != Some(&b']') {
            let item = node(bytes, cursor)?;
            cursor = skip_space(bytes, item.end);
            items.push(item);
            if bytes.get(cursor) == Some(&b',') {
                cursor = skip_space(bytes, cursor + 1);
            }
        }
    }
    Ok(Node {
        value,
        start,
        end,
        fields,
        items,
    })
}

fn location(bytes: &[u8], offset: usize, zero_column: bool) -> String {
    let text = String::from_utf8_lossy(bytes);
    let source = if bytes.len() <= 500 {
        text.into_owned()
    } else {
        format!(
            "{}\"[truncated {} bytes]",
            text.chars().take(500).collect::<String>(),
            bytes.len() - 500
        )
    };
    let prefix = &bytes[..offset.min(bytes.len())];
    let line = prefix.iter().filter(|byte| **byte == b'\n').count() + 1;
    let column = if zero_column {
        0
    } else {
        prefix
            .iter()
            .rposition(|byte| *byte == b'\n')
            .map_or(prefix.len() + 1, |index| prefix.len() - index)
    };
    format!("\n at [Source: (byte[])\"{source}\"; line: {line}, column: {column}]")
}

fn mismatch(value: &Value, ty: &str) -> String {
    let (kind, token) = match value {
        Value::Object(_) => ("Object", "START_OBJECT"),
        Value::Array(_) => ("Array", "START_ARRAY"),
        Value::Bool(true) => ("Boolean", "VALUE_TRUE"),
        Value::Bool(false) => ("Boolean", "VALUE_FALSE"),
        Value::Number(_) => ("Number", "VALUE_NUMBER_INT"),
        _ => ("String", "VALUE_STRING"),
    };
    format!("Cannot deserialize value of type `{ty}` from {kind} value (token `JsonToken.{token}`)")
}

fn constructor(value: &Value, ty: &str) -> String {
    let detail = if let Some(text) = value.as_str() {
        format!("String-argument constructor/factory method to deserialize from String value ('{text}')")
    } else {
        format!("int/Int-argument constructor/factory method to deserialize from Number value ({value})")
    };
    format!(
        "Cannot construct instance of `{ty}` (although at least one Creator exists): no {detail}"
    )
}

fn map_object(bytes: &[u8], input: &Node, binding: bool, prefix: &str) -> Result<Value, String> {
    let class = if binding { BINDING } else { DOCUMENT };
    if !input.value.is_object() {
        let message = if input.value.is_string() || input.value.is_number() {
            constructor(&input.value, class)
        } else {
            mismatch(&input.value, class)
        };
        return Err(format!("{message}{}", location(bytes, input.start, false)));
    }
    let mut object = serde_json::Map::new();
    for (name, item) in &input.fields {
        let chain = format!("{prefix}{class}[\"{name}\"]");
        let known = if binding {
            [
                "id",
                "kind",
                "parentId",
                "path",
                "aliases",
                "status",
                "derivedFrom",
            ]
            .contains(&name.as_str())
        } else {
            ["schemaVersion", "revision", "modelId", "bindings"].contains(&name.as_str())
        };
        if !known {
            let properties = if binding {
                "7 known properties: \"status\", \"kind\", \"parentId\", \"derivedFrom\", \"id\", \"aliases\", \"path\"]"
            } else {
                "4 known properties: \"revision\", \"modelId\", \"bindings\", \"schemaVersion\"]"
            };
            let offset = item.end;
            return Err(format!("Unrecognized field \"{name}\" (class {class}), not marked as ignorable ({properties}){} (through reference chain: {chain})", location(bytes, offset, false)));
        }
        let list = matches!(name.as_str(), "bindings" | "aliases" | "derivedFrom");
        let number = matches!(name.as_str(), "revision" | "schemaVersion");
        let value = if list {
            if item.value.is_null() {
                Value::Array(Vec::new())
            } else if !item.value.is_array() {
                let ty = if name == "bindings" {
                    format!("java.util.ArrayList<{BINDING}>")
                } else {
                    "java.util.ArrayList<java.lang.String>".into()
                };
                let message = if item.value.is_string() {
                    constructor(&item.value, "java.util.ArrayList")
                } else {
                    mismatch(&item.value, &ty)
                };
                let offset = item.start;
                return Err(format!(
                    "{message}{} (through reference chain: {chain})",
                    location(bytes, offset, false)
                ));
            } else {
                let mut values = Vec::new();
                for (index, entry) in item.items.iter().enumerate() {
                    if name == "bindings" {
                        values.push(map_object(
                            bytes,
                            entry,
                            true,
                            &format!("{chain}->java.util.ArrayList[{index}]->"),
                        )?);
                    } else {
                        values.push(string_value(
                            bytes,
                            entry,
                            &format!("{chain}->java.util.ArrayList[{index}]"),
                        )?);
                    }
                }
                Value::Array(values)
            }
        } else if number {
            let ty = if name == "revision" { "long" } else { "int" };
            if item.value.is_null() {
                Value::from(0)
            } else if let Some(number) = item.value.as_i64() {
                Value::from(number)
            } else if let Some(number) = item.value.as_f64() {
                Value::from(number as i64)
            } else if let Some(text) = item.value.as_str() {
                if text.trim().is_empty() {
                    Value::from(0)
                } else if let Ok(number) = text.trim().parse::<i64>() {
                    Value::from(number)
                } else {
                    return Err(format!("Cannot deserialize value of type `{ty}` from String \"{text}\": not a valid `{ty}` value{} (through reference chain: {chain})", location(bytes, item.start, false)));
                }
            } else {
                return Err(format!(
                    "{}{} (through reference chain: {chain})",
                    mismatch(&item.value, ty),
                    location(bytes, item.start, false)
                ));
            }
        } else {
            string_value(bytes, item, &chain)?
        };
        object.insert(name.clone(), value);
    }
    Ok(Value::Object(object))
}

fn string_value(bytes: &[u8], input: &Node, chain: &str) -> Result<Value, String> {
    match &input.value {
        Value::String(_) | Value::Null => Ok(input.value.clone()),
        Value::Number(_) | Value::Bool(_) => Ok(Value::String(input.value.to_string())),
        _ => Err(format!(
            "{}{} (through reference chain: {chain})",
            mismatch(&input.value, "java.lang.String"),
            location(bytes, input.start, false)
        )),
    }
}

pub(super) fn load(bytes: &[u8]) -> Result<SemanticIdentityDocument, String> {
    let input = node(bytes, 0).map_err(|error| {
        let text = String::from_utf8_lossy(bytes);
        let start = bytes.split(|byte| *byte == b'\n').take(error.line().saturating_sub(1)).map(|line|line.len()+1).sum::<usize>();
        let offset = start + error.column().saturating_sub(1);
        let character = text.get(offset..).and_then(|rest|rest.chars().next()).unwrap_or('\0');
        let detail = error.to_string();
        let message = if text.trim().is_empty() { format!("No content to map due to end-of-input{}",location(bytes,bytes.len(),true)) }
        else if error.is_eof() { format!("Unexpected end-of-input within/between Object entries{}",location(bytes,bytes.len(),false)) }
        else if detail.starts_with("key must be a string") || detail.starts_with("trailing comma") {
            format!("Unexpected character ('{character}' (code {})): was expecting double-quote to start field name{}",character as u32,location(bytes,offset+character.len_utf8(),false))
        } else if detail.starts_with("expected `:`") {
            format!("Unexpected character ('{character}' (code {})): was expecting a colon to separate field name and value{}",character as u32,location(bytes,offset+character.len_utf8(),false))
        } else { detail };
        message
    })?;
    let value = map_object(bytes, &input, false, "")?;
    serde_json::from_value(value).map_err(|error| error.to_string())
}
