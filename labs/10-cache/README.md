# 10-cache

## Goal

An in-memory HTTP response cache in front of an upstream, keyed by method
+ URI (folding in `Vary`-listed headers), with freshness, revalidation,
bounded memory, and a purge path.

## Done when

- [ ] A second `GET` for a cacheable response is served without contacting the upstream (count upstream requests), and the response says so (for example an `X-Cache: HIT` header).
- [ ] `Cache-Control: no-store`, `private`, `max-age`, and `s-maxage` are honored, and `Vary` produces separate entries per varying header value.
- [ ] A stale entry with an `ETag` is revalidated with `If-None-Match`, and a `304` from the upstream refreshes it instead of refetching the body.
- [ ] 100 concurrent requests for a cold key produce exactly one upstream request (single-flight, [`instruction/05-http-stack/08-cache-stampede.md`](../../instruction/05-http-stack/08-cache-stampede.md)).
- [ ] Memory is bounded by a configured size, and a Zipf-distributed load test reports the hit ratio for at least two eviction policies (for example LRU vs TinyLFU).
- [ ] A purge request removes an entry, and the next request goes to the upstream.
- [ ] Reviewed per [`instruction/00-introduction/03-study-loop.md`](../../instruction/00-introduction/03-study-loop.md) step 5.

## Handbook references
- [`instruction/05-http-stack/07-cache.md`](../../instruction/05-http-stack/07-cache.md) — proxy caching semantics
- [`instruction/05-http-stack/08-cache-stampede.md`](../../instruction/05-http-stack/08-cache-stampede.md) — single-flight coalescing, stale-while-revalidate
- [`instruction/13-algorithms/lru.md`](../../instruction/13-algorithms/lru.md), [`instruction/13-algorithms/lfu.md`](../../instruction/13-algorithms/lfu.md), [`instruction/13-algorithms/arc.md`](../../instruction/13-algorithms/arc.md), [`instruction/13-algorithms/tinylfu.md`](../../instruction/13-algorithms/tinylfu.md) — eviction algorithms

## Run

```
cargo run -p cache
```
