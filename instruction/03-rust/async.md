# Async/Future

## What to learn

### The `Future` trait
An `async fn` is sugar for a function returning `impl Future<Output = T>`.
`Future` has one required method, `poll(self: Pin<&mut Self>, cx: &mut
Context) -> Poll<T>`, which returns `Poll::Ready(value)` or `Poll::Pending`.
Nothing runs until something calls `poll` — a `Future` on its own is an
inert state machine, not a scheduled unit of work. This is why an async
function body doesn't execute at all until it's `.await`ed or spawned.

```rust
async fn fetch(id: u64) -> Response { /* ... */ }

let fut = fetch(1); // nothing has run yet — fut is just a value
let resp = fut.await; // NOW it runs, possibly suspending at internal .await points
```

### async/await desugaring and state machines
The compiler turns an `async fn`'s body into an anonymous struct implementing
`Future`, with one enum variant per suspension point (`.await`) — each
variant holds exactly the local variables still needed after that point.
This is why the future's size is fixed at compile time and why locals held
across `.await` must satisfy whatever bounds the future needs (usually
`Send`, if you plan to `tokio::spawn` it) — see `03-rust/lifetimes.md` for
what breaks when a borrow is one of those held-across-await locals.

### Cooperative scheduling and blocking
Tokio's executor is cooperative: a task runs until it returns `Poll::Pending`
(usually because it's waiting on I/O) or finishes — it never gets
preempted mid-poll. Calling a *blocking* function (`std::thread::sleep`,
synchronous file I/O, a CPU-heavy loop) inside an async fn stalls the entire
worker thread, starving every other task scheduled on it. This is one of
the most common real-world tokio bugs in a proxy: one connection doing
synchronous DNS or disk I/O silently stalls unrelated connections sharing
that thread.

```rust
// WRONG in async code: blocks the whole worker thread
std::thread::sleep(std::time::Duration::from_secs(1));

// RIGHT: yields control back to the executor while waiting
tokio::time::sleep(std::time::Duration::from_secs(1)).await;

// For unavoidable blocking work (sync I/O, heavy CPU):
tokio::task::spawn_blocking(|| { /* do_blocking_work() */ });
```

### Cancellation: drop is cancel
Dropping a `Future` before it resolves cancels it immediately, at whatever
`.await` point it was suspended at — there is no cleanup callback beyond
normal `Drop`. `tokio::select!` and timeouts (`tokio::time::timeout`) rely
entirely on this: the "losing" branch's future is simply dropped. This means
async code must be written so that being dropped mid-operation never leaves
shared state inconsistent — e.g. a half-sent request to an upstream needs to
either fully commit or be safely abandoned, not leave a connection in limbo.

```rust
tokio::select! {
    resp = upstream_call() => handle(resp),
    _ = tokio::time::sleep(std::time::Duration::from_secs(5)) => handle_timeout(),
}
// if the timeout branch wins, upstream_call()'s future is dropped mid-flight
```

Gotcha: a dropped future does not mean the underlying OS-level operation
(e.g. an in-flight write syscall) is un-done — cancellation is cooperative at
the Rust level, not at the kernel level. See `06-proxy/retry.md` for what
this means for retry safety (idempotency).

## Practice
1. Read the desugared state-machine shape by hand: write a 2-`.await` async
   fn, then sketch the enum the compiler would generate for it (which
   locals live in which variant).
2. Reproduce a stalled-executor bug on purpose: call `std::thread::sleep`
   inside an async task on a single-worker-thread runtime while another
   task tries to make progress; observe the stall, then fix it with
   `tokio::time::sleep` or `spawn_blocking`.
3. Build a `tokio::select!` with a real operation racing a timeout; confirm
   via a `Drop` impl on a guard type that the losing branch is actually
   dropped/cancelled.
4. In a scratch project (not part of this workspace — there's no dedicated
   lab for a hand-rolled executor here), implement the `Future` trait by
   hand for a simple timer type (`poll` returns `Pending` until a deadline,
   `Ready(())` after), then drive it with your own executor instead of
   tokio's.
5. In that same scratch executor, implement a `Waker` (via `std::task::Wake`
   or `RawWakerVTable`) and a single run-queue executor that polls a task
   only when its waker is called — this is the mechanism `04-runtime/waker.md`
   and tokio's reactor both build on.
