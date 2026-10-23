use crate::context::{
    RenderDomain, RenderEntity, RenderField, RenderRelation, RenderReverseRelation,
};
use heck::ToShoutySnakeCase;
use serde_json::{Map, Value, json};

fn full_class(package: &str, entity: &RenderEntity) -> String {
    format!(
        "{package}.{}.{}",
        crate::shadow::java_package_segment(&entity.name),
        entity.rust_struct
    )
}

fn full_request(package: &str, entity: &RenderEntity) -> String {
    format!("{package}.{}.{}", crate::shadow::java_package_segment(&entity.name), entity.type_name_with_suffix("request"))
}

fn type_name(field: &RenderField) -> &str {
    if let Some(name) = field.attribute("javaType") {
        return name;
    }
    match field.source_type_name.as_str() {
        "id" | "version" | "long" => "java.lang.Long",
        "integer" => "java.lang.Integer",
        "boolean" | "bool" => "java.lang.Boolean",
        "number" | "decimal" | "money" => "java.math.BigDecimal",
        "float" => "java.lang.Float",
        "double" => "java.lang.Double",
        "time" => "java.time.LocalTime",
        "date" => "java.time.LocalDate",
        "datetime" | "dateTime" | "createTime" | "updateTime" => "java.time.LocalDateTime",
        _ => "java.lang.String",
    }
}

fn is_business_id(field: &RenderField) -> bool {
    crate::business_id::is_business_id(field)
}

fn field_value(field: &RenderField, entity: &RenderEntity, package: &str) -> Value {
    let name = crate::preview_domain::camel(&field.name);
    let suffix = crate::preview_domain::pascal(&field.name);
    let prefix = if entity.is_human { "whose" } else { "with" };
    let bool_prefix = if entity.is_human {
        "whoAre"
    } else {
        "whichAre"
    };
    let bool_negative = if entity.is_human {
        "whoAreNot"
    } else {
        "whichAreNot"
    };
    let is_string = matches!(
        field.source_type_name.as_str(),
        "string" | "text" | "password" | "uniStr" | "jsonMe" | "businessId" | "business_id"
    );
    let is_date = matches!(
        field.source_type_name.as_str(),
        "date" | "datetime" | "dateTime" | "createTime" | "updateTime"
    );
    let is_number = field.is_number();
    let is_bool = matches!(field.source_type_name.as_str(), "bool" | "boolean");
    json!({
        "MemberName": suffix,
        "memberName": name,
        "memberConstantName": crate::preview_domain::field_constant_name(&field.name),
        "simpleProperty": true,
        "fieldType": {"fullName": type_name(field)},
        "objectDescriptor": {"RequestName": entity.type_name_with_suffix("request"), "objectBooleanAttributePredictPrefix": if entity.is_human {"who"} else {"which"}},
        "attributePredicatePrefixExpr": format!("{prefix}{suffix}"),
        "attributePredicateSuffixExpr": if entity.is_human {"s"} else {"ing"},
        "boolAttributePredicateSuffixExpr": format!("{bool_prefix}{suffix}"),
        "boolNegativeAttributePredicateExpr": format!("{bool_negative}{suffix}"),
        "isBool": field.flag("isBool", is_bool),
        "isId": field.is_id,
        "isNumber": field.flag("isNumber", is_number),
        "isDate": field.flag("isDate", is_date),
        "isString": field.flag("isString", is_string),
        "isText": field.is_text,
        "isPassword": field.is_password,
        "isBusinessId": is_business_id(field),
        "businessIdFullTypeName": format!("{}.{}", full_class(package, entity), suffix),
    })
}

fn relation_value(
    relation: &RenderRelation,
    entity: &RenderEntity,
    domain: &RenderDomain,
    package: &str,
) -> Value {
    let target = domain
        .source_entities
        .iter()
        .find(|target| target.rust_struct == relation.target_struct);
    let target_class = target
        .map(|target| full_class(package, target))
        .unwrap_or_default();
    let target_request = target.map(|target| full_request(package, target)).unwrap_or_default();
    let name = crate::preview_domain::camel(&relation.name);
    json!({
        "MemberName": crate::preview_domain::pascal(&relation.name),
        "memberName": name,
        "memberConstantName": crate::preview_domain::field_constant_name(&relation.name),
        "simpleProperty": false,
        "fieldType": {
            "fullName": target_class,
            "fullRequestName": target_request,
            "camelNameInPluralForm": target.map(|target| target.camel_plural_name()).unwrap_or_default()
        },
        "objectDescriptor": {"RequestName": entity.type_name_with_suffix("request"), "domain": {"fullQ": format!("{package}.Q")}},
        "attributePredicatePrefixExpr": format!("{}{}", if entity.is_human {"whose"} else {"with"}, crate::preview_domain::pascal(&relation.name)),
        "isId": false,
        "isBusinessId": false,
    })
}

