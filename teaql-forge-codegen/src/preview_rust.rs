//! Fixed-source Rust naming enhancers on the reference-preserving preview graph.
use crate::context::RenderDomain;
use crate::preview_domain::{pascal, snake};
use regex::Regex;
use serde_json::{json, Map, Value};
use std::fmt::Write;

type Graph = Map<String, Value>;
pub(crate) fn text<'a>(node: &'a Value, key: &str) -> &'a str {
    node.get(key).and_then(Value::as_str).unwrap_or("")
}
pub(crate) fn attr<'a>(node: &'a Value, key: &str) -> &'a str {
    node.get("__stg_attributes")
        .map(|a| text(a, key))
        .unwrap_or("")
}
pub(crate) fn flag(node: &Value, key: &str) -> bool {
    attribute_flag(attr(node, key))
}
pub(crate) fn attribute_flag(value: &str) -> bool {
    matches!(
        value
            .trim_matches(|c| c <= '\u{20}')
            .to_lowercase()
            .as_str(),
        "true" | "yes" | "y" | "t" | "ok" | "1" | "on" | "是" | "对" | "真" | "對" | "√"
    )
}
pub(crate) fn node<'a>(graph: &'a Graph, value: &'a Value) -> &'a Value {
    value
        .get("__stg_ref")
        .and_then(Value::as_str)
        .and_then(|id| graph.get(id))
        .unwrap_or(value)
}
pub(crate) fn refs(value: &Value, key: &str) -> Vec<Value> {
    value
        .get(key)
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}
pub(crate) fn fault(message: &str) -> Value {
    json!({"__stg_getter_fault":message})
}
pub(crate) fn quoted(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}
pub(crate) fn java_hash(value: &str) -> i32 {
    value
        .encode_utf16()
        .fold(0_i32, |h, u| h.wrapping_mul(31).wrapping_add(i32::from(u)))
}
pub(crate) fn candidate_method(value: &str, id: i64) -> String {
    let raw = snake(value).to_lowercase();
    let mut method = Regex::new("[^a-z0-9_]+")
        .unwrap()
        .replace_all(&raw, "_")
        .into_owned();
    method = method
        .split('_')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("_");
    if method.is_empty() {
        method = format!("value_{id}");
    }
    if method.starts_with(|c: char| c.is_ascii_digit()) {
        method = format!("value_{method}");
    }
    method
}
fn candidate(id: i64, value: &str, line: usize) -> Value {
    json!({"id":id,"value":value,"lineNumber":line,"rustCandidateMethodValue":candidate_method(value,id),"rustCandidateFilterValue":value})
}
pub(crate) fn raw_type<'a>(_graph: &'a Graph, fd: &'a Value) -> &'a str {
    fd.get("fieldType")
        .and_then(|v| v.get("__stg_ref"))
        .and_then(Value::as_str)
        .and_then(|id| id.strip_prefix("type:"))
        .unwrap_or("")
}
fn type_value(graph: &Graph, fd: &Value) -> String {
    text(node(graph, &fd["fieldType"]), "rustType").to_string()
}
fn identifier(name: &str) -> String {
    let name = snake(name);
    if matches!(
        name.as_str(),
        "as" | "break"
            | "const"
            | "continue"
            | "crate"
            | "else"
            | "enum"
            | "extern"
            | "false"
            | "fn"
            | "for"
            | "if"
            | "impl"
            | "in"
            | "let"
            | "loop"
            | "match"
            | "mod"
            | "move"
            | "mut"
            | "pub"
            | "ref"
            | "return"
            | "self"
            | "Self"
            | "static"
            | "struct"
            | "super"
            | "trait"
            | "true"
            | "type"
            | "unsafe"
            | "use"
            | "where"
            | "while"
            | "async"
            | "await"
            | "dyn"
            | "abstract"
            | "become"
            | "box"
            | "do"
            | "final"
            | "macro"
            | "override"
            | "priv"
            | "typeof"
            | "unsized"
            | "virtual"
            | "yield"
            | "try"
    ) {
        format!("{name}_field")
    } else {
        name
    }
}
fn integer(value: &str, suffix: &str) -> String {
    let digits = value.strip_prefix('-').unwrap_or(value);
    format!(
        "{}_{suffix}",
        if !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()) {
            value
        } else {
            "1"
        }
    )
}
fn floating(value: &str, suffix: &str, marker: char) -> String {
    let value = value
        .strip_suffix(marker)
        .or_else(|| value.strip_suffix(marker.to_ascii_uppercase()))
        .unwrap_or(value);
    let valid = Regex::new(r"^[+-]?(?:[0-9]+(?:\.[0-9]*)?|\.[0-9]+)$").unwrap();
    let value = if valid.is_match(value) { value } else { "1.0" };
    format!(
        "{}{}_{suffix}",
        value,
        if value.contains('.') { "" } else { ".0" }
    )
}
fn json_literal(value: &str) -> Result<String, String> {
    if value.is_empty() {
        return Ok("serde_json::Value::Null".into());
    }
    let trimmed = value.trim_matches(crate::input_path::blank_char);
    if (trimmed.starts_with('{') && trimmed.ends_with('}'))
        || (trimmed.starts_with('[') && trimmed.ends_with(']'))
    {
        return crate::hutool_json::compact(trimmed)
            .map(|json| format!("serde_json::json!({json})"));
    }
    Ok(format!("serde_json::json!({})", quoted(value)))
}
pub(crate) fn object_property(fd: &Value) -> bool {
    fd["fieldType"]["__stg_ref"]
        .as_str()
        .is_some_and(|id| id.starts_with("object-type:"))
}

