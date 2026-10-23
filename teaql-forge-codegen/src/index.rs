use crate::context::RenderDomain;

pub(crate) fn render_index(domain: &RenderDomain) -> Result<String, String> {
    const TARGETS: &[&str] = &[
        "agent", "doc", "evaluation", "mergemodel",
        "rust-app-console", "rust-lib-core", "rust-web-axum", "rust-web-topcoat",
    ];
    let source = include_str!("../templates/index.html.stg");
    let (_, body) = source
        .split_once("::= <<\n")
        .ok_or("Invalid pinned source index template")?;
    let body = body
        .strip_suffix("\n>>\n")
        .ok_or("Invalid pinned source index template end")?;
    let target_tags = TARGETS
        .iter()
        .map(|target| format!("<span class=\"target-tag\">{target}</span>"))
        .collect::<String>();
    let first_entity = domain
        .entities
        .first()
        .map(|entity| entity.name.as_str())
        .unwrap_or("");
    Ok(body
        .replace(
            "$targets:{t | <span class=\"target-tag\">$t$</span>}$",
            &target_tags,
        )
        .replace("$first(domain.objectDescriptors).od_name$", first_entity)
        .replace("$domain.name$", &domain.name)
        .replace("        \n\n", "\n")
        .replace("        \n", "\n"))
}
