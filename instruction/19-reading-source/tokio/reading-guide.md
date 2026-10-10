# Reading guide: tokio

A route through tokio's source, with questions to answer as you go. This
is a guide, not the notes: the per-project files listed in
[`00-README.md`](00-README.md) (`architecture.md`, `request-flow.md`, ...) are yours to
write from what you find. Answers aren't given here on purpose — finding
them in the code is the exercise.

Repository: `github.com/tokio-rs/tokio`, crate directory `tokio/src/`.
File paths below match tokio 1.x at the time of writing; code moves between
releases, so if a path is gone, search the repository for the type or
function named next to it.

## When to read which part

| Read | After | Why then |
| --- | --- | --- |
| Stop 1-2: the I/O driver | The raw-epoll exercise in [`02-linux/14-epoll.md`](../../02-linux/14-epoll.md) | Your own `epoll_wait` loop is the thing to compare against |
| Stop 3-4: scheduler and tasks | [`04-runtime/01-tokio.md`](../../04-runtime/01-tokio.md), [`04-runtime/02-waker.md`](../../04-runtime/02-waker.md), [`labs/00-tcp-server`](../../../labs/00-tcp-server) | You've spawned tasks and seen them spread across workers |
| Stop 5: blocking pool and coop | [`04-runtime/03-runtime-config.md`](../../04-runtime/03-runtime-config.md) | You've stalled a worker yourself and fixed it |
| Stop 6: channels | [`03-rust/11-concurrency-patterns.md`](../../03-rust/11-concurrency-patterns.md) | You've used `mpsc`/`oneshot` in a lab |

## The route

### Stop 1: from `TcpStream::read` down to readiness
Start at `net/tcp/stream.rs` and follow a read: `TcpStream` wraps a
`PollEvented` (`io/poll_evented.rs`), which talks to the I/O driver
(`runtime/io/driver.rs`, `runtime/io/registration.rs`,
`runtime/io/scheduled_io.rs`).
- Where does a read that would block turn into `Poll::Pending`?
- Where is the task's waker stored while it waits, and what data structure keys it to the socket?
- What does tokio do when the OS reports readiness but the read still returns `WouldBlock`?

### Stop 2: the driver loop itself
In `runtime/io/driver.rs`, find the function that blocks waiting for
events (it goes through `mio`, see [`19-reading-source/mio/`](../mio)).
- Compare it to your hand-written `epoll_wait` loop. What does tokio do that you didn't?
- Is it edge-triggered or level-triggered? Where in the code do you see the answer, and how does tokio avoid the missed-wakeup bug from [`02-linux/14-epoll.md`](../../02-linux/14-epoll.md)?

### Stop 3: the work-stealing scheduler
`runtime/scheduler/multi_thread/worker.rs` is the worker loop;
`runtime/scheduler/multi_thread/queue.rs` is the per-worker run queue;
the global inject queue lives under `runtime/scheduler/inject`.
- In what order does a worker look for its next task: local queue, global queue, stealing, the I/O driver?
- How many tasks does a steal take at once, and why not just one?
- What is the LIFO slot, and what workload does it optimize?

### Stop 4: what a task is
`runtime/task/` — start with `raw.rs`, `harness.rs`, `state.rs`, and
`join.rs`.
- How does a task's state word represent "running," "notified," "complete," and "cancelled" at once, and why is it a single atomic?
- What happens, step by step, when you call `JoinHandle::abort()` on a task that is currently running on another thread?
- Where does a panic inside a task get caught and turned into a `JoinError` ([`03-rust/08-error-handling.md`](../../03-rust/08-error-handling.md))?

### Stop 5: `spawn_blocking` and cooperative budgeting
The blocking pool is under `runtime/blocking/`. The cooperative budget is
the `coop` module (search for `coop` if it has moved).
- How does the blocking pool decide to start a new thread versus reuse an idle one, and when do idle threads exit?
- Where is a task's budget decremented, and what happens when it reaches zero in the middle of a busy loop over a channel?

### Stop 6: a channel end to end
`sync/mpsc/` — `bounded.rs`, `chan.rs`, and the block-linked list it uses.
- Why is the queue a linked list of fixed-size blocks rather than a ring buffer or a `VecDeque`?
- On a bounded channel, where does a full-channel `send().await` park, and what wakes it?

## What to write in your notes
After the route, write the per-project files from [`00-README.md`](00-README.md). At minimum,
`request-flow.md` should trace one `TcpStream::read` from your code down
to the syscall and back up to your task being polled again — in your own
words, with file names — and `what-to-learn.md` should list which
handbook files the code confirmed, contradicted, or went beyond.
