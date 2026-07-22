# Maglev Hashing

Google's load balancer hashing scheme: O(1) lookup, near-perfect balance,
minimal disruption. The choice when consistent hashing's ring
(`13-algorithms/consistent-hash.md`) is too slow or too skewed.

## What to learn

### What it optimizes for
Maglev precomputes a fixed-size **lookup table** of size M (a prime, e.g.
65537), where each slot holds an upstream index. Lookup is then:

```rust
fn pick(table: &[usize], key_hash: u64) -> usize {
    table[(key_hash % table.len() as u64) as usize]
}
```

One modulo and one array index — O(1), cache-friendly, no tree walk, no
per-upstream hashing. All the work moves to table construction, which
happens only when pool membership changes.

The tradeoff versus consistent hashing is explicit: Maglev accepts a
*slightly* higher remap fraction on membership change in exchange for
near-perfect balance and constant-time lookup.

### Table construction: permutations
Each upstream generates a **preference list** — a permutation of all M
slots, describing the order in which it would like to claim them. The
permutation is derived from two independent hashes of the upstream's name:

```
offset = h1(upstream) % M
skip   = (h2(upstream) % (M - 1)) + 1
permutation[j] = (offset + j * skip) % M
```

Because M is prime and `skip` is in `1..M`, stepping by `skip` visits
every slot exactly once before repeating — that is what makes each
upstream's preference list a true permutation, and it is precisely why M
must be prime. Pick M composite and `skip` sharing a factor with M, and
the upstream can only ever reach a fraction of the table.

### Table construction: the population loop
Fill the table by round-robin across upstreams: each upstream, in turn,
proposes its next-preferred slot; if that slot is empty it claims it,
otherwise it advances through its own preference list until it finds an
empty one. Repeat until all M slots are filled.

Each upstream ends up owning within one slot of M/N — that is the
near-perfect balance. Weights are applied by letting a weight-w upstream
take w turns per round instead of one.

Gotcha: the "advance until empty" step is where a naive implementation
goes quadratic. Each upstream must keep its own cursor into its
preference list across rounds; restarting the search from the beginning
each time turns an O(M log M)-ish fill into O(M²) and becomes visible as a
multi-second stall on every config reload.

### Disruption on membership change
Removing an upstream frees its slots, and the rebuild redistributes them —
but the population order shifts slightly for everyone, so the remap
fraction is a bit above the theoretical 1/N (typically single-digit
percent worse). Increasing M reduces this: M should be at least ~100x the
upstream count, which is why 65537 is a common default for pools in the
hundreds.

Gotcha: the table is only deterministic across proxy instances if every
instance agrees on the upstream *set*, the *ordering* used in the
population round-robin, and the hash functions. Sort upstreams by a stable
id before building, and pin the hash — otherwise two instances build
different tables from identical config and affinity silently breaks
between them.

### When not to use it
Maglev's cost is the rebuild: O(M) work and an M-sized table per pool. For
a handful of upstreams behind one proxy, a `BTreeMap` ring or plain HRW
(`13-algorithms/rendezvous-hash.md`) is simpler, rebuilds instantly, and
the O(log N) or O(N) lookup is not your bottleneck. Maglev earns its
complexity at hundreds-to-thousands of upstreams and high request rates.

## Practice
1. In `labs/06-load-balancer`, implement permutation generation for M=65537
   and assert that one upstream's preference list visits all M slots
   exactly once — this is the test that catches a non-prime M.
2. Implement the population loop with a per-upstream cursor; build a table
   for 10 upstreams and confirm each owns within ±1 slot of M/10.
3. Time table construction at 10, 100, and 1000 upstreams. Then
   deliberately restart the preference scan from index 0 each round and
   re-time it — confirm the quadratic blowup.
4. Remove one upstream from 10, rebuild, and measure what fraction of the
   M slots changed owner. Compare to the ~1/10 your consistent-hash ring
   achieved in `13-algorithms/consistent-hash.md`'s exercise.
5. Build the same table twice from the same upstream set in shuffled input
   order; confirm the tables are identical only once you sort by stable id
   first.
6. Benchmark lookup latency for Maglev vs ring vs HRW at 1000 upstreams.
