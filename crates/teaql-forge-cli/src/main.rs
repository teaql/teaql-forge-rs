use clap::{Parser, Subcommand};
use miette::{IntoDiagnostic, Result, WrapErr};
use std::fs;
use std::path::Path;
use teaql_forge_model::parser::parse_model;
use teaql_forge_codegen::context::build_render_context;
use teaql_forge_codegen::engine::generate_crate;

#[derive(Parser)]
#[command(name = "teaql-forge")]
#[command(about = "TeaQL Rust domain crate generator", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Check if the model is valid
    Check {
        /// The model XML file path
        model_file: String,
    },
    /// Inspect the parsed model IR
    Inspect {
        /// The model XML file path
        model_file: String,
    },
    /// Generate Rust crate
    Generate {
        /// The model XML file path
        model_file: String,
        /// Output directory
        #[arg(short, long, default_value = "./generated")]
        output: String,
        /// Run cargo check after generation
        #[arg(long)]
        cargo_check: bool,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Check { model_file } => {
            let src = fs::read_to_string(model_file)
                .into_diagnostic()
                .wrap_err_with(|| format!("Failed to read {}", model_file))?;
            let _domain = parse_model(&src)?;
            println!("Model is valid.");
        }
        Commands::Inspect { model_file } => {
            let src = fs::read_to_string(model_file)
                .into_diagnostic()
                .wrap_err_with(|| format!("Failed to read {}", model_file))?;
            let domain = parse_model(&src)?;
            let json = serde_json::to_string_pretty(&domain).into_diagnostic()?;
            println!("{}", json);
        }
        Commands::Generate {
            model_file,
            output,
            cargo_check,
        } => {
            let src = fs::read_to_string(model_file)
                .into_diagnostic()
                .wrap_err_with(|| format!("Failed to read {}", model_file))?;
            let domain = parse_model(&src)?;
            let render_context = build_render_context(&domain);
            let output_path = Path::new(output);
            
            generate_crate(&render_context, output_path)
                .into_diagnostic()
                .wrap_err_with(|| format!("Failed to generate crate to {:?}", output_path))?;
                
            println!("Successfully generated crate to {:?}", output_path);
            
            if *cargo_check {
                let status = std::process::Command::new("cargo")
                    .arg("check")
                    .current_dir(output_path)
                    .status()
                    .into_diagnostic()?;
                if !status.success() {
                    miette::bail!("cargo check failed");
                }
            }
        }
    }

    Ok(())
}
