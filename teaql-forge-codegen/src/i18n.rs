use crate::context::RenderDomain;
use serde_json::json;

fn title_case(input: &str) -> String {
    let mut result = String::new();
    for (index, ch) in input.chars().enumerate() {
        let (lower, upper, flags) = crate::preview_domain::character_case(ch);
        if index == 0 || flags & 2 != 0 {
            if index > 0 {
                result.push(' ');
            }
            result.push(upper);
        } else {
            result.push(lower);
        }
    }
    result
}

pub(crate) fn java_hash(input: &str) -> u32 {
    let mut hash = 0u32;
    for code in input.encode_utf16() {
        hash = hash.wrapping_mul(31).wrapping_add(code as u32);
    }
    hash ^ (hash >> 16)
}

fn hash_map_order(entries: &mut [(String, String)]) {
    // Hutool 5.7.11 asks HashMap for 16 / 0.75 + 1 slots, rounded to 32.
    let mut capacity = 32usize;
    while entries.len() > capacity * 3 / 4 {
        capacity *= 2;
    }
    // Java 8 HashMap keeps insertion order within a bucket when it resizes.
    entries.sort_by_key(|(key, _)| (java_hash(key) as usize) & (capacity - 1));
}

pub(crate) fn i18n_catalog(domain: &RenderDomain) -> String {
    let mut english = Vec::new();
    let mut chinese = Vec::new();
    for entity in &domain.entities {
        let bean = crate::preview_domain::camel(&entity.name);
        let entity_key = format!("entity.{bean}");
        english.push((entity_key.clone(), title_case(&bean)));
        if let Some(value) = entity
            .metadata_ordered
            .iter()
            .find(|(key, _)| key == "zh_CN")
            .map(|(_, value)| value)
            .filter(|value| !value.trim().is_empty())
        {
            chinese.push((entity_key, value.clone()));
        }
        let mut fields = entity
            .fields
            .iter()
            .map(|field| {
                (
                    field.source_order,
                    field.name.as_str(),
                    field.metadata.get("zh_CN"),
                )
            })
            .chain(entity.object_fields.iter().map(|relation| {
                (
                    relation.source_order,
                    relation.name.as_str(),
                    relation.metadata.get("zh_CN"),
                )
            }))
            .collect::<Vec<_>>();
        fields.sort_by_key(|(order, _, _)| *order);
        for (_, name, translation) in fields {
            let member = crate::preview_domain::camel(name);
            let key = format!("property.{bean}.{member}");
            english.push((key.clone(), title_case(&member)));
            if let Some(value) = translation.filter(|value| !value.trim().is_empty()) {
                chinese.push((key, value.clone()));
            }
        }
    }
    hash_map_order(&mut english);
    hash_map_order(&mut chinese);
    let locales = [
        "en", "zh-CN", "zh-TW", "ja", "ko", "de", "fr", "es", "pt", "ar", "th", "id", "fil", "uk",
        "vi",
    ];
    let mut locales = locales
        .into_iter()
        .map(|name| (name.to_string(), String::new()))
        .collect::<Vec<_>>();
    hash_map_order(&mut locales);
    let mut output = String::from(
        "{\n    \"schema\": \"teaql.i18n/v1\",\n    \"defaultLocale\": \"en\",\n    \"locales\": {\n",
    );
    for (index, (locale, _)) in locales.iter().enumerate() {
        output.push_str(&format!(
            "        \"{locale}\": {{\n            \"vocabulary\": {{\n"
        ));
        let entries: &[(String, String)] = if locale == "en" {
            &english
        } else if locale == "zh-CN" {
            &chinese
        } else {
            &[]
        };
        for (entry_index, (key, value)) in entries.iter().enumerate() {
            let comma = if entry_index + 1 == entries.len() {
                ""
            } else {
                ","
            };
            output.push_str(&format!(
                "                {}: {}{comma}\n",
                json!(key),
                json!(value)
            ));
        }
        output.push_str("            },\n            \"messages\": {\n            }\n        }");
        output.push_str(if index + 1 == locales.len() {
            "\n"
        } else {
            ",\n"
        });
    }
    output.push_str("    }\n}");
    output
}
