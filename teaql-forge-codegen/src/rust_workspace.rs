use crate::context::RenderDomain;
use crate::engine::GeneratedFile;
use crate::java_stg::StgTemplate;
use serde_json::{Map, Value};

const CONSOLE: [(&str, &str); 9] = [
    (
        "Cargo.toml",
        include_str!("../templates/rust-app-console-source/Cargo.toml.stg"),
    ),
    (
        "README.md",
        include_str!("../templates/rust-app-console-source/README.md.stg"),
    ),
    (
        ".gitignore",
        include_str!("../templates/rust-app-console-source/gitignore.stg"),
    ),
    (
        "Makefile",
        include_str!("../templates/rust-app-console-source/Makefile.stg"),
    ),
    (
        "AGENTS.md",
        include_str!("../templates/rust-app-console-source/AGENTS.md.stg"),
    ),
    (
        "RUNTIME_CUSTOM_GUIDE.md",
        include_str!("../templates/rust-app-console-source/RUNTIME_CUSTOM_GUIDE.md.stg"),
    ),
    (
        "TOOL_API_GUIDE.md",
        include_str!("../templates/rust-app-console-source/TOOL_API_GUIDE.md.stg"),
    ),
    (
        "src/main.rs",
        include_str!("../templates/rust-app-console-source/main.rs.stg"),
    ),
    (
        "src/lib.rs",
        include_str!("../templates/rust-app-console-source/lib.rs.stg"),
    ),
];

const AXUM: [(&str, &str); 9] = [
    (
        "Cargo.toml",
        include_str!("../templates/rust-web-axum-source/Cargo.toml.stg"),
    ),
    (
        "README.md",
        include_str!("../templates/rust-web-axum-source/README.md.stg"),
    ),
    (
        ".gitignore",
        include_str!("../templates/rust-web-axum-source/gitignore.stg"),
    ),
    (
        "Makefile",
        include_str!("../templates/rust-web-axum-source/Makefile.stg"),
    ),
    (
        "AGENTS.md",
        include_str!("../templates/rust-web-axum-source/AGENTS.md.stg"),
    ),
    (
        "RUNTIME_CUSTOM_GUIDE.md",
        include_str!("../templates/rust-web-axum-source/RUNTIME_CUSTOM_GUIDE.md.stg"),
    ),
    (
        "TOOL_API_GUIDE.md",
        include_str!("../templates/rust-web-axum-source/TOOL_API_GUIDE.md.stg"),
    ),
    (
        "src/main.rs",
        include_str!("../templates/rust-web-axum-source/main.rs.stg"),
    ),
    (
        "src/lib.rs",
        include_str!("../templates/rust-web-axum-source/lib.rs.stg"),
    ),
];

const TOPCOAT: [(&str, &str); 9] = [
    (
        "Cargo.toml",
        include_str!("../templates/rust-web-topcoat-source/Cargo.toml.stg"),
    ),
    (
        "README.md",
        include_str!("../templates/rust-web-topcoat-source/README.md.stg"),
    ),
    (
        ".gitignore",
        include_str!("../templates/rust-web-topcoat-source/gitignore.stg"),
    ),
    (
        "Makefile",
        include_str!("../templates/rust-web-topcoat-source/Makefile.stg"),
    ),
    (
        "AGENTS.md",
        include_str!("../templates/rust-web-topcoat-source/AGENTS.md.stg"),
    ),
    (
        "RUNTIME_CUSTOM_GUIDE.md",
        include_str!("../templates/rust-web-topcoat-source/RUNTIME_CUSTOM_GUIDE.md.stg"),
    ),
    (
        "TOOL_API_GUIDE.md",
        include_str!("../templates/rust-web-topcoat-source/TOOL_API_GUIDE.md.stg"),
    ),
    (
        "src/main.rs",
        include_str!("../templates/rust-web-topcoat-source/main.rs.stg"),
    ),
    (
        "src/lib.rs",
        include_str!("../templates/rust-web-topcoat-source/lib.rs.stg"),
    ),
];

pub(crate) fn generate(domain: &RenderDomain, scope: &str) -> Result<Vec<GeneratedFile>, String> {
    let templates = match scope {
        "rust-app-console" => &CONSOLE,
        "rust-web-axum" => &AXUM,
        "rust-web-topcoat" => &TOPCOAT,
        _ => return Err(format!("unsupported Rust workspace scope {scope}")),
    };
    let mut value = crate::rust_globals::domain_value(domain)?;
    let item = value.as_object_mut().ok_or("domain is not an object")?;
    let crate_name = &domain.rust_crate_name;
    item.insert("rustStructName".into(), Value::String(String::new()));
    item.insert(
        "rustModuleName".into(),
        Value::String(crate_name.replace('-', "_")),
    );
    item.insert(
        "rustWorkspaceCrateName".into(),
        Value::String(domain.rust_workspace_crate_name.clone()),
    );
    item.insert(
        "rustWebAxumCrateName".into(),
        Value::String(format!("{crate_name}-web-axum")),
    );
    let topcoat = format!("{}-web-topcoat", crate::preview_rust::symbol(&domain.name, '-').replace('_', "-"));
    item.insert(
        "rustWebTopcoatCrateName".into(),
        Value::String(topcoat.clone()),
    );
    item.insert(
        "rustWebTopcoatModuleName".into(),
        Value::String(topcoat.replace('-', "_")),
    );
    let mut files = Vec::new();
    for &(path, source) in templates {
        let mut vars = Map::new();
        vars.insert("domain".into(), value.clone());
        let rendered = StgTemplate::parse(source)?.render_template("index", vars)?;
        let rendered = if path == "src/main.rs" {
            rendered
                .replace("\n\n    Ok(())", "\n    Ok(())")
                .replace("\n\n    let bind_addr", "\n    let bind_addr")
        } else if scope == "rust-web-axum"
            && path == "Cargo.toml"
            && domain.data_service.name != "linux"
        {
            format!("{rendered}\n")
        } else {
            rendered
        };
        files.push(GeneratedFile {
            path: path.into(),
            content: rendered,
        });
    }
    files.push(GeneratedFile {
        path: "teaql-i18n.json".into(),
        content: crate::i18n::i18n_catalog(domain),
    });
    let library =
        crate::engine::generate_virtual_crate(domain).map_err(|error| error.to_string())?;
    files.extend(library);
    Ok(files)
}
