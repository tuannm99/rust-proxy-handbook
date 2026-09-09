# LFU (Least Frequently Used)

`13-algorithms/lru.md` covers recency-based eviction and its blind spot
against scans. This file covers the frequency-based alternative and why
it isn't a straightforward drop-in fix.

## What to learn

### The basic idea, and the naive implementation's cost
LFU evicts the entry with the lowest access count, on the theory that
frequently-used items are more valuable to keep than recently-used ones.
The naive implementation — a hash map to a count, and a linear scan to
find the minimum on eviction — is O(n) per eviction, which is the reason
LFU has a reputation for being slower than LRU.

### O(1) LFU: two levels of linked structure
The standard O(1) construction (Ketan Shah et al.) keeps a doubly-linked
list of *frequency* buckets, each holding a doubly-linked list of the keys
currently at that frequency:

```rust
struct FreqNode<K> {
    freq: u64,
    keys: std::collections::HashSet<K>, // or an intrusive list, for O(1) removal
    prev: Option<usize>,
    next: Option<usize>, // arena indices, as in lru.md — same reasoning applies
}
```

A hit removes the key from its current frequency bucket, increments its
count, and inserts it into the bucket for the new count (creating that
bucket if it doesn't exist, right after the old one in the list). Eviction
removes a key from the *head* frequency bucket (the lowest count present)
— never a scan. Both operations are O(1) amortized, same complexity class
as LRU, at the cost of a more intricate structure and more bookkeeping per
access.

### The problem LRU doesn't have: stale winners
LFU's count only ever goes up (in the plain form above), so an item that
was extremely popular yesterday and is never touched again keeps a high
count forever and is effectively unevictable — it starves out items that
are popular *right now*. This is the mirror image of LRU's scan
vulnerability: LRU forgets frequency entirely, plain LFU never forgets it.

Mitigation is **aging/decay**: periodically halve every count (or decay
multiplicatively on a timer, or on every Nth access), so old popularity
fades and new popularity can compete on a level footing. This turns plain
LFU into an approximation of "frequency over a recent window" rather than
"frequency ever," which is almost always what you actually want in a
cache.

### Why production caches don't ship plain LFU
Between the bucket-list complexity and the aging tuning problem, plain LFU
is rarely used as-is in production caches. Two directions fix it
differently:
- **ARC** (`13-algorithms/arc.md`) tracks both recency and frequency and
  adapts the balance between them automatically, without a manual decay
  knob.
- **TinyLFU** (`13-algorithms/tinylfu.md`) keeps the frequency *idea* but
  replaces the exact counter with a probabilistic count-min sketch
  (`13-algorithms/count-min-sketch.md`) that has built-in periodic aging,
  and uses it only as an *admission* filter in front of a much simpler
  main structure (typically LRU-based) rather than as the eviction policy
  for the whole cache.

Gotcha: don't reach for hand-rolled LFU in `proxy/` — it's here so you can
recognize the tradeoff by name and understand what ARC and TinyLFU are
actually improving on. Production systems (`moka`, Caffeine) use
TinyLFU-derived designs, not plain LFU, for exactly the staleness reason
above.

## Practice
1. In `labs/10-cache`, implement the O(1) LFU structure (frequency-bucket
   list of key lists) behind the same eviction trait you used for LRU.
2. Reproduce the staleness bug on purpose: make one key extremely popular,
   stop touching it, then flood the cache with a different, shifting
   working set — confirm the stale key never gets evicted despite being
   cold, and measure the resulting hit-rate loss versus LRU on the same
   trace.
3. Add periodic count halving (aging) and rerun the same trace; confirm
   the stale key eventually becomes evictable and hit rate recovers.
4. Compare implementation complexity and eviction correctness against
   your `13-algorithms/lru.md` arena-backed LRU on the same benchmark
   harness, and write down, concretely, which workload shape (steady hot
   set vs. shifting popularity vs. one-shot scan) favors which policy.
