# CPU Scheduling in Practice: Run Queues, Priorities, Affinity, and CFS Throttling

How the kernel decides which thread runs on which core, and what that means for tail
latency, CPU limits in containers, and thread counts. The practical layer; the
academic algorithms (FCFS, round-robin, MLFQ, fairness proofs) are in
[`22-theory/04-cpu-scheduling.md`](../22-theory/04-cpu-scheduling.md) and Linux's internals in [`16-kernel/07-scheduler.md`](../16-kernel/07-scheduler.md).

## What to learn

### Threads wait in queues; the scheduler picks
At any instant there are far more runnable threads than cores. Each core has a **run queue** of threads
that are ready; the kernel scheduler picks one, lets it run for a **timeslice** (or until it blocks or is
preempted by the timer interrupt, [`02-hardware-basics.md`](02-hardware-basics.md)), then picks again. A thread is in one of a few
states: **running**, **runnable** (waiting for a core — `R` in `ps`), **sleeping** (blocked on I/O, a lock, or a timer —
`S`, or uninterruptible `D` for some disk waits), or stopped/zombie ([`04-process-lifecycle.md`](04-process-lifecycle.md)). A blocking
`read` ([`09-blocking-io-and-signals.md`](09-blocking-io-and-signals.md)) moves a thread runnable -> sleeping, freeing the core; data arrival moves it
back to runnable. **Latency from "ready" to "actually running" is run-queue wait**, and it is the part of
your tail latency that no amount of efficient code fixes.

### CFS: fairness by virtual runtime
Linux's default scheduler for normal threads (CFS, and its successor EEVDF in recent kernels) aims at
**fair share**: it tracks how much CPU each thread has received (its **virtual runtime**, `vruntime`) and always
runs the thread that has received the least, so every runnable thread gets roughly an equal slice over time. A
thread that sleeps a lot (an I/O-bound proxy worker) accrues little vruntime, so when its I/O completes it is
favored and runs soon — **interactive and I/O-bound tasks naturally get good latency** against CPU hogs. A
**CPU-bound** thread uses its full slice and is preempted. The timeslice scales with how many threads compete:
more runnable threads per core means each gets a shorter slice and waits longer between turns.

### Priorities: nice, real-time, and what not to do
Weight for fairness comes from **nice** values (-20..+19; higher nice = lower share). `nice -n 10 ./batch` and
`renice` adjust it; lowering below 0 needs privilege (`CAP_SYS_NICE`, [`07-users-permissions-capabilities.md`](07-users-permissions-capabilities.md)). Separate
**real-time** policies (`SCHED_FIFO`, `SCHED_RR`) run strictly ahead of all normal threads with fixed priorities; a
real-time thread that spins starves everything else, including the kernel's own housekeeping, and can lock up
a machine. For a proxy this is almost never the right tool — fix the contention instead
([`17-performance/`](../17-performance)). `chrt -p <pid>` shows a thread's policy.

