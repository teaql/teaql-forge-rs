//! Ordered service definitions, including source attribute append/override rules.
use crate::types::TypeAttribute;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DataService {
    pub name: String,
    pub is_default: bool,
    pub support_stack: String,
    pub attributes: Vec<TypeAttribute>,
}

impl DataService {
    pub fn attribute(&self, name: &str) -> Option<&str> {
        self.attributes
            .iter()
            .find(|attr| attr.name == name)
            .map(|attr| attr.value.as_str())
    }
}

pub fn builtin_services() -> &'static [DataService] {
    static SERVICES: OnceLock<Vec<DataService>> = OnceLock::new();
    SERVICES.get_or_init(|| {
        #[derive(Deserialize)]
        struct Catalog {
            data_services: Vec<DataService>,
        }
        serde_json::from_str::<Catalog>(include_str!("data/source-data-services.json"))
            .expect("pinned data service catalog")
            .data_services
    })
}

pub fn find<'a>(definitions: &'a [DataService], name: &str) -> Option<&'a DataService> {
    definitions
        .iter()
        .chain(builtin_services())
        .find(|service| service.name.eq_ignore_ascii_case(name))
}

pub fn all(definitions: &[DataService]) -> Vec<&DataService> {
    builtin_services()
        .iter()
        .map(|service| find(definitions, &service.name).unwrap())
        .chain(definitions.iter().filter(|service| {
            !builtin_services()
                .iter()
                .any(|builtin| builtin.name.eq_ignore_ascii_case(&service.name))
        }))
        .collect()
}

pub fn definitions(document: &roxmltree::Document<'_>) -> Vec<DataService> {
    // The source first merges SAX attributes into user definitions, retaining
    // each attribute's original append flag, then merges these into built-ins.
    let mut parsed: Vec<(String, Vec<(String, String, bool)>)> = Vec::new();
    for node in document
        .descendants()
        .filter(|node| node.is_element() && node.tag_name().name() == "_data_service")
    {
        let Some(name) = node
            .attribute("name")
            .filter(|name| !name.trim().is_empty())
        else {
            continue;
        };
        let index = parsed
            .iter()
            .position(|(existing, _)| existing.eq_ignore_ascii_case(name))
            .unwrap_or_else(|| {
                parsed.push((name.to_string(), Vec::new()));
                parsed.len() - 1
            });
        for attr in node.attributes() {
            let (key, append) = attr
                .name()
                .strip_suffix("_delta")
                .map(|key| (key, true))
                .unwrap_or((attr.name(), false));
            if let Some((_, value, _)) = parsed[index].1.iter_mut().find(|(name, _, _)| name == key)
            {
                if append {
                    value.push(',');
                    value.push_str(attr.value());
                } else {
                    *value = attr.value().to_string();
                }
            } else {
                parsed[index]
                    .1
                    .push((key.to_string(), attr.value().to_string(), append));
            }
        }
    }
    parsed
        .into_iter()
        .map(|(name, attributes)| {
            let mut service = find(&[], &name).cloned().unwrap_or_else(|| DataService {
                support_stack: if matches!(name.as_str(), "rusqlite" | "linux") {
                    "rust"
                } else {
                    "java,rust"
                }
                .into(),
                name,
                is_default: false,
                attributes: Vec::new(),
            });
            for (name, value, append) in attributes {
                if let Some(existing) = service.attributes.iter_mut().find(|attr| attr.name == name)
                {
                    if append {
                        existing.value.push(',');
                        existing.value.push_str(&value);
                    } else {
                        existing.value = value;
                    }
                } else {
                    service.attributes.push(TypeAttribute { name, value, append, override_value: !append });
                }
            }
            service
        })
        .collect()
}
