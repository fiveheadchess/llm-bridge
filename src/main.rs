use std::sync::Arc;
use tokio::runtime::Builder;
use tokio::{
    io::AsyncReadExt, io::AsyncWriteExt, io::BufStream, net::TcpListener, net::TcpStream,
    sync::Semaphore, time::timeout, time::Duration,
};
use tracing::{error, info};
//TODO: make configurable with env
// a message end sequnce to expect from the client
const MESSAGE_END_SEQUENCE: &[u8] = b"<<END>>\n";

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
    // This is manually tellint the tokio runtime what to do when
    //      allocating worker threads rather than letting it
    //      do it its default method
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

// TODO: we can have an echo for testing purposes and a GET for sending to LLM
// TODO: right now this just echos, no further work is being done

async fn handle_connection(mut socket: TcpStream) {
    let mut stream = BufStream::new(socket);
    let mut message = Vec::new();
    let mut buffer = [0u8; 1024];
    const TIMEOUT_DURATION: Duration = Duration::from_secs(30);

    loop {
        // Handle the timeout Result first
        let read_result = match timeout(TIMEOUT_DURATION, stream.read(&mut buffer)).await {
            Ok(result) => result,
            Err(_) => {
                error!("Connection timed out");
                return;
            }
        };

        // Then handle the read Result
        let n = match read_result {
            Ok(0) => break, // Connection closed by client
            Ok(n) => n,
            Err(e) => {
                error!("Read error: {}", e);
                return;
            }
        };

        message.extend_from_slice(&buffer[..n]);
        
        // Check if we have our end sequence
        if message
            .windows(MESSAGE_END_SEQUENCE.len())
            .any(|window| window == MESSAGE_END_SEQUENCE)
        {
            break;
        }
        
        // Prevent memory exhaustion
        if message.len() > 1024 * 1024 {
            // 1MB limit
            error!("Message too large");
            return;
        }
    }

    if let Err(e) = stream.write_all(&message).await {
        error!("Write error: {}", e);
    }
    
    // Make sure to flush the buffer
    if let Err(e) = stream.flush().await {
        error!("Flush error: {}", e);
    }
}
