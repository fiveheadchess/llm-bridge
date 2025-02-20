use dotenvy::dotenv;
use reqwest::header::{HeaderMap, HeaderValue};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{env, net::SocketAddr};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{TcpListener, TcpStream},
};
use uuid::Uuid;

mod models;
use crate::models::{ClaudeMessage, ClaudeStreamApiRequest, StreamEvent};

#[derive(Deserialize)]
#[serde(tag = "type")]
enum Request {
    #[serde(rename = "ECHO")]
    Echo { message: String },
    #[serde(rename = "LLM")]
    Llm { prompt: String },
}

struct Client {
    id: String,
    addr: SocketAddr,
    socket: TcpStream,
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
        let client = Client {
            id: Uuid::new_v4().to_string(),
            addr: addr,
            socket: socket,
        };
        println!(
            "new connected client \nid: {}\naddress: {}",
            client.id, client.addr
        );
        // naive approach give a thread for a conn and let the OS handle
        //      the queue-ing of connections that pile up on this socket
        tokio::spawn(async move {
            if let Err(e) = handle_client(client).await {
                eprintln!("error handling connection: {}", e);
            }
        });
    }
}

async fn handle_client(mut client: Client) -> Result<(), Box<dyn std::error::Error>> {
    println!("handling request from: {}", client.addr);
    // split into reader and writer, writer is direct back to client
    let (reader, mut writer) = client.socket.split();
    let mut reader = BufReader::new(reader);
    let mut line = String::new();

    // now we can determine mode or go through to the LLM
    reader.read_line(&mut line).await?;
    println!("received request: {}", line);
    //TODO: this should be decoupled probably
    match serde_json::from_str::<Request>(&line) {
        Ok(Request::Echo { message }) => {
            let response = json!({ "type": "ECHO", "message": message });
            writer
                .write_all(serde_json::to_string(&response)?.as_bytes())
                .await?;
            writer.flush().await?;
        }
        Ok(Request::Llm { prompt }) => {
            bridge_llm(prompt, &client.id, &mut writer).await?;
        }
        Err(e) => {
            writer.write_all(format!("error: {}", e).as_bytes()).await?;
        }
    }
    Ok(())
}

async fn bridge_llm(
    prompt: String,
    client_id: &str,
    writer: &mut (impl AsyncWriteExt + Unpin),
) -> Result<(), Box<dyn std::error::Error>> {
    println!("sending req to LLM");
    let api_key = env::var("ANTHROPIC_API_KEY")?;
    println!("Got API key of length: {}", api_key.len());

    let claude_req = ClaudeStreamApiRequest {
        model: "claude-3-opus-20240229".to_string(),
        max_tokens: 1024,
        messages: vec![ClaudeMessage {
            role: "user".to_string(),
            content: prompt.clone(),
        }],
        stream: true,
    };
    println!(
        "Sending request to Claude with prompt length: {}",
        prompt.len()
    );
    stream_from_claude(api_key, client_id, claude_req, writer).await?;
    Ok(())
}

async fn stream_from_claude(
    api_key: String,
    client_id: &str,
    request: ClaudeStreamApiRequest,
    writer: &mut (impl AsyncWriteExt + Unpin),
) -> Result<(), Box<dyn std::error::Error>> {
    let mut headers = HeaderMap::new();
    headers.insert("x-api-key", HeaderValue::from_str(&api_key)?);
    headers.insert("anthropic-version", HeaderValue::from_static("2023-06-01"));
    headers.insert("content-type", HeaderValue::from_static("application/json"));

    let client = reqwest::Client::new();
    let mut response = client
        .post("https://api.anthropic.com/v1/messages")
        .headers(headers)
        .json(&request)
        .send()
        .await?
        .error_for_status()?;

    println!("streaming response from claude to client");

    while let Some(chunk) = response.chunk().await? {
        let text = String::from_utf8(chunk.to_vec())?;
        let lines: Vec<&str> = text.lines().collect();

        for line in lines {
            if let Some(event) = parse_sse_line(line) {
                match event {
                    StreamEvent::MessageStart { message } => {
                        println!("Message started: {}", message.id);
                    }
                    StreamEvent::ContentBlockStart {
                        index,
                        content_block,
                    } => {
                        println!("Content block {} started", index);
                    }
                    StreamEvent::ContentBlockDelta { index, delta } => {
                        println!(
                            "client {}: received {} bytes from LLM, sending to client",
                            client_id,
                            delta.text.len()
                        );
                        writer.write_all(delta.text.as_bytes()).await?;
                        writer.flush().await?;
                    }
                    StreamEvent::ContentBlockStop { index } => {
                        println!("Content block {} stopped", index);
                    }
                    StreamEvent::MessageDelta { delta, usage } => {
                        if let Some(reason) = delta.stop_reason {
                            println!("client {}: stream complete: {}", client_id, reason);
                            if let Some(tokens) = usage.output_tokens {
                                println!("Total tokens used: {}", tokens);
                            }
                            writer.write_all(b"\n").await?;
                            writer.flush().await?;
                        }
                    }
                    StreamEvent::MessageStop => {
                        println!("Message complete");
                    }
                    StreamEvent::Ping => {
                        // Ignore ping events
                    }
                }
            }
        }
    }

    println!(
        "client {}: LLM connection closed, closing client connection",
        client_id
    );
    writer.shutdown().await?;
    println!("client {}: connection closed successfully", client_id);

    Ok(())
}

// Add thfunction to parse SSE events
pub fn parse_sse_line(line: &str) -> Option<StreamEvent> {
    if line.starts_with("data: ") {
        let json = &line["data: ".len()..];
        match serde_json::from_str(json) {
            Ok(event) => Some(event),
            Err(e) => {
                eprintln!("Error parsing SSE data: {}", e);
                None
            }
        }
    } else {
        None
    }
}
