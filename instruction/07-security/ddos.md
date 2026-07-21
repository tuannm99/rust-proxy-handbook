# DDoS & Volumetric Mitigation

The layer below `ratelimit.md` and `waf.md`: attacks that try to exhaust connections or bandwidth before any request is even parsed.

## What to learn
### Where this differs from rate limiting and WAF
`ratelimit.md` and `waf.md` operate on parsed HTTP requests — they assume the connection is already accepted and the proxy is reading bytes off it. A volumetric or connection-exhaustion attack (SYN flood, a flood of legitimate-looking connection attempts, slow-client attacks) tries to win *before* that point, by exhausting file descriptors, memory, or CPU on the accept path itself. Defenses here have to be cheaper per-attempt than an HTTP-layer rate limiter, because you can't afford to fully parse a request just to reject it.

### SYN floods are usually not the proxy's problem to solve
A SYN flood is answered by the kernel's SYN cookie mechanism (`net.ipv4.tcp_syncookies`) or by infrastructure in front of the proxy (a cloud provider's L3/L4 DDoS scrubbing, an anycast edge). A single Rust process cannot out-scale a real volumetric flood — trying to handle it entirely in application code is the wrong layer to fight at. The proxy's job is defense-in-depth for what actually reaches it as an established connection, not replacing a scrubbing layer.

### Accept-rate limiting: the proxy's actual line of defense
What the proxy *can* police cheaply: how fast it accepts new connections and how many it holds concurrently, independent of any per-client HTTP rate limit. A semaphore around the accept loop caps concurrent connections in flight; a token bucket on `accept()` itself caps the rate of new connections, rejecting (closing immediately) once either limit is hit — far cheaper than reading any bytes from the client first.

```rust
use tokio::sync::Semaphore;
use std::sync::Arc;

async fn accept_loop(listener: tokio::net::TcpListener, max_inflight: usize) {
    let permits = Arc::new(Semaphore::new(max_inflight));
    loop {
        let (socket, _addr) = match listener.accept().await {
            Ok(pair) => pair,
            Err(_) => continue,
        };
        let permits = permits.clone();
        match permits.clone().try_acquire_owned() {
            Ok(permit) => {
                tokio::spawn(async move {
                    let _permit = permit; // held until the connection task ends
                    handle_connection(socket).await;
                });
            }
            Err(_) => drop(socket), // over capacity: reject immediately, don't queue
        }
    }
}
```
Gotcha: rejecting by dropping the socket immediately is intentional — queuing rejected connections (or worse, doing any parsing before rejecting) just moves the exhaustion point from file descriptors to whatever resource the queue consumes.

### Slow-client attacks (Slowloris) need a data-rate floor, not just a total timeout
An attacker that sends one byte every 10 seconds can hold a connection open indefinitely under a naive "no total timeout" policy. The fix is a *minimum data rate* enforced on the read side — e.g. "must receive at least N bytes within T seconds of connecting, and again after every read" — not just a fixed overall deadline, which a well-paced slow client can dodge. Tie the header-read phase specifically to this: `05-http-stack/parser.md` and `labs/01-http-parser` are where a request actually gets slow-fed one byte at a time.

## Practice
1. In `proxy`, add an accept-rate/concurrency limiter (semaphore or token bucket) around the accept loop; log and metric-count rejections separately from application-level 429s from `ratelimit.md`.
2. Add a minimum-data-rate timeout around header parsing (using `labs/01-http-parser`'s parser or hyper's equivalent) that kills a connection sending data too slowly to ever complete a request.
3. Write a simple Slowloris-style test client (send 1 byte every few seconds to many connections) and confirm the proxy sheds these connections without needing operator intervention.
4. Load-test normal traffic (`12-testing/load-testing.md`) *while* the slow-client test runs, and confirm legitimate requests aren't starved by connections stuck in the accept/read path.
5. Write down, in the crate's README or a comment, which attack classes `proxy` can mitigate itself (slow clients, connection floods within its own resource limits) vs which require infrastructure in front of it (real SYN floods, volumetric bandwidth floods) — this boundary is the actual design decision, not a detail to skip.
