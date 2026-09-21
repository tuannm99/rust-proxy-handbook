# Token Bucket

`07-security/07-ratelimit.md` covers the policy question — per-client vs
global, distributed limiting, when to use leaky bucket instead. This file
covers making the counter itself correct and fast.

## What to learn

### Lazy refill and the state you actually need
The refill-on-read formulation in `07-security/07-ratelimit.md` (tokens +
elapsed × rate, capped at capacity) is the right one: no background timer
per key, and state is just a token count plus a timestamp. Two details
decide whether it is correct:

**Cap before consuming, not after.** An idle bucket must clamp to
`capacity` on refill, or a client silent for an hour accumulates an hour
of tokens and lands the entire backlog at once — which is the exact
traffic spike the limiter exists to prevent.

**Never let the clamp run backwards.** If `now` is earlier than
`last_refill`, `elapsed` is negative and tokens are *removed*. This is not
hypothetical: `Instant` is monotonic and safe, but the moment you store
`SystemTime` (to share buckets across processes, or to persist them) an
NTP step backwards silently drains every bucket. Use `Instant` locally,
and if the timestamp must cross a process boundary, clamp elapsed at zero.

### GCRA: the same limiter with one number
The Generic Cell Rate Algorithm stores a single timestamp — the
"theoretical arrival time" (TAT) of the next permitted request — instead
of a float plus a timestamp:

```rust
struct Gcra {
    tat: std::time::Instant, // when the next request would be exactly on-rate
}
// accept if now >= tat - burst_tolerance; on accept, tat = max(tat, now) + interval
```

`interval` is `1/rate` and `burst_tolerance` is `capacity × interval`. It
is mathematically equivalent to token bucket, but the state is one
`Instant` (8-16 bytes) with no floating-point drift, and it yields the
`Retry-After` value for free: `tat - burst_tolerance - now` is exactly how
long the client must wait. The `governor` crate is built on it.

Gotcha: floating-point token counts accumulate rounding error over
millions of refills, so a bucket that should sit at exactly `capacity`
drifts slightly below and rejects a request it should allow. GCRA's
integer/duration arithmetic sidesteps this entirely.

### Making it concurrent
The naive `Mutex<HashMap<IpAddr, TokenBucket>>` serializes every request in
the proxy on one lock. Two fixes, in order:

1. **Shard the map** — `dashmap`, as `07-security/07-ratelimit.md` suggests,
   which shards internally so different keys rarely contend.
2. **Make the bucket itself lock-free** — pack GCRA's TAT into an
   `AtomicU64` (nanoseconds since a fixed epoch) and update with a
   compare-and-swap loop. On CAS failure, re-read and retry; the loop
   terminates because a competing writer only ever moves TAT forward.

Gotcha: a read-then-write under a `RwLock` read guard is a race, not an
optimization. Two requests both read 1 token, both decide "allowed", both
write 0 — the limiter leaks. Token bucket has no read-only path; the check
and the decrement must be one atomic operation.

### Unbounded key growth
A per-source-IP map is attacker-controlled: spoofed or distributed sources
create an entry each, and the map is a memory-exhaustion vector
(`07-security/09-ddos.md`). Bound it, by one of:
- **Sweep idle entries.** A bucket at full capacity carries no
  information — deleting it is equivalent to keeping it. Sweep anything
  untouched for a few refill intervals.
- **Cap the map** and evict LRU (`13-algorithms/lru.md`) past the cap.
- **Fixed-size approximate counting.** Hash keys into a fixed array of
  buckets and accept that collisions merge two clients' limits — bounded
  memory by construction, at the cost of occasional false rejections. A
  count-min sketch (`13-algorithms/count-min-sketch.md`) is the principled
  version.

Gotcha: sweeping on a timer while requests concurrently touch the map
needs the sweep and the touch to agree on what "idle" means, or you delete
a bucket a request is mid-way through updating and hand that client a free
reset. Check-and-remove must be atomic against the update path.

## Practice
1. In `labs/11-rate-limit`, implement both the float token bucket and GCRA
   behind one trait; assert they accept/reject identically across a
   scripted request timeline including idle gaps.
2. Write the idle-accumulation test: leave a bucket untouched for 60x the
   refill interval, then fire a burst — confirm at most `capacity`
   requests pass.
3. Feed a timestamp that moves backwards and confirm your implementation
   neither drains the bucket nor grants free tokens.
4. Make GCRA lock-free with an `AtomicU64` + CAS loop; hammer it from 8
   tasks and assert the total accepted count never exceeds the
   mathematical maximum for the elapsed wall time.
5. Reproduce the read-then-write race on purpose with an `RwLock`, observe
   the over-admission, then fix it.
6. Fill the per-IP map with 1M synthetic source addresses and measure
   memory; add idle-sweeping and confirm it plateaus.
