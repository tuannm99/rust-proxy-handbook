# Kernel

Deeper than `02-linux/06-epoll.md`/`02-io_uring.md`'s application-facing
treatment — what's actually happening inside the kernel below the syscall
you called.

## Status: written; read `02-linux/06-epoll.md`/`02-io_uring.md` first

Everything below assumes you've already hit the application-facing
version of the same mechanism (`02-linux/06-epoll.md`'s `EAGAIN` bug,
`02-linux/07-io_uring.md`'s basic submit/complete usage) and want to know why
it behaves that way. If reading straight through, a reasonable order is:
`01-epoll-internals.md` → `02-io_uring-internals.md` → `03-tcp-stack.md` →
`04-interrupt.md` → `05-rss.md` → `06-rps.md` → `07-scheduler.md` → `08-page-cache.md` →
`09-ebpf.md` → `10-xdp.md` (the last two back `labs/17-ebpf` directly and are
fine to read on their own whenever that lab comes up).

## Written

- `09-ebpf.md` — the eBPF VM, the verifier, maps, attach points, Aya; backs `labs/17-ebpf`
- `10-xdp.md` — dropping packets in the NIC driver, attach modes, the proxy↔XDP feedback loop
- `08-page-cache.md` — how the kernel caches file data, and how that interacts with `mmap`/`sendfile` in `02-linux/10-zerocopy.md`
- `03-tcp-stack.md` — the kernel's TCP state machine, SYN vs accept backlog, TIME_WAIT
- `01-epoll-internals.md` — how epoll is implemented inside the kernel (red-black tree of watched fds, ready list)
- `02-io_uring-internals.md` — submission/completion queue mechanics below the `io_uring` crate API
- `07-scheduler.md` — CPU scheduling basics relevant to a latency-sensitive proxy (CFS, priorities, `nice`)
- `04-interrupt.md` — hardware interrupts vs softirqs, why they matter for network-heavy workloads
- `05-rss.md` — Receive Side Scaling, spreading NIC interrupts across cores in hardware
- `06-rps.md` — Receive Packet Steering, the software fallback when hardware queues aren't enough
