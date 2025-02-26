use dotenvy::dotenv;
use reqwest::header::{HeaderMap, HeaderValue};
use serde::Deserialize;
use serde_json::json;
use std::{convert::Infallible, env, path::PathBuf, sync::Arc, time::Instant};

use futures_util::stream::StreamExt;
use hyper::{
    service::{make_service_fn, service_fn},
    Body, Method, Request, Response, Server, StatusCode,
};
use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::fs;
use tower_http::cors::{Any, CorsLayer};
use uuid::Uuid;

mod models;
use crate::models::{ClaudeMessage, ClaudeStreamApiRequest, StreamEvent};

// Client tracking structure
struct ClientInfo {
    id: String,
    start_time: Instant,
    request_path: String,
    chunks_sent: AtomicUsize,
    bytes_sent: AtomicUsize,
    client_ip: String,
}

impl ClientInfo {
    fn new(path: &str, ip: &str) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            start_time: Instant::now(),
            request_path: path.to_string(),
            chunks_sent: AtomicUsize::new(0),
            bytes_sent: AtomicUsize::new(0),
            client_ip: ip.to_string(),
        }
    }

    fn increment_chunks(&self, bytes: usize) -> (usize, usize) {
        let chunks = self.chunks_sent.fetch_add(1, Ordering::SeqCst) + 1;
        let total_bytes = self.bytes_sent.fetch_add(bytes, Ordering::SeqCst) + bytes;
        (chunks, total_bytes)
    }

    fn duration_ms(&self) -> u128 {
        self.start_time.elapsed().as_millis()
    }
}

// Type to hold the request body for LLM requests
#[derive(Deserialize)]
struct LlmRequest {
    prompt: String,
}

// Type to hold the request body for echo requests
#[derive(Deserialize)]
struct EchoRequest {
    message: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // load dotenv
    dotenv().ok();

    // Verify API key is available
    match env::var("ANTHROPIC_API_KEY") {
        Ok(key) => {
            println!("ANTHROPIC_API_KEY found, length: {}", key.len());
        }
        Err(_) => {
            eprintln!("ERROR: ANTHROPIC_API_KEY not found in environment variables");
            eprintln!("Please create a .env file with your Anthropic API key");
            return Err("Missing API key".into());
        }
    }

    // Define the socket address
    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    println!("Server listening on http://0.0.0.0:8080");

    // Create a service function that will handle incoming requests
    let make_svc = make_service_fn(|conn: &hyper::server::conn::AddrStream| {
        let remote_addr = conn.remote_addr();
        async move {
            Ok::<_, Infallible>(service_fn(move |req| {
                handle_request(req, remote_addr.to_string())
            }))
        }
    });

    // Create and start the server
    let server = Server::bind(&addr).serve(make_svc);
    println!("Server started successfully");

    // Wait for the server to complete
    if let Err(e) = server.await {
        eprintln!("server error: {}", e);
    }

    Ok(())
}

// Function to handle incoming HTTP requests
async fn handle_request(
    req: Request<Body>,
    remote_addr: String,
) -> Result<Response<Body>, Infallible> {
    // Extract the path and method from the request
    let path = req.uri().path().to_string();
    let method = req.method().clone();

    // Create client info
    let client_info = Arc::new(ClientInfo::new(&path, &remote_addr));

    println!(
        "Client connected: [{}] {} requesting {} {}",
        client_info.id,
        client_info.client_ip,
        method.as_str(),
        client_info.request_path
    );

    // Match on the path and method to determine the action
    let response = match (method.as_str(), path.as_str()) {
        // API endpoints
        ("POST", "/api/echo") => handle_echo(req, Arc::clone(&client_info)).await,
        ("POST", "/api/llm") => handle_llm(req, Arc::clone(&client_info)).await,

        // Static files
        ("GET", "/") => serve_file("index.html", "text/html", Arc::clone(&client_info)).await,
        ("GET", path) if path.starts_with("/static/") => {
            let file_path = &path[8..]; // Remove "/static/" prefix
            let content_type = match file_path.split('.').last() {
                Some("html") => "text/html",
                Some("js") => "application/javascript",
                Some("css") => "text/css",
                _ => "application/octet-stream",
            };
            serve_file(file_path, content_type, Arc::clone(&client_info)).await
        }

        // Health check endpoint
        ("GET", "/health") => {
            println!("Client [{}]: health check", client_info.id);
            Ok(Response::new(Body::from("OK")))
        }

        // Handle any other request with a 404
        _ => {
            println!("Client [{}]: 404 Not Found for {}", client_info.id, path);
            let mut response = Response::new(Body::from("Not Found"));
            *response.status_mut() = StatusCode::NOT_FOUND;
            Ok(response)
        }
    };

    // Log request completion for non-streaming responses
    if path != "/api/llm" {
        println!(
            "Client [{}]: request completed in {}ms",
            client_info.id,
            client_info.duration_ms()
        );
    }

    response
}

