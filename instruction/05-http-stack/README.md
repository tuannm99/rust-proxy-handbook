# HTTP Stack

Phase 5. Everything the proxy does with an HTTP message before deciding
where to send it — parse it, route it, and the per-protocol behavior that
makes a proxy different from an HTTP server.

## Files

- `parser.md` — hand-writing an HTTP/1.1 parser: incremental parsing, framing, limits
- `hop-by-hop-headers.md` — which headers must never be forwarded, and why forwarding framing headers builds a smuggling bug
- `router.md` — matching method and path, precedence, and path normalization as a security boundary
- `keepalive.md` — persistent connections, pooling lifecycle, connection lifetime caps, the close/request race
- `static.md` — streaming files, range requests, conditional requests, path traversal
- `compression.md` — gzip/brotli/zstd negotiation, streaming compression, BREACH
- `cache.md` — proxy caching semantics: directives, keys, `Vary`, poisoning
- `cache-stampede.md` — single-flight coalescing, stale-while-revalidate, TTL jitter
- `websocket.md` — the upgrade handshake, Origin checking, framing, backpressure
- `grpc.md` — trailers, streaming shapes, per-RPC load balancing, gRPC-shaped errors
- `vhost-routing.md` — Host/SNI routing, multi-tenant certificates, tenant isolation

## Reading order

`parser.md` first (it backs `labs/01-http-parser`, which the rest assumes
you've done), then `hop-by-hop-headers.md` and `router.md` — those two
define what a proxy is allowed to forward and how it decides where. The
rest are independent and map to their own labs: `static.md` →
`labs/04-static-server`, `cache.md` + `cache-stampede.md` →
`labs/10-cache`, and so on.

Eviction algorithms live in `13-algorithms/`; the normalization discipline
that `router.md` and `07-security/waf.md` share lives in
`07-security/normalization.md`.
