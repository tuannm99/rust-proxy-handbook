# Page Cache

The kernel's cache of file data in RAM. What makes static file serving
(`05-http-stack/05-static.md`) fast, and what makes your proxy's memory
accounting confusing.

## What to learn

### Every file read goes through it
`read()` on a file does not reach the disk if the data is already cached:
the kernel keeps file contents in page-sized (4 KB) units in the page
cache, keyed by (inode, offset). A hit copies from RAM; a miss triggers
disk I/O, populates the cache, then copies.

Two consequences for a proxy serving static files: the second request for
a file is served from RAM regardless of what your application does, and
your own userspace cache of file contents may be **duplicating** the page
cache — paying twice in memory for one speedup. Cache parsed or compressed
artifacts, not raw file bytes the kernel is already holding.

### Why `free -h` looks alarming and is not
Page cache shows as "buff/cache" and routinely consumes all otherwise-idle
RAM. This is correct behavior — cached pages backed by an unmodified file
are reclaimable instantly, since the data exists on disk. The number to
watch is `available`, not `free`.

Gotcha: inside a container this stops being harmless. Page cache generated
by a cgroup counts against that cgroup's memory limit, so a proxy streaming
large files can be OOM-killed for memory that is *reclaimable* — the
kernel usually reclaims before killing, but under memory pressure combined
with a burst of dirty pages, it does not always win the race. If your proxy
dies with an OOM whose RSS looks far below the limit, this is the first
thing to check.

### Zero-copy depends on it entirely
`sendfile()` and `splice()` (see `02-linux/11-zerocopy.md`) move data from the
page cache to a socket without copying through user space. That is only
fast on a cache *hit* — on a miss the syscall blocks on disk I/O, and in an
async runtime that blocks the whole worker thread
(`03-rust/05-async.md`'s cooperative-scheduling problem), stalling every other
connection on it.

This is the trap in "just use sendfile for static files": it is excellent
for a hot working set and a latency landmine for a cold one. `tokio::fs`
sidesteps it by dispatching to a blocking thread pool, which costs a copy
but keeps the reactor responsive.

Gotcha: `mmap` has the same shape and is worse in an async context — a page
fault on a cold mapping blocks with no syscall boundary for the runtime to
observe, so it is invisible to every tokio diagnostic.

### Readahead
The kernel detects sequential access and prefetches ahead of the reader,
which is why streaming a large file sequentially is much faster than the
same bytes read randomly. `posix_fadvise` lets you state intent explicitly:
`SEQUENTIAL` to increase readahead, `RANDOM` to disable it, `WILLNEED` to
prefetch before you need it, `DONTNEED` to evict.

For a proxy, `WILLNEED` on a file you are about to stream can turn the
first-chunk latency from a disk seek into a cache hit. `DONTNEED` after
streaming a very large one-shot file stops it from evicting your genuinely
hot working set — the page cache's own version of the scan-pollution
problem in `13-algorithms/lru.md`.

### Eviction is CLOCK, and dirty pages are different
Clean pages are dropped on reclaim, essentially free. Dirty pages (written
but not yet persisted) must be written back to disk first, so reclaiming
them can block. Writeback is governed by `vm.dirty_ratio` and
`vm.dirty_background_ratio`; crossing the hard ratio makes *writers* block
synchronously until writeback catches up.

A proxy writing access logs to disk (`08-observability/01-logging.md`) is a
dirty-page producer. Under heavy logging on slow storage, a log write can
block a request-handling thread — which is exactly why that file
recommends `tracing_appender::non_blocking`.

The reclaim policy itself is an approximate-LRU variant: two lists (active
and inactive) with reference bits, which is the CLOCK-family algorithm
described in `13-algorithms/lru.md`. Same reasoning — real LRU's per-access
list surgery is unaffordable at page-cache scale.

### Measuring it
`/proc/meminfo` gives `Cached`, `Dirty`, and `Writeback` globally. For a
specific file, `mincore()` reports which pages are resident (the `vmtouch`
tool wraps this). To test cold-cache behavior honestly, drop the cache
between runs with `echo 3 > /proc/sys/vm/drop_caches` — otherwise your
static-file benchmark is measuring RAM and telling you nothing about
production, where the working set is larger than memory.

## Practice
1. Read a large file twice, timing both. Drop the caches, repeat, and
   confirm the first read's timing returns.
2. Use `vmtouch` (or `mincore` directly) to show which pages of a file
   `labs/04-static-server` has resident after serving it once.
3. Benchmark `labs/04-static-server` with a working set that fits in RAM,
   then one several times larger. Compare p99 latency and explain the gap
   from what you know about cache hits and blocking.
4. Demonstrate the blocking hazard: serve a cold large file with a
   `sendfile`-style path on a single-worker runtime and measure the latency
   of concurrent requests during the disk read.
5. Add `posix_fadvise(WILLNEED)` before streaming and measure the change in
   first-byte latency on a cold file.
6. Run the proxy in a container with a memory limit, stream files totaling
   more than the limit, and watch the cgroup's `memory.current` and page
   cache accounting.