pub(crate) fn sample(fd: &Value, line: usize) -> String {
    if object_property(fd) {
        return "1".into();
    }
    let attrs = &fd["__stg_attributes"];
    let raw = attrs
        .get("candidates")
        .and_then(Value::as_str)
        .or_else(|| attrs.get("sampleValue").and_then(Value::as_str))
        .unwrap_or("");
    candidate_sample(raw, line).into()
}

// FieldDescriptor.getCandidateValue guards the complete JSON text before
// splitting, and Hutool's comma split keeps leading and trailing empty values.
pub(crate) fn candidate_sample(raw: &str, line: usize) -> &str {
    let trimmed = raw.trim_matches(|c| c <= '\u{20}');
    if trimmed.starts_with('{') && trimmed.ends_with('}') {
        return trimmed;
    }
    raw.split(',')
        .nth(line.saturating_sub(1))
        .filter(|_| !raw.is_empty())
        .unwrap_or("1")
}
pub(crate) fn seed(graph: &Graph, fd: &Value, constant: Option<&Value>) -> Result<String, String> {
    let kind = raw_type(graph, fd);
    if flag(fd, "isId") {
        return Ok(constant
            .map(|c| format!("{}_u64", c["id"]))
            .unwrap_or("1_u64".into()));
    }
    if flag(fd, "isVersion") {
        return Ok("1_i64".into());
    }
    if constant.is_some() && object_property(fd) {
        return Ok("1_u64".into());
    }
    let mut value = if let Some(c) = constant {
        if kind == "jsonMe" {
            let sample = attr(fd, "sampleValue");
            if sample.is_empty() {
                attr(fd, "candidates").into()
            } else {
                sample.into()
            }
        } else {
            sample(fd, c["lineNumber"].as_u64().unwrap_or(1) as usize)
        }
    } else {
        let candidates = attr(fd, "candidates");
        let raw = if candidates.is_empty() {
            attr(fd, "sampleValue")
        } else {
            candidates
        };
        if kind == "jsonMe" {
            return json_literal(raw);
        }
        raw.split(',').next().unwrap_or("").into()
    };
    if value.is_empty() {
        value = String::new();
    }
    if kind == "jsonMe" {
        return json_literal(&value);
    }
    Ok(match kind {
        "createTime" | "updateTime" | "dateTime" => "teaql_core::time::Timestamp::now()".into(),
        "date" => "chrono::NaiveDate::from_ymd_opt(1970, 1, 1).unwrap()".into(),
        "bool" => value.eq_ignore_ascii_case("true").to_string(),
        "id" => integer(&value, "u64"),
        "version" | "long" | "integer" => integer(&value, "i64"),
        "float" if constant.is_some() => floating(&value, "f32", 'f'),
        "double" if constant.is_some() => floating(&value, "f64", 'd'),
        _ => quoted(&value),
    })
}
/// Bootstrap and preview use the same fixed-source scalar literal rules.
pub(crate) fn bootstrap_literal(
    _entity: &crate::context::RenderEntity,
    field: &crate::context::RenderField,
    index: Option<usize>,
) -> Result<String, String> {
    let builtin = teaql_forge_model::types::builtin_types();
    let attributes = if field.type_attributes.is_empty() {
        builtin
            .get(&field.source_type_name)
            .cloned()
            .unwrap_or_default()
    } else {
        field.type_attributes.clone()
    };
    let mut attrs: std::collections::BTreeMap<String, String> =
        attributes.into_iter().map(|a| (a.name, a.value)).collect();
    attrs.extend(field.metadata.clone());
    attrs
        .entry("sampleValue".into())
        .or_insert_with(|| field.source_value.clone());
    // Preserve the physical candidate column. Rejoining reconstructed seed rows
    // can drop extra values and accidentally turn a JSON prefix into a literal.
    let fd = json!({"objectProperty":false,"fieldType":{"__stg_ref":format!("type:{}",field.source_type_name)},"__stg_attributes":attrs});
    let candidate = index.map(|index| json!({"id":1,"lineNumber":index+1}));
    if field.source_type_name == "number" && index.is_some() {
        return Ok(format!(
            "rust_decimal::Decimal::from_str_exact({}).unwrap()",
            quoted(&sample(&fd, index.unwrap() + 1))
        ));
    }
    seed(&Map::new(), &fd, candidate.as_ref())
}

