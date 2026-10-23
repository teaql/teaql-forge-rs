use std::error::Error;
use std::fs;
use std::path::Path;
use teaql_forge_codegen::context::build_render_context;
use teaql_forge_codegen::engine::render_preview;
use teaql_forge_codegen::input::ModelInput;

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = teaql_forge_codegen::process_properties::launch_arguments()
        .into_iter()
        .skip(1);
    let preview_path = args
        .next()
        .ok_or("Usage: forge-preview PREVIEW_PATH MODEL_XML OUTPUT_FILE")?;
    let model_path = args.next().ok_or("Missing MODEL_XML")?;
    let output_file = args.next().ok_or("Missing OUTPUT_FILE")?;
    if args.next().is_some() {
        return Err("Too many arguments".into());
    }
    if Path::new(&output_file).exists() {
        return Err(format!("Refusing to replace existing output: {output_file}").into());
    }

    let model_bytes = fs::read(&model_path)?;
    let model_name = Path::new(&model_path)
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or("MODEL_XML must have a UTF-8 filename")?;
    let input = ModelInput::load(&model_bytes, model_name)?;
    let mixed_case_assist = !preview_path.contains("-assist-")
        && preview_path.to_lowercase().contains("-assist-");
    if mixed_case_assist {
        teaql_forge_codegen::templates::validate_preview_path(&preview_path)?;
    }
    let domain = build_render_context(&input.parse_preview_domain().map_err(|message| {
        input.preview_factory_failure()
            .and_then(|error| error.local_preview_msv())
            .unwrap_or(message)
    })?);
    if teaql_forge_codegen::engine::assist_query_null_entity_path(&preview_path) {
        return Err(teaql_forge_codegen::engine::ASSIST_QUERY_NULL_ENTITY_MESSAGE.into());
    }
    let canonical_path = preview_path.strip_prefix("doc/").unwrap_or(&preview_path);
    let doc_template = (preview_path.starts_with("doc/") || !preview_path.contains('/'))
        .then(|| canonical_path.rsplit('/').next().unwrap())
        .and_then(teaql_forge_codegen::templates::document_preview_template);
    let (template_name, target_entity) = match doc_template {
        Some("model-view") => ("doc/model-view", None),
        Some("data-design") => ("doc/data-design", None),
        Some("data-design-react") => ("doc/data-design-react", None),
        _ if teaql_forge_codegen::preview::has_builtin(&preview_path) => {
            (preview_path.as_str(), None)
        }
        _ => match preview_path.split_once('/') {
            Some((template_name, entity)) if template_name.contains("-assist-") => {
                (template_name, entity.split('/').next().map(str::to_string))
            }
            Some((template_name, entity)) => (template_name, Some(entity.to_string())),
            None => (preview_path.as_str(), None),
        },
    };
    let content = if let Some(source) = teaql_forge_codegen::templates::read(&preview_path) {
        if mixed_case_assist && source.is_empty() {
            missing_markdown_preview(&preview_path)
        } else {
            teaql_forge_codegen::engine::render_named_external_preview(&domain, &preview_path, &source)?
        }
    } else if mixed_case_assist {
        missing_markdown_preview(&preview_path)
    } else {
        match render_preview(&domain, template_name, target_entity) {
            Ok(content) => content,
            Err(error)
                if preview_path.starts_with("doc/") && error.starts_with("template not found:") =>
            {
                let filename = preview_path.rsplit('/').next().unwrap();
                format!("## ❌ Error\n\nTemplate `{filename}.stg` not found in `doc`.\n")
            }
            Err(error) => return Err(error.into()),
        }
    };
    if let Some(parent) = Path::new(&output_file).parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(output_file, content.as_bytes())?;
    Ok(())
}

fn missing_markdown_preview(path: &str) -> String {
    let scope = path.split('/').next().unwrap_or("");
    let filename = path.rsplit('/').next().unwrap_or("");
    format!("## ❌ Error\n\nTemplate `{filename}.stg` not found in `{scope}`.\n")
}
