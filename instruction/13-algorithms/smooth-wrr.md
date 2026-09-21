# Smooth Weighted Round Robin

The algorithm behind nginx's `weight=` directive. `06-proxy/02-load-balancer.md`
introduces WRR and says picks should be "interleaved rather than bursty" —
this file is how that interleaving is actually produced.

## What to learn

### Why naive WRR is not good enough
The obvious weighted round robin expands weights into a list
(`[a,a,a,b]` for weights 3 and 1) and cycles it. That satisfies the
*average* ratio but produces a bursty sequence: `a,a,a,b,a,a,a,b`. Three
consecutive requests hit `a` before `b` gets any, so `a`'s connection
count spikes and recovers on a 4-request cycle. With larger weights
(`weight=100` vs `weight=1`) the burst is 100 requests long — long enough
to matter for tail latency and for least-connection-style metrics
observed by anything downstream.

Smooth WRR produces `a,a,b,a` for the same weights: same 3:1 ratio, but
`b` appears as early as its weight allows.

### The current-weight algorithm
Each upstream carries two numbers: a static `weight` (from config) and a
mutable `current_weight`. On every pick:

1. Add each upstream's `weight` to its `current_weight`.
2. Select the upstream with the largest `current_weight`.
3. Subtract `total_weight` (the sum of all static weights) from the
   selected upstream's `current_weight`.

```rust
struct WeightedUpstream {
    weight: i64,          // static, from config
    current_weight: i64,  // mutable selection state
}
```

Step 3 is the whole trick: the winner goes deeply negative and has to
climb back over several rounds, during which lower-weight peers get their
turn. The sequence is provably periodic with period `total_weight`, and
over one period each upstream is selected exactly `weight` times — you get
the exact ratio *and* even spacing, with no expanded list to store.

Gotcha: `current_weight` must be signed. Selecting a weight-1 upstream out
of a pool with `total_weight = 100` drives its value to -99; clamping at
zero (or using an unsigned type) destroys the ratio, because the upstream
no longer has to "pay back" its turn before being eligible again.

### Effective weight and passive failure response
nginx carries a third number, `effective_weight`, which is what actually
gets added in step 1. On a failed request to an upstream it decrements
that upstream's `effective_weight`; on success it increments it back,
capped at the configured `weight`. The result is a load balancer that
gradually sheds traffic from a degrading upstream and gradually restores
it — without any active health check (`06-proxy/03-healthcheck.md`) firing.

This is *passive* health checking expressed purely as weight arithmetic,
and it composes: the smooth-WRR selection loop is unchanged, it just reads
a number that drifts with observed success rate.

### Cost and concurrency
Selection is O(N) over the pool per request, with N = number of upstreams
— fine for the tens-of-upstreams case a single proxy typically has, and
cheaper in practice than it looks because the state is a small contiguous
array. It is not lock-free: the entire pick-and-update must be atomic with
respect to other pickers, or two concurrent requests read the same
`current_weight` and both select the same upstream.

Gotcha: a per-pick `Mutex` around the pool becomes the proxy's central
contention point at high request rates — every request serializes on it.
The usual fixes are sharding the balancer per worker thread (each thread
keeps its own `current_weight` array, accepting per-thread rather than
global ratio accuracy) or moving to an algorithm with no shared mutable
state at all, like rendezvous hashing (`13-algorithms/rendezvous-hash.md`).

## Practice
1. Implement smooth WRR in `labs/06-load-balancer` for weights `[5,1,1]`
   and print the first 21 picks (three full periods). Confirm each period
   contains exactly 5/1/1 selections and that the weight-1 upstreams are
   spread out rather than adjacent.
2. Diff it against naive expanded-list WRR on the same weights: print both
   sequences side by side and identify the longest run of identical picks
   in each.
3. Change `current_weight` to an unsigned type (or clamp it at 0) and
   re-run — observe the ratio break, and write down why in one sentence.
4. Add `effective_weight`: mark one upstream as failing 50% of requests,
   decrement/increment on failure/success, and plot the traffic share it
   receives over 1000 requests as it degrades and recovers.
5. Benchmark selection under 8 concurrent tasks with a single `Mutex`
   around the pool, then with per-task sharded state; compare throughput
   and the resulting global ratio accuracy.
