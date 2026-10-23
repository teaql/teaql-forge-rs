//! Shared JavaBean getters used by language templates and external previews.
use crate::context::{RenderDomain, RenderField};
use crate::preview_domain::{snake, upper_first};
use crate::preview_rust::{attr, attribute_flag, raw_type, sample};
use serde_json::{Map, Value, json};

pub(crate) fn public_type_flag(field: &RenderField, name: &str) -> Value {
    use std::sync::OnceLock;
    static BUILTINS: OnceLock<
        std::collections::BTreeMap<String, Vec<teaql_forge_model::types::TypeAttribute>>,
    > = OnceLock::new();
    let builtin = BUILTINS.get_or_init(teaql_forge_model::types::builtin_types);
    let attached = field.attribute(name).or_else(|| {
        if !field.type_attributes.is_empty() {
            return None;
        }
        builtin
            .get(&field.source_type_name)?
            .iter()
            .find(|a| a.name == name)
            .map(|a| a.value.as_str())
    });
    if let Some(value) = attached.filter(|value| !value.is_empty()) {
        return match value.to_ascii_lowercase().as_str() {
            "true" | "yes" | "on" => json!(true),
            "false" | "no" | "off" => json!(false),
            _ => json!(value),
        };
    }
    let kind = field.source_type_name.to_ascii_lowercase();
    json!(match name {
        "isBoolean" | "isBool" => matches!(kind.as_str(), "bool" | "boolean"),
        "isInteger" => matches!(kind.as_str(), "int" | "integer"),
        "isJSON" => matches!(kind.as_str(), "json" | "jsonme"),
        "isDateTime" => kind == "datetime",
        "isDateTimeWithZone" => kind == "datetimewithzone",
        "isNumber" => matches!(
            kind.as_str(),
            "int" | "integer" | "number" | "double" | "money" | "long"
        ),
        "isTime" => matches!(kind.as_str(), "datetime" | "time"),
        "isFloat" => kind == "float",
        "isDouble" => kind == "double",
        "isDate" => kind == "date",
        "isLong" => kind == "long",
        "isMoney" => kind == "money",
        _ => false,
    })
}

pub(crate) fn runtime_version(domain: &RenderDomain, default: &str, names: &[&str]) -> String {
    let version = names
        .iter()
        .filter_map(|name| domain.root_attributes.get(*name))
        .find(|value| !value.is_empty());
    version
        .filter(|value| !value.chars().all(crate::input_path::blank_char))
        .map_or_else(|| default.into(), |value| (*value).clone())
}

pub(crate) fn json_field_profile(domain: &RenderDomain) -> Result<&str, String> {
    let value = domain
        .root_attributes
        .get("json_field_naming")
        .map(String::as_str)
        .unwrap_or("");
    // java.lang.String.isBlank uses Character.isWhitespace, excluding NBSP.
    if value.chars().all(|ch| matches!(ch, '\u{0009}'..='\u{000d}' | '\u{001c}'..='\u{0020}' | '\u{1680}' | '\u{2000}'..='\u{2006}' | '\u{2008}'..='\u{200a}' | '\u{2028}'..='\u{2029}' | '\u{205f}' | '\u{3000}')) {
        return Ok("camelCase");
    }
    match value {
        "camelCase" | "snake_case" | "PascalCase" => Ok(value),
        _ => Err(format!(
            "Unsupported json_field_naming '{value}'; expected camelCase, snake_case, or PascalCase"
        )),
    }
}

pub(crate) fn json_field_name(name: &str, profile: &str) -> String {
    if profile == "snake_case" {
        return name.into();
    }
    name.split('_')
        .enumerate()
        .map(|(index, token)| {
            if profile == "camelCase" && index == 0 {
                token.to_string()
            } else {
                let mut chars = token.chars();
                chars
                    .next()
                    .map(|ch| {
                        format!(
                            "{}{}",
                            crate::preview_domain::character_case(ch).1,
                            chars.as_str()
                        )
                    })
                    .unwrap_or_default()
            }
        })
        .collect()
}

