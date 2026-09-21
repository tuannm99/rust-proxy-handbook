# HTTP Stack

Phase 5. Everything the proxy does with an HTTP message before deciding
where to send it — parse it, route it, and the per-protocol behavior that
makes a proxy different from an HTTP server.

## Files

- `01-parser.md` — hand-writing an HTTP/1.1 parser: incremental parsing, framing, limits
- `02-hop-by-hop-headers.md` — which headers must never be forwarded, and why forwarding framing headers builds a smuggling bug
- `03-router.md` — matching method and path, precedence, and path normalization as a security boundary
- `04-keepalive.md` — persistent connections, pooling lifecycle, connection lifetime caps, the close/request race
- `05-static.md` — streaming files, range requests, conditional requests, path traversal
- `06-compression.md` — gzip/brotli/zstd negotiation, streaming compression, BREACH
- `07-cache.md` — proxy caching semantics: directives, keys, `Vary`, poisoning
- `08-cache-stampede.md` — single-flight coalescing, stale-while-revalidate, TTL jitter
- `09-websocket.md` — the upgrade handshake, Origin checking, framing, backpressure
- `10-grpc.md` — trailers, streaming shapes, per-RPC load balancing, gRPC-shaped errors
- `11-vhost-routing.md` — Host/SNI routing, multi-tenant certificates, tenant isolation

## Reading order

`01-parser.md` first (it backs `labs/01-http-parser`, which the rest assumes
you've done), then `02-hop-by-hop-headers.md` and `03-router.md` — those two
define what a proxy is allowed to forward and how it decides where. The
rest are independent and map to their own labs: `05-static.md` →
`labs/04-static-server`, `07-cache.md` + `08-cache-stampede.md` →
`labs/10-cache`, and so on.

Eviction algorithms live in `13-algorithms/`; the normalization discipline
that `03-router.md` and `07-security/06-waf.md` share lives in
`07-security/04-normalization.md`.
