use axum::{
    extract::State,
    response::sse::{Event, Sse},
    routing::post,
    Json, Router,
};
use futures::stream::Stream;
use futures_util::StreamExt;
use reqwest::header::{HeaderMap, HeaderValue};
use std::{error::Error as StdError, sync::Arc};
use tokio::sync::Semaphore;
use tracing::info;

mod models;
use models::{parse_sse_line, ClaudeStreamApiRequest, StreamEvent};

type BoxError = Box<dyn StdError + Send + Sync + 'static>;

async fn stream_ai_response(
    api_key: String,
    request: ClaudeStreamApiRequest,
) -> impl Stream<Item = Result<Event, BoxError>> {
    let mut headers = HeaderMap::new();
    headers.insert("x-api-key", HeaderValue::from_str(&api_key).unwrap());
    headers.insert("anthropic-version", HeaderValue::from_static("2023-06-01"));

    let client = reqwest::Client::builder()
        .default_headers(headers)
        .build()
        .unwrap();

    let response = client
        .post("https://api.anthropic.com/v1/messages")
        .json(&request)
        .send()
        .await
        .expect("Failed to send request");

    response.bytes_stream().map(move |chunk| {
        let chunk = chunk.map_err(|e| Box::new(e) as BoxError)?;
        let text = String::from_utf8_lossy(&chunk);

        if let Some(event) = parse_sse_line(&text) {
            match event {
                StreamEvent::ContentBlockDelta { delta, .. } => {
                    Ok(Event::default().data(delta.text))
                }
                StreamEvent::MessageDelta { delta, .. } => {
                    if let Some(reason) = delta.stop_reason {
                        Ok(Event::default().data(format!("\nStop reason: {}", reason)))
                    } else {
                        Ok(Event::default().data("".to_string()))
                    }
                }
                _ => Ok(Event::default().data("".to_string())),
            }
        } else {
            Ok(Event::default().data("".to_string()))
        }
    })
}

async fn handle_request(
    Json(request): Json<ClaudeStreamApiRequest>,
) -> Sse<impl Stream<Item = Result<Event, BoxError>>> {
    let api_key = std::env::var("ANTHROPIC_API_KEY").expect("ANTHROPIC_API_KEY must be set");
    let stream = stream_ai_response(api_key, request).await;
    Sse::new(stream)
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let app = Router::new().route("/ai", post(handle_request));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await.unwrap();
    info!("server running on 8080");

    axum::serve(listener, app).await.unwrap();
}
