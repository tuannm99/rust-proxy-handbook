# False Sharing

Two unrelated atomics on the same cache line silently serializing code you
believed was lock-free. The concurrent counterpart to
`17-performance/01-cpu-cache.md`.

## What to learn

### The mechanism
Cache coherence works at cache-line granularity (64 bytes), not per
variable. When core A writes any byte of a line, every other core's copy of
that *entire* line is invalidated and must be re-fetched. So two atomics
that live on the same line — even though no thread ever touches the *other*
one — ping-pong the line between cores on every write. The threads share no
data logically, but the hardware treats them as sharing, hence "false"
sharing.

```rust
struct Stats {
    requests: AtomicU64,  // core 1 hammers this
    errors:   AtomicU64,  // core 2 hammers this — same 64-byte line
}                         // every write to one invalidates the other's cache
```

Two threads incrementing these two independent counters can run *slower*
than one thread doing both, because they spend their time bouncing the line
back and forth. This is the classic trap: you sharded the state to remove
contention and accidentally reintroduced it in hardware.

### Where a proxy hits it
Per-worker or per-core counters are the prime suspect — exactly the metrics
work in `08-observability/02-metrics.md`. You split a global counter into a
`Vec<AtomicU64>`, one slot per worker, to avoid contention... and pack them
8-to-a-line, so neighboring workers fight over lines anyway. Sharded LRU
locks (`13-algorithms/lru.md`) and per-connection atomic state have the
same exposure.

### The fix: pad to a cache line
Force each hot atomic onto its own line. `crossbeam`'s `CachePadded<T>` does
exactly this, and is the idiomatic answer:

```rust
use crossbeam_utils::CachePadded;
struct Stats {
    requests: CachePadded<AtomicU64>, // now on its own line
    errors:   CachePadded<AtomicU64>,
}
// or a per-worker array:
counters: Vec<CachePadded<AtomicU64>>,
```

Gotcha: padding costs memory — each counter now occupies 64+ bytes instead
of 8. That is the right trade for a handful of contended hot counters and
badly wrong for a million cold ones. Pad the few atomics that are written
concurrently at high rate; never pad reflexively. And note the modern
prefetcher sometimes pulls *pairs* of lines (128-byte effective sharing),
which is why `CachePadded` may pad to 128 on some targets.

### Read-mostly data does not suffer
False sharing is a *write* problem. Many cores reading the same line share
it happily — coherence only fights on writes. So config or routing tables
that are read on every request but written only on reload
(`09-architecture/03-config.md`) are fine to pack tightly; do not pad them.
Reserve padding for concurrently-*written* state.

### It is invisible without measurement
Nothing in the source says "these two fields share a line." The symptom is
throughput that does not scale with cores, or gets *worse* as you add them.
Confirm it before fixing: `perf c2c` (cache-to-cache) is built for exactly
this and points at the contended line. As with everything in this folder,
the trigger is a measurement, not a hunch — see
`08-observability/04-profiling.md`.

## Practice
1. Reproduce it: two threads incrementing two `AtomicU64`s packed in one
   struct, then the same two `CachePadded`. Measure throughput and confirm
   the padded version scales while the packed one does not (or regresses).
2. Build the per-worker counter array both ways (`Vec<AtomicU64>` vs
   `Vec<CachePadded<AtomicU64>>`), drive it from N threads, and plot
   throughput against N — the packed version stops scaling early.
3. Use `perf c2c` to identify the contended line in the packed version and
   confirm it matches the fields you expect.
4. Audit your `proxy` metrics (`08-observability/02-metrics.md`) for
   concurrently-written counters sharing a line; pad only those, and verify
   with a load test that it helps and that cold metrics stayed unpadded.
5. Demonstrate the non-problem: pack read-mostly routing data tightly, show
   concurrent readers do not suffer, and articulate why padding it would
   only waste memory.
