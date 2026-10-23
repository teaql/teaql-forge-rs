use crate::context::{RenderDomain, RenderEntity};
use crate::java_stg::StgTemplate;
use serde_json::{json, Map, Value};
use std::fmt::Write;

fn sources(scope: &str, field: bool) -> Option<(&'static str, &'static str)> {
    Some(match scope {
        "rust-assist-create" => (
            include_str!("../templates/rust-assist-source/rust-assist-create/prompt.md.stg"),
            include_str!("../templates/rust-assist-source/rust-assist-create/contract.md.txt"),
        ),
        "rust-assist-update" => (
            include_str!("../templates/rust-assist-source/rust-assist-update/prompt.md.stg"),
            include_str!("../templates/rust-assist-source/rust-assist-update/contract.md.txt"),
        ),
        "rust-assist-delete" => (
            include_str!("../templates/rust-assist-source/rust-assist-delete/prompt.md.stg"),
            include_str!("../templates/rust-assist-source/rust-assist-delete/contract.md.txt"),
        ),
        "rust-assist-query" if field => (
            include_str!("../templates/rust-assist-source/rust-assist-query/field.md.stg"),
            include_str!("../templates/rust-assist-source/rust-assist-query/contract.md.txt"),
        ),
        "rust-assist-query" => (
            include_str!("../templates/rust-assist-source/rust-assist-query/prompt.md.stg"),
            include_str!("../templates/rust-assist-source/rust-assist-query/contract.md.txt"),
        ),
        "rust-assist-expression" => (
            include_str!("../templates/rust-assist-source/rust-assist-expression/prompt.md.stg"),
            include_str!("../templates/rust-assist-source/rust-assist-expression/contract.md.txt"),
        ),
        "rust-assist-list-page" => (
            include_str!("../templates/rust-assist-source/rust-assist-list-page/prompt.md.stg"),
            include_str!("../templates/rust-assist-source/rust-assist-list-page/contract.md.txt"),
        ),
        "rust-assist-debug" => (
            include_str!("../templates/rust-assist-source/rust-assist-debug/prompt.md.stg"),
            include_str!("../templates/rust-assist-source/rust-assist-debug/contract.md.txt"),
        ),
        "rust-assist-i18n" => (
            include_str!("../templates/rust-assist-source/rust-assist-i18n/prompt.md.stg"),
            include_str!("../templates/rust-assist-source/rust-assist-i18n/contract.md.txt"),
        ),
        "rust-assist-ensure-schema" => (
            include_str!("../templates/rust-assist-source/rust-assist-ensure-schema/prompt.md.stg"),
            include_str!(
                "../templates/rust-assist-source/rust-assist-ensure-schema/contract.md.txt"
            ),
        ),
        "rust-assist-runtime-custom" => (
            include_str!(
                "../templates/rust-assist-source/rust-assist-runtime-custom/prompt.md.stg"
            ),
            include_str!(
                "../templates/rust-assist-source/rust-assist-runtime-custom/contract.md.txt"
            ),
        ),
        "rust-assist-tool-api" => (
            include_str!("../templates/rust-assist-source/rust-assist-tool-api/prompt.md.stg"),
            include_str!("../templates/rust-assist-source/rust-assist-tool-api/contract.md.txt"),
        ),
        _ => return None,
    })
}

fn tool_module(name: &str) -> Option<&'static str> {
    Some(match name {
        "time" => include_str!("../templates/rust-assist-source/rust-assist-tool-api/docs/time.md"),
        "id" => include_str!("../templates/rust-assist-source/rust-assist-tool-api/docs/id.md"),
        "text" => include_str!("../templates/rust-assist-source/rust-assist-tool-api/docs/text.md"),
        "json" => include_str!("../templates/rust-assist-source/rust-assist-tool-api/docs/json.md"),
        "decimal" => {
            include_str!("../templates/rust-assist-source/rust-assist-tool-api/docs/decimal.md")
        }
        "money" => {
            include_str!("../templates/rust-assist-source/rust-assist-tool-api/docs/money.md")
        }
        "http" => include_str!("../templates/rust-assist-source/rust-assist-tool-api/docs/http.md"),
        _ => return None,
    })
}

fn field_value(
    field: &crate::context::RenderField,
    entity: &RenderEntity,
) -> Result<Value, String> {
    let mut value = crate::rust_entity::field_value(field)?;
    value["fieldType"] = json!({"name": field.source_type_name});
    value["snakeBooleanPredicateExpr"] = json!(format!(
        "{}_{}",
        entity.bool_predicate_prefix, field.rust_storage_member_name
    ));
    value["snakeBooleanNegativePredicateExpr"] = json!(format!(
        "{}_not_{}",
        entity.bool_predicate_prefix, field.rust_storage_member_name
    ));
    value["objectProperty"] = json!(false);
    value["rustMemberName"] = json!(field.rust_storage_member_name);
    value["candidateForConst"] = Value::Array(Vec::new());
    let nouns = if value["isNumber"] == true {
        vec![
            "min",
            "max",
            "sum",
            "avg",
            "std_dev",
            "std_dev_pop",
            "var_samp",
            "var_pop",
        ]
    } else {
        Vec::new()
    };
    value["aggregationFunctions"] = json!(nouns
        .iter()
        .map(|noun| json!({"snakeNoun": noun}))
        .collect::<Vec<_>>());
    Ok(value)
}

