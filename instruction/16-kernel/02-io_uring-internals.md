# io_uring Internals

`02-linux/07-io_uring.md` covers using `io_uring` from the application side.
This file covers the submission/completion mechanics underneath that
make it different in kind from epoll, not just a faster version of it.

## What to learn

### Two ring buffers, shared with the kernel
`io_uring` sets up a **submission queue (SQ)** and **completion queue
(CQ)** as memory regions `mmap`'d into both the process and the kernel —
no copy is needed to hand a request to the kernel or to read a result
back, because both sides are reading and writing the same memory. A
submission queue entry (**SQE**) describes one operation: which syscall-
equivalent (`read`, `write`, `accept`, `connect`, ...), which fd, which
buffer, which offset. A completion queue entry (**CQE**) reports the
result once the kernel finishes it. Submitting many operations is
writing many SQEs into the ring, not making many syscalls.

### Why this is a different model than epoll, not just faster
epoll only ever tells you an fd is *ready* — the actual `read()`/`write()`
is still a normal, potentially blocking syscall you issue yourself
afterward. For file I/O in particular, "ready" isn't really well-defined
the way it is for a socket, which is why async file I/O under the epoll
model has always meant offloading to a blocking thread pool
(`03-rust/05-async.md`'s cooperative-scheduling problem — this is exactly
why `tokio::fs` does that). `io_uring` operations are genuinely
asynchronous at the kernel level for every operation it supports,
including file reads: you submit the SQE and get a CQE when it's done,
with no blocking syscall on the calling thread at any point.

### Registered/fixed buffers
Passing a raw userspace pointer in an SQE means the kernel validates and
pins it fresh on every single operation. Pre-registering a set of
buffers once (`io_uring_register_buffers`) and referencing them by index
in subsequent SQEs skips that per-operation validation — a real
throughput win at high operation rates, at the cost of managing a fixed
buffer pool yourself (see `14-memory/04-buffer-pool.md`).

### SQPOLL: skipping the submission syscall entirely
Normally, after writing SQEs into the ring, you still need one
`io_uring_enter` syscall to tell the kernel new work exists. In
`SQPOLL` mode, a dedicated kernel thread polls the SQ ring continuously,
so a busy submitter never needs to call `io_uring_enter` for submission
at all — pure ring-buffer writes, no syscall in the loop. This trades a
CPU core (the polling kernel thread) for eliminating submission syscall
overhead entirely; worth it only under very high operation rates.

### Gotcha: availability and attack-surface concerns are real
`io_uring` has a track record of kernel privilege-escalation CVEs
specifically in its own code — enough that some environments (Docker's
default seccomp profile at various points, ChromeOS, some managed cloud
platforms) have disabled or restricted it outright. Check your actual
deployment target's kernel version and syscall allow-list before
designing `proxy/` around it as a hard dependency; treat it as an
optimization with a fallback, not a foundation.

## Practice
1. Read the `io-uring` or `tokio-uring` crate's source for how it maps
   SQE submission and CQE completion onto Rust futures/wakers.
2. Benchmark file-read latency for a cold, disk-backed file via a
   traditional `tokio::fs` (thread-pool-offloaded) path versus an
   `io_uring`-based path, on a system where both are available.
3. Register a fixed buffer set and compare per-operation overhead against
   passing raw pointers, at a high operation rate.
4. Check whether your actual deployment target (a container image, a
   specific cloud VM type) allows the `io_uring` syscalls at all —
   `docker run --security-opt seccomp=... ` or the platform's own docs —
   before assuming it's available in production.
