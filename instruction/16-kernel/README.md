# Kernel

Deeper than `02-linux/epoll.md`/`io_uring.md`'s application-facing
treatment — what's actually happening inside the kernel below the syscall
you called.

## Planned topics

- `tcp-stack.md` — the kernel's TCP state machine, send/receive buffers, backlog
- `epoll-internals.md` — how epoll is implemented inside the kernel (red-black tree of watched fds, ready list)
- `io_uring-internals.md` — submission/completion queue mechanics below the `io_uring` crate API
- `page-cache.md` — how the kernel caches file data, and how that interacts with `mmap`/`sendfile` in `02-linux/zerocopy.md`
- `scheduler.md` — CPU scheduling basics relevant to a latency-sensitive proxy (CFS, priorities, `nice`)
- `interrupt.md` — hardware interrupts vs softirqs, why they matter for network-heavy workloads
- `rss.md` / `rps.md` — Receive Side Scaling / Receive Packet Steering, spreading NIC interrupts across cores
- `xdp.md` / `ebpf.md` — kernel-bypass-adjacent packet processing, relevant for `07-security/ddos.md` mitigation at the edge
