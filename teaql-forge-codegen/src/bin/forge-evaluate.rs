use std::{error::Error, fs, path::Path};
use teaql_forge_codegen::{evaluation::render_markdown_report, input::ModelInput};

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = teaql_forge_codegen::process_properties::launch_arguments().into_iter().skip(1);
    let model = args
        .next()
        .ok_or("Usage: forge-evaluate MODEL_INPUT OUTPUT_FILE [SCOPE]")?;
    let output = args.next().ok_or("Missing OUTPUT_FILE")?;
    let scope = args.next().unwrap_or_else(|| "evaluate".into());
    if args.next().is_some() {
        return Err("Too many arguments".into());
    }
    if Path::new(&output).exists() {
        return Err(format!("Refusing to replace existing output: {output}").into());
    }
    let filename = Path::new(&model)
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or("MODEL_INPUT must have a UTF-8 filename")?;
    let input = ModelInput::load(&fs::read(&model)?, filename)?;
    if let Some(error) = input.fatal_parse_failure() {
        return Err(error.into());
    }
    let outcome = input.try_evaluate(&scope).map_err(|error| error.local_uncaught_msv())?;
    if let Some(parent) = Path::new(&output).parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(
        output,
        render_markdown_report(&outcome.response),
    )?;
    Ok(())
}
