# 05-reverse-proxy

## Goal

A hyper-based proxy that forwards incoming requests to one of several
upstream servers and returns their response. Done means: it round-robins
across at least 2 upstreams, stops routing to an upstream that fails
health checks, retries a request against a different upstream on
connection failure (without retrying non-idempotent requests blindly),
and correctly proxies a streamed request/response body. Deeper
load-balancing algorithms beyond basic round robin live in
`labs/06-load-balancer`.

## Handbook references
- `instruction/06-proxy/01-upstream.md` — connection pooling to upstreams
- `instruction/06-proxy/03-healthcheck.md` — active probing, probe depth, deep-check correlated failure
- `instruction/06-proxy/04-outlier-detection.md` — passive detection, flap damping, slow start
- `instruction/06-proxy/05-retry.md` — retry budgets, idempotency, backoff, hedging
- `instruction/06-proxy/06-circuit-breaker.md` — tripping on a failure rate, half-open gating
- `instruction/06-proxy/07-service-discovery.md` — static list vs dynamic upstream membership
- `instruction/01-network/04-http.md` — which headers a proxy must rewrite (`Host`, `X-Forwarded-For`, `Connection`)

## Run

```
cargo run -p reverse-proxy
```