### Affinity and migration: which core?
By default a thread can run on any core, and the scheduler **migrates** threads to balance load. Each migration
costs warm cache: the thread's data is in the *old* core's L1/L2 ([`17-performance/`](../17-performance)). **CPU affinity** pins
a thread/process to chosen cores (`taskset -c 0-3 ./proxy`, `sched_setaffinity`). It helps in
thread-per-core designs (one worker per core, never migrated, sharing nothing — the architecture of nginx workers,
and a reason to look at `glommio` over tokio's work-stealing for extreme throughput), and in keeping a proxy on the
same cores that handle its NIC queue ([`16-kernel/05-rss.md`](../16-kernel/05-rss.md)) or on a single **NUMA** node
([`16-memory.md`](16-memory.md)). It hurts if you pin carelessly: two busy workers on one core while others idle.
`isolcpus`/cgroup `cpuset` reserve cores for latency-critical work.

### Load average and what "busy" means
`uptime`'s three **load averages** (1/5/15 min) count threads that are *runnable or in uninterruptible sleep* — not CPU
percentage. A load of 8 on 8 cores means saturated; 8 on 32 cores is mostly idle; but a high load with *low* CPU usage
points to threads stuck in `D` state (a slow disk or NFS) rather than computing. Pair it with `top`/`vmstat`
(`r` = runnable, `b` = blocked), and the CPU split `us`/`sy`/`si`/`wa`/`st` ([`02-hardware-basics.md`](02-hardware-basics.md)).
**Pressure Stall Information** (`/proc/pressure/cpu`, `cpu.pressure` per cgroup) reports directly the share of time tasks
were runnable but not running — the clearest "I am starved for CPU" signal.

### cgroup CPU limits and throttling: the container surprise
In a container ([`13-containers.md`](13-containers.md)) a CPU **limit** (`cpu.max`, Kubernetes `limits.cpu`) is a *quota per period*:
e.g. 200 ms of CPU time per 100 ms period = 2 cores' worth. A multi-threaded process can burn the whole quota in the first part of
a period using several cores in parallel, and then **all its threads are frozen (throttled) until the next period** — adding
tens of milliseconds of latency to every request in that window even though average CPU use looks low. `cpu.stat` shows
`nr_throttled` and `throttled_usec`; a nonzero, growing count is the diagnosis. Two traps compound it:

- A runtime sizes its worker threads from the host's core count (`nproc`) — often 64 — not from the container's quota. Sixty-four
  tokio workers sharing a 2-core quota *throttle themselves*. Set the worker thread count explicitly to match the limit
  ([`04-runtime/03-runtime-config.md`](../04-runtime/03-runtime-config.md)).
- A `request` (relative weight) differs from a `limit` (hard cap): requests decide who wins under contention; limits throttle even
  when the machine is idle. Many latency-sensitive teams set requests but no CPU limit.

### Threads vs cores: sizing the runtime
For async code that never blocks, use about one worker per core: more only adds context switches and cache churn
([`02-hardware-basics.md`](02-hardware-basics.md)). For *blocking* work (file I/O via `tokio::fs`, DNS via `getaddrinfo`, CPU-heavy compression)
move it to `spawn_blocking` (a separate, larger pool) so it can't starve the async workers; otherwise a few slow tasks hold every
worker and **every connection stalls together** — the "all latencies spike at once" signature
([`01-network/14-dns.md`](../01-network/14-dns.md)). Sampling where threads are runnable vs waiting is the job of `perf` and `tokio-console`
([`12-testing/05-debugging.md`](../12-testing/05-debugging.md)).

### Gotcha: the noisy neighbor and steal time
On a VM, `st` (steal) is time the hypervisor ran *someone else* while your vCPU was runnable. On shared hosts, another tenant's
burst shows up as your p99 jumping with no change in your code. You can't fix it from inside the box; you can only detect it
(`st` in `top`/`vmstat`) so you stop chasing a ghost.

## Practice

1. Run `top -H -p <pid>` (threads) and `ps -eLo pid,tid,psr,stat,pcpu,comm` against your tokio proxy under load; identify which
   core (`psr`) each worker thread last ran on and how often it moves, and which threads are `R` vs `S`.
2. Start a CPU hog (`yes > /dev/null`) pinned with `taskset -c 0` and run [`labs/00-tcp-server`](../../labs/00-tcp-server) with and without
   affinity to core 0; measure request latency (`wrk`/`hey`, [`12-testing/01-load-testing.md`](../12-testing/01-load-testing.md)) in each case, then
   `nice -n 19` the hog and compare.
3. Read `/proc/loadavg`, `vmstat 1` (`r`, `b`), and `cat /proc/pressure/cpu` while the hog and a load test run together; relate
   each number to the "runnable" count.
4. In a container with `--cpus=2` on a many-core host, run a tokio program using default workers and a CPU-bound request handler;
   watch `cat /sys/fs/cgroup/cpu.stat` (`nr_throttled`) and a latency histogram, then set `worker_threads(2)` explicitly and compare
   p99.
5. Run [`labs/05-reverse-proxy`](../../labs/05-reverse-proxy) with `taskset -c 0-1`, 4 workers, then 2 workers; load test and explain the
   difference using run-queue wait.
6. Move a CPU-heavy step (e.g. gzip of a large body) from inline in a handler to `spawn_blocking`, and show with a concurrent
   tiny-request latency probe that the tiny requests no longer stall.
