//! Physical field attributes retained by DomainParser before getter adaptation.
use crate::ir::{Entity, EntityMember};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Attribute {
    pub name: String,
    pub value: String,
    pub append: bool,
    #[serde(rename = "override")]
    pub override_value: bool,
    pub remove: bool,
    pub deleted: bool,
}

pub type Fields = BTreeMap<String, BTreeMap<String, Vec<Attribute>>>;

pub struct FieldValue {
    pub value: String,
    pub label: Option<String>,
    pub json_literal: bool,
}

/// DomainParser checks JSON before translation, and checks absolute URLs with
/// Java's default (non-DOTALL) regex. Preserve the original value otherwise.
pub fn parse_value(original: &str) -> FieldValue {
    let trimmed = original.trim_matches(|ch| ch <= '\u{20}');
    let json_literal = trimmed.starts_with('{') && trimmed.ends_with('}');
    let absolute_url = original.split_once("://").is_some_and(|(scheme, _)| {
        scheme.as_bytes().first().is_some_and(u8::is_ascii_alphabetic)
            && scheme.chars().all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '+' | '.' | '-'))
            && !original.contains(['\n', '\r', '\u{85}', '\u{2028}', '\u{2029}'])
    });
    if !json_literal && !absolute_url {
        let (label, value) = split_pair(original, &[':', '：']);
        if !value.chars().all(crate::attributes::blank_char) {
            return FieldValue { value, label: Some(label), json_literal };
        }
    }
    FieldValue { value: original.into(), label: None, json_literal }
}

pub fn empty_value(original: &str) -> bool {
    let parsed = parse_value(original);
    !parsed.json_literal && split(&parsed.value, '|').is_empty()
}

fn set(attributes: &mut Vec<Attribute>, name: &str, value: &str) {
    let (name, append) = name.strip_suffix("_delta").map_or((name, false), |name| (name, true));
    attributes.retain(|attribute| attribute.name != name);
    attributes.push(Attribute { name: name.into(), value: value.into(), append,
        override_value: !append, remove: false, deleted: false });
}

fn get<'a>(attributes: &'a [Attribute], name: &str) -> Option<&'a str> {
    attributes.iter().find(|attribute| attribute.name == name).map(|attribute| attribute.value.as_str())
}

/// The source Parser drops empty segments and retains its pending escape until
/// a separator or a second backslash consumes it.
pub fn split(value: &str, separator: char) -> Vec<String> {
    let mut result = Vec::new();
    let mut token = String::new();
    let mut escape = false;
    for ch in value.chars() {
        if ch == separator {
            if escape { token.push(ch); escape = false; }
            else if !token.is_empty() { result.push(std::mem::take(&mut token)); }
        } else if ch == '\\' {
            if escape { token.push(ch); escape = false; } else { escape = true; }
        } else { token.push(ch); }
    }
    if !token.is_empty() { result.push(token); }
    result
}

pub fn split_pair(value: &str, separators: &[char]) -> (String, String) {
    let mut token = String::new();
    let mut escape = false;
    for (index, ch) in value.char_indices() {
        if separators.contains(&ch) {
            if escape { token.push(ch); escape = false; }
            else { return (token, value[index + ch.len_utf8()..].into()); }
        } else if ch == '\\' {
            if escape { token.push(ch); escape = false; } else { escape = true; }
        } else { token.push(ch); }
    }
    (token, String::new())
}

pub fn options(value: &str) -> Vec<(String, String)> {
    value.split(|ch| matches!(ch, '\t'..='\r' | ' ' | '{' | '}'))
        .map(|meta| meta.trim_matches(|ch| ch <= '\u{20}')).filter(|meta| !meta.is_empty())
        .map(|meta| {
            let (key, value) = split_pair(meta, &['=']);
            if !value.is_empty() { (key, value) }
            else { meta.strip_prefix('!').map_or((meta.into(), "true".into()), |key| (key.into(), "false".into())) }
        }).collect()
}

pub fn declare(fields: &mut Fields, owner: &str, field: &str, value: &str, label: Option<&str>, json_literal: bool) {
    let attrs = fields.entry(owner.into()).or_default().entry(field.into()).or_default();
    let trimmed = value.trim_matches(|ch| ch <= '\u{20}');
    if json_literal {
        set(attrs, "sampleValue", trimmed);
        return;
    }
    if let Some(label) = label { set(attrs, "zh_CN", label); }
    let mut parts = split(value, '|');
    if parts.len() > 1 {
        if let Some((min, max)) = parts.last().and_then(|part| part.strip_prefix('['))
            .and_then(|part| part.strip_suffix(']')).and_then(|part| part.split_once(','))
        {
            set(attrs, "min", min.trim_matches(|ch| ch <= '\u{20}'));
            set(attrs, "max", max.trim_matches(|ch| ch <= '\u{20}'));
            parts.pop();
        }
    }
    if let Some(last) = parts.last_mut() {
        if last.ends_with('?') { last.pop(); set(attrs, "optional", "true"); }
    }
    if parts.len() > 1 { set(attrs, "candidates", &parts.join(",")); }
    if let Some(last) = parts.last() { set(attrs, "sampleValue", last); }
}

