# Rate Limiting
Token Bucket, Leaky Bucket.

The algorithms themselves (lazy refill, GCRA, lock-free counters, sliding
windows) are covered in `13-algorithms/token-bucket.md`,
`13-algorithms/sliding-window.md`, and `13-algorithms/leaky-bucket.md`.
This file is the policy layer: what to count, what to do when the limit is
hit, and what happens when the limiter itself fails.

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

### Choosing the key: the decision that actually matters
The algorithm is the easy part. *What you count per* determines whether
the limiter protects you or just annoys your users.

**Source IP** is the default and has two specific failure modes. Shared
addresses (CGNAT, corporate NAT, a university) put thousands of users
behind one key, so a limit sized for one person throttles all of them —
the same shared-address problem as `ip-filtering.md`.

And the IPv6 case is worse in the opposite direction: an attacker
typically gets a **/64 allocation, which is 2^64 addresses**. Keying on the
full 128-bit address means every single request can carry a fresh key, and
your per-IP limiter never triggers even once — while your bucket map grows
without bound. Key IPv6 on the **/64 prefix** (sometimes /56, depending on
what your customers are actually assigned), not the full address. A
limiter that's been in production for years can have this hole and nobody
notices until an attacker uses it.

**Authenticated identity** (API key, user ID from validated claims —
`auth.md`) is strictly better where available: it's stable, it's not
shared, and it's what your business rules are actually expressed in. The
catch is that auth runs after the limiter in some designs, so you need
both — a cheap IP-based limit in front to protect the auth path itself,
and a real per-identity limit behind it.

**Route** should almost always be part of the key. One global limit per
client means a burst of cheap requests consumes the budget that a single
expensive endpoint needed (`ddos.md`). Limit `/search` separately from
`/health`.

Gotcha: whatever you key on, that key comes from attacker-controlled data
and indexes a map. Bound it, per `13-algorithms/token-bucket.md`'s section
on key growth — this is the same memory-exhaustion vector, and the /64
mistake above turns it from theoretical into trivially exploitable.

### Not every request costs the same
A single "1 request = 1 token" model prices a 2 KB health check the same
as a report that pins a CPU for 400ms. Cost-based limiting charges tokens
proportional to actual cost: a static value per route (cheap and usually
enough), or tokens deducted *after* the fact based on measured upstream
time or response bytes — which lets a client's next request be throttled
by what their last one actually cost.

Gotcha: post-hoc charging means a client can always exceed the limit by
exactly one expensive request, since the cost isn't known until it's
done. That's acceptable for cost *smoothing* and useless as a hard
ceiling — pair it with a concurrency cap per route if a single request can
hurt you.

### Per-client vs global limits
Per-client (per API key / per IP) limiting needs a keyed map of buckets —
watch memory growth (evict idle entries) and use a sharded map
(`dashmap`) to avoid one global lock becoming the bottleneck under high
concurrency. Global limits (protect the whole fleet regardless of client)
are a single shared bucket/counter and are comparatively easy.

Run both. Per-client limits enforce fairness; a global limit is what
actually protects the upstream, because "10,000 clients each within their
limit" can still exceed capacity. The global one should be sized from
measured capacity (`12-testing/load-testing.md`), not chosen as a round
number.

### What to return when you reject
`429 Too Many Requests`, with `Retry-After` giving the seconds until
capacity exists. GCRA (`13-algorithms/token-bucket.md`) yields that number
exactly; a token bucket computes it as `(1 - tokens) / rate`.

Beyond that, the `RateLimit-Limit` / `RateLimit-Remaining` /
`RateLimit-Reset` header family (the IETF draft that GitHub, Stripe and
others already ship in some form) lets well-behaved clients pace
themselves *before* being rejected — which is far more effective at
reducing load than rejecting them after the fact, since a client that
knows it has 3 requests left will slow down and one that doesn't will
hammer you until it gets a 429.

