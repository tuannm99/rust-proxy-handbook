# Skip List

An ordered structure that reaches the same expected O(log n)
search/insert/delete as a balanced tree, but with a much simpler
concurrent story — worth knowing as the "ordered map when a `BTreeMap`
won't do under contention" option.

## What to learn

### The structure: randomized levels, not rebalancing
A skip list is a linked list with extra "express lane" layers: every node
exists in the base (level 0) list; each node is *also* promoted to level
1 with probability `p` (commonly 1/2), to level 2 with probability `p²`,
and so on. A search starts at the top level and drops down a level each
time it would overshoot, skipping most of the base list on the way:

```rust
struct SkipNode<K, V> {
    key: K,
    value: V,
    // one forward pointer per level this node participates in
    forward: Vec<Option<usize>>, // arena indices, as in lru.md
}
```

No rotations, no rebalancing — insertion just flips coins to decide how
many levels the new node joins, then splices it into each of those levels
the way a plain linked-list insert would. Expected search/insert/delete
is O(log n); it's probabilistic rather than a guaranteed worst case, which
is the price paid for not needing tree-rebalancing logic at all.

### Why this matters more than the complexity bound alone suggests
A balanced tree's rebalancing (rotations in a red-black tree, node splits
in a B-tree) touches multiple nodes as one logical operation, which is
awkward to make lock-free or even coarsely concurrent without blocking
wide swaths of the structure. A skip list's insert only ever splices
pointers at the levels the new node joins — a small, local set of
pointer updates — which is why lock-free skip lists (used in some
in-memory databases and the memtable design behind several LSM-tree
storage engines) are common, while lock-free balanced trees are rare and
notoriously hard to get right.

### Where it fits in this handbook
No lab requires a skip list directly, but it's the natural structure for
any "keep entries sorted by an evolving key, and the map changes under
concurrent access" problem — e.g. tracking cache entries by expiry time
so `labs/10-cache` can find "what expires next" without scanning the
whole cache, an alternative to a timer wheel
(`13-algorithms/priority-queue.md`) when exact ordering matters more than
O(1) bucketing.

## Practice
1. Implement a single-threaded skip list as an ordered set, with
   probabilistic level promotion (`p = 0.5`) and arena-backed nodes.
2. Instrument level counts across 10,000 random insertions and confirm
   the level-height distribution roughly matches the geometric
   distribution `p` predicts.
3. Use it in `labs/10-cache` to keep entries ordered by expiry time;
   implement "evict everything expired" as a walk from the smallest key
   instead of a full scan, and compare cost against a linear scan at
   10,000 entries.
4. (Stretch) Read about one lock-free skip list design (e.g. the one used
   in Java's `ConcurrentSkipListMap`) and write down, in your own words,
   why its insert can be done with a bounded number of CAS operations
   where a balanced tree's rebalancing generally cannot.
