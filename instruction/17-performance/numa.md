# NUMA

Non-uniform memory access — on a multi-socket machine, memory attached to
another socket is slower to reach, and ignoring that can halve throughput.
Relevant only on multi-socket hardware; skip on a single-socket box.

## What to learn

### What NUMA is
On a multi-socket server each CPU socket has its own memory controller and
its own bank of RAM (a "NUMA node"). A core can read any node's memory, but
reaching a *remote* node crosses the inter-socket interconnect (UPI/QPI) —
typically 1.5–2× the latency and lower bandwidth than local memory. On a
single-socket machine there is one node and none of this applies, which is
most deployments; this matters on the big 2- and 4-socket boxes a
high-traffic proxy sometimes runs on.

### The default allocation policy and its trap
Linux uses first-touch: a page is placed on the node of the core that first
*writes* it, not the one that allocated it. So if a startup thread
initializes a big buffer pool and worker threads on other sockets then use
it, every access is remote. The pool "belongs" to the wrong node for its
whole life. This is the most common NUMA mistake and it is invisible in
code — the allocation looks perfectly normal.

### The fix: pin, and touch locally
The strategy is to keep each worker's memory on the worker's node:

- Pin worker threads to specific cores (`sched_setaffinity`, or the
  `core_affinity` crate) so a worker stays on one socket.
- Have each worker allocate and first-touch its *own* buffers, so
  first-touch places them locally, rather than sharing one global pool
  initialized elsewhere.
- This turns the whole proxy into a shared-nothing, per-core design — which
  is what pingora and the DPDK-style data planes do, and it composes with
  the per-core counters in `17-performance/false-sharing.md` and the
  per-core caches in `13-algorithms/lru.md`.

```text
run per socket:  ./proxy  →  numactl --cpunodebind=0 --membind=0 ./proxy (inst A)
                             numactl --cpunodebind=1 --membind=1 ./proxy (inst B)
```

Gotcha: the blunt-but-effective production answer is often not to make one
process NUMA-aware at all, but to run *one proxy instance per socket*, each
pinned with `numactl`, behind a load balancer. Two shared-nothing instances
sidestep every cross-node question that a single process would have to
solve carefully.

### Interrupts and NICs are part of the picture
A NIC's interrupts land on some node; if packets are DMA'd into node 0's
memory but processed by a worker on node 1, you pay the remote cost per
packet before your code even runs. Aligning NIC IRQ affinity (and RSS/RPS,
`16-kernel/`) with the workers that process those packets is the other half
of NUMA tuning, and often matters more than where your heap lives.

### Measuring it
`numastat` shows per-node allocation and, crucially, `numa_miss` /
`numa_foreign` counts — remote accesses that wanted to be local. `perf` can
attribute remote-memory stalls. As with the rest of
`17-performance/`, do not tune speculatively: confirm you are actually
NUMA-bound (throughput scales poorly across sockets, high remote-access
counts) before pinning anything, because on a single-socket box this is all
wasted effort.

## Practice
1. Check whether it even applies: `numactl --hardware` and `lscpu` to see
   node count. On a single-socket dev box, note that this folder is a
   no-op there and move on.
2. On a multi-socket machine (or a cloud instance that exposes NUMA),
   reproduce the first-touch trap: init a large buffer in one thread, use
   it from threads pinned to another node, and read `numastat`'s
   `numa_foreign`.
3. Fix it by having each pinned worker first-touch its own buffers;
   re-measure `numa_miss`/`numa_foreign` and throughput.
4. Compare the two deployment shapes for `proxy`: one NUMA-aware process
   vs two `numactl`-pinned instances behind a balancer, under
   `12-testing/load-testing.md` load.
5. Align NIC IRQ affinity with your worker sockets and measure whether
   per-packet remote cost drops — connect this to RSS/RPS in `16-kernel/`.
