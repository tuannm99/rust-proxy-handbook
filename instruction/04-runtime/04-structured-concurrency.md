# Structured Concurrency and Cancellation

## What to learn

### `JoinHandle`: a spawned task is not fire-and-forget
`tokio::spawn` returns a `JoinHandle<T>` — dropping it does *not* cancel the task (it keeps running detached), but you lose the ability to get its result or observe a panic (`03-rust/08-error-handling.md`'s "silently swallowed panic" gotcha lives here). `.abort()` on a `JoinHandle` cancels the task at its next `.await` point, and awaiting the handle afterward returns a `JoinError` you can inspect to tell "panicked" apart from "cancelled."

```rust
let handle = tokio::spawn(async { long_upstream_call().await });
tokio::time::sleep(std::time::Duration::from_secs(5)).await;
handle.abort();
match handle.await {
    Ok(v) => use_result(v),
    Err(e) if e.is_cancelled() => tracing::warn!("cancelled"),
    Err(e) => tracing::error!("panicked: {e}"),
}
```

### `JoinSet`: managing many tasks as one unit
Spawning N tasks and manually tracking N `JoinHandle`s to know when they're all done is exactly what `tokio::task::JoinSet` exists for — it owns a dynamic set of spawned tasks, and `.join_next().await` yields results as they complete, in completion order rather than spawn order. Dropping a `JoinSet` aborts every task still inside it, which is the structured-concurrency property plain detached `tokio::spawn` calls don't give you for free: a `JoinSet` going out of scope is a real cancellation boundary.

```rust
let mut set = tokio::task::JoinSet::new();
for upstream in pool.iter() {
    set.spawn(health_check(upstream.clone()));
}
while let Some(res) = set.join_next().await {
    handle_health_result(res);
}
// if this function returns early (e.g. via `?`), the JoinSet drops here and every
// still-running health check is aborted — no orphaned tasks left behind
```
This is the direct answer to `06-proxy/03-healthcheck.md`'s "who cancels the health checkers when the pool is torn down" question.

### `select!` and cancellation-safety, with real APIs
`03-rust/05-async.md` covers "drop is cancel" conceptually; in practice the question is: does an operation leave shared state consistent if dropped mid-flight? `tokio::sync::Mutex::lock().await` is cancellation-safe — dropping the future before it resolves just means the lock was never acquired, nothing to clean up. A hand-rolled protocol that sends a request header, then a body, across two separate `.await` points is *not* automatically cancellation-safe: cancellation between the two leaves the peer having received half a message. `tokio::select!`'s documentation maintains a list of which stdlib/tokio futures are cancellation-safe for exactly this reason — check it before relying on a "loses the race, gets dropped" branch for anything with a multi-step side effect.

### Task-local storage
`tokio::task_local!` gives each task its own instance of a value, accessible without threading it through every function call — the async equivalent of a thread-local, scoped to a spawned task rather than an OS thread (which wouldn't make sense here, since many tasks share a thread). The common real use is a per-request trace/span context (`08-observability/03-tracing.md`) or request ID that every log line inside that task's call graph should carry, without an explicit parameter everywhere.

```rust
tokio::task_local! {
    static REQUEST_ID: u64;
}
REQUEST_ID.scope(request_id, async move {
    handle_request().await // anything called from here can do REQUEST_ID.with(|id| ...)
}).await;
```

### Graceful shutdown as structured concurrency
`09-architecture/04-graceful-shutdown.md`'s core problem — stop accepting new connections, let in-flight ones finish, then exit — is a cancellation/completion-tracking problem at heart: a `JoinSet`, or a `tokio::sync::watch` shutdown signal raced via `select!` inside each connection's loop, is the concrete mechanism this design pattern compiles down to.

## Practice
1. Spawn a task, drop its `JoinHandle` immediately, and confirm — via a log line inside the task — that it still runs to completion, detached rather than cancelled.
2. Rebuild the same task with `.abort()` called partway through, confirm it stops at its next `.await` point, and branch on `.is_cancelled()` on the resulting `JoinError`.
3. Replace a manual `Vec<JoinHandle>` in `06-proxy/03-healthcheck.md`'s exercise with a `JoinSet`, then prove the structured-cancellation property: return early from the owning function and confirm (via a per-task log line on drop) that in-flight checks are aborted.
4. Write a hand-rolled two-`.await`-point "protocol" (send header, then body, each behind a `tokio::time::sleep` standing in for real I/O), race it against a short timeout with `select!`, and demonstrate the peer ends up with a half-sent message.
5. Wire `tokio::task_local!` for a request ID into `labs/05-reverse-proxy`, and confirm every log line anywhere in that request's call graph can read it without it being passed as a parameter.