Gotcha: a 429 must be cheap. If rejecting costs a database lookup, a log
write with full request context, and a rendered error page, an attacker
gets a better cost ratio from being rate-limited than from being served
(`ddos.md`). Reject early in the pipeline, log at a sampled rate rather
than every occurrence.

Gotcha: make sure your own clients don't retry 429s immediately. A retry
storm of rejected requests is exactly the load the limit exists to
prevent — see `06-proxy/retry.md`; `Retry-After` is there to be honored.

### Distributed rate limiting
A single proxy instance's in-memory bucket only limits traffic through
*that instance*. With N proxy replicas behind another LB, a per-client
limit of "100 req/s" becomes "100*N req/s" unless replicas coordinate —
either via a shared store (Redis with atomic `INCR`+`EXPIRE`, or a
Lua/token-bucket script) or by accepting the approximation and dividing
the limit by N. Coordinating on every request adds a network round trip
per request — usually mitigated by batching/local caching with periodic
sync, trading strict accuracy for latency.

Three approaches, with the trade stated honestly:
- **Divide by N.** Zero coordination cost, and wrong whenever traffic
  isn't evenly distributed — which is exactly the case under consistent
  hashing or when one client's connections land on one replica. A client
  entitled to 100 req/s gets 20 because they happened to hit one replica.
- **Shared store on every request.** Accurate, and adds a network RTT plus
  a hard dependency to the hot path of every single request.
- **Local buckets with periodic async sync.** Each replica enforces
  locally and reconciles its counters with the shared store every few
  hundred milliseconds. Briefly permissive during the sync window, no
  latency on the request path, and this is what most production systems
  actually do.

Gotcha: the shared store is now a dependency that can fail, and you must
decide in advance which way. **Fail open** (allow when Redis is down) keeps
the site up and removes your protection at the worst possible moment.
**Fail closed** (reject when Redis is down) turns a cache outage into a
site outage. The usual answer is to fail open *to the local limiter* —
keep enforcing each replica's own local budget, which is approximately
"divide by N" behavior, so a coordination outage degrades accuracy instead
of removing the limit or the service. Decide this deliberately and test
it; discovering it during an incident is how "the rate limiter was down"
becomes "the site was down."

## Practice
Build these in order.

1. In `labs/11-rate-limit`, implement the `TokenBucket` above keyed by
   source IP with a `dashmap`. **Done when** a client exceeding the rate
   gets rejected and a client within it never is.
2. Fix the key. **Done when** IPv6 clients are keyed on the /64 prefix —
   prove it with a test that sends from 10,000 distinct addresses within
   one /64 and confirms they share a bucket, after first watching the
   /128 version let all 10,000 through.
3. Add route to the key and per-route limits from config. **Done when** a
   burst on `/health` doesn't consume the budget for `/search`.
4. Return proper 429s with `Retry-After` and the `RateLimit-*` headers.
   **Done when** `Retry-After` is accurate to within a second of when
   capacity actually returns — verify by sleeping exactly that long and
   confirming the next request succeeds.
5. Add idle-bucket eviction and a bounded map. **Done when** filling the
   map with 1M synthetic keys plateaus in memory rather than growing,
   and active clients' buckets are never evicted out from under them.
6. Add a global limit alongside the per-client ones, sized from a real
   load test. **Done when** 10,000 individually-compliant clients are
   collectively capped at your measured upstream capacity.
7. Add cost-based charging for one expensive route. **Done when** a client
   calling the expensive endpoint exhausts their budget proportionally
   faster than one calling a cheap endpoint.
8. Run two instances and demonstrate the multiplication. **Done when** you
   can show a client getting 2x their limit; then implement local buckets
   with periodic sync to a shared store and show it converging near 1x.
9. Kill the shared store mid-test. **Done when** the proxy keeps serving
   traffic with local enforcement still active — neither unlimited nor
   rejecting everything — and a metric records that it is running
   degraded.
