# Bloom Filter

`13-algorithms/tinylfu.md` uses a Bloom filter as a "doorkeeper" to keep
one-hit-wonders out of its frequency sketch. This file covers the
structure itself: fast, tiny, probabilistic *membership* testing with no
false negatives.

## What to learn

### The structure and the guarantee
An `m`-bit array and `k` independent hash functions. **Insert**: set bits
`h_1(key) % m`, ..., `h_k(key) % m`. **Query**: report "possibly present"
if all `k` bits are set, "definitely absent" otherwise.

```rust
struct BloomFilter {
    bits: Vec<u64>, // m bits, packed
    k: usize,
}
// insert/query both compute k hash positions and set/check bits
```

Because insertion only ever sets bits, a query can never produce a false
*negative* — if the bits say absent, the key was never inserted, full
stop. It can produce a false *positive*: enough other keys' bits happened
to cover this key's positions by coincidence.

### Sizing: the false-positive rate is a knob, not an accident
For `n` expected insertions, the optimal number of hash functions is
`k = (m/n) * ln(2)`, and the resulting false-positive rate is
approximately `(1 - e^(-kn/m))^k`. Concretely: 10 bits per expected
element with the optimal `k` (~7) gives roughly a 1% false-positive rate,
independent of what the keys actually are. Decide the acceptable
false-positive rate first, then size `m` and `k` from it — don't pick
round numbers and hope.

### Where it earns its place in this handbook
- **TinyLFU's doorkeeper** (`13-algorithms/tinylfu.md`): a key must appear
  twice before it's counted in the frequency sketch, and the Bloom
  filter is the cheap first-appearance check.
- **A first-pass IP/rule blocklist check** (`07-security/waf.md`,
  `07-security/ip-filtering.md`): checking a large deny-list is often
  dominated by "the common case is not on the list" — a Bloom filter in
  front of the real lookup answers "definitely not blocked" for most
  traffic in O(k) with no memory access outside the filter itself, and
  only falls through to the expensive real check on a possible hit.

### Gotcha: no removal, and no over-capacity
A plain Bloom filter cannot un-set a bit for one key without possibly
breaking membership for every other key that also set that bit — removal
needs a different structure (a **counting Bloom filter**, which uses
small counters instead of single bits, at proportionally more memory).
And a filter sized for `n` elements that receives significantly more than
`n` insertions sees its false-positive rate climb sharply past the
number you designed for — size for your actual expected cardinality, and
monitor actual insertions against it if that number can grow unbounded.

## Practice
1. Implement a Bloom filter sized for a chosen `n` and false-positive
   target; empirically measure the actual false-positive rate against a
   `HashSet` ground truth and confirm it matches the formula.
2. Use it as a doorkeeper in front of your `13-algorithms/tinylfu.md`
   frequency sketch (`labs/10-cache`) and confirm one-hit-wonder keys
   never reach the sketch.
3. Build a first-pass check in front of an IP blocklist
   (`labs/12-waf` or `labs/11-rate-limit`) and measure the fraction of
   allowed traffic that the Bloom filter resolves without touching the
   real list.
4. Deliberately insert well past your sized `n` and re-measure the
   false-positive rate to see the degradation the sizing formula predicts.
