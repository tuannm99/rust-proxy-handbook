# proxy

**This is the deliverable** — the single, complete, production-grade L7
proxy the whole handbook builds toward. Everything the `labs/` crates
exercise in isolation (TCP handling, HTTP parsing/routing/static files,
reverse proxying, load balancing, TLS, HTTP/2, caching, rate limiting,
WAF, hot reload, plugins, metrics, tracing) gets combined and hardened
here into something you'd actually run.

## Goal

Everything from the `labs/` crates, hardened toward production: TLS
termination, authentication, rate limiting, structured logs/metrics/
traces, config that reloads without dropping connections, and a shutdown
path that drains in-flight requests instead of cutting them off. "Done"
means: you can `curl` it over HTTPS, hit a rate limit and get a proper
429, change the upstream list in the config file and see it take effect
without a restart, and send `SIGTERM` during a load test and see zero
failed in-flight requests (only refused new ones).

## What to learn

- `instruction/01-network/07-tls.md` — handshake, SNI, ALPN (also decides h1 vs h2), session resumption
- `instruction/01-network/08-proxy-protocol.md` — preserving real client IP when this proxy sits behind another LB
- `instruction/07-security/01-auth.md`, `jwt.md`, `mtls.md` — pipeline position, identity propagation, the two mechanisms
- `instruction/07-security/07-ratelimit.md`, `instruction/07-security/06-waf.md`, `instruction/07-security/04-normalization.md` — token/leaky bucket, rule-based filtering, parser differentials
- `instruction/07-security/05-request-smuggling.md`, `instruction/07-security/08-ip-filtering.md` — parser ambiguity attacks, allow/deny lists
- `instruction/08-observability/01-logging.md`, `metrics.md`, `tracing.md`, `profiling.md` — structured logs, Prometheus metrics, distributed traces, flamegraphs
- `instruction/09-architecture/01-components.md`, `config.md`, `plugin.md`, `graceful-shutdown.md`, `canary-deploy.md` — how the pieces wire together, hot reload, drain-on-shutdown, staged rollout

## Practice

Build `proxy` incrementally, on top of what `labs/05-reverse-proxy` and
`labs/06-load-balancer` taught you:

1. Add TLS termination with `tokio-rustls`; confirm ALPN correctly picks h1 vs h2 per client.
2. Load config (upstreams, listen addr, rate limits) from a TOML file at startup; add SIGHUP or file-watch based hot reload that swaps config without dropping active connections.
3. Add JWT validation on a protected route set and per-client token-bucket rate limiting; confirm a 401 and a 429 are returned correctly and rate-limit state resets on schedule.
4. Add `tracing` spans around the request lifecycle and export Prometheus metrics (request count, latency histogram, upstream error count).
5. Implement graceful shutdown: on SIGTERM, stop accepting new connections, let in-flight requests finish (with a deadline), then exit — verify with a load test running across the signal.
6. Add IP allow/deny lists and at least one WAF-style rule (e.g. block requests with suspicious header patterns); write a request-smuggling test case against your parser/hyper config and confirm it's rejected, not silently misrouted.

Run with:

```
cargo run -p proxy
```
