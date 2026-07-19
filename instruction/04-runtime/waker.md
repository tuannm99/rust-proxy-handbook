# Waker & Poll

How a `Future` tells the executor "poll me again."

## What to learn

### The `Future::poll` contract
`Future::poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output>` either returns `Poll::Ready(value)` or `Poll::Pending`. Returning `Pending` is a promise: *something* will call `cx.waker().wake()` once this future can make progress again. If nothing ever calls it, the task sleeps forever — a real and common bug class ("lost wakeup").

```rust
impl Future for MyTimer {
    type Output = ();
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        if self.deadline_reached() {
            Poll::Ready(())
        } else {
            self.register_waker(cx.waker().clone()); // someone must call .wake() later
            Poll::Pending
        }
    }
}
```

### `Waker`, `RawWaker`, and `std::task::Wake`
A `Waker` is a type-erased handle back to "reschedule this specific task." Historically you built one from a `RawWaker` + a hand-written vtable (clone/wake/wake_by_ref/drop as raw function pointers over an `Arc`-like pointer) — unsafe, easy to get wrong. `std::task::Wake` (stable since 1.68) wraps that unsafety: implement `wake(self: Arc<Self>)` on your task type and the standard library builds the vtable for you.

### Where wakeups actually come from
For I/O futures, the waker ends up stored in the reactor, keyed by the registered fd (see `04-runtime/tokio.md`); epoll reporting readiness is what triggers `.wake()`. For a hand-rolled future (a timer, a channel), *you* are responsible for calling `.wake()` at the right moment — e.g. a background thread that fires at the deadline, or the sending side of a channel waking the receiver.

### Poll-driven vs push-driven mental model
It helps to think of `poll` as "ask, don't tell": the executor asks a future "are you done yet?", and the future's only way to say "not yet, but I'll tell you when" is to stash the waker. Multiple `.poll()` calls with different wakers (e.g. across `select!` branches) must always wake the *latest* registered waker, not a stale one — this is a classic bug in hand-written futures.

## Practice
1. In `labs/mini-runtime`, implement a `Waker` for your executor via `std::task::Wake`: `wake()` should push the task id back onto a run queue (e.g. a `VecDeque` behind a `Mutex`, or an `mpsc` channel).
2. Write a `Delay` future by hand (store a `Instant` deadline) that spawns a background `std::thread` to sleep then call `.wake()`, and drive it to completion on your mini executor.
3. Deliberately implement a buggy future that drops the waker instead of storing it, run it, and observe the task never wakes — confirm you understand why.
4. Compare your hand-rolled waker to tokio's real one by running the same future under `#[tokio::main]` and under your `mini-runtime`.