// Function to serve static files
async fn serve_file(
    path: &str,
    content_type: &str,
    client_info: Arc<ClientInfo>,
) -> Result<Response<Body>, Infallible> {
    let mut file_path = PathBuf::from("static");
    file_path.push(path);

    match fs::read(file_path).await {
        Ok(content) => {
            println!(
                "Client [{}]: serving file {} ({} bytes)",
                client_info.id,
                path,
                content.len()
            );

            let mut response = Response::new(Body::from(content));
            response
                .headers_mut()
                .insert("Content-Type", HeaderValue::from_str(content_type).unwrap());
            Ok(response)
        }
        Err(e) => {
            println!(
                "Client [{}]: file not found: {} ({})",
                client_info.id, path, e
            );

            let mut response = Response::new(Body::from("File not found"));
            *response.status_mut() = StatusCode::NOT_FOUND;
            Ok(response)
        }
    }
}

// Function to handle echo requests
async fn handle_echo(
    req: Request<Body>,
    client_info: Arc<ClientInfo>,
) -> Result<Response<Body>, Infallible> {
    println!("Client [{}]: echo request received", client_info.id);

    // Convert the request body to bytes
    let body_bytes = hyper::body::to_bytes(req.into_body())
        .await
        .unwrap_or_default();

    // Try to parse the body as an EchoRequest
    match serde_json::from_slice::<EchoRequest>(&body_bytes) {
        Ok(echo_req) => {
            let response = json!({ "type": "ECHO", "message": echo_req.message });
            let response_body = serde_json::to_string(&response).unwrap_or_default();

            println!(
                "Client [{}]: echo response sent ({} bytes)",
                client_info.id,
                response_body.len()
            );

            Ok(Response::new(Body::from(response_body)))
        }
        Err(e) => {
            println!("Client [{}]: bad echo request: {}", client_info.id, e);

            // If we can't parse the body, return a 400 Bad Request
            let mut response = Response::new(Body::from(format!("Bad request: {}", e)));
            *response.status_mut() = StatusCode::BAD_REQUEST;
            Ok(response)
        }
    }
}

// Function to handle LLM requests
async fn handle_llm(
    req: Request<Body>,
    client_info: Arc<ClientInfo>,
) -> Result<Response<Body>, Infallible> {
    println!("Client [{}]: LLM request received", client_info.id);

    // Convert the request body to bytes
    let body_bytes = hyper::body::to_bytes(req.into_body())
        .await
        .unwrap_or_default();

    // Try to parse the body as an LlmRequest
    match serde_json::from_slice::<LlmRequest>(&body_bytes) {
        Ok(llm_req) => {
            println!(
                "Client [{}]: LLM request with prompt length: {} chars",
                client_info.id,
                llm_req.prompt.len()
            );

            // Create a channel for streaming
            let (tx, rx) = tokio::sync::mpsc::channel(100);

            // Convert channel receiver to a Stream for hyper
            let stream =
                tokio_stream::wrappers::ReceiverStream::new(rx).map(|r| Ok::<_, Infallible>(r));

            // Set up streaming response
            let mut response = Response::new(Body::wrap_stream(stream));
            response
                .headers_mut()
                .insert("Content-Type", HeaderValue::from_static("text/plain"));
            response
                .headers_mut()
                .insert("Transfer-Encoding", HeaderValue::from_static("chunked"));
            response
                .headers_mut()
                .insert("Cache-Control", HeaderValue::from_static("no-cache"));
            response
                .headers_mut()
                .insert("Connection", HeaderValue::from_static("keep-alive"));
            response
                .headers_mut()
                .insert("Access-Control-Allow-Origin", HeaderValue::from_static("*"));

            // Create a clone of client_info for the spawned task
            let client_info_clone = Arc::clone(&client_info);
            let prompt = llm_req.prompt;

            // Launch a separate task to process the LLM request
            tokio::spawn(async move {
                process_llm_request(prompt, tx, client_info_clone).await;
            });

            Ok(response)
        }
        Err(e) => {
            println!("Client [{}]: bad LLM request: {}", client_info.id, e);

            // If we can't parse the body, return a 400 Bad Request
            let mut response = Response::new(Body::from(format!("Bad request: {}", e)));
            *response.status_mut() = StatusCode::BAD_REQUEST;
            Ok(response)
        }
    }
}

