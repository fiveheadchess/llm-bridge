use crate::models::{ClaudeStreamApiRequest, StreamEvent};
use futures_util::StreamExt;
use reqwest::header::{HeaderMap, HeaderValue};
use std::error::Error;
use tokio::io::AsyncWriteExt;

pub async fn stream_ai_response(
    api_key: String,
    request: ClaudeStreamApiRequest,
    writer: &mut (impl AsyncWriteExt + Unpin),
) -> Result<(), Box<dyn Error>> {
    // Setup headers
    let mut headers = HeaderMap::new();
    headers.insert("x-api-key", HeaderValue::from_str(&api_key)?);
    headers.insert("anthropic-version", HeaderValue::from_static("2023-06-01"));

    // Create client and send request
    let client = reqwest::Client::new();
    let response = client
        .post("https://api.anthropic.com/v1/messages")
        .headers(headers)
        .json(&request)
        .send()
        .await?;

    // Stream the response
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        let text = String::from_utf8(chunk.to_vec())?;

        // Parse SSE and send content
        if text.starts_with("data: ") {
            if let Ok(event) = serde_json::from_str::<StreamEvent>(&text["data: ".len()..]) {
                match event {
                    StreamEvent::ContentBlockDelta { delta } => {
                        writer.write_all(delta.text.as_bytes()).await?;
                        writer.flush().await?;
                    }
                    StreamEvent::MessageDelta { delta } => {
                        if let Some(reason) = delta.stop_reason {
                            println!("Stream complete: {}", reason);
                            break;
                        }
                    }
                }
            }
        }
    }

    Ok(())
}
