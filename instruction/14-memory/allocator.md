# General-Purpose Allocators

`14-memory/fragmentation.md` covers what goes wrong over time. This file
covers what's actually behind `malloc`/Rust's global allocator, and why
swapping it is one of the highest-leverage single changes for a
multi-threaded proxy.

## What to learn

### Size classes: the allocator's own bucketing
A general-purpose allocator doesn't hand back exactly-sized memory; it
rounds each request up to one of a fixed set of size classes (e.g. 8,
16, 32, 48, 64, 96, 128, ... bytes) and serves it from a free list for
that class. This bounds fragmentation to "wasted space within a size
class" rather than arbitrary external fragmentation, at the cost of some
internal waste — see `fragmentation.md` for the concrete failure mode
this trades against.

### Thread-local arenas: avoiding one global lock
A single global free list would serialize every allocation across every
thread in a multi-threaded proxy — a lock held on the hottest possible
path. Production allocators (jemalloc, mimalloc, tcmalloc) give each
thread its own arena with its own free lists, so most allocations and
frees never touch a shared lock at all; cross-thread frees (a buffer
allocated on one thread, freed on another — common in an async runtime
that moves tasks across worker threads) are the remaining coordination
point, handled differently by each allocator's design.

### Rust's `GlobalAlloc` trait and swapping it
```rust
use std::alloc::{GlobalAlloc, Layout, System};

struct MyAllocator; // wraps mimalloc, jemalloc, or your own
unsafe impl GlobalAlloc for MyAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 { /* ... */ System.alloc(layout) }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) { /* ... */ System.dealloc(ptr, layout) }
}

#[global_allocator]
static GLOBAL: MyAllocator = MyAllocator;
```
In practice you reach for the `mimalloc` or `tikv-jemallocator` crate
rather than writing this by hand — the snippet shows what `#[global_allocator]`
actually replaces: every `Box`, `Vec`, `String` allocation in the whole
binary, process-wide, with no per-call-site opt-out.

### Why glibc's default is usually the wrong choice for a proxy
glibc's `ptmalloc` is a reasonable general-purpose allocator but is
notably conservative about returning memory to the OS and multiplies its
per-thread arena count under contention (`fragmentation.md` covers the
`MALLOC_ARENA_MAX` angle). mimalloc and jemalloc are both designed
around exactly the workload a proxy has — many small, short-lived
allocations across many threads — and consistently benchmark better on
it; this is a decision worth making deliberately rather than inheriting
by default.

### Gotcha: benchmark under the real shape of contention
A single-threaded microbenchmark of alloc/free in a loop tells you
almost nothing about which allocator wins under a proxy's actual load:
many threads, allocating and freeing at different rates, with objects
sometimes freed on a different thread than the one that allocated them.
Benchmark with `12-testing/load-testing.md`'s concurrent traffic, not a
synthetic single-thread loop, before picking one.

## Practice
1. Swap `proxy`'s (or a `labs/` crate's) global allocator to `mimalloc`
   via `#[global_allocator]` and confirm the binary still builds and
   passes its tests.
2. Benchmark an allocation-heavy hot path (e.g. per-request header
   parsing in `labs/01-http-parser`) under concurrent load with the
   system allocator, then mimalloc, then jemalloc; compare throughput and
   tail latency, not just mean allocation time.
3. Reproduce the cross-thread-free pattern deliberately (allocate on one
   tokio worker, send the value across a channel, free it on another) and
   check whether your chosen allocator's documentation calls this out as
   a slow path.
4. Re-run `14-memory/fragmentation.md`'s RSS-over-time experiment with
   your chosen allocator and compare the plateau height against the
   system allocator's.
