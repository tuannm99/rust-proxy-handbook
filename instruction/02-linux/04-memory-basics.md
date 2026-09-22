# Memory Basics

Part of the from-scratch fundamentals series — see `02-linux/01-fundamentals.md`
for the full index. Deliberately brief: `02-linux/08-memory.md` is where
virtual memory, overcommit, the page cache, and NUMA get developed in
real depth — this file exists only so that development doesn't start
from zero.

## What to learn

### Every process has its own virtual address space
A process's "own private world" (`02-processes-and-threads.md`) isn't
physical RAM directly — it's a **virtual** address space that the
kernel maps to physical memory (or to "not present yet, fault me in on
first access") via page tables. Your program only ever sees virtual
addresses; the mapping to real RAM is the kernel's problem, invisible to
you except in its performance consequences. Two processes can both use
virtual address `0x1000` for completely different data, safely, because
the kernel's page tables map each process's `0x1000` to different
physical memory.

That's genuinely all you need here to make `02-linux/08-memory.md`'s
opening paragraph — "every process gets its own virtual address space;
the kernel's page tables map virtual pages to physical frames" — land as
a restatement rather than new information.

### The memory hierarchy: registers, cache, RAM, disk
Roughly, from fastest/smallest/most expensive per byte to
slowest/largest/cheapest: CPU **registers**, then **cache** (L1/L2/L3, a
handful of MB, built into the CPU), then **RAM** (gigabytes, still
volatile — lost on power-off), then **disk/SSD** (much larger, much
slower, persistent). Each level acts as a cache for the level below it:
RAM caches disk content (`08-memory.md`'s page cache section is exactly
this), CPU cache caches RAM content.

The number that matters most in practice: an L1 cache hit is roughly
1 nanosecond; a RAM access is roughly 100 nanoseconds; an SSD access is
tens of *microseconds*; a spinning disk seek is *milliseconds* —
each step down is roughly 1-2 orders of magnitude slower. This is the
entire motivation behind `17-performance/01-cpu-cache.md`'s existence
(data layout choices that keep hot data in cache) and behind why
`02-linux/08-memory.md`'s page cache matters so much for a proxy serving
static files: a cache hit there is a RAM access; a miss is a disk access,
100-1000x slower.

### Stack vs heap, briefly
Each thread (`02-processes-and-threads.md`) gets its own **stack** — a
fixed-direction, automatically-managed region for local variables and
function call frames, fast to allocate from (just move a pointer) and
automatically reclaimed on function return. The **heap** is shared across
all threads in a process, used for anything that needs to outlive the
function that created it or whose size isn't known at compile time —
Rust's `Box`, `Vec`, `String` all allocate here. Heap allocation is
slower than stack allocation (it goes through an allocator, sometimes a
syscall — `14-memory/01-allocator.md`) and doesn't free itself
automatically the way a stack frame does; in Rust, `Drop` is what ties
heap deallocation back to a scope ending, without needing a garbage
collector.

### Why this matters for a proxy specifically
A proxy's entire performance profile is a memory-hierarchy story:
keeping a hot route table small enough to stay cache-resident
(`17-performance/01-cpu-cache.md`), letting the kernel's page cache
absorb repeated static-file reads instead of re-implementing that cache
yourself (`08-memory.md`), and avoiding unnecessary heap allocation on
the request hot path (`14-memory/02-arena.md`, `14-memory/03-object-pool.md`)
are all different instances of "keep data as close to the top of this
hierarchy as you can, for as long as you can."

## Practice
1. Run `free -h` and identify total RAM, used, and "available" (not the
   same as "free" — `08-memory.md` explains why once you get there).
2. Write a small Rust program that allocates a large `Vec<u8>` with
   `with_capacity` (reserves virtual memory) versus one that additionally
   writes to every byte (forces physical pages to back it) — watch
   `VmRSS` in `/proc/self/status` before and after each step and note
   which one actually grows RSS.
3. Look up (or measure, with a micro-benchmark) the rough latency of an
   L1 cache hit, a RAM access, and an SSD read on your own machine's
   class of hardware — write down the three numbers and their ratios.
4. Read `14-memory/02-arena.md`'s opening section now that stack-vs-heap
   is fresh, and explain in one sentence why bump allocation is closer in
   spirit to stack allocation than to a general-purpose heap allocator.
