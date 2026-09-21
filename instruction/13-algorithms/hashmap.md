# Hash Maps: Chaining vs Open Addressing

Used everywhere in this handbook — route tables, upstream pools,
rate-limit buckets, cache keys. This file covers what's actually inside
the `HashMap` you reach for by default, and the one production gotcha
that applies specifically because a proxy's map keys often come from an
attacker.

## What to learn

### Two families
**Chaining**: each bucket holds a list (or small `Vec`) of every entry
that hashed there; a collision just grows the list. Simple, degrades
gracefully, but each lookup after the first bucket index is a pointer
chase — cache-unfriendly.

**Open addressing**: every entry lives directly in the backing array; on
a collision, probe to another slot (linear, quadratic, or via a second
hash) until an empty one is found. No pointer chasing, much better cache
behavior — the entire table is one contiguous allocation — but needs
tombstones or backward-shifting to handle deletion without breaking probe
sequences for entries inserted after a collision.

### What Rust's std `HashMap` actually is
Since Rust 1.36, `std::collections::HashMap` is backed by `hashbrown`, a
Rust port of Google's SwissTable design: open addressing with SIMD-
accelerated probing (a control byte per slot lets the probe check up to
16 slots per SIMD instruction) and robin-hood-style displacement to keep
probe sequences short. This is why "just use `HashMap`" is genuinely good
advice in Rust specifically — it isn't the naive chaining map some other
languages default to.

### Hash flooding: an attacker-chosen key space
A `HashMap<IpAddr, TokenBucket>` (`13-algorithms/token-bucket.md`) or any
map keyed by client-controlled data (headers, query params) has its key
distribution chosen by whoever sends requests. Against a **non-keyed**
hash function (FxHash, a raw FNV, anything without a per-process random
seed), an attacker who knows the hash algorithm can choose inputs that
all collide, degrading every operation toward the collision-chain
length — O(n) per lookup instead of O(1), turning a hash map into a
denial-of-service vector on its own (`07-security/09-ddos.md`).

Rust's default hasher (SipHash, keyed with a random seed generated per
process at startup) is specifically DoS-resistant against this: without
knowing the process's random key, an attacker cannot predict which inputs
collide. **This is exactly why swapping to a faster hasher (`ahash`,
`FxHash` — both common performance advice) is only safe for keys the
proxy itself controls** (an internal route table built from static
config) and unsafe for keys derived from client input (source IP, header
values) unless that faster hasher is also keyed and seeded
unpredictably — check before swapping, don't assume "faster hash map" is
a free win once client-controlled keys are involved.

## Practice
1. Implement a toy chaining hash map and a toy open-addressing hash map
   (linear probing) over the same key type; compare lookup time as load
   factor increases from empty to nearly full.
2. Reproduce hash flooding: pick a non-keyed hash function, construct a
   set of inputs that all map to the same bucket, and measure a chaining
   map's lookup degrade toward O(n). Confirm Rust's default `HashMap`
   with the same crafted inputs does not degrade the same way.
3. In `labs/11-rate-limit`, benchmark the per-IP bucket map with the
   default `HashMap` hasher versus `ahash`/`FxHash`; then repeat step 2's
   flooding attempt against the faster-hasher version specifically to see
   whether it's still resistant.
4. Audit `proxy/`'s maps (or design them, if not yet written) and
   classify each by whether its keys are trusted (internal config) or
   untrusted (client-derived) — decide the hasher for each based on that,
   not on benchmark speed alone.