fn reverse_value(
    relation: &RenderReverseRelation,
    entity: &RenderEntity,
    domain: &RenderDomain,
    package: &str,
) -> Value {
    let child = domain
        .source_entities
        .iter()
        .find(|child| child.rust_struct == relation.target_struct);
    let child_class = child
        .map(|child| full_class(package, child))
        .unwrap_or_default();
    let member = &relation.java_member_name;
    json!({
        "MemberName": crate::preview_domain::upper_first(member),
        "MemberNames": relation.java_member_names.clone(),
        "memberName": member,
        "memberNames": "",
        "memberConstantName": format!("{}_PROPERTY", crate::preview_domain::snake(member).to_uppercase()),
        "beanName": relation.java_bean_name,
        "oneToOne": !relation.many,
        "parent": {"EntityName": entity.rust_struct, "RequestName": entity.type_name_with_suffix("request")},
        "child": {
            "RequestName": child.map(|child| child.type_name_with_suffix("request")).unwrap_or_default(),
            "requestName": child.map(|child| crate::preview_domain::camel(&format!("{}_request",child.name))).unwrap_or_default(),
            "fullName": child_class,
            "fullRequestName": child.map(|child| full_request(package,child)).unwrap_or_default(),
            "camelNameInPluralForm": child.map(|child| child.camel_plural_name()).unwrap_or_default(),
            "domain": {"fullQ": format!("{package}.Q")},
        },
        "property": {"memberConstantName": crate::preview_domain::field_constant_name(child.and_then(|child| child.object_fields.iter().find(|field| field.local_key == relation.local_key)).map(|field| field.name.as_str()).unwrap_or_else(|| relation.local_key.trim_end_matches("_id")))},
    })
}

