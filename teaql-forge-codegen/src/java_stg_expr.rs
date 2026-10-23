//! Value expressions shared by the StringTemplate renderer and conditionals.
use serde_json::{Map, Value};

pub(crate) fn matching_end(input: &str, start: usize, open: char, close: char) -> Option<usize> {
    let mut depth = 0;
    let mut quoted = false;
    let mut escaped = false;
    for (offset, ch) in input[start..].char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            continue;
        }
        if ch == '"' {
            quoted = !quoted;
            continue;
        }
        if quoted {
            continue;
        }
        if ch == open {
            depth += 1;
        }
        if ch == close {
            depth -= 1;
            if depth == 0 {
                return Some(start + offset);
            }
        }
    }
    None
}

pub(crate) fn dictionary_keys(value: &Value) -> Option<Vec<Value>> {
    value.get("__stg_dictionary")?;
    Some(value["__stg_keys"].as_array()?.clone())
}

pub(crate) fn java_sequence(value: &Value) -> Option<&Vec<Value>> {
    value
        .get("__stg_java_array")
        .or_else(|| value.get("__stg_java_sequence"))?
        .as_array()
}

pub(crate) fn sequence_items(value: &Value) -> Option<&Vec<Value>> {
    value.as_array().or_else(|| java_sequence(value))
}

pub(crate) fn java_collection(kind: &str, values: Vec<Value>) -> Value {
    serde_json::json!({"__stg_java_sequence":values,"__stg_java_sequence_type":kind})
}

fn property<'a>(value: &'a Value, key: &str) -> Option<&'a Value> {
    if let Some(dictionary) = value.get("__stg_dictionary") {
        dictionary.get(key).or_else(|| dictionary.get("default"))
    } else {
        value.get(key)
    }
}

pub(crate) fn split_top_level<'a>(input: &'a str, separator: &str) -> Vec<&'a str> {
    let mut result = Vec::new();
    let mut start = 0;
    let mut depth = 0_i32;
    let mut quoted = false;
    let mut escaped = false;
    let mut cursor = 0;
    while cursor < input.len() {
        let ch = input[cursor..].chars().next().unwrap();
        if quoted {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                quoted = false;
            }
        } else if ch == '"' {
            quoted = true;
        } else if depth == 0 && input[cursor..].starts_with(separator) {
            result.push(input[start..cursor].trim());
            cursor += separator.len();
            start = cursor;
            continue;
        } else {
            match ch {
                '(' | '[' | '{' => depth += 1,
                ')' | ']' | '}' => depth -= 1,
                _ => (),
            }
        }
        cursor += ch.len_utf8();
    }
    result.push(input[start..].trim());
    result
}

pub(crate) fn unparenthesize(mut input: &str) -> &str {
    loop {
        if !input.starts_with('(') || !input.ends_with(')') {
            return input;
        }
        // An outer pair may only be removed when it encloses the whole expression.
        let mut depth = 0;
        let mut quoted = false;
        let mut escaped = false;
        let mut closes_early = false;
        for (position, ch) in input.char_indices() {
            if quoted {
                if escaped {
                    escaped = false;
                } else if ch == '\\' {
                    escaped = true;
                } else if ch == '"' {
                    quoted = false;
                }
                continue;
            }
            match ch {
                '"' => quoted = true,
                '(' => depth += 1,
                ')' => {
                    depth -= 1;
                    if depth == 0 && position + 1 < input.len() {
                        closes_early = true;
                        break;
                    }
                }
                _ => (),
            }
        }
        if closes_early {
            return input;
        }
        input = input[1..input.len() - 1].trim();
    }
}

pub(crate) fn builtin(name: &str) -> bool {
    matches!(
        name,
        "first" | "last" | "rest" | "trunc" | "strip" | "reverse" | "length" | "strlen" | "trim"
    )
}

