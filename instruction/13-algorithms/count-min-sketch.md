# Count-Min Sketch

Approximate frequency counting in fixed memory. How you answer "how many
requests has this IP sent" for millions of IPs without a map that grows
with the attacker's budget (`07-security/ddos.md`).

## What to learn

### The problem: exact counting is an attack surface
A `HashMap<IpAddr, u64>` of request counts is controlled by whoever sends
the requests. A distributed attacker with a million source addresses
creates a million entries — the counter you added to *detect* the attack
becomes the memory-exhaustion vector that completes it. Exact counting of
an adversary-chosen key space is not a viable design.

A count-min sketch gives approximate counts in memory you choose in
advance, independent of how many distinct keys arrive.

### The structure
A 2D array of counters, `d` rows × `w` columns, plus `d` independent hash
functions — one per row.

- **Increment**: for each row `i`, increment `table[i][h_i(key) % w]`.
- **Query**: take the **minimum** across the `d` rows.

```rust
struct CountMinSketch {
    table: Vec<u32>, // d * w, flattened
    width: usize,    // w
    depth: usize,    // d
}
```

Collisions only ever inflate a counter, never deflate it — so every row
gives an overestimate, and the minimum is the tightest bound available.
The count is never too *low*, which is the property that matters for
detection: a heavy hitter can never hide.

### Error bounds
With `w = ceil(e/ε)` and `d = ceil(ln(1/δ))`, the estimate exceeds the true
count by more than `ε × N` (N = total increments) with probability at most
`δ`. For ε=0.001 and δ=0.01 that is roughly 2718 × 5 counters — about 54 KB
at 4 bytes each, for a 0.1% error bound at any key cardinality.

Read that trade honestly: memory is fixed and known, error scales with
*total traffic volume* N, not key count. Under a volumetric flood N is
enormous, so absolute error grows — which is fine for "is this IP a heavy
hitter" and useless for "exactly how many requests did this IP send."

Gotcha: the `d` hash functions must be independent. Reusing one hash with
different seeds is acceptable only if the hash actually mixes the seed
(SipHash with distinct keys, xxHash with distinct seeds); `hash(key) + i`
is not independent and collapses the error guarantee, because two keys
colliding in one row then collide in every row and the minimum stops
helping.

### The heavy-hitter pattern
The sketch alone tells you a count only if you already have the key. Pair
it with a small top-K structure: on each request, increment the sketch,
query the estimate, and if it exceeds a threshold, insert the key into a
bounded min-heap of the worst offenders. The sketch handles unbounded
cardinality in fixed memory; the heap holds only the keys that matter.

This is also how TinyLFU (`13-algorithms/lru.md`) estimates access
frequency for cache admission in a few bits per key.

### Decay: counts must forget
A sketch that only increments is monotone — an IP that was a heavy hitter
an hour ago stays flagged forever, and every counter eventually saturates.
Two standard fixes:
- **Halve every counter** periodically (a "conservative aging" pass).
  Cheap, preserves relative ordering, and is what TinyLFU does.
- **Sliding windows**: keep several sketches, one per time bucket, rotate
  and zero the oldest. Costs `d × w × buckets` memory but gives real
  windowed counts, which is what a rate limiter needs.

Gotcha: `u32` counters saturate under a sustained flood. Use saturating
arithmetic, and size the decay interval so counters cannot reach the
ceiling between passes — a wrapped counter reads as *low* traffic, which
inverts your detection exactly when it is under attack.

### Conservative update
A refinement worth knowing: on increment, update only the rows whose
counter equals the current minimum, leaving already-inflated rows alone.
This measurably reduces overestimation at no memory cost, and only affects
the write path. It gives up the ability to *decrement*, so it is
incompatible with sliding-window schemes that subtract.

## Practice
1. Implement a count-min sketch and validate the error bound empirically:
   feed a Zipf-distributed key stream, compare estimates against exact
   counts from a `HashMap`, and confirm the overestimate stays within
   `ε × N` for the ε you sized for.
2. Show the failure mode: replace the `d` independent hashes with
   `hash(key) + i` and re-run step 1; measure how much the error degrades.
3. In `labs/11-rate-limit`, add a sketch-based global heavy-hitter detector
   alongside the exact per-IP buckets. Compare memory at 1M distinct source
   IPs.
4. Add periodic halving; verify a burst-then-idle key drops below the
   threshold within the expected number of decay passes.
5. Saturate a `u32` counter deliberately and observe what your detector
   reports; then fix it with saturating arithmetic and a decay interval
   that keeps counters in range.
6. Implement conservative update and measure the reduction in average
   overestimation against step 1's baseline.
