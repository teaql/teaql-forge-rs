use crate::context::{RenderDomain, RenderEntity};
use crate::java_stg::StgTemplate;
use heck::{ToPascalCase, ToSnakeCase};
use regex::Regex;
use serde_json::{Map, Value, json};

fn candidate_method(value: &str, id: u64) -> String {
    crate::preview_rust::candidate_method(value, id as i64)
}

fn java_candidate_id(value: &str) -> i64 {
    i64::from(value
        .encode_utf16()
        .fold(0_i32, |hash, unit| {
            hash.wrapping_mul(31).wrapping_add(i32::from(unit))
        })
        .wrapping_abs())
}

pub(crate) fn field_value(field: &crate::context::RenderField) -> Result<Value, String> {
    let mut value = serde_json::to_value(field).map_err(|error| error.to_string())?;
    let item = value.as_object_mut().ok_or("field is not an object")?;
    if field.xml_path.is_empty() || field.xml_path == "null" {
        item.insert("xmlPath".into(), Value::Null);
    }
    item.insert("contextRoot".into(), crate::business_id::scalar_context_value(field));
    item.insert(
        "isBusinessId".into(),
        json!(crate::business_id::is_business_id(field)),
    );
    if crate::business_id::is_business_id(field) {
        let type_name = field.name.to_pascal_case();
        item.insert("rustBusinessIdTypeName".into(), json!(type_name));
        item.insert("rustType".into(), json!(format!("crate::{type_name}")));
        item.extend(crate::business_id::field_values(field, ""));
    }
    item.insert(
        "rustTimestamp".into(),
        json!(matches!(
            field.source_type_name.as_str(),
            "createTime" | "updateTime" | "datetime" | "dateTime"
        )),
    );
    item.insert(
        "canonicalPropertyName".into(),
        Value::String(crate::preview_domain::snake(&field.name)),
    );
    item.insert("id".into(), json!(field.is_id));
    item.insert("version".into(), json!(field.is_version));
    item.insert("optional".into(), json!(!field.required));
    item.insert("isNumber".into(), json!(field.is_number()));
    item.insert("isString".into(), json!(field.is_string || field.is_text));
    // RustFieldDescriptorNaming.rustCandidates constructs Candidate(value,value)
    // from the physical attribute, independently of the constant row identity.
    let candidates = field.metadata.get("candidates").map(|raw| {
        raw.split(',').filter(|candidate| !candidate.is_empty()).map(|candidate| {
            let id = java_candidate_id(candidate);
            json!({"id": id, "rustCandidateMethodValue": crate::preview_rust::candidate_method(candidate, id), "rustCandidateFilterValue": candidate})
        }).collect::<Vec<_>>()
    }).unwrap_or_default();
    item.insert("rustCandidates".into(), Value::Array(candidates));
    item.insert("isBoolean".into(), json!(field.is_boolean));
    item.insert("isBool".into(), json!(field.is_boolean));
    item.insert("rustBoolean".into(), json!(field.is_boolean));
    let default_sample = field
        .metadata
        .get("candidates")
        .map(String::as_str)
        .or_else(|| field.metadata.get("sampleValue").map(String::as_str))
        .unwrap_or_else(|| {
            if field.source_value.is_empty() {
                "1"
            } else {
                &field.source_value
            }
        });
    let default_sample = crate::preview_rust::candidate_sample(default_sample, 1);
    item.insert("defaultSampleValue".into(), field.attribute("defaultSampleValue")
        .filter(|value| !value.is_empty()).map(crate::preview_domain::attribute_value)
        .unwrap_or_else(|| json!(default_sample)));
    item.insert(
        "rustSafeDefaultSampleValue".into(),
        json!(default_sample.replace('\\', "\\\\").replace('"', "\\\"")),
    );
    item.insert(
        "rustDefaultBooleanSampleValue".into(),
        json!(if matches!(
            default_sample.to_ascii_lowercase().as_str(),
            "true" | "on" | "yes"
        ) {
            "true"
        } else {
            "false"
        }),
    );
    item.insert(
        "isJsonLiteral".into(),
        // The source ST adaptor requests getIsJsonLiteral/isIsJsonLiteral for this
        // property name; FieldDescriptor.isJsonLiteral is exposed as jsonLiteral.
        json!(
            field
                .metadata
                .get("isJsonLiteral")
                .is_some_and(|value| value == "true")
        ),
    );
    item.insert(
        "snakeBooleanPredicateExpr".into(),
        json!(format!("which_are_{}", field.rust_storage_member_name)),
    );
    item.insert(
        "snakeBooleanNegativePredicateExpr".into(),
        json!(format!("which_are_not_{}", field.rust_storage_member_name)),
    );
    item.insert(
        "createFunction".into(),
        json!(field.automatic_function("createFunction")),
    );
    item.insert(
        "updateFunction".into(),
        json!(field.automatic_function("updateFunction")),
    );
    Ok(value)
}