pub(crate) fn apply_builtin(name: &str, argument: Value) -> Value {
    let dictionary = dictionary_keys(&argument);
    let items = dictionary.as_ref().or_else(|| sequence_items(&argument));
    let kind = argument
        .get("__stg_java_sequence_type")
        .and_then(Value::as_str)
        .unwrap_or("");
    let list = argument.is_array()
        || matches!(
            kind,
            "java.util.ArrayList"
                | "java.util.LinkedList"
                | "org.stringtemplate.v4.Interpreter$ObjectList"
                | "java.util.ArrayList$SubList"
                | "java.util.AbstractList$SubList"
                | "java.util.AbstractList$RandomAccessSubList"
        );
    let array = argument.get("__stg_java_array_type").is_some();
    if let Some(items) = items {
        let result = match name {
            "length" => Some(Value::from(items.len())),
            "first" => Some(items.first().cloned().unwrap_or_else(|| argument.clone())),
            "last" => Some(if let Some(last) = items.last() {
                last.clone()
            } else if list || array {
                serde_json::json!({"__stg_getter_fault":"Index -1 out of bounds for length 0"})
            } else {
                argument.clone()
            }),
            "rest" | "trunc" => Some(
                if list && items.len() <= 1 || name == "rest" && items.is_empty() {
                    Value::Null
                } else {
                    let values = if name == "rest" {
                        items[1..].to_vec()
                    } else {
                        items[..items.len().saturating_sub(1)].to_vec()
                    };
                    let output = if !list {
                        "java.util.ArrayList"
                    } else if matches!(
                        kind,
                        "java.util.LinkedList" | "java.util.AbstractList$SubList"
                    ) {
                        "java.util.AbstractList$SubList"
                    } else {
                        "java.util.ArrayList$SubList"
                    };
                    java_collection(output, values)
                },
            ),
            "strip" => Some(java_collection(
                "java.util.ArrayList",
                items.iter().filter(|v| !v.is_null()).cloned().collect(),
            )),
            "reverse" => Some(java_collection(
                "java.util.LinkedList",
                items.iter().rev().cloned().collect(),
            )),
            _ => None,
        };
        if let Some(result) = result {
            return result;
        }
    }
    match name {
        "length" => Value::from(match &argument {
            Value::Null => 0,
            Value::Array(items) => items.len(),
            _ => 1,
        }),
        "strlen" => argument
            .as_str()
            .map(|text| Value::from(text.encode_utf16().count()))
            .unwrap_or(Value::Null),
        "trim" => argument
            .as_str()
            .map(|text| Value::String(text.trim_matches(|ch| ch <= '\u{20}').into()))
            .unwrap_or(Value::Null),
        "first" => match argument {
            Value::Array(items) if !items.is_empty() => items[0].clone(),
            other => other,
        },
        "last" => match argument {
            Value::Array(items) => items.last().cloned().unwrap_or(Value::Null),
            other => other,
        },
        "rest" => match argument {
            Value::Array(items) if items.len() > 1 => Value::Array(items[1..].to_vec()),
            _ => Value::Null,
        },
        "trunc" => match argument {
            Value::Array(mut items) if items.len() > 1 => {
                items.pop();
                Value::Array(items)
            }
            _ => Value::Null,
        },
        "strip" => match argument {
            Value::Array(items) => {
                Value::Array(items.into_iter().filter(|item| !item.is_null()).collect())
            }
            other => other,
        },
        "reverse" => match argument {
            Value::Array(mut items) => {
                items.reverse();
                Value::Array(items)
            }
            other => other,
        },
        _ => Value::Null,
    }
}

pub(crate) fn value(expr: &str, vars: &Map<String, Value>) -> Value {
    value_with_graph(expr, vars, &Map::new())
}

