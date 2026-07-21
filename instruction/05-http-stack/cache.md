# HTTP Cache

Eviction algorithms (LRU, LFU, ARC, TinyLFU) are covered in
`13-algorithms/`; this file covers proxy-specific caching semantics.

## What to learn

### Cache-Control directives that matter for a proxy
`max-age`, `s-maxage` (shared/proxy caches specifically, overrides `max-age` for proxies), `no-store` (never cache), `no-cache` (cache but always revalidate), and `private` (do not cache in a shared cache like this proxy — only the end client may). A proxy that ignores `private`/`no-store` can leak one user's response to another.

### Freshness vs validation
A cached response is either *fresh* (within its `max-age`) and can be served as-is, or *stale* and must be revalidated with the origin (a conditional request using `ETag`/`Last-Modified`, see `05-http-stack/static.md`) before being reused. Serving stale data without revalidation is a correctness bug, not an optimization.

### Where a proxy cache sits vs a CDN
A CDN caches at the edge, close to users, across many origins. An L7 proxy's cache (if it has one) sits directly in front of one origin/service, primarily to shield that origin from repeated identical requests — different scale, same mechanics (RFC 9111).

### Cache key design
The naive cache key is the URL, but correctness requires including `Vary`-listed headers (e.g. `Vary: Accept-Encoding` means gzip and uncompressed responses must be cached separately) and often the auth/session context if responses differ per user — otherwise you serve user A's response to user B.

```rust
// sketch: cache key must fold in Vary-listed request headers
struct CacheKey {
    method: Method,
    uri: String,
    vary_header_values: Vec<(String, String)>, // (header name, request's value) for each name in Vary
}
```

### Invalidation
Time-based expiry (`max-age`) is the easy case. Explicit invalidation (origin pushes a purge, or a write invalidates a related read) is the hard case every real cache eventually needs — plan for a purge-by-key or purge-by-prefix mechanism from the start rather than bolting it on later.

## Practice
1. In `labs/10-cache`, add an in-memory response cache (a `HashMap` behind a lock is fine to start) keyed by method+URI for GET requests only.
2. Respect `Cache-Control: no-store`/`private` from the origin response by never caching those.
3. Add `Vary` support: fold the listed request headers into the cache key and verify two clients requesting different `Accept-Encoding` get distinct cache entries.
4. Add freshness checking (`max-age`) and a revalidation path using conditional requests to the origin when stale.
5. Add a manual purge endpoint (e.g. `PURGE /path` or an admin API) and verify it evicts the right entries.
