# Kernel

Deeper than `02-linux/epoll.md`/`io_uring.md`'s application-facing
treatment — what's actually happening inside the kernel below the syscall
you called.

## Status: written; read `02-linux/epoll.md`/`io_uring.md` first

Everything below assumes you've already hit the application-facing
version of the same mechanism (`02-linux/epoll.md`'s `EAGAIN` bug,
`02-linux/io_uring.md`'s basic submit/complete usage) and want to know why
it behaves that way. If reading straight through, a reasonable order is:
`epoll-internals.md` → `io_uring-internals.md` → `tcp-stack.md` →
`interrupt.md` → `rss.md` → `rps.md` → `scheduler.md` → `page-cache.md` →
`ebpf.md` → `xdp.md` (the last two back `labs/17-ebpf` directly and are
fine to read on their own whenever that lab comes up).

## Written

- `ebpf.md` — the eBPF VM, the verifier, maps, attach points, Aya; backs `labs/17-ebpf`
- `xdp.md` — dropping packets in the NIC driver, attach modes, the proxy↔XDP feedback loop
- `page-cache.md` — how the kernel caches file data, and how that interacts with `mmap`/`sendfile` in `02-linux/zerocopy.md`
- `tcp-stack.md` — the kernel's TCP state machine, SYN vs accept backlog, TIME_WAIT
- `epoll-internals.md` — how epoll is implemented inside the kernel (red-black tree of watched fds, ready list)
- `io_uring-internals.md` — submission/completion queue mechanics below the `io_uring` crate API
- `scheduler.md` — CPU scheduling basics relevant to a latency-sensitive proxy (CFS, priorities, `nice`)
- `interrupt.md` — hardware interrupts vs softirqs, why they matter for network-heavy workloads
- `rss.md` — Receive Side Scaling, spreading NIC interrupts across cores in hardware
- `rps.md` — Receive Packet Steering, the software fallback when hardware queues aren't enough