pub(crate) fn root_field_value(field: &crate::context::RenderField) -> Result<Value, String> {
    field_value(field)
}

fn relation_value(relation: &crate::context::RenderReverseRelation) -> Result<Value, String> {
    let mut value = serde_json::to_value(relation).map_err(|error| error.to_string())?;
    if relation.xml_path.is_empty() || relation.xml_path == "null" {
        value["xmlPath"] = Value::Null;
    }
    let item = value.as_object_mut().ok_or("relation is not an object")?;
    let descriptor = format!(
        "#[teaql(reverse_relation(name = \"{}\", target = \"{}\", local_key = \"id\", foreign_key = \"{}\"{}))]",
        relation.rust_member_name,
        relation.target_struct,
        relation.local_key,
        if relation.many { ", many" } else { "" }
    );
    item.insert("rustDescriptorAttr".into(), Value::String(descriptor));
    item.insert(
        "rustMemberNames".into(),
        Value::String(relation.rust_member_names.clone()),
    );
    item.insert(
        "property".into(),
        json!({"rustStorageFieldName": relation.local_key}),
    );
    item.insert("child".into(), json!({"rustStructName": relation.target_struct, "rustPluralName": relation.target_plural, "rustModuleName": relation.target_module}));
    item.insert("rustCountAggregateMethod".into(), json!({"name": format!("count_{}", relation.rust_member_names), "aliasName": format!("count_{}_as", relation.rust_member_names), "defaultAlias": format!("count_{}", relation.rust_member_names)}));
    Ok(value)
}

pub(crate) fn object_field_value(
    relation: &crate::context::RenderRelation,
) -> Result<Value, String> {
    let mut value = serde_json::to_value(relation).map_err(|error| error.to_string())?;
    if relation.xml_path.is_empty() || relation.xml_path == "null" {
        value["xmlPath"] = Value::Null;
    }
    let item = value
        .as_object_mut()
        .ok_or("object field is not an object")?;
    item.insert(
        "fieldType".into(),
        json!({"refer": {"name": relation.target, "rustStructName": relation.target_struct, "rustPluralName": relation.target_plural}}),
    );
    let candidates = relation
        .candidates
        .iter()
        .map(|candidate| {
            json!({
                "rustCandidateMethodValue": candidate_method(&crate::preview_domain::pascal(&candidate.rust_candidate_filter_value.to_lowercase()), candidate.id),
                "rustCandidateFilterValue": candidate.rust_candidate_filter_value,
                "id": candidate.id,
            })
        })
        .collect::<Vec<_>>();
    item.insert("candidateForConst".into(), Value::Array(candidates));
    item.insert(
        "objectDescriptor".into(),
        json!({"rustPluralName": relation.target_plural}),
    );
    Ok(value)
}

pub(crate) fn descriptor_value(entity: &RenderEntity) -> Result<Value, String> {
    descriptor_value_with_profile(entity, "camelCase")
}

