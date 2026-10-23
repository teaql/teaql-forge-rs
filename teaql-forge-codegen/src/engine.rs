use crate::context::RenderDomain;
use minijinja::{context, Environment};

#[derive(Debug, Clone)]
pub struct GeneratedFile {
    pub path: String,
    pub content: String,
}

#[derive(Debug, thiserror::Error)]
pub enum GenerationError {
    #[error("Unsupported scope: {0}")]
    UnsupportedScope(String),
    #[error("{0}")]
    InvalidJavaModel(String),
    #[error("{0}")]
    InvalidRustModel(String),
    #[error(transparent)]
    Render(#[from] minijinja::Error),
}

pub fn render_external_preview(domain: &RenderDomain, source: &str) -> Result<String, String> {
    let (vars, graph) = crate::preview_graph::context(domain);
    crate::java_stg::StgTemplate::parse_preview(source)?.render_graph_preview(vars, graph)
}

pub fn render_named_external_preview(domain: &RenderDomain, path: &str, source: &str) -> Result<String, String> {
    let filename=path.rsplit('/').next().unwrap_or(path);
    let data_design=matches!(filename,"data-design"|"data-design.html"|"data-design-react"|"data-design-react.html")
        || filename.eq_ignore_ascii_case("data-design.md");
    if !data_design { return render_external_preview(domain,source); }
    let (mut vars,mut graph)=crate::preview_graph::context(domain);
    vars["domain"]=crate::graph_service::attach(domain,&mut graph);
    crate::java_stg::StgTemplate::parse_preview(source)?.render_graph_preview(vars,graph)
}

pub fn generate_for_scope(
    domain: &RenderDomain,
    scope: &str,
) -> Result<Vec<GeneratedFile>, GenerationError> {
    let normalized = scope.trim().to_ascii_lowercase();
    let scope = if normalized.is_empty() {
        "rust-lib-core"
    } else {
        &normalized
    };
    let entity_view;
    let domain =
        if scope.starts_with("rust") || scope.starts_with("golang") || scope.starts_with("swift") {
            entity_view = domain.entity_domain();
            &entity_view
        } else {
            domain
        };
    match scope {
        "frontend" | "tool" => {
            crate::auxiliary::generate(domain, scope).map_err(GenerationError::InvalidJavaModel)
        }
        "doc" | "model-view" | "data-design" => {
            let mut files = Vec::new();
            if scope != "data-design" {
                files.push(GeneratedFile {
                    path: "model-view.html".into(),
                    content: crate::java_doc::render_with_descriptor_override(domain, include_str!("../templates/doc-shadow-source/model-view.stg"))?
                        .unwrap_or_else(|| crate::java_doc::render_model_view(domain)),
                });
            }
            if scope != "model-view" {
                files.push(GeneratedFile {
                    path: "data-design.html".into(),
                    content: crate::java_doc::render_with_descriptor_override(domain, include_str!("../templates/doc-shadow-source/data-design.stg"))?
                        .unwrap_or_else(|| crate::java_doc::render_data_design(domain)),
                });
            }
            if scope != "model-view" {
                files.push(GeneratedFile {
                    path: "data-design-react.html".into(),
                    content: crate::java_doc::render_with_descriptor_override(domain, include_str!("../templates/doc-shadow-source/data-design-react.stg"))?
                        .unwrap_or_else(|| crate::java_doc::render_data_design_react(domain)),
                });
            }
            Ok(files)
        }
        "rust-lib" | "rust_lib" | "rust-lib-core" => Ok(generate_virtual_crate(domain)?),
        "rust-workspace" | "rust-app-console" | "rust-web-axum" | "rust-web-topcoat" => {
            let scope = if scope == "rust-workspace" {
                "rust-app-console"
            } else {
                scope
            };
            crate::rust_workspace::generate(domain, scope)
                .map_err(GenerationError::InvalidRustModel)
        }
        _ => Err(GenerationError::UnsupportedScope(scope.to_string())),
    }
}

pub use crate::rust_engine::{generate_all, generate_virtual_crate, generate_virtual_workspace};

pub fn render_preview(
    domain: &RenderDomain,
    template_name: &str,
    target_entity: Option<String>,
) -> Result<String, String> {
    let target_entity = if template_name.contains("-assist-") {
        target_entity.filter(|location| !location.is_empty())
    } else {
        target_entity
    };
    if let Some(source) = crate::templates::read(template_name) {
        return render_named_external_preview(domain, template_name, &source);
    }
    if let Some(result) = crate::preview::render_builtin(domain, template_name) {
        return result;
    }
    if template_name.contains("-assist-") && !has_builtin_assist_template(template_name) {
        let file = if template_name.ends_with("-assist-query")
            && target_entity.as_deref().is_some_and(|location| location.contains('.'))
        {
            "field.md.stg"
        } else {
            "prompt.md.stg"
        };
        return Err(format!(
            "Template not found or empty: generator/{template_name}/{file}"
        ));
    }
    if template_name == "doc/model-view.MD" {
        return Ok("## ❌ Error\n\nTemplate `model-view.MD.stg` not found in `doc`.\n".into());
    }
    if template_name == "doc/model-view" {
        return Ok(crate::java_doc::render_with_descriptor_override(domain, include_str!("../templates/doc-shadow-source/model-view.stg")).map_err(|error| error.to_string())?
            .unwrap_or_else(|| crate::java_doc::render_model_view(domain)));
    }
    if template_name == "doc/data-design-react" {
        return Ok(crate::java_doc::render_with_descriptor_override(domain, include_str!("../templates/doc-shadow-source/data-design-react.stg")).map_err(|error| error.to_string())?
            .unwrap_or_else(|| crate::java_doc::render_data_design_react(domain)));
    }
    if template_name == "doc/data-design" {
        return Ok(crate::java_doc::render_with_descriptor_override(domain, include_str!("../templates/doc-shadow-source/data-design.stg")).map_err(|error| error.to_string())?
            .unwrap_or_else(|| crate::java_doc::render_data_design(domain)));
    }
    if template_name == "index" {
        return crate::index::render_index(domain);
    }
    if template_name.starts_with("rust-assist-") {
        return crate::rust_assist::render(domain, template_name, target_entity.as_deref());
    }
    let mut env = Environment::new();
    env.set_trim_blocks(true);
    env.set_lstrip_blocks(true);

    env.add_template(
        "doc/model-view",
        include_str!("../templates/doc/model-view.j2"),
    )
    .unwrap();
    env.add_template(
        "doc/data-design",
        include_str!("../templates/doc/data-design.j2"),
    )
    .unwrap();
    env.add_template(
        "doc/data-design-react",
        include_str!("../templates/doc/data-design-react.j2"),
    )
    .unwrap();
    env.add_template(
        "rust-assist-create",
        include_str!("../templates/rust-assist-create/prompt.md.j2"),
    )
    .unwrap();
    env.add_template(
        "rust-assist-debug",
        include_str!("../templates/rust-assist-debug/prompt.md.j2"),
    )
    .unwrap();
    env.add_template(
        "rust-assist-delete",
        include_str!("../templates/rust-assist-delete/prompt.md.j2"),
    )
    .unwrap();
    env.add_template(
        "rust-assist-expression",
        include_str!("../templates/rust-assist-expression/prompt.md.j2"),
    )
    .unwrap();
    env.add_template(
        "rust-assist-list-page",
        include_str!("../templates/rust-assist-list-page/prompt.md.j2"),
    )
    .unwrap();
    env.add_template(
        "rust-assist-query",
        include_str!("../templates/rust-assist-query/prompt.md.j2"),
    )
    .unwrap();
    env.add_template(
        "rust-assist-runtime-custom",
        include_str!("../templates/rust-assist-runtime-custom/prompt.md.j2"),
    )
    .unwrap();
    env.add_template(
        "rust-assist-tool-api",
        include_str!("../templates/rust-assist-tool-api/prompt.md.j2"),
    )
    .unwrap();
    env.add_template(
        "rust-assist-update",
        include_str!("../templates/rust-assist-update/prompt.md.j2"),
    )
    .unwrap();

    let template = env.get_template(template_name).map_err(|e| e.to_string())?;

    let context = if let Some(entity_name) = target_entity {
        let object_descriptor = domain.find_object_descriptor(&entity_name).cloned();
        if object_descriptor.is_none() {
            return Err(format!("Entity {} not found", entity_name));
        }
        context! {
            domain => domain,
            objectDescriptor => object_descriptor.clone().unwrap(),
            entity => object_descriptor.unwrap(),
        }
    } else {
        context! {
            domain => domain,
        }
    };

    template.render(context).map_err(|e| e.to_string())
}

fn has_builtin_assist_template(scope: &str) -> bool {
    let Some((language, action)) = scope.split_once("-assist-") else {
        return false;
    };
    matches!(
        language,
        "rust"
    ) && matches!(
        action,
        "query"
            | "list-page"
            | "create"
            | "update"
            | "delete"
            | "expression"
            | "ensure-schema"
            | "debug"
            | "i18n"
            | "runtime-custom"
            | "tool-api"
    )
}

pub const ASSIST_QUERY_NULL_ENTITY_MESSAGE: &str =
    "Cannot invoke \"com.skynet.codegenerator.meta.ObjectDescriptor.getFields()\" because \"entity\" is null";

pub fn assist_query_null_entity_path(path: &str) -> bool {
    let mut parts = path.split('/');
    let scope = parts.next().unwrap_or("");
    scope.ends_with("-assist-query")
        && parts.next() == Some("")
        && parts.any(|part| !part.is_empty())
}
