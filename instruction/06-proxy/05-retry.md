# Retry

Recovering an individual failed request. The layer above — deciding an
upstream should stop receiving requests at all — is
`06-proxy/06-circuit-breaker.md`.

## What to learn
### Idempotency: the rule that comes before any retry logic
Never retry a request whose method/semantics aren't safe to repeat unless
you know the upstream is idempotent for it. GET/HEAD/PUT/DELETE are
generally safe to retry; a bare POST usually is not (it might create a
resource twice). A production proxy either only auto-retries
idempotent methods, or requires an explicit idempotency key from the
client for POST retries.

Gotcha: "idempotent by RFC" and "idempotent in this upstream" are
different claims. `DELETE /orders/42` is idempotent by spec, but if the
upstream emits a webhook or decrements inventory on each call, retrying it
has a visible side effect anyway. Method-based retry policy is a
reasonable default, not a proof — make it overridable per route
(`05-http-stack/03-router.md`) so a team that knows their endpoint is unsafe
can turn it off.

### The proxy-specific constraint: you may not be able to retry at all
A retry means re-sending the request, which means still having it. A proxy
streaming a request body upstream has already consumed it from the client
socket — the bytes are gone, and there is nothing left to replay. So
retrying a request with a body requires buffering that body first, and
buffering is bounded: you cannot hold a 5 GB upload in memory to preserve
the *option* of retrying.

This produces a hard rule with a threshold in it: buffer request bodies up
to N bytes and keep them retryable; past N, stream and mark the request
non-retryable from that point on. Envoy exposes exactly this as
`per_try_buffer_limit`/retry buffer limits.

Gotcha: the same applies to the *response*. Once you've sent the first
response byte to the client, you cannot retry — the client has already
seen a status line and headers. Any retry decision must happen before the
response head is forwarded, which means an upstream that returns 200 and
*then* fails mid-body is unretryable no matter how idempotent the method
was.

