use crate::models::{ClaudeStreamApiRequest, StreamEvent};
use axum::extract::ws::Message;
use futures_util::{Stream, StreamExt};
use reqwest::header::{HeaderMap, HeaderValue};
use std::error::Error;
use tokio_stream::wrappers::ReceiverStream;
use tracing::{error, info};

/// Streams AI responses via WebSockets.
pub async fn stream_ai_response(
    api_key: String,
    request: ClaudeStreamApiRequest,
) -> impl Stream<Item = Result<Message, Box<dyn Error + Send + Sync>>> {
    info!("Processing request for model: {}", request.model);

    let mut headers = HeaderMap::new();
    headers.insert("x-api-key", HeaderValue::from_str(&api_key).unwrap());
    headers.insert("anthropic-version", HeaderValue::from_static("2023-06-01"));

    let client = reqwest::Client::builder()
        .default_headers(headers)
        .build()
        .unwrap();

    let response = match client
        .post("https://api.anthropic.com/v1/messages")
        .json(&request)
        .send()
        .await
    {
        Ok(resp) => {
            info!("Connected to Claude API successfully");
            resp
        }
        Err(e) => {
            error!("Claude API connection failed: {}", e);
            return tokio_stream::once(async { Err(Box::new(e) as Box<dyn Error + Send + Sync>) });
        }
    };

    response.bytes_stream().map(move |chunk| match chunk {
        Ok(chunk) => {
            let text = match String::from_utf8(chunk.to_vec()) {
                Ok(s) => s,
                Err(e) => {
                    error!("Invalid UTF-8 in AI response: {}", e);
                    return Err(Box::new(e) as Box<dyn Error + Send + Sync>);
                }
            };

            if let Some(event) = parse_sse_line(&text) {
                match event {
                    StreamEvent::ContentBlockDelta { delta, .. } => Ok(Message::Text(delta.text)),
                    StreamEvent::MessageDelta { delta, .. } => {
                        if let Some(reason) = delta.stop_reason {
                            info!("Stream completed. Stop reason: {}", reason);
                            Ok(Message::Text(format!("\nStop reason: {}", reason)))
                        } else {
                            Ok(Message::Text("".to_string()))
                        }
                    }
                }
            } else {
                Ok(Message::Text("".to_string()))
            }
        }
        Err(e) => {
            error!("Error processing stream chunk: {}", e);
            Err(Box::new(e) as Box<dyn Error + Send + Sync>)
        }
    })
}

/// Parses SSE lines into `StreamEvent`
fn parse_sse_line(line: &str) -> Option<StreamEvent> {
    if line.starts_with("data: ") {
        let json = &line["data: ".len()..];
        serde_json::from_str(json).ok()
    } else {
        None
    }
}
