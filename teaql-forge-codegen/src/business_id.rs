//! Direct FieldDescriptor Business ID getters from the pinned Java model.
use crate::context::{RenderDomain, RenderEntity, RenderField, RenderRelation};
use serde_json::{Map, Value, json};
use std::{collections::BTreeMap, sync::OnceLock};

/// The physical descriptor getter searches each object's own fields in source
/// order, then its parent. A false marker never qualifies a non-root target.
#[derive(Clone, Copy)]
pub(crate) enum RootMember<'a> {
    Scalar(&'a RenderField),
    Relation(&'a RenderRelation),
}

impl RootMember<'_> {
    fn order(&self) -> usize {
        match self { Self::Scalar(field) => field.source_order, Self::Relation(field) => field.source_order }
    }

    fn context_root(&self, domain: &RenderDomain) -> bool {
        match self {
            Self::Scalar(field) => attribute(field, "contextRoot"),
            Self::Relation(field) => field.attribute(domain, "contextRoot"),
        }.is_some_and(crate::preview_rust::attribute_flag)
    }

    pub(crate) fn target<'a>(&self, domain: &'a RenderDomain) -> Option<&'a RenderEntity> {
        let Self::Relation(field) = self else { return None };
        domain.source_entities.iter().chain(domain.entities.iter())
            .find(|entity| entity.rust_struct == field.target_struct)
    }
}

pub(crate) fn root_field<'a>(entity: &'a RenderEntity, domain: &'a RenderDomain, context_only: bool)
    -> Option<(&'a RenderEntity, RootMember<'a>)>
{
    let mut current = Some(domain.source_entities.iter().find(|item| item.name == entity.name).unwrap_or(entity));
    let mut visited = std::collections::BTreeSet::new();
    while let Some(owner) = current.filter(|owner| visited.insert(owner.name.as_str())) {
        let mut fields = owner.fields.iter().map(RootMember::Scalar)
            .chain(owner.object_fields.iter().map(RootMember::Relation)).collect::<Vec<_>>();
        fields.sort_by_key(RootMember::order);
        if let Some(field) = fields.into_iter().find(|field| field.context_root(domain)
            || (!context_only && field.target(domain).is_some_and(|target| !target.has_parent() && target.object_fields.is_empty()))) {
            return Some((owner, field));
        }
        current = owner.metadata_ordered.iter().find(|(key, _)| key == "parent")
            .and_then(|(_, parent)| domain.source_entities.iter().chain(domain.entities.iter()).find(|item| item.name == *parent));
    }
    None
}

pub(crate) fn root_object<'a>(entity: &'a RenderEntity, domain: &'a RenderDomain) -> &'a RenderEntity {
    root_field(entity, domain, false).and_then(|(_, member)| member.target(domain)).unwrap_or(entity)
}

pub(crate) fn scalar_context_value(field: &RenderField) -> Value {
    attribute(field, "contextRoot").filter(|value| !value.is_empty())
        .map(crate::preview_domain::attribute_value).unwrap_or(json!(false))
}

pub(crate) fn java_whitespace(ch: char) -> bool {
    matches!(ch, '\u{0009}'..='\u{000d}' | '\u{001c}'..='\u{0020}' | '\u{1680}' | '\u{2000}'..='\u{2006}' | '\u{2008}'..='\u{200a}' | '\u{2028}'..='\u{2029}' | '\u{205f}' | '\u{3000}')
}

pub(crate) fn attribute<'a>(field: &'a RenderField, name: &str) -> Option<&'a str> {
    static BUILTINS: OnceLock<BTreeMap<String, Vec<teaql_forge_model::types::TypeAttribute>>> =
        OnceLock::new();
    field.attribute(name).or_else(|| {
        if !field.type_attributes.is_empty() {
            return None;
        }
        BUILTINS
            .get_or_init(teaql_forge_model::types::builtin_types)
            .get(&field.source_type_name)?
            .iter()
            .find(|a| a.name == name)
            .map(|a| a.value.as_str())
    })
}

pub(crate) fn is_business_id(field: &RenderField) -> bool {
    matches!(
        field.source_type_name.to_ascii_lowercase().as_str(),
        "businessid" | "business_id"
    ) || attribute(field, "isBusinessId").is_some_and(crate::preview_rust::attribute_flag)
}

pub(crate) fn values<'a>(
    owner: &str,
    attribute: impl Fn(&str) -> Option<&'a str>,
) -> Map<String, Value> {
    let profile = attribute("profile").unwrap_or("daily-permuted-v1");
    let prefix = attribute("prefix")
        .filter(|v| !v.chars().all(crate::input_path::blank_char))
        .map(|v| v.trim_matches(|ch| ch <= '\u{20}').to_uppercase())
        .unwrap_or_else(|| {
            let initials = owner
                .split(|ch: char| !ch.is_ascii_alphanumeric())
                .filter_map(|word| word.chars().next())
                .map(|ch| ch.to_ascii_uppercase())
                .collect::<String>();
            if initials.is_empty() {
                "ID".into()
            } else {
                initials
            }
        });
    let integer = |name: &str, fallback: i32| {
        attribute(name)
            .and_then(crate::hutool_json::parse_integer)
            .and_then(|v| i32::try_from(v).ok())
            .unwrap_or(fallback)
    };
    let digits = integer(
        "preserve_digits",
        if profile == "daily-permuted-v1" { 6 } else { 8 },
    );
    Map::from_iter([
        ("businessIdProfile".into(), json!(profile)),
        ("businessIdPrefix".into(), json!(prefix)),
        (
            "businessIdSplitByDate".into(),
            json!(attribute("splitbydate").is_none_or(|v| v.eq_ignore_ascii_case("true"))),
        ),
        (
            "businessIdDateFormat".into(),
            json!(attribute("dateFormat").unwrap_or("yyyyMMdd")),
        ),
        (
            "businessIdReset".into(),
            json!(attribute("reset").unwrap_or("daily")),
        ),
        ("businessIdDigits".into(), json!(digits)),
        (
            "businessIdSeparator".into(),
            json!(attribute("separator").unwrap_or("-")),
        ),
        (
            "businessIdNamespace".into(),
            json!(attribute("namespace").unwrap_or(owner)),
        ),
        (
            "businessIdPolicyVersion".into(),
            json!(integer("policyVersion", 1)),
        ),
    ])
}

