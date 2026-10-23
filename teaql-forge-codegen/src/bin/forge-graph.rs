//! Deterministic transport probe; never included in production feature builds.
use std::{fs, path::Path};
use teaql_forge_codegen::{
    context::build_render_context, engine, graph_service, input::ModelInput,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = teaql_forge_codegen::process_properties::launch_arguments();
    let mode = args.get(1).ok_or("mode required")?;
    let fixture = args.get(2).ok_or("fixture or - required")?;
    let input = Path::new(args.get(3).ok_or("input required")?);
    let output = Path::new(args.get(4).ok_or("output required")?);
    let parameter = args.get(5).map(String::as_str).unwrap_or("");
    if mode == "mtls" {
        fs::write(
            output,
            serde_json::to_vec_pretty(&graph_service::mutual_tls_probe(
                Path::new(fixture),
                input,
                output,
            ))?,
        )?;
        return Ok(());
    }
    let action = || -> Result<(), Box<dyn std::error::Error>> {
        if mode == "keys" {
            fs::write(
                output,
                serde_json::to_vec_pretty(&graph_service::key_probe())?,
            )?;
            return Ok(());
        }
        if mode == "trust" {
            fs::write(
                output,
                serde_json::to_vec_pretty(&graph_service::trust_probe())?,
            )?;
            return Ok(());
        }
        if mode == "proxy" {
            let cases = serde_json::from_slice(&fs::read(input)?)?;
            fs::write(
                output,
                serde_json::to_vec_pretty(&graph_service::proxy_probe(&cases))?,
            )?;
            return Ok(());
        }
        if mode == "client-outcome" {
            fs::write(
                output,
                serde_json::to_vec_pretty(&graph_service::client_outcome(fs::read_to_string(
                    input,
                )?))?,
            )?;
            return Ok(());
        }
        if mode == "client" {
            fs::write(
                output,
                graph_service::client_probe(fs::read_to_string(input)?),
            )?;
            return Ok(());
        }
        let loaded = ModelInput::load(
            &fs::read(input)?,
            input.file_name().unwrap().to_str().unwrap(),
        )?;
        if mode == "preview" {
            let domain = build_render_context(&loaded.parse_preview_domain()?);
            let name = match parameter {
                "doc/data-design.html" => "doc/data-design",
                value => value,
            };
            fs::write(output, engine::render_preview(&domain, name, None)?)?;
        } else {
            for scope in parameter.split(',') {
                let outcome = loaded.try_evaluate(scope)?;
                assert!(!outcome.response.blocks_generation(), "{scope}");
                let domain = build_render_context(&outcome.domain.ok_or("no domain")?);
                for file in engine::generate_for_scope(&domain, scope)? {
                    let destination = output.join(scope).join(file.path);
                    fs::create_dir_all(destination.parent().unwrap())?;
                    fs::write(destination, file.content)?;
                }
            }
        }
        Ok(())
    };
    if fixture == "-" {
        if args.get(6).is_some_and(|value| value == "--observe") {
            let (result, observations) = graph_service::observe(action);
            fs::write(
                output.with_extension("exchange.json"),
                serde_json::to_vec_pretty(&observations)?,
            )?;
            result?;
        } else {
            action()?;
        }
    } else {
        let (result, requests) =
            graph_service::with_response(serde_json::from_slice(&fs::read(fixture)?)?, action);
        fs::write(
            output.with_extension("requests.json"),
            serde_json::to_vec_pretty(&requests)?,
        )?;
        result?;
    }
    Ok(())
}
