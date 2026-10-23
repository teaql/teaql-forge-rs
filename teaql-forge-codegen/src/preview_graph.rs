//! Reference-preserving model graph for the source preview's Java adaptor.
use crate::context::{RenderDomain, RenderEntity, RenderRelation};
use crate::preview_domain::{camel, pascal, snake, upper_first};
use serde_json::{Map, Value, json};
use std::collections::BTreeMap;

type Attributes = BTreeMap<String, String>;

fn reference(id: impl Into<String>) -> Value {
    json!({"__stg_ref": id.into()})
}
fn object_id(index: usize) -> String {
    format!("object:{index}")
}
fn field_id(index: usize, name: &str) -> String {
    format!("field:{index}:{name}")
}
fn reverse_id(index: usize, name: &str) -> String {
    format!("reverse:{index}:{name}")
}
fn bool_value(value: Option<&String>) -> bool {
    value.is_some_and(|value| {
        matches!(
            value
                .trim_matches(|ch| ch <= '\u{20}')
                .to_lowercase()
                .as_str(),
            "true" | "yes" | "y" | "t" | "ok" | "1" | "on" | "是" | "对" | "真" | "對" | "√"
        )
    })
}
fn attribute_values(attributes: &Attributes) -> Map<String, Value> {
    let mut values = Map::new();
    for (name, value) in attributes {
        if value.is_empty() {
            continue;
        }
        values.insert(
            name.clone(),
            match value.to_ascii_lowercase().as_str() {
                "true" | "yes" | "on" => json!(true),
                "false" | "no" | "off" => json!(false),
                _ => json!(value),
            },
        );
    }
    values.insert("__stg_attributes".into(), json!(attributes));
    values
}
fn entity_attributes(entity: &RenderEntity) -> Attributes {
    entity.metadata_ordered.iter().cloned().collect()
}
fn table(entity: &RenderEntity) -> String {
    entity
        .metadata_ordered
        .iter()
        .find(|(key, value)| key == "table" && !value.is_empty())
        .map(|(_, value)| value.clone())
        .unwrap_or_else(|| format!("{}_data", entity.name))
}
fn package(domain: &RenderDomain, entity: &RenderEntity) -> String {
    format!(
        "{}.{}",
        domain.java_full_package.as_deref().unwrap_or(""),
        camel(&entity.name).to_lowercase()
    )
}
fn full_name(domain: &RenderDomain, entity: &RenderEntity) -> String {
    format!("{}.{}", package(domain, entity), pascal(&entity.name))
}
fn present(name: &str) -> String {
    name.split('_')
        .map(upper_first)
        .collect::<Vec<_>>()
        .join(" ")
}

