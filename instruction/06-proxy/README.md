# Reverse Proxy

Phase 6 — the core of what `proxy/` actually is. Everything here is about
the *upstream* side: which backend gets the request, what happens when it
fails, and how the set of backends changes underneath you.

## Files

- `upstream.md` — modeling an upstream, connection reuse and pool sizing, the three timeouts
- `load-balancer.md` — round robin, least connections, P2C, peak EWMA, consistent hashing
- `healthcheck.md` — active probing, what each probe depth proves, deep-check correlated failure
- `outlier-detection.md` — passive detection from real traffic, flap damping, slow start
- `retry.md` — idempotency, retry budgets, backoff and jitter, hedged requests
- `circuit-breaker.md` — tripping on a failure rate, half-open gating, scope per upstream
- `service-discovery.md` — DNS and watch-based membership, never accepting an empty result, draining

## Reading order

`upstream.md` first — it defines the type everything else operates on.
Then `load-balancer.md` (picking one), then the failure-handling pair in
either order: `healthcheck.md` + `outlier-detection.md` (deciding a host
is bad) and `retry.md` + `circuit-breaker.md` (reacting to a failed
request). `service-discovery.md` last, since it changes the pool the rest
assumed was fixed.

Together these back `labs/05-reverse-proxy` and `labs/06-load-balancer`.
The algorithms behind the balancer — smooth WRR, Maglev, rendezvous
hashing — have their own deep dives in `13-algorithms/`.
