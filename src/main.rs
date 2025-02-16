use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    routing::get,
    Router,
};
use futures_util::{stream::StreamExt, SinkExt};
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::{sync::mpsc, sync::RwLock, time::interval};
use tracing::{error, info};
use uuid::Uuid;

mod ai_service;
mod models;
use ai_service::stream_ai_response;
use models::ClaudeStreamApiRequest;

type ClientMap = Arc<RwLock<HashMap<String, mpsc::UnboundedSender<Message>>>>;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().with_env_filter("info").init();
    info!("Starting WebSocket LLM Bridge server...");

    let clients: ClientMap = Arc::new(RwLock::new(HashMap::new()));

    let app = Router::new().route(
        "/ws",
        get(|ws: WebSocketUpgrade| async move {
            ws.on_upgrade(|socket| handle_socket(socket, clients))
        }),
    );

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await.unwrap();
    info!("Server running on ws://0.0.0.0:8080/ws");
    axum::serve(listener, app).await.unwrap();
}

async fn handle_socket(socket: WebSocket, clients: ClientMap) {
    let (mut sender, mut receiver) = socket.split();
    let (tx, mut rx) = mpsc::unbounded_channel();
    let (internal_tx, mut internal_rx) = mpsc::unbounded_channel();
    let client_id = Uuid::new_v4().to_string();

    // Insert client into map (write lock)
    {
        let mut clients_lock = clients.write().await;
        clients_lock.insert(client_id.clone(), tx);
    }

    info!("New WebSocket client connected: {}", client_id);

    let last_active = Arc::new(RwLock::new(Instant::now()));
    let last_active_clone = last_active.clone();

    // Keep-alive task (pings clients every 30 seconds)
    let keep_alive_clients = clients.clone();
    let keep_alive_client_id = client_id.clone();
    let keep_alive_tx = internal_tx.clone();
    tokio::spawn(async move {
        let mut interval = interval(Duration::from_secs(30));
        loop {
            interval.tick().await;

            let elapsed = last_active_clone.read().await.elapsed();
            if elapsed > Duration::from_secs(60) {
                info!(
                    "Client {} timed out due to inactivity",
                    keep_alive_client_id
                );
                let mut clients_lock = keep_alive_clients.write().await;
                clients_lock.remove(&keep_alive_client_id);
                break;
            }

            if keep_alive_tx.send(Message::Ping(vec![])).is_err() {
                error!("Failed to send ping to client {}", keep_alive_client_id);
                break;
            }
        }
    });

    // Handle incoming messages
    let handle_ai_clients = clients.clone();
    let handle_messages_client_id = client_id.clone();
    let message_internal_tx = internal_tx.clone();
    tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            *last_active.write().await = Instant::now(); // Update last active time

            if let Message::Text(text) = msg {
                info!(
                    "Received message from {}: {}",
                    handle_messages_client_id, text
                );
                let request: ClaudeStreamApiRequest = match serde_json::from_str(&text) {
                    Ok(req) => req,
                    Err(e) => {
                        error!("Invalid JSON from {}: {}", handle_messages_client_id, e);
                        continue;
                    }
                };

                let api_key = match std::env::var("ANTHROPIC_API_KEY") {
                    Ok(key) => key,
                    Err(_) => {
                        error!("Missing API key for Claude API");
                        continue;
                    }
                };

                let response_stream = stream_ai_response(api_key, request).await;
                tokio::spawn(handle_ai_response(
                    handle_messages_client_id.clone(),
                    response_stream,
                    handle_ai_clients.clone(),
                ));
            }
        }

        // Remove client on disconnect
        let mut clients_lock = handle_ai_clients.write().await;
        clients_lock.remove(&handle_messages_client_id);
        info!("Client {} disconnected", handle_messages_client_id);
    });

    // Handle outgoing messages
    let outgoing_client_id = client_id.clone();
    tokio::spawn(async move {
        loop {
            tokio::select! {
                Some(msg) = rx.recv() => {
                    if sender.send(msg).await.is_err() {
                        error!("Client {} disconnected unexpectedly", outgoing_client_id);
                        break;
                    }
                }
                Some(msg) = internal_rx.recv() => {
                    if sender.send(msg).await.is_err() {
                        error!("Client {} disconnected unexpectedly", outgoing_client_id);
                        break;
                    }
                }
                else => break
            }
        }

        // Remove client on disconnect
        let mut clients_lock = clients.write().await;
        clients_lock.remove(&outgoing_client_id);
        info!("Client {} disconnected", outgoing_client_id);
    });
}

async fn handle_ai_response(
    client_id: String,
    mut response_stream: futures_util::stream::BoxStream<
        'static,
        Result<Message, Box<dyn std::error::Error + Send + Sync>>,
    >,
    clients: ClientMap,
) {
    let clients_lock = clients.read().await;
    if let Some(client) = clients_lock.get(&client_id) {
        let client_tx = client.clone();

        while let Some(result) = response_stream.next().await {
            match result {
                Ok(msg) => {
                    if client_tx.send(msg).is_err() {
                        error!("Failed to send AI response to {}", client_id);
                        break;
                    }
                }
                Err(e) => {
                    error!("Error in AI response stream for {}: {}", client_id, e);
                    break;
                }
            }
        }

        info!("Finished streaming AI response to {}", client_id);
    }
}
