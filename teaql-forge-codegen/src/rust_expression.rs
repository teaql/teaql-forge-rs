use crate::context::{RenderDomain, RenderEntity};
use crate::java_stg::StgTemplate;
use serde_json::{Map, Value};

pub(crate) fn render_expression(
    _domain: &RenderDomain,
    entity: &RenderEntity,
) -> Result<String, String> {
    let descriptor = crate::rust_entity::descriptor_value(entity)?;
    let template = StgTemplate::parse(include_str!(
        "../templates/rust-lib-core/expression/index.stg"
    ))?;
    let vars = || -> Result<Map<String, Value>, String> {
        let mut vars = Map::new();
        vars.insert("domain".into(), Value::Null);
        vars.insert("objectDescriptor".into(), descriptor.clone());
        Ok(vars)
    };
    let first = template.render_template("entityExpression", vars()?)?;
    let second = template.render_template("listExpression", vars()?)?;
    Ok(format!("{first}\n\n{second}"))
}
