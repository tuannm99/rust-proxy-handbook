# CPU Scheduling

Why a proxy's p99 latency sometimes has nothing to do with its own code:
the OS scheduler decides which thread runs on which core when, and a
busy host can add latency your profiler will never show you.

## What to learn

### CFS: fairness by virtual runtime, not FIFO
Linux's Completely Fair Scheduler keeps runnable tasks in a red-black
tree ordered by **virtual runtime** (`vruntime`) — accumulated CPU time,
weighted by priority. The scheduler always picks the task with the
lowest `vruntime` to run next, so a task that's used less CPU recently
gets preference; running advances its `vruntime`, which eventually makes
it not the minimum anymore, and something else runs. This is "fair"
scheduling, not "first ready, first run" — no task is guaranteed to run
within any specific bound, only that CPU time is being distributed
proportionally to weight over time.

### `nice`/priority controls weight, not a hard guarantee
`nice` values map to weights that scale how fast a task's `vruntime`
accrues (a higher-priority task's `vruntime` grows more slowly per unit
of wall-clock CPU time, so it stays "owed" CPU longer). This shifts *how
much* CPU a task gets relative to others, not a latency guarantee — a
`proxy` process at high priority on an idle machine behaves identically
to one at normal priority; the difference only shows up when something
else is actually contending for the CPU.

### CPU affinity: fighting migration cost, not just fairness
A task migrating between cores loses its warm L1/L2 cache state on the
old core and starts cold on the new one (`17-performance/cpu-cache.md`,
`17-performance/numa.md`). Pinning a proxy's worker threads to specific
cores (`taskset`, or `sched_setaffinity` from within the program) trades
away CFS's freedom to load-balance across all cores for consistent,
warm-cache execution on a fixed set — a real tail-latency win at the
cost of losing automatic load balancing if the pinned set becomes
unevenly loaded.

### Gotcha: tokio's scheduler is a second, separate layer above this one
tokio's work-stealing scheduler decides which *task* (an async green
thread) runs on which *tokio worker thread* it manages — that's entirely
userspace bookkeeping. The OS scheduler separately decides which of your
process's actual OS threads gets a CPU core at all, and when. A task
tokio considers "ready to poll" can sit unrun for a while not because of
anything tokio did, but because the OS thread that would poll it isn't
currently scheduled on any core — an oversubscribed host (more runnable
threads across all processes than cores) causes exactly this, and it
shows up as latency tokio's own metrics won't explain. Check
`vmstat`/`mpstat`'s run-queue length (`r` column) alongside tokio-level
metrics before concluding a latency spike is a tokio or application
problem.

## Practice
1. Run `proxy` (or a `labs/` crate) under `12-testing/load-testing.md`
   on a host you've deliberately oversubscribed (spin up enough
   CPU-bound background processes to exceed the core count) and observe
   the effect on p99 latency versus an unloaded host.
2. Check `vmstat 1`'s run-queue column during that test and confirm it
   correlates with the latency increase, distinguishing this from a
   tokio-level scheduling problem.
3. Pin the proxy's worker threads to a subset of cores with `taskset`
   (leaving the oversubscribing processes on the rest) and re-measure
   p99 latency.
4. Compare `nice`-based priority boosting against core pinning for the
   same oversubscribed scenario, and describe in writing which one
   actually addresses the tail-latency problem and why.
