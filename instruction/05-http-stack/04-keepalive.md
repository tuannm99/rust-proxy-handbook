# HTTP Keep-Alive & Connection Reuse

## What to learn

### Persistent connections
In HTTP/1.0, every request opened a new TCP connection by default — expensive given the TCP handshake (`01-network/02-tcp.md`) and, for HTTPS, a full TLS handshake too (`01-network/07-tls.md`). HTTP/1.1 makes connections persistent by default: after a response, the same connection stays open for the next request unless either side sends `Connection: close`.

### Hop-by-hop headers: what a proxy must strip
Keep-alive state is per connection, and so is a whole set of headers that
describe it — `Connection`, `Keep-Alive`, `Transfer-Encoding`, `TE`,
`Trailer`, `Upgrade`, `Proxy-*`. A proxy terminates one connection and
opens another, so it must regenerate these rather than forward them;
forwarding `Transfer-Encoding` in particular is one of the standard ways
to manufacture a request-smuggling vulnerability
(`07-security/05-request-smuggling.md`).

See `05-http-stack/02-hop-by-hop-headers.md` — including the part where
`Connection` names *additional* headers to strip, and why the strip has to
happen before your own trusted headers are applied.

### Pipelining (and why it's effectively dead)
Pipelining means sending multiple requests on a connection without waiting for each response — allowed by the spec but responses must still come back strictly in order (head-of-line blocking), and a single misbehaving intermediary in the path can corrupt the stream. Essentially no production HTTP/1.1 client pipelines anymore; HTTP/2's multiplexed streams (`01-network/05-http2.md`) solve the same problem correctly instead.

Gotcha: "no client pipelines" is not a reason for your *server* side to
mishandle it. If bytes for a second request arrive while you're still
responding to the first, the safe behaviors are to handle them in order or
to close the connection — never to interleave responses, and never to
discard buffered bytes silently. A parser that drops them is how a
smuggled request disappears from your logs while still reaching the
upstream.

