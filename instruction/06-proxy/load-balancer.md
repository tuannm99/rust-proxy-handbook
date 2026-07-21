# Algorithms
- Round Robin
- Least Connection
- Consistent Hash

Deeper variants (smooth WRR internals, rendezvous hashing, Maglev) live in
`13-algorithms/`.

## What to learn

### Round Robin (+ weighted)
Simplest algorithm: cycle through upstreams in order via an atomic counter
modulo pool size. Weighted round robin (WRR) biases the cycle so a
weight-3 upstream is picked 3x as often as a weight-1 one — implemented via
"smooth WRR" (nginx's algorithm) so picks are interleaved rather than
bursty (3,3,3,1 instead of 1,1,1,3,3,3).

```rust
use std::sync::atomic::{AtomicUsize, Ordering};

struct RoundRobin {
    next: AtomicUsize,
}

impl RoundRobin {
    fn pick<'a>(&self, upstreams: &'a [Upstream]) -> &'a Upstream {
        let i = self.next.fetch_add(1, Ordering::Relaxed) % upstreams.len();
        &upstreams[i]
    }
}
```
Gotcha: plain round robin ignores load — if one upstream is slow, it still
gets an equal share of new requests and its queue backs up.

### Least Connection
Pick the upstream with the fewest `active_conns` right now. Better than
round robin under uneven request cost (some requests are cheap, some
expensive) because it reacts to actual load, not just count. Needs an
accurate, low-overhead `active_conns` counter per upstream (see
`upstream.md`) — a scan over N upstreams per pick is fine for tens of
upstreams, not for thousands (use a heap if you need to scale further).

Gotcha: least-connection can thundering-herd onto a newly-recovered
upstream (0 connections looks maximally attractive) — combine with a slow
start / connection ramp-up.

### Consistent Hash
Used when you need the *same* client (or cache key) to keep landing on the
same upstream — session affinity, or upstream-side caching. Hash the
upstream onto a ring (often with 100+ virtual nodes per upstream to smooth
distribution), hash the request key, walk clockwise to the first node.

```rust
fn ring_lookup(ring: &std::collections::BTreeMap<u64, usize>, key_hash: u64) -> usize {
    ring.range(key_hash..).next()
        .or_else(|| ring.iter().next())
        .map(|(_, &upstream_idx)| upstream_idx)
        .expect("ring is non-empty")
}
```
Gotcha: removing one upstream from an N-upstream ring only remaps ~1/N of
keys (that's the whole point vs. `hash % N`), but *adding* virtual nodes
without care can still skew distribution — always benchmark ring balance,
don't assume it.

## Practice
1. In `labs/06-load-balancer`, implement Round Robin first —
   confirm requests visibly cycle across 3 dummy upstreams.
2. Add Least Connection; write a small load test that sends slow and fast
   requests to different upstreams and confirm LC routes more traffic to
   the faster one than RR would.
3. Implement Consistent Hash keyed on a header (e.g. a session id) and
   verify the same key always lands on the same upstream across restarts
   of the ring construction.
4. Remove one upstream from the ring and measure what fraction of keys
   remap — confirm it's close to 1/N, not 100%.
