# Linux

Phase 2. The syscall-level machinery a proxy runs on — what tokio is doing
underneath, and what the kernel will and won't do for you.

## Files

**Fundamentals** (start here if "syscall," "file descriptor," "kernel
space," or "container" don't already have a precise meaning — the one
group in this directory written as a from-scratch primer; every file
below assumes what these cover):
- `01-fundamentals.md` — what the kernel/user-space split is for, and an index to the five files below
- `02-processes-and-threads.md` — process vs thread, why threads are cheaper, the two stacked schedulers (kernel + tokio)
- `03-kernel-and-syscalls.md` — the syscall boundary and its real cost, file descriptors
- `04-memory-basics.md` — virtual address space (a pointer into `09-memory.md`'s depth), the cache/RAM/disk hierarchy, stack vs heap
- `05-blocking-io-and-signals.md` — why syscalls block, why event loops exist, signals as async kernel notifications
- `06-containers.md` — namespaces and cgroups: what a container actually is, not a tiny VM

**Kernel mechanisms:**
- `07-epoll.md` — readiness notification, level- vs edge-triggered, the `EAGAIN` bug you should hit yourself
- `08-io_uring.md` — the newer async I/O interface and where it differs in kind from epoll
- `09-memory.md` — virtual memory, paging, RSS, swapping out the global allocator
- `10-signals.md` — signal handling in an async process, `SIGTERM`/`SIGHUP` conventions
- `11-zerocopy.md` — `sendfile`, `splice`, `mmap`, and when they actually pay

## Where it goes next

The fundamentals group first if you need it — everything else assumes it.
`07-epoll.md` underpins `04-runtime/01-tokio.md`; `10-signals.md`
underpins `09-architecture/03-config.md` and `04-graceful-shutdown.md`;
`11-zerocopy.md` underpins `05-http-stack/05-static.md`. For what happens
*inside* the kernel below these calls, see `16-kernel/` — particularly
`01-epoll-internals.md`, `02-io_uring-internals.md`, and
`08-page-cache.md`.
