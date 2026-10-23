use crate::context::RenderDomain;
use crate::java_stg::StgTemplate;
use serde_json::{Map, Value};

pub(crate) fn domain_value(domain: &RenderDomain) -> Result<Value, String> {
    let profile = crate::preview_beans::json_field_profile(domain)?;
    let mut value = serde_json::to_value(domain).map_err(|error| error.to_string())?;
    let item = value.as_object_mut().ok_or("domain is not an object")?;
    let descriptors = domain
        .object_descriptors
        .iter()
        .map(|entity| crate::rust_entity::descriptor_value_with_profile(entity, profile))
        .collect::<Result<Vec<_>, _>>()?;
    item.insert("objectDescriptors".into(), Value::Array(descriptors));
    if let Some(service) = item.get_mut("dataService").and_then(Value::as_object_mut) {
        service.insert(
            "rustHasDataServiceProvider".into(),
            Value::Bool(
                domain.data_service.rust_has_sql_provider || domain.data_service.name == "linux",
            ),
        );
        service.insert(
            "rustIsLinuxProvider".into(),
            Value::Bool(domain.data_service.name == "linux"),
        );
        service.insert(
            "rustSupportsBusinessIdInfrastructure".into(),
            Value::Bool(domain.data_service.name == "sqlite"),
        );
        service.insert(
            "rustBusinessIdAllocatorType".into(),
            Value::String("teaql_provider_sqlite::SqliteBusinessIdAllocator".into()),
        );
    }
    item.insert(
        "businessIdFields".into(),
        Value::Array(
            domain
                .entities
                .iter()
                .flat_map(|entity| &entity.fields)
                .filter(|field| {
                    matches!(
                        field.source_type_name.as_str(),
                        "businessId" | "business_id"
                    )
                })
                .map(|field| Value::String(field.name.clone()))
                .collect(),
        ),
    );
    item.insert(
        "rustGeneratedBootstrap".into(),
        // The source enhancer callback aborts this ST render on failure and
        // preserves the bytes written before the property was evaluated.
        crate::rust_bootstrap::render(domain).map(Value::String)
            .unwrap_or_else(|error| crate::preview_rust::fault(&error)),
    );
    Ok(value)
}

pub(crate) fn render_request_support(domain: &RenderDomain) -> Result<String, String> {
    let mut vars = Map::new();
    vars.insert("domain".into(), domain_value(domain)?);
    let rendered =
        StgTemplate::parse(include_str!("../templates/rust-lib-core/request/index.stg"))?
            .render_template("index", vars)?;
    Ok(format!("{}\n", rendered.replace("\n        \n", "\n\n")))
}

pub(crate) fn render_q(domain: &RenderDomain) -> Result<String, String> {
    let mut vars = Map::new();
    vars.insert("domain".into(), domain_value(domain)?);
    StgTemplate::parse_for_vars(include_str!("../templates/rust-lib-core/request/index.stg"), &vars)?
        .render_template("q", vars)
}