// Function to process an LLM request and send results through a channel
async fn process_llm_request(
    prompt: String,
    tx: tokio::sync::mpsc::Sender<String>,
    client_info: Arc<ClientInfo>,
) {
    let client_id = &client_info.id;

    // Initial message
    let _ = tx.send("Connecting to Claude API...\n".to_string()).await;

    // Get the API key
    let api_key = match env::var("ANTHROPIC_API_KEY") {
        Ok(key) => key,
        Err(e) => {
            let err_msg = format!("Error: API key not found - {}\n", e);
            println!("Client [{}]: {}", client_id, err_msg);
            let _ = tx.send(err_msg).await;
            return;
        }
    };

    // Create the Claude request
    let claude_req = ClaudeStreamApiRequest {
        model: "claude-3-opus-20240229".to_string(),
        max_tokens: 1024,
        messages: vec![ClaudeMessage {
            role: "user".to_string(),
            content: prompt,
        }],
        stream: true,
    };

    // Set up headers
    let mut headers = HeaderMap::new();
    match HeaderValue::from_str(&api_key) {
        Ok(value) => {
            headers.insert("x-api-key", value);
        }
        Err(e) => {
            let err_msg = format!("Error: Invalid API key format - {}\n", e);
            println!("Client [{}]: {}", client_id, err_msg);
            let _ = tx.send(err_msg).await;
            return;
        }
    }
    headers.insert("anthropic-version", HeaderValue::from_static("2023-06-01"));
    headers.insert("content-type", HeaderValue::from_static("application/json"));

    // Make the request
    println!("Client [{}]: sending request to Claude API", client_id);
    let client = reqwest::Client::new();

    let response = match client
        .post("https://api.anthropic.com/v1/messages")
        .headers(headers)
        .json(&claude_req)
        .send()
        .await
    {
        Ok(resp) => {
            if !resp.status().is_success() {
                let status = resp.status();
                let error_text = match resp.text().await {
                    Ok(text) => text,
                    Err(e) => format!("Failed to get error text: {}", e),
                };

                let err_msg = format!("Claude API error ({}): {}\n", status, error_text);
                println!("Client [{}]: {}", client_id, err_msg);
                let _ = tx.send(err_msg).await;
                return;
            }
            resp
        }
        Err(e) => {
            let err_msg = format!("Error connecting to Claude API: {}\n", e);
            println!("Client [{}]: {}", client_id, err_msg);
            let _ = tx.send(err_msg).await;
            return;
        }
    };

    println!("Client [{}]: streaming response from Claude", client_id);
    let _ = tx.send("\n".to_string()).await;

    // Process the stream
    let mut stream = response.bytes_stream();
    let mut buffer = String::new();

    while let Some(chunk_result) = stream.next().await {
        match chunk_result {
            Ok(chunk) => {
                match String::from_utf8(chunk.to_vec()) {
                    Ok(text) => {
                        // Process the text line by line
                        for c in text.chars() {
                            if c == '\n' {
                                // Process the completed line
                                process_sse_line(&buffer, &tx, client_info.clone()).await;
                                buffer.clear();
                            } else {
                                buffer.push(c);
                            }
                        }
                    }
                    Err(e) => {
                        let err_msg = format!("Error decoding chunk: {}\n", e);
                        println!("Client [{}]: {}", client_id, err_msg);
                        let _ = tx.send(err_msg).await;
                    }
                }
            }
            Err(e) => {
                let err_msg = format!("Error receiving chunk: {}\n", e);
                println!("Client [{}]: {}", client_id, err_msg);
                let _ = tx.send(err_msg).await;
                break;
            }
        }
    }

    println!(
        "Client [{}]: stream complete, sent {} chunks ({} bytes) in {}ms",
        client_id,
        client_info.chunks_sent.load(Ordering::SeqCst),
        client_info.bytes_sent.load(Ordering::SeqCst),
        client_info.duration_ms()
    );
}

// Process an SSE line and extract content if available
async fn process_sse_line(
    line: &str,
    tx: &tokio::sync::mpsc::Sender<String>,
    client_info: Arc<ClientInfo>,
) {
    if !line.starts_with("data: ") {
        return;
    }

    let data = &line["data: ".len()..];
    if data == "[DONE]" {
        println!("Client [{}]: Received [DONE] marker", client_info.id);
        return;
    }

    match serde_json::from_str::<StreamEvent>(data) {
        Ok(event) => {
            match event {
                StreamEvent::ContentBlockDelta { delta, .. } => {
                    // Increment chunk counter and get stats
                    let (chunk_num, total_bytes) = client_info.increment_chunks(delta.text.len());

                    println!(
                        "Client [{}]: sending chunk #{} ({} bytes), total: {} bytes",
                        client_info.id,
                        chunk_num,
                        delta.text.len(),
                        total_bytes
                    );

                    // Send the content to the client
                    let _ = tx.send(delta.text).await;
                }
                StreamEvent::MessageDelta { delta, .. } => {
                    if let Some(reason) = delta.stop_reason {
                        println!(
                            "Client [{}]: stream complete reason: {}",
                            client_info.id, reason
                        );
                    }
                }
                _ => {} // Ignore other event types
            }
        }
        Err(e) => {
            println!("Client [{}]: Error parsing event: {}", client_info.id, e);
            println!("Client [{}]: Raw data: {}", client_info.id, data);
        }
    }
}
