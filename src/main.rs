use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{TcpListener, TcpStream},
};

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

    while reader.read_line(&mut line).await? > 0 {
        // read until newline
        println!("received: {}", line.trim());
        // echo
        writer.write_all(line.as_bytes()).await?;
        writer.flush().await?;
        line.clear()
    }
    Ok(())
}
