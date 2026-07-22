# Rendezvous Hashing (HRW)

Highest Random Weight: an alternative to consistent hashing
(`13-algorithms/consistent-hash.md`) that needs no ring, no virtual nodes,
and no shared mutable state.

## What to learn

### The algorithm
For a key, compute `hash(key, upstream)` for *every* upstream and pick the
one with the highest value. That is the entire algorithm.

```rust
fn pick<'a>(key: &str, upstreams: &'a [Upstream]) -> &'a Upstream {
    upstreams
        .iter()
        .max_by_key(|u| hash64(key, &u.id))
        .expect("pool is non-empty")
}
```

Because the score depends only on `(key, upstream_id)`, every proxy
instance computes the same answer independently with zero coordination,
and there is no data structure to build, rebuild, or lock.

### Why it remaps minimally
Remove an upstream: only keys whose *winner* was that upstream move, and
they move to whichever upstream had the second-highest score — every other
key's winner is unaffected, because removing a non-winner cannot change
who the maximum was. That is exactly the ~1/N remap fraction consistent
hashing achieves, but derived from the algorithm itself rather than from
tuning virtual node counts.

Adding an upstream is the mirror image: a key moves only if the new
upstream outscores its current winner, which happens for ~1/(N+1) of keys.

### Distribution quality vs consistent hashing
Consistent hashing's balance depends on how evenly the virtual nodes
happen to land on the ring — with too few virtual nodes you get real skew,
which is why 100-200 per upstream is typical (and why the ring costs
memory proportional to `upstreams × vnodes`). Rendezvous has no such knob:
distribution is as uniform as your hash function, full stop. For a proxy
with a modest number of upstreams this is strictly less to get wrong.

Gotcha: the hash must genuinely mix the upstream id into the key's hash.
`hash(key) ^ hash(upstream)` looks fine and is not — XOR with a fixed
per-upstream constant preserves the key's bit structure, so correlated
keys get correlated scores and distribution skews. Hash the concatenation
(or feed both into one hasher), and use a hash with good avalanche
behavior (xxHash, SipHash, FNV-1a at minimum) rather than the default
`DefaultHasher` whose output is not stable across Rust releases — a fact
that matters the moment two proxy instances on different builds must agree
on the same winner.

### Weighted rendezvous
Weights fold in via a log transform: score each upstream as
`weight / -ln(h)` where `h` is the hash normalized to (0,1). The upstream
with the highest transformed score wins, and selection probability comes
out proportional to weight while keeping the minimal-disruption property.
This is materially harder to get right than smooth WRR
(`13-algorithms/smooth-wrr.md`) — reach for it only when you need weights
*and* affinity at once.

### The real cost
Selection is O(N) hashes per request, versus O(log N) for a ring lookup
and O(1) for Maglev (`13-algorithms/maglev.md`). At tens of upstreams,
N hashes of a short string is tens of nanoseconds and the simplicity wins.
At thousands of upstreams it is the wrong choice — that is Maglev's
territory.

## Practice
1. Implement HRW in `labs/06-load-balancer` behind the same trait as your
   round-robin and consistent-hash implementations, so the three are
   swappable.
2. Hash 100k synthetic keys across 10 upstreams and report the per-upstream
   distribution — confirm it is within a few percent of uniform.
3. Remove one upstream, re-run the same 100k keys, and measure the exact
   fraction that changed winners. Confirm it is ~1/10, and that every moved
   key went to a *different* upstream than the removed one.
4. Replace the hash with `hash(key) ^ hash(upstream_id)` and re-run step 2
   with keys sharing a long common prefix (e.g. `session-00001`...);
   observe the skew this introduces.
5. Compare selection latency of HRW vs your ring lookup at 10, 100, and
   1000 upstreams; find the crossover point where the ring wins.