fn fallback(values: &mut Map<String, Value>, name: &str, value: Value) {
    values.entry(name.to_string()).or_insert(value.clone());
    values.entry(upper_first(name)).or_insert(value);
}

pub(crate) fn inherited_getters(
    name: Option<&str>,
    attrs: &Value,
    line: usize,
    path: Option<&str>,
) -> Value {
    let display = ["name", "chinese_name", "english_name", "title"]
        .iter()
        .find_map(|key| {
            attrs
                .get(*key)
                .and_then(Value::as_str)
                .filter(|s| !s.is_empty())
        })
        .or(name);
    json!({"name":name,"snakeName":name.map(snake).unwrap_or_default(),
        "displayName":display,"deleted":false,"lineNumber":line,"xmlPath":path})
}

pub(crate) fn preserve_getters(domain: &RenderDomain, graph: &mut Map<String, Value>) {
    for (id, value) in graph.iter_mut() {
        let Some(node) = value.as_object_mut() else {
            continue;
        };
        let kind = node
            .get("__stg_java_class")
            .and_then(Value::as_str)
            .unwrap_or("");
        if kind.is_empty() {
            continue;
        }
        let mut line = 0;
        let mut path = None;
        let raw = if id == "domain" {
            Some(domain.name.as_str())
        } else if let Some(index) = id
            .strip_prefix("object:")
            .and_then(|n| n.parse::<usize>().ok())
        {
            let entity = &domain.entities[index];
            line = entity.line_number;
            path = Some(entity.xml_path.as_str());
            Some(entity.name.as_str())
        } else if let Some(rest) = id.strip_prefix("field:") {
            let (owner, name) = rest.split_once(':').unwrap();
            let entity = &domain.entities[owner.parse::<usize>().unwrap()];
            if let Some(field) = entity.fields.iter().find(|f| f.name == name) {
                line = field.line_number;
                path = (!field.xml_path.is_empty()).then_some(field.xml_path.as_str());
            } else if let Some(field) = entity.object_fields.iter().find(|f| f.name == name) {
                line = field.line_number;
                path = (!field.xml_path.is_empty()).then_some(field.xml_path.as_str());
            }
            Some(name)
        } else if let Some(name) = id.strip_prefix("type:") {
            Some(name)
        } else {
            None
        };
        let attrs = node
            .get("__stg_attributes")
            .cloned()
            .unwrap_or_else(|| json!({}));
        let mut getters = inherited_getters(raw, &attrs, line, path)
            .as_object()
            .unwrap()
            .clone();
        if kind.ends_with(".Type") || kind.ends_with(".ObjectType") {
            getters.insert("default".into(), json!(false));
        }
        if let Some(state) = node.get("__stg_java_type_state").filter(|v| v.is_object()) {
            getters = inherited_getters(
                state["name"].as_str(),
                &attrs,
                state["lineNumber"].as_u64().unwrap_or(0) as usize,
                state["xmlPath"].as_str(),
            )
            .as_object()
            .unwrap()
            .clone();
            getters.insert("default".into(), state["default"].clone());
            getters.insert("parent".into(), state["parent"].clone());
            getters.insert(
                "parentType".into(),
                state["parentType"]
                    .as_str()
                    .map(|name| json!({"__stg_ref":format!("type:{name}")}))
                    .unwrap_or(Value::Null),
            );
        }
        if kind.ends_with(".FieldDescriptor") {
            for (property, attribute) in [
                ("id", "isId"),
                ("version", "isVersion"),
                ("required", "required"),
            ] {
                getters.insert(
                    property.into(),
                    json!(attribute_flag(attrs[attribute].as_str().unwrap_or(""))),
                );
            }
        }
        node.insert("__stg_java_getters".into(), Value::Object(getters));
    }
}

