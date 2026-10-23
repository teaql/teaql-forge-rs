//! DomainNaming and attached attributes exposed by the source preview adaptor.
use crate::context::RenderDomain;
use serde_json::{Map, Value, json};

pub(crate) fn attribute_value(value: &str) -> Value {
    match value.to_ascii_lowercase().as_str() {
        "true" | "yes" | "on" => Value::Bool(true),
        "false" | "no" | "off" => Value::Bool(false),
        _ => Value::String(value.into()),
    }
}

pub(crate) fn has_descriptor_override(domain: &RenderDomain) -> bool {
    domain.root_attributes.get("objectDescriptors").is_some_and(|value| !value.is_empty())
}

// Sorted big-endian u16 records: input, lowercase, uppercase, predicate flags
// (isLowerCase = 1, isUpperCase = 2). SourceCharacterCaseOracle regenerates
// these Java 25 char results; SourceNamingCaseOracle checks actual Hutool output.
const CHARACTER_CASE: &[u8] = include_bytes!("data/java-character-case-25.bin");
const CODE_POINT_CASE: &[u8] = include_bytes!("data/java-code-point-case-25.bin");

pub(crate) fn character_case(ch: char) -> (char, char, u8) {
    // Hutool iterates Java UTF-16 chars, so supplementary letters do not change.
    if u32::from(ch) > 0xffff {
        return (ch, ch, 0);
    }
    let records = CHARACTER_CASE.as_chunks::<8>().0;
    let found = records.binary_search_by_key(&(ch as u16), |record| {
        u16::from_be_bytes([record[0], record[1]])
    });
    if let Ok(index) = found {
        let record = records[index];
        (
            char::from_u32(u16::from_be_bytes([record[2], record[3]]) as u32).unwrap(),
            char::from_u32(u16::from_be_bytes([record[4], record[5]]) as u32).unwrap(),
            record[7],
        )
    } else {
        (ch, ch, 0)
    }
}

fn code_point_case(ch: char) -> (char, char) {
    if u32::from(ch) <= 0xffff {
        let (lower, upper, _) = character_case(ch);
        return (lower, upper);
    }
    let records = CODE_POINT_CASE.as_chunks::<12>().0;
    let found = records.binary_search_by_key(&u32::from(ch), |record| {
        u32::from_be_bytes([record[0], record[1], record[2], record[3]])
    });
    if let Ok(index) = found {
        let record = records[index];
        let lower = u32::from_be_bytes([record[4], record[5], record[6], record[7]]);
        let upper = u32::from_be_bytes([record[8], record[9], record[10], record[11]]);
        (char::from_u32(lower).unwrap(), char::from_u32(upper).unwrap())
    } else {
        (ch, ch)
    }
}

/// Java 25 String.equalsIgnoreCase compares UTF-16 lengths, then uppercase
/// code points and finally lowercase versions of those uppercase code points.
pub(crate) fn java_equals_ignore_case(left: &str, right: &str) -> bool {
    if left == right {
        return true;
    }
    if left.encode_utf16().count() != right.encode_utf16().count()
        || left.chars().count() != right.chars().count()
    {
        return false;
    }
    left.chars().zip(right.chars()).all(|(left, right)| {
        if left == right {
            return true;
        }
        let left_upper = code_point_case(left).1;
        let right_upper = code_point_case(right).1;
        left_upper == right_upper
            || code_point_case(left_upper).0 == code_point_case(right_upper).0
    })
}

#[cfg(test)]
mod java_ignore_case_tests {
    use super::java_equals_ignore_case;

    #[test]
    fn complete_java_25_case_mapping_corpus_matches_equals_ignore_case() {
        let corpus = include_str!("../tests/expected/java-ignore-case-25.tsv");
        let mut compared = 0;
        for line in corpus.lines() {
            let mut parts = line.split('\t');
            let left = char::from_u32(u32::from_str_radix(parts.next().unwrap(), 16).unwrap())
                .unwrap()
                .to_string();
            let right = char::from_u32(u32::from_str_radix(parts.next().unwrap(), 16).unwrap())
                .unwrap()
                .to_string();
            let expected = parts.next().unwrap() == "true";
            assert_eq!(java_equals_ignore_case(&left, &right), expected, "{line}");
            compared += 1;
        }
        assert_eq!(compared, 8_799);
        for (left, right, expected) in [
            ("nodeURL", "NODEURL", true),
            ("nodeURL", "node_UR_l", false),
            ("nodeURL", "NoDeUrL", true),
            ("𐐀", "𐐨", true),
            ("𐐀", "AB", false),
        ] {
            assert_eq!(java_equals_ignore_case(left, right), expected);
        }
    }
}

