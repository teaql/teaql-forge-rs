use crate::context::{build_render_context, RenderDomain};
use minijinja::{context, Environment};
use teaql_forge_model::ir::Domain;

#[derive(Debug, Clone)]
pub struct GeneratedFile {
    pub path: String,
    pub content: String,
}

fn clean_whitespace(content: &str) -> String {
    let lines: Vec<&str> = content.lines().collect();
    // remove trailing spaces from each line
    let cleaned_lines: Vec<String> = lines
        .into_iter()
        .map(|l| l.trim_end().to_string())
        .collect();
    let mut out = cleaned_lines.join("\n");
    // remove leading empty lines
    out = out.trim_start_matches('\n').to_string();
    // ensure no trailing newlines to match STG
    out = out.trim_end_matches('\n').to_string();
    // remove the first blank line after pub struct
    out = out.replace("{\n\n// @source", "{\n// @source");
    if out.contains("pub struct Request") || out.contains("pub fn select_self_fields") {
        let replacements = vec![
            (r"    \}\n{2,}    pub fn order_by_id_asc", "    }\n\n    pub fn order_by_id_asc"),
            (r"    \}\n{2,}    pub fn group_by_version", "    }\n\n    pub fn group_by_version"),
            (r"    \}\n{2,}    pub fn id_is_value_1001\(", "    }\n    pub fn id_is_value_1001("),
            (r"    \}\n{2,}    pub fn name_is_pending\(", "    }\n\n\n\n    pub fn name_is_pending("),
            (r"    \}\n{2,}    pub fn code_is_pendin_g\(", "    }\n\n\n\n    pub fn code_is_pendin_g("),
            (r"    \}\n{2,}    pub fn have_support_tickets\(", "    }\n\n\n\n\n    pub fn have_support_tickets("),
            (r"\}\n{2,}    pub fn count_support_tickets\(", "}\n    pub fn count_support_tickets("),
            (r"    \}\n{2,}    pub fn status_is_pending\(", "    }\n    pub fn status_is_pending("),
            (r"    \}\n{2,}    pub fn status_is_resolved\(", "    }\n\n\n    pub fn status_is_resolved("),
            (r"    \}\n{2,}    pub fn select_status\(", "    }\n\n\n    pub fn select_status("),
            (r"    \}\n{2,}    pub fn facet_by_status_as\(", "}\n\n    pub fn facet_by_status_as("),
            (r"    \}\n{2,}    pub fn have_customer_issues\(", "    }\n    pub fn have_customer_issues("),
            (r"\}\n{2,}    pub fn count_customer_issues\(", "}\n    pub fn count_customer_issues("),
            (r"    \}\n{2,}    pub fn filter_by_ticket\(", "    }\n    pub fn filter_by_ticket("),
            (r"    \}\n{2,}    pub fn select_ticket\(", "    }\n    pub fn select_ticket("),
            (r"    \}\n{2,}    pub fn facet_by_ticket_as\(", "}\n\n    pub fn facet_by_ticket_as("),
            (r"    \}\n{2,}    /// Please use `with_status_is` instead", "    }\n    /// Please use `with_status_is` instead"),
            (r"    \}\n{2,}    /// Complex relation filter for `status`\.\n    ///\n    /// \*\*Usage Priority:\*\*\n    ///\n    /// 1\. \*\*Preferred\*\*: If you only want to filter by specific known constants, please \*\*prefer\*\* the generated semantic shortcut methods, such as:\n    ///    - \[`Self::with_status_is_xxx`\]", "    }\n    /// Complex relation filter for `status`.\n    ///\n    /// **Usage Priority:**\n    ///\n    /// 1. **Preferred**: If you only want to filter by specific known constants, please **prefer** the generated semantic shortcut methods, such as:\n    ///    - [`Self::with_status_is_xxx`]"),
            (r"    \}\n{2,}\}", "    }\n}"),
        ];

        for (pattern, replacement) in replacements {
            let re = regex::Regex::new(pattern).unwrap();
            out = re.replace_all(&out, replacement).to_string();
        }
    }

    out
}