pub fn identify(fields: &mut Fields, owner: &str, name: &str) {
    if let Some(attrs) = fields.get_mut(owner).and_then(|fields| fields.get_mut(name)) {
        set(attrs, "identifier", "true");
    }
}

/// Java String.split(",") discards trailing empty fields, except for an empty input.
pub(crate) fn split_java_commas(value: &str) -> Vec<&str> {
    let mut parts = value.split(',').collect::<Vec<_>>();
    if !value.is_empty() {
        while parts.last() == Some(&"") { parts.pop(); }
    }
    parts
}

#[cfg(test)]
mod comma_split_tests {
    use super::split_java_commas;

    #[test]
    fn java_split_drops_only_trailing_empty_fields() {
        assert_eq!(split_java_commas(""), [""]);
        assert_eq!(split_java_commas(","), [] as [&str; 0]);
        assert_eq!(split_java_commas("a,"), ["a"]);
        assert_eq!(split_java_commas("a,,"), ["a"]);
        assert_eq!(split_java_commas("a,,b,"), ["a", "", "b"]);
    }
}

pub fn candidate(fields: &mut Fields, owner: &str, field: &str, value: &str, remove: bool) {
    let Some(attrs) = fields.get_mut(owner).and_then(|fields| fields.get_mut(field)) else { return };
    if let Some(existing) = attrs.iter_mut().find(|attribute| attribute.name == "candidates") {
        if remove {
            let mut values = split_java_commas(&existing.value).into_iter().map(str::to_string).collect::<Vec<_>>();
            for value in split_java_commas(value) {
                if let Some(index) = values.iter().position(|existing| existing == value) {
                    values.remove(index);
                }
            }
            existing.value = values.join(",");
        } else { existing.value.push(','); existing.value.push_str(value); }
        existing.deleted = false;
    } else if !remove {
        attrs.push(Attribute { name: "candidates".into(), value: value.into(), append: true,
            override_value: false, remove: false, deleted: false });
    }
}

pub fn finish(fields: &mut Fields, entities: &[Entity], type_names: &[String]) {
    for entity in entities {
        let Some(owned) = fields.get_mut(&entity.name) else { continue };
        for member in &entity.members {
            let name = match member { EntityMember::Field(field) => &field.name, EntityMember::Relation(field) => &field.name };
            let Some(attrs) = owned.get_mut(name) else { continue };
            let sample = get(attrs, "sampleValue").unwrap_or("").to_string();
            let call = sample.find(['(', '{']).filter(|&index| index > 0 && !sample[..index].contains([')', '}']) && sample.ends_with([')', '}']))
                .map(|index| (&sample[..index], &sample[index + 1..sample.len() - 1]));
            if let Some((kind, options)) = call {
                for (key, value) in self::options(options) {
                    set(attrs, &key, &value);
                }
                if type_names.iter().any(|name| name.eq_ignore_ascii_case(kind)) || entities.iter().any(|entity| entity.name.eq_ignore_ascii_case(kind)) {
                    if let Some(candidates) = get(attrs, "candidates").filter(|value| !value.is_empty()).map(str::to_string) {
                        let cleaned = candidates.replace(&sample, "").split(',')
                            .map(|part| part.trim_matches(crate::attributes::blank_char))
                            .filter(|part| !part.is_empty()).collect::<Vec<_>>().join(",");
                        set(attrs, "candidates", &cleaned);
                    }
                }
            }
            let root = !entity.metadata.contains_key("parent") && entity.members.iter().all(|member| !matches!(member, EntityMember::Relation(_)));
            let constant = entity.metadata.get("constant").is_some_and(|value| crate::parser::hutool_true(value));
            if root || constant {
                if get(attrs, "candidates").is_none() {
                    let value = get(attrs, "sampleValue").unwrap_or("").to_string();
                    set(attrs, "candidates", &value);
                }
                if let Some(value) = get(attrs, "autoFunction").map(str::to_string) { set(attrs, "autoFunction", &value); }
            }
        }
    }
}