pub(crate) fn field_values(field: &RenderField, owner: &str) -> Map<String, Value> {
    values(owner, |name| attribute(field, name))
}

pub(crate) fn enhance(domain: &crate::context::RenderDomain, graph: &mut Map<String, Value>) {
    use crate::preview_rust::{attribute_flag, raw_type};
    let reference = |id: String| json!({"__stg_ref":id});
    let fallback = |node: &mut Map<String, Value>, name: &str, value: Value| {
        node.entry(name.to_string()).or_insert(value.clone());
        node.entry(crate::preview_domain::upper_first(name))
            .or_insert(value);
    };
    let field_ids = graph
        .keys()
        .filter(|id| id.starts_with("field:"))
        .cloned()
        .collect::<Vec<_>>();
    let mut business = std::collections::BTreeSet::new();
    for id in field_ids {
        let node = &graph[&id];
        let owner = id.split(':').nth(1).unwrap().parse::<usize>().unwrap();
        let owner_name = &domain.entities[owner].name;
        let kind = raw_type(graph, node).to_ascii_lowercase();
        let attrs = node["__stg_attributes"].as_object().unwrap();
        let is_business = matches!(kind.as_str(), "businessid" | "business_id")
            || attrs
                .get("isBusinessId")
                .and_then(Value::as_str)
                .is_some_and(attribute_flag);
        if is_business {
            business.insert(id.clone());
        }
        let config = values(owner_name, |name| attrs.get(name).and_then(Value::as_str));
        let node = graph.get_mut(&id).unwrap().as_object_mut().unwrap();
        fallback(node, "businessId", json!(is_business));
        fallback(node, "isBusinessId", json!(is_business));
        for (name, value) in config {
            fallback(node, &name, value);
        }
    }
    let mut all = Vec::new();
    for index in 0..domain.entities.len() {
        let id = format!("object:{index}");
        let fields = graph[&id]["fields"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|field| {
                field["__stg_ref"]
                    .as_str()
                    .is_some_and(|id| business.contains(id))
            })
            .cloned()
            .collect::<Vec<_>>();
        all.extend(fields.clone());
        let mut context_root = None;
        let mut business_root = None;
        let mut current = Some(index);
        let mut visited = std::collections::BTreeSet::new();
        while let Some(owner) = current.filter(|owner| visited.insert(*owner)) {
            for field in graph[&format!("object:{owner}")]["fields"]
                .as_array()
                .unwrap()
            {
                let field = &graph[field["__stg_ref"].as_str().unwrap()];
                let context = field["__stg_attributes"]["contextRoot"]
                    .as_str()
                    .is_some_and(attribute_flag);
                if context && context_root.is_none() {
                    context_root = Some(field.clone());
                }
                let target = field["referenceObject"]["__stg_ref"]
                    .as_str()
                    .and_then(|id| id.strip_prefix("object:"))
                    .and_then(|id| id.parse::<usize>().ok());
                let root = target.is_some_and(|target| {
                    !domain.entities[target].has_parent()
                        && domain.entities[target].object_fields.is_empty()
                });
                if business_root.is_none() && (context || root) {
                    business_root = Some(field.clone());
                }
            }
            current = graph[&format!("object:{owner}")]["parentOd"]["__stg_ref"]
                .as_str()
                .and_then(|id| id.strip_prefix("object:"))
                .and_then(|id| id.parse().ok());
        }
        let field_ref = |field: Option<&Value>| {
            field
                .and_then(|f| {
                    f["objectDescriptor"]["__stg_ref"]
                        .as_str()
                        .zip(f["originalName"].as_str())
                })
                .map(|(owner, name)| {
                    reference(format!(
                        "field:{}:{name}",
                        owner.strip_prefix("object:").unwrap()
                    ))
                })
                .unwrap_or(Value::Null)
        };
        let root_object = business_root
            .as_ref()
            .and_then(|f| f["referenceObject"]["__stg_ref"].as_str())
            .unwrap_or(&id)
            .to_string();
        let root_index = root_object
            .strip_prefix("object:")
            .unwrap()
            .parse::<usize>()
            .unwrap();
        let node = graph.get_mut(&id).unwrap().as_object_mut().unwrap();
        fallback(node, "businessIdFields", json!(fields));
        fallback(node, "contextRootField", field_ref(context_root.as_ref()));
        fallback(
            node,
            "businessIdRootField",
            field_ref(business_root.as_ref()),
        );
        fallback(
            node,
            "businessIdRootTypeName",
            json!(domain.entities[root_index].name),
        );
        fallback(node, "businessIdRootObject", reference(root_object));
    }
    fallback(
        graph.get_mut("domain").unwrap().as_object_mut().unwrap(),
        "businessIdFields",
        json!(all),
    );
}
