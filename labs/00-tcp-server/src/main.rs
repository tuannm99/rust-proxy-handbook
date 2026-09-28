// TCP Echo Server. See instruction/01-network/07-socket.md, instruction/01-network/08-tcp.md, instruction/03-rust/05-async.md, instruction/04-runtime/01-tokio.md.

// REVIEW(medium): nothing bounds what a single client can hold. There is no
// cap on concurrent connections and no idle/read/write timeout, so anyone can
// open connections until the process hits EMFILE and then keep them forever.
// Measured: while pinned at EMFILE, 14 of 200 new connection attempts timed
// out — legitimate clients get locked out. Not required for this lab's
// checklist; resolve it here or record it as deliberate debt to pay in
// proxy/ — see instruction/07-security/09-ddos.md (accept-rate/concurrency
// limiting) and instruction/07-security/10-slowloris.md (timeouts, rate floor).

use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // bind IP + Port
    // REVIEW(nit): address and port are hardcoded, so the server can't be run
    // twice, moved to another port, or reached from another network namespace
    // without recompiling. Take it from an argument or environment variable.
    let listener = TcpListener::bind("127.0.0.1:5000").await?;

    loop {
        match listener.accept().await {
            // REVIEW(nit): the peer address is discarded. It's the one piece of
            // context that makes the error logs below traceable to a client —
            // see instruction/12-testing/05-debugging.md.
            // REVIEW(low): Nagle's algorithm is still on for accepted sockets.
            // With many small echoes back to back, Nagle plus the client's
            // delayed ACK can add tens of milliseconds per message. Measure
            // before and after deciding — see instruction/01-network/08-tcp.md.
            Ok((mut socket, _addr)) => {
                tokio::spawn(async move {
                    // REVIEW(low): 1 KB means a 10 MB transfer costs ~10k reads
                    // plus ~10k writes (check with `strace -c`), while a larger
                    // buffer is held inside every idle connection's task. Either
                    // can be right — pick a size deliberately and write down why.
                    let mut buf = [0; 1024];

                    // In a loop, read data from the socket and write the data back.
                    loop {
                        let n = match socket.read(&mut buf).await {
                            // socket closed
                            Ok(0) => return,
                            Ok(n) => n,
                            Err(e) => {
                                // REVIEW(low): a client resetting the connection
                                // (ECONNRESET) is normal behavior, not a server
                                // failure, yet it's logged the same way as a real
                                // error. Under load this buries the lines that
                                // matter. Classify by error kind before logging.
                                eprintln!("failed to read from socket; err = {:?}", e);
                                return;
                            }
                        };

                        // Write the data back
                        // REVIEW(low): same as the read path — ECONNRESET and
                        // BrokenPipe here mean the client went away, and were
                        // logged as failures during the checklist run.
                        if let Err(e) = socket.write_all(&buf[0..n]).await {
                            eprintln!("failed to write to socket; err = {:?}", e);
                            return;
                        }
                    }
                });
            }
            Err(e) => {
                // REVIEW(low): while fds are exhausted this logs on every retry —
                // 295 identical lines in 30 seconds during the ulimit test, i.e.
                // the log floods exactly while the incident is happening. Log
                // the transition into and out of the exhausted state instead.
                eprintln!("accept error: {:?}", e);

                // REVIEW(low): only EMFILE/ENFILE back off. ENOBUFS and ENOMEM
                // are also persistent resource exhaustion, and they fall through
                // to the immediate `continue` below — the same busy-spin this
                // branch exists to prevent — see instruction/07-security/09-ddos.md.
                // REVIEW(nit): `condition` doesn't say what it means; name it
                // after the question it answers.
                let condition = match e.raw_os_error() {
                    // libc::EMFILE: Per-process limit reached (Process hết FD)
                    // libc::ENFILE: System-wide limit reached (Toàn hệ thống hết FD)
                    Some(code) => code == libc::EMFILE || code == libc::ENFILE,
                    None => false,
                };

                if condition {
                    tokio::time::sleep(Duration::from_millis(100)).await;
                } else {
                    // REVIEW(nit): redundant — the loop continues anyway.
                    continue;
                }
            }
        }
    }
}
