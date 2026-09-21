# Binary Heap

`06-proxy/02-load-balancer.md`'s least-outstanding-requests variant needs to
always know which upstream currently has the fewest in-flight requests.
A binary heap is the structure that answers "what's the minimum" in
O(log n) per update instead of scanning every upstream on every request.

## What to learn

### The structure: a complete tree packed into an array
A binary heap stores a complete binary tree directly in a `Vec`, with no
pointers: for index `i`, the parent is at `(i - 1) / 2` and children are
at `2i + 1` and `2i + 2`. The heap invariant — every parent is `<=`
(min-heap) or `>=` (max-heap) than its children — is maintained by two
operations: **sift-up** (a new or decreased element bubbles toward the
root while smaller than its parent) and **sift-down** (the root, after
removal, sinks while larger than its smallest child).

```rust
// conceptually what std::collections::BinaryHeap does internally,
// as a min-heap over (outstanding_count, upstream_id)
fn sift_down(heap: &mut Vec<(u32, usize)>, mut i: usize) {
    loop {
        let (l, r) = (2 * i + 1, 2 * i + 2);
        let mut smallest = i;
        if l < heap.len() && heap[l] < heap[smallest] { smallest = l; }
        if r < heap.len() && heap[r] < heap[smallest] { smallest = r; }
        if smallest == i { break; }
        heap.swap(i, smallest);
        i = smallest;
    }
}
```

Peek-min is O(1) (it's always the root, index 0); insert and remove-min
are both O(log n).

### The gap std's `BinaryHeap` leaves open: decrease-key
Least-outstanding-requests needs to *change* an upstream's key (its
outstanding count goes up on dispatch, down on completion) and keep the
heap valid — a "decrease-key" operation Rust's `std::collections::BinaryHeap`
doesn't expose at all; it only supports push and pop-max/min. Two
practical fixes:
- **Lazy deletion**: push a new entry with the updated count, leave the
  stale one in place, and when popping, skip (discard) any entry that no
  longer matches the upstream's current count (tracked separately in a
  `HashMap<UpstreamId, u32>`). Simple, but the heap can accumulate stale
  entries between real pops.
- **An indexed heap**: maintain a side table mapping key → heap index, and
  update that table on every swap during sift-up/down, so a decrease-key
  can find its element directly and sift it in O(log n) without a scan.
  More bookkeeping, no stale-entry accumulation.

### d-ary heaps: fewer levels, better cache behavior
A heap with `d` children per node (instead of 2) has depth `log_d(n)` —
fewer levels to sift through — at the cost of comparing up to `d`
children per sift-down step instead of 2. For small `d` (4, sometimes 8)
this is a net win in practice because it does fewer cache-line-crossing
jumps for a similar total comparison count; production priority-queue
implementations (some OS schedulers, some LB implementations) use d-ary
heaps specifically for this reason. See `17-performance/01-cpu-cache.md`
before micro-tuning `d` — measure first.

## Practice
1. In `labs/06-load-balancer`, implement least-outstanding-requests
   using a min-heap keyed by outstanding count; start with the
   lazy-deletion approach for decrease-key.
2. Reproduce the stale-entry problem: dispatch and complete requests
   rapidly and confirm the heap's size grows relative to the number of
   real upstreams; measure how much it costs before a pop clears it out.
3. Replace lazy deletion with an indexed heap (a `HashMap` from upstream
   ID to current heap index, updated on every swap) and confirm the heap
   size stays bounded at the number of upstreams.
4. (Stretch) Implement a 4-ary heap variant and benchmark sift-down cost
   against the binary version at a few hundred upstreams.
