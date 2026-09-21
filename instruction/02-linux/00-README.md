# Linux

Phase 2. The syscall-level machinery a proxy runs on — what tokio is doing
underneath, and what the kernel will and won't do for you.

## Files

- `01-epoll.md` — readiness notification, level- vs edge-triggered, the `EAGAIN` bug you should hit yourself
- `02-io_uring.md` — the newer async I/O interface and where it differs in kind from epoll
- `03-memory.md` — virtual memory, paging, RSS, swapping out the global allocator
- `04-signals.md` — signal handling in an async process, `SIGTERM`/`SIGHUP` conventions
- `05-zerocopy.md` — `sendfile`, `splice`, `mmap`, and when they actually pay

## Where it goes next

`01-epoll.md` underpins `04-runtime/01-tokio.md`; `04-signals.md` underpins
`09-architecture/03-config.md` and `graceful-shutdown.md`; `05-zerocopy.md`
underpins `05-http-stack/05-static.md`. For what happens *inside* the kernel
below these calls, see `16-kernel/` — particularly
`epoll-internals.md`, `io_uring-internals.md`, and `page-cache.md`.