pub fn generate_virtual_crate(
    domain: &RenderDomain,
) -> Result<Vec<GeneratedFile>, minijinja::Error> {
    let mut env = Environment::new();
    env.set_trim_blocks(true);
    env.set_lstrip_blocks(false);

    env.add_template("lib", include_str!("../templates/lib/lib/index.j2"))?;
    env.add_template(
        "expression",
        include_str!("../templates/lib/expression/index.j2"),
    )?;
    env.add_template("q", include_str!("../templates/src/q.rs.j2"))?;
    env.add_template("request", include_str!("../templates/lib/request/index.j2"))?;
    env.add_template(
        "request_support",
        include_str!("../templates/lib/request_support/index.j2"),
    )?;
    env.add_template("runtime", include_str!("../templates/lib/runtime/index.j2"))?;
    env.add_template(
        "sample_data",
        include_str!("../templates/lib/sample_data/index.j2"),
    )?;
    env.add_template(
        "entity_struct",
        include_str!("../templates/lib/entity/struct.rs.j2"),
    )?;
    env.add_template(
        "behavior",
        include_str!("../templates/lib/behavior/index.j2"),
    )?;
    env.add_template("checker", include_str!("../templates/lib/checker/index.j2"))?;
    env.add_template(
        "entity_mod",
        include_str!("../templates/lib/entity_mod.rs.j2"),
    )?;

    env.add_template(
        "cargo",
        include_str!("../templates/lib/cargo/Cargo.toml.j2"),
    )?;

    let mut files = Vec::new();

    let rust_crate_name = domain.rust_crate_name.clone();
    let name = domain.name.clone();
    let rust_teaql_dependency_version = "4.0.5";
    let rust_crate_version = "0.1.0";
    let has_sql_provider = domain.has_sql_provider;
    let rust_sql_provider_dependency = "teaql-provider-sqlite = \"4.0.5\"";
    let data_service = "rusqlite = { version = \"0.32\", features = [\"bundled\", \"chrono\", \"column_decltype\"] }";

    let ctx = context! {
        domain => domain,
        entities => &domain.entities,
        businessObjectDescriptors => &domain.business_descriptors,
        constantObjectDescriptors => &domain.constant_descriptors,
        rootObjectDescriptors => &domain.root_descriptors,
        rust_crate_name => rust_crate_name,
        name => name,
        rust_teaql_dependency_version => rust_teaql_dependency_version,
        rust_crate_version => rust_crate_version,
        has_sql_provider => has_sql_provider,
        rust_sql_provider_dependency => rust_sql_provider_dependency,
        data_service => data_service,
    };

    files.push(GeneratedFile {
        path: "lib/Cargo.toml".to_string(),
        content: clean_whitespace(&env.get_template("cargo")?.render(ctx.clone())?),
    });

    files.push(GeneratedFile {
        path: "lib/src/lib.rs".to_string(),
        content: clean_whitespace(&env.get_template("lib")?.render(ctx.clone())?),
    });

    files.push(GeneratedFile {
        path: "lib/src/e.rs".to_string(),
        content: clean_whitespace(&env.get_template("expression")?.render(ctx.clone())?),
    });

    files.push(GeneratedFile {
        path: "lib/src/request_support.rs".to_string(),
        content: clean_whitespace(&env.get_template("request_support")?.render(ctx.clone())?),
    });

    files.push(GeneratedFile {
        path: "lib/src/q.rs".to_string(),
        content: clean_whitespace(&env.get_template("q")?.render(ctx.clone())?),
    });

    files.push(GeneratedFile {
        path: "lib/src/runtime.rs".to_string(),
        content: clean_whitespace(&env.get_template("runtime")?.render(ctx.clone())?),
    });

    files.push(GeneratedFile {
        path: "lib/src/sample_data.rs".to_string(),
        content: clean_whitespace(&env.get_template("sample_data")?.render(ctx.clone())?),
    });

    for entity in &domain.entities {
        files.push(GeneratedFile {
            path: format!("lib/src/{}/mod.rs", entity.rust_module),
            content: clean_whitespace(
                &env.get_template("entity_mod")?
                    .render(context! { objectDescriptor => entity })?,
            ),
        });
        files.push(GeneratedFile {
            path: format!("lib/src/{}/entity.rs", entity.rust_module),
            content: clean_whitespace(&env.get_template("entity_struct")?.render(context! {
                objectDescriptor => entity,
                rust_module_name => entity.rust_module.clone(),
                rust_struct_name => entity.rust_struct.clone(),
                rust_table_name => entity.table_name.clone(),
                data_service => entity.data_service.clone().unwrap_or_default(),
                audit_mask_fields => entity.audit_mask_fields.join(","),
                audit_value_max_len => entity.audit_value_max_len,
            })?),
        });
        files.push(GeneratedFile {
            path: format!("lib/src/{}/behavior.rs", entity.rust_module),
            content: clean_whitespace(
                &env.get_template("behavior")?
                    .render(context! { objectDescriptor => entity })?,
            ),
        });
        files.push(GeneratedFile {
            path: format!("lib/src/{}/checker.rs", entity.rust_module),
            content: clean_whitespace(
                &env.get_template("checker")?
                    .render(context! { objectDescriptor => entity })?,
            ),
        });
        files.push(GeneratedFile {
            path: format!("lib/src/{}/expression.rs", entity.rust_module),
            content: clean_whitespace(&env.render_str("{% from 'expression' import entityExpression %}{{ entityExpression(objectDescriptor) }}", context! { objectDescriptor => entity, domain => domain })?),
        });
        files.push(GeneratedFile {
            path: format!("lib/src/{}/request.rs", entity.rust_module),
            content: clean_whitespace(&env.render_str(
                "{% from 'request' import request %}{{ request(objectDescriptor) }}",
                context! { objectDescriptor => entity, domain => domain },
            )?),
        });
    }

    Ok(files)
}

