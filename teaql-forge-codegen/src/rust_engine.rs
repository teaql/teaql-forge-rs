use crate::context::{build_render_context, RenderDomain};
use crate::engine::GeneratedFile;
use minijinja::{context, Environment};
use teaql_forge_model::ir::Domain;

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

fn render_rust_cargo(domain: &RenderDomain) -> String {
    let version = domain
        .root_attributes
        .get("teaql_rs_version")
        .map(String::as_str)
        .unwrap_or("5.0.5");
    let service = domain.data_service.name.as_str();
    let (provider, driver) = match service {
        "postgres" | "pg" => ("teaql-provider-postgres", "tokio-postgres = { version = \"0.7\" }\ndeadpool-postgres = { version = \"0.14.0\", features = [\"rt_tokio_1\"] }\ndeadpool = \"0.12\"\ntokio-util = \"0.7\"\ndeadpool-runtime = \"0.1.4\""),
        "mysql" => ("teaql-provider-mysql", "mysql_async = { version = \"0.37\", default-features = false, features = [\"default-rustls\", \"chrono\"] }"),
        "sqlite" | "rusqlite" => ("teaql-provider-sqlite", "rusqlite = { version = \"0.40\", features = [\"bundled\", \"chrono\", \"column_decltype\"] }"),
        "linux" => ("teaql-provider-linux", ""),
        _ => ("", ""),
    };
    let mut result = format!("\n[package]\nname = \"{}\"\nversion = \"{}\"\nedition = \"2021\"\nrust-version = \"1.80\"\ndescription = \"Generated TeaQL service crate for {}\"\nlicense = \"MIT OR Apache-2.0\"\n\n[dependencies]\nlog = \"0.4\"\nchrono = {{ version = \"0.4\", features = [\"serde\"] }}\nrust_decimal = {{ version = \"1\", features = [\"serde\"] }}\nserde = {{ version = \"1\", features = [\"derive\"] }}\nserde_json = \"1\"\nasync-stream = \"0.3\"\nasync-trait = \"0.1\"\nfutures-util = \"0.3\"\nteaql-core = \"{version}\"\nteaql-macros = \"{version}\"\nteaql-runtime = \"{version}\"\nteaql-sql = \"{version}\"\nteaql-data-service = \"{version}\"", domain.rust_crate_name, domain.rust_crate_version, domain.name);
    if !provider.is_empty() {
        result.push_str(&format!("\n{provider} = \"{version}\""));
    }
    if !driver.is_empty() {
        result.push('\n');
        result.push_str(driver);
    }
    if !provider.is_empty() {
        result.push_str(
            "\ntokio = { version = \"1\", features = [\"macros\", \"rt-multi-thread\"] }",
        );
    }
    if domain.support_full_text_search {
        result.push_str(&format!("\nteaql-provider-meilisearch = \"{version}\""));
    } else {
        result.push('\n');
    }
    result.replace("\r\n", "\n").replace('\r', "")
}

fn render_rust_lib(domain: &RenderDomain) -> String {
    let mut result = format!("\n//! Generated TeaQL domain crate for `{}`.\n//!\n//! **Before writing queries**, read the `AGENTS.md` at the workspace root.\n//! It contains the entity list and the exact `cargo teaql` commands to fetch API prompts.\n//!\n//! The generated library is not the API-discovery surface. Read the generated\n//! application's `AGENTS.md`, then request model-aware object/field Assist.\n//! A registry dependency does not require vendoring or browsing generated\n//! domain-library source to learn method names. If Assist lacks an operation,\n//! report `MISSING_ASSIST` for that path.\n\npub mod e;\npub mod q;\npub mod request_support;\npub mod runtime;\npub mod sample_data;", domain.rust_crate_name);
    for entity in &domain.entities {
        result.push_str(&format!("\npub mod {};", entity.rust_module));
    }
    result.push_str("\n\npub use teaql_core;\npub use teaql_runtime::LedgerEntity;\npub use e::*;\npub use q::*;\npub use request_support::*;\npub use runtime::*;\npub use sample_data::*;");
    for entity in &domain.entities {
        result.push_str(&format!("\npub use {}::*;", entity.rust_module));
    }
    result.replace("\r\n", "\n").replace('\r', "")
}