pub(crate) fn descriptor_value_with_profile(
    entity: &RenderEntity,
    profile: &str,
) -> Result<Value, String> {
    let mut descriptor = serde_json::to_value(entity).map_err(|error| error.to_string())?;
    let item = descriptor
        .as_object_mut()
        .ok_or("entity is not an object")?;
    item.insert("name".into(), Value::String(entity.display_name.clone()));
    item.insert("od_name".into(), Value::String(entity.name.clone()));
    item.insert(
        "rustBehaviorName".into(),
        Value::String(format!("{}Behavior", entity.rust_struct)),
    );
    item.insert(
        "rustStructName".into(),
        Value::String(entity.rust_struct.clone()),
    );
    item.insert(
        "rustPluralName".into(),
        Value::String(entity.rust_plural.clone()),
    );
    item.insert(
        "rustModuleName".into(),
        Value::String(entity.rust_module.clone()),
    );
    item.insert(
        "rustTableName".into(),
        Value::String(entity.table_name.clone()),
    );
    item.insert("xmlPath".into(), Value::Null);
    item.insert(
        "dataService".into(),
        entity
            .data_service
            .as_ref()
            .map(|name| json!({"name": crate::data_service_properties::public_name(name, &entity.data_service_attributes)}))
            .unwrap_or(Value::Null),
    );
    item.insert(
        "audit_mask_fields".into(),
        Value::String(entity.audit_mask_fields.join(",")),
    );
    let mut mask_fields = entity
        .fields
        .iter()
        .map(|field| (field.source_order, field.name.as_str()))
        .chain(
            entity
                .object_fields
                .iter()
                .map(|relation| (relation.source_order, relation.name.as_str())),
        )
        .filter(|(_, name)| {
            entity
                .audit_mask_fields
                .iter()
                .any(|mask| mask.to_snake_case() == name.to_snake_case())
        })
        .collect::<Vec<_>>();
    mask_fields.sort_by_key(|(order, _)| *order);
    item.insert(
        "canonicalAuditMaskFields".into(),
        Value::String(
            mask_fields
                .into_iter()
                .map(|(_, name)| name.to_snake_case())
                .collect::<Vec<_>>()
                .join(", "),
        ),
    );
    let business_fields = entity
        .fields
        .iter()
        .filter(|field| crate::business_id::is_business_id(field))
        .map(|field| {
            let mut value = field_value(field)?;
            let type_name = field.name.to_pascal_case();
            value["rustBusinessIdSlotTypeName"] =
                json!(format!("{}{}BusinessIdSlot", entity.rust_struct, type_name));
            value
                .as_object_mut()
                .unwrap()
                .extend(crate::business_id::field_values(field, &entity.name));
            value["objectDescriptor"] = json!({"rustModuleName": entity.rust_module});
            Ok::<Value, String>(value)
        })
        .collect::<Result<Vec<_>, _>>()?;
    item.insert("businessIdFields".into(), Value::Array(business_fields));
    let mut all_fields = entity
        .fields
        .iter()
        .map(|field| {
            (
                field.source_order,
                json!({
                    "canonicalPropertyName": field.rust_storage_member_name,
                    "jsonFieldName": crate::preview_beans::json_field_name(&field.name, profile),
                }),
            )
        })
        .collect::<Vec<_>>();
    for relation in &entity.object_fields {
        all_fields.push((
            relation.source_order,
            json!({
                "canonicalPropertyName": crate::preview_domain::snake(&relation.name),
                "jsonFieldName": crate::preview_beans::json_field_name(&relation.name, profile),
            }),
        ));
    }
    all_fields.sort_by_key(|(order, _)| *order);
    item.insert(
        "fields".into(),
        Value::Array(all_fields.into_iter().map(|(_, field)| field).collect()),
    );
    item.insert(
        "rustObjectFields".into(),
        Value::Array(
            entity
                .object_fields
                .iter()
                .map(|relation| {
                    let mut value = object_field_value(relation)?;
                    value["objectDescriptor"]["rustPluralName"] =
                        Value::String(entity.rust_plural.clone());
                    Ok::<Value, String>(value)
                })
                .collect::<Result<Vec<_>, _>>()?,
        ),
    );
    item.insert(
        "constantObjectFields".into(),
        Value::Array(
            entity
                .constant_object_fields
                .iter()
                .map(|relation| {
                    let mut value = object_field_value(relation)?;
                    value["objectDescriptor"]["rustPluralName"] =
                        Value::String(entity.rust_plural.clone());
                    Ok::<Value, String>(value)
                })
                .collect::<Result<Vec<_>, _>>()?,
        ),
    );
    item.insert(
        "rustScalarFields".into(),
        Value::Array(
            entity
                .fields
                .iter()
                .map(|field| {
                    let mut value = if entity.object_fields.is_empty()
                        && !entity.constant
                        && !entity.has_parent()
                    {
                        root_field_value(field)?
                    } else {
                        field_value(field)?
                    };
                    value["snakeBooleanPredicateExpr"] = json!(format!(
                        "{}_{}",
                        entity.bool_predicate_prefix, field.rust_storage_member_name
                    ));
                    value["snakeBooleanNegativePredicateExpr"] = json!(format!(
                        "{}_not_{}",
                        entity.bool_predicate_prefix, field.rust_storage_member_name
                    ));
                    if crate::business_id::is_business_id(field) {
                        value["rustBusinessIdSlotTypeName"] = json!(format!(
                            "{}{}BusinessIdSlot",
                            entity.rust_struct,
                            field.name.to_pascal_case()
                        ));
                        value["objectDescriptor"] = json!({"rustModuleName": entity.rust_module});
                    }
                    Ok::<Value, String>(value)
                })
                .collect::<Result<Vec<_>, _>>()?,
        ),
    );
    item.insert(
        "rustOneToOneRelations".into(),
        Value::Array(
            entity
                .one_to_one_relations
                .iter()
                .map(relation_value)
                .collect::<Result<Vec<_>, _>>()?,
        ),
    );
    item.insert(
        "rustOneToManyRelations".into(),
        Value::Array(
            entity
                .one_to_many_relations
                .iter()
                .map(relation_value)
                .collect::<Result<Vec<_>, _>>()?,
        ),
    );
    Ok(descriptor)
}