pub(crate) fn camel(name: &str) -> String {
    if !name.contains('_') {
        return name.into();
    }
    let mut upper = false;
    name.chars()
        .filter_map(|ch| {
            if ch == '_' {
                upper = true;
                None
            } else {
                let (lower, uppercase, _) = character_case(ch);
                let converted = if upper { uppercase } else { lower };
                upper = false;
                Some(converted)
            }
        })
        .collect()
}

pub(crate) fn pascal(name: &str) -> String {
    let value = camel(name);
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return value;
    };
    let (_, upper, flags) = character_case(first);
    if flags & 1 != 0 {
        format!("{upper}{}", chars.as_str())
    } else {
        value
    }
}

/// ObjectDescriptor.getMethodName uses Hutool's UTF-16 Pascal conversion,
/// then removes literal spaces from the result.
pub(crate) fn constant_method_name(name: &str) -> String {
    pascal(name).replace(' ', "")
}

pub(crate) fn upper_first(value: &str) -> String {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return value.into();
    };
    let (_, upper, flags) = character_case(first);
    if flags & 1 != 0 {
        format!("{upper}{}", chars.as_str())
    } else {
        value.into()
    }
}

pub(crate) fn snake(value: &str) -> String {
    let chars = value.chars().collect::<Vec<_>>();
    let mut output = String::new();
    for (index, &ch) in chars.iter().enumerate() {
        let (lower, _, flags) = character_case(ch);
        let mut converted = ch;
        if flags & 2 != 0 {
            let next_lower = chars
                .get(index + 1)
                .is_none_or(|ch| character_case(*ch).2 & 1 != 0);
            if let Some(&previous) = index.checked_sub(1).and_then(|index| chars.get(index)) {
                if previous != '_' && (character_case(previous).2 & 1 != 0 || next_lower) {
                    output.push('_');
                }
            }
            if next_lower {
                converted = lower;
            }
        }
        output.push(converted);
    }
    output
}

/// FieldDescriptorNaming passes the suffix through Hutool's snake conversion
/// with the original field name. Doing the two steps separately changes an
/// acronym at the suffix boundary and can discard combining marks.
pub(crate) fn field_constant_name(value: &str) -> String {
    snake(&format!("{value}_property")).to_uppercase()
}

pub(crate) fn has_attribute(value: &Value, property: &str) -> Option<Value> {
    let attributes = value.get("__stg_attributes")?.as_object()?;
    let suffix = property.strip_prefix("has")?;
    let mut chars = suffix.chars();
    let name = if let Some(first) = chars.next() {
        let (lower, _, flags) = character_case(first);
        format!(
            "{}{}",
            if flags & 2 != 0 { lower } else { first },
            chars.as_str()
        )
    } else {
        String::new()
    };
    Some(Value::Bool(attributes.contains_key(&name)))
}

fn organization(org: &str, separator: &str) -> String {
    let mut parts = org.split('.').collect::<Vec<_>>();
    if !org.is_empty() {
        while parts.last() == Some(&"") {
            parts.pop();
        }
    }
    if parts.len() == 1 {
        format!("com{separator}{org}")
    } else {
        parts.join(separator)
    }
}

