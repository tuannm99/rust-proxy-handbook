# HyperLogLog

`13-algorithms/count-min-sketch.md` answers "how many times has this key
appeared." HyperLogLog answers a different question in similarly tiny
fixed memory: "how many *distinct* keys have appeared" — cardinality
estimation, useful for "how many distinct attacker IPs hit us in the last
minute" (`07-security/09-ddos.md`) without ever storing the set of IPs.

## What to learn

### The core trick: leading zeros as a signal of scale
Hash each incoming key to a uniform random bit string. The position of
the leftmost set bit (equivalently, the count of leading zeros) in that
hash is, on its own, a weak signal of cardinality: seeing a hash with 10
leading zeros is a `1-in-1024` event, so observing one at all suggests
roughly a thousand distinct hashes have been tried. One such observation
is far too noisy to trust alone — HyperLogLog's contribution is averaging
this signal across many independent buckets to control the variance.

### The structure
Split the hash into two parts: a few bits select one of `m` registers
(buckets), the rest is used to compute leading-zero count, and each
register stores the *maximum* leading-zero count seen for any key that
hashed to it:

```rust
struct HyperLogLog {
    registers: Vec<u8>, // m registers, each holds a max leading-zero count
}
// add(key): (bucket, rest) = split(hash(key));
//           registers[bucket] = registers[bucket].max(leading_zeros(rest))
// estimate(): harmonic mean of 2^registers[i] across all buckets,
//             scaled by a bias-correction constant depending on m
```

With `m = 16384` registers (2 KB at one byte each), the standard error is
about `1.04 / sqrt(m) ≈ 0.8%` — and critically, that error bound holds
whether the true cardinality is a thousand or a billion; the memory is
fixed regardless of how many distinct keys actually show up, which is
exactly the property that matters against an attacker who controls key
cardinality.

### Mergeable across shards
Two HyperLogLog structures over disjoint data streams can be combined
into their union's estimate by taking the element-wise **maximum** of
their registers — no need to re-scan either data set. This is why
distributed systems (Redis's `PFCOUNT`/`PFMERGE`, BigQuery/Presto's
`APPROX_COUNT_DISTINCT`) use it: each shard/worker keeps its own small
structure, and a global distinct count is a cheap register-wise merge
away, not a coordination problem.

### Gotcha: it counts "ever seen," not "seen in the last N minutes"
A HyperLogLog register only ever grows (`max`, never decreases), so it
answers "distinct elements since I started counting," not a sliding
window. For a DDoS signal like "distinct attacker IPs in the last
minute," combine it with the same decay/rotation trick
`13-algorithms/count-min-sketch.md` uses: keep one HLL per time bucket,
merge the recent N buckets for the windowed estimate, and drop the
oldest bucket as time advances.

## Practice
1. Implement basic HyperLogLog and validate it against a `HashSet`
   ground truth on a synthetic stream with a known number of distinct
   keys; confirm the error stays within the theoretical bound for your
   chosen `m`.
2. Feed the same key twice and confirm the estimate doesn't change —
   only distinctness matters, not frequency.
3. Build two HyperLogLog structures over disjoint key streams, merge them
   via element-wise max, and confirm the merged estimate is close to the
   true union cardinality.
4. Add time-bucket rotation (one HLL per minute, merge the last 5 for a
   windowed count) and use it as a distinct-IP-rate signal alongside
   `07-security/09-ddos.md`'s volumetric detection; test it against a
   simulated distributed flood from many synthetic source IPs.