pub(crate) fn render_entity(
    _domain: &RenderDomain,
    entity: &RenderEntity,
) -> Result<String, String> {
    let descriptor = descriptor_value(entity)?;
    let mut vars = Map::new();
    vars.insert("domain".into(), Value::Null);
    vars.insert("objectDescriptor".into(), descriptor);
    let rendered = StgTemplate::parse(include_str!(
        "../templates/rust-lib-core/entity/struct.rs.stg"
    ))?
    .render(vars, "")?;
    let required = Regex::new(r"(?m)^        if !self\.is_loaded\(([^\n]+)\) \{\n            (teaql_core::eval::EvalResult::NotLoaded[^\n]+)\n        \} else \{\n            (teaql_core::eval::EvalResult::Value[^\n]+)\n        \}\n    \}").map_err(|error| error.to_string())?;
    let mut rendered = required.replace_all(&rendered, "        if !self.is_loaded($1) {\n                    $2\n                } else {\n                    $3\n                }}").into_owned();
    let optional = Regex::new(
        r"(?m)^        if !self\.is_loaded\(([^\n]+)\) \{\n((?: {8,}[^\n]*\n)+)        \}\n    \}",
    )
    .map_err(|error| error.to_string())?;
    rendered = optional
        .replace_all(&rendered, |captures: &regex::Captures<'_>| {
            if !captures[2].contains("match &self.") {
                return captures[0].to_string();
            }
            let body = captures[2]
                .lines()
                .map(|line| format!("        {line}\n"))
                .collect::<String>();
            format!(
                "        if !self.is_loaded({}) {{\n{}                }}}}",
                &captures[1], body
            )
        })
        .into_owned();
    let relation_resolver = Regex::new(r"(?m)(            self\.__teaql_runtime_state\(\)\.resolve_entity\([^\n]+\))\n        \}\)")
        .map_err(|error| error.to_string())?;
    rendered = relation_resolver
        .replace_all(&rendered, "$1})")
        .into_owned();
    let optional_relation = Regex::new(r"(?m)^                (\.and_then\(\|id\| self\.__teaql_runtime_state\(\)\.resolve_entity\(id\)\))\n        \}\)").map_err(|error| error.to_string())?;
    rendered = optional_relation
        .replace_all(&rendered, "                            $1})")
        .into_owned();
    for field in &entity.fields {
        if matches!(
            field.source_type_name.as_str(),
            "createTime" | "updateTime" | "datetime" | "dateTime"
        ) {
            let name = &field.rust_storage_member_name;
            rendered = rendered.replace(
                &format!("    }}\n\n    pub fn changed_{name}"),
                &format!("    }}\n    pub fn changed_{name}"),
            );
        }
    }
    if entity
        .fields
        .iter()
        .any(|field| crate::business_id::is_business_id(field))
    {
        Ok(format!("{rendered}\n"))
    } else {
        Ok(format!("{rendered}\n\n"))
    }
}
