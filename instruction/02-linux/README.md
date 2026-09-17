# Linux

Phase 2. The syscall-level machinery a proxy runs on — what tokio is doing
underneath, and what the kernel will and won't do for you.

## Files

- `epoll.md` — readiness notification, level- vs edge-triggered, the `EAGAIN` bug you should hit yourself
- `io_uring.md` — the newer async I/O interface and where it differs in kind from epoll
- `memory.md` — virtual memory, paging, RSS, swapping out the global allocator
- `signals.md` — signal handling in an async process, `SIGTERM`/`SIGHUP` conventions
- `zerocopy.md` — `sendfile`, `splice`, `mmap`, and when they actually pay

## Where it goes next

`epoll.md` underpins `04-runtime/tokio.md`; `signals.md` underpins
`09-architecture/config.md` and `graceful-shutdown.md`; `zerocopy.md`
underpins `05-http-stack/static.md`. For what happens *inside* the kernel
below these calls, see `16-kernel/` — particularly
`epoll-internals.md`, `io_uring-internals.md`, and `page-cache.md`.