### Connection pooling to upstreams
A reverse proxy talking to upstreams (`06-proxy/01-upstream.md`) should reuse connections rather than opening a new one per client request — pool a set of idle-but-open connections per upstream, hand one out per request, return it to the pool when the response completes (only if the response was framed unambiguously and the connection wasn't marked `close`).

```rust
// sketch: a bounded per-upstream connection pool
struct Pool {
    idle: VecDeque<Connection>,
    max_idle: usize,
    max_idle_time: Duration, // evict connections idle longer than this
}
```

Pool sizing, the dead-connection race, and when retrying on a fresh
connection is safe are covered in depth in `06-proxy/01-upstream.md` — this
file is about the connection *lifecycle* on both sides of the proxy.

Gotcha: never return a connection to the pool whose response framing you
weren't certain about — an unexpected EOF, a length mismatch, a parse
anomaly. Whatever is left in that connection's buffer becomes the next
request's prefix, which is exactly the desync in
`07-security/05-request-smuggling.md`. When in doubt, close it; a discarded
connection costs one handshake, a poisoned one costs a security incident.

### Idle timeout tuning
Two independent timeouts matter: how long the proxy keeps a client connection open with no requests (too long wastes fds/memory on idle clients; too short causes needless reconnects), and how long it keeps a pooled upstream connection open with no traffic (must be shorter than the upstream's own idle timeout, or the proxy will hand out a connection the upstream already silently closed — a classic "connection reset" bug).

Gotcha: the client-side idle timeout must not apply to connections that
are legitimately long-lived and quiet by design — an idle WebSocket
(`05-http-stack/09-websocket.md`), a server-streaming gRPC call
(`05-http-stack/10-grpc.md`), an SSE stream. Applying a 60-second
"no new request" timeout to those kills working connections on a timer,
and the resulting bug reports ("it disconnects every minute") are a
well-worn genre. Timeouts must be per connection *mode*, not global.

### Bounding connection lifetime, not just idleness
A connection that stays busy never hits an idle timeout and can live
forever — which causes two problems worth a deliberate `max_requests`
and/or max-lifetime cap:
- **It never rebalances.** Scale the upstream pool out
  (`06-proxy/07-service-discovery.md`) and existing long-lived connections
  keep going to the hosts they were pinned to; the new capacity gets only
  new connections. A lifetime cap is what eventually redistributes load.
- **Per-connection state accumulates.** Buffers grow to the largest
  request ever seen on that connection, allocator arenas fragment
  (`14-memory/06-fragmentation.md`). Recycling connections periodically
  bounds it.

nginx spells these `keepalive_requests` and `keepalive_time`; both default
to finite values for exactly these reasons.

### Detecting a connection the peer already closed
TCP doesn't tell you a peer closed an idle connection until you try to use it (or a keepalive probe fires). A pool must handle "I checked out a connection but writing to it failed immediately" by retrying on a fresh connection rather than surfacing the error to the client — tie this to `06-proxy/05-retry.md`.

Gotcha: TCP keepalive (`SO_KEEPALIVE`) is not this. Its defaults are
measured in *hours* (`tcp_keepalive_time` is 7200 seconds on Linux), so
out of the box it detects nothing on the timescale you care about. It's
useful when tuned down to minutes for detecting dead peers behind a NAT or
firewall that dropped state silently — but it is not a substitute for
application-level idle timeouts or for handling a failed write on
checkout.

### Closing a connection without dropping a request
There's an unavoidable race: you decide to close an idle keep-alive
connection at the same moment the client sends a new request on it. The
client's request is already in flight; your `FIN` and their bytes cross on
the wire, and they see a reset for a request the server never processed.

The mitigation on the response path is to announce it in advance — send
`Connection: close` on the *last* response you intend to serve, so the
client knows not to reuse the connection rather than finding out by
failure. HTTP/2 solves it properly with `GOAWAY`, which names the last
stream ID the server will process, letting the client retry anything above
it safely (`01-network/05-http2.md`); this is also the mechanism graceful
shutdown depends on (`09-architecture/04-graceful-shutdown.md`).

Gotcha: clients still race, and some don't honor `Connection: close`
promptly. A request that fails on a *reused idle* connection with zero
bytes of response received is the one case where even a non-idempotent
retry is conventionally treated as safe — see `06-proxy/01-upstream.md` for
why that convention exists and where it stops being safe.

## Practice
Build these in order.

1. In `labs/02-http-server`, verify with `tcpdump` or connection logging
   that two sequential requests from one client reuse one TCP connection.
   **Done when** you see one handshake for two requests, and
   `Connection: close` produces a `FIN` after the response.
2. Work through `05-http-stack/02-hop-by-hop-headers.md`'s exercises. **Done
   when** hop-by-hop headers are stripped in one early stage and framing
   headers are regenerated from the body you actually send.
3. Add per-mode timeouts: an idle timeout for normal keep-alive
   connections that does *not* apply to upgraded or streaming ones.
   **Done when** an idle keep-alive connection is closed on schedule and
   an idle WebSocket on the same server survives indefinitely.
4. In `labs/05-reverse-proxy`, build the per-upstream pool with idle
   eviction. **Done when** pooled connections older than the threshold are
   closed, and the pool never exceeds its configured maximum under load.
5. Simulate the upstream silently closing a pooled idle connection (close
   it out of band, then send a request). **Done when** the proxy detects
   the failed write and retries on a fresh connection instead of
   surfacing an error to the client.
6. Add `max_requests` and max-lifetime caps on pooled connections. **Done
   when** adding a new upstream mid-load-test results in traffic shifting
   to it within the lifetime window, rather than only new connections
   finding it.
7. Add `Connection: close` on the final response before a planned close.
   **Done when** a test that repeatedly idles out connections under
   concurrent load produces zero client-visible errors — run it first
   without the announcement and count the failures, so you know the race
   is real.
8. Measure reuse's effect: compare pooled vs one-connection-per-request
   under `12-testing/01-load-testing.md`. **Done when** you have p50/p99
   numbers for both, over plain HTTP and TLS separately (the TLS gap is
   where the real win is).
