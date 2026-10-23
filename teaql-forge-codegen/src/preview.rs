use crate::context::RenderDomain;
use crate::java_stg::StgTemplate;
use serde_json::Map;

pub fn has_builtin(path: &str) -> bool {
    source(path).is_some()
}

fn source(path: &str) -> Option<&'static str> {
    crate::preview_templates::source(path).or_else(|| {
        let (target, _) = path.split_once('/')?;
        let filename = path.rsplit('/').next()?;
        crate::preview_templates::source(&format!("{target}/{filename}"))
    })
}

pub fn render_builtin(domain: &RenderDomain, path: &str) -> Option<Result<String, String>> {
    let source = source(path)?;
    Some((|| {
        let mut vars = Map::new();
        let scope = path.split('/').next().unwrap_or_default();
        let mut value = if scope.starts_with("rust-") {
            crate::rust_globals::domain_value(domain)?
        } else {
            serde_json::to_value(domain).map_err(|error| error.to_string())?
        };
        if scope.starts_with("rust-") {
            value.as_object_mut().unwrap().remove("rustStructName");
        }
        vars.insert("domain".into(), value);
        StgTemplate::parse_preview(source)?.render_preview(vars)
    })())
}