fn fields(graph: &Graph, fd: &Value) -> Map<String, Value> {
    let mut out = Map::new();
    let raw_name = text(fd, "originalName");
    let kind = raw_type(graph, fd);
    let object = object_property(fd);
    let optional = flag(fd, "optional");
    let name = identifier(raw_name);
    let storage = format!("{name}{}", if object { "_id" } else { "" });
    let base_type = type_value(graph, fd);
    let business = matches!(
        kind.to_ascii_lowercase().as_str(),
        "businessid" | "business_id"
    ) || flag(fd, "isBusinessId");
    let owned_type = if business {
        format!("crate::{}", pascal(raw_name))
    } else {
        base_type.clone()
    };
    let rust_type = if optional {
        format!("Option<{owned_type}>")
    } else {
        owned_type
    };
    let column = if attr(fd, "column").is_empty() {
        text(fd, "columnName")
    } else {
        attr(fd, "column")
    };
    let mut attributes = Vec::new();
    if flag(fd, "isId") {
        attributes.push("#[teaql(id)]".into());
    }
    if flag(fd, "isVersion") {
        attributes.push("#[teaql(version)]".into());
    }
    if attr(fd, "sqlType").eq_ignore_ascii_case("LARGE_TEXT") || flag(fd, "isText") {
        attributes.push("#[teaql(large_text)]".into());
    }
    if let Some(max) =
        crate::hutool_json::parse_integer(attr(fd, "max").trim_matches(|c| c <= '\u{20}'))
            .and_then(|n| i32::try_from(n).ok())
    {
        if flag(fd, "isString") && !flag(fd, "isText") && max > 0 {
            attributes.push(format!("#[teaql(max_length = {max})]"));
        }
    }
    let shape = Regex::new(
        r"(?i)^[ \t\r\n\x0b\x0c]*(?:NUMERIC|DECIMAL)\s*\(\s*([0-9]+)\s*,\s*([0-9]+)\s*\)\s*$",
    )
    .unwrap();
    if let Some(c) = shape.captures(attr(fd, "sqlType")) {
        if let (Ok(p), Ok(s)) = (c[1].parse::<i32>(), c[2].parse::<i32>()) {
            if p > 0 && s >= 0 && s <= p {
                attributes.push(format!("#[teaql(numeric_precision = {p})]"));
                attributes.push(format!("#[teaql(numeric_scale = {s})]"));
            }
        }
    }
    if !column.is_empty() && column != storage {
        attributes.push(format!("#[teaql(column = \"{column}\")]"));
    }
    let default = if optional {
        "None"
    } else if object {
        "0_u64"
    } else {
        match kind {
            "createTime" | "updateTime" | "dateTime" => "teaql_core::time::Timestamp::now()",
            "date" => "chrono::NaiveDate::from_ymd_opt(1970, 1, 1).unwrap()",
            "bool" => "false",
            "id" => "0_u64",
            "version" | "long" | "integer" => "0_i64",
            "number" => "rust_decimal::Decimal::ZERO",
            "float" => "0.0_f32",
            "double" => "0.0_f64",
            "jsonMe" => "serde_json::Value::Null",
            _ => "String::new()",
        }
    };
    let convert = if object {
        "try_u64()"
    } else {
        match kind {
            "id" => "try_u64()",
            "version" | "long" => "try_i64()",
            "integer" => "try_i64().map(|value| value as i64)",
            "number" => "try_decimal()",
            "float" => "try_f64().map(|value| value as f32)",
            "double" => "try_f64()",
            "bool" => "try_bool()",
            "date" => "try_date()",
            "createTime" | "updateTime" | "dateTime" => "try_timestamp()",
            _ => "",
        }
    };
    let getter = if optional || kind == "jsonMe" {
        format!("self.{storage}.clone()")
    } else if convert.is_empty() {
        format!("self.changed_{storage}().and_then(|value| value.try_text().map(|value| value.to_owned())).unwrap_or_else(|| self.{storage}.clone())")
    } else {
        let (first, cast) = convert
            .split_once(".map(")
            .map(|(a, b)| (a, format!(".map({b}")))
            .unwrap_or((convert, String::new()));
        format!("self.changed_{storage}().and_then(|value| value.{first}){cast}.unwrap_or(self.{storage})")
    };
    let fallback = format!("self.{storage}.clone()");
    let conversion = if kind == "jsonMe" && !object {
        "Some(value.to_json_value())".into()
    } else if convert.is_empty() {
        format!(
            "value.try_text().map(|value| value.{}to_owned())",
            if base_type == "String" { "trim()." } else { "" }
        )
    } else {
        format!("value.{convert}")
    };
    let assignment = if optional {
        format!("if matches!(value, teaql_core::Value::Null) {{ None }} else {{ {conversion}.map(Some).unwrap_or_else(|| {fallback}) }}")
    } else if kind == "jsonMe" && !object {
        "value.to_json_value()".into()
    } else if convert.is_empty() {
        format!("{conversion}.unwrap_or_else(|| {fallback})")
    } else {
        format!("{conversion}.unwrap_or({fallback})")
    };
    let target = text(node(graph, &fd["referenceObject"]), "rustStructName");
    let owner = node(graph, &fd["objectDescriptor"]);
    let owner_class = pascal(text(owner, "originalName"));
    for (key, value) in [
        (
            "rustTimestamp",
            json!(matches!(kind, "createTime" | "updateTime" | "dateTime")),
        ),
        (
            "rustBoolean",
            json!(matches!(
                kind.to_ascii_lowercase().as_str(),
                "bool" | "boolean"
            )),
        ),
        (
            "rustDefaultBooleanSampleValue",
            json!(matches!(
                sample(fd, 1)
                    .trim_matches(|c| c <= '\u{20}')
                    .to_lowercase()
                    .as_str(),
                "true" | "yes" | "on"
            )
            .to_string()),
        ),
        ("rustMemberName", json!(name)),
        ("rustStorageMemberName", json!(storage)),
        (
            "rustStorageFieldName",
            json!(format!(
                "{}{}",
                snake(raw_name),
                if object { "_id" } else { "" }
            )),
        ),
        ("rustColumnName", json!(column)),
        (
            "rustAttributePredicatePrefixExpr",
            json!(snake(text(fd, "attributePredicatePrefixExpr")).to_lowercase()),
        ),
        (
            "rustAttributePredicateSuffixExpr",
            fd["attributePredicateSuffixExpr"].clone(),
        ),
        (
            "rustCandidates",
            json!(attr(fd, "candidates")
                .split(',')
                .filter(|v| !v.is_empty())
                .map(|v| candidate(i64::from(java_hash(v).wrapping_abs()), v, 1))
                .collect::<Vec<_>>()),
        ),
        ("rustType", json!(rust_type)),
        (
            "rustStorageType",
            json!(if object {
                if optional {
                    "Option<u64>".into()
                } else {
                    "u64".into()
                }
            } else if business {
                if optional {
                    "Option<String>".into()
                } else {
                    "String".into()
                }
            } else {
                rust_type.clone()
            }),
        ),
        ("rustBusinessIdTypeName", json!(pascal(raw_name))),
        (
            "rustBusinessIdSlotTypeName",
            json!(format!("{owner_class}{}BusinessIdSlot", pascal(raw_name))),
        ),
        (
            "rustRelationType",
            json!(if object {
                format!("Option<crate::{target}>")
            } else {
                rust_type.clone()
            }),
        ),
        (
            "rustRelationBorrowType",
            json!(if object {
                format!("crate::{target}")
            } else {
                rust_type.clone()
            }),
        ),
        ("rustFieldAttr", json!(attributes.join("\n"))),
        (
            "rustRelationAttr",
            json!(if object {
                format!("#[teaql(relation(target = \"{target}\", local_key = \"{}_id\", foreign_key = \"id\"))]",snake(raw_name))
            } else {
                String::new()
            }),
        ),
        (
            "rustRootSeedValue",
            seed(graph, fd, None)
                .map(Value::String)
                .unwrap_or_else(|e| fault(&e)),
        ),
        ("rustDefaultValue", json!(default)),
        ("rustGetterBody", json!(getter)),
        ("rustUpdateAssignment", json!(assignment)),
    ] {
        out.insert(key.into(), value);
    }
    out
}
fn aggregate(
    graph: &Graph,
    reverse: &Value,
    field: Option<&Value>,
    verb: &str,
    query: &str,
    prefix: &str,
) -> Value {
    let member = text(reverse, "rustMemberName");
    let members = text(reverse, "rustMemberNames");
    let child = node(graph, &reverse["child"]);
    let plural = text(child, "rustPluralName");
    let field_name = field
        .map(|fd| text(fd, "rustStorageMemberName"))
        .unwrap_or("");
    let name = if field.is_some() {
        format!("{verb}_{field_name}_of_{members}")
    } else {
        format!("{verb}_{members}")
    };
    json!({"__string":format!("RustRelationAggregateMethod({name})"),"name":name,"aliasName":format!("{name}_as"),"defaultAlias":name,
        "relationMemberName":member,"relationMemberNames":members,"childPluralName":plural,"queryMethodName":query,
        "fieldMemberName":field.map(|fd|fd["rustStorageMemberName"].clone()).unwrap_or(Value::Null),
        "fieldStorageName":field.map(|fd|fd["rustStorageFieldName"].clone()).unwrap_or(Value::Null),
        "queryAlias":field.map(|_|json!(format!("{prefix}_{field_name}"))).unwrap_or(Value::Null)})
}
fn identity(graph: &Graph, od: &Value) -> Vec<Value> {
    let fields = refs(od, "fields");
    let Some(identifier) = fields
        .iter()
        .map(|fd| node(graph, fd))
        .find(|fd| flag(fd, "identifier"))
    else {
        return vec![];
    };
    let raw = attr(identifier, "candidates");
    if raw.is_empty() {
        return vec![];
    }
    let ids = fields
        .iter()
        .map(|fd| node(graph, fd))
        .find(|fd| text(fd, "originalName") == "id")
        .map(|fd| attr(fd, "candidates"))
        .unwrap_or("");
    let mut values = raw.split(',').collect::<Vec<_>>();
    while values.last() == Some(&"") {
        values.pop();
    }
    values
        .into_iter()
        .enumerate()
        .map(|(i, v)| {
            let value = pascal(&v.to_lowercase());
            let id = ids
                .split(',')
                .nth(i)
                .and_then(|v| v.parse::<i64>().ok())
                .unwrap_or_else(|| i64::from(java_hash(&value.to_uppercase()).wrapping_abs()));
            let mut result = candidate(id, &value, i + 1);
            result["key"] = json!(snake(&format!("{}_{}", text(od, "originalName"), v.to_lowercase())).to_uppercase());
            result
        })
        .collect()
}
pub(crate) fn first(domain: &RenderDomain, default: String, names: &[&str]) -> String {
    names
        .iter()
        .filter_map(|name| domain.root_attributes.get(*name))
        .find(|value| !value.chars().all(crate::input_path::blank_char))
        .cloned()
        .unwrap_or(default)
}