fn object_value(field: &crate::context::RenderRelation) -> Result<Value, String> {
    let mut value = crate::rust_entity::object_field_value(field)?;
    value["ksmlName"] = json!(field.name);
    value["name"] = json!(field.name);
    value["fieldType"]["name"] = json!(field.target);
    value["objectProperty"] = json!(true);
    value["id"] = json!(false);
    value["version"] = json!(false);
    value["rustMemberName"] = json!(field.rust_member_name);
    value["candidateForConst"] = value["candidateForConst"].clone();
    Ok(value)
}

fn entity_value(
    domain: &RenderDomain,
    entity: &RenderEntity,
    domain_json: &Value,
) -> Result<Value, String> {
    let mut value = crate::rust_entity::descriptor_value(entity)?;
    let mut fields = Vec::new();
    for field in &entity.fields {
        fields.push((field.source_order, field_value(field, entity)?));
    }
    for field in &entity.object_fields {
        fields.push((field.source_order, object_value(field)?));
    }
    fields.sort_by_key(|(order, _)| *order);
    value["fields"] = Value::Array(fields.into_iter().map(|(_, field)| field).collect());
    value["domain"] = domain_json.clone();
    value["domain"]["rustGraphCompositionAvailable"] =
        json!(domain.rust_teaql_dependency_version.starts_with("4.3."));
    Ok(value)
}

fn field_index(entity: &RenderEntity) -> String {
    let mut out = String::from("\n\n## Field-specific Query Assist\n\nUse the canonical KSML field name from this list. Do not substitute a language member name, JSON name, or database column.\n\n| KSML field | Type | Field help |\n| --- | --- | --- |\n");
    let mut fields = Vec::new();
    for field in &entity.fields {
        fields.push((
            field.source_order,
            field.name.as_str(),
            field.source_type_name.clone(),
        ));
    }
    for field in &entity.object_fields {
        fields.push((
            field.source_order,
            field.name.as_str(),
            format!("relation<{}>", field.target_module),
        ));
    }
    fields.sort_by_key(|(order, _, _)| *order);
    for (_, name, kind) in fields {
        writeln!(
            out,
            "| `{name}` | `{kind}` | `rust-assist-query/{}.{name}` |",
            entity.name
        )
        .unwrap();
    }
    if !entity.reverse_relations.is_empty() {
        out.push_str("\nGenerated reverse relations are derived from referencing KSML fields. Use their exact generated location for relation selection and access:\n\n| Generated reverse relation | Child entity | Field help |\n| --- | --- | --- |\n");
        for relation in &entity.reverse_relations {
            writeln!(
                out,
                "| `{}` | `{}` | `rust-assist-query/{}.{}` |",
                crate::preview_domain::snake(&relation.java_member_name),
                relation.raw_target_name,
                entity.name,
                crate::preview_domain::snake(&relation.java_member_name)
            )
            .unwrap();
        }
    }
    out
}

fn reverse_relation_help(
    entity: &RenderEntity,
    relation: &crate::context::RenderReverseRelation,
    location: &str,
) -> String {
    let name = &relation.rust_member_name;
    let plural = &relation.target_plural;
    let member_names = &relation.rust_member_names;
    let child = &relation.raw_target_name;
    let value_type = if relation.many {
        format!("SmartList<{}>", relation.target_struct)
    } else {
        relation.target_struct.clone()
    };
    format!(
        "<!-- ephemeral -->\n\n# Rust Query Reverse-Relation Assist — `{location}`\n\nKSML entity: `{}`  \nGenerated reverse relation: `{name}`  \nChild entity: `{child}`\n\n| Capability | Generated API |\n| --- | --- |\n| Select | `.select_{name}()`, `.select_{name}_with(Q::{plural}_minimal())` |\n| Filter nested request | `with_{name}_matching(request)`, `without_{name}_matching(request)` |\n| Presence | `have_{member_names}()`, `have_no_{member_names}()` |\n| Entity access | `entity.{name}()` returns `RelationHandle<'_, {value_type}>` |\n\n`RelationHandle` never performs an implicit database query. Inspect `state()` when the distinction matters, and call `value()` only after the relation was explicitly selected. `value()` is present for Loaded and loaded-empty lists; it is absent for NotLoaded (and for an empty to-one relation).\n\n```rust\nlet handle = entity.{name}();\nlet value = handle.value().ok_or(\"{name} was not loaded\")?;\n```\n\nApply query methods before `.purpose(...)`; execution still requires non-empty `.comment(...)`, `.purpose(...)`, and exactly one context argument.",
        entity.name
    )
}

