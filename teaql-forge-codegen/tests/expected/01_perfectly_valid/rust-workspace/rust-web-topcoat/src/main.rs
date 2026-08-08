#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let bootstrap_context = perfect_service_core::service_runtime_from_env().await?;
    bootstrap_context.ensure_schema().await?;
    let bind_addr = std::env::var("_BIND_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:3000".to_owned());
    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;
    println!("listening on http://{bind_addr}");
    topcoat::serve(listener, perfect_service_core_topcoat::router()).await?;
    Ok(())
}