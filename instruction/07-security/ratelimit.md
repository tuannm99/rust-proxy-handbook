# Rate Limiting
Token Bucket, Leaky Bucket.

## What to learn
### Token Bucket
A bucket holds up to `capacity` tokens, refilling at `rate` tokens/sec;
each request consumes one token, and is rejected (or delayed) if none are
available. Naturally allows bursts up to `capacity` while enforcing an
average rate over time — this is why it's the more common choice for API
rate limiting (bursty client traffic is normal and shouldn't be punished
as long as the average holds).

```rust
struct TokenBucket {
    capacity: f64,
    tokens: f64,
    rate_per_sec: f64,
    last_refill: std::time::Instant,
}

impl TokenBucket {
    fn try_consume(&mut self, now: std::time::Instant) -> bool {
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        self.tokens = (self.tokens + elapsed * self.rate_per_sec).min(self.capacity);
        self.last_refill = now;
        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            true
        } else {
            false
        }
    }
}
```
Gotcha: refill lazily (as above, on each check) rather than with a
background timer per client — a timer per rate-limited key doesn't scale
to millions of clients.

### Leaky Bucket
Models a queue that drains (leaks) at a constant rate; requests enqueue
and are processed at that fixed rate, or are dropped if the queue is full.
Unlike token bucket, it smooths output to a strictly constant rate rather
than allowing bursts — appropriate when the *downstream* system truly
can't handle bursts (e.g. protecting a fixed-capacity legacy backend),
less appropriate for "fair API usage" limiting where bursts are fine.

### Per-client vs global limits
Per-client (per API key / per IP) limiting needs a keyed map of buckets —
watch memory growth (evict idle entries) and use a sharded map
(`dashmap`) to avoid one global lock becoming the bottleneck under high
concurrency. Global limits (protect the whole fleet regardless of client)
are a single shared bucket/counter and are comparatively easy.

### Distributed rate limiting
A single proxy instance's in-memory bucket only limits traffic through
*that instance*. With N proxy replicas behind another LB, a per-client
limit of "100 req/s" becomes "100*N req/s" unless replicas coordinate —
either via a shared store (Redis with atomic `INCR`+`EXPIRE`, or a
Lua/token-bucket script) or by accepting the approximation and dividing
the limit by N. Coordinating on every request adds a network round trip
per request — usually mitigated by batching/local caching with periodic
sync, trading strict accuracy for latency.

## Practice
1. In `proxy`, implement the `TokenBucket` above and
   apply it per source IP using a `dashmap`.
2. Add eviction for idle per-client buckets (e.g. sweep entries untouched
   for 5 minutes) so memory doesn't grow unbounded.
3. Load-test a single client past its limit and confirm a 429 with a
   `Retry-After` header once tokens run out.
4. (Stretch) Run two instances of the proxy and demonstrate the
   per-instance limit multiplying traffic through; then fix it with a
   shared Redis-backed counter.
