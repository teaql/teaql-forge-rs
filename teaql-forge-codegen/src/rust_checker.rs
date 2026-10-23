use crate::context::{RenderDomain, RenderEntity};
use crate::java_stg::StgTemplate;
use heck::ToPascalCase;
use serde_json::{Map, Value, json};

pub(crate) fn render_checker(
    domain: &RenderDomain,
    entity: &RenderEntity,
) -> Result<String, String> {
    let mut descriptor = crate::rust_entity::descriptor_value(entity)?;
    let item = descriptor
        .as_object_mut()
        .ok_or("entity is not an object")?;
    item.insert(
        "businessIdRootField".into(),
        crate::business_id::root_field(entity, domain, false).map(|(_, member)| {
            let target = member.target(domain).map(|root| json!({"rustStructName":root.rust_struct})).unwrap_or(Value::Null);
            json!({"fieldType": {"refer": target}})
        }).unwrap_or(Value::Null),
    );
    let mut fields = entity
        .fields
        .iter()
        .map(|field| {
            let mut value = crate::rust_entity::field_value(field)?;
            if crate::business_id::is_business_id(field) {
                value["rustBusinessIdSlotTypeName"] = json!(format!(
                    "{}{}BusinessIdSlot",
                    entity.rust_struct,
                    field.name.to_pascal_case()
                ));
                value["objectDescriptor"] = json!({"rustModuleName": entity.rust_module});
            }
            Ok((field.source_order, value))
        })
        .collect::<Result<Vec<_>, String>>()?;
    for relation in &entity.object_fields {
        let mut value = crate::rust_entity::object_field_value(relation)?;
        let field = value.as_object_mut().ok_or("relation is not an object")?;
        field.insert(
            "canonicalPropertyName".into(),
            Value::String(crate::preview_domain::snake(&relation.name)),
        );
        field.insert("isBusinessId".into(), json!(false));
        field.insert("contextRoot".into(), relation.context_root.clone());
        field.insert("createFunction".into(), json!(relation.automatic_function(domain, "createFunction")));
        field.insert("updateFunction".into(), json!(relation.automatic_function(domain, "updateFunction")));
        fields.push((relation.source_order, value));
    }
    fields.sort_by_key(|(order, _)| *order);
    item.insert(
        "fields".into(),
        Value::Array(fields.into_iter().map(|(_, value)| value).collect()),
    );
    let mut vars = Map::new();
    vars.insert("domain".into(), Value::Null);
    vars.insert("objectDescriptor".into(), descriptor);
    StgTemplate::parse(include_str!("../templates/rust-lib-core/checker/index.stg"))?
        .render(vars, "")
}
