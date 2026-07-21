# HTTP Keep-Alive & Connection Reuse

## What to learn

### Persistent connections
In HTTP/1.0, every request opened a new TCP connection by default — expensive given the TCP handshake (`01-network/tcp.md`) and, for HTTPS, a full TLS handshake too (`01-network/tls.md`). HTTP/1.1 makes connections persistent by default: after a response, the same connection stays open for the next request unless either side sends `Connection: close`.

### Pipelining (and why it's effectively dead)
Pipelining means sending multiple requests on a connection without waiting for each response — allowed by the spec but responses must still come back strictly in order (head-of-line blocking), and a single misbehaving intermediary in the path can corrupt the stream. Essentially no production HTTP/1.1 client pipelines anymore; HTTP/2's multiplexed streams (`01-network/http2.md`) solve the same problem correctly instead.

### Connection pooling to upstreams
A reverse proxy talking to upstreams (`06-proxy/upstream.md`) should reuse connections rather than opening a new one per client request — pool a set of idle-but-open connections per upstream, hand one out per request, return it to the pool when the response completes (only if the response was framed unambiguously and the connection wasn't marked `close`).

```rust
// sketch: a bounded per-upstream connection pool
struct Pool {
    idle: VecDeque<Connection>,
    max_idle: usize,
    max_idle_time: Duration, // evict connections idle longer than this
}
```

### Idle timeout tuning
Two independent timeouts matter: how long the proxy keeps a client connection open with no requests (too long wastes fds/memory on idle clients; too short causes needless reconnects), and how long it keeps a pooled upstream connection open with no traffic (must be shorter than the upstream's own idle timeout, or the proxy will hand out a connection the upstream already silently closed — a classic "connection reset" bug).

### Detecting a connection the peer already closed
TCP doesn't tell you a peer closed an idle connection until you try to use it (or a keepalive probe fires). A pool must handle "I checked out a connection but writing to it failed immediately" by retrying on a fresh connection rather than surfacing the error to the client — tie this to `06-proxy/retry.md`.

## Practice
1. In `labs/02-http-server`, verify (with `tcpdump`/logging) that two sequential requests from the same client reuse one TCP connection, and that `Connection: close` ends it.
2. In `labs/05-reverse-proxy`, build a small per-upstream connection pool: check out an idle connection or open a new one, return it after a successful response.
3. Add an idle-timeout eviction task that closes pooled connections older than a configurable threshold.
4. Simulate the upstream silently closing a pooled idle connection (close it out-of-band) and verify the proxy detects the failed write and retries on a new connection instead of erroring out to the client.
5. Measure connection-reuse's effect on p99 latency under `12-testing/load-testing.md` by comparing pooled vs. one-connection-per-request.