pub(crate) fn enhance(domain: &RenderDomain, graph: &mut Graph) {
    // Types and objects precede fields so relation type references are resolved.
    let ids = graph.keys().cloned().collect::<Vec<_>>();
    for id in &ids {
        let kind = id.strip_prefix("type:");
        let target = id
            .strip_prefix("object-type:")
            .and_then(|s| s.parse::<usize>().ok());
        if kind.is_none() && target.is_none() {
            continue;
        }
        let item = &graph[id];
        let configured = attr(item, "rustType");
        let value = if let Some(index) = target {
            format!("crate::{}", pascal(&domain.entities[index].name))
        } else if !configured.is_empty() {
            configured.into()
        } else {
            match kind.unwrap() {
                "id" => "u64",
                "version" | "integer" | "long" => "i64",
                "float" => "f32",
                "double" => "f64",
                "number" => "rust_decimal::Decimal",
                "bool" => "bool",
                "jsonMe" => "serde_json::Value",
                "date" => "chrono::NaiveDate",
                "dateTime" | "createTime" | "updateTime" => "teaql_core::time::Timestamp",
                _ => "String",
            }
            .into()
        };
        graph.get_mut(id).unwrap()["rustType"] = json!(value);
    }
    for (index, entity) in domain.entities.iter().enumerate() {
        let id = format!("object:{index}");
        let old = &graph[&id];
        let mut out = Map::new();
        let scalar = refs(old, "fields")
            .into_iter()
            .filter(|fd| {
                entity
                    .fields
                    .iter()
                    .any(|f| fd["__stg_ref"] == format!("field:{index}:{}", f.name))
            })
            .collect::<Vec<_>>();
        let object = refs(old, "fields")
            .into_iter()
            .filter(|fd| {
                entity
                    .object_fields
                    .iter()
                    .any(|f| fd["__stg_ref"] == format!("field:{index}:{}", f.name))
            })
            .collect::<Vec<_>>();
        let reverse = refs(old, "reverseRelations");
        let one = reverse
            .iter()
            .filter(|r| node(graph, r)["oneToOne"] == true)
            .cloned()
            .collect::<Vec<_>>();
        let many = reverse
            .iter()
            .filter(|r| node(graph, r)["oneToOne"] != true)
            .cloned()
            .collect::<Vec<_>>();
        let class = pascal(&entity.name);
        let module = snake(&entity.name);
        for (key, value) in [
            ("rustStructName", json!(class)),
            ("rustBehaviorName", json!(format!("{class}Behavior"))),
            ("rustModuleName", json!(module)),
            ("rustFileName", json!(format!("{module}.rs"))),
            ("rustPath", json!(module)),
            (
                "rustPluralName",
                json!(snake(&crate::context::pluralize_name(&entity.name))),
            ),
            (
                "rustTypeMatchList",
                json!(std::iter::once(entity)
                    .chain(entity.descendants(domain))
                    .map(|e| quoted(&pascal(&e.name)))
                    .collect::<Vec<_>>()
                    .join(", ")),
            ),
            ("rustTableName", old["tableName"].clone()),
            ("rustScalarFields", json!(scalar)),
            ("rustObjectFields", json!(object)),
            ("rustNeedsSmartList", json!(!many.is_empty())),
            (
                "rustHasSelectAllRelations",
                json!(!object.is_empty() || !one.is_empty()),
            ),
            ("rustHasSelectAnyRelations", json!(!many.is_empty())),
            ("rustOneToManyRelations", json!(many)),
            ("rustOneToOneRelations", json!(one)),
        ] {
            out.insert(key.into(), value);
        }
        graph
            .get_mut(&id)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .extend(out);
    }
    for id in &ids {
        if !id.starts_with("field:") {
            continue;
        }
        let out = fields(graph, &graph[id]);
        graph
            .get_mut(id)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .extend(out);
    }
    for id in &ids {
        if !id.starts_with("reverse:") {
            continue;
        }
        let old = &graph[id];
        let child = node(graph, &old["child"]);
        let property = node(graph, &old["property"]);
        let many = if old["oneToOne"] == true {
            ""
        } else {
            ", many"
        };
        let target = text(child, "rustStructName");
        let foreign = text(property, "rustStorageFieldName");
        let member = snake(text(old, "memberName")).to_lowercase();
        let members = snake(text(old, "MemberNames")).to_lowercase();
        let out = json!({"rustMemberName":member,"rustMemberNames":members,"rustType":if many.is_empty(){format!("Option<crate::{target}>")}else{format!("SmartList<crate::{target}>")},"rustRelationAttr":format!("#[teaql(relation(target = \"{target}\", local_key = \"id\", foreign_key = \"{foreign}\"{many}))]"),"rustDescriptorAttr":format!("#[teaql(reverse_relation(name = \"{member}\", target = \"{target}\", local_key = \"id\", foreign_key = \"{foreign}\"{many}))]")});
        graph
            .get_mut(id)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .extend(out.as_object().unwrap().clone());
        let reverse = &graph[id];
        let child = node(graph, &reverse["child"]);
        let mut methods = Vec::new();
        for reference in refs(child, "rustScalarFields") {
            let fd = node(graph, &reference);
            let numeric = flag(fd, "isNumber");
            let comparable = matches!(
                raw_type(graph, fd),
                "createTime" | "updateTime" | "dateTime" | "date"
            );
            let verbs = if numeric {
                vec![
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
                ]
            } else if comparable {
                vec![("min", "min", "min"), ("max", "max", "max")]
            } else {
                vec![]
            };
            for (v, q, p) in verbs {
                methods.push(aggregate(graph, reverse, Some(fd), v, q, p));
            }
        }
        let count = aggregate(graph, reverse, None, "count", "count", "count");
        let out = graph.get_mut(id).unwrap();
        out["rustFieldAggregateMethods"] = json!(methods);
        out["rustCountAggregateMethod"] = count;
    }
    for (i, _) in domain.entities.iter().enumerate() {
        let id = format!("object:{i}");
        let od = &graph[&id];
        let candidates = identity(graph, od);
        let mut graphs = Vec::new();
        let mut error = None;
        for candidate in &candidates {
            let mut result = format!(
                "teaql_runtime::GraphNode::new(\"{}\")",
                text(od, "rustStructName")
            );
            for reference in refs(od, "rustScalarFields")
                .into_iter()
                .chain(refs(od, "rustObjectFields"))
            {
                let fd = node(graph, &reference);
                write!(
                    result,
                    "\n            .value(\"{}\", {})",
                    text(fd, "rustStorageFieldName"),
                    match seed(graph, fd, Some(candidate)) {
                        Ok(value) => value,
                        Err(e) => {
                            error = Some(e);
                            break;
                        }
                    }
                )
                .unwrap();
            }
            graphs.push(result);
        }
        graph.get_mut(&id).unwrap()["identityConstants"] = if candidates.is_empty() {
            Value::Null
        } else {
            json!(candidates)
        };
        graph.get_mut(&id).unwrap()["rustConstantSeedGraphs"] =
            error.map_or_else(|| json!(graphs), |e| fault(&e));
    }
    let name = symbol(&domain.name, '-').replace('_', "-");
    let krate = first(
        domain,
        format!("{name}-core"),
        &["rust_crate_name", "rust_project_name"],
    );
    let module = krate.replace('-', "_");
    let version = first(domain, "5.0.5".into(), &["teaql_rs_version"]);
    let mut parts = version
        .strip_prefix('=')
        .unwrap_or(&version)
        .split('.')
        .collect::<Vec<_>>();
    while parts.last() == Some(&"") {
        parts.pop();
    }
    let composition = if parts.len() == 3 {
        match (
            crate::hutool_json::parse_integer(parts[0]).and_then(|n| i32::try_from(n).ok()),
            crate::hutool_json::parse_integer(parts[1]).and_then(|n| i32::try_from(n).ok()),
            crate::hutool_json::parse_integer(parts[2]).and_then(|n| i32::try_from(n).ok()),
        ) {
            (Some(4), Some(minor), Some(patch)) => minor > 3 || (minor == 3 && patch >= 9),
            _ => false,
        }
    } else {
        false
    };
    let topcoat = first(
        domain,
        format!("{name}-web-topcoat"),
        &["rust_web_topcoat_crate_name", "rust_web_crate_name"],
    );
    let provider = crate::data_service_properties::rust_provider_crate(&domain.data_service.name);
    let general = if domain.data_service.name == "linux" {
        Some("teaql-provider-linux")
    } else {
        provider
    };
    let bootstrap = crate::rust_bootstrap::render(domain)
        .map(Value::String)
        .unwrap_or_else(|e| fault(&e));
    let out = json!({"rustCrateName":krate,"rustModuleName":module,"rustEnvPrefix":module.to_uppercase(),"rustTeaqlDependencyVersion":version,"rustGraphCompositionAvailable":composition,"rustWorkspaceCrateName":first(domain,format!("{krate}-workspace"),&["rust_workspace_crate_name","rust_app_crate_name"]),"rustWebAxumCrateName":first(domain,format!("{krate}-web-axum"),&["rust_web_axum_crate_name","rust_web_crate_name"]),"rustWebTopcoatCrateName":topcoat,"rustWebTopcoatModuleName":topcoat.replace('-',"_"),"rustGeneratedBootstrap":bootstrap,"rustWorkspaceGeneratedLibPath":first(domain,"../rust-lib-core/lib".into(),&["rust_workspace_generated_lib_path","rust_generated_lib_path","rust_lib_path"]),"rustSqlProviderDependency":provider.map(|p|format!("{p} = \"{version}\"")),"rustProviderDependency":general.map(|p|format!("{p} = \"{version}\""))});
    graph
        .get_mut("domain")
        .unwrap()
        .as_object_mut()
        .unwrap()
        .extend(out.as_object().unwrap().clone());
}
pub(crate) fn symbol(value: &str, separator: char) -> String {
    let chars = value.chars().collect::<Vec<_>>();
    let mut out = String::new();
    for (i, &ch) in chars.iter().enumerate() {
        let (lower, _, flags) = crate::preview_domain::character_case(ch);
        let mut ch = ch;
        if flags & 2 != 0 {
            let next = chars
                .get(i + 1)
                .is_none_or(|c| crate::preview_domain::character_case(*c).2 & 1 != 0);
            if let Some(&prev) = i.checked_sub(1).and_then(|p| chars.get(p)) {
                if prev != separator
                    && (crate::preview_domain::character_case(prev).2 & 1 != 0 || next)
                {
                    out.push(separator);
                }
            }
            if next {
                ch = lower;
            }
        }
        out.push(ch);
    }
    out
}

