# Linux

Phase 2. The syscall-level machinery a proxy runs on — what tokio is doing
underneath, and what the kernel will and won't do for you.

## How to read this directory

Two ways to use it, depending on where you're starting from:

- **No OS/systems background yet:** read every file below in order,
  [`01-fundamentals.md`](01-fundamentals.md) through [`21-systemd-and-services.md`](21-systemd-and-services.md), doing each file's
  `## Practice` before moving to the next. Treat the whole directory as
  one tutorial — the Kernel mechanisms group assumes the Fundamentals
  group, so don't skip ahead.
- **Already comfortable with processes, syscalls, and epoll/select:** skip
  the entire Fundamentals group and start at [`14-epoll.md`](14-epoll.md) — that's where
  this directory stops being generic OS knowledge and starts being the
  specific mechanisms (`epoll`, `io_uring`, zero-copy) a proxy actually
  leans on. Jump between files in the Kernel mechanisms group in whatever
  order matches your gaps; they don't depend on each other as tightly as
  the Fundamentals group does.

## Files

**Fundamentals** (start here if "syscall," "file descriptor," "kernel
space," "inode," or "container" don't already have a precise meaning — the
group in this directory written as a from-scratch primer; every file
below assumes what these cover):
- [`01-fundamentals.md`](01-fundamentals.md) — what the kernel/user-space split is for, and an index to the twelve files below
- [`02-hardware-basics.md`](02-hardware-basics.md) — CPU privilege levels, MMU, interrupts, DMA, and how a packet reaches your `read()`
- [`03-processes-and-threads.md`](03-processes-and-threads.md) — process vs thread, why threads are cheaper, the two stacked schedulers (kernel + tokio)
- [`04-process-lifecycle.md`](04-process-lifecycle.md) — `fork`/`exec`/`wait`, zombies, orphans, process groups, PID 1 in containers
- [`05-kernel-and-syscalls.md`](05-kernel-and-syscalls.md) — the syscall boundary and its real cost, file descriptors
- [`06-filesystem-and-vfs.md`](06-filesystem-and-vfs.md) — inodes, paths, VFS, page cache vs durability, atomic replace, `/proc`
- [`07-users-permissions-capabilities.md`](07-users-permissions-capabilities.md) — UIDs, permission bits, capabilities, dropping privileges, port 443 without root
- [`08-memory-basics.md`](08-memory-basics.md) — virtual address space (a pointer into [`16-memory.md`](16-memory.md)'s depth), the cache/RAM/disk hierarchy, stack vs heap
- [`09-blocking-io-and-signals.md`](09-blocking-io-and-signals.md) — why syscalls block, why event loops exist, signals as async kernel notifications
- [`10-ipc.md`](10-ipc.md) — pipes, Unix sockets, `SCM_RIGHTS` fd passing, shared memory, `eventfd`, `futex`
- [`11-time-and-timers.md`](11-time-and-timers.md) — monotonic vs wall-clock, timer wheel, timeouts and deadlines
- [`12-cpu-scheduling.md`](12-cpu-scheduling.md) — run queues, CFS, nice/affinity, load average, cgroup CPU throttling
- [`13-containers.md`](13-containers.md) — namespaces and cgroups: what a container actually is, not a tiny VM

**Kernel mechanisms and operations:**
- [`14-epoll.md`](14-epoll.md) — readiness notification, level- vs edge-triggered, the `EAGAIN` bug you should hit yourself
- [`15-io_uring.md`](15-io_uring.md) — the newer async I/O interface and where it differs in kind from epoll
- [`16-memory.md`](16-memory.md) — page faults, mmap, RSS/PSS, swap and reclaim, the OOM killer, huge pages, NUMA, swapping out the global allocator
- [`17-signals.md`](17-signals.md) — signal handling in an async process, `SIGTERM`/`SIGHUP` conventions
- [`18-zerocopy.md`](18-zerocopy.md) — `sendfile`, `splice`, `mmap`, and when they actually pay
- [`19-netfilter-and-linux-networking.md`](19-netfilter-and-linux-networking.md) — netfilter hooks, iptables/nftables, conntrack, NAT, transparent proxying, namespaces/veth, `tc netem`
- [`20-limits-and-proc.md`](20-limits-and-proc.md) — rlimits and `EMFILE`, `/proc`, and the observability toolbox by question
- [`21-systemd-and-services.md`](21-systemd-and-services.md) — unit files, the SIGTERM/restart contract, socket activation, hardening

**Review:**
- [`22-recall-and-review.md`](22-recall-and-review.md) — the fifteen-fact skeleton, per-file questions, drawings to redo from memory, predict-then-run experiments; use it on a 1/3/7/21-day schedule so the material sticks

## Where it goes next

The fundamentals group first if you need it — everything else assumes it.
[`14-epoll.md`](14-epoll.md) underpins [`04-runtime/01-tokio.md`](../04-runtime/01-tokio.md); [`17-signals.md`](17-signals.md)
underpins [`09-architecture/03-config.md`](../09-architecture/03-config.md) and [`09-architecture/04-graceful-shutdown.md`](../09-architecture/04-graceful-shutdown.md);
[`18-zerocopy.md`](18-zerocopy.md) underpins [`05-http-stack/06-static.md`](../05-http-stack/06-static.md); [`19-netfilter-and-linux-networking.md`](19-netfilter-and-linux-networking.md) is where [`01-network/`](../01-network) meets the OS; [`21-systemd-and-services.md`](21-systemd-and-services.md) and [`20-limits-and-proc.md`](20-limits-and-proc.md) are what you need to deploy and operate [`proxy/`](../../proxy). For what happens
*inside* the kernel below these calls, see [`16-kernel/`](../16-kernel) — particularly
[`16-kernel/01-epoll-internals.md`](../16-kernel/01-epoll-internals.md), [`16-kernel/02-io_uring-internals.md`](../16-kernel/02-io_uring-internals.md), and
[`16-kernel/08-page-cache.md`](../16-kernel/08-page-cache.md).
