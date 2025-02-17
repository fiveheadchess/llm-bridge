use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tracing::{error, info};

#[tokio::main]
async fn main() {
    // Initialize logging
    tracing_subscriber::fmt().with_env_filter("info").init();

    info!("Starting TCP Echo Server...");

    // Bind to the address
    let listener = TcpListener::bind("0.0.0.0:8080").await.unwrap();
    info!("Server running on tcp://0.0.0.0:8080");

    // Accept connections
    loop {
        match listener.accept().await {
            Ok((socket, addr)) => {
                info!("New client connected: {}", addr);

                // Spawn a new task for each connection
                tokio::spawn(async move {
                    handle_connection(socket).await;
                });
            }
            Err(e) => {
                error!("Failed to accept connection: {}", e);
            }
        }
    }
}

async fn handle_connection(mut socket: TcpStream) {
    let mut buffer = vec![0; 1024];

    // Read from the socket
    match socket.read(&mut buffer).await {
        Ok(n) if n == 0 => {
            // Connection closed
            return;
        }
        Ok(n) => {
            // Echo the data back
            if let Err(e) = socket.write_all(&buffer[..n]).await {
                error!("Failed to write to socket: {}", e);
                return;
            }
        }
        Err(e) => {
            error!("Failed to read from socket: {}", e);
            return;
        }
    }
}
