# Amdahl's Law and Parallel Speedup

## What to learn

### Amdahl's Law: the serial fraction is the ceiling
If a fraction `p` of a program's work can be parallelized and `(1-p)` must run serially, the maximum speedup from N processors is `1 / ((1-p) + p/N)`. As N grows without bound, speedup approaches `1/(1-p)` — a hard ceiling set entirely by the serial fraction, no matter how many cores are thrown at it.

```text
speedup(N) = 1 / ((1-p) + p/N)
p = 0.95 (95% parallelizable): speedup(unbounded N) = 1/0.05 = 20x, never more
p = 0.50: speedup(unbounded N) = 1/0.50 = 2x — half the work being serial caps you at 2x forever
```
This is the formal backing for a very concrete proxy question: "we doubled `worker_threads`, why didn't throughput double?" If any meaningful fraction of the request path serializes — a single global `Mutex` on a routing table, a single-threaded metrics aggregator, a shared connection-pool lock under heavy contention — that fraction sets the ceiling long before cores run out.

### Finding the serial fraction in practice
Amdahl's Law is only useful once `p` is known, and in a real system that means profiling ([`08-observability/04-profiling.md`](../08-observability/04-profiling.md)), not guessing — measure wall-clock time spent inside any single-threaded or lock-serialized section under load, divide by total time, and that is `(1-p)`. A proxy with a single `Mutex<Vec<Upstream>>` load-balancer state hit on every request has a serial fraction directly proportional to how contended that lock is — [`03-rust/04-sync.md`](../03-rust/04-sync.md)'s advice to reach for `RwLock` or `watch` instead of a naive `Mutex` is, in Amdahl's terms, a direct attack on `(1-p)`.

### Gustafson's Law: the optimistic reframing
Amdahl's Law assumes a *fixed* problem size and asks how much faster it can finish; Gustafson's Law instead assumes a *fixed time budget* and asks how much *more work* fits in it as N grows — under this framing, speedup scales closer to linearly with N, because the serial portion doesn't grow with problem size the way the parallel portion does. Neither law is "more correct" — they answer different questions. Amdahl's is the right lens for "make this one request faster"; Gustafson's is the right lens for "serve more concurrent requests with more cores," which is the actual shape of a proxy's scaling problem.

```text
Amdahl:    fixed work, more cores -> less time (bounded speedup)
Gustafson: fixed time, more cores -> more work done (near-linear scaling)
```

### Why this favors tokio's work-stealing model for a proxy specifically
A proxy's workload — many independent requests, each mostly I/O-bound — has an inherently small serial fraction *if* global contention points are avoided, which is exactly Gustafson's regime: adding cores lets more concurrent connections be served, not any single request go faster (a single request's latency is dominated by network RTT and upstream time, not CPU parallelism). This is the quantitative reason [`04-runtime/01-tokio.md`](../04-runtime/01-tokio.md)'s work-stealing scheduler targets throughput under concurrency rather than parallelizing one request's own work across cores.

### Where the serial fraction actually hides in a proxy
The classic offenders: a single-threaded logger or metrics aggregator every request funnels through ([`08-observability/01-logging.md`](../08-observability/01-logging.md)), a global rate-limiter counter ([`07-security/07-ratelimit.md`](../07-security/07-ratelimit.md) — mitigated by sharding the counter, not by a faster lock), a config snapshot behind a `Mutex` instead of `ArcSwap`/`watch` ([`09-architecture/03-config.md`](../09-architecture/03-config.md)), and TLS session-cache contention on a single shared cache. Each is a small serial fraction individually, but serial fractions add: a 2% serial fraction alone caps speedup at 50x, while three independent 2% sections make `(1-p) = 6%` and cap it near 17x — far worse than any one of them suggests in isolation. Measure the combined effect instead of dismissing each as negligible.

## Practice
1. Profile [`labs/05-reverse-proxy`](../../labs/05-reverse-proxy) under increasing `worker_threads` (1, 2, 4, 8) and plot throughput; fit the curve against Amdahl's formula to estimate the actual serial fraction `(1-p)`.
2. Deliberately introduce a global `Mutex` on the hot path (wrap the upstream pool lookup in one shared lock even where `RwLock` would do), re-measure, and confirm the throughput ceiling drops in a way consistent with Amdahl's Law.
3. Replace that `Mutex` with `RwLock` or a sharded/lock-free structure, re-measure, and quantify how much of the ceiling was recovered.
4. Using `tokio::runtime::Handle::metrics()` ([`04-runtime/03-runtime-config.md`](../04-runtime/03-runtime-config.md)), find one component of [`proxy`](../../proxy) (or a [`labs/`](../../labs) crate) that serializes all requests through one point, and estimate its contribution to the overall serial fraction.
5. Write one paragraph explaining, in Gustafson's terms rather than Amdahl's, why adding cores to a proxy host is expected to raise sustainable concurrent-connection count more reliably than it lowers any single request's latency.
