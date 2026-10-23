use crate::context::{RenderDomain, RenderEntity};
use crate::engine::GeneratedFile;
use crate::java_stg::StgTemplate;
use serde_json::{json, Map, Value};

fn methods(entity: &RenderEntity, domain: &RenderDomain) -> Value {
    let package = domain.java_full_package.as_deref().unwrap_or("example");
    let descriptor =
        crate::descriptor_context::context(domain, entity, package)["objectDescriptor"].clone();
    let list = |key: &str| descriptor[key].as_array().cloned().unwrap_or_default();
    let properties = list("requestFilterProperties");
    let extended_properties = [properties.clone(), list("parentFilterProperties")].concat();
    let reverse = [list("reverseRelations"), list("parentReverseRelations")].concat();
    let many = [list("oneToManyRelations"), list("parentOneToManyRelations")].concat();
    let aggregates = [
        list("aggregationProperties"),
        list("parentAggregationProperties"),
    ]
    .concat();
    let mut output = Vec::new();
    // Java Pattern's default \w and \s classes are ASCII here. Rust regex's
    // Unicode defaults would absorb Unicode letters into the parameter names.
    let parameter_pattern = regex::Regex::new(
        r"((?:[A-Za-z0-9_]+\.)*[A-Za-z0-9_]+(?:<[^>]+>)?)[ \t\n\x0b\x0c\r]*(\.{3})?[ \t\n\x0b\x0c\r]*([A-Za-z0-9_]+)",
    ).unwrap();
    let mut add = |name: String, parameters: String| {
        let parameters = parameter_pattern
            .captures_iter(&parameters)
            .map(|capture| {
                format!(
                    "{}{} {}",
                    &capture[1],
                    capture.get(2).map_or("", |part| part.as_str()),
                    &capture[3]
                )
            })
            .collect::<Vec<_>>()
            .join(",");
        output.push(json!({"name":name,"parametersExpr":parameters}));
    };
    for (name, parameters) in [
        ("enableAggregationCache", "long cacheExpiredMillis"),
        ("enableAggregationCache", ""),
        ("propagateAggregationCache", "long cacheExpiredMillis"),
        ("appendSearchCriteria", "SearchCriteria searchCriteria"),
        (
            "filter",
            "String property1, Operator operator, String property2",
        ),
        ("rawSql", "String rawSql"),
    ] {
        add(name.into(), parameters.into());
    }
    let request = format!(
        "{} {}",
        entity.type_name_with_suffix("request"),
        crate::preview_domain::camel(&format!("{}_request",entity.name))
    );
    add("matchAnyOf".into(), request.clone());
    if entity.descendants(domain).is_empty() {
        add("enhanceChild".into(), request);
    }
    for name in [
        "tryEnhanceChildren",
        "withDeletedRows",
        "deletedRowsOnly",
        "selectSelf",
        "selectSelfWithoutParent",
        "selectAll",
        "selectAny",
    ] {
        add(name.into(), String::new());
    }
    for property in &extended_properties {
        let member = text(property, "MemberName");
        let name = text(property, "memberName");
        add(format!("select{member}"), String::new());
        add(format!("select{member}"), "String rawSqlStatement".into());
        if property["isNumber"] == true {
            add(
                format!("select{member}"),
                "AggrFunction aggrFunction".into(),
            );
        }
        if property["simpleProperty"] == false {
            add(format!("select{member}IdOnly"), String::new());
            add(
                format!("select{member}"),
                format!(
                    "{} {name}",
                    property["fieldType"]["fullRequestName"]
                        .as_str()
                        .unwrap_or("")
                ),
            );
        }
        add(format!("unselect{member}"), String::new());
    }
    for property in &properties {
        let member = text(property, "MemberName");
        add(format!("groupBy{member}"), String::new());
        add(
            format!("groupBy{member}"),
            "String resultVariableName".into(),
        );
    }
    for property in list("constantObjectFields") {
        for candidate in property["candidateForConst"]
            .as_array()
            .into_iter()
            .flatten()
        {
            add(
                format!(
                    "{}Is{}",
                    text(&property, "attributePredicatePrefixExpr"),
                    text(candidate, "value")
                ),
                String::new(),
            );
        }
    }
    for property in list("parentFilterProperties") {
        let member = text(&property, "MemberName");
        add(format!("groupBy{member}"), String::new());
        add(format!("groupBy{member}"), "String resultVariableName".into());
    }
    for property in &extended_properties {
        let member = text(property, "MemberName");
        for suffix in ["Ascending", "Descending"] {
            add(format!("orderBy{member}{suffix}"), String::new());
        }
        if property["isString"] == true && property["isText"] != true {
            for suffix in ["AscendingUsingGBK", "DescendingUsingGBK"] {
                add(format!("orderBy{member}{suffix}"), String::new());
            }
        }
    }
    for relation in &reverse {
        let member = text(relation, "MemberName");
        let child = &relation["child"];
        add(format!("select{member}"), String::new());
        add(
            format!("select{member}"),
            format!(
                "{} {}",
                text(child, "RequestName"),
                text(child, "requestName")
            ),
        );
    }
    add("count".into(), String::new());
    add("count".into(), "String resultVariableName".into());
    for aggregate in &aggregates {
        let name = format!(
            "{}{}",
            text(&aggregate["func"], "verb"),
            text(&aggregate["fd"], "MemberName")
        );
        add(name.clone(), String::new());
        add(name, "String resultVariableName".into());
    }
    for property in &extended_properties {
        if property["simpleProperty"] == false {
            add(
                format!("aggregate{}", text(property, "MemberName")),
                format!(
                    "{} subRequest",
                    text(&property["fieldType"], "fullRequestName")
                ),
            );
        }
    }
    for relation in &reverse {
        add(
            format!("aggregate{}", text(relation, "MemberNames")),
            format!("{} subRequest", text(&relation["child"], "RequestName")),
        );
    }
    for property in &properties {
        let member = text(property, "MemberName");
        let name = text(property, "memberName");
        let ty = text(&property["fieldType"], "fullName");
        let prefix = text(property, "attributePredicatePrefixExpr");
        add(format!("filterBy{member}"), format!("{ty}... {name}"));
        add(prefix.into(), "Operator operator,Object... values".into());
        for suffix in ["isNull", "isNotNull"] {
            add(format!("{prefix}{suffix}"), String::new());
        }
        if property["simpleProperty"] == true {
            for suffix in [
                "GreatThan",
                "GreatThanOrEqual",
                "LessThan",
                "LessThanOrEqual",
            ] {
                add(format!("{prefix}{suffix}"), format!("{ty} {name}"));
            }
            add(
                format!("{prefix}Between"),
                format!("{ty} startOf{member},{ty} endOf{member}"),
            );
            if property["isString"] == true {
                for suffix in ["StarsWith", "EndsWith", "Contains", "Is", "SoundsLike"] {
                    add(format!("{prefix}{suffix}"), format!("{ty} {name}"));
                }
            }
        } else {
            add(format!("filterBy{member}"), format!("Long {name}Id"));
            add(
                format!("filterBy{member}"),
                format!("{} {name}", text(&property["fieldType"], "fullRequestName")),
            );
        }
    }
    let children = entity.children(domain);
    if !children.is_empty() {
        for child in children {
            add(format!("is{}", child.rust_struct), String::new());
        }
        add("matchTypes".into(), "String... types".into());
        add("withTypeGroup".into(), "String... types".into());
    }
    for relation in &many {
        let name = format!("statsFrom{}", text(relation, "MemberNames"));
        let request = text(&relation["child"], "RequestName");
        add(name.clone(), format!("String name,{request} subRequest"));
        add(
            name.clone(),
            format!("String name,{request} subRequest, boolean singleResult"),
        );
        add(name, format!("{request} subRequest"));
    }
    for property in &properties {
        if property["simpleProperty"] == false {
            add(
                format!("rollUpTo{}", text(property, "MemberName")),
                String::new(),
            );
        }
    }
    for relation in &many {
        let name = format!("count{}", text(relation, "MemberName"));
        let request = text(&relation["child"], "RequestName");
        add(name.clone(), String::new());
        add(name.clone(), "String resultVariableName".into());
        add(
            name,
            format!("String resultVariableName,{request} subRequest"),
        );
    }
    for entry in list("aggregationEntries") {
        let name = format!(
            "{}{}Of{}",
            text(&entry["aggProperty"]["func"], "verb"),
            text(&entry["aggProperty"]["fd"], "MemberName"),
            text(&entry["relation"], "MemberName")
        );
        add(name.clone(), String::new());
        add(name.clone(), "String resultVariableName".into());
        add(
            name,
            format!(
                "String resultVariableName{} subRequest",
                text(&entry["relation"]["child"], "RequestName")
            ),
        );
    }
    let plural = entity.camel_plural_name();
    json!({"odLevelRequestMethods":[{"name":plural,"parametersExpr":""},{"name":format!("{plural}WithChildren"),"parametersExpr":""}],"requestMethods":output,"matches":format!("\"Q.{plural}().\",\"Q.{plural}WithChildren().\"")})
}

