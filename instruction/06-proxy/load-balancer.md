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

Gotcha: `fetch_add` on a shared counter is a contended cache line across
every worker thread on every request (`17-performance/false-sharing.md`) —
at high request rates this single atomic becomes measurable. A per-worker
counter, each starting at a different offset, gets the same distribution
with no cross-core traffic at all, and is the standard fix once round
robin shows up in a profile.

Gotcha: `% upstreams.len()` on a *changing* pool silently reshuffles
everything when the length changes by one (`service-discovery.md`) — fine
for stateless round robin, fatal if anything downstream assumed stability.
That difference is exactly why consistent hashing exists.

### Least Connection
Pick the upstream with the fewest `active_conns` right now. Better than
round robin under uneven request cost (some requests are cheap, some
expensive) because it reacts to actual load, not just count. Needs an
accurate, low-overhead `active_conns` counter per upstream (see
`upstream.md`) — a scan over N upstreams per pick is fine for tens of
upstreams, not for thousands (use a heap, `13-algorithms/heap.md`, if you
need to scale further).

Gotcha: least-connection can thundering-herd onto a newly-recovered
upstream (0 connections looks maximally attractive) — combine with a slow
start / connection ramp-up (`healthcheck.md`).

Gotcha, and this one is structural: **your connection counts are local.**
With M proxy instances, each one knows only the connections *it* opened.
Every instance independently computes "upstream 7 is least loaded" from
its own partial view, and they all send the next request there
simultaneously. The algorithm that was supposed to spread load has
synchronized M proxies onto one host. Least-connection's quality degrades
as proxy count rises, which is precisely the opposite of what you want
from a scaling story — and is the reason for the next section.

### Power of two choices (P2C)
Instead of scanning for the global minimum, pick **two upstreams at
random** and take the less loaded of the two:

```rust
fn pick_p2c<'a>(upstreams: &'a [Upstream]) -> &'a Upstream {
    let (a, b) = two_distinct_random_indices(upstreams.len());
    if upstreams[a].load() <= upstreams[b].load() { &upstreams[a] } else { &upstreams[b] }
}
```

This is the single highest-value algorithm to know here, for two reasons.
It's O(1) per pick regardless of pool size — no scan, no heap, no sorted
structure. And the randomness is what fixes the herd above: two proxies
choosing independently rarely sample the same pair, so they don't converge
on the same "best" host the way exact least-connection does. The classic
result ("The Power of Two Random Choices") is that sampling two instead of
one drops maximum load from `O(log n / log log n)` to `O(log log n)` — an
exponential improvement — while sampling more than two adds almost
nothing. It is the default in linkerd and available in Envoy, and it
should generally be your default too.

Gotcha: P2C is only as good as the load metric you compare. With
`active_conns` it inherits the cancellation-leak bug from `upstream.md`
(a leaked counter makes a healthy host permanently unattractive); with
latency it inherits the cold-host problem below.

### Latency-aware: peak EWMA
Connection count is a proxy for load, not load itself — an upstream with 4
fast requests is less loaded than one with 3 slow ones. Peak EWMA scores
each upstream by an exponentially-weighted moving average of its observed
response latency, multiplied by its outstanding request count, and picks
the lowest score (usually combined with P2C rather than a global scan).
It reacts to a host that has become slow without having failed — the
partial-degradation case that health checks miss entirely
(`healthcheck.md`).

Gotcha: a host that receives no traffic has no recent latency samples, so
its EWMA is stale — and stale-and-fast looks like the best host in the
pool, which sends it a burst. Decay the average toward a pessimistic
default over time, or treat "no recent sample" as a distinct state rather
than as a great score. The same reasoning applies to a host that just came
back from a circuit-breaker open state.

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

Gotcha: consistent hashing distributes *keys* evenly, which is not the
same as distributing *load* evenly. One hot key — a single celebrity
account, one cache entry everyone wants — lands entirely on one upstream
and no amount of virtual nodes helps, because the ring is doing exactly
what you asked. The mitigation is "consistent hashing with bounded loads":
if the chosen upstream is above a load threshold, walk to the next node on
the ring instead. You give up strict affinity for the overloaded slice of
traffic, which is the right trade when the alternative is one host melting.

Gotcha: the hash must be stable across processes and restarts. A
`DefaultHasher` from `std` is explicitly not stable across Rust releases,
and `HashMap`'s `SipHash` is seeded randomly per process (see
`13-algorithms/hashmap.md`) — two proxy instances using it would build
*different* rings from the same config and disagree about every key. Use a
fixed-seed, explicitly-specified hash (xxHash, or SipHash with a constant
key) for anything whose result must match across processes.

### Choosing between them
A short decision guide, since this is the actual question when you sit
down to write `proxy/`:
- No affinity needed, uniform request cost, small pool → round robin. It
  is cheap and its weaknesses don't bite.
- No affinity needed, variable request cost → **P2C over a load metric**.
  This is the sane default for a real proxy.
- Partial degradation is your main risk (slow hosts, not dead ones) →
  P2C over peak EWMA.
- Affinity required (upstream cache, sticky session) → consistent hash,
  with bounded loads if any key can be hot.

## Practice
Build these in order — each one needs the previous one's measurements to
be judged against.

1. In `labs/06-load-balancer`, implement round robin. **Done when** a load
   test shows requests distributed within ±1% across 3 equal dummy
   upstreams.
2. Add a deliberately slow upstream (inject 200ms into one backend) and
   re-run the same test. **Done when** you can show round robin still
   sends it a third of traffic and your p99 is wrecked — this is the
   baseline every later algorithm must beat.
3. Add least connection. **Done when** the slow-upstream test shows
   measurably less traffic going to the slow host and a better p99 than
   step 2, with numbers recorded for both.
4. Reproduce the distributed herd: run 3 instances of your balancer
   against the same pool, all using exact least-connection, and log which
   upstream each picks per request. **Done when** you can show the
   instances converging on the same upstream simultaneously.
5. Implement P2C over the same counter. **Done when** re-running step 4
   shows the convergence gone, and re-running step 3 shows p99 at least as
   good as exact least-connection — at O(1) instead of O(n) per pick.
6. Implement peak EWMA scoring behind the same trait and combine with P2C.
   **Done when** a test with one upstream that is *slow but healthy*
   routes traffic away from it faster than connection-count-based
   scoring does, and a test with an idle upstream shows it does not
   receive a burst when it first gets traffic.
7. Implement consistent hash keyed on a header. **Done when** the same key
   lands on the same upstream across a full process restart (proving your
   hash is stable), and removing one upstream from a 5-host ring remaps
   close to 20% of keys — measure it, don't assume.
8. Add bounded loads to the ring. **Done when** a test where 90% of
   requests share one key spreads that traffic across multiple upstreams
   instead of melting one, while low-volume keys keep perfect affinity.
