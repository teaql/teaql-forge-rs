use crate::context::RenderDomain;
use minijinja::Environment;
use std::fs;
use std::path::Path;

pub struct GeneratedFile {
    pub path: String,
    pub content: String,
}

pub fn generate_virtual_crate(domain: &RenderDomain) -> Result<Vec<GeneratedFile>, minijinja::Error> {
    let mut env = Environment::new();
    env.add_template("Cargo.toml", include_str!("../templates/Cargo.toml.j2"))?;
    env.add_template("lib.rs", include_str!("../templates/src/lib.rs.j2"))?;
    env.add_template("entities_mod.rs", include_str!("../templates/src/entities/mod.rs.j2"))?;
    env.add_template("entity.rs", include_str!("../templates/src/entities/entity.rs.j2"))?;
    env.add_template("q.rs", include_str!("../templates/src/q.rs.j2"))?;
    env.add_template("runtime.rs", include_str!("../templates/src/runtime.rs.j2"))?;

    let mut files = Vec::new();

    files.push(GeneratedFile {
        path: "Cargo.toml".to_string(),
        content: env.get_template("Cargo.toml")?.render(domain)?,
    });
    files.push(GeneratedFile {
        path: "src/lib.rs".to_string(),
        content: env.get_template("lib.rs")?.render(domain)?,
    });
    files.push(GeneratedFile {
        path: "src/entities/mod.rs".to_string(),
        content: env.get_template("entities_mod.rs")?.render(domain)?,
    });
    files.push(GeneratedFile {
        path: "src/q.rs".to_string(),
        content: env.get_template("q.rs")?.render(domain)?,
    });
    files.push(GeneratedFile {
        path: "src/runtime.rs".to_string(),
        content: env.get_template("runtime.rs")?.render(domain)?,
    });

    for entity in &domain.entities {
        files.push(GeneratedFile {
            path: format!("src/entities/{}.rs", entity.rust_module),
            content: env.get_template("entity.rs")?.render(minijinja::context! {
                entity => entity,
            })?,
        });
    }

    Ok(files)
}

pub fn generate_crate(domain: &RenderDomain, output_dir: &Path) -> std::io::Result<()> {
    let files = generate_virtual_crate(domain).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
    
    for file in files {
        let full_path = output_dir.join(&file.path);
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(full_path, file.content)?;
    }

    Ok(())
}
