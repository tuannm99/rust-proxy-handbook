# ARC (Adaptive Replacement Cache)

`13-algorithms/lru.md` and `13-algorithms/lfu.md` each capture one signal
— recency or frequency — and each has a workload that defeats it (LRU:
one-shot scans; LFU: stale winners). ARC's pitch is that it doesn't make
you pick: it tracks both and *adapts* the balance between them online,
based on which one is actually producing hits for your traffic right now.

## What to learn

### Four lists, not one
ARC splits tracked keys across four LRU-ordered lists:
- **T1** — entries seen exactly once recently (a plain-LRU signal).
- **T2** — entries seen at least twice recently (a frequency signal).
- **B1** — "ghost" list: keys recently evicted from T1 (metadata only, no
  cached value).
- **B2** — ghost list: keys recently evicted from T2.

`T1` and `T2` together hold the actual cached entries, bounded by total
cache capacity `c`. `B1` and `B2` hold only keys and are bounded roughly to
`c` each — they exist purely to detect a pattern the live cache can no
longer see.

```rust
struct Arc<K> {
    t1: LruList<K>, t2: LruList<K>,   // resident, sum of lengths <= c
    b1: LruList<K>, b2: LruList<K>,   // ghost, keys only
    p: usize,                          // target size of T1, adapts over time
    c: usize,                          // total capacity
}
```

### The adaptation rule
`p` is the current target boundary between "recency space" and "frequency
space" within the resident cache. It moves on every ghost-list hit, which
is the mechanism that makes ARC self-tuning:
- A hit in **B1** (a key evicted from the recency list came back) means
  the workload wants more recency capacity — the cache evicted it too
  soon. Increase `p` (grow T1's target share).
- A hit in **B2** (a key evicted from the frequency list came back) means
  the workload wants more frequency capacity. Decrease `p` (shrink T1's
  target share, grow T2's).

No external tuning parameter decides the recency/frequency split — the
cache's own recent miss history decides it, continuously, in one direction
or the other after every ghost hit.

### Why this beats picking LRU or LFU up front
A workload that mixes a small hot set (frequency-favoring) with an
occasional large one-shot scan (recency-agnostic, but must not evict the
hot set) defeats plain LRU (the scan flushes T2-equivalent entries) and
gives plain LFU no fast way to admit newly-hot items. ARC's ghost lists
let it detect, from real miss patterns, that "scans are happening but
shouldn't take frequency's cache share" and correct `p` accordingly,
without a human deciding that in advance. This adaptivity, not raw hit
rate on any one static trace, is ARC's actual contribution.

### Where it's used, and the honest complexity cost
ARC is real production code, not just a paper: ZFS's ARC cache is named
after and implements this algorithm for disk block caching. But it is
meaningfully more state and more bookkeeping than either LRU or LFU alone
— four lists to maintain consistently, and a `p` update on every ghost hit
that must not race with concurrent eviction. (The original algorithm
predates a since-expired IBM patent; it's free to implement today, but
that history is part of why it took years to show up in mainstream
open-source caches.)

Gotcha: the four-list bookkeeping is exactly the kind of thing that's
easy to get subtly wrong under concurrency — a key must never be present
in more than one list at a time, and a promotion from T1 to T2 (on a
second access) must remove-then-insert atomically relative to a
concurrent eviction, or you get a duplicate or a leaked entry.

## Practice
1. In `labs/10-cache`, implement ARC as a fourth eviction policy behind
   your shared trait, using arena-backed lists (`13-algorithms/lru.md`'s
   technique) for all four lists.
2. Reproduce the scenario ARC is meant for: a small steady hot set plus a
   periodic large one-shot scan. Run plain LRU, plain LFU
   (`13-algorithms/lfu.md`), and ARC on the same trace and compare hit
   rate — confirm ARC's `p` shifts toward T1 during the scan and recovers
   afterward without losing the hot set.
3. Add an assertion that no key ever appears in more than one of the four
   lists simultaneously, and run it under concurrent access to catch a
   promotion/eviction race.
4. (Stretch) Log `p` over time during a trace with a gradually shifting
   workload (recency-favoring, then frequency-favoring) and confirm it
   tracks the shift without manual retuning.
