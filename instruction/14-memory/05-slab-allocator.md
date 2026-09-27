# Slab Allocator

Fixed-size-class allocation: the allocator-level answer to fragmentation.
`13-algorithms/slab.md` covers the Rust data structure you write; this file
covers the allocation strategy underneath it.

## What to learn

### The idea
A general-purpose allocator must serve any size, which is what makes it
vulnerable to fragmentation (`14-memory/06-fragmentation.md`). A slab
allocator gives up generality: it serves exactly one object size, from
pre-carved regions ("slabs") divided into equal slots.

That restriction buys three things at once:
- **No external fragmentation, by construction.** Every free slot fits
  every request, because every request is the same size.
- **O(1) alloc and free.** Pop from or push to a free list — no size search,
  no coalescing, no splitting.
- **Cache locality.** Objects of the same type are contiguous, so iterating
  active connections touches consecutive cache lines (the CPU-cache side of
  this is covered in `17-performance/01-cpu-cache.md`).

This originated in the Solaris kernel and is how Linux allocates its own
fixed-size objects (`task_struct`, inodes, socket buffers) — the same
reason applies to a proxy allocating one connection struct per connection.

### Structure: slabs, free lists, and the cache
An allocator for one size class holds a set of slabs, each typically one or
a few pages, carved into N slots. Slabs are tracked by state — full,
partial, empty — and allocation prefers a *partial* slab, so partly-used
slabs fill up rather than every slab staying half-empty. Empty slabs are
the only ones that can be returned to the OS.

As in `13-algorithms/slab.md`, the free list threads through the free slots
themselves, so it costs no extra memory.

Gotcha: allocate from partial slabs first, and free-list order matters more
than it looks. LIFO (reuse the most recently freed slot) keeps the working
set hot in cache; FIFO cycles through all slots and evicts constantly. This
is a one-line difference with a measurable throughput effect.

### Per-CPU caches
The free list is shared mutable state, so a naive slab allocator serializes
every allocation on one lock — unacceptable on a multi-threaded proxy where
allocation is on the hot path.

Real implementations keep a small per-CPU (or per-thread) magazine of free
objects, refilled from the shared slabs in batches. The common case touches
only thread-local state with no atomics at all; the shared lock is taken
once per batch instead of once per allocation. This is the core trick in
jemalloc's tcache and mimalloc alike.

Gotcha: per-thread caching creates cross-thread imbalance. A proxy where
thread A accepts connections and thread B closes them accumulates free
objects in B's cache while A starves and keeps refilling from the shared
pool. Allocators handle this with periodic cache flushing and remote-free
queues; if you hand-roll a pool, this asymmetry is the bug you will hit,
and it appears as steadily growing memory rather than as a crash.

### Where this actually belongs in a proxy
You almost certainly should not write a global slab allocator. jemalloc and
mimalloc already implement size-class allocation with per-CPU caches, and
swapping the global allocator (`02-linux/09-memory.md`) gets you most of the
benefit for one line of code.

What is worth hand-rolling is a **typed pool** for the few objects
allocated once per connection or once per request — connection state and
I/O buffers (dedicated buffer pooling is planned in `14-memory/00-README.md`).
Those are known-size, high-churn, and long-lived enough that pooling them
removes the allocator from the hot path entirely, which is a different and
larger win than making allocation cheaper.

Gotcha: a pooled object must be fully reset on release. A connection struct
returned to the pool still holding the previous request's headers or peer
address is a data-disclosure bug that the type system will not catch,
because the type is valid — only the contents are stale. Reset on release,
not on acquire, so a leaked stale object is never handed out even if the
acquire path is later refactored.

### Bounding the pool
An unbounded pool converts a traffic spike into a permanent memory
high-water mark. Cap the pool size and let allocations past the cap fall
through to the global allocator (degrade in performance, not in
correctness), and export the pool's size and hit rate as metrics
(`08-observability/02-metrics.md`) so the cap is tuned from data.

## Practice
1. Implement a single-size-class slab allocator: carve a page into slots,
   thread a free list through the free ones, and implement alloc/free.
   Verify with an assertion that no two live allocations overlap.
2. Track full/partial/empty slabs and confirm allocation prefers partial
   slabs; construct a workload that would leave every slab half-empty
   without that rule.
3. Compare LIFO vs FIFO free-list order under an alloc/free-heavy benchmark
   and measure the difference in cache misses (`perf stat -e cache-misses`).
4. Add a per-thread magazine and benchmark against the single-lock version
   at 1, 4, and 8 threads.
5. Reproduce the cross-thread imbalance: allocate on one thread and free on
   another in a loop, and watch total memory grow. Then add a flush or
   remote-free path and confirm it stabilizes.
6. Build a bounded, typed buffer pool for `proxy`'s per-connection read
   buffers with reset-on-release, and export size/hit-rate metrics. Compare
   allocation counts under load with and without it.
