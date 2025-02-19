use std::sync::Arc;
use tokio::runtime::Builder;
use tokio::{
    io::AsyncReadExt, io::AsyncWriteExt, net::TcpListener, net::TcpStream, sync::Semaphore,
};
use tracing::{error, info};

/// Thread Model:
///     tokio by default creates a mult threaded scheduler
///         with a number of worker threads equal to the number of CPU cores
///         #[tokio::main] uses this default configuration
///     worker threads are responsible for executing tasks that are
///         spawned with tokio::spawn()
///         the tasks are lightweight "green threads" (coroutines or async tasks)
///         they can be moved between worker threads
///
/// Backlog Queue:
///     at the OS level, incoming TCP conns are handled through a listening
///     socket's backlog Queue
///     When a TcpListener is created the OS maintains
///         SYN Queue -> incomplete conns
///         Accept Queue (complete conns waiting to be accepted)
///     in linux you can check max backlog size with
///         "cat /proc/sys/net/core/somaxconn"
#[tokio::main]
async fn main() {
    // TODO: this is a mac M1 optimization that needs to be removed
    //       when deployed on cloud
    let runtime = Builder::new_multi_thread()
        .worker_threads(6)
        .thread_name("worker")
        .build()
        .unwrap();

    // TODO: conservative connection limit to start, change for prod
    let connection_semaphore = Arc::new(Semaphore::new(10_000));

    runtime.block_on(async {
        // TODO: this address must be turned to an env var for prod
        let listener = TcpListener::bind("0.0.0.0:8080").await.unwrap();
        info!("server running with 6 worker threads, 10l connection limit");
        loop {
            let permit = connection_semaphore.clone();
            let (socket, _addr) = listener.accept().await.unwrap();
            tokio::spawn(async move {
                // just reply then terminate
                handle_connection(socket).await;
                drop(permit);
            });
        }
    })
}

async fn handle_connection(mut socket: TcpStream) {
    let mut buffer = vec![0; 1024];

    // Read from the socket
    match socket.read(&mut buffer).await {
        Ok(n) if n == 0 => {
            // Connection closed
        }
        Ok(n) => {
            // Echo the data back
            if let Err(e) = socket.write_all(&buffer[..n]).await {
                error!("Failed to write to socket: {}", e);
            }
        }
        Err(e) => {
            error!("Failed to read from socket: {}", e);
        }
    }
}