fn object_value(domain: &RenderDomain, index: usize, entity: &RenderEntity) -> Map<String, Value> {
    let mut values = crate::descriptor_context::context(
        domain,
        entity,
        domain.java_full_package.as_deref().unwrap_or(""),
    )["objectDescriptor"]
        .as_object()
        .cloned()
        .unwrap_or_default();
    let attrs = entity_attributes(entity);
    values.insert("name".into(), json!(entity.name));
    values.insert("tableName".into(), json!(table(entity)));
    values.insert(
        "root".into(),
        json!(!entity.has_parent() && entity.object_fields.is_empty()),
    );
    values.insert("constant".into(), json!(entity.constant));
    values.insert("humanOd".into(), json!(entity.is_human));
    values.insert(
        "parentOd".into(),
        entity
            .ancestors(domain)
            .first()
            .and_then(|parent| {
                domain
                    .entities
                    .iter()
                    .position(|item| item.name == parent.name)
            })
            .map(|index| reference(object_id(index)))
            .unwrap_or(Value::Null),
    );
    for (key, entities) in [
        ("childrenOds", entity.children(domain)),
        ("allChildrenOds", entity.descendants(domain)),
    ] {
        values.insert(
            key.into(),
            json!(
                entities
                    .iter()
                    .filter_map(|e| domain.entities.iter().position(|item| item.name == e.name))
                    .map(|index| reference(object_id(index)))
                    .collect::<Vec<_>>()
            ),
        );
    }
    values.extend(attribute_values(&attrs));
    // ObjectModelAdaptor also accepts "Name" through the actual getName bean
    // method. The capitalized form does not invoke the lowercase enhancer.
    values.entry("Name").or_insert_with(|| json!(entity.name));
    if !attrs
        .get("hasParent")
        .is_some_and(|value| !value.is_empty())
    {
        values.remove("hasParent");
    }
    let class = pascal(&entity.name);
    let own_package = package(domain, entity);
    for suffix in [
        "Request",
        "Query",
        "Expression",
        "ListExpression",
        "Checker",
    ] {
        let name = pascal(&format!("{}_{}", entity.name, snake(suffix)));
        values.insert(format!("{suffix}Name"), json!(name));
        values.insert(
            format!("full{suffix}Name"),
            json!(format!("{own_package}.{name}")),
        );
    }
    let normalize_mask = |name: &str| {
        let camel_break = regex::Regex::new(r"([a-z0-9])([A-Z])")
            .unwrap()
            .replace_all(name, "${1}_${2}")
            .into_owned();
        regex::Regex::new(r"[^A-Za-z0-9]+")
            .unwrap()
            .replace_all(&camel_break, "_")
            .trim_matches('_')
            .to_ascii_lowercase()
    };
    let requested = attrs
        .get("audit_mask_fields")
        .map(|value| {
            value
                .split(',')
                .map(|name| normalize_mask(name.trim()))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let mut masks = entity
        .fields
        .iter()
        .map(|field| (field.source_order, &field.name))
        .chain(
            entity
                .object_fields
                .iter()
                .map(|field| (field.source_order, &field.name)),
        )
        .filter(|(_, name)| requested.contains(&normalize_mask(name)))
        .collect::<Vec<_>>();
    masks.sort_by_key(|(order, _)| *order);
    values.insert(
        "canonicalAuditMaskFields".into(),
        json!(
            masks
                .into_iter()
                .map(|(_, name)| snake(name))
                .collect::<Vec<_>>()
                .join(", ")
        ),
    );
    let ancestors = entity.ancestors(domain);
    let version_owner = ancestors.last().copied().unwrap_or(entity);
    values.insert(
        "attributePredicatePrefixExprForVersion".into(),
        json!(format!(
            "{}Version",
            if version_owner.is_human {
                "whose"
            } else {
                "with"
            }
        )),
    );
    values.insert("EntityName".into(), json!(class));
    values.insert(
        "PresentTableName".into(),
        json!(table(entity).to_uppercase()),
    );
    values.insert("od_name".into(), json!(entity.name));
    values.insert("originalName".into(), json!(entity.name));
    values.insert("fullpackage".into(), json!(own_package));
    values.insert("PresentName".into(), json!(present(&entity.name)));
    values.insert(
        "fullPath".into(),
        json!(format!(
            "{}/{}",
            domain
                .java_full_package
                .as_deref()
                .unwrap_or("")
                .replace('.', "/"),
            camel(&entity.name).to_lowercase()
        )),
    );
    values.insert("fullName".into(), json!(full_name(domain, entity)));
    let bean = camel(&entity.name);
    let plural = crate::context::pluralize_name(&entity.name);
    for key in ["beanNames", "camelNameInPluralForm"] {
        values.insert(key.into(), json!(camel(&plural)));
    }
    values.insert("beanName".into(), json!(bean));
    values.insert("snakeNames".into(), json!(snake(&plural)));
    values.insert("BeanNames".into(), json!(upper_first(&camel(&plural))));
    values.insert("BeanName".into(), json!(upper_first(&bean)));
    for (key, suffix, default) in [
        ("pojoParent", "", "io.teaql.core.BaseEntity"),
        ("requestParent", "Request", "io.teaql.core.BaseRequest"),
        (
            "expressionParent",
            "Expression",
            "io.teaql.core.value.ExpressionAdaptor",
        ),
    ] {
        values.insert(
            key.into(),
            json!(
                entity
                    .ancestors(domain)
                    .first()
                    .map(|parent| format!("{}{suffix}", full_name(domain, parent)))
                    .unwrap_or_else(|| default.into())
            ),
        );
    }
    values.insert("checkerBeanName".into(), json!(format!("{bean}Checker")));
    values.insert(
        "ROOT_ID".into(),
        json!(snake(&format!("{}_id", entity.name)).to_uppercase()),
    );
    values.insert(
        "ROOT_ENTITY".into(),
        json!(snake(&entity.name).to_uppercase()),
    );
    let repository = !bool_value(attrs.get("valueObject")) && !bool_value(attrs.get("viewObject"));
    let descriptor = if repository {
        entity
            .data_service_attributes
            .get("entityClass")
            .map(String::as_str)
            .unwrap_or("io.teaql.core.meta.EntityDescriptor")
    } else {
        "io.teaql.core.meta.EntityDescriptor"
    };
    values.insert("entityDescriptorClass".into(), json!(descriptor));
    values.insert(
        "sqlRepository".into(),
        json!(repository && bool_value(entity.data_service_attributes.get("sqlRepository"))),
    );
    values.insert(
        "fullProcessorName".into(),
        json!(format!("{own_package}.{class}Processor")),
    );
    values.insert("ProcessorName".into(), json!(format!("{class}Processor")));
    values.insert("processorName".into(), json!(format!("{bean}Processor")));
    values.insert(
        "processorPath".into(),
        json!(format!(
            "{}Services/{bean}Processor",
            camel(&domain.name.replace('-', "_"))
        )),
    );
    values.insert(
        "objectBooleanAttributePredictPrefix".into(),
        json!(if entity.is_human { "who" } else { "which" }),
    );
    values.insert(
        "requestName".into(),
        json!(camel(&format!("{}_request", entity.name))),
    );
    values.insert("domain".into(), reference("domain"));
    values.insert("__stg_object_index".into(), json!(index));
    values.insert(
        "__stg_java_class".into(),
        json!("com.skynet.codegenerator.meta.ObjectDescriptor"),
    );
    values
}

fn field_value(
    domain: &RenderDomain,
    owner: usize,
    name: &str,
    attrs: Attributes,
    type_id: &str,
    type_full_name: Option<&str>,
    graphql_type: Option<&str>,
    sql_type: &str,
    target: Option<usize>,
) -> Map<String, Value> {
    let entity = &domain.entities[owner];
    let member = camel(name);
    let suffix = upper_first(&member);
    let column = attrs
        .get("column")
        .cloned()
        .unwrap_or_else(|| crate::sql::quoted_column(entity, &snake(name)));
    let mut values = Map::new();
    values.insert("name".into(), json!(name));
    for (key, value) in [
        ("simpleProperty", target.is_none()),
        ("objectProperty", target.is_some()),
        ("isId", bool_value(attrs.get("isId"))),
        ("isVersion", bool_value(attrs.get("isVersion"))),
    ] {
        values.insert(key.into(), json!(value));
    }
    values.extend(attribute_values(&attrs));
    values.insert("objectDescriptor".into(), reference(object_id(owner)));
    for name in ["isId", "isVersion"] {
        // Attached names have no Java isIsId/isIsVersion getter fallback.
        if !attrs.get(name).is_some_and(|v| !v.is_empty()) {
            values.insert(name.into(), Value::Null);
        }
    }
    values.insert(
        "__stg_java_class".into(),
        json!("com.skynet.codegenerator.meta.FieldDescriptor"),
    );
    values.insert("fieldType".into(), reference(type_id));
    values.insert(
        "referenceObject".into(),
        target
            .map(|index| reference(object_id(index)))
            .unwrap_or(Value::Null),
    );
    values.insert(
        "reverseRelation".into(),
        target
            .map(|_| reference(reverse_id(owner, name)))
            .unwrap_or(Value::Null),
    );
    values.insert(
        "getter".into(),
        json!(format!(
            "{}{suffix}",
            if type_full_name == Some("java.lang.Boolean") {
                "is"
            } else {
                "get"
            }
        )),
    );
    values.insert(
        "memberConstantName".into(),
        json!(snake(&format!("{name}_property")).to_uppercase()),
    );
    values.insert("setter".into(), json!(format!("set{suffix}")));
    values.insert("memberName".into(), json!(member));
    values.insert("canonicalPropertyName".into(), json!(snake(name)));
    values.insert(
        "memberNames".into(),
        json!(camel(&crate::context::pluralize_name(name))),
    );
    values.insert("MemberName".into(), json!(suffix));
    values.insert("businessIdTypeName".into(), json!(suffix));
    values.insert(
        "businessIdFullTypeName".into(),
        json!(format!("{}.{suffix}", full_name(domain, entity))),
    );
    values.insert("PresentName".into(), json!(present(name)));
    values.insert("SQLPresentName".into(), json!(column.to_uppercase()));
    values.insert("updater".into(), json!(format!("update{suffix}")));
    let prefix = format!("{}{suffix}", if entity.is_human { "whose" } else { "with" });
    let ending = if entity.is_human { "s" } else { "ing" };
    let boolean = format!(
        "{}{suffix}",
        if entity.is_human {
            "whoAre"
        } else {
            "whichAre"
        }
    );
    let negative = format!(
        "{}{suffix}",
        if entity.is_human {
            "whoAreNot"
        } else {
            "whichAreNot"
        }
    );
    for (key, value) in [
        ("attributePredicatePrefixExpr", prefix.clone()),
        ("attributePredicateSuffixExpr", ending.into()),
        ("boolAttributePredicateSuffixExpr", boolean.clone()),
        ("BooleanPredicateExpr", upper_first(&boolean)),
        ("boolNegativeAttributePredicateExpr", negative.clone()),
        ("BooleanNegativePredicateExpr", upper_first(&negative)),
        ("AttributePredicateExpr", upper_first(&prefix)),
        ("snakeAttributePredicateExpr", snake(&prefix)),
        (
            "stringContainsPredicateExpr",
            format!("{prefix}Contain{ending}"),
        ),
        (
            "StringContainsPredicateExpr",
            upper_first(&format!("{prefix}Contain{ending}")),
        ),
        (
            "StringNotContainsPredicateExpr",
            upper_first(&format!("{prefix}NotContain{ending}")),
        ),
        (
            "StringStartsWithPredicateExpr",
            upper_first(&format!("{prefix}Start{ending}With")),
        ),
        (
            "StringEndsWithPredicateExpr",
            upper_first(&format!("{prefix}End{ending}With")),
        ),
        (
            "snakeStringContainsPredicateExpr",
            snake(&format!("{prefix}Contain{ending}")),
        ),
        ("snakeBooleanPredicateExpr", snake(&boolean)),
        ("snakeBooleanNegativePredicateExpr", snake(&negative)),
    ] {
        values.insert(key.into(), json!(value));
    }
    values.insert(
        "tableName".into(),
        json!(
            attrs
                .get("table")
                .cloned()
                .unwrap_or_else(|| snake(&format!("{}_data", entity.name)))
        ),
    );
    values.insert("columnName".into(), json!(column));
    values.insert("sqlType".into(), json!(sql_type));
    values.insert("graphqlType".into(), json!(graphql_type));
    values.insert("required".into(), json!(!bool_value(attrs.get("optional"))));
    let original = attrs
        .get("candidates")
        .map(|value| value.replace(',', "|"))
        .or_else(|| attrs.get("sampleValue").cloned())
        .unwrap_or_else(|| "null".into());
    values.insert(
        "originalValue".into(),
        json!(format!(
            "{}{original}{}",
            attrs
                .get("zh_CN")
                .map(|value| format!("{value}:"))
                .unwrap_or_default(),
            if bool_value(attrs.get("optional")) {
                "?"
            } else {
                ""
            }
        )),
    );
    values.insert("originalName".into(), json!(name));
    values
}

fn reverse_value(
    domain: &RenderDomain,
    parent: usize,
    child: usize,
    relation: &RenderRelation,
    attrs: &Attributes,
) -> Map<String, Value> {
    let child_entity = &domain.entities[child];
    let many = child_entity
        .object_fields
        .iter()
        .filter(|other| other.target_struct == domain.entities[parent].rust_struct)
        .count()
        > 1;
    let one = attrs.get("relationType").map(String::as_str) == Some("oneToOne");
    let role = if many {
        format!("As{}", pascal(&relation.name))
    } else {
        String::new()
    };
    let member = attrs.get("reverseName").cloned().unwrap_or_else(|| {
        format!(
            "{}{}{role}",
            camel(&child_entity.name),
            if one { "" } else { "List" }
        )
    });
    let plural = attrs
        .get("reverseName")
        .map(|value| upper_first(value))
        .unwrap_or_else(|| {
            format!(
                "{}{role}",
                pascal(&crate::context::pluralize_name(&child_entity.name))
            )
        });
    let bean = format!("{}{role}", camel(&child_entity.name));
    let data = json!({
        "parent": reference(object_id(parent)), "child": reference(object_id(child)),
        "property": reference(field_id(child, &relation.name)),
        "__stg_java_class":"com.skynet.codegenerator.meta.ReverseRelation",
        "__stg_attributes":{},
        "getter": format!("get{}", upper_first(&member)), "setter": format!("set{}", upper_first(&member)),
        "updater": format!("update{}", upper_first(&member)), "appender": format!("add{}{role}", upper_first(&camel(&child_entity.name))),
        "memberConstantName": format!("{}_property", snake(&member)).to_uppercase(),
        "MemberName": upper_first(&member), "MemberNames": plural, "memberName": member,
        "snakeName": snake(&member), "snakeNames": snake(&plural), "beanName": bean,
        "BeanName": format!("{}{role}", upper_first(&camel(&child_entity.name))), "hasMany": many, "oneToOne": one,
    });
    data.as_object().unwrap().clone()
}

fn populate_types(domain: &RenderDomain, graph: &mut Map<String, Value>) {
    let builtins = teaql_forge_model::types::builtin_types();
    for state in &domain.source_type_states {
        let key = format!("type:{}", state.name);
        let existing = graph.get(&key).cloned().or_else(|| {
            graph
                .iter()
                .find(|(key, _)| {
                    key.strip_prefix("type:")
                        .is_some_and(|name| name.eq_ignore_ascii_case(&state.name))
                })
                .map(|(_, value)| value.clone())
        });
        let mut value=existing.and_then(|v| v.as_object().cloned()).unwrap_or_else(|| {
            let attributes=domain.source_types.iter().find(|(name,_)| name.eq_ignore_ascii_case(&state.name))
                .map(|(_,attrs)|attrs).or_else(||builtins.get(&state.name));
            let attrs=attributes.into_iter().flatten().map(|a|(a.name.clone(),a.value.clone())).collect();
            let mut data=attribute_values(&attrs);
            data.entry("name").or_insert_with(||json!(state.name));
            data.insert("__stg_java_class".into(),json!("com.skynet.codegenerator.meta.Type"));
            data.insert("fullName".into(),json!(attrs.get("javaType")));
            data.insert("camelNameInPluralForm".into(),Value::Null);
            for (property,suffix) in [("fullRequestName","RequestName"),("fullExpressionName","ExpressionName"),("beanName","Request beanName")] {
                data.insert(property.into(),json!({"__stg_getter_fault":format!("类型 {} 是简单类型， 不存在{suffix}",state.name)}));
            }
            data
        });
        value.insert(
            "__stg_java_type_state".into(),
            serde_json::to_value(state).unwrap(),
        );
        value.entry("parentType").or_insert_with(|| {
            state
                .parent_type
                .as_ref()
                .map(|name| reference(format!("type:{name}")))
                .unwrap_or(Value::Null)
        });
        graph.insert(key, Value::Object(value));
    }
    for (key, value) in graph.iter_mut() {
        let Some(name) = key.strip_prefix("type:") else {
            continue;
        };
        let Some(state) = domain
            .source_type_states
            .iter()
            .find(|s| s.name.eq_ignore_ascii_case(name))
        else {
            continue;
        };
        value["__stg_java_type_state"] = serde_json::to_value(state).unwrap();
        if value.get("parentType").is_none() {
            value["parentType"] = state
                .parent_type
                .as_ref()
                .map(|name| reference(format!("type:{name}")))
                .unwrap_or(Value::Null);
        }
    }
    graph["domain"]["types"] = json!(
        domain
            .source_type_states
            .iter()
            .map(|s| reference(format!("type:{}", s.name)))
            .collect::<Vec<_>>()
    );
}

pub fn context(domain: &RenderDomain) -> (Map<String, Value>, Map<String, Value>) {
    context_with_extra(domain, |_, _| {})
}

pub fn context_with_extra(
    domain: &RenderDomain,
    extra: impl FnOnce(&RenderDomain, &mut Map<String, Value>),
) -> (Map<String, Value>, Map<String, Value>) {
    let mut graph = Map::new();
    let types = teaql_forge_model::types::builtin_types();
    let mut fields: Vec<Vec<(usize, String, bool)>> = vec![Vec::new(); domain.entities.len()];
    let mut reverse: Vec<Vec<Value>> = vec![Vec::new(); domain.entities.len()];
    for (index, entity) in domain.entities.iter().enumerate() {
        for field in &entity.fields {
            let type_attrs = if field.type_attributes.is_empty() {
                types
                    .get(&field.source_type_name)
                    .cloned()
                    .unwrap_or_default()
            } else {
                field.type_attributes.clone()
            };
            let type_attrs = type_attrs
                .iter()
                .map(|a| (a.name.clone(), a.value.clone()))
                .collect::<Attributes>();
            let mut attrs = type_attrs.clone();
            attrs.extend(field.metadata.clone());
            // Java appends implicit ID/version descriptors after processing
            // _value rows. Those rows cannot attach candidates to an implicit ID.
            if field.line_number == 0 {
                attrs.remove("candidates");
                if let Some(value) = type_attrs.get("candidates") {
                    attrs.insert("candidates".into(), value.clone());
                }
            }
            if entity
                .metadata_ordered
                .iter()
                .any(|(key, value)| key == "identifier" && value == &field.name)
            {
                attrs.insert("identifier".into(), "true".into());
            }
            // The physical field metadata already contains the original column.
            // Seed-row reconstruction can truncate that column before previews
            // evaluate getCandidateValue and the JSON-literal guard.
            if (entity.constant || (!entity.has_parent() && entity.object_fields.is_empty()))
                && !attrs.contains_key("candidates")
            {
                if let Some(value) = attrs.get("sampleValue").cloned() {
                    attrs.insert("candidates".into(), value);
                }
            }
            let type_id = format!("type:{}", field.source_type_name);
            let type_full_name = type_attrs.get("javaType").map(String::as_str);
            let mut type_value = Map::new();
            type_value.insert(
                "__stg_java_class".into(),
                json!("com.skynet.codegenerator.meta.Type"),
            );
            type_value.insert("name".into(), json!(field.source_type_name));
            type_value.extend(attribute_values(&type_attrs));
            type_value.insert("fullName".into(), json!(type_full_name));
            type_value.insert("camelNameInPluralForm".into(), Value::Null);
            for (property, suffix) in [
                ("fullRequestName", "RequestName"),
                ("fullExpressionName", "ExpressionName"),
                ("beanName", "Request beanName"),
            ] {
                type_value.insert(property.into(), json!({"__stg_getter_fault": format!("类型 {} 是简单类型， 不存在{suffix}", field.source_type_name)}));
            }
            graph.insert(type_id.clone(), Value::Object(type_value));
            let value = field_value(
                domain,
                index,
                &field.name,
                attrs,
                &type_id,
                type_full_name,
                type_attrs.get("graphqlType").map(String::as_str),
                &crate::sql::field_type(entity, field),
                None,
            );
            graph.insert(field_id(index, &field.name), Value::Object(value));
            fields[index].push((
                field.source_order,
                field.name.clone(),
                bool_value(type_attrs.get("isBaseEntityField")),
            ));
        }
        for relation in &entity.object_fields {
            let Some(target) = domain
                .entities
                .iter()
                .position(|item| item.rust_struct == relation.target_struct)
            else {
                continue;
            };
            let target_entity = &domain.entities[target];
            let inherited = entity_attributes(target_entity);
            let mut attrs = inherited.clone();
            attrs.extend(relation.metadata.clone());
            if entity
                .metadata_ordered
                .iter()
                .any(|(key, value)| key == "identifier" && value == &relation.name)
            {
                attrs.insert("identifier".into(), "true".into());
            }
            if entity.constant && !attrs.contains_key("candidates") {
                if let Some(value) = attrs.get("sampleValue").cloned() {
                    attrs.insert("candidates".into(), value);
                }
            }
            let type_id = format!("object-type:{target}");
            let mut type_value = Map::new();
            type_value.insert(
                "__stg_java_class".into(),
                json!("com.skynet.codegenerator.meta.ObjectType"),
            );
            type_value.insert("name".into(), Value::Null);
            type_value.extend(attribute_values(&inherited));
            let target_class = full_name(domain, target_entity);
            type_value.insert("fullName".into(), json!(target_class));
            type_value.insert("refer".into(), reference(object_id(target)));
            type_value.insert(
                "camelNameInPluralForm".into(),
                json!(camel(&crate::context::pluralize_name(&target_entity.name))),
            );
            type_value.insert(
                "fullRequestName".into(),
                json!(format!("{target_class}Request")),
            );
            type_value.insert(
                "fullExpressionName".into(),
                json!(format!("{target_class}Expression")),
            );
            type_value.insert("beanName".into(), json!(camel(&target_entity.name)));
            graph.insert(type_id.clone(), Value::Object(type_value));
            let value = field_value(
                domain,
                index,
                &relation.name,
                attrs.clone(),
                &type_id,
                Some(&target_class),
                Some(&upper_first(&camel(&target_entity.name))),
                &crate::sql::relation_type(domain, entity),
                Some(target),
            );
            graph.insert(field_id(index, &relation.name), Value::Object(value));
            graph.insert(
                reverse_id(index, &relation.name),
                Value::Object(reverse_value(domain, target, index, relation, &attrs)),
            );
            reverse[target].push(reference(reverse_id(index, &relation.name)));
            fields[index].push((
                relation.source_order,
                relation.name.clone(),
                bool_value(inherited.get("isBaseEntityField")),
            ));
        }
    }
    for (index, entity) in domain.entities.iter().enumerate() {
        fields[index].sort_by_key(|(order, _, _)| *order);
        let mut value = object_value(domain, index, entity);
        for (key, all) in [("fields", true), ("properties", false)] {
            value.insert(
                key.into(),
                json!(
                    fields[index]
                        .iter()
                        .filter(|(_, _, base)| all || !base)
                        .map(|(_, name, _)| reference(field_id(index, name)))
                        .collect::<Vec<_>>()
                ),
            );
        }
        value.insert("reverseRelations".into(), json!(reverse[index]));
        value.insert("reverseRelationCount".into(), json!(reverse[index].len()));
        if let Some(service) = entity.data_service.as_deref().and_then(|name| {
            teaql_forge_model::data_services::find(&domain.source_data_services, name)
        }) {
            value.insert(
                "dataService".into(),
                crate::data_service_properties::preview_value(service),
            );
        }
        graph.insert(object_id(index), Value::Object(value));
    }
    let objects = (0..domain.entities.len())
        .map(|index| reference(object_id(index)))
        .collect();
    let services = teaql_forge_model::data_services::all(&domain.source_data_services)
        .into_iter()
        .map(crate::data_service_properties::preview_value)
        .collect();
    graph.insert(
        "domain".into(),
        crate::preview_domain::domain_value(domain, objects, services),
    );
    populate_types(domain, &mut graph);
    crate::preview_rust::enhance(domain, &mut graph);
    extra(domain, &mut graph);
    crate::preview_beans::enhance(domain, &mut graph);
    crate::business_id::enhance(domain, &mut graph);
    crate::preview_objects::enhance(domain, &mut graph);
    crate::preview_beans::preserve_getters(domain, &mut graph);
    crate::preview_fields::enhance(domain, &mut graph);
    crate::preview_domain_collections::enhance(&mut graph);
    crate::preview_beans::attach_collection_getters(&mut graph);
    let mut vars = Map::new();
    vars.insert("domain".into(), reference("domain"));
    (vars, graph)
}
