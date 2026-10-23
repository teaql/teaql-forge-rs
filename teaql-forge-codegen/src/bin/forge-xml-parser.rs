//! Explicit source-parity probe for the two XML parsing paths.
use serde_json::json;
use std::{collections::BTreeMap, error::Error, fs, path::Path};
use teaql_forge_codegen::{evaluation::render_markdown_report, input::ModelInput};

fn main() -> Result<(), Box<dyn Error>> {
    let args = teaql_forge_codegen::process_properties::launch_arguments();
    let input = Path::new(args.get(1).ok_or("input directory required")?);
    let output = Path::new(args.get(2).ok_or("output directory required")?);
    fs::create_dir_all(output)?;
    let mut all = BTreeMap::new();
    for entry in fs::read_dir(input)? {
        let file = entry?.path();
        if !matches!(file.extension().and_then(|ext| ext.to_str()), Some("xml" | "zip")) {
            continue;
        }
        let name = file
            .file_name()
            .unwrap()
            .to_str()
            .ok_or("invalid filename")?;
        let bytes = fs::read(&file)?;
        let loaded = ModelInput::load(&bytes, name)?;
        let fields_only = args.get(3).is_some_and(|arg| arg == "--field-attributes");
        let mut field_attributes = serde_json::Value::Null;
        let mut generation = BTreeMap::new();
        let domain = match loaded.parse_preview_domain() {
            Ok(domain) => {
                if fields_only {
                    field_attributes = json!({"platform.name": domain.source_field_attributes.get("platform").and_then(|fields| fields.get("name"))});
                }
                if let Some(scopes) = args.get(3).filter(|_| !fields_only) {
                    for scope in scopes.split(',') {
                        let model = loaded.parse_preview_domain()?;
                        let render = teaql_forge_codegen::context::build_render_context(&model);
                        match teaql_forge_codegen::engine::generate_for_scope(&render, scope) {
                            Ok(files) => {
                                for file in &files {
                                    let target = output
                                        .join("generation")
                                        .join(name)
                                        .join(scope)
                                        .join(&file.path);
                                    fs::create_dir_all(target.parent().unwrap())?;
                                    fs::write(target, file.content.as_bytes())?;
                                }
                                generation.insert(scope.to_owned(), json!({"files":files.len()}));
                            }
                            Err(message) => {
                                generation.insert(
                                    scope.to_owned(),
                                    json!({"message":message.to_string()}),
                                );
                            }
                        }
                    }
                }
                if let Some(previews) = args.get(4) {
                    for preview in previews.split(',') {
                        let model = loaded.parse_preview_domain()?;
                        let render = teaql_forge_codegen::context::build_render_context(&model);
                        let text = if let Some(source) =
                            teaql_forge_codegen::templates::read(preview)
                        {
                            teaql_forge_codegen::engine::render_external_preview(&render, &source)?
                        } else {
                            teaql_forge_codegen::engine::render_preview(&render, preview, None)?
                        };
                        let target = output
                            .join("previews")
                            .join(name)
                            .join(preview)
                            .join("body.txt");
                        fs::create_dir_all(target.parent().unwrap())?;
                        fs::write(target, text)?;
                    }
                }
                json!({"ok":true,"name":domain.name,"objects":domain.entities.iter().map(|entity|&entity.name).collect::<Vec<_>>()})
            }
            Err(message) => {
                if fields_only { field_attributes = json!({"error":{"message":message}}); }
                match loaded.preview_factory_failure() {
                Some(error) => json!({"ok":false,"error":error}),
                None => json!({"ok":false,"error":{"message":message}}),
                }
            },
        };
        let mut result = match loaded.try_evaluate("evaluate") {
            Ok(outcome) => {
                fs::write(output.join(format!("{name}.md")), render_markdown_report(&outcome.response))?;
                json!({"domain":domain,"evaluation":outcome.response,"generation":generation})
            },
            Err(error) => json!({"domain":domain,"evaluation_failure":error,"generation":generation}),
        };
        if fields_only { result["field_attributes"] = field_attributes; }
        all.insert(name.to_owned(), result);
    }
    fs::write(
        output.join("actual-xml.json"),
        serde_json::to_vec_pretty(&all)?,
    )?;
    Ok(())
}
