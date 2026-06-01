use crate::context::RenderDomain;
use minijinja::Environment;
use std::fs;
use std::path::Path;

pub fn generate_crate(domain: &RenderDomain, output_dir: &Path) -> std::io::Result<()> {
    let mut env = Environment::new();
    
    // Add templates
    env.add_template("Cargo.toml", include_str!("../templates/Cargo.toml.j2")).unwrap();
    env.add_template("lib.rs", include_str!("../templates/src/lib.rs.j2")).unwrap();
    env.add_template("entities_mod.rs", include_str!("../templates/src/entities/mod.rs.j2")).unwrap();
    env.add_template("entity.rs", include_str!("../templates/src/entities/entity.rs.j2")).unwrap();
    env.add_template("q.rs", include_str!("../templates/src/q.rs.j2")).unwrap();
    env.add_template("runtime.rs", include_str!("../templates/src/runtime.rs.j2")).unwrap();

    fs::create_dir_all(output_dir.join("src").join("entities"))?;

    // Render Cargo.toml
    let cargo_toml = env.get_template("Cargo.toml").unwrap().render(domain).unwrap();
    fs::write(output_dir.join("Cargo.toml"), cargo_toml)?;

    // Render lib.rs
    let lib_rs = env.get_template("lib.rs").unwrap().render(domain).unwrap();
    fs::write(output_dir.join("src").join("lib.rs"), lib_rs)?;

    // Render entities/mod.rs
    let entities_mod = env.get_template("entities_mod.rs").unwrap().render(domain).unwrap();
    fs::write(output_dir.join("src").join("entities").join("mod.rs"), entities_mod)?;

    // Render q.rs
    let q_rs = env.get_template("q.rs").unwrap().render(domain).unwrap();
    fs::write(output_dir.join("src").join("q.rs"), q_rs)?;

    // Render runtime.rs
    let runtime_rs = env.get_template("runtime.rs").unwrap().render(domain).unwrap();
    fs::write(output_dir.join("src").join("runtime.rs"), runtime_rs)?;

    // Render each entity
    for entity in &domain.entities {
        let entity_code = env.get_template("entity.rs").unwrap().render(minijinja::context! {
            entity => entity,
        }).unwrap();
        
        fs::write(output_dir.join("src").join("entities").join(format!("{}.rs", entity.rust_module)), entity_code)?;
    }

    Ok(())
}
