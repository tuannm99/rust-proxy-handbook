# 05-reverse-proxy

## Goal

A hyper-based proxy that forwards incoming requests to one of several
upstream servers and returns their response. Deeper load-balancing
algorithms beyond basic round robin live in
[`labs/06-load-balancer`](../06-load-balancer).

## Done when

- [ ] Requests round-robin across at least 2 upstreams (verify by having each upstream return its own name).
- [ ] Upstream connections are pooled: under steady load, `ss -tanp` shows a stable set of upstream connections instead of one new connection per request.
- [ ] An upstream that fails active health checks stops receiving traffic within your configured interval, and rejoins after it recovers.
- [ ] A request that fails to *connect* is retried on a different upstream; a `POST` that fails after being sent is **not** retried blindly ([`instruction/06-proxy/05-retry.md`](../../instruction/06-proxy/05-retry.md)).
- [ ] Distinct statuses for distinct failures: `502` upstream refused/invalid, `503` no healthy upstream, `504` upstream timed out.
- [ ] Headers are rewritten correctly: hop-by-hop headers stripped both directions, `X-Forwarded-For` appended, `Host` handled per your documented policy.
- [ ] A 1 GB upload and a 1 GB download stream through with flat proxy memory, and a slow client slows the upstream read instead of filling proxy memory.
- [ ] Reviewed per [`instruction/00-introduction/03-study-loop.md`](../../instruction/00-introduction/03-study-loop.md) step 5.

## Handbook references
- [`instruction/06-proxy/01-upstream.md`](../../instruction/06-proxy/01-upstream.md) — connection pooling to upstreams
- [`instruction/06-proxy/03-healthcheck.md`](../../instruction/06-proxy/03-healthcheck.md) — active probing, probe depth, deep-check correlated failure
- [`instruction/06-proxy/04-outlier-detection.md`](../../instruction/06-proxy/04-outlier-detection.md) — passive detection, flap damping, slow start
- [`instruction/06-proxy/05-retry.md`](../../instruction/06-proxy/05-retry.md) — retry budgets, idempotency, backoff, hedging
- [`instruction/06-proxy/06-circuit-breaker.md`](../../instruction/06-proxy/06-circuit-breaker.md) — tripping on a failure rate, half-open gating
- [`instruction/06-proxy/07-service-discovery.md`](../../instruction/06-proxy/07-service-discovery.md) — static list vs dynamic upstream membership
- [`instruction/01-network/10-http.md`](../../instruction/01-network/10-http.md) — which headers a proxy must rewrite (`Host`, `X-Forwarded-For`, `Connection`)
- [`instruction/04-runtime/04-structured-concurrency.md`](../../instruction/04-runtime/04-structured-concurrency.md) — cancelling health-check tasks when the pool goes away

## Run

```
cargo run -p reverse-proxy
```
