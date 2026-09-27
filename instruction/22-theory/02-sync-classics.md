# Classical Synchronization Problems

## What to learn

### Producer-consumer (bounded buffer)
One or more producers add items to a fixed-size buffer, one or more consumers remove them; the buffer must never be written to when full or read from when empty. The classic textbook solution uses two counting semaphores (tracking empty slots and full slots) plus a mutex for the buffer itself — three primitives cooperating, which is exactly what a bounded `mpsc` channel ([`03-rust/11-concurrency-patterns.md`](../03-rust/11-concurrency-patterns.md)) gives you for free, already proven correct, instead of hand-rolled.

```rust
// tokio::sync::mpsc::channel(capacity) *is* a solved producer-consumer problem:
// bounded capacity = the buffer size, .send().await blocks when full,
// .recv().await blocks when empty — no manual semaphore juggling needed
let (tx, rx) = tokio::sync::mpsc::channel::<Job>(16);
```

### Readers-writers
Any number of readers may hold a resource concurrently, but a writer needs exclusive access — reads don't conflict with each other, only with writes. The classic problem is about *fairness*: a naive implementation can starve writers forever if readers keep arriving, or starve readers if writers are prioritized. `RwLock` ([`03-rust/04-sync.md`](../03-rust/04-sync.md)) is this problem solved for you, but its fairness policy is a real, consequential choice, not a detail.

Gotcha: Rust's `std::sync::RwLock` makes no fairness guarantee at all — a write-starved `RwLock` under constant read pressure is a real, silent failure mode. `parking_lot::RwLock` and `tokio::sync::RwLock` document their specific fairness policy; read it before assuming "readers-writers" automatically means "no starvation."

### Semaphores vs. mutexes, formally
A mutex is a binary semaphore with an ownership rule attached (only the thread that locked it may unlock it); a general counting semaphore has no such rule — any thread may increment it, and it starts at N rather than 1, modeling N interchangeable resource permits rather than one exclusively-owned resource. This is precisely why `tokio::sync::Semaphore` (used for the accept-rate limiter in [`07-security/09-ddos.md`](../07-security/09-ddos.md)) is the right primitive for "at most N concurrent things," and a `Mutex` is the wrong one — a mutex has no notion of "N permits," only "locked or not."

```rust
let permits = tokio::sync::Semaphore::new(100); // 100 interchangeable permits, not 1 owned resource
```

### The barrier problem
A barrier makes every participating thread wait until *all* threads have reached the barrier point before any may proceed — useful for a fixed round of parallel work that must fully complete before the next round starts. `std::sync::Barrier` implements this directly; it shows up rarely in a request-serving proxy (which has no natural "rounds") but is common in batch/warm-up code — parallel cache pre-warming across N shards, waiting for all shards before serving traffic.

### Why these are worth knowing even with channels available
Every one of these classical problems is "solved" in the sense that a library primitive already exists for it, but recognizing *which* classical shape a real problem has is what tells you which primitive is actually correct. Reaching for a `Mutex` where the real shape is readers-writers, or a plain channel where the real shape needs a semaphore's permit-counting, produces code that technically works but has the wrong performance or fairness characteristics under load.

## Practice
1. Implement bounded producer-consumer by hand with two `std::sync::Condvar`s (or `tokio::sync::Notify`) instead of a channel, to feel what the channel does for you for free; then swap it for `tokio::sync::mpsc` and delete the hand-rolled version.
2. Write a readers-writers test that starves writers on purpose (many overlapping readers, one writer) using `std::sync::RwLock`; then switch to `parking_lot::RwLock` or `tokio::sync::RwLock` and compare fairness behavior.
3. Replace a `Mutex<usize>` used as a crude concurrency counter somewhere in your workspace with a `tokio::sync::Semaphore`, and explain in one sentence why the semaphore version is the more correct model.
4. Implement a parallel cache-warming step across N simulated shards using `std::sync::Barrier` (or `tokio::sync::Barrier`), and confirm no shard starts serving until all N have finished warming.
5. For one real synchronization point in [`proxy`](../../proxy) (or a [`labs/`](../../labs) crate), name which classical problem it actually is — bounded buffer, readers-writers, or N-permits — and confirm the primitive you used matches.
