# Async Runtime

Phase 4. How tokio turns [`02-linux/07-epoll.md`](../02-linux/07-epoll.md)'s readiness notifications and
[`03-rust/05-async.md`](../03-rust/05-async.md)'s state machines into a working scheduler — and what
that means for code you write on top of it.

## Files

- [`01-tokio.md`](01-tokio.md) — the multi-threaded scheduler, work stealing, `spawn`, blocking-pool offload
- [`02-waker.md`](02-waker.md) — `Poll::Pending`, wakers, what `.await` actually registers
- [`03-runtime-config.md`](03-runtime-config.md) — `multi_thread` vs `current_thread`, `LocalSet`/`spawn_local`, blocking-pool sizing, `tokio-console`
- [`04-structured-concurrency.md`](04-structured-concurrency.md) — `JoinHandle`/`JoinSet`, cancellation-safety with real APIs, `task_local!`
- [`05-runtime-comparisons.md`](05-runtime-comparisons.md) — work-stealing vs thread-per-core (`glommio`/`monoio`), and why [`proxy`](../../proxy) picked tokio

## Where it goes next

Everything from [`05-http-stack/`](../05-http-stack) onward runs on this. The two failure
modes to carry forward: blocking a worker thread stalls every connection
multiplexed on it (see [`08-observability/04-profiling.md`](../08-observability/04-profiling.md) and
`tokio-console` for finding it), and a dropped future is a cancelled
operation, which [`06-proxy/01-upstream.md`](../06-proxy/01-upstream.md) turns into a concrete bug —
[`04-structured-concurrency.md`](04-structured-concurrency.md) is where that turns into a `JoinSet`-based
fix rather than just a conceptual warning. [`09-architecture/04-graceful-shutdown.md`](../09-architecture/04-graceful-shutdown.md)
leans directly on [`04-structured-concurrency.md`](04-structured-concurrency.md)'s cancellation
machinery, and [`05-runtime-comparisons.md`](05-runtime-comparisons.md) is the pointer to reach for if
a specific hot path ever seems to outgrow tokio's default model.
