use crate::models::{ClaudeStreamApiRequest, StreamEvent};
use axum::extract::ws::Message;
use futures_util::stream::BoxStream;
use futures_util::StreamExt;
use reqwest::header::{HeaderMap, HeaderValue};
use std::error::Error;
use tracing::{error, info};

type StreamResult = Result<Message, Box<dyn Error + Send + Sync>>;

/// Streams AI responses via WebSockets.
pub async fn stream_ai_response(
    api_key: String,
    request: ClaudeStreamApiRequest,
) -> BoxStream<'static, StreamResult> {
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
            let error_stream = futures_util::stream::once(async move {
                Err(Box::new(e) as Box<dyn Error + Send + Sync>)
            });
            return Box::pin(error_stream);
        }
    };

    let stream = response.bytes_stream().map(move |chunk| match chunk {
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
    });

    Box::pin(stream)
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