pub(crate) fn render(
    domain: &RenderDomain,
    scope: &str,
    location: Option<&str>,
) -> Result<String, String> {
    let is_field =
        scope == "rust-assist-query" && location.is_some_and(|value| value.contains('.'));
    let Some((template, contract)) = sources(scope, is_field) else {
        return Err(format!("unsupported Rust Assist scope {scope}"));
    };
    let (entity_name, field_name) =
        location.map_or((None, None), |value| match value.split_once('.') {
            Some((entity, field)) if scope == "rust-assist-query" => (Some(entity), Some(field)),
            _ => (Some(value), None),
        });
    let entity = entity_name.and_then(|name| domain.find_object_descriptor(name));
    if let Some(name) = entity_name {
        if entity.is_none() && scope != "rust-assist-tool-api" {
            return Ok(format!(
                "## ❌ Error\n\nObject `{name}` not found in model.\n"
            ));
        }
    }
    let mut domain_json = crate::rust_globals::domain_value(domain)?;
    domain_json["rustModuleName"] = json!(domain.rust_crate_name.replace('-', "_"));
    let mut vars = Map::new();
    vars.insert("domain".into(), domain_json.clone());
    vars.insert(
        "path".into(),
        json!(location.map_or(scope.to_string(), |part| format!("{scope}/{part}"))),
    );
    vars.insert("moduleName".into(), json!(entity_name.unwrap_or_default()));
    vars.insert("moduleContent".into(), Value::Null);
    if scope == "rust-assist-tool-api" {
        if let Some(name) = entity_name {
            let Some(content) = tool_module(name) else {
                return Ok(format!(
                    "## ❌ Error\n\nUnknown Tool API module `{name}`.\n"
                ));
            };
            vars.insert("moduleContent".into(), json!(content));
        }
    }
    if let Some(entity) = entity {
        let descriptor = entity_value(domain, entity, &domain_json)?;
        vars.insert("entity".into(), descriptor.clone());
        if let Some(name) = field_name {
            let object_location = location.expect("field Assist location");
            if crate::semantic::is_blank(name) || name.contains('.') {
                let index = field_index(entity);
                let table = index
                    .split_once("| KSML field")
                    .map(|(_, rest)| format!("| KSML field{rest}"))
                    .unwrap_or_default();
                return Ok(format!("## Unsupported Query Assist location\n\nRequested KSML object location: `{object_location}`\n\nOnly one KSML field is supported in the current field Assist version.\n\n{table}"));
            }
            if let Some(field) = descriptor["fields"].as_array().and_then(|fields| {
                fields
                    .iter()
                    .find(|field| field["name"] == name || field["ksmlName"] == name)
            }) {
                let mut field = field.clone();
                field["name"] = json!(entity.assist_field_name(domain, name));
                field["fieldType"]["name"] = json!(entity.assist_field_type_name(domain, name));
                vars.insert("field".into(), field);
                vars.insert(
                    "objectLocation".into(),
                    json!(object_location),
                );
            } else if let Some(relation) = entity
                .reverse_relations
                .iter()
                .find(|relation| crate::preview_domain::snake(&relation.java_member_name) == name)
            {
                let body =
                    reverse_relation_help(entity, relation, object_location);
                return Ok(body + "\n" + contract);
            } else {
                let index = field_index(entity);
                let table = index
                    .split_once("| KSML field")
                    .map(|(_, rest)| format!("| KSML field{rest}"))
                    .unwrap_or_default();
                return Ok(format!("## Unsupported Query Assist location\n\nRequested KSML object location: `{object_location}`\n\nField `{name}` was not found using exact KSML spelling.\n\n{table}"));
            }
        }
    }
    let rendered = StgTemplate::parse(template)?.render_template("index", vars)?;
    let rendered = if scope == "rust-assist-query" && field_name.is_none() {
        match entity {
            Some(entity) => rendered + "\n" + &field_index(entity),
            None => rendered + "\n",
        }
    } else {
        rendered
    };
    let extra_newlines = if scope == "rust-assist-query" && field_name.is_none() {
        ""
    } else if scope == "rust-assist-tool-api" && entity_name.is_some() {
        "\n\n"
    } else if matches!(
        scope,
        "rust-assist-create"
            | "rust-assist-update"
            | "rust-assist-delete"
            | "rust-assist-query"
            | "rust-assist-expression"
            | "rust-assist-list-page"
            | "rust-assist-i18n"
            | "rust-assist-tool-api"
    ) {
        "\n"
    } else {
        ""
    };
    Ok(rendered + extra_newlines + contract)
}