pub(crate) fn render_runtime(domain: &RenderDomain) -> Result<String, String> {
    let value = domain_value(domain)?;
    let bootstrap_fault = value["rustGeneratedBootstrap"].get("__stg_getter_fault").is_some();
    let mut vars = Map::new();
    vars.insert("domain".into(), value);
    if bootstrap_fault {
        // Failed source renders retain their writer prefix, including pending
        // newlines; the normal generator layout adapter trims final newlines.
        return StgTemplate::parse_preview(include_str!("../templates/rust-lib-core/runtime/index.stg"))?
            .render_template("index", vars)
            .map(|value| value.replace("\n    \n", "\n\n"));
    }
    let rendered =
        StgTemplate::parse(include_str!("../templates/rust-lib-core/runtime/index.stg"))?
            .render_template("index", vars)?;
    let rendered = rendered
        .replace(
            "\n\n\nimpl From<teaql_runtime::RuntimeError>",
            "\n\nimpl From<teaql_runtime::RuntimeError>",
        )
        .replace(
            "DataServiceMutationExecutor::new(pool);\n    let mut context",
            "DataServiceMutationExecutor::new(pool);    let mut context",
        )
        .replace(
            "DataServiceMutationExecutor::new(pool);    let mut context",
            "DataServiceMutationExecutor::new(pool);let mut context",
        )
        .replace("\n    \n", "\n\n")
        .replace(
            "before creating an Aggregate.\n\n    let executor",
            "before creating an Aggregate.\n    let executor",
        )
        .replace(
            "\n\n\npub fn repository_registry",
            "\n\npub fn repository_registry",
        )
        .replace(
            "}\n\npub fn repository_registry",
            "}\npub fn repository_registry",
        );
    let rendered = rendered
        .replace(
            "Allocator;\n\npub type DataServicePool",
            "Allocator;\npub type DataServicePool",
        )
        .replace(
            "Configuration(error.to_string()))?;\n\n    let executor",
            "Configuration(error.to_string()))?;\n    let executor",
        );
    if domain.data_service.rust_sql_supports_transactions || domain.data_service.name == "linux" {
        Ok(rendered
            .replace("ServiceRuntimeError::Rusqlite(err)\n    }\n}\n\nimpl", "ServiceRuntimeError::Rusqlite(err)\n    }\n}\nimpl")
            .replace("DataServiceIdGenerator::from_executor(mutation_executor.clone());\n    let mut context", "DataServiceIdGenerator::from_executor(mutation_executor.clone());let mut context")
            .replace("}\npub fn repository_registry", "}\n\npub fn repository_registry"))
    } else {
        Ok(rendered)
    }
}

pub(crate) fn render_sample_data(domain: &RenderDomain) -> Result<String, String> {
    let mut vars = Map::new();
    vars.insert("domain".into(), domain_value(domain)?);
    let mut sorted = domain.entities.iter().collect::<Vec<_>>();
    let depths = domain.entities.iter().zip(crate::preview_objects::source_depths(domain))
        .map(|(entity, depth)| (entity.rust_struct.clone(), depth))
        .collect::<std::collections::HashMap<_, _>>();
    sorted.sort_by(|a, b| {
        depths[&a.rust_struct]
            .cmp(&depths[&b.rust_struct])
            .then_with(|| a.name.cmp(&b.name))
    });
    let roots = domain
        .root_descriptors
        .iter()
        .map(|entity| entity.rust_struct.as_str())
        .collect::<std::collections::HashSet<_>>();
    let has_business = sorted
        .iter()
        .any(|entity| !entity.constant && !roots.contains(entity.rust_struct.as_str()));
    for key in [
        "rootObjectDescriptors",
        "constantObjectDescriptors",
        "businessObjectDescriptors",
    ] {
        let entities = sorted
            .iter()
            .filter(|entity| match key {
                "rootObjectDescriptors" => roots.contains(entity.rust_struct.as_str()),
                "constantObjectDescriptors" => entity.constant,
                _ => !entity.constant && !roots.contains(entity.rust_struct.as_str()),
            })
            .copied()
            .collect::<Vec<_>>();
        vars.insert(
            key.into(),
            Value::Array(
                entities
                    .into_iter()
                    .map(crate::rust_entity::descriptor_value)
                    .collect::<Result<Vec<_>, _>>()?,
            ),
        );
    }
    let rendered = StgTemplate::parse(include_str!(
        "../templates/rust-lib-core/sample_data/index.stg"
    ))?
    .render_template("index", vars)?;
    Ok(format!(
        "{}{}",
        rendered.replace(
            "\n        entity.audit_as(\"Init Sample Data\")",
            "\nentity.audit_as(\"Init Sample Data\")"
        ),
        if has_business { "\n" } else { "\n\n\n" }
    ))
}
