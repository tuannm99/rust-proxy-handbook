# Page Replacement Algorithms

## What to learn

### The problem: more virtual pages than physical frames
When physical memory is full and a process needs a new page, the OS must evict something — page replacement is the policy for choosing what. This is [`02-linux/16-memory.md`](../02-linux/16-memory.md)'s virtual memory picture (built on [`02-linux/08-memory-basics.md`](../02-linux/08-memory-basics.md)'s primer) with the missing piece filled in: what actually happens on a page fault when there's no free frame left.

### FIFO, and why it's worse than intuition suggests
Evict the oldest-loaded page. Simple, but suffers Bélády's anomaly: increasing the number of physical frames can, counterintuitively, *increase* the number of page faults for some access patterns — a result surprising enough that it's the standard example for "your intuition about caching policies is not proof."

### LRU: the practical standard, and its exact cost
Evict the least-recently-used page — the same policy as [`13-algorithms/lru.md`](../13-algorithms/lru.md)'s cache eviction, applied to physical page frames instead of an application-level cache. True LRU requires a timestamp (or position) update on every memory access, far too expensive to implement exactly at page-fault-relevant scale in hardware or the OS — real systems approximate it.

### Clock (second-chance): LRU's practical approximation
Pages sit in a circular list, each with a single "referenced" bit set by hardware on access. The eviction pointer sweeps the circle: if a page's bit is set, clear it and move on (giving it a "second chance"); if the bit is already clear, evict it. This approximates LRU's recency ordering using one bit per page instead of a timestamp — the actual mechanism inside Linux's page reclaim, via a more refined active/inactive list variant.

```text
pages: [A(1), B(0), C(1), D(0)]   (bit = referenced)
sweep hits A: bit=1 -> clear to 0, skip
sweep hits B: bit=0 -> evict B
```

### Optimal (Bélády's algorithm) as the unreachable benchmark
Evict the page that won't be used for the longest time in the future — provably minimizes page faults, and provably impossible to implement online, since it requires knowing the future access pattern. Its value is as a theoretical upper bound: real algorithms are evaluated by how close they get to optimal on a given trace, not against any absolute standard.

### Thrashing and the working-set model
When the sum of processes' active working sets exceeds physical memory, the system spends more time servicing page faults than doing useful work — throughput collapses even though CPU utilization looks busy (it's busy paging, not computing). The working-set model formalizes "active working set" as the pages referenced in the last Δ time units, and is the theoretical basis for [`02-linux/16-memory.md`](../02-linux/16-memory.md)'s practical advice: know a process's real memory footprint before setting a container memory limit, because a limit below the working set doesn't slow the process down gracefully — it thrashes it.

## Practice
1. Implement FIFO and LRU page replacement as a simulation over a fixed access trace (a list of page numbers) with a small fixed number of frames, and count page faults for each.
2. Reproduce Bélády's anomaly: find or construct an access trace where FIFO's fault count goes *up* when you add one more frame, and verify LRU doesn't exhibit the same anomaly on that trace.
3. Implement clock/second-chance over the same trace and compare its fault count against true LRU — confirm it's close but not identical.
4. Implement the (offline, cheating) optimal algorithm on the same trace using full knowledge of future accesses, and use it as your baseline to score how close FIFO/LRU/clock get.
5. Connect it back to [`02-linux/16-memory.md`](../02-linux/16-memory.md): run a process with a working set larger than a cgroup memory limit you set, and observe thrashing (via `vmstat`'s `si`/`so` columns or `/proc/vmstat` page-fault counters) rather than a graceful slowdown.
