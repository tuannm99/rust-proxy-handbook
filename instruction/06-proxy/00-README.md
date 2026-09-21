# Reverse Proxy

Phase 6 — the core of what `proxy/` actually is. Everything here is about
the *upstream* side: which backend gets the request, what happens when it
fails, and how the set of backends changes underneath you.

## Files

- `01-upstream.md` — modeling an upstream, connection reuse and pool sizing, the three timeouts
- `02-load-balancer.md` — round robin, least connections, P2C, peak EWMA, consistent hashing
- `03-healthcheck.md` — active probing, what each probe depth proves, deep-check correlated failure
- `04-outlier-detection.md` — passive detection from real traffic, flap damping, slow start
- `05-retry.md` — idempotency, retry budgets, backoff and jitter, hedged requests
- `06-circuit-breaker.md` — tripping on a failure rate, half-open gating, scope per upstream
- `07-service-discovery.md` — DNS and watch-based membership, never accepting an empty result, draining

## Reading order

`01-upstream.md` first — it defines the type everything else operates on.
Then `02-load-balancer.md` (picking one), then the failure-handling pair in
either order: `03-healthcheck.md` + `04-outlier-detection.md` (deciding a host
is bad) and `05-retry.md` + `06-circuit-breaker.md` (reacting to a failed
request). `07-service-discovery.md` last, since it changes the pool the rest
assumed was fixed.

Together these back `labs/05-reverse-proxy` and `labs/06-load-balancer`.
The algorithms behind the balancer — smooth WRR, Maglev, rendezvous
hashing — have their own deep dives in `13-algorithms/`.
