# Upstream Pool

## What to learn
### Representing an upstream
An upstream is more than a socket address — it needs health state, active
connection count, and (optionally) a weight. Model it as a struct behind an
`Arc` so the load balancer and health checker can share it without cloning
the whole pool on every request.

```rust
struct Upstream {
    addr: std::net::SocketAddr,
    weight: u32,
    healthy: std::sync::atomic::AtomicBool,
    active_conns: std::sync::atomic::AtomicUsize,
}

struct UpstreamPool {
    upstreams: Vec<std::sync::Arc<Upstream>>,
}
```
Gotcha: don't wrap the whole `Vec` in a `Mutex` if you only need to flip a
health flag — a `Mutex<Vec<Upstream>>` serializes every request's upstream
pick behind one lock. Use atomics per-upstream and `arc-swap`/`RwLock` only
for pool membership changes (see `service-discovery.md`).

### Connection reuse to upstreams
Opening a fresh TCP (+ TLS) connection per proxied request is expensive.
Keep a small per-upstream connection pool and reuse idle connections
(`hyper-util`'s `client-legacy` pool does this for you, but you should know
why it exists). Track HTTP/1.1 keep-alive vs HTTP/2 multiplexing separately
— HTTP/2 needs far fewer pooled connections per upstream since one
connection carries many concurrent streams.

Gotcha: a "dead" pooled connection (upstream closed it, you haven't noticed
yet) causes the next request on it to fail. Either probe on checkout or
retry once on a fresh connection when a pooled one errors immediately.

### Weighting and marking upstreams down
Weight biases how often an upstream is chosen relative to others (a bigger
box gets proportionally more traffic). Marking an upstream "down" must be
cheap and lock-light since it happens on the hot path when a request fails,
and must be visible to the load balancer's next pick immediately.

## Practice
1. In `labs/05-reverse-proxy`, define an `Upstream`/`UpstreamPool`
   type as above; hardcode 2-3 static upstream addresses to start.
2. Add an atomic `active_conns` counter, incremented/decremented around each
   proxied request; expose it later to `08-observability/metrics.md`.
3. Wire in connection reuse via `hyper_util::client::legacy::Client` and
   confirm (with logging) that a second request to the same upstream reuses
   a connection instead of opening a new one.
4. Simulate an upstream going down (kill the backend process) and confirm
   the pool marks it unhealthy without a request thread blocking on it.
