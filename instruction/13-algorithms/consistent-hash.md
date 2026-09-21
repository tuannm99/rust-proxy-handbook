# Consistent Hashing

`06-proxy/02-load-balancer.md` covers the ring lookup and why removal remaps
only ~1/N of keys. This file covers what that summary skips: virtual node
sizing, the balance problem, and bounded loads.

## What to learn

### Why `hash(key) % N` fails
With N upstreams, `hash % N` assigns keys deterministically — until N
changes. Going from 4 upstreams to 5 changes the modulus for *every* key:
roughly 80% of keys remap. For session affinity that means mass session
loss; for an upstream-side cache it means a near-total cache miss storm
hitting your origins at the exact moment you were trying to add capacity.
Consistent hashing exists to make that fraction 1/N instead.

### Virtual nodes and the balance problem
Placing each upstream at a single point on the ring gives terrible
balance: with 4 upstreams, the four arcs between them are randomly sized,
and a 2-3x load imbalance is routine. Virtual nodes fix this by placing
each upstream at V distinct ring positions (`hash("upstream-1#0")`,
`hash("upstream-1#1")`, ...), so each upstream owns V small arcs whose
total length concentrates near the mean as V grows.

The standard deviation of load falls roughly as `1/sqrt(V)`. That sets the
practical range: V=1 is unusable, V=10 still shows ~30% spread, V=100-200
lands within a few percent, and V=1000 buys little for 10x the memory.

```rust
use std::collections::BTreeMap;

struct Ring {
    // ring position -> upstream index; V entries per upstream
    nodes: BTreeMap<u64, usize>,
    vnodes_per_upstream: usize, // 100-200 typical
}
```

Gotcha: memory and rebuild cost are `O(upstreams × V)`. At 1000 upstreams
× 200 vnodes that is 200k `BTreeMap` entries rebuilt on every membership
change — which is when rendezvous hashing
(`13-algorithms/rendezvous-hash.md`, no structure to rebuild) or Maglev
(`13-algorithms/maglev.md`, O(1) lookup) becomes the better answer.

### Weights
Weighted consistent hashing is expressed as vnode count: a weight-3
upstream gets 3x the virtual nodes of a weight-1 one, so it owns ~3x the
ring. This is simple but coarse — the ratio is only as accurate as the
vnode count allows, so a weight-1 upstream needs enough vnodes in absolute
terms (not just relative) for its share to be stable.

### Bounded-load consistent hashing
Plain consistent hashing is oblivious to actual load: if one key is
extremely hot, its owning upstream is overwhelmed while the rest idle.
Consistent hashing *with bounded loads* fixes this by capping each
upstream at `c × average_load` (c slightly above 1, e.g. 1.25); when the
walk lands on an upstream already at its cap, it continues clockwise to
the next one with spare capacity.

This preserves affinity for the common case while degrading gracefully
under hot keys, and it is what makes consistent hashing safe to use as a
general-purpose balancer rather than only for cache routing. Envoy and
HAProxy both ship a variant of it.

Gotcha: the cap must be computed against *current* average load and
recomputed as load changes, and the overflow walk must be bounded — a
naive implementation with all upstreams at cap walks the entire ring on
every request.

### Where it actually belongs in a proxy
Use consistent hashing when the upstream keeps per-key state that is
expensive to rebuild: an upstream-side cache, a sticky session, a shard
owner. Do *not* use it as a default balancer for stateless upstreams —
least-connection or smooth WRR reacts to real load, and consistent hashing
deliberately does not.

## Practice
1. In `labs/06-load-balancer`, build the ring with V=1 and hash 100k keys
   across 5 upstreams; record the per-upstream percentages and the ratio
   between the busiest and idlest.
2. Repeat at V = 10, 100, 500. Plot max/min ratio against V and confirm it
   tightens roughly as `1/sqrt(V)`; pick the V you would actually ship and
   justify it.
3. Measure ring build time and memory at 1000 upstreams × 200 vnodes, then
   compare against the HRW implementation from
   `13-algorithms/rendezvous-hash.md` for the same pool.
4. Implement bounded loads: cap each upstream at 1.25× average and walk on
   overflow. Send 50% of traffic to one hot key and confirm the load
   spreads instead of pinning one upstream.
5. Verify the overflow walk terminates when every upstream is at cap —
   write the test that would have caught an unbounded walk.