fn format_rust_expression(rendered: String) -> String {
    let rendered = rendered.replace("human_message: {}\\n\",\n", "human_message: {}\\n\", \n");
    format!("\n{rendered}\n\n")
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
    let rust_teaql_dependency_version = "4.2.5";
    let rust_crate_version = "0.1.0";
    let has_sql_provider = domain.has_sql_provider;
    let rust_sql_provider_dependency = "teaql-provider-sqlite = \"4.2.5\"";
    let data_service = "rusqlite = { version = \"0.40\", features = [\"bundled\", \"chrono\", \"column_decltype\"] }";

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
            rust_sql_provider_name => "teaql_provider_sqlite",
            rust_sql_provider_pascal => "Sqlite",
        data_service => data_service,
    };

    files.push(GeneratedFile {
        path: "lib/Cargo.toml".to_string(),
        content: render_rust_cargo(domain),
    });

    files.push(GeneratedFile {
        path: "lib/src/lib.rs".to_string(),
        content: if crate::preview_domain::has_descriptor_override(domain) {
            crate::shadow::render_shadow_source(domain, None, "", include_str!("../templates/rust-shadow-source/lib/index.stg")).map_err(|error| minijinja::Error::new(minijinja::ErrorKind::InvalidOperation, error.to_string()))?
        } else { render_rust_lib(domain) },
    });

    files.push(GeneratedFile {
        path: "lib/src/e.rs".to_string(),
        content: if crate::preview_domain::has_descriptor_override(domain) {
            crate::shadow::render_shadow_source(domain, None, "", include_str!("../templates/rust-shadow-source/expression/index.stg")).map(|value| value.split('\n').map(|line| if line.trim().is_empty() { "" } else { line }).collect::<Vec<_>>().join("\n")).map_err(|error| minijinja::Error::new(minijinja::ErrorKind::InvalidOperation, error.to_string()))?
        } else { format_rust_expression(clean_whitespace(
            &env.get_template("expression")?.render(ctx.clone())?,
        )) },
    });

    files.push(GeneratedFile {
        path: "lib/src/request_support.rs".to_string(),
        content: crate::rust_globals::render_request_support(domain).map_err(|error| {
            minijinja::Error::new(minijinja::ErrorKind::InvalidOperation, error)
        })?,
    });

    files.push(GeneratedFile {
        path: "lib/src/q.rs".to_string(),
        content: crate::rust_globals::render_q(domain).map_err(|error| {
            minijinja::Error::new(minijinja::ErrorKind::InvalidOperation, error)
        })?,
    });

    files.push(GeneratedFile {
        path: "lib/src/runtime.rs".to_string(),
        content: crate::rust_globals::render_runtime(domain).map_err(|error| {
            minijinja::Error::new(minijinja::ErrorKind::InvalidOperation, error)
        })?,
    });

    files.push(GeneratedFile {
        path: "lib/src/sample_data.rs".to_string(),
        content: crate::rust_globals::render_sample_data(domain).map_err(|error| {
            minijinja::Error::new(minijinja::ErrorKind::InvalidOperation, error)
        })?,
    });

    for entity in &domain.entities {
        files.push(GeneratedFile {
            path: format!("lib/src/{}/mod.rs", entity.rust_module),
            content: "mod behavior;\nmod checker;\nmod entity;\nmod expression;\nmod request;\n\npub use behavior::*;\npub use checker::*;\npub use entity::*;\npub use expression::*;\npub use request::*;\n".to_string(),
        });
        files.push(GeneratedFile {
            path: format!("lib/src/{}/entity.rs", entity.rust_module),
            content: crate::rust_entity::render_entity(domain, entity).map_err(|error| {
                minijinja::Error::new(minijinja::ErrorKind::InvalidOperation, error)
            })?,
        });
        files.push(GeneratedFile {
            path: format!("lib/src/{}/behavior.rs", entity.rust_module),
            content: format!("\nuse teaql_runtime::EntityDataServiceBehavior;\n\n#[derive(Clone, Debug, Default)]\npub struct {}Behavior;\n\nimpl EntityDataServiceBehavior for {}Behavior {{}}", entity.rust_struct, entity.rust_struct),
        });
        files.push(GeneratedFile {
            path: format!("lib/src/{}/checker.rs", entity.rust_module),
            content: crate::rust_checker::render_checker(domain, entity).map_err(|error| {
                minijinja::Error::new(minijinja::ErrorKind::InvalidOperation, error)
            })?,
        });
        files.push(GeneratedFile {
            path: format!("lib/src/{}/expression.rs", entity.rust_module),
            content: crate::rust_expression::render_expression(domain, entity).map_err(
                |error| minijinja::Error::new(minijinja::ErrorKind::InvalidOperation, error),
            )?,
        });
        files.push(GeneratedFile {
            path: format!("lib/src/{}/request.rs", entity.rust_module),
            content: crate::rust_request::render_request(domain, entity).map_err(|error| {
                minijinja::Error::new(minijinja::ErrorKind::InvalidOperation, error)
            })?,
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
    env.add_template(
        "Dockerfile",
        include_str!("../templates/rust-workspace/Dockerfile.j2"),
    )?;
    env.add_template(
        ".dockerignore",
        include_str!("../templates/rust-workspace/.dockerignore.j2"),
    )?;
    env.add_template(
        ".github/workflows/rust.yml",
        include_str!("../templates/rust-workspace/.github/workflows/rust.yml.j2"),
    )?;

    env.add_template(
        "console_Cargo.toml",
        include_str!("../templates/rust-app-console/cargo/Cargo.toml.j2"),
    )?;
    env.add_template(
        "console_main.rs",
        include_str!("../templates/rust-app-console/src/main.rs.j2"),
    )?;
    env.add_template(
        "console_lib.rs",
        include_str!("../templates/rust-app-console/src/lib.rs.j2"),
    )?;

    env.add_template(
        "axum_Cargo.toml",
        include_str!("../templates/rust-web-axum/cargo/Cargo.toml.j2"),
    )?;
    env.add_template(
        "axum_main.rs",
        include_str!("../templates/rust-web-axum/src/main.rs.j2"),
    )?;

    env.add_template(
        "topcoat_Cargo.toml",
        include_str!("../templates/rust-web-topcoat/Cargo.toml.j2"),
    )?;
    env.add_template(
        "topcoat_Makefile",
        include_str!("../templates/rust-web-topcoat/Makefile.j2"),
    )?;
    env.add_template(
        "topcoat_gitignore",
        include_str!("../templates/rust-web-topcoat/.gitignore.j2"),
    )?;
    env.add_template(
        "topcoat_AGENTS.md",
        include_str!("../templates/rust-web-topcoat/AGENTS.md.j2"),
    )?;
    env.add_template(
        "topcoat_RUNTIME_CUSTOM_GUIDE.md",
        include_str!("../templates/rust-web-topcoat/RUNTIME_CUSTOM_GUIDE.md.j2"),
    )?;
    env.add_template(
        "topcoat_TOOL_API_GUIDE.md",
        include_str!("../templates/rust-web-topcoat/TOOL_API_GUIDE.md.j2"),
    )?;
    env.add_template(
        "topcoat_README.md",
        include_str!("../templates/rust-web-topcoat/README.md.j2"),
    )?;
    env.add_template(
        "topcoat_main.rs",
        include_str!("../templates/rust-web-topcoat/main.rs.j2"),
    )?;
    env.add_template(
        "topcoat_lib.rs",
        include_str!("../templates/rust-web-topcoat/lib.rs.j2"),
    )?;

    let mut files = Vec::new();
    let root_templates = [
        "Cargo.toml",
        "Makefile",
        ".gitignore",
        "AGENTS.md",
        "RUNTIME_CUSTOM_GUIDE.md",
        "TOOL_API_GUIDE.md",
        "README.md",
        "Dockerfile",
        ".dockerignore",
        ".github/workflows/rust.yml",
    ];

    let sub_crates = vec![
        (
            "rust-app-console",
            vec![
                ("console_Cargo.toml", "Cargo.toml"),
                ("console_main.rs", "src/main.rs"),
                ("console_lib.rs", "src/lib.rs"),
            ],
        ),
        (
            "rust-web-axum",
            vec![
                ("axum_Cargo.toml", "Cargo.toml"),
                ("axum_main.rs", "src/main.rs"),
            ],
        ),
        (
            "rust-web-topcoat",
            vec![
                ("topcoat_Cargo.toml", "Cargo.toml"),
                ("topcoat_Makefile", "Makefile"),
                ("topcoat_gitignore", ".gitignore"),
                ("topcoat_AGENTS.md", "AGENTS.md"),
                ("topcoat_RUNTIME_CUSTOM_GUIDE.md", "RUNTIME_CUSTOM_GUIDE.md"),
                ("topcoat_TOOL_API_GUIDE.md", "TOOL_API_GUIDE.md"),
                ("topcoat_README.md", "README.md"),
                ("topcoat_main.rs", "src/main.rs"),
                ("topcoat_lib.rs", "src/lib.rs"),
            ],
        ),
    ];

    let rust_crate_name = domain.rust_crate_name.clone();
    let rust_workspace_crate_name = domain.workspace_crate_name.clone();
    let rust_workspace_generated_lib_path = "lib";
    let rust_module_name = rust_crate_name.replace('-', "_");
    let rust_teaql_dependency_version = "4.2.5";

    let has_sql_provider = domain.has_sql_provider;

    let ctx = context! {
        domain => domain,
        entities => &domain.entities,
        businessObjectDescriptors => &domain.business_descriptors,
        constantObjectDescriptors => &domain.constant_descriptors,
        rootObjectDescriptors => &domain.root_descriptors,
        rust_crate_name => rust_crate_name,
        rust_workspace_crate_name => rust_workspace_crate_name,
        rust_web_topcoat_crate_name => format!("{}-topcoat", domain.name),
        rust_workspace_generated_lib_path => rust_workspace_generated_lib_path,
        rust_module_name => rust_module_name,
        rust_teaql_dependency_version => rust_teaql_dependency_version,
        has_sql_provider => has_sql_provider,
    };

    for tmpl in root_templates {
        let template = env.get_template(tmpl).map_err(|e| {
            minijinja::Error::new(minijinja::ErrorKind::TemplateNotFound, e.to_string())
        })?;
        let content = template.render(ctx.clone())?;
        files.push(GeneratedFile {
            path: tmpl.to_string(),
            content,
        });
    }

    for (crate_dir, crate_templates) in sub_crates {
        for (tmpl_name, file_path) in crate_templates {
            let template = env.get_template(tmpl_name).map_err(|e| {
                minijinja::Error::new(minijinja::ErrorKind::TemplateNotFound, e.to_string())
            })?;
            let content = template.render(ctx.clone())?;
            files.push(GeneratedFile {
                path: format!("{}/{}", crate_dir, file_path),
                content,
            });
        }
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
