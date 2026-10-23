use std::error::Error;
use std::fs;
use std::path::Path;
use teaql_forge_codegen::context::build_render_context;
use teaql_forge_codegen::engine::generate_for_scope;
use teaql_forge_codegen::evaluation::render_markdown_report;
use teaql_forge_codegen::input::ModelInput;
use teaql_forge_codegen::semantic;

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = teaql_forge_codegen::process_properties::launch_arguments().into_iter().skip(1);
    let scope = args
        .next()
        .ok_or("Usage: forge-generate SCOPE MODEL_XML OUTPUT_DIR")?;
    let model_path = args.next().ok_or("Missing MODEL_XML")?;
    let output_dir = args.next().ok_or("Missing OUTPUT_DIR")?;
    let model_bytes = fs::read(&model_path)?;
    let model_name = Path::new(&model_path)
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or("MODEL_XML must have a UTF-8 filename")?;
    let input = ModelInput::load(&model_bytes, model_name)?;
    if let Some(error) = input.fatal_parse_failure() {
        return Err(error.into());
    }
    let outcome = input.try_evaluate(&scope).map_err(|error| error.local_uncaught_msv())?;
    fs::create_dir_all(&output_dir)?;
    if outcome.response.blocks_generation() {
        fs::write(
            Path::new(&output_dir).join("teaql-evaluation-report.md"),
            render_markdown_report(&outcome.response),
        )?;
        return Err(format!("KSML evaluation failed; report written to {output_dir}").into());
    }
    let domain = build_render_context(
        &outcome
            .domain
            .ok_or("Evaluation completed without a parsed domain")?,
    );
    let mut files = generate_for_scope(&domain, &scope)?;
    if let Some(document) = input.semantic_document()? {
        files.push(teaql_forge_codegen::engine::GeneratedFile {
            path: semantic::OUTPUT_PATH.into(),
            content: semantic::write_string(&document),
        });
    }
    for file in &files {
        let path = Path::new(&output_dir).join(&file.path);
        fs::create_dir_all(path.parent().ok_or("Generated path has no parent")?)?;
        fs::write(path, file.content.as_bytes())?;
    }
    println!("Generated {} files for {scope}", files.len());
    Ok(())
}
