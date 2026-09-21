# HTTP Cache

Eviction algorithms (LRU, LFU, ARC, TinyLFU) are covered in
`13-algorithms/`; this file covers proxy-specific caching semantics.

## What to learn

### Cache-Control directives that matter for a proxy
`max-age`, `s-maxage` (shared/proxy caches specifically, overrides `max-age` for proxies), `no-store` (never cache), `no-cache` (cache but always revalidate), and `private` (do not cache in a shared cache like this proxy — only the end client may). A proxy that ignores `private`/`no-store` can leak one user's response to another.

Four more that earn their keep:
- **`must-revalidate`**: once stale, the cache may *not* serve the stale
  copy even if the origin is unreachable. It removes the stale-on-error
  escape hatch below, which is sometimes exactly what a correctness-
  critical response needs.
- **`immutable`**: this will never change within its freshness lifetime,
  so don't even revalidate on a user-initiated reload. Pairs with
  content-hashed asset filenames (`05-http-stack/05-static.md`).
- **`stale-while-revalidate=N`**: serve the stale copy immediately and
  refresh in the background for up to N seconds. This is the single
  highest-leverage directive for a proxy cache — it decouples user-facing
  latency from origin latency entirely.
- **`stale-if-error=N`**: serve stale rather than propagating an origin
  error. Turns an origin outage into slightly-old content instead of a
  user-visible 5xx.

Gotcha: emit an `Age` header, and compute it correctly (time in this cache
plus any `Age` already present from an upstream cache). Downstream caches
and clients use it to compute remaining freshness; omitting it makes every
cache below you treat your stale response as brand new.

### Freshness vs validation
A cached response is either *fresh* (within its `max-age`) and can be served as-is, or *stale* and must be revalidated with the origin (a conditional request using `ETag`/`Last-Modified`, see `05-http-stack/05-static.md`) before being reused. Serving stale data without revalidation is a correctness bug, not an optimization.

Gotcha: what happens when the origin sends *no* freshness information at
all? RFC 9111 permits **heuristic freshness** — typically 10% of the time
since `Last-Modified` — and a proxy that quietly applies it starts caching
responses the origin never said were cacheable. That is standards-
compliant and still a surprise to whoever wrote the upstream. For a proxy
cache, defaulting to "no explicit freshness means don't cache" is the
safer policy; make heuristic caching opt-in per route.

### What is even eligible to cache
Before any of the above: the method must be `GET` or `HEAD` (never cache a
`POST` response against the URL alone), and the status code must be one
the spec allows caching by default — 200, 203, 204, 206, 300, 301, 404,
405, 410, 414, 501. Caching 404s and 301s is legitimate and valuable;
caching a 500 is not.

Gotcha: a request carrying `Authorization` must not have its response
stored in a *shared* cache unless the response explicitly permits it
(`public`, `s-maxage`, or `must-revalidate`). Skipping this check is a
direct path to serving one authenticated user's response to another — the
highest-severity bug this file can prevent.

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

Gotcha: `Vary` is a hit-rate destroyer when honored literally. `Vary:
User-Agent` means every browser version gets its own entry and your hit
rate collapses toward zero; `Vary: *` means never reuse, ever. And
`Accept-Encoding` values in the wild are wildly varied strings that all
mean the same few things — normalize them to a small set of buckets
(`gzip` / `br` / `identity`) before keying, or you fragment the cache
across dozens of equivalent entries.

Gotcha: the query string is part of the key, and its *order* usually
shouldn't be. `?a=1&b=2` and `?b=2&a=1` are the same resource to most
origins and two entries to a naive cache. Sort parameters (and decide
explicitly about ones that don't affect the response, like tracking
parameters) when building the key.

### Cache poisoning: the unkeyed-input attack
If any input influences the *response* but is not part of the *key*, an
attacker who controls that input can poison the entry for everyone else.
The classic shape: an origin reflects `X-Forwarded-Host` into absolute
URLs in the page, the proxy doesn't include that header in the key, so one
attacker request with `X-Forwarded-Host: evil.com` caches a page that
points every subsequent visitor at the attacker's domain.

Two defenses, both needed. Strip headers the origin shouldn't be seeing
from clients anyway (the trust-boundary discipline from
`07-security/08-ip-filtering.md` and `07-security/01-auth.md`). And treat any
header you deliberately forward as a key input unless you have positively
established the response doesn't depend on it.

Gotcha: "unkeyed input" includes things that don't look like inputs —
request method quirks, the port in `Host`, whether the path had a trailing
slash, and anything your own proxy adds before forwarding. The systematic
way to find them is to vary one input at a time and diff the response.

### Cache stampede
When a popular entry expires, every concurrent request for it misses at
once and they all go to the origin — the cache doing maximum damage at the
moment it stops helping. Request coalescing (single-flight) plus
`stale-while-revalidate` removes it entirely, and a cold start after a
restart is the same problem for every key at once.

See `05-http-stack/08-cache-stampede.md`.

### Invalidation
Time-based expiry (`max-age`) is the easy case. Explicit invalidation (origin pushes a purge, or a write invalidates a related read) is the hard case every real cache eventually needs — plan for a purge-by-key or purge-by-prefix mechanism from the start rather than bolting it on later.

Gotcha: with N proxy instances, each holds its own cache, so a purge must
reach all of them — and a purge endpoint that any client can call is a
cache-flushing denial of service (`07-security/09-ddos.md`). Authenticate the
purge path, and accept that propagation is eventually-consistent: design
for "purge reaches all instances within a few seconds", not instantly.

Gotcha: tag-based invalidation ("purge everything tagged `user:42`")
generalizes far better than purge-by-URL, because the code doing a write
knows what it changed but not which URLs rendered it. Storing a tag set
per entry costs little and is very hard to retrofit.

### Where a proxy cache sits vs a CDN
A CDN caches at the edge, close to users, across many origins. An L7 proxy's cache (if it has one) sits directly in front of one origin/service, primarily to shield that origin from repeated identical requests — different scale, same mechanics (RFC 9111).

## Practice
Build these in order.

1. In `labs/10-cache`, add an in-memory cache keyed by method+URI for
   `GET` only. **Done when** a second identical request is served from
   cache without touching the origin (prove it with an origin-side
   counter).
2. Add eligibility checks: method, cacheable status codes, `no-store`,
   `private`, and the `Authorization` rule. **Done when** an
   authenticated response is never served to a second user — write a test
   that fails against your step-1 implementation.
3. Add `Vary` support with `Accept-Encoding` normalization and query
   parameter sorting. **Done when** two clients with different
   `Accept-Encoding` get distinct entries, and `?a=1&b=2` / `?b=2&a=1`
   share one.
4. Add freshness (`max-age`/`s-maxage`), the `Age` header, and
   revalidation via conditional requests. **Done when** a stale entry
   triggers exactly one conditional request and a `304` refreshes it
   without transferring the body.
5. Work through `05-http-stack/08-cache-stampede.md`'s exercises. **Done
   when** 500 concurrent requests for one expired key produce exactly one
   origin hit, and `stale-while-revalidate` means none of them wait.
6. Add `stale-if-error`. **Done when** taking the origin fully offline
   still serves cached content rather than 5xx.
7. Mount a cache-poisoning attack: make the origin reflect a header you
   forward but don't key on, poison an entry, and fetch it as a different
   client. **Done when** the attack works, and then **done again when**
   stripping/keying that header stops it.
8. Add authenticated purge, by key and by tag. **Done when** a tagged
   purge evicts every related entry, an unauthenticated purge is rejected,
   and running two proxy instances shows both converging after a purge.
