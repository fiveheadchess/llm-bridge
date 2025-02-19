use dotenvy::dotenv;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::json;
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
#[serde(tag = "type")]
enum Request {
    #[serde(rename = "ECHO")]
    Echo { message: String },
    #[serde(rename = "LLM")]
    Llm { prompt: String },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // load dotenv
    dotenv().ok();
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
    let addr = socket.peer_addr()?;
    println!("handling connectoin from: {}", addr);

    let (reader, mut writer) = socket.split();
    let mut reader = BufReader::new(reader);
    let mut line = String::new();

    // now we can determine mode or go through to the LLM
    reader.read_line(&mut line).await?;
    println!("received request: {}", line);

    match serde_json::from_str::<Request>(&line) {
        Ok(Request::Echo { message }) => {
            let response = json!({ "type": "ECHO", "message": message });
            writer
                .write_all(serde_json::to_string(&response)?.as_bytes())
                .await?;
            writer.flush().await?;
        }
        Ok(Request::Llm { prompt }) => {
            send_to_llm(prompt, &mut writer).await?;
        }
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
    println!("send req to LLM");
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

    // Now we just pass the writer directly to stream_ai_response
    stream_ai_response(api_key, claude_req, writer).await?;
    Ok(())
}
