# TinyLFU / W-TinyLFU

`13-algorithms/lru.md` introduces TinyLFU in outline as "a small LRU
admission window in front of a frequency-based main cache." This file is
the deep dive: it's the eviction/admission design most production Rust
and JVM caches actually ship (`moka`, Caffeine), and it's worth
understanding why it wins over ARC's and LFU's more direct approaches.

## What to learn

### The reframing: admission, not eviction
Every policy so far (LRU, LFU, ARC) answers "which resident entry do I
evict?" TinyLFU asks a different question first: **"should this new,
non-resident item be let in at all?"** A candidate is admitted only if its
estimated access frequency is higher than the frequency of the entry that
would have to be evicted to make room for it. If not, the candidate is
simply dropped — the cache's contents don't change, and no eviction
bookkeeping happens at all.

This one reframing is what makes TinyLFU scan-resistant *by construction*:
a one-shot scan's items have frequency ~1, essentially always lower than
anything already resident and warm, so they are rejected at the door
instead of flushing the working set the way a scan flushes LRU.

### The frequency estimator: count-min sketch, not a counter per key
Storing an exact counter per key (as plain LFU does) costs memory
proportional to the number of distinct keys ever seen, which is unbounded
for a proxy cache. TinyLFU instead uses a count-min sketch
(`13-algorithms/count-min-sketch.md`) — fixed-size, small (a few bits per
expected key), with bounded overestimation error and no per-key
allocation at all.

```rust
struct TinyLfu {
    sketch: CountMinSketch, // 13-algorithms/count-min-sketch.md
    total_increments: u64,
    reset_threshold: u64,   // e.g. 10x the sketch's expected key count
}
// admit(candidate_key, victim_key):
//   sketch.estimate(candidate_key) > sketch.estimate(victim_key)
```

Gotcha: without periodic aging, sketch counters only grow and eventually
saturate, at which point the estimator loses all discriminating power —
every popular-enough key looks equally "infinitely popular." Halve every
counter in the sketch once `total_increments` crosses `reset_threshold`.
This is the same aging problem `13-algorithms/lfu.md` has with exact
counters, solved here by periodically resetting a fixed-size structure
instead of decaying a growing map.

### The doorkeeper: don't let one-hit-wonders pollute the sketch
A cache fronting arbitrary internet traffic sees enormous numbers of
keys accessed exactly once, ever. Recording every one of them into the
count-min sketch wastes sketch capacity and adds noise that degrades
estimates for keys that matter. The standard fix is a **doorkeeper**: a
Bloom filter that a key must pass through twice before it's counted in
the sketch at all — the first access only sets the Bloom bit; only the
*second* access (a real repeat) increments the sketch. This keeps the
sketch's limited bits allocated to keys that have shown up more than
once.

### W-TinyLFU: adding back a recency window
Pure frequency-based admission has its own blind spot: a *newly* hot item
has frequency 0 the first time it's seen and would never be admitted on
frequency alone, no matter how hot it's about to become — a cold-start
problem for anything genuinely new. **W-TinyLFU** (windowed TinyLFU) fixes
this by carving off a small slice of the cache (commonly ~1%) as a plain
LRU **window**, and running admission/TinyLFU logic only on the remaining
~99% **main** cache:

- New entries always enter the small LRU window first.
- An entry evicted from the window competes for admission into the main
  cache via the TinyLFU frequency check above.
- The main cache itself is typically a **Segmented LRU (SLRU)**: a
  *probationary* segment (recently admitted) and a *protected* segment
  (survived a second access) — conceptually close to LFU's "seen more
  than once" tier, but LRU-ordered within each segment rather than
  frequency-bucketed.

This is exactly `moka`'s and Caffeine's design: LRU window for cold-start
recency, count-min-sketch-gated admission for scan resistance, SLRU main
cache for cheap approximate frequency tiering — no ARC-style four-list
bookkeeping, no plain-LFU exact-counter memory cost.

### Why this beats ARC and plain LFU in practice
TinyLFU's frequency estimator is O(1) fixed memory regardless of key
cardinality (ARC and exact LFU are not — their bookkeeping scales with
distinct keys touched, even ghost/evicted ones). Its admission-first
design means most of the "is this worth caching" decision is a handful of
sketch lookups, not a multi-list promotion dance. The tradeoff is that the
frequency signal is approximate (bounded overestimation, never
underestimation, from the count-min sketch) rather than exact — a
tradeoff that in measured practice (Caffeine's own published benchmarks)
costs essentially nothing in hit rate while costing much less in memory
and CPU than ARC or ideal-LFU.

## Practice
1. In `labs/10-cache`, implement the count-min-sketch-based frequency
   estimator with periodic halving, then wire it into an admission check
   in front of your existing LRU (`13-algorithms/lru.md`) as the main
   cache — this is TinyLFU without the windowed cold-start fix yet.
2. Reproduce the cold-start problem on purpose: introduce a brand-new key
   that is about to become very hot, and confirm frequency-only admission
   rejects it every time at first. Then add the small LRU window in front
   and confirm the same key gets a chance to prove itself before being
   judged on frequency.
3. Add the doorkeeper Bloom filter and measure sketch quality (compare
   estimated vs. true frequency for a known set of keys) with and without
   it, on a trace dominated by one-hit-wonders.
4. Run the same three-workload comparison you used for `13-algorithms/arc.md`
   (steady hot set, periodic scan, shifting popularity) against your
   W-TinyLFU implementation, and compare hit rate *and* memory usage
   against ARC and plain LRU on identical traces.
