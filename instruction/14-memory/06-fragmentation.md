# Fragmentation

Why a long-running proxy's memory climbs and never comes back down, even
though it has no leak.

## What to learn

### The symptom that looks like a leak but is not
A proxy runs for a week; RSS grows from 200 MB to 2 GB and plateaus. Every
allocation has a matching free — valgrind and the leak detectors are clean —
and yet the memory is gone. This is fragmentation, and diagnosing it as a
leak sends you hunting for a bug that does not exist.

The distinction that matters operationally: a leak grows without bound and
kills the process; fragmentation grows to a high-water mark set by your
worst traffic pattern and stays there. Both look identical on a graph for
the first few hours.

### External fragmentation
Free memory exists but is split into pieces too small to satisfy a request.
Allocate 10,000 × 1 KB buffers, free every other one, and you have ~5 MB
free that cannot serve a single 2 KB allocation contiguously.

For a proxy the trigger is mixed lifetimes: short-lived request buffers
interleaved with long-lived connection state. The connection state that
survives punches holes through regions the allocator would otherwise return
to the OS.

### Internal fragmentation and size classes
Allocators round allocations up to a size class (8, 16, 32, 48, 64, 80, 96,
112, 128, ... bytes in jemalloc-style allocators). A 65-byte allocation
occupies an 80-byte slot; 15 bytes are wasted and unreachable.

Usually minor — unless your hot-path struct sits just past a boundary. A
129-byte connection struct takes a 160-byte slot, wasting 24% at 100k
connections. This is why struct field reordering
(`17-performance/04-memory-layout.md`) is not micro-optimization at scale: shrinking a
struct below a size-class boundary is a step-function win, not a linear one.

### Why memory does not return to the OS
`free()` returns memory to the *allocator*, not the kernel. The allocator
returns it to the OS only when it can release a whole region (via `munmap`
or `madvise(MADV_DONTNEED)`), which requires that region to be entirely
free. One long-lived object anchoring a 4 MB region keeps all 4 MB
resident.

This is precisely the slab shrink problem in `13-algorithms/slab.md`,
generalized: any structure that grows to a peak and then holds its capacity
pins the allocator's regions along with it.

Gotcha: glibc's malloc is particularly reluctant to return memory, and its
per-thread arenas multiply the effect — each thread gets its own arena, so
a proxy with 16 worker threads can hold 16 separate high-water marks.
`MALLOC_ARENA_MAX` bounds this, and switching to jemalloc or mimalloc
(`02-linux/08-memory.md` covers the `#[global_allocator]` swap) usually helps
more than any tuning of glibc.

### The structural fixes
Fragmentation is an allocation-pattern problem, so the fixes change the
pattern rather than the allocator:

- **Pool same-sized objects.** A buffer pool hands back the same 8 KB
  buffers forever; nothing is ever freed, so nothing can fragment.
- **Arena-allocate per request.** Bump-allocate everything a request needs
  into one region and drop the whole region at the end. No interleaving of
  lifetimes, no holes.
- **Separate lifetimes into separate allocators.** Keep long-lived
  connection state away from short-lived request data so the former cannot
  pin regions belonging to the latter.
- **Pre-size for peak.** If a structure will reach 100k entries, allocating
  that capacity once beats growing into it while fragmenting.

Gotcha: pooling has its own failure mode — a pool that grows to serve a
traffic spike and never shrinks *is* the high-water mark, just under your
control instead of the allocator's. That is usually the better trade
(bounded and observable), but only if you actually bound it and export the
size as a metric (`08-observability/02-metrics.md`).

### Measuring it
The number to watch is the ratio of RSS to bytes your application believes
are live. jemalloc exposes both directly (`stats.allocated` vs
`stats.resident`); a ratio drifting from ~1.1 toward 2+ is fragmentation,
not a leak. Track it as a gauge rather than diagnosing it once, because the
whole point is that it develops over days.

## Practice
1. Reproduce external fragmentation: allocate 100k × 1 KB buffers, free
   every other one, then try to allocate 10k × 2 KB. Record RSS at each
   step and confirm it does not fall after the frees.
2. Run the same test under glibc malloc, then under jemalloc and mimalloc
   via `#[global_allocator]`. Compare the RSS high-water mark.
3. Measure size-class rounding: allocate structs of 64, 65, 128, and 129
   bytes 100k times each and compare actual RSS growth against the
   arithmetic you would expect.
4. Instrument `proxy` with an allocated-vs-resident gauge and run
   `12-testing/01-load-testing.md` traffic against it for an extended period;
   watch the ratio over time rather than at a single instant.
5. Add a buffer pool for request I/O buffers, re-run step 4, and compare
   the ratio's drift with and without pooling.
