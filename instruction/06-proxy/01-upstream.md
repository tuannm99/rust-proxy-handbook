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
for pool membership changes (see `07-service-discovery.md`).

`Ordering::Relaxed` is the right choice for `healthy` and `active_conns`
specifically because neither flag *publishes* other data — a reader only
needs the value itself, not a guarantee about what was written before it.
The moment a flag guards other fields ("healthy means `last_probe_result`
is valid"), `Relaxed` is wrong and you need `Release`/`Acquire`; see
`03-rust/04-sync.md`.

### Keeping `active_conns` honest under cancellation
The counter is only useful if it's exactly balanced, and the naive version
isn't:

```rust
upstream.active_conns.fetch_add(1, Ordering::Relaxed);
let resp = forward(&upstream, req).await;   // <-- if this future is dropped here...
upstream.active_conns.fetch_sub(1, Ordering::Relaxed);  // <-- ...this never runs
```

In an async proxy, a client disconnecting mid-request drops the task, and
a dropped future simply stops at its last `.await` — the decrement after
it never executes (`03-rust/05-async.md`: drop *is* cancel). Every cancelled
request permanently inflates the count. Least-connection balancing
(`02-load-balancer.md`) then routes *away* from a perfectly healthy upstream
forever, and the failure is silent: no error, no log, just steadily skewed
traffic that looks like an algorithm bug.

The fix is RAII — decrement in `Drop`, which runs on the cancellation path
too:

```rust
struct ConnGuard(std::sync::Arc<Upstream>);
impl Drop for ConnGuard {
    fn drop(&mut self) {
        self.0.active_conns.fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
    }
}
```
Gotcha: this is the same class of bug as an unreturned object-pool entry
(`14-memory/03-object-pool.md`) — any manual "increment, do work, decrement"
pair in async code is a leak waiting for its first cancellation.

### Connection reuse to upstreams
Opening a fresh TCP (+ TLS) connection per proxied request is expensive:
one RTT for the TCP handshake, one or two more for TLS
(`01-network/13-tls.md`), paid before a single request byte moves. Keep a
small per-upstream connection pool and reuse idle connections
(`hyper-util`'s `client-legacy` pool does this for you, but you should know
why it exists).

Track HTTP/1.1 keep-alive vs HTTP/2 multiplexing separately: an HTTP/1.1
connection carries exactly one request at a time, so N concurrent requests
to an upstream need N connections. An HTTP/2 connection carries many
concurrent streams, so the same N requests may need only one — bounded by
the upstream's advertised `SETTINGS_MAX_CONCURRENT_STREAMS`
(`01-network/11-http2.md`), past which new streams queue behind finished ones
rather than opening a second connection unless you explicitly allow it.

### Sizing the pool
Pool size is a concurrency limit in disguise: an HTTP/1.1 pool capped at
16 idle connections to an upstream means at most 16 in-flight requests to
it, and the 17th waits. Little's law gives the floor — required concurrency
= throughput × average latency, so 1000 req/s at 20ms is 20 concurrent
requests, and a 16-connection cap silently throttles you below your actual
capacity.

Sizing the other direction is equally wrong: an unbounded pool lets a
traffic spike open thousands of sockets to one upstream, and every one of
them consumes an fd on your side and a kernel socket buffer on both sides
(`16-kernel/03-tcp-stack.md`). The upstream's own accept backlog, not your
pool, then becomes the thing that fails.

Gotcha: idle timeout must be *shorter* than the upstream's own keep-alive
timeout, or you lose the race described next. If the upstream closes idle
connections after 60s, a 75s idle timeout on your side guarantees you
regularly hand out connections the upstream has already closed.

### The dead pooled connection, and when retrying it is safe
A pooled connection the upstream has already closed produces a failure on
the next request that uses it — often before a single response byte
arrives. This is not rare; it happens every time an upstream's keep-alive
timeout, a deploy, or a load balancer's idle reaper fires between two of
your requests.

The conventional rule: a failure on a *reused* connection, with **zero**
response bytes received, is treated as safe to retry once on a fresh
connection even for a non-idempotent method — the reasoning being that the
request almost certainly never reached the upstream's application code.
Note what "almost certainly" is doing there: the upstream may in fact have
read the request, acted on it, and died before responding, in which case
the retry duplicates a side effect. Browsers and most HTTP clients accept
that risk for reused connections; a payment proxy should not. Decide
deliberately, and see `05-retry.md` for the general idempotency rule this is
an exception to.

Gotcha: the retry-once-on-fresh-connection path must not consume the
retry budget from `05-retry.md` — it's a connection-liveness retry, not a
failure retry, and counting it against the budget means an upstream with
aggressive keep-alive reaping exhausts your budget during normal operation.

### Per-upstream timeouts
"Timeout" is at least three different numbers, and collapsing them into
one is a common source of both hung requests and spurious failures:
- **Connect timeout** — how long to wait for the TCP (+TLS) handshake.
  Should be short (hundreds of ms on a LAN); a slow connect means the
  upstream is unreachable or its backlog is full, not that it's working.
- **Read/idle timeout** — how long to wait for the *next* byte from the
  upstream. Guards against an upstream that accepted the connection and
  then stalled.
- **Total request timeout** — an upper bound on the whole exchange.
  Necessary because a malicious or broken upstream can trickle one byte
  just under the read timeout forever, keeping the request alive
  indefinitely (the upstream-side mirror of the Slowloris attack in
  `07-security/09-ddos.md`).

Gotcha: the total request timeout must account for streaming responses. A
30s total timeout silently breaks a legitimate large file download or an
SSE/long-poll endpoint — those need the read timeout (progress is being
made) without a total cap, which means routing them through a different
timeout policy rather than one global number.

### Weighting and marking upstreams down
Weight biases how often an upstream is chosen relative to others (a bigger
box gets proportionally more traffic). Marking an upstream "down" must be
cheap and lock-light since it happens on the hot path when a request fails,
and must be visible to the load balancer's next pick immediately.

Gotcha: weight and health interact badly if the balancer reads them
separately — it can pick a weight-10 upstream on the strength of its
weight, then discover it's unhealthy, and either fall back to a poor
second choice or (worse) retry the pick in a loop that spins when *all*
upstreams are unhealthy. Decide up front what "all upstreams down" does:
fail fast with 503, or send to the least-recently-failed one anyway. Both
are defensible; spinning is not.

## Practice
Build these in order — each step's exit criterion is what makes the next
one meaningful.

1. In `labs/05-reverse-proxy`, define `Upstream`/`UpstreamPool` as above
   with 2-3 hardcoded static addresses. **Done when** a request is
   forwarded to one of them and the response reaches the client unchanged.
2. Add `active_conns` with an RAII `ConnGuard`. **Done when** a test that
   cancels 1000 requests mid-flight (drop the client future, or kill the
   client) leaves `active_conns` at exactly 0 afterward — write the naive
   `fetch_sub`-after-await version first and watch it end at a nonzero
   number, so you've seen the bug before you fix it.
3. Wire in connection reuse via `hyper_util::client::legacy::Client`.
   **Done when** logs (or `ss -tan` against the upstream) show a second
   request reusing a connection rather than opening a new one.
4. Set the pool's idle timeout deliberately *longer* than a dummy
   upstream's keep-alive timeout and drive intermittent traffic. **Done
   when** you can reproduce the dead-pooled-connection failure on demand;
   then fix it both ways (shorter idle timeout, plus retry-once-on-fresh)
   and confirm it disappears.
5. Add the three timeouts as separate values. **Done when** an upstream
   that accepts and then stalls forever is failed by the read timeout, and
   an upstream that trickles one byte every second is failed by the total
   timeout — and a legitimate 100 MB streaming download is *not* failed by
   either.
6. Simulate an upstream going down (kill the backend). **Done when** the
   pool marks it unhealthy and no request thread blocks on it — measure
   p99 latency during the kill and confirm it doesn't spike past your
   connect timeout.
