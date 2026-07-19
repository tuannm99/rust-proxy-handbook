# Project 3
Reverse Proxy.

## Goal
A hyper-based proxy that forwards incoming requests to one of several
upstream servers and returns their response. "Done" means: it load-balances
across at least 2 upstreams, stops routing to an upstream that fails health
checks, retries a request against a different upstream on connection failure
(without retrying non-idempotent requests blindly), and correctly proxies a
streamed request/response body (not just small buffered ones).

## What to learn
- `06-proxy/upstream.md` — connection pooling to upstreams, reusing vs opening new connections per request
- `06-proxy/load-balancer.md` — round robin, least connections, consistent hash — and when each is the wrong choice
- `06-proxy/healthcheck.md` — active vs passive health checks, flapping/hysteresis
- `06-proxy/retry.md` — retry budgets, idempotency, circuit breaker states (closed/open/half-open)
- `06-proxy/service-discovery.md` — static list vs dynamic upstream membership
- `01-network/http.md` — which headers a proxy must rewrite (`Host`, `X-Forwarded-For`, `Connection`)

## Practice
Build `milestones/03-reverse-proxy` incrementally:
1. Hardcode 2 upstream addresses, forward every request to upstream #1 only, stream the response back unmodified.
2. Add round-robin selection across both upstreams; verify with a counter in each upstream's logs.
3. Add an active health check (periodic GET to a health path) and remove/re-add upstreams from rotation based on it.
4. Add retry-on-connection-failure to the other upstream for idempotent methods (GET/HEAD), and a circuit breaker that stops trying an upstream that's failed N times in a row.
5. Confirm request/response bodies stream through (test with a large file upload/download) instead of being fully buffered in memory.
6. Kill one upstream mid-load-test and confirm the proxy keeps serving from the healthy one with no client-visible errors beyond a brief blip.
