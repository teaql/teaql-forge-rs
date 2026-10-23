//! SQL metadata exposed by the source model's preview adaptors.
use crate::context::{RenderEntity, RenderField};
use std::collections::BTreeMap;
use std::sync::OnceLock;
use teaql_forge_model::types::TypeAttribute;

pub(crate) fn field_type(entity: &RenderEntity, field: &RenderField) -> String {
    static TYPES: OnceLock<BTreeMap<String, Vec<TypeAttribute>>> = OnceLock::new();
    let types = TYPES.get_or_init(teaql_forge_model::types::builtin_types);
    let attribute = |name: &str| {
        field.attribute(name).or_else(|| {
            types
                .get(&field.source_type_name)?
                .iter()
                .find(|attr| attr.name == name)
                .map(|attr| attr.value.as_str())
        })
    };
    let service_type = format!(
        "{}_sqlType",
        entity.data_service.as_deref().unwrap_or("postgres")
    );
    attribute(&service_type)
        .or_else(|| attribute("sqlType"))
        .unwrap_or("VARCHAR(100)")
        .replace("<max>", attribute("max").unwrap_or("100"))
}

pub(crate) fn relation_column(relation: &crate::context::RenderRelation) -> String {
    relation
        .metadata
        .get("column")
        .cloned()
        .unwrap_or_else(|| crate::preview_domain::snake(&relation.name))
}

pub(crate) fn relation_type(
    domain: &crate::context::RenderDomain,
    entity: &RenderEntity,
) -> String {
    let builtin = teaql_forge_model::types::builtin_types();
    let attributes = domain.source_types.get("id").or_else(|| builtin.get("id"));
    let attribute = |name: &str| {
        attributes
            .and_then(|attributes| attributes.iter().find(|attribute| attribute.name == name))
            .map(|attribute| attribute.value.clone())
    };
    attribute(&format!(
        "{}_sqlType",
        entity.data_service.as_deref().unwrap_or("postgres")
    ))
    .or_else(|| attribute("sqlType"))
    .unwrap_or_else(|| "BIGINT".into())
}

pub(crate) fn quoted_column(entity: &RenderEntity, name: &str) -> String {
    let quote = entity
        .data_service_attributes
        .get("quoteCharForKeywords")
        .map(String::as_str)
        .unwrap_or("");
    quoted_name(name, quote)
}

pub(crate) fn quoted_name(name: &str, quote: &str) -> String {
    let keyword = include_str!("../templates/doc/SQL2016.kw")
        .split_whitespace()
        .any(|keyword| keyword == name);
    if !keyword {
        return name.to_string();
    }
    format!("{quote}{name}{quote}")
}

pub(crate) fn field_column(entity: &RenderEntity, field: &RenderField) -> String {
    field
        .attribute("column")
        .map(str::to_string)
        .unwrap_or_else(|| quoted_column(entity, &crate::preview_domain::snake(&field.name)))
}

pub(crate) fn relation_sql_column(
    entity: &RenderEntity,
    relation: &crate::context::RenderRelation,
) -> String {
    relation
        .metadata
        .get("column")
        .cloned()
        .unwrap_or_else(|| quoted_column(entity, &relation_column(relation)))
}
