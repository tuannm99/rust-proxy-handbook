# io_uring

Submission/completion queues.

## What to learn

### The completion-based model
epoll tells you "this fd is ready, now go call read/write yourself" — it's
still a readiness API. `io_uring` is a *completion* API: you submit an
operation (read, write, accept, ...) into a Submission Queue Entry (SQE), the
kernel performs it asynchronously, and a Completion Queue Entry (CQE) shows
up when it's done. Both queues are lock-free ring buffers shared via `mmap`
between kernel and userspace, so submitting/reaping work can avoid a syscall
entirely in the common case (`SQPOLL` mode).

### Why it beats epoll for some workloads
Epoll still costs one syscall per read/write plus one for `epoll_wait`.
io_uring can batch many operations into one `io_uring_enter` call, and file
I/O (unlike sockets) has no readiness notion at all under epoll — io_uring
is the first Linux API to give you real async file I/O. For an L7 proxy
that's mostly socket-to-socket, the win is smaller than for a
storage-heavy workload; io_uring pays off most when you're also serving
static files (`05-http-stack/static.md`) or doing heavy disk-backed caching.

### Rust crate landscape
- `io-uring`: thin, unsafe-ish bindings close to the raw ring layout — you
  build SQEs and reap CQEs yourself.
- `tokio-uring`: an alternative *runtime* (not a drop-in tokio feature) built
  entirely around io_uring; it is not fully interoperable with regular
  `tokio::net` types, which matters if you want to mix it into an existing
  tokio-based proxy.
- `glommio`: a thread-per-core, io_uring-native runtime, a different
  architectural bet than tokio's work-stealing model entirely.

```rust
// Rough shape of raw io-uring usage (crate `io-uring`):
// let mut ring = IoUring::new(256)?;
// let sqe = opcode::Read::new(fd, buf.as_mut_ptr(), buf.len() as _).build();
// unsafe { ring.submission().push(&sqe)?; }
// ring.submit_and_wait(1)?;
// let cqe = ring.completion().next().unwrap();
```

### When it's not worth it
Kernel version matters a lot: usable io_uring needs a fairly recent kernel
(5.11+ for solid networking support; earlier kernels had security holes that
got several distros to disable it by default). It also had real CVEs, and
some hardened environments (e.g. Docker/Kubernetes with seccomp profiles,
some cloud sandboxes) block it outright — a proxy that *requires* io_uring
may simply refuse to start there. For an L7 proxy whose bottleneck is
usually TLS handshakes, header parsing, and upstream connection reuse rather
than raw syscall count, epoll + tokio is almost always the pragmatic choice;
treat io_uring as an optimization to reach for after profiling shows syscall
overhead is actually your bottleneck, not a default.

## Practice
1. Read the `io-uring` crate docs and write a minimal program that reads a file with a single SQE/CQE round trip.
2. Extend it to submit multiple reads before reaping any completions, and observe the batching in `strace`.
3. Port `labs/epoll-echo` (or a copy of it) from epoll to `tokio-uring` and compare code complexity and behavior under connection churn.
4. Check `uname -r` on your dev machine and any target deployment environment; confirm whether io_uring is even available/enabled there.
5. Write down, in your own words, why `proxy` should default to tokio's epoll-based reactor rather than io_uring.
