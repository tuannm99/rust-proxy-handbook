# Concurrency Patterns: Channels and Message Passing

## What to learn

### Shared state vs. message passing
[`03-rust/04-sync.md`](04-sync.md) covers `Arc<Mutex<_>>`-style shared state: many tasks reading/writing the same memory under a lock. The alternative is message passing — tasks don't share memory, they send values through a channel, and only one task owns the data at a time. Rust's ownership model makes message passing unusually cheap to reason about: sending a value into a channel is a real move, so the sender loses the ability to touch it afterward by construction, not by convention.

### Tokio's channel zoo, and when to reach for each
- `mpsc` — many senders, one receiver; the default work queue (many connection handlers feeding one aggregator task).
- `oneshot` — exactly one value, exactly once; the standard way to get a *reply* back from a task you spawned (send a `oneshot::Sender` inside a request message, `.await` the matching `oneshot::Receiver` for the result).
- `broadcast` — one-to-many: every receiver gets every message (a config-reload signal fanned out to all connection handlers — see [`09-architecture/03-config.md`](../09-architecture/03-config.md)).
- `watch` — like broadcast but keeps only the *latest* value; a receiver that started late just sees current state, not a backlog (live health-check status, or the current config snapshot each request handler reads).

```rust
let (tx, rx) = tokio::sync::oneshot::channel();
tokio::spawn(async move {
    let result = do_upstream_call().await;
    let _ = tx.send(result); // ignore send error: receiver may have been dropped (caller cancelled)
});
let result = rx.await?;
```
Gotcha: sending on a bounded `mpsc` channel is itself an `.await` point that can block indefinitely if the receiver is slow or stuck — correct backpressure, but it means a bounded-channel send deserves the same cancellation-safety thinking as any other "blocked on a slow peer" operation (race it with `tokio::select!` and a timeout, same as [`03-rust/05-async.md`](05-async.md)'s cancellation discussion).

### The actor pattern
An "actor" is a task that owns some state exclusively and exposes it only through messages on a channel — no `Mutex` needed, because only that one task ever touches the state directly. This trades lock contention for message-passing overhead (a channel round trip plus one extra task hop), and is the natural shape for anything with invariants that are easy to violate under partial mutation: a rate limiter's sliding window ([`07-security/07-ratelimit.md`](../07-security/07-ratelimit.md)), a hot-reloadable config store ([`09-architecture/03-config.md`](../09-architecture/03-config.md)), a connection pool's free-list ([`06-proxy/01-upstream.md`](../06-proxy/01-upstream.md)).

```rust
enum PoolMsg { Acquire(tokio::sync::oneshot::Sender<Conn>), Release(Conn) }

async fn pool_actor(mut rx: tokio::sync::mpsc::Receiver<PoolMsg>) {
    let mut free: Vec<Conn> = Vec::new();
    while let Some(msg) = rx.recv().await {
        match msg {
            PoolMsg::Acquire(reply) => { if let Some(c) = free.pop() { let _ = reply.send(c); } }
            PoolMsg::Release(conn) => free.push(conn),
        }
    }
}
```

### Choosing shared-state vs. actor
If the critical section is short and contention is low, `Arc<Mutex<_>>` is simpler and usually faster — no extra task hop. If the state has non-trivial invariants that are easy to corrupt under partial mutation, or you want one clear serialization point that's easy to instrument, an actor is often worth the throughput cost. Neither is "the" idiomatic choice — a real proxy codebase uses both, per component.

## Practice
1. Build a request/reply round trip with `oneshot`: spawn a worker that does work and replies, have the caller await the reply, then drop the receiver early (simulating cancellation) and confirm the worker's `tx.send()` fails harmlessly instead of panicking.
2. Implement [`06-proxy/01-upstream.md`](../06-proxy/01-upstream.md)'s connection pool once as `Arc<Mutex<Vec<Conn>>>` and once as an actor behind an `mpsc` channel; load-test both and compare latency/throughput under contention.
3. Use `tokio::sync::watch` to fan out a config-reload signal to several running "handler" tasks; confirm a handler spawned *after* the last update still immediately sees the current value rather than waiting for the next change.
4. Reproduce the bounded-`mpsc`-backpressure gotcha: make the receiver artificially slow, send from multiple tasks, observe senders stall on `.send().await`, then race a send against a `tokio::time::timeout` to bound how long a caller waits.
5. Pick one real [`proxy`](../../proxy) component (rate limiter, connection pool, or config store) and write down which pattern you'd choose and why, citing the specific handbook file for that component.
