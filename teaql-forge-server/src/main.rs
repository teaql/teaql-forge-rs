use axum::{
    extract::Multipart,
    http::{header, HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use std::io::Write;
use std::net::SocketAddr;
use std::path::PathBuf;
use teaql_forge_codegen::context::{build_render_context, RenderDomain};
use teaql_forge_codegen::engine::generate_virtual_crate;
use teaql_forge_model::parser::parse_model;
use zip::write::SimpleFileOptions;

use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(long, default_value = "127.0.0.1")]
    host: String,

    #[arg(short, long, default_value_t = 8081)]
    port: u16,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    let app = Router::new()
        .route("/version", get(version_handler))
        .route("/generate", post(generate_handler));

    if args.host == "0.0.0.0" {
        println!("Warning: You are exposing TeaQL Local Server to the network.");
        println!("Use Enterprise Mode or configure TLS/auth for production.");
    }

    let addr = format!("{}:{}", args.host, args.port);
    println!("Listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn version_handler() -> impl IntoResponse {
    let mut map = serde_json::Map::new();
    map.insert(
        "version".to_string(),
        serde_json::Value::String("1.0.0".to_string()),
    );
    map.insert(
        "name".to_string(),
        serde_json::Value::String("teaql-code-generator".to_string()),
    );
    Json(map)
}

async fn generate_handler(mut multipart: Multipart) -> impl IntoResponse {
    let mut file_content = None;
    let mut scope = "rust-lib".to_string();

    while let Some(field) = multipart.next_field().await.unwrap_or(None) {
        let name = field.name().unwrap_or("").to_string();
        if name == "file" {
            let data = field.bytes().await.unwrap();
            file_content = Some(String::from_utf8_lossy(&data).to_string());
        } else if name == "scope" {
            let data = field.bytes().await.unwrap();
            scope = String::from_utf8_lossy(&data).to_string();
        }
    }

    let xml = match file_content {
        Some(c) => c,
        None => return (StatusCode::BAD_REQUEST, "Missing file part").into_response(),
    };

    if scope != "rust-lib" && scope != "rust_lib" && scope != "rust-workspace" {
        println!("Warning: unsupported scope {}, assuming rust-lib", scope);
    }

    let domain = match parse_model(&xml) {
        Ok(d) => d,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("Parse error: {}", e)).into_response(),
    };

    let render_domain = build_render_context(&domain);
    let files = if scope == "rust-workspace" {
        match teaql_forge_codegen::engine::generate_virtual_workspace(&render_domain) {
            Ok(f) => f,
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("CodeGen error: {}", e)).into_response(),
        }
    } else {
        match generate_virtual_crate(&render_domain) {
            Ok(f) => f,
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("CodeGen error: {}", e)).into_response(),
        }
    };

    let mut zip_buffer = Vec::new();
    {
        let mut zip = zip::ZipWriter::new(std::io::Cursor::new(&mut zip_buffer));
        let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

        for generated_file in files {
            // Need to add directories if missing, but zip crate might handle it or we can just add files
            // For safety, we can just add the file and zip viewers usually figure it out, 
            // but let's add files properly.
            if let Err(e) = zip.start_file(&generated_file.path, options) {
                return (StatusCode::INTERNAL_SERVER_ERROR, format!("Zip error: {}", e)).into_response();
            }
            if let Err(e) = zip.write_all(generated_file.content.as_bytes()) {
                return (StatusCode::INTERNAL_SERVER_ERROR, format!("Zip write error: {}", e)).into_response();
            }
        }

        if let Err(e) = zip.finish() {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("Zip finish error: {}", e)).into_response();
        }
    }

    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        "application/octet-stream".parse().unwrap(),
    );
    headers.insert(
        header::CONTENT_DISPOSITION,
        "attachment; filename=\"domain.zip\"".parse().unwrap(),
    );

    (headers, zip_buffer).into_response()
}