pub(crate) fn attach_collection_getters(graph: &mut Map<String, Value>) {
    for value in graph.values_mut() {
        let Some(node) = value.as_object_mut() else { continue };
        let kind = node.get("__stg_java_class").and_then(Value::as_str).unwrap_or("");
        let properties: &[&str] = match kind {
            "com.skynet.codegenerator.meta.Domain" => &["objectDescriptors", "types"],
            "com.skynet.codegenerator.meta.ObjectDescriptor" => &["properties"],
            "com.skynet.codegenerator.meta.FieldDescriptor" => &["fieldType"],
            _ => &[],
        };
        for &property in properties {
            let physical = node.get(property).cloned().unwrap_or(Value::Null);
            if let Some(getters) = node.get_mut("__stg_java_getters").and_then(Value::as_object_mut) {
                getters.insert(property.into(), physical);
            }
            if let Some(attached) = node.get("__stg_attributes").and_then(|attrs| attrs.get(property))
                .and_then(Value::as_str).filter(|value| !value.is_empty())
                .map(crate::preview_domain::attribute_value)
            {
                node.insert(property.into(), attached);
            }
        }
    }
}

pub(crate) fn enhance(domain: &RenderDomain, graph: &mut Map<String, Value>) {
    let root = graph.get_mut("domain").unwrap().as_object_mut().unwrap();
    let profile = json_field_profile(domain);
    let naming = profile
        .as_ref()
        .map(|profile| json!(profile))
        .unwrap_or_else(|error| json!({"__stg_getter_fault": error}));
    let naming_profile = profile
        .as_ref()
        .map(|profile| {
            let name=match *profile {
                "snake_case" => "SNAKE_CASE",
                "PascalCase" => "PASCAL_CASE",
                _ => "CAMEL_CASE",
            };
            json!({"__stg_java_enum":"com.skynet.codegenerator.meta.JsonFieldNaming","__string":name})
        })
        .unwrap_or_else(|error| json!({"__stg_getter_fault": error}));
    fallback(root, "jsonFieldNaming", naming);
    fallback(root, "jsonFieldNamingProfile", naming_profile);
    for (property, default, attributes) in [
        (
            "teaqlGoVersion",
            "v0.2.9",
            &["teaql_golang_version", "teaql_go_version"][..],
        ),
        ("teaqlPythonVersion", "0.2.9", &["teaql_python_version"][..]),
        (
            "teaqlTypescriptVersion",
            "0.2.10",
            &["teaql_typescript_version", "teaql_ts_version"][..],
        ),
        ("teaqlDotnetVersion", "0.2.9", &["teaql_dotnet_version"][..]),
        ("teaqlSwiftVersion", "0.2.9", &["teaql_swift_version"][..]),
    ] {
        fallback(
            root,
            property,
            json!(runtime_version(domain, default, attributes)),
        );
    }
    let java = domain
        .root_attributes
        .get("teaql_java_version")
        .filter(|value| !value.is_empty())
        .map(String::as_str)
        .unwrap_or("1.553-RELEASE");
    fallback(root, "teaqlVersion", json!(java));
    fallback(
        root,
        "teaqlRepository",
        json!(
            domain
                .root_attributes
                .get("teaql_repository")
                .map(String::as_str)
                .unwrap_or("https://maven.teaql.io/repository/maven-releases/")
        ),
    );
    let ids = graph
        .keys()
        .filter(|id| id.starts_with("field:"))
        .cloned()
        .collect::<Vec<_>>();
    for id in ids {
        let field = &graph[&id];
        let kind = raw_type(graph, field).to_ascii_lowercase();
        let raw = sample(field, 1);
        let trimmed = raw.trim_matches(|c| c <= '\u{20}');
        let json_literal = trimmed.starts_with('{') && trimmed.ends_with('}');
        let arguments = field["__stg_attributes"]["createFunctionArgNames"]
            .as_str()
            .map(|names| names.split(',').map(str::to_string).collect::<Vec<_>>())
            .unwrap_or_default();
        let argument_values = arguments
            .iter()
            .map(|name| attr(field, name).to_string())
            .collect::<Vec<_>>();
        let functions = if attribute_flag(attr(field, "isNumber")) {
            [
                ("min", "min", "min", "Min"),
                ("max", "max", "max", "Max"),
                ("sum", "sum", "sum", "Sum"),
                ("avg", "avg", "avg", "Avg"),
                ("stddev", "standardDeviation", "stdDev", "Stddev"),
                (
                    "stddev_pop",
                    "squareRootOfPopulationStandardDeviation",
                    "stdDevPop",
                    "StddevPop",
                ),
                ("var_samp", "sampleVariance", "varSamp", "VarSamp"),
                ("var_pop", "samplePopulationVariance", "varPop", "VarPop"),
            ]
            .into_iter()
            .enumerate()
            .map(|(i, (sql, verb, noun, go))| {
                json!({
                    "__string":sql,"verb":verb,"pascalVerb":upper_first(verb),"noun":noun,
                    "snakeNoun":snake(noun),"golangRuntimeMethod":go,"portableBaseline":i<4,
                })
            })
            .collect::<Vec<_>>()
        } else {
            vec![]
        };
        let flags = [
            (
                "number",
                "isNumber",
                matches!(
                    kind.as_str(),
                    "int" | "integer" | "number" | "double" | "money" | "long"
                ),
            ),
            ("date", "isDate", kind == "date"),
            (
                "time",
                "isTime",
                matches!(kind.as_str(), "datetime" | "time"),
            ),
            (
                "boolean",
                "isBoolean",
                matches!(kind.as_str(), "bool" | "boolean"),
            ),
            ("", "isBool", matches!(kind.as_str(), "bool" | "boolean")),
            ("double", "isDouble", kind == "double"),
            ("float", "isFloat", kind == "float"),
            (
                "decimal",
                "isDecimal",
                matches!(kind.as_str(), "decimal" | "number"),
            ),
            ("money", "isMoney", kind == "money"),
            ("dateTime", "isDateTime", kind == "datetime"),
            (
                "dateTimeWithZone",
                "isDateTimeWithZone",
                kind == "datetimewithzone",
            ),
            ("JSON", "isJSON", matches!(kind.as_str(), "json" | "jsonme")),
            (
                "integer",
                "isInteger",
                matches!(kind.as_str(), "int" | "integer"),
            ),
            ("long", "isLong", kind == "long"),
        ];
        let field = graph.get_mut(&id).unwrap().as_object_mut().unwrap();
        let raw_name = id.splitn(3, ':').nth(2).unwrap();
        fallback(
            field,
            "jsonFieldName",
            profile
                .as_ref()
                .map(|profile| json!(json_field_name(raw_name, profile)))
                .unwrap_or_else(|error| json!({"__stg_getter_fault": error})),
        );
        for (logical, getter, value) in flags {
            if !logical.is_empty() {
                fallback(field, logical, json!(value));
            }
            fallback(field, getter, json!(value));
        }
        for (name, value) in [
            (
                "createTime",
                kind.starts_with("create_time") || kind.starts_with("createtime"),
            ),
            (
                "updateTime",
                kind.starts_with("update_time") || kind.starts_with("updatetime"),
            ),
            ("jsonLiteral", json_literal),
        ] {
            fallback(field, name, json!(value));
        }
        fallback(field, "defaultSampleValue", json!(raw));
        fallback(
            field,
            "rustSafeDefaultSampleValue",
            json!(raw.replace('\\', "\\\\").replace('"', "\\\"")),
        );
        for name in ["createFunctionArgumentNames", "updateFunctionArgumentNames"] {
            fallback(field, name, json!(arguments));
        }
        for name in ["createFunctionArguments", "updateFunctionArguments"] {
            fallback(field, name, json!(argument_values));
        }
        fallback(field, "aggregationFunctions", json!(functions));
        fallback(field, "Name", json!(id.splitn(3, ':').nth(2).unwrap()));
    }
}