### Retry budgets
Retrying blindly on every failure can turn a small upstream blip into a
retry storm that takes the upstream down completely (each failed request
now costs 2-3x). A retry budget caps total retries as a percentage of
total requests over a rolling window (e.g. "retries may not exceed 10% of
requests in the last 10s") — if the budget is exhausted, stop retrying and
fail fast instead.

Why a percentage budget rather than a per-request attempt cap: a per-
request cap of 3 is fine when 1% of requests fail, and catastrophic when
100% do, because the cap is per-request and the *load* is what matters to
the struggling upstream. A budget is defined against total traffic, so it
degrades to "no retries" exactly when retries would hurt most. This is
what linkerd and Envoy both implement, and it's the single most important
retry control to get right.

Gotcha: budget accounting must be per-upstream-pool, not global. One
misbehaving backend exhausting a global budget disables retries for every
other route in the proxy, converting a localized failure into a
fleet-wide degradation.

### Retry amplification across layers
Retries multiply. A client retrying 3x in front of a proxy retrying 3x in
front of a service mesh sidecar retrying 3x means one user action can
become 27 requests at the bottom of the stack — and every layer thinks it
is being modest. This is a recurring cause of full outages during partial
degradation, because the traffic multiplier peaks exactly when the system
is least able to absorb it.

Two disciplines keep it bounded: **retry at one layer only** (usually the
one closest to the failure, with the most context), and propagate
"already retried" state so downstream layers don't add their own —
concretely, a header the proxy sets and the next hop honors. If you
control only your own layer, at minimum know what the layers above and
below you are configured to do, and write it down next to your retry
config.

### Exponential backoff with jitter
Fixed-delay retries from many clients synchronize into retry storms.
Exponential backoff with random jitter spreads retries out in time.

```rust
fn backoff(attempt: u32, base: std::time::Duration) -> std::time::Duration {
    let exp = base * 2u32.pow(attempt.min(6));
    let jitter_ms = rand_range(0..exp.as_millis() as u64 / 2);
    exp + std::time::Duration::from_millis(jitter_ms)
}
```
Gotcha: cap the exponent (as above) — `2u32.pow(attempt)` overflows fast if
`attempt` is unbounded.

The jitter shape matters more than people expect. The version above
("equal jitter") keeps half the delay deterministic; **full jitter** —
`random(0, min(cap, base * 2^attempt))` — discards the deterministic half
entirely and measurably beats it at reducing contention, per AWS's
published analysis. The intuition: any deterministic component is a
schedule that all clients still share, so only the random part actually
spreads load.

Gotcha: backoff between retries of a *single* request adds directly to
that request's latency, and the client has its own timeout. Three retries
with exponential backoff can easily exceed a 1s client timeout, at which
point every retry after the first is pure load with no chance of being
useful. Bound total retry time by the request's remaining deadline, not
just by attempt count.

### Which upstream should the retry go to
Retrying the request against the upstream that just failed is usually
wasted — whatever broke it is unlikely to be fixed microseconds later.
Retry against a *different* upstream from the pool, and exclude the failed
one from the candidate set for that request.

Gotcha: this conflicts with consistent hashing (`02-load-balancer.md`) when
affinity is load-bearing — an upstream-side cache or a stateful session
means the "different" upstream is a cold miss or an outright error. When
affinity matters, prefer failing over to the ring's *next* node
deterministically (so all clients of that key agree on the fallback) over
picking a random other upstream.

### When retrying stops being the answer
A retry handles an individual blip. When blips become the norm, retrying
into a failing upstream is just load it can't absorb — the layer above is
a circuit breaker, which stops calling that upstream entirely for a
cool-down period and fails fast instead.

The one thing to get right from the retry side: **retries must respect the
circuit**, skipping hosts whose circuit is open rather than treating
"circuit open" as another failure to retry past. See
`06-proxy/06-circuit-breaker.md`.

### Hedged requests: the tail-latency variant
Retries fire on failure. **Hedging** fires on *slowness*: if a request
hasn't responded within, say, the pool's p95 latency, send a second copy
to a different upstream and take whichever answers first, cancelling the
loser. Since the bad case in a large fleet is usually one slow host rather
than a broken one, hedging cuts p99 latency dramatically for a small
increase in total load — this is the core technique from Google's "The
Tail at Scale".

Gotcha: hedging inherits every constraint above *and* adds one — the
duplicate is in flight simultaneously, so a non-idempotent hedge is
strictly worse than a non-idempotent retry (both copies may succeed).
Hedge only idempotent requests, hedge at a threshold derived from measured
latency (`08-observability/02-metrics.md`), and count hedges against the
retry budget — otherwise a latency regression across the whole fleet turns
into every request being sent twice, precisely when capacity is short.

## Practice
Build these in order.

1. In `labs/05-reverse-proxy`, add a retry wrapper that only retries
   GET/HEAD, uses the backoff function above capped at 3 attempts, and
   sends each retry to a *different* upstream. **Done when** killing one
   upstream mid-load-test produces zero client-visible errors, and logs
   show retries landing on the other upstreams.
2. Switch the jitter to full jitter and measure. **Done when** you can
   show, from timestamps of 1000 synchronized retrying clients, that
   retry arrivals are spread across the backoff window rather than
   clustered — compare against the equal-jitter version on the same test.
3. Bound total retry time by a per-request deadline. **Done when** a
   request with 200ms remaining budget stops retrying instead of running
   three exponential backoffs that the client has already given up on.
4. Add the request-body buffer limit. **Done when** a small POST is
   retryable and a 100 MB upload streams through without being buffered —
   and the large one is correctly marked non-retryable rather than failing
   or OOMing.
5. Add a rolling percentage retry budget, per upstream pool. **Done when**
   forcing 100% upstream failure causes retries to stop within one window
   (rather than tripling load), and traffic to a second, healthy pool
   still retries normally.
6. Work through `06-proxy/06-circuit-breaker.md`'s exercises, then make
   retries circuit-aware. **Done when** a retry skips a host with an open
   circuit rather than spending an attempt on it.
7. (Stretch) Add hedging on GET at the measured p95. **Done when** p99
   latency under a load test with one deliberately slow upstream drops
   measurably, and total request count to upstreams rises by only a few
   percent — if it rises by 50%, your threshold is wrong.