pub(crate) fn value_with_graph(
    expr: &str,
    vars: &Map<String, Value>,
    graph: &Map<String, Value>,
) -> Value {
    let expr = unparenthesize(expr.trim());
    if expr.starts_with('"') && expr.ends_with('"') {
        return serde_json::from_str(expr)
            .unwrap_or_else(|_| Value::String(expr[1..expr.len() - 1].to_string()));
    }
    if expr == "true" {
        return Value::Bool(true);
    }
    if expr == "false" {
        return Value::Bool(false);
    }
    if let Some(list) = expr
        .strip_prefix('[')
        .and_then(|rest| rest.strip_suffix(']'))
    {
        let mut values = Vec::new();
        if !list.trim().is_empty() {
            for item in split_top_level(list, ",") {
                match value_with_graph(item, vars, graph) {
                    Value::Array(items) => values.extend(items),
                    item => values.push(item),
                }
            }
        }
        return Value::Array(values);
    }
    if let Some((name, args)) = expr.split_once('(') {
        if builtin(name) && args.ends_with(')') {
            let argument = value_with_graph(&args[..args.len() - 1], vars, graph);
            return apply_builtin(name, argument);
        }
    }
    let parts = split_top_level(expr, ".");
    use std::borrow::Cow;
    let head = parts[0];
    let computed_head = parts.len() > 1
        && (head.starts_with('(')
            || head
                .split_once('(')
                .is_some_and(|(name, args)| builtin(name) && args.ends_with(')')));
    let mut current = if computed_head {
        Cow::Owned(value_with_graph(head, vars, graph))
    } else {
        Cow::Borrowed(vars.get(head).unwrap_or(&Value::Null))
    };
    for part in parts.into_iter().skip(1) {
        if let Some(found) = dereference(&current, graph) {
            current = Cow::Borrowed(found);
        }
        if getter_fault(&current).is_some() {
            return current.into_owned();
        }
        let key_value = if part.starts_with('(') {
            value_with_graph(part, vars, graph)
        } else {
            Value::String(part.into())
        };
        if getter_fault(&key_value).is_some() {
            return key_value;
        }
        let key = string(&key_value);
        if let Some(keys) = dictionary_keys(&current) {
            let dictionary = &current["__stg_dictionary"];
            current = Cow::Owned(if dictionary.get(&key).is_none() && key == "keys" {
                Value::Array(keys)
            } else if dictionary.get(&key).is_none() && key == "values" {
                Value::Array(
                    keys.iter()
                        .map(|key| dictionary[key.as_str().unwrap()].clone())
                        .collect(),
                )
            } else {
                let found = if key_value.is_null() {
                    dictionary.get("default").cloned().unwrap_or(Value::Null)
                } else {
                    property(&current, &key).cloned().unwrap_or(Value::Null)
                };
                if found.get("__stg_key") == Some(&Value::Bool(true)) {
                    key_value
                } else {
                    found
                }
            });
        } else {
            if current.get(&key).is_none() {
                if let Some(found) = crate::preview_domain::has_attribute(&current, &key) {
                    current = Cow::Owned(found);
                    continue;
                }
            }
            current = match current {
                Cow::Borrowed(value) => Cow::Borrowed(value.get(&key).unwrap_or(&Value::Null)),
                Cow::Owned(value) => Cow::Owned(value.get(&key).cloned().unwrap_or(Value::Null)),
            };
        }
        if current.get("__stg_graph_service_payload").is_some() {
            current = Cow::Owned(crate::graph_service::getter(current.into_owned()));
        }
    }
    dereference(&current, graph)
        .cloned()
        .unwrap_or_else(|| current.into_owned())
}

pub(crate) fn dereference<'a>(value: &Value, graph: &'a Map<String, Value>) -> Option<&'a Value> {
    let object = value.as_object()?;
    if object.len() != 1 {
        return None;
    }
    graph.get(object.get("__stg_ref")?.as_str()?)
}

pub(crate) fn string(value: &Value) -> String {
    if let Some(items) = java_sequence(value) {
        return items.iter().map(string).collect();
    }
    match value {
        Value::String(text) => text.clone(),
        Value::Null => String::new(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        Value::Array(values) => values.iter().map(string).collect::<Vec<_>>().join(""),
        Value::Object(values) => values.get("__string").map(string).unwrap_or_default(),
    }
}

pub(crate) fn truthy(value: &Value) -> bool {
    if let Some(items) = java_sequence(value) {
        return !items.is_empty();
    }
    if let Some(keys) = dictionary_keys(value) {
        return !keys.is_empty();
    }
    match value {
        Value::Null => false,
        Value::Bool(value) => *value,
        Value::Array(items) => !items.is_empty(),
        Value::Object(items) => !items.is_empty(),
        _ => true, // ST4 treats defined strings (including "") and zero as true.
    }
}

/// A Java enhancer exception aborts the current ST instance, preserving output.
pub(crate) fn getter_fault(value: &Value) -> Option<&str> {
    let object = value.as_object()?;
    if object.len() != 1 {
        return None;
    }
    object.get("__stg_getter_fault").and_then(Value::as_str)
}
