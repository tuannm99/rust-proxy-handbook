# HTTP/2

Frames, streams, HPACK, multiplexing.

## What to learn

### Frames and the binary framing layer
HTTP/2 replaces the text-based /1.1 wire format with typed, length-prefixed
binary frames (`HEADERS`, `DATA`, `SETTINGS`, `WINDOW_UPDATE`, `RST_STREAM`,
`GOAWAY`, ...) multiplexed over a single TCP connection. Every frame
belongs to a stream ID (0 = connection-level). A proxy terminating HTTP/2
has to speak this frame layer explicitly — it's not "HTTP with fancier
headers", it's a genuinely different wire protocol.

### Streams and multiplexing
Many logical request/response exchanges (streams) run concurrently over
one TCP connection, each independently flow-controlled. This is what fixes
HTTP/1.1's need for 6 parallel TCP connections per origin. Gotcha:
multiplexing solves *connection-level* head-of-line blocking but not
*TCP-level* HOL blocking — one dropped TCP segment still stalls every
stream on that connection until it's retransmitted (this is exactly what
HTTP/3 over QUIC fixes, see `http3.md`).

### HPACK header compression
Headers are compressed with HPACK: a static table of common header
name/value pairs, a dynamic table built incrementally per connection, and
Huffman coding for literal values. Because the dynamic table is stateful
per-connection, a proxy that decodes HPACK on the client-facing side and
re-encodes for the upstream side must maintain two independent HPACK
states — you cannot just relay raw frames if headers are being modified.

### Flow control
Both stream-level and connection-level flow control use a credit-based
`WINDOW_UPDATE` mechanism — a sender must not send more `DATA` than the
receiver's advertised window. A proxy forwarding a large response body from
a fast upstream to a slow client must respect the client's flow control
window and apply backpressure to the upstream read, not buffer unboundedly.

### Where Rust fits
`h2` (used internally by `hyper` when the `http2` feature is enabled) is
the de facto HTTP/2 implementation in the Rust ecosystem; `hyper-util`'s
auto server builder in `milestones/02-http`/`milestone-03-reverse-proxy`
negotiates HTTP/1.1 vs HTTP/2 via ALPN (see `tls.md`) so you get this "for
free" once TLS is wired up, but you should still be able to explain what's
happening below that abstraction.

## Practice

1. Capture an HTTP/2 connection with Wireshark (or `nghttp -v`) and
   identify at least 4 distinct frame types on the wire.
2. Enable the `http2` feature on the hyper server in
   `milestones/02-http` and confirm via `curl --http2` that it
   negotiates HTTP/2 over TLS (ALPN).
3. Send two concurrent requests over the same HTTP/2 connection with
   `curl --http2 -v` and confirm both streams complete on one TCP
   connection (check with `ss` or `lsof`).
4. Read the `h2` crate's flow-control docs and explain, in your own words,
   what happens if your proxy's client-facing connection is HTTP/2 but the
   upstream connection is HTTP/1.1 — where does the flow-control mismatch
   get absorbed?
