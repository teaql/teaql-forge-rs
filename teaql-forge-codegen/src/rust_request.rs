use crate::context::{RenderDomain, RenderEntity};
use crate::java_stg::StgTemplate;
use serde_json::{Map, Value, json};

pub(crate) fn render_request(
    domain: &RenderDomain,
    entity: &RenderEntity,
) -> Result<String, String> {
    let mut descriptor = crate::rust_entity::descriptor_value(entity)?;
    let item = descriptor
        .as_object_mut()
        .ok_or("entity is not an object")?;
    if let Some(relations) = item
        .get_mut("rustOneToManyRelations")
        .and_then(Value::as_array_mut)
    {
        for relation in relations {
            let target = relation["targetStruct"].as_str().unwrap_or_default();
            let child = domain
                .source_entities
                .iter()
                .find(|candidate| candidate.rust_struct == target)
                .ok_or_else(|| format!("missing reverse relation child {target}"))?;
            let plural = relation["targetPlural"]
                .as_str()
                .unwrap_or_default()
                .to_string();
            let member_names = relation["rustMemberNames"]
                .as_str()
                .unwrap_or_default()
                .to_string();
            let mut methods = Vec::new();
            for field in &child.fields {
                let numeric = field.is_number();
                let comparable = matches!(
                    field.source_type_name.as_str(),
                    "createTime" | "updateTime" | "dateTime" | "date"
                );
                if !numeric && !comparable {
                    continue;
                }
                const NUMERIC: [(&str, &str, &str); 8] = [
                    ("sum", "sum", "sum"),
                    ("min", "min", "min"),
                    ("max", "max", "max"),
                    ("avg", "avg", "avg"),
                    ("standard_deviation", "stddev", "stdDev"),
                    (
                        "square_root_of_population_standard_deviation",
                        "stddev_pop",
                        "stdDevPop",
                    ),
                    ("sample_variance", "var_samp", "varSamp"),
                    ("sample_population_variance", "var_pop", "varPop"),
                ];
                const COMPARABLE: [(&str, &str, &str); 2] =
                    [("min", "min", "min"), ("max", "max", "max")];
                let specs: &[(&str, &str, &str)] = if numeric { &NUMERIC } else { &COMPARABLE };
                for &(verb, query_method, alias_prefix) in specs {
                    let name = format!(
                        "{verb}_{}_of_{member_names}",
                        field.rust_storage_member_name
                    );
                    methods.push(json!({
                        "name": name,
                        "aliasName": format!("{name}_as"),
                        "defaultAlias": name,
                        "childPluralName": plural,
                        "relationMemberNames": member_names,
                        "queryMethodName": query_method,
                        "fieldStorageName": field.rust_storage_field_name,
                        "queryAlias": format!("{alias_prefix}_{}", field.rust_storage_member_name),
                    }));
                }
            }
            relation["rustFieldAggregateMethods"] = Value::Array(methods);
        }
    }
    let scalar = item
        .get("rustScalarFields")
        .and_then(Value::as_array)
        .ok_or("scalar fields are not an array")?;
    let mut fields = entity
        .fields
        .iter()
        .zip(scalar)
        .map(|(field, value)| (field.source_order, value.clone()))
        .collect::<Vec<_>>();
    for relation in &entity.object_fields {
        let mut value = crate::rust_entity::object_field_value(relation)?;
        value["objectDescriptor"]["rustPluralName"] = Value::String(entity.rust_plural.clone());
        let field = value.as_object_mut().ok_or("relation is not an object")?;
        field.insert("id".into(), json!(false));
        field.insert("version".into(), json!(false));
        field.insert("isNumber".into(), json!(false));
        field.insert("isString".into(), json!(false));
        fields.push((relation.source_order, value));
    }
    fields.sort_by_key(|(order, _)| *order);
    item.insert(
        "fields".into(),
        Value::Array(fields.into_iter().map(|(_, value)| value).collect()),
    );
    item.insert("childrenOds".into(), json!(entity.children(domain).iter().map(|child| json!({
        "rustModuleName":child.rust_module, "rustPluralName":child.rust_plural,
        "rustTypeMatchList":std::iter::once(*child).chain(child.descendants(domain)).map(|entity|format!("\"{}\"",entity.rust_struct)).collect::<Vec<_>>().join(", ")
    })).collect::<Vec<_>>()));
    let template =
        StgTemplate::parse(include_str!("../templates/rust-lib-core/request/index.stg"))?;
    let vars = || -> Result<Map<String, Value>, String> {
        let mut vars = Map::new();
        vars.insert("domain".into(), Value::Null);
        vars.insert("objectDescriptor".into(), descriptor.clone());
        Ok(vars)
    };
    let reference = template.render_template("entityReferenceImpl", vars()?)?;
    let request = template.render_template("request", vars()?)?;
    let mut request = request.replace("    \n    pub fn purpose", "\n    pub fn purpose");
    for relation in &entity.object_fields {
        request = request.replacen(
            &format!(
                "\n\n    /// Complex relation filter for `{}`.",
                relation.rust_member_name
            ),
            &format!(
                "\n    /// Complex relation filter for `{}`.",
                relation.rust_member_name
            ),
            1,
        );
    }
    for field in &entity.fields {
        if field.is_boolean || crate::business_id::is_business_id(field) {
            request = request.replace(
                &format!(
                    "    }}\n\n    pub fn order_by_{}_asc(mut",
                    field.rust_storage_member_name
                ),
                &format!(
                    "    }}\n    pub fn order_by_{}_asc(mut",
                    field.rust_storage_member_name
                ),
            );
        }
    }
    Ok(format!(
        "use std::marker::PhantomData;\n\nuse serde_json::Value as JsonValue;\nuse teaql_core::{{Aggregate, AggregateFunction, EntityDescriptor, Expr, SelectQuery, SmartList}};\nuse teaql_runtime::RuntimeError;\n\nuse crate::request_support::*;\n\n{reference}\n\n{request}\n"
    ))
}
