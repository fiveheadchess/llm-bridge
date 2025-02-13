use axum::{routing::post, Json, Router};
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;
use tracing::{error, info};
use tracing_subscriber;

mod modules;

#[derive(Deserialize)]
struct LLMRequest {
    prompt: String,
}

#[derive(Serialize)]
struct LLMResponse {
    response: String,
}

async fn handle_request(Json(payload): Json<LLMRequest>) -> Json<LLMResponse> {
    info!("received requestL {:?}", payload.prompt);
    // placeholder for calling LLM API
    let llm_output = format!("echo: {}", payload.prompt);
    Json(LLMResponse {
        response: llm_output,
    })
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let app = Router::new().route("/ai", post(handle_request));
    let listener = TcpListener::bind("0.0.0.0:8080").await.unwrap();
    info!("server running on http://0.0.0.0:8080");
    axum::serve(listener, app).await.unwrap();
}