#[cfg(test)]
mod literal_parity {
    #[test]
    fn all_rust_json_literals_match_actual_pinned_source_values_and_failures() {
        let source: serde_json::Map<String, serde_json::Value> = serde_json::from_str(
            include_str!("../tests/expected/rust-json-literals-6f2d0a.json"),
        )
        .unwrap();
        assert_eq!(source.len(), 223);
        for (input, expected) in source {
            let actual = super::json_literal(&input);
            if let Some(expected) = expected["ok"].as_str() {
                assert_eq!(actual.unwrap(), expected, "{input:?}");
            } else {
                assert!(actual.is_err(), "source failed for {input:?}");
            }
        }
    }
    #[test]
    #[ignore = "exports pinned-source differential literal inputs"]
    fn export_rust_literal_corpus() {
        let words: Vec<String> = serde_json::from_str(
            &std::fs::read_to_string(std::env::var("TEAQL_RUST_LITERAL_INPUT").unwrap()).unwrap(),
        )
        .unwrap();
        let values: serde_json::Map<String, serde_json::Value> = words
            .into_iter()
            .map(|word| {
                let result = match super::json_literal(&word) {
                    Ok(value) => serde_json::json!({"ok":value}),
                    Err(_) => serde_json::json!({"error":true}),
                };
                (word, result)
            })
            .collect();
        std::fs::write(
            std::env::var("TEAQL_RUST_LITERAL_OUTPUT").unwrap(),
            serde_json::to_string_pretty(&values).unwrap() + "\n",
        )
        .unwrap();
    }
}