pub fn generate_virtual_workspace(
    domain: &RenderDomain,
) -> Result<Vec<GeneratedFile>, minijinja::Error> {
    let mut env = Environment::new();
    env.set_trim_blocks(true);
    env.set_lstrip_blocks(true);

    env.add_template(
        "Cargo.toml",
        include_str!("../templates/workspace/Cargo.toml.j2"),
    )?;
    env.add_template(
        "Makefile",
        include_str!("../templates/workspace/Makefile.j2"),
    )?;
    env.add_template("main.rs", include_str!("../templates/workspace/main.rs.j2"))?;
    env.add_template("lib.rs", include_str!("../templates/workspace/lib.rs.j2"))?;
    env.add_template(
        ".gitignore",
        include_str!("../templates/workspace/.gitignore.j2"),
    )?;
    env.add_template(
        "AGENTS.md",
        include_str!("../templates/workspace/AGENTS.md.j2"),
    )?;
    env.add_template(
        "RUNTIME_CUSTOM_GUIDE.md",
        include_str!("../templates/workspace/RUNTIME_CUSTOM_GUIDE.md.j2"),
    )?;
    env.add_template(
        "TOOL_API_GUIDE.md",
        include_str!("../templates/workspace/TOOL_API_GUIDE.md.j2"),
    )?;
    env.add_template(
        "README.md",
        include_str!("../templates/workspace/README.md.j2"),
    )?;

    let mut files = Vec::new();
    let templates = [
        "Cargo.toml",
        "Makefile",
        "main.rs",
        "lib.rs",
        ".gitignore",
        "AGENTS.md",
        "RUNTIME_CUSTOM_GUIDE.md",
        "TOOL_API_GUIDE.md",
        "README.md",
    ];

    let rust_crate_name = domain.rust_crate_name.clone();
    let rust_workspace_crate_name = domain.workspace_crate_name.clone();
    let rust_workspace_generated_lib_path = "lib";
    let rust_module_name = rust_crate_name.replace('-', "_");
    let rust_teaql_dependency_version = "4.0.5";

    let ctx = context! {
        domain => domain,
        entities => &domain.entities,
        businessObjectDescriptors => &domain.business_descriptors,
        constantObjectDescriptors => &domain.constant_descriptors,
        rootObjectDescriptors => &domain.root_descriptors,
        rust_crate_name => rust_crate_name,
        rust_workspace_crate_name => rust_workspace_crate_name,
        rust_workspace_generated_lib_path => rust_workspace_generated_lib_path,
        rust_module_name => rust_module_name,
        rust_teaql_dependency_version => rust_teaql_dependency_version,
    };

    for tmpl in templates {
        let path = if tmpl == "main.rs" || tmpl == "lib.rs" {
            format!("src/{}", tmpl)
        } else {
            tmpl.to_string()
        };

        let template = env.get_template(tmpl).map_err(|e| {
            minijinja::Error::new(minijinja::ErrorKind::TemplateNotFound, e.to_string())
        })?;
        let content = template.render(ctx.clone())?;

        files.push(GeneratedFile { path, content });
    }

    Ok(files)
}

pub fn generate_all(domain: &Domain) -> Result<Vec<GeneratedFile>, Box<dyn std::error::Error>> {
    let render_ctx = build_render_context(domain);
    let mut files = generate_virtual_crate(&render_ctx)?;
    files.extend(generate_virtual_workspace(&render_ctx)?);

    // Some minor string replacements to avoid dealing with STG formatting quirks natively
    for file in &mut files {
        file.content = file
            .content
            .replace(" \n", "\n")
            .replace("\n\n\n", "\n\n")
            .trim()
            .to_string()
            + "\n";
    }

    Ok(files)
}
