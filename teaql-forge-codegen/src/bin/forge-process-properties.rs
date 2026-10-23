//! Explicit parity runner; excluded from normal generator builds.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let arguments = teaql_forge_codegen::process_properties::launch_arguments();
    let output = arguments.get(1).ok_or("output file required")?;
    let report = teaql_forge_codegen::process_properties::parity_report(
        arguments.get(2).is_some_and(|arg| arg == "--read-demo"),
    );
    if let Some(parent) = std::path::Path::new(output).parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(output, serde_json::to_string_pretty(&report)? + "\n")?;
    Ok(())
}