pub(crate) fn context(
    domain: &RenderDomain,
    entity: &RenderEntity,
    package: &str,
) -> Map<String, Value> {
    let own_package = format!(
        "{package}.{}",
        crate::shadow::java_package_segment(&entity.name)
    );
    let mut properties: Vec<(usize, Value)> = entity
        .fields
        .iter()
        .map(|field| (field.source_order, field_value(field, entity, package)))
        .chain(entity.object_fields.iter().map(|relation| {
            (
                relation.source_order,
                relation_value(relation, entity, domain, package),
            )
        }))
        .collect();
    properties.sort_by_key(|(order, _)| *order);
    let all_fields = properties
        .into_iter()
        .map(|(_, value)| value)
        .collect::<Vec<_>>();
    let properties = all_fields
        .iter()
        .filter(|field| !entity.has_parent() || field["isId"] != true)
        .cloned()
        .collect::<Vec<_>>();
    let objects = entity
        .object_fields
        .iter()
        .map(|relation| relation_value(relation, entity, domain, package))
        .collect::<Vec<_>>();
    let reverse = entity
        .reverse_relations
        .iter()
        .map(|relation| reverse_value(relation, entity, domain, package))
        .collect::<Vec<_>>();
    let one_to_many = entity
        .one_to_many_relations
        .iter()
        .map(|relation| reverse_value(relation, entity, domain, package))
        .collect::<Vec<_>>();
    let one_to_one = entity
        .one_to_one_relations
        .iter()
        .map(|relation| reverse_value(relation, entity, domain, package))
        .collect::<Vec<_>>();
    let constant_objects = entity.constant_object_fields.iter().map(|relation| {
        let mut value = relation_value(relation, entity, domain, package);
        let target = domain.entities.iter().find(|target| target.rust_struct == relation.target_struct);
        let candidates = relation.candidates.iter().map(|candidate| json!({
            "value": crate::preview_domain::pascal(&candidate.rust_candidate_filter_value.to_lowercase()),
            "key": format!("{}_{}", target.map(|target| target.name.to_shouty_snake_case()).unwrap_or_default(), crate::preview_domain::snake(&candidate.rust_candidate_filter_value.to_lowercase()).to_uppercase())
        })).collect::<Vec<_>>();
        value["candidateForConst"] = json!(candidates);
        value
    }).collect::<Vec<_>>();
    let functions = [
        ("min", "min"),
        ("max", "max"),
        ("sum", "sum"),
        ("avg", "avg"),
        ("standardDeviation", "stdDev"),
        ("squareRootOfPopulationStandardDeviation", "stdDevPop"),
        ("sampleVariance", "varSamp"),
        ("samplePopulationVariance", "varPop"),
    ];
    let aggregation_for = |target: &RenderEntity| {
        target
            .fields
            .iter()
            .filter(|field| field.is_number())
            .flat_map(|field| {
                functions.iter().map(move |(verb, noun)| {
                    json!({
                        "fd": field_value(field, target, package),
                        "func": {"verb": verb, "noun": noun, "__string": match *verb {
                            "standardDeviation" => "stddev",
                            "squareRootOfPopulationStandardDeviation" => "stddev_pop",
                            "sampleVariance" => "var_samp",
                            "samplePopulationVariance" => "var_pop",
                            other => other,
                        }},
                    })
                })
            })
            .collect::<Vec<_>>()
    };
    let aggregation_properties = aggregation_for(entity);
    let aggregation_entries = entity
        .one_to_many_relations
        .iter()
        .flat_map(|relation| {
            let child = domain
                .entities
                .iter()
                .find(|child| child.rust_struct == relation.target_struct);
            child
                .map(|child| {
                    aggregation_for(child)
                        .into_iter()
                        .map(|property| {
                            json!({
                                "relation": reverse_value(relation, entity, domain, package),
                                "aggProperty": property,
                            })
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default()
        })
        .collect::<Vec<_>>();
    let bean_name = crate::preview_domain::camel(&entity.name);
    let mut object_descriptor = json!({
        "EntityName": entity.rust_struct,
        "RequestName": entity.type_name_with_suffix("request"),
        "fullpackage": own_package,
        "requestParent": "io.teaql.core.BaseRequest",
        "beanName": bean_name,
        "beanNames": entity.camel_plural_name(),
        "allChildrenOds": false,
        "childrenOds": [],
        "ChildrenOds": [],
        "requestFilterProperties": properties,
        "fields": all_fields,
        "parentFilterProperties": [],
        "reverseRelations": reverse,
        "parentReverseRelations": [],
        "oneToManyRelations": one_to_many,
        "parentOneToManyRelations": [],
        "oneToOneRelations": one_to_one,
        "objectProperties": objects,
        "constantObjectFields": constant_objects,
        "aggregationProperties": aggregation_properties,
        "parentAggregationProperties": [],
        "aggregationEntries": aggregation_entries,
        "parentAggregationEntries": [],
        "supportFullTextSearch": entity.data_service.as_deref().is_some_and(|name| name.eq_ignore_ascii_case("meilisearch")),
        "domain": {"fullQ": format!("{package}.Q")}
    });
    let child_value = |child: &RenderEntity| {
        json!({
            "EntityName":child.rust_struct,
            "fullName":full_class(package, child),
            "fullRequestName":full_request(package, child),
            "camelNameInPluralForm":child.camel_plural_name(),
            "childrenOds":!child.children(domain).is_empty(),
        })
    };
    let children = entity
        .children(domain)
        .into_iter()
        .map(child_value)
        .collect::<Vec<_>>();
    object_descriptor["childrenOds"] = json!(children);
    object_descriptor["ChildrenOds"] = json!(children);
    object_descriptor["allChildrenOds"] = json!(
        entity
            .descendants(domain)
            .into_iter()
            .map(child_value)
            .collect::<Vec<_>>()
    );
    let ancestors = entity.ancestors(domain);
    if let Some(parent) = ancestors
        .first()
        .filter(|_| !ancestors.iter().any(|parent| parent.name == entity.name))
    {
        object_descriptor["requestParent"] =
            json!(full_request(package, parent));
        let mut inherited = Vec::new();
        let mut reverse = Vec::new();
        let mut one_to_many = Vec::new();
        let mut aggregations = Vec::new();
        let mut entries = Vec::new();
        for parent in ancestors {
            let parent_context = context(domain, parent, package);
            let descriptor = &parent_context["objectDescriptor"];
            inherited.extend(
                descriptor["requestFilterProperties"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter(|field| field["isId"] != true)
                    .cloned(),
            );
            reverse.extend(
                parent
                    .reverse_relations
                    .iter()
                    .map(|relation| reverse_value(relation, parent, domain, package)),
            );
            one_to_many.extend(
                parent
                    .one_to_many_relations
                    .iter()
                    .map(|relation| reverse_value(relation, entity, domain, package)),
            );
            aggregations.extend(
                descriptor["aggregationProperties"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .cloned(),
            );
            entries.extend(
                descriptor["aggregationEntries"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .cloned(),
            );
        }
        if let Some(id) = entity.fields.iter().find(|field| field.is_id) {
            inherited.push(field_value(id, entity, package));
        }
        object_descriptor["parentFilterProperties"] = json!(inherited);
        object_descriptor["parentReverseRelations"] = json!(reverse);
        object_descriptor["parentOneToManyRelations"] = json!(one_to_many);
        object_descriptor["parentAggregationProperties"] = json!(aggregations);
        object_descriptor["parentAggregationEntries"] = json!(entries);
    }
    let mut vars = Map::new();
    vars.insert("objectDescriptor".into(), object_descriptor);
    let use_has = domain
        .root_attributes
        .get("use_has")
        .is_some_and(|value| crate::preview_rust::attribute_flag(value));
    vars.insert("domain".into(), json!({"fullQ": format!("{package}.Q"), "fullConstants": format!("{package}.Constants"), "haveVerb": if use_has { "has" } else { "have" }, "haveNoVerb": if use_has { "hasNo" } else { "haveNo" }}));
    vars
}