pub(crate) fn domain_value(
    domain: &RenderDomain,
    objects: Vec<Value>,
    services: Vec<Value>,
) -> Value {
    let mut data = Map::new();
    data.insert(
        "__stg_java_class".into(),
        json!("com.skynet.codegenerator.meta.Domain"),
    );
    for (name, value) in &domain.root_attributes {
        // The source adaptor ignores empty attached values before its hasX
        // lookup. Attribute presence still includes these keys below.
        if value.is_empty() {
            continue;
        }
        let typed = match value.to_ascii_lowercase().as_str() {
            "true" | "on" | "yes" => Value::Bool(true),
            "false" | "off" | "no" => Value::Bool(false),
            _ => json!(value),
        };
        data.insert(name.clone(), typed);
    }
    data.insert("__stg_attributes".into(), json!(domain.root_attributes));
    let package_name = domain
        .name
        .chars()
        .filter(char::is_ascii_alphabetic)
        .collect::<String>()
        .to_ascii_lowercase();
    let group = domain
        .root_attributes
        .get("org")
        .map(|org| organization(org, "."));
    let fullpackage = group
        .as_ref()
        .map(|group| format!("{group}.{package_name}"));
    data.insert("name".into(), json!(domain.name));
    data.entry("Name").or_insert_with(|| json!(domain.name));
    data.insert("packageDomainName".into(), json!(package_name));
    data.insert("orgAsGroup".into(), json!(group));
    data.insert("fullpackage".into(), json!(fullpackage));
    data.insert(
        "fullPath".into(),
        json!(
            domain
                .root_attributes
                .get("org")
                .map(|org| format!("{}/{package_name}", organization(org, "/")))
        ),
    );
    for (property, class) in [
        ("fullQ", "Q"),
        ("fullE", "E"),
        ("fullConstants", "Constants"),
    ] {
        data.insert(
            property.into(),
            json!(
                fullpackage
                    .as_ref()
                    .map(|package| format!("{package}.{class}"))
            ),
        );
    }
    data.insert(
        "workspaceVersion".into(),
        json!(
            domain
                .root_attributes
                .get("version")
                .filter(|value| !value.is_empty())
                .map(String::as_str)
                .unwrap_or("0.0.1-SNAPSHOT")
        ),
    );
    data.insert("databaseName".into(), json!(package_name));
    data.insert(
        "camelName".into(),
        json!(camel(&domain.name.replace('-', "_"))),
    );
    data.insert(
        "UserContext".into(),
        json!(format!(
            "{}UserContext",
            pascal(&domain.name.replace('-', "_"))
        )),
    );
    data.insert(
        "alias".into(),
        json!(domain.root_attributes.get("alias").map(|alias| {
            let base = if alias.ends_with(['q', 'Q']) {
                &alias[..alias.len() - 1]
            } else {
                alias
            };
            format!("{}Q", pascal(base))
        })),
    );
    data.insert(
        "workspaceJdbcUrl".into(),
        json!(
            domain
                .data_service
                .attributes
                .get("jdbcUrl")
                .filter(|url| !url.is_empty())
                .map(|url| url.replace("${databaseName}", &package_name))
        ),
    );
    let mut examples = domain.entities.iter().enumerate().collect::<Vec<_>>();
    examples.sort_by(|(_, a), (_, b)| {
        b.reverse_relations
            .len()
            .cmp(&a.reverse_relations.len())
            .then_with(|| a.name.encode_utf16().cmp(b.name.encode_utf16()))
    });
    let referenced = examples
        .iter()
        .filter(|(_, entity)| !entity.reverse_relations.is_empty())
        .copied()
        .collect::<Vec<_>>();
    if !referenced.is_empty() {
        examples = referenced;
    }
    let examples = examples
        .into_iter()
        .take(3)
        .map(|(index, _)| objects[index].clone())
        .collect::<Vec<_>>();
    data.insert(
        "primaryExampleObjectDescriptor".into(),
        examples.first().cloned().unwrap_or(Value::Null),
    );
    data.insert("exampleObjectDescriptors".into(), json!(examples));
    data.insert("objectDescriptors".into(), json!(objects));
    data.insert(
        "dataService".into(),
        services
            .iter()
            .find(|service| service["__stg_service_name"] == domain.data_service.name)
            .cloned()
            .unwrap_or(Value::Null),
    );
    data.insert("dataServices".into(), json!(services));
    Value::Object(data)
}

#[cfg(all(test, feature = "parity-probes"))]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    #[ignore = "explicit source Hutool underscore-boundary export"]
    fn export_descriptor_snake_case() {
        let mut output =
            std::fs::File::create(std::env::var("TEAQL_SNAKE_PARITY_OUTPUT").unwrap()).unwrap();
        for code in (0..=0xffff).chain([0x10400, 0x10428, 0x1d400, 0x1f600, 0x10ffff]) {
            let Some(ch) = char::from_u32(code) else {
                continue;
            };
            for word in [
                ch.to_string(),
                format!("a{ch}B"),
                format!("a{ch}BC"),
                format!("_{ch}Ab"),
                format!("ID{ch}Value"),
                format!("4{ch}End"),
                format!("{ch}tail"),
            ] {
                let units = snake(&word).encode_utf16().collect::<Vec<_>>();
                output
                    .write_all(&(units.len() as u32).to_be_bytes())
                    .unwrap();
                for unit in units {
                    output.write_all(&unit.to_be_bytes()).unwrap();
                }
            }
        }
    }

    #[test]
    #[ignore = "explicit source Hutool comparison writes isolated oracle output"]
    fn export_domain_naming_case() {
        let path = std::env::var("TEAQL_NAMING_PARITY_OUTPUT").unwrap();
        let mut output = std::fs::File::create(path).unwrap();
        for code in (0..=0xffff).chain([0x10400, 0x10428, 0x1d400, 0x1f600, 0x10ffff]) {
            let Some(ch) = char::from_u32(code) else {
                continue;
            };
            for value in [
                camel(&format!("UP_{ch}_Tail")),
                pascal(&format!("{ch}_Tail")),
            ] {
                let units = value.encode_utf16().collect::<Vec<_>>();
                output
                    .write_all(&(units.len() as u32).to_be_bytes())
                    .unwrap();
                for unit in units {
                    output.write_all(&unit.to_be_bytes()).unwrap();
                }
            }
        }
    }
}