fn text<'a>(value: &'a Value, key: &str) -> &'a str {
    value[key].as_str().unwrap_or("")
}

pub(crate) fn generate(domain: &RenderDomain, scope: &str) -> Result<Vec<GeneratedFile>, String> {
    let attributes = |input: &[(String, String)]| {
        input
            .iter()
            .map(|(name, value)| json!({"name":name,"value":value}))
            .collect::<Vec<_>>()
    };
    let (source, path, objects) = if scope == "frontend" {
        let objects = domain.entities.iter().map(|entity| {
            let mut properties = entity.fields.iter().filter(|field| !field.is_id && !field.is_version).map(|field| {
                let label = field.metadata.get("zh_CN").map(|label| format!("{label}:")).unwrap_or_default();
                let value = field.metadata.get("candidates").map(|value| value.replace(',', "|")).or_else(|| field.metadata.get("sampleValue").map(|value| if entity.object_fields.is_empty() && !entity.has_parent() { value.replace(',', "|") } else { value.clone() })).unwrap_or_default();
                (field.source_order, json!({"originalName":field.name,"originalValue":format!("{label}{value}{}", if field.required {""} else {"?"})}))
            }).chain(entity.object_fields.iter().map(|relation| {
                let value = relation.metadata.get("candidates")
                    .map(|value| value.replace(',', "|"))
                    .or_else(|| relation.metadata.get("sampleValue").cloned())
                    .unwrap_or_else(|| relation.source_value.clone());
                (relation.source_order, json!({
                    "originalName": relation.name,
                    "originalValue": format!("{}{}{}",
                        relation.metadata.get("zh_CN").map(|label| format!("{label}:")).unwrap_or_default(),
                        value,
                        if relation.required { "" } else { "?" }),
                }))
            })).collect::<Vec<_>>();
            properties.sort_by_key(|(order, _)| *order);
            let properties = if let Some((_, value)) = entity.metadata_ordered.iter()
                .find(|(name, value)| name == "properties" && !value.is_empty()) {
                match value.to_ascii_lowercase().as_str() {
                    "true" | "yes" | "on" => json!(true),
                    "false" | "no" | "off" => json!(false),
                    _ => json!(value),
                }
            } else {
                json!(properties.into_iter().map(|(_, property)| property).collect::<Vec<_>>())
            };
            json!({"originalName":entity.name,"attributes":attributes(&entity.metadata_ordered),"properties":properties})
        }).collect::<Vec<_>>();
        (
            include_str!("../templates/auxiliary/merge-model.stg"),
            format!("{}.xml", domain.name),
            objects,
        )
    } else {
        (
            include_str!("../templates/auxiliary/autocomplete.stg"),
            "auto-complete.js".into(),
            domain
                .entities
                .iter()
                .map(|entity| methods(entity, domain))
                .collect(),
        )
    };
    let mut vars = Map::new();
    vars.insert("domain".into(),json!({"rootAttributes":domain.root_attributes,"attributes":attributes(&domain.root_attributes_ordered),"objectDescriptors":objects}));
    let mut content = StgTemplate::parse(source)?.render(vars, "")?;
    if scope == "tool" {
        content.push_str("\n\n\n");
    }
    Ok(vec![GeneratedFile { path, content }])
}
