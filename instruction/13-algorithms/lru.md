# LRU and Cache Eviction

`05-http-stack/07-cache.md` covers HTTP caching semantics (freshness, `Vary`,
invalidation). This file covers the eviction policy underneath: what to
throw away when the cache is full.

## What to learn

### LRU: the structure
Least Recently Used evicts the entry untouched for the longest. The
classic implementation pairs a hash map with an intrusive doubly-linked
list: the map gives O(1) lookup, the list gives O(1) move-to-front on hit
and O(1) eviction from the tail.

```rust
struct Node<K, V> {
    key: K,
    value: V,
    prev: Option<usize>, // index into an arena, not a raw pointer
    next: Option<usize>,
}
```

Gotcha: writing this with `Rc<RefCell<Node>>` produces reference cycles
that never free, and writing it with raw pointers means real `unsafe`
(see `03-rust/03-unsafe.md`). The idiomatic Rust answer is an arena — store
nodes in a `Vec` and use `usize` indices as links, which makes the whole
structure safe, compact, and cache-friendly. This is the same technique as
`13-algorithms/slab.md`; a slab is the natural backing store for an LRU.
In production, reach for the `lru` or `moka` crate rather than
hand-rolling.

### The concurrency problem
LRU's fatal flaw in a proxy is that **every read is a write**: a cache hit
must move the entry to the list head, so a shared LRU needs an exclusive
lock on every lookup. Under a proxy's concurrency that single lock becomes
the bottleneck, and it does so precisely when the cache is working well
(high hit rate = maximum lock traffic).

Mitigations, in increasing order of sophistication:
- **Shard** the cache into K independent LRUs keyed by `hash(key) % K`.
  Each has its own lock; contention drops ~K-fold. Eviction becomes
  per-shard rather than global, which is a minor accuracy loss for a large
  win. This is the standard first move.
- **Batch the recency updates**: record hits in a lock-free ring buffer and
  apply them to the list periodically under one lock. This is what `moka`
  and Caffeine do.
- **Approximate LRU** (CLOCK, below), which needs no list at all.

### CLOCK: approximate LRU without the list
CLOCK keeps entries in a circular array, each with one reference bit. A
hit sets the bit — a single atomic store, no lock, no pointer surgery. On
eviction, a hand sweeps the circle: if the bit is set, clear it and move
on; if clear, evict. Entries touched since the last sweep survive one
round, approximating recency closely enough for most workloads at a
fraction of the coordination cost. This is what the Linux page cache uses
(`16-kernel/08-page-cache.md`).

### LRU's blind spot: scans
A single pass over a large set of one-shot items (a crawler walking every
URL, a backup job) evicts the entire working set even though none of the
scanned items will be read again. LRU cannot distinguish "recently used
once" from "used constantly", because it only tracks recency, never
frequency.

LFU tracks frequency instead and resists scans, but adapts poorly when the
working set genuinely shifts — an item popular yesterday keeps a high
count today. The practical answers combine both:
- **ARC** balances a recency list and a frequency list, shifting capacity
  between them based on which is producing hits.
- **TinyLFU / W-TinyLFU** puts a small LRU admission window in front of a
  frequency-based main cache, using a count-min sketch
  (`13-algorithms/count-min-sketch.md`) to estimate frequency in a few
  bits per key. A new item is admitted only if its estimated frequency
  beats the entry it would evict. This is the current default choice — it
  is what `moka` implements — and it is scan-resistant by construction.

Gotcha: measure hit rate against *your* traffic before picking. A proxy
fronting a small set of hot endpoints does fine on plain sharded LRU; the
sophisticated policies earn their complexity on long-tail workloads.

### Sizing by bytes, not entries
An HTTP response cache holds wildly variable object sizes — 200-byte JSON
next to 50 MB videos. A cache bounded by entry *count* has unbounded
memory. Bound by total bytes, and evict in a loop until the incoming
object fits. Cap the maximum cacheable object size too, or one large
response evicts thousands of small hot ones.

## Practice
1. In `labs/10-cache`, implement an arena-backed LRU (indices, not
   pointers) bounded by total response bytes rather than entry count; add
   a max-cacheable-object-size cap.
2. Write the eviction test that matters: insert until full, verify the
   least-recently-*read* entry is evicted, not the least-recently-inserted.
3. Demonstrate the lock bottleneck: drive the cache from 8 concurrent
   tasks at a high hit rate with a single `Mutex<Lru>`, record throughput,
   then shard it 16 ways and re-measure.
4. Reproduce scan pollution: build a hot working set, confirm a high hit
   rate, then run one pass over 10x as many unique cold keys and measure
   the hit rate afterward.
5. Implement CLOCK as an alternative policy behind the same trait; compare
   hit rate and concurrent throughput against sharded LRU on both the hot
   workload and the scan workload from step 4.
