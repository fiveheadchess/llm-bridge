use dotenvy::dotenv;
use futures_util::StreamExt;
use reqwest::header::{HeaderMap, HeaderValue};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{env, net::SocketAddr};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{TcpListener, TcpStream},
};

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
        let client = Client { socket, addr };
        println!("new connection from: {}", addr);
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
            bridge_llm(prompt, &mut writer).await?;
        }
        Err(e) => {
            writer.write_all(format!("error: {}", e).as_bytes()).await?;
        }
    }
    Ok(())
}

async fn bridge_llm(
    prompt: String,
    writer: &mut (impl AsyncWriteExt + Unpin),
) -> Result<(), Box<dyn std::error::Error>> {
    println!("sending req to LLM");
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
    stream_from_claude(api_key, claude_req, writer).await?;
    Ok(())
}

async fn stream_from_claude(
    api_key: String,
    request: ClaudeStreamApiRequest,
    writer: &mut (impl AsyncWriteExt + Unpin),
) -> Result<(), Box<dyn std::error::Error>> {
    let mut headers = HeaderMap::new();
    headers.insert("x-api-key", HeaderValue::from_str(&api_key)?); // Added ?
    headers.insert("anthropic-version", HeaderValue::from_static("2023-06-01"));
    // this is a reqwest specific client different from the connected client used before
    let client = reqwest::Client::new();
    let mut response = client
        .post("https://api.anthropic.com/v1/messages")
        .headers(headers)
        .json(&request)
        .send()
        .await?;
    println!("streaming response from claude to client");
    // using the term block for data block rather than chunk which is ugly
    while let Some(chunk) = response.chunk().await? {
        let text = String::from_utf8(chunk.to_vec())?;

        if text.starts_with("data: ") {
            if let Ok(event) = serde_json::from_str::<StreamEvent>(&text["data: ".len()..]) {
                match event {
                    StreamEvent::ContentBlockDelta { delta } => {
                        writer.write_all(delta.text.as_bytes()).await?;
                        writer.flush().await?;
                    }
                    StreamEvent::MessageDelta { delta } => {
                        if let Some(reason) = delta.stop_reason {
                            println!("stream complete: {}", reason);
                            break;
                        }
                    }
                }
            }
        }
    }
    Ok(())
}
