# Sliding Window Rate Limiting

`07-security/07-ratelimit.md` and `13-algorithms/token-bucket.md` cover token
bucket and leaky bucket. This file covers the other family: limiting by
counting requests in a moving time window instead of modeling a bucket.

## What to learn

### Fixed window counter: simple, and wrong at the boundary
The naive version: a counter per `(key, window)`, incremented per request,
reset when the window rolls over (e.g. every wall-clock minute).

```rust
struct FixedWindow {
    window_start: std::time::Instant,
    window_len: std::time::Duration,
    count: u32,
    limit: u32,
}
```

Gotcha: this allows exactly `2 * limit` requests in any real one-window
span, not `limit`. A client that sends `limit` requests in the last
millisecond of window N and another `limit` in the first millisecond of
window N+1 never technically exceeds either window's counter, but the
origin sees `2 * limit` requests in ~2ms. This boundary-burst is the
textbook failure mode fixed-window limiters ship with by accident.

### Sliding window log: exact, but O(n) memory
Store the timestamp of every request in the trailing window (e.g. a
`VecDeque<Instant>` per key); on each request, drop timestamps older than
`now - window_len`, then check `deque.len() < limit`. This is exactly
correct — no boundary burst — because the window is truly continuous, not
bucketed.

Gotcha: memory is proportional to `limit`, per key. At a limit of 10,000
req/window across a million keys, that's potentially 10 billion stored
timestamps. This does not scale to a proxy's per-IP or per-token limiting
without a memory cap alongside it.

### Sliding window counter: the approximation everyone actually ships
Keep two fixed-window counters — the current window and the previous one —
and weight the previous window's count by how much of it still overlaps
the trailing window:

```rust
fn sliding_count(prev_count: u32, curr_count: u32, elapsed_into_curr: f64, window_len: f64) -> f64 {
    let prev_weight = (window_len - elapsed_into_curr) / window_len;
    curr_count as f64 + prev_count as f64 * prev_weight
}
// reject if sliding_count(..) >= limit
```

This is what Cloudflare's and Kong's rate limiters implement: O(1) memory
per key (two integers, not a log), and it eliminates the boundary-burst
problem to within a bounded approximation error — the assumption that
requests are spread evenly through the previous window, which is close
enough in practice and provably bounds the worst case at `2x` only in the
same pathological all-at-the-edges pattern, weighted down rather than
counted in full.

Gotcha: the two-counter version needs both counters to be read and rolled
over atomically relative to each other, or a request landing exactly on
the window boundary can read a half-rolled-over state (previous count
already reset to 0, current count not yet the "current" one) and undercount.
Roll the window over as one operation — swap `curr` into `prev` and zero
`curr` — under the same lock/CAS that increments the count, not as two
separate steps.

### Choosing against token bucket
Token bucket and sliding window counter solve the same problem with
different tradeoffs: token bucket naturally allows a burst up to
`capacity` and then throttles to the steady rate, forever, with O(1) state
that never needs a "previous window." Sliding window counter enforces "no
more than N in any trailing window," which is a tighter, more literal
reading of "N req/sec" — useful when a downstream SLA is phrased that way
— but needs the previous-window bookkeeping and only approximates the
true count. Pick based on which guarantee the limit is actually promising
to a client or a downstream.

## Practice
1. In `labs/11-rate-limit`, implement the fixed window counter first and
   write the boundary-burst test explicitly: send `limit` requests at
   `window_end - 1ms` and another `limit` at `window_end + 1ms`, and
   observe both succeed despite `2x` the intended rate in ~2ms.
2. Implement the sliding window log and rerun the same test; confirm it
   rejects correctly, then measure its memory at a large `limit` and many
   keys to see why it doesn't scale unmodified.
3. Implement the sliding window counter and rerun the boundary test a
   third time; confirm the observed overshoot is bounded and much smaller
   than the fixed-window version's `2x`.
4. Reproduce the atomic-rollover bug on purpose (roll over `curr`→`prev`
   and reset as two separate, non-atomic steps under concurrent access),
   observe undercounting, then fix it.
5. (Stretch) Compare sliding window counter against your `token-bucket.md`
   GCRA implementation on an identical bursty traffic trace and describe,
   in your own words, which one a client would experience as more
   "unfair" and why.
