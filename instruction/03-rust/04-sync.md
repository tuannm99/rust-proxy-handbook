# Arc, Mutex, RwLock, Atomics

## What to learn

### `Arc<T>`: shared ownership across threads
`Arc` (atomic reference count) lets multiple owners share one heap
allocation safely across threads; cloning an `Arc` bumps a refcount, it
doesn't copy `T`. This is the standard way to share a proxy's upstream pool,
config snapshot, or connection registry across every connection-handling
task without copying it per-request.

```rust
let pool: Arc<UpstreamPool> = Arc::new(build_pool());
let pool2 = pool.clone(); // cheap: one atomic increment
tokio::spawn(async move { pool2.pick_upstream(); });
```

Gotcha: `Arc<T>` alone only gives shared *read* access (`&T`) — you need
interior mutability (`Mutex`/`RwLock`, or atomics) to actually mutate the
shared value, and `Arc<T>` requires `T: Send + Sync` to cross thread
boundaries, which the compiler checks for you.

### `Mutex<T>` vs `RwLock<T>`
`Mutex<T>` allows one thread exclusive access at a time (readers included);
`RwLock<T>` allows many concurrent readers OR one writer. For a proxy's
upstream health table — read on every request, written rarely by a
background health-checker — `RwLock` lets thousands of concurrent request
tasks read without blocking each other, which is exactly the access pattern
you want. Reach for `Mutex` when reads and writes are similarly frequent, or
when the simplicity matters more than read concurrency.

```rust
struct HealthTable(RwLock<HashMap<UpstreamId, bool>>);

impl HealthTable {
    fn is_healthy(&self, id: UpstreamId) -> bool {
        *self.0.read().unwrap().get(&id).unwrap_or(&false)
    }
}
```

Gotcha: holding a `std::sync::MutexGuard` across an `.await` point makes the
future `!Send` (the guard isn't `Send`, and it's still alive across the
await), which breaks `tokio::spawn`. Either use `tokio::sync::Mutex` (which
is safe to hold across `.await`, at the cost of being slower for
uncontended locks), or restructure so the guard is dropped before awaiting.

### Atomics and `Ordering`
Atomics (`AtomicU64`, `AtomicBool`, ...) give lock-free reads/writes of
single values — ideal for counters (active connections, requests-per-second)
on the hot path where a `Mutex` would be needless overhead. `Ordering`
controls what reordering the compiler/CPU may do around the atomic op:
`Relaxed` for independent counters, `Acquire`/`Release` when one atomic op
needs to establish a happens-before relationship with other memory (e.g.
publishing a pointer), `SeqCst` when you want the simplest-to-reason-about
(and slowest) total order.

```rust
static ACTIVE_CONNS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
ACTIVE_CONNS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
```

### Shared state vs message-passing
"Do not communicate by sharing memory; share memory by communicating."
Tokio's `mpsc`/`broadcast`/`watch` channels let you replace a shared
`Mutex<State>` with a single task owning the state and everyone else sending
it messages — no lock contention, no deadlock risk from lock ordering. Use
`tokio::sync::watch` in particular for "latest value, many readers" data
like a live-reloaded config (`09-architecture/03-config.md`); use shared
`RwLock`/`Arc` when the data is large and cloning it per update would be
wasteful (e.g. a big routing table).

Gotcha: under real load, lock contention on a naive `Mutex<Vec<Upstream>>`
shared by every request task is a common self-inflicted bottleneck — measure
before reaching for lock-free structures, but know `RwLock` and `watch` are
the first two escape hatches to reach for.

## Practice
1. Build a `HealthTable` as above with `Arc<RwLock<HashMap<...>>>`, spawn 8
   tasks reading it in a loop and 1 task writing to it every 100ms; confirm
   readers aren't blocked by each other.
2. Reproduce the "future cannot be sent between threads" error by holding a
   `std::sync::MutexGuard` across an `.await`, then fix it with
   `tokio::sync::Mutex` and again by restructuring to drop the guard first.
3. Replace a counter protected by `Mutex<u64>` with `AtomicU64` and confirm
   with a quick benchmark that it's faster under contention.
4. In `labs/05-reverse-proxy`, decide whether your upstream pool
   is `Arc<RwLock<Vec<Upstream>>>` or owned by one task and accessed via a
   `tokio::sync::watch` channel — implement one, and write a sentence on why
   you didn't pick the other.
5. Deliberately construct a two-lock deadlock (task A locks X then Y, task B
   locks Y then X) and then fix it by establishing a consistent lock order.
