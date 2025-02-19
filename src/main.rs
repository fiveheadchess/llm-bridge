use futures_util::StreamExt;
use serde::Deserialize;
use std::env;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{TcpListener, TcpStream},
};

mod models;
use crate::models::{ClaudeMessage, ClaudeStreamApiRequest};
mod ai_service;
use crate::ai_service::stream_ai_response;

#[derive(Deserialize)]
struct ClientRequest {
    prompt: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // bind the socket TcpListener
    let listener = TcpListener::bind("0.0.0.0:8080").await?;
    println!("listening on port 8080");

    loop {
        let (socket, addr) = listener.accept().await?;
        println!("new connection from: {}", addr);
        // naive approach give a thread for a conn and let the OS handle
        //      the queue-ing of connections that pile up on this socket
        tokio::spawn(async move {
            if let Err(e) = handle_connection(socket).await {
                eprintln!("error handling connection: {}", e);
            }
        });
    }
}

async fn handle_connection(mut socket: TcpStream) -> Result<(), Box<dyn std::error::Error>> {
    let (reader, mut writer) = socket.split();
    let mut reader = BufReader::new(reader);
    let mut line = String::new();

    // now we can determine mode or go through to the LLM
    reader.read_line(&mut line).await?;

    if env::var("ECHO_MODE").is_ok() {
        writer.write_all(line.as_bytes()).await?;
        writer.flush().await?;
        return Ok(());
    }

    // parse request and call the API, if it matches format send to LLM
    match serde_json::from_str::<ClientRequest>(&line) {
        Ok(req) => send_to_llm(req.prompt, &mut writer).await?,
        Err(e) => {
            writer.write_all(format!("error: {}", e).as_bytes()).await?;
        }
    }

    Ok(())
}

async fn send_to_llm(
    prompt: String,
    writer: &mut (impl AsyncWriteExt + Unpin),
) -> Result<(), Box<dyn std::error::Error>> {
    let api_key = env::var("ANTHROPIC_API_KEY")?;
    let claude_req = ClaudeStreamApiRequest {
        model: "claude-3-opus-20240229".to_string(),
        max_tokens: 1024,
        messages: vec![ClaudeMessage {
            role: "user".to_string(),
            content: prompt,
        }],
        stream: true,
    };
    let mut stream = stream_ai_response(api_key, claude_req).await;
    while let Some(Ok(msg)) = stream.next().await {
        if let axum::extract::ws::Message::Text(text) = msg {
            writer.write_all(text.as_bytes()).await?;
        }
    }
    Ok(())
}
