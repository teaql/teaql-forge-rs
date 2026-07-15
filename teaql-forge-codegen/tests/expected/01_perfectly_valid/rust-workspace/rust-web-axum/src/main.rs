use axum::{routing::get, Router};
use serde_json::{json, Value};
use teaql_runtime::UserContext;
use teaql_web_integration_axum::{ContextProvider, WebResponse};
#[derive(Clone)]
struct AppState;
impl ContextProvider for AppState {
    fn build_context(&self) -> UserContext {
        futures::executor::block_on(perfect_service_core::service_runtime_from_env())
            .expect("service runtime")
    }
}
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = perfect_service_core::service_runtime_from_env().await?;
    runtime.ensure_schema().await?;
    let bind_addr = std::env::var("PERFECT_SERVICE_CORE_BIND_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:3000".to_owned());
    let app = Router::new()
        .route("/health", get(health))
        .with_state(AppState);
    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;
    println!("listening on http://{bind_addr}");
    axum::serve(listener, app).await?;
    Ok(())
}
async fn health() -> WebResponse<Value> {
    WebResponse::of_single(json!({
        "status": "ok",
        "service": "perfect-service-core-web-axum"
    }))
}