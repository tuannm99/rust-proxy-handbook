# WebSocket Proxying

## What to learn

### The Upgrade handshake
A WebSocket connection starts as a normal HTTP/1.1 GET request with `Upgrade: websocket`, `Connection: Upgrade`, and a `Sec-WebSocket-Key`. The server responds `101 Switching Protocols` with `Sec-WebSocket-Accept` computed from that key (SHA-1 + a fixed GUID, per RFC 6455 §1.3). After the `101`, the connection stops being HTTP entirely — it's now a raw framed byte stream over the same TCP (or TLS) connection.

```rust
// sketch: computing Sec-WebSocket-Accept
use sha1::{Sha1, Digest};
const WS_GUID: &str = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";
fn accept_key(client_key: &str) -> String {
    let mut hasher = Sha1::new();
    hasher.update(client_key.as_bytes());
    hasher.update(WS_GUID.as_bytes());
    base64::encode(hasher.finalize())
}
```

### Why a proxy can't treat this as request/response after the upgrade
Once the `101` is sent, both the router and any per-request middleware (auth, compression, caching) built around "one request in, one response out" no longer apply — there is no next request on this connection, just a bidirectional byte pipe. A reverse proxy must special-case Upgrade: after forwarding the handshake, it switches to relaying raw bytes both directions until either side closes.

### Framing basics
WebSocket frames have an opcode (text, binary, close, ping, pong, continuation), a payload length (with an extended-length encoding for large payloads), and a masking key for client→server frames (masking is mandatory client-side, forbidden server-side — an anti-cache-poisoning measure from the RFC). A proxy that only forwards bytes doesn't need to fully parse frames unless it needs to inspect/filter messages.

### Keepalive via ping/pong
Long-lived idle WebSocket connections look identical to a dead connection from the network's perspective (a NAT box or LB can silently drop them). Either side can send a `ping` frame; the other must reply `pong` with the same payload. A proxy relaying WebSocket traffic should either pass these through transparently or, if terminating and re-establishing two separate WebSocket legs, generate its own keepalive pings on each leg independently.

### Backpressure
Unlike normal request/response where `Content-Length` bounds the work, a WebSocket connection can have one side producing messages faster than the other can consume — the proxy sits in the middle of two independently-paced streams and needs bounded buffers (not unbounded queues) so a slow client can't cause unbounded memory growth on the proxy.

## Practice
1. In `labs/02-http-server`, implement the Upgrade handshake by hand (validate headers, compute `Sec-WebSocket-Accept`) without a WebSocket crate, to see the raw mechanics.
2. After sending `101`, take over the raw `TcpStream`/`Upgraded` connection and implement a trivial echo of WebSocket text frames (parse just enough framing to unmask and re-frame).
3. In `labs/05-reverse-proxy`, add pass-through WebSocket proxying: forward the handshake to the chosen upstream, then relay raw bytes bidirectionally with `tokio::io::copy_bidirectional`.
4. Add a bounded buffer/backpressure test: have a slow "client" reader and a fast upstream writer, and verify proxy memory use stays bounded rather than growing unbounded.
5. Add ping/pong keepalive on an idle connection and verify the proxy either passes it through correctly or terminates the connection after a missed pong.
