# proxy

**This is the deliverable** — the single, complete, production-grade L7
proxy the whole handbook builds toward. Everything the [`labs/`](../labs) crates
exercise in isolation (TCP handling, HTTP parsing/routing/static files,
reverse proxying, load balancing, TLS, HTTP/2, caching, rate limiting,
WAF, hot reload, plugins, metrics, tracing) gets combined and hardened
here into something you'd actually run.

## Goal

Everything from the [`labs/`](../labs) crates, hardened toward production: TLS
termination, authentication, rate limiting, structured logs/metrics/
traces, config that reloads without dropping connections, and a shutdown
path that drains in-flight requests instead of cutting them off. "Done"
means: you can `curl` it over HTTPS, hit a rate limit and get a proper
429, change the upstream list in the config file and see it take effect
without a restart, and send `SIGTERM` during a load test and see zero
failed in-flight requests (only refused new ones).

## What to learn

- [`instruction/01-network/13-tls.md`](../instruction/01-network/13-tls.md) — handshake, SNI, ALPN (also decides h1 vs h2), session resumption
- [`instruction/01-network/14-proxy-protocol.md`](../instruction/01-network/14-proxy-protocol.md) — preserving real client IP when this proxy sits behind another LB
- [`instruction/07-security/01-auth.md`](../instruction/07-security/01-auth.md), [`instruction/07-security/02-jwt.md`](../instruction/07-security/02-jwt.md), [`instruction/07-security/03-mtls.md`](../instruction/07-security/03-mtls.md) — pipeline position, identity propagation, the two mechanisms
- [`instruction/07-security/07-ratelimit.md`](../instruction/07-security/07-ratelimit.md), [`instruction/07-security/06-waf.md`](../instruction/07-security/06-waf.md), [`instruction/07-security/04-normalization.md`](../instruction/07-security/04-normalization.md) — token/leaky bucket, rule-based filtering, parser differentials
- [`instruction/07-security/05-request-smuggling.md`](../instruction/07-security/05-request-smuggling.md), [`instruction/07-security/08-ip-filtering.md`](../instruction/07-security/08-ip-filtering.md) — parser ambiguity attacks, allow/deny lists
- [`instruction/08-observability/01-logging.md`](../instruction/08-observability/01-logging.md), [`instruction/08-observability/02-metrics.md`](../instruction/08-observability/02-metrics.md), [`instruction/08-observability/03-tracing.md`](../instruction/08-observability/03-tracing.md), [`instruction/08-observability/04-profiling.md`](../instruction/08-observability/04-profiling.md) — structured logs, Prometheus metrics, distributed traces, flamegraphs
- [`instruction/09-architecture/01-components.md`](../instruction/09-architecture/01-components.md), [`instruction/09-architecture/03-config.md`](../instruction/09-architecture/03-config.md), [`instruction/09-architecture/02-plugin.md`](../instruction/09-architecture/02-plugin.md), [`instruction/09-architecture/04-graceful-shutdown.md`](../instruction/09-architecture/04-graceful-shutdown.md), [`instruction/09-architecture/06-canary-deploy.md`](../instruction/09-architecture/06-canary-deploy.md) — how the pieces wire together, hot reload, drain-on-shutdown, staged rollout
- [`instruction/12-testing/`](../instruction/12-testing) — load, fuzz, and chaos testing the finished proxy

## Practice

Build [`proxy`](.) incrementally, on top of what [`labs/05-reverse-proxy`](../labs/05-reverse-proxy) and
[`labs/06-load-balancer`](../labs/06-load-balancer) taught you:

1. Add TLS termination with `tokio-rustls`; confirm ALPN correctly picks h1 vs h2 per client.
2. Load config (upstreams, listen addr, rate limits) from a TOML file at startup; add SIGHUP or file-watch based hot reload that swaps config without dropping active connections.
3. Add JWT validation on a protected route set and per-client token-bucket rate limiting; confirm a 401 and a 429 are returned correctly and rate-limit state resets on schedule.
4. Add `tracing` spans around the request lifecycle and export Prometheus metrics (request count, latency histogram, upstream error count).
5. Implement graceful shutdown: on SIGTERM, stop accepting new connections, let in-flight requests finish (with a deadline), then exit — verify with a load test running across the signal.
6. Add IP allow/deny lists and at least one WAF-style rule (e.g. block requests with suspicious header patterns); write a request-smuggling test case against your parser/hyper config and confirm it's rejected, not silently misrouted.
7. Benchmark against nginx configured as an equivalent reverse proxy, on the same machine and upstreams ([`instruction/12-testing/01-load-testing.md`](../instruction/12-testing/01-load-testing.md)). **Done when** you have throughput and p99 latency for both, and can explain every gap larger than 2x from a profile rather than a guess.
8. Run the chaos exercises in [`instruction/12-testing/03-chaos.md`](../instruction/12-testing/03-chaos.md) against it. **Done when** each injected failure produces the behavior and alert you predicted beforehand.
9. Read [`instruction/19-reading-source/pingora/reading-guide.md`](../instruction/19-reading-source/pingora/reading-guide.md) end to end and write your notes. **Done when** you've listed, per stop, the design choice where pingora differs from you and whether you'd adopt it.

Run with:

```
cargo run -p proxy
```
