# HTTP Stack

Phase 5. Everything the proxy does with an HTTP message before deciding
where to send it — parse it, route it, and the per-protocol behavior that
makes a proxy different from an HTTP server.

## Files

- [`01-parser.md`](01-parser.md) — hand-writing an HTTP/1.1 parser: incremental parsing, framing, limits
- [`02-hyper.md`](02-hyper.md) — hyper 1.x as a map: the crates, connection futures, services and why `Err` isn't a 500, streaming bodies and backpressure, timeouts, the pooled client
- [`03-hop-by-hop-headers.md`](03-hop-by-hop-headers.md) — which headers must never be forwarded, and why forwarding framing headers builds a smuggling bug
- [`04-router.md`](04-router.md) — matching method and path, precedence, and path normalization as a security boundary
- [`05-keepalive.md`](05-keepalive.md) — persistent connections, pooling lifecycle, connection lifetime caps, the close/request race
- [`06-static.md`](06-static.md) — streaming files, range requests, conditional requests, path traversal
- [`07-compression.md`](07-compression.md) — gzip/brotli/zstd negotiation, streaming compression, BREACH
- [`08-cache.md`](08-cache.md) — proxy caching semantics: directives, keys, `Vary`, poisoning
- [`09-cache-stampede.md`](09-cache-stampede.md) — single-flight coalescing, stale-while-revalidate, TTL jitter
- [`10-websocket.md`](10-websocket.md) — the upgrade handshake, Origin checking, framing, backpressure
- [`11-grpc.md`](11-grpc.md) — trailers, streaming shapes, per-RPC load balancing, gRPC-shaped errors
- [`12-vhost-routing.md`](12-vhost-routing.md) — Host/SNI routing, multi-tenant certificates, tenant isolation

## Reading order

[`01-parser.md`](01-parser.md) first (it backs [`labs/01-http-parser`](../../labs/01-http-parser), which the rest assumes
you've done), then [`02-hyper.md`](02-hyper.md) before [`labs/02-http-server`](../../labs/02-http-server) (every later lab is
built on hyper), then [`03-hop-by-hop-headers.md`](03-hop-by-hop-headers.md) and [`04-router.md`](04-router.md) — those two
define what a proxy is allowed to forward and how it decides where. The
rest are independent and map to their own labs: [`06-static.md`](06-static.md) →
[`labs/04-static-server`](../../labs/04-static-server), [`08-cache.md`](08-cache.md) + [`09-cache-stampede.md`](09-cache-stampede.md) →
[`labs/10-cache`](../../labs/10-cache), and so on.

Eviction algorithms live in [`13-algorithms/`](../13-algorithms); the normalization discipline
that [`04-router.md`](04-router.md) and [`07-security/06-waf.md`](../07-security/06-waf.md) share lives in
[`07-security/04-normalization.md`](../07-security/04-normalization.md).
