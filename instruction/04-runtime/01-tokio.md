# Tokio Internals

Reactor, executor, scheduler.

## What to learn

### Reactor
The reactor owns the OS event source (epoll on Linux, see `02-linux/01-epoll.md`) and turns readiness events into wakeups. Every `TcpStream`/`TcpListener` registers its fd with the reactor once; when epoll reports the fd readable, the reactor finds the `Waker` associated with the task blocked on that fd and calls `.wake()`. The reactor does not run your code — it only decides *when* a task deserves another `poll()`.

### Executor & work-stealing scheduler
Tokio's multi-threaded executor runs N worker threads, each with a local run queue, plus a global injection queue. Idle workers steal tasks from busy workers' queues instead of blocking, which keeps CPUs busy without a central lock on every schedule. `tokio::spawn` puts a task on the current worker's local queue; cheap, but it means a burst of spawns from one connection can starve other workers until the next steal.

```rust
#[tokio::main] // multi-threaded runtime, one worker per core by default
async fn main() {
    tokio::spawn(handle_connection(socket)); // scheduled on some worker, not necessarily this one
}
```

### Blocking work vs `spawn_blocking`
A `.await` point is a promise that the current poll won't block the OS thread. Anything that can block for real (sync file I/O, CPU-heavy compression, a blocking mutex, a call into a sync C library) must go through `tokio::task::spawn_blocking`, which runs it on a separate blocking-thread pool. Forgetting this is the single most common way to accidentally stall an entire worker thread — and every task scheduled on that worker — under load.

### Cooperative scheduling & starvation
Tokio budgets a number of polls per task before forcing a yield back to the scheduler (`tokio::task::coop`), so one task that's always immediately ready (e.g. a tight loop over an in-memory channel) can't monopolize a worker thread forever. This matters for a proxy: a hot upstream connection pumping data should not starve health-check tasks on the same worker.

### Why this matters for a proxy
A reverse proxy is fundamentally "read from one socket, write to another, repeat, times tens of thousands of connections." Tokio's job is to make that cheap: one task per connection (not one thread), non-blocking I/O multiplexed through a handful of OS threads, and a scheduler that keeps all cores busy. Getting the reactor/executor split wrong (e.g. blocking a worker thread) degrades every connection on that worker, not just the slow one.

## Practice
1. In the hand-rolled executor you'll build in `03-rust/05-async.md`'s exercise (a scratch project, not part of this workspace), poll a `Vec` of futures in a loop with a no-op waker, and observe it busy-spins instead of sleeping — this is *why* a real reactor + waker exist.
2. In `labs/00-tcp-server`, log which OS thread ID handles each connection (`std::thread::current().id()`) and confirm connections are spread across workers.
3. Deliberately call a blocking `std::thread::sleep` inside an async handler in `tcp-server` and observe other connections stall; fix it with `tokio::time::sleep` and again with `spawn_blocking`, and compare.
4. Read the tokio worker metrics (`tokio::runtime::Handle::metrics()`, requires `tokio_unstable` or the stable subset available) and print steal counts under concurrent load.
