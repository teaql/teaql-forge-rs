//! Source type attributes and ordered inheritance for KSML type definitions.
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TypeAttribute {
    pub name: String,
    pub value: String,
    #[serde(skip)]
    pub append: bool,
    #[serde(skip)]
    pub override_value: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TypeState {
    pub name: String,
    pub parent: Option<String>,
    #[serde(rename = "default")]
    pub default_value: bool,
    pub line_number: usize,
    pub xml_path: Option<String>,
    pub parent_type: Option<String>,
}

pub fn states(document: &roxmltree::Document<'_>, xml_path: &str) -> Vec<TypeState> {
    #[derive(Deserialize)]
    struct Catalog {
        types: Vec<TypeState>,
    }
    let mut types =
        serde_json::from_str::<Catalog>(include_str!("data/source-type-beans-6f2d0a.json"))
            .expect("pinned Type bean catalog")
            .types;
    let mut parsed: Vec<TypeState> = Vec::new();
    for node in document
        .descendants()
        .filter(|n| n.is_element() && n.tag_name().name() == "_type_definition")
    {
        let Some(name) = node.attribute("name").filter(|s| !s.trim().is_empty()) else {
            continue;
        };
        let position = if let Some(position) = parsed
            .iter()
            .position(|t| t.name.eq_ignore_ascii_case(name))
        {
            position
        } else {
            parsed.push(TypeState {
                name: name.into(),
                parent: None,
                default_value: false,
                line_number: 0,
                xml_path: None,
                parent_type: None,
            });
            parsed.len() - 1
        };
        let value = &mut parsed[position];
        value.parent = node.attribute("parent").map(str::to_owned);
        let start = node
            .attributes()
            .last()
            .map(|a| a.range().end)
            .unwrap_or(node.range().start);
        let end = start + document.input_text()[start..].find('>').unwrap_or(0);
        value.line_number = document.text_pos_at(end).row as usize;
        value.xml_path = Some(xml_path.into());
    }
    for value in parsed {
        if !types
            .iter()
            .any(|t| t.name.eq_ignore_ascii_case(&value.name))
        {
            types.push(value);
        }
    }
    let names = types.iter().map(|t| t.name.clone()).collect::<Vec<_>>();
    for value in &mut types {
        let parent = value
            .parent
            .as_deref()
            .filter(|s| !s.is_empty())
            .or_else(|| (value.name != "root").then_some("root"));
        value.parent_type = parent
            .and_then(|p| names.iter().find(|n| n.eq_ignore_ascii_case(p)))
            .cloned();
    }
    types
}

pub fn builtin_types() -> BTreeMap<String, Vec<TypeAttribute>> {
    #[derive(Deserialize)]
    struct Catalog {
        types: BTreeMap<String, Vec<TypeAttribute>>,
    }
    serde_json::from_str::<Catalog>(include_str!("data/source-types.json"))
        .expect("pinned type catalog")
        .types
}

/// Source merges existing types without replacing their parent, then resolves
/// all new parents case-insensitively before any field is rendered.
#[derive(Clone, Copy)]
pub enum InheritanceFailure {
    Cycle,
    MissingParent,
}

pub const MISSING_PARENT_MESSAGE: &str =
    "Cannot invoke \"com.skynet.codegenerator.meta.Type.getParent()\" because \"type\" is null";

pub fn has_inheritance_cycle(document: &roxmltree::Document<'_>) -> bool {
    matches!(
        inheritance_failure(document),
        Some(InheritanceFailure::Cycle)
    )
}

pub fn inheritance_failure(document: &roxmltree::Document<'_>) -> Option<InheritanceFailure> {
    let mut definitions = builtin_types()
        .into_keys()
        .map(|name| (name, None::<String>))
        .collect::<Vec<_>>();
    for definition in document
        .descendants()
        .filter(|node| node.is_element() && node.tag_name().name() == "_type_definition")
    {
        let Some(name) = definition
            .attribute("name")
            .filter(|name| !name.trim().is_empty())
        else {
            continue;
        };
        if definitions
            .iter()
            .any(|(existing, _)| existing.eq_ignore_ascii_case(name))
        {
            continue;
        }
        definitions.push((
            name.into(),
            definition.attribute("parent").map(str::to_string),
        ));
    }
    let parents = definitions
        .iter()
        .map(
            |(_, parent)| match parent.as_ref().filter(|parent| !parent.is_empty()) {
                None => Ok(None),
                Some(parent) => definitions
                    .iter()
                    .position(|(name, _)| name.eq_ignore_ascii_case(parent))
                    .map(Some)
                    .ok_or(InheritanceFailure::MissingParent),
            },
        )
        .collect::<Vec<_>>();
    let mut visited = vec![0_u8; definitions.len()];
    for start in 0..definitions.len() {
        let mut next = Some(start);
        let mut path = Vec::new();
        while let Some(index) = next {
            match visited[index] {
                1 => return Some(InheritanceFailure::Cycle),
                2 => break,
                _ => {}
            }
            visited[index] = 1;
            path.push(index);
            next = match parents[index] {
                Ok(parent) => parent,
                Err(error) => return Some(error),
            };
        }
        for index in path {
            visited[index] = 2;
        }
    }
    None
}

pub fn definitions(document: &roxmltree::Document<'_>) -> BTreeMap<String, Vec<TypeAttribute>> {
    let mut types = builtin_types();
    let builtin_names = types.keys().cloned().collect::<Vec<_>>();
    let physical = states(document, "");
    let mut parents = BTreeMap::new();
    for definition in document
        .descendants()
        .filter(|node| node.is_element() && node.tag_name().name() == "_type_definition")
    {
        let Some(name) = definition
            .attribute("name")
            .filter(|name| !name.trim().is_empty())
        else {
            continue;
        };
        let builtin = builtin_names
            .iter()
            .find(|existing| existing.eq_ignore_ascii_case(name));
        if !types.contains_key(name) {
            if let Some(existing) = builtin {
                types.insert(name.into(), types[existing].clone());
            }
        }
        let attributes = types.entry(name.to_string()).or_default();
        for attribute in definition.attributes() {
            let (name, append) = attribute.name().strip_suffix("_delta")
                .map_or((attribute.name(), false), |name| (name, true));
            if let Some(existing) = attributes.iter_mut().find(|item| item.name == name) {
                if append { existing.value.push(','); existing.value.push_str(attribute.value()); }
                else { existing.value = attribute.value().into(); }
            } else {
                attributes.push(TypeAttribute { name: name.into(), value: attribute.value().into(), append, override_value: !append });
            }
        }
        let parent = if builtin.is_some() {
            physical
                .iter()
                .find(|t| t.name.eq_ignore_ascii_case(name))
                .and_then(|t| t.parent.clone())
        } else {
            definition.attribute("parent").map(str::to_string)
        };
        parents.insert(name.to_string(), parent);
    }
    for name in parents.keys() {
        inherit(name, &parents, &mut types, &mut HashSet::new());
    }
    types
        .into_iter()
        .filter(|(name, _)| parents.contains_key(name))
        .collect()
}

fn inherit(
    name: &str,
    parents: &BTreeMap<String, Option<String>>,
    types: &mut BTreeMap<String, Vec<TypeAttribute>>,
    active: &mut HashSet<String>,
) {
    if !active.insert(name.to_string()) {
        return;
    }
    if let Some(Some(parent)) = parents.get(name) {
        inherit(parent, parents, types, active);
        let inherited = types.get(parent).cloned().unwrap_or_default();
        let attributes = types.get_mut(name).expect("defined type");
        for attribute in inherited {
            if !attributes
                .iter()
                .any(|existing| existing.name == attribute.name)
            {
                attributes.push(attribute);
            }
        }
    }
    active.remove(name);
}
