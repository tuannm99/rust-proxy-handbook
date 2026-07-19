# Project 4
Production L7 Proxy.

## Goal
Everything from Projects 1-3, hardened toward something you'd actually run:
TLS termination, authentication, rate limiting, structured logs/metrics/
traces, config that reloads without dropping connections, and a shutdown
path that drains in-flight requests instead of cutting them off. "Done"
means: you can `curl` it over HTTPS, hit a rate limit and get a proper 429,
change the upstream list in the config file and see it take effect without
a restart, and send `SIGTERM` during a load test and see zero failed
in-flight requests (only refused new ones).

## What to learn
- `01-network/tls.md` — handshake, SNI, ALPN (also decides h1 vs h2), session resumption
- `01-network/proxy-protocol.md` — preserving real client IP when this proxy sits behind another LB
- `07-security/auth.md`, `07-security/ratelimit.md`, `07-security/waf.md` — JWT/mTLS, token/leaky bucket, rule-based filtering
- `07-security/request-smuggling.md`, `07-security/ip-filtering.md` — parser ambiguity attacks, allow/deny lists
- `08-observability/logging.md`, `metrics.md`, `tracing.md`, `profiling.md` — structured logs, Prometheus metrics, distributed traces, flamegraphs
- `09-architecture/components.md`, `config.md`, `plugin.md`, `graceful-shutdown.md`, `canary-deploy.md` — how the pieces wire together, hot reload, drain-on-shutdown, staged rollout

## Practice
Build `proxy` incrementally, on top of project-03's proxy logic:
1. Add TLS termination with `tokio-rustls`; confirm ALPN correctly picks h1 vs h2 per client.
2. Load config (upstreams, listen addr, rate limits) from a TOML file at startup; add SIGHUP or file-watch based hot reload that swaps config without dropping active connections.
3. Add JWT validation on a protected route set and per-client token-bucket rate limiting; confirm a 401 and a 429 are returned correctly and rate-limit state resets on schedule.
4. Add `tracing` spans around the request lifecycle and export Prometheus metrics (request count, latency histogram, upstream error count).
5. Implement graceful shutdown: on SIGTERM, stop accepting new connections, let in-flight requests finish (with a deadline), then exit — verify with a load test running across the signal.
6. Add IP allow/deny lists and at least one WAF-style rule (e.g. block requests with suspicious header patterns); write a request-smuggling test case against your parser/hyper config and confirm it's rejected, not silently misrouted.
