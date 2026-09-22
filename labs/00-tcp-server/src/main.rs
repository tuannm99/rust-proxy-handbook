// TCP Echo Server. See instruction/01-network/07-socket.md, instruction/01-network/08-tcp.md, instruction/03-rust/05-async.md, instruction/04-runtime/01-tokio.md.

use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // bind IP + Port
    let listener = TcpListener::bind("127.0.0.1:5000").await?;

    loop {
        match listener.accept().await {
            Ok((mut socket, _addr)) => {
                tokio::spawn(async move {
                    let mut buf = [0; 1024];

                    // In a loop, read data from the socket and write the data back.
                    loop {
                        let n = match socket.read(&mut buf).await {
                            // socket closed
                            Ok(0) => return,
                            Ok(n) => n,
                            Err(e) => {
                                eprintln!("failed to read from socket; err = {:?}", e);
                                return;
                            }
                        };

                        // Write the data back
                        if let Err(e) = socket.write_all(&buf[0..n]).await {
                            eprintln!("failed to write to socket; err = {:?}", e);
                            return;
                        }
                    }
                });
            }
            Err(e) => {
                eprintln!("accept error: {:?}", e);

                let condition = match e.raw_os_error() {
                    // libc::EMFILE: Per-process limit reached (Process hết FD)
                    // libc::ENFILE: System-wide limit reached (Toàn hệ thống hết FD)
                    Some(code) => code == libc::EMFILE || code == libc::ENFILE,
                    None => false,
                };

                if condition {
                    tokio::time::sleep(Duration::from_millis(100)).await;
                } else {
                    continue;
                }
            }
        }
    }
}
