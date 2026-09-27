# Runtime Configuration and Flavors

## What to learn

### `multi_thread` vs. `current_thread`
`#[tokio::main]` defaults to the multi-threaded work-stealing runtime from [`01-tokio.md`](01-tokio.md), with worker count defaulting to the number of CPU cores. `#[tokio::main(flavor = "current_thread")]` runs everything on the single calling thread, with no work-stealing at all. A proxy almost always wants `multi_thread` in production (it spreads connections across cores), but `current_thread` is genuinely useful for tests and tools where deterministic, single-threaded execution is worth more than throughput.

```rust
#[tokio::main(worker_threads = 4)] // explicit override; default is num_cpus
async fn main() { /* ... */ }

#[tokio::test] // defaults to current_thread — fast to start, simpler to reason about
async fn handles_one_request() { /* ... */ }
```

### `LocalSet` and `!Send` futures
Every future spawned with `tokio::spawn` must be `Send`, because the scheduler may move it between worker threads. `tokio::task::LocalSet` (paired with a `current_thread` runtime, or entered via `LocalSet::run_until`) lets you `spawn_local` a future that is *not* `Send` — useful when wrapping a non-thread-safe C library via FFI ([`03-rust/14-ffi-and-abi.md`](../03-rust/14-ffi-and-abi.md)), or reusing an `Rc<RefCell<_>>`-based structure without paying for `Arc<Mutex<_>>`.

```rust
let local = tokio::task::LocalSet::new();
local.run_until(async {
    tokio::task::spawn_local(async { /* !Send future, fine here */ }).await.unwrap();
}).await;
```
Gotcha: this only works because everything stays on one thread. Reaching for `LocalSet` just to silence a `Send` error, without understanding why the future wasn't `Send` in the first place, usually means there's a hidden `Rc`/`RefCell` that should have been `Arc`/`Mutex` instead.

### Sizing the blocking pool
`spawn_blocking` ([`04-runtime/01-tokio.md`](01-tokio.md)) runs work on a separate thread pool sized by `max_blocking_threads` (default 512) — generous, because blocking threads mostly sit idle waiting on I/O or a mutex rather than burning CPU, so having many is cheap. This is unrelated to `worker_threads`, which should stay close to the core count since those threads are meant to be CPU-busy; conflating the two — cranking `worker_threads` well past core count "for more parallelism" — makes context-switching overhead worse, not better.

### Observing a running runtime: tokio-console and metrics
`tokio-console` (a TUI connected via the `console-subscriber` crate) shows live task counts and poll durations, and makes a blocking-call-in-async-code bug ([`01-tokio.md`](01-tokio.md)'s central failure mode) visible directly instead of inferred from symptoms. `tokio::runtime::Handle::metrics()` (a stable subset, more under `tokio_unstable`) gives programmatic access to worker steal counts, queue depths, and busy time — the same data [`08-observability/02-metrics.md`](../08-observability/02-metrics.md) would want exported as Prometheus gauges for a running proxy.

### Choosing `worker_threads` deliberately
Fewer workers than cores under-utilizes hardware; more workers than cores adds scheduling overhead with nowhere for the extra threads to run truly concurrently. The one common deliberate exception: reserving a core for something else — a dedicated metrics-scrape thread, or headroom on a shared host — by setting `worker_threads` to cores-minus-one. For the formal reason more workers stops helping — Amdahl's Law's serial-fraction ceiling, and why a proxy's workload is closer to Gustafson's regime than Amdahl's — see [`22-theory/08-amdahls-law.md`](../22-theory/08-amdahls-law.md).

## Practice
1. Run [`labs/05-reverse-proxy`](../../labs/05-reverse-proxy) under both `flavor = "multi_thread"` (default) and `flavor = "current_thread"`, load-test both, and measure the throughput difference under concurrent connections.
2. Build a small example using `LocalSet` + `spawn_local` with `Rc<RefCell<_>>` shared state, and confirm the same code fails to compile with plain `tokio::spawn`.
3. Install `tokio-console`, wire `console-subscriber` into a lab crate, and deliberately call a blocking function inside an async handler — find it in the console by its poll duration.
4. Print `tokio::runtime::Handle::metrics()`'s worker steal counts for [`labs/05-reverse-proxy`](../../labs/05-reverse-proxy) under load, and connect a spike in steals to uneven connection distribution across workers.
5. Explain in your own words why setting `worker_threads` far above the core count would make [`proxy`](../../proxy) slower, not faster, under sustained load.
