use crate::context::{RenderDomain, RenderEntity};
use crate::engine::GenerationError;

pub(crate) fn java_package_segment(name: &str) -> String {
    // Heck drops private-use supplementary code points while Hutool's camel
    // conversion keeps them. Those characters are valid Woodstox XML names.
    if name.chars().any(|ch| ch as u32 >= 0xf0000) {
        crate::preview_domain::camel(name).to_lowercase()
    } else {
        heck::ToLowerCamelCase::to_lower_camel_case(name).to_lowercase()
    }
}

pub(crate) fn render_shadow_source(
    domain: &RenderDomain, entity: Option<&RenderEntity>, package: &str, source: &str,
) -> Result<String, GenerationError> {
    let (mut vars, graph) = crate::preview_graph::context(domain);
    vars.insert("__stg_generator_naming".into(), serde_json::json!(true));
    let own_package = if let Some(entity) = entity {
        let index = domain.entities.iter().position(|item| item.name == entity.name).unwrap();
        vars.insert("objectDescriptor".into(), serde_json::json!({"__stg_ref":format!("object:{index}")}));
        format!("{package}.{}", crate::shadow::java_package_segment(&entity.name))
    } else { package.to_owned() };
    crate::java_stg::StgTemplate::parse_preview(source)
        .and_then(|template| template.render_graph(vars, graph, &own_package))
        .map(|value| value.replace("__stg_empty_generator_imports__", ""))
        .map_err(GenerationError::InvalidJavaModel)
}
